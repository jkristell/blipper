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
use common::{Reply, Command, Info};

mod blip;

const VERSION: u32 = 1;
const SAMPLERATE: u32 = 40_000;


#[app(device = stm32f1xx_hal::stm32)]
const APP: () = {

    static mut USB_DEV: UsbDevice<'static, UsbBusType> = ();
    static mut SERIAL: SerialPort<'static, UsbBusType> = ();

    static mut TIMER_MS: Timer<device::TIM2> = ();
    static mut IRPIN: PB8<Input<Floating>> = ();
    static mut SERIAL_RECV_BUF: Vec<u8, U64> = ();

    static mut BLIP: blip::Blip = ();

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
                .product("Blipper 010")
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

        init::LateResources {
            TIMER_MS: timer_ms,
            IRPIN: irpin,
            USB_DEV: usb_dev,
            SERIAL: serial,
            SERIAL_RECV_BUF: Default::default(),
            BLIP: blip::Blip::new(SAMPLERATE),
        }
    }

    #[idle]
    fn idle() -> ! {
        hprintln!("In idle").unwrap();
        loop {
            continue;
        }
    }

    #[interrupt(
        spawn = [send_reply],
        resources = [TIMER_MS, IRPIN, BLIP],
    )]
    fn TIM2() {
        static mut TS: u32 = 0;
        let edge = resources.IRPIN.is_low();
        // Ack the timer interrupt
        resources.TIMER_MS.clear_update_interrupt_flag();

        let blip = &mut resources.BLIP;

        match blip.state {
            blip::State::Idle => {}
            blip::State::CaptureRaw => {
                if let Some(reply) = blip.sample(edge, *TS) {
                    if spawn.send_reply(reply).is_err() {
                        hprintln!("Error sending").unwrap();
                    }

                    blip.reset();
                }
            }
        }

        // Update our timestamp
        *TS = TS.wrapping_add(1);
    }

    #[task(
        resources = [USB_DEV, SERIAL, ],
    )]
    fn send_reply(reply: Reply) {
        let mut serial = &mut resources.SERIAL;

        let reply_vec: heapless::Vec<u8, U512> = to_vec(&reply).unwrap();
        usb_write(&mut serial, &reply_vec);
    }

    #[interrupt(resources = [USB_DEV, SERIAL, SERIAL_RECV_BUF, BLIP, ])]
    fn USB_HP_CAN_TX() {
        let mut buf = resources.SERIAL_RECV_BUF;
        let mut blipper_blip = &mut resources.BLIP;
        usb_poll(&mut resources.USB_DEV, &mut resources.SERIAL, &mut buf, &mut blipper_blip);
    }

    #[interrupt(resources = [USB_DEV, SERIAL, SERIAL_RECV_BUF, BLIP, ])]
    fn USB_LP_CAN_RX0() {
        let mut buf = resources.SERIAL_RECV_BUF;
        let mut blipper_blip = &mut resources.BLIP;
        usb_poll(&mut resources.USB_DEV, &mut resources.SERIAL, &mut buf, &mut blipper_blip);
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
    blip: &mut blip::Blip
    //state: &mut BlipperState,
    //samplerate: &mut u32,
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
                blip.state = blip::State::Idle;
                usb_send_reply(serial, &Reply::Ok);
            }
            Command::Info => {
                let info: Info = Info {
                    version: VERSION,
                };
                usb_send_reply(serial, &Reply::Info {info});
            }
            Command::CaptureRaw => {
                hprintln!("cap raw").unwrap();
                blip.state = blip::State::CaptureRaw;
            }
            Command::CaptureProtocol(id) => {
                hprintln!("Capture protocol not implemented {}", id).unwrap();
            }
        }
        Err(_) => {},
    };

    buf.clear();
}


fn usb_write<B: bus::UsbBus>(
    serial: &mut SerialPort<'static, B>,
    towrite: &[u8],
) {
    let count = towrite.len();
    let mut write_offset = 0;

    while write_offset < count {
        match serial.write(&towrite[write_offset..]) {
            Ok(read) if read > 0 => {
                write_offset += read
            },
            _ => {},
        }
    }
}


fn usb_send_reply<B: bus::UsbBus>(
    serial: &mut SerialPort<'static, B>,
    reply: &Reply,
) {
    let replybytes: heapless::Vec<u8, U1024> = to_vec(&reply).unwrap();

    usb_write(serial, &replybytes);
}