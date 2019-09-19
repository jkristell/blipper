#![no_main]
#![no_std]
#![allow(deprecated)]

use panic_semihosting as _;
use cortex_m::asm::delay;
use cortex_m_semihosting::hprintln;
use rtfm::app;

use stm32f1xx_hal::{
    gpio::{gpiob::PB8, Floating, Input},
    prelude::*,
    timer::{self, Timer},
    device,
};

use stm32_usbd::{UsbBus, UsbBusType};
use usb_device::bus;
use usb_device::prelude::*;
use usbd_serial::{SerialPort, USB_CLASS_CDC};

use heapless::{
    consts::*,
    Vec,
};

use postcard::{to_vec, from_bytes};

use infrared::{Receiver, ReceiverState};
use infrared::trace::{TraceReceiver, TraceResult};

use common::{Reply, Command};

const SAMPLERATE: u32 = 40_000;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum BlipperState {
    Idle,
    CaptureRaw,
}

#[app(device = stm32f1xx_hal::stm32)]
const APP: () = {

    static mut USB_DEV: UsbDevice<'static, UsbBusType> = ();
    static mut SERIAL: SerialPort<'static, UsbBusType> = ();

    static mut TIMER_MS: Timer<device::TIM2> = ();
    static mut RECEIVER: TraceReceiver = ();
    static mut IRPIN: PB8<Input<Floating>> = ();

    static mut SERIAL_RECV_BUF: Vec<u8, U64> = ();
    static mut BLIPPER_STATE: BlipperState = BlipperState::Idle;

    #[init]
    fn init() -> init::LateResources {
        static mut USB_BUS: Option<bus::UsbBusAllocator<UsbBusType>> = None;

        let mut flash = device.FLASH.constrain();
        let mut rcc = device.RCC.constrain();

        let clocks = rcc.cfgr
            .use_hse(8.mhz())
            .sysclk(48.mhz())
            .pclk1(24.mhz())
            .freeze(&mut flash.acr);

        assert!(clocks.usbclk_valid());

        let mut gpioa = device.GPIOA.split(&mut rcc.apb2);

        // BluePill board has a pull-up resistor on the D+ line.
        // Pull the D+ pin down to send a RESET condition to the USB bus.
        let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
        usb_dp.set_low();
        delay(clocks.sysclk().0 / 100);

        let usb_dm = gpioa.pa11;
        let usb_dp = usb_dp.into_floating_input(&mut gpioa.crh);

        *USB_BUS = Some(UsbBus::new(device.USB, (usb_dm, usb_dp)));

        let serial = SerialPort::new(USB_BUS.as_ref().unwrap());

        let usb_dev =
            UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x16c0, 0x27dd))
                .manufacturer("Blipper Remotes")
                .product("Blipper")
                .serial_number("007")
                .device_class(USB_CLASS_CDC)
                .build();

        // Setup the Timer
        let mut timer_ms = Timer::tim2(device.TIM2,
                                       SAMPLERATE.hz(),
                                       clocks,
                                       &mut rcc.apb1);

        timer_ms.listen(timer::Event::Update);

        // Setup the IR input pin
        let mut gpiob = device.GPIOB.split(&mut rcc.apb2);
        let irpin = gpiob.pb8.into_floating_input(&mut gpiob.crh);

        // Setup the receiver
        let receiver = TraceReceiver::new(SAMPLERATE);

        init::LateResources {
            TIMER_MS: timer_ms,
            RECEIVER: receiver,
            IRPIN: irpin,
            USB_DEV: usb_dev,
            SERIAL: serial,
            SERIAL_RECV_BUF: Default::default(),
        }
    }

    #[idle]
    fn idle() -> ! {

        hprintln!("Hello").unwrap();

        loop {}
    }

    #[interrupt(
        spawn = [send_capture],
        resources = [TIMER_MS, RECEIVER, IRPIN, BLIPPER_STATE],
    )]
    fn TIM2() {
        // Sample num
        static mut TS: u32 = 0;
        // Active low
        let rising = resources.IRPIN.is_low();
        // Ack the timer interrupt
        resources.TIMER_MS.clear_update_interrupt_flag();

        // Step the receivers state machine
        let state = resources.RECEIVER.event(rising, *TS);

        match state {
            ReceiverState::Done(res) => {

                resources.BLIPPER_STATE.lock(|&mut state|
                    if state == BlipperState::CaptureRaw {
                        if spawn.send_capture(res).is_err() {
                            hprintln!("Error sending").unwrap();
                        }
                    }
                );

                resources.RECEIVER.reset();
            }
            ReceiverState::Receiving => (),
            _ => (),
        }

        // Update our timestamp
        *TS = TS.wrapping_add(1);
    }

    #[task(
        resources = [USB_DEV, SERIAL, ],
    )]
    fn send_capture(tr: TraceResult) {

        let mut serial = &mut resources.SERIAL;

        let mut data = [0u8; 4 * 128];

        for i in 0..tr.buf_len {
            let bytes = tr.buf[i].to_le_bytes();
            data[i*4 .. i*4 + 4].copy_from_slice(&bytes);
        }

        let reply_cmd = Reply::CaptureRawData {
            data: &data[0..tr.buf_len * 4],
        };
        let reply: heapless::Vec<u8, U1024> = to_vec(&reply_cmd).unwrap();
        usb_write(&mut serial, &reply);
    }

    #[interrupt(resources = [USB_DEV, SERIAL, SERIAL_RECV_BUF, BLIPPER_STATE])]
    fn USB_HP_CAN_TX() {
        let mut buf = resources.SERIAL_RECV_BUF;
        usb_poll(&mut resources.USB_DEV, &mut resources.SERIAL, &mut buf, &mut resources.BLIPPER_STATE);
    }

    #[interrupt(resources = [USB_DEV, SERIAL, SERIAL_RECV_BUF, BLIPPER_STATE])]
    fn USB_LP_CAN_RX0() {
        let mut buf = resources.SERIAL_RECV_BUF;
        let mut state = &mut resources.BLIPPER_STATE;
        usb_poll(&mut resources.USB_DEV, &mut resources.SERIAL, &mut buf, &mut state);
    }

    // Interrupt used by the tasks
    extern "C" {
        fn USART1();
    }
};


