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
    spsc::{self, Producer, Consumer},
    consts::{U8, U64, U256, U1024},
    Vec,
};

use postcard::{to_vec, from_bytes, Serializer};

use infrared::{Receiver, ReceiverState};
use infrared::trace::{TraceReceiver, TraceResult};

use common;

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

    //static mut RESQ: Queue<TraceResult, U8> = ();
    static mut PROD: Producer<'static, TraceResult, U8> = ();
    static mut CONS: Consumer<'static, TraceResult, U8> = ();

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

        static mut QUEUE: spsc::Queue<TraceResult, U8> = spsc::Queue(heapless::i::Queue::new());
        let (prod, cons) = unsafe {QUEUE.split()};

        init::LateResources {
            PROD: prod,
            CONS: cons,

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
        priority = 2,
        spawn = [send_capture],
        resources = [TIMER_MS, RECEIVER, IRPIN, PROD],
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
                //let _ = resources.PROD.enqueue(res);

                spawn.send_capture(res);

                resources.RECEIVER.reset();
            }
            ReceiverState::Receiving => {
            }
            _ => (),
        }

        // Update our timestamp
        *TS = TS.wrapping_add(1);
    }

    #[task(
        priority = 1,
        resources = [USB_DEV, SERIAL, CONS],
    )]
    fn send_capture(tr: TraceResult) {

        //let mut consumer = &mut resources.CONS;
        let mut serial = &mut resources.SERIAL;

        {

            let mut data = [0u8; 4 * 128];

            for i in 0..tr.buf_len {
                let bytes = tr.buf[i].to_le_bytes();
                data[i*4 .. i*4 + 4].copy_from_slice(&bytes);
            }

            let reply_cmd = common::Reply::CaptureRawData {
                data: &data[0..tr.buf_len * 4],
            };
            let reply: heapless::Vec<u8, U1024> = to_vec(&reply_cmd).unwrap();
            usb_write(&mut serial, &reply);
        }



        //usb_write(&mut resources.SERIAL, b"DATA ");
    }

    #[task(
        priority = 1,
        resources = [USB_DEV, SERIAL],
    )]
    fn send_trace(res: TraceResult) {

        usb_write(&mut resources.SERIAL, b"DATA ");

        for i in 0..res.buf_len {
            let mut sb = [b' '; 11];
            let s = u32_to_buf(res.buf[i], &mut sb[0..10]);
            usb_write(&mut resources.SERIAL, &sb[s..]);
        }

        usb_write(&mut resources.SERIAL, b"\r\n");
    }

    #[interrupt(resources = [USB_DEV, SERIAL, SERIAL_RECV_BUF, BLIPPER_STATE])]
    fn USB_HP_CAN_TX() {
        let mut buf = resources.SERIAL_RECV_BUF;
        let mut state = &mut resources.BLIPPER_STATE;
        usb_poll(&mut resources.USB_DEV, &mut resources.SERIAL, &mut buf, &mut state);
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
    state: &BlipperState,
) -> BlipperState {


    if !usb_dev.poll(&mut [serial]) {
        //FIXME:
        return *state;
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

    // Try deserialising
    let state = match from_bytes::<common::Command>(&buf) {
        Ok(cmd) => match cmd {
            common::Command::Idle => hprintln!("cmd idle").unwrap(),
            common::Command::CaptureRaw => {
                // Initialize the capturing


                hprintln!("cmd craw").unwrap()
            },
        }
        _ => (), //hprintln!("FAIL").unwrap(),
    };

    buf.clear();

    BlipperState::Idle
}

fn usb_write<B: bus::UsbBus>(
    serial: &mut SerialPort<'static, B>,
    towrite: &[u8],
) {
    serial.write(towrite).ok();
}

fn u32_to_buf(mut num: u32, buf: &mut [u8]) -> usize {

    let mut i = buf.len() - 1;

    if num == 0 {
        buf[i] = b'0';
    } else {
        loop {
            let last = num % 10;
            buf[i] = b'0' + (last as u8);

            num /= 10;
            if num == 0 {
                break
            }
            i -= 1;
        }
    }

    i
}

