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
    consts::U8,
};

use infrared::{Receiver, ReceiverState};
use infrared::trace::{TraceReceiver, TraceResult};


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
                .manufacturer("Fake company")
                .product("Serial port")
                .serial_number("TEST")
                .device_class(USB_CLASS_CDC)
                .build();

        // Setup the Timer
        let mut timer_ms = Timer::tim2(device.TIM2,
                                       20.khz(),
                                       clocks,
                                       &mut rcc.apb1);

        timer_ms.listen(timer::Event::Update);

        // Setup the IR input pin
        let mut gpiob = device.GPIOB.split(&mut rcc.apb2);
        let irpin = gpiob.pb8.into_floating_input(&mut gpiob.crh);

        // Setup the receiver
        let receiver = TraceReceiver::new(20_000);

        static mut QUEUE: spsc::Queue<TraceResult, U8> = spsc::Queue(heapless::i::Queue::new());
        let (prod, cons) = unsafe{QUEUE.split()};

        init::LateResources {
            PROD: prod,
            CONS: cons,

            TIMER_MS: timer_ms,
            RECEIVER: receiver,
            IRPIN: irpin,
            USB_DEV: usb_dev,
            SERIAL: serial,
        }
    }

    #[idle(
        resources = [CONS],
        spawn = [send_trace],
    )]
    fn idle() -> ! {

        loop {
            while let Some(tr) = resources.CONS.dequeue() {
                //hprintln!("r: {:?}", tr).unwrap();

                spawn.send_trace(tr).unwrap();
            }
        }
    }

    #[interrupt(
        priority = 2,
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
                resources.PROD.enqueue(res).unwrap();
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
        resources = [USB_DEV, SERIAL],
    )]
    fn send_trace(res: TraceResult) {

        usb_write(&mut resources.SERIAL, b"DATA ");

        for n in &res.buf {
            let mut sb = [b' '; 11];
            let s = u32_to_buf(*n, &mut sb[0..10]);
            usb_write(&mut resources.SERIAL, &sb[s..]);
        }

        usb_write(&mut resources.SERIAL, b"\r\n");
    }

    #[interrupt(resources = [USB_DEV, SERIAL])]
    fn USB_HP_CAN_TX() {
        usb_poll(&mut resources.USB_DEV, &mut resources.SERIAL);
    }

    #[interrupt(resources = [USB_DEV, SERIAL])]
    fn USB_LP_CAN_RX0() {
        usb_poll(&mut resources.USB_DEV, &mut resources.SERIAL);
    }

    // Interrupt used by the tasks
    extern "C" {
        fn USART1();
    }
};


fn usb_poll<B: bus::UsbBus>(
    usb_dev: &mut UsbDevice<'static, B>,
    serial: &mut SerialPort<'static, B>,
) {
    if !usb_dev.poll(&mut [serial]) {
        return;
    }

    let mut buf = [0u8; 8];

    match serial.read(&mut buf) {
        Ok(count) if count > 0 => {
            // Echo back in upper case
            for c in buf[0..count].iter_mut() {
                if 0x61 <= *c && *c <= 0x7a {
                    *c &= !0x20;
                }
            }

            serial.write(&buf[0..count]).ok();
        }
        _ => {}
    }
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