fn usb_poll<B: bus::UsbBus>(
    usb_dev: &mut UsbDevice<'static, B>,
    serial: &mut SerialPort<'static, B>,
    buf: &mut Vec<u8, U64>,
    state: &mut BlipperState,
) {


    if !usb_dev.poll(&mut [serial]) {
        return;
    }

    let mut localbuf = [0u8; 32];

    match serial.read(&mut localbuf) {
        Ok(count) if count > 0 => {
            for c in &localbuf {
                buf.push(*c).unwrap();
            }
        }
        _ => {}
    }

    match from_bytes::<Command>(&buf) {
        Ok(cmd) => match cmd {
            Command::Idle => {
                hprintln!("cmd idle").unwrap();
                *state = BlipperState::Idle;
                send_reply(serial, &Reply::Ok);
            },
            Command::CaptureRaw => {
                // Initialize the capturing
                *state = BlipperState::CaptureRaw;

                let reply = Reply::CaptureRawHeader {samplerate: SAMPLERATE};
                send_reply(serial, &reply);

                hprintln!("cmd craw").unwrap()
            },
        }
        _ => (),
    };

    //buf.clear();
}

fn usb_write<B: bus::UsbBus>(
    serial: &mut SerialPort<'static, B>,
    towrite: &[u8],
) {
    serial.write(towrite).ok();
}


fn send_reply<B: bus::UsbBus>(
    serial: &mut SerialPort<'static, B>,
    reply: &Reply,
) {
    let reply: heapless::Vec<u8, U1024> = to_vec(&reply).unwrap();
    serial.write(&reply).ok();
}