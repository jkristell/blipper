#![no_main]
#![no_std]

use cortex_m::asm::delay;
use cortex_m_semihosting::hprintln;
use panic_semihosting as _;
use rtic::app;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use stm32f1xx_hal::usb::{Peripheral, UsbBus, UsbBusType};
use usb_device::bus;
use usb_device::prelude::*;
use usbd_serial::{SerialPort, USB_CLASS_CDC};

use stm32f1xx_hal::{
    gpio::{gpiob::PB8, Floating, Input},
    pac,
    prelude::*,
    pwm::{PwmChannel, C4},
    stm32::TIM4,
    timer::{self, CountDownTimer, Timer, Tim4NoRemap},
};

use blipper_protocol::{Command, Info, Reply};
use heapless::{consts::*, Vec};
use postcard::{from_bytes, to_vec};

mod blip;

const VERSION: u32 = 1;
const SAMPLERATE: u32 = 40_000;

#[app(device = stm32f1xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        usbdev: UsbDevice<'static, UsbBusType>,
        serial: SerialPort<'static, UsbBusType>,
        recvbuf: Vec<u8, U64>,
        timer2: CountDownTimer<pac::TIM2>,
        irpin: PB8<Input<Floating>>,
        pwm: PwmChannel<TIM4, C4>,
        blip: blip::Blip,
    }

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<bus::UsbBusAllocator<UsbBusType>> = None;

        let device = ctx.device;
        let mut flash = device.FLASH.constrain();
        let mut rcc = device.RCC.constrain();

        let clocks = rcc
            .cfgr
            .use_hse(8.mhz())
            .sysclk(48.mhz())
            .pclk1(24.mhz())
            .freeze(&mut flash.acr);

        assert!(clocks.usbclk_valid());

        let mut gpioa = device.GPIOA.split(&mut rcc.apb2);

        // BluePill board has a pull-up resistor on the D+ line.
        // Pull the D+ pin down to send a RESET condition to the USB bus.
        let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
        let _ = usb_dp.set_low().ok();
        delay(clocks.sysclk().0 / 100);

        let usb_dm = gpioa.pa11;
        let usb_dp = usb_dp.into_floating_input(&mut gpioa.crh);

        let usb = Peripheral {
            usb: device.USB,
            pin_dm: usb_dm,
            pin_dp: usb_dp,
        };

        *USB_BUS = Some(UsbBus::new(usb));
        let serial = SerialPort::new(USB_BUS.as_ref().unwrap());

        let usbdev = UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("Blipper Remotes")
            .product("Blipper 010")
            .serial_number("007")
            .device_class(USB_CLASS_CDC)
            .build();

        // Setup the Timer
        let mut timer = Timer::tim2(device.TIM2,
                                    &clocks,
                                    &mut rcc.apb1)
            .start_count_down(SAMPLERATE.hz());

        timer.listen(timer::Event::Update);

        // Setup the IR input pin
        let mut gpiob = device.GPIOB.split(&mut rcc.apb2);
        let irpin = gpiob.pb8.into_floating_input(&mut gpiob.crh);

        // PWM
        let mut afio = device.AFIO.constrain(&mut rcc.apb2);
        let irled = gpiob.pb9.into_alternate_push_pull(&mut gpiob.crh);

        let pwm = Timer::tim4(device.TIM4, &clocks, &mut rcc.apb1).pwm::<Tim4NoRemap, _, _, _>(
            irled,
            &mut afio.mapr,
            38.khz(),
        );


        let mut pwmpin = pwm.split();

        pwmpin.set_duty(pwmpin.get_max_duty() / 2);
        pwmpin.disable();



        init::LateResources {
            usbdev,
            serial,
            recvbuf: Default::default(),
            timer2: timer,
            irpin,
            pwm: pwmpin,
            blip: blip::Blip::new(SAMPLERATE),
        }
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        let _ = hprintln!("Ready!").ok();
        loop {
            continue;
        }
    }

    #[task(resources = [serial])]
    fn send_reply(ctx: send_reply::Context, reply: Reply) {
        let mut serial = ctx.resources.serial;
        let reply_vec: heapless::Vec<u8, U512> = to_vec(&reply).unwrap();
        usb_write(&mut serial, &reply_vec);
    }

    #[task(
        binds = TIM2,
        spawn = [send_reply],
        resources = [timer2, irpin, blip, pwm],
    )]
    fn t2_irq(ctx: t2_irq::Context) {
        static mut TS: u32 = 0;
        let t2_irq::Resources {timer2, irpin, blip, pwm} = ctx.resources;

        let level = irpin.is_low().unwrap();
        timer2.clear_update_interrupt_flag();

        if let Some(reply) = blip.tick(*TS, level, pwm) {
            ctx.spawn.send_reply(reply).unwrap();
        }

        // Update our timestamp
        *TS = TS.wrapping_add(1);
    }

    #[task(
        binds = USB_HP_CAN_TX,
        resources = [usbdev, serial, recvbuf, blip]
    )]
    fn usb_tx(ctx: usb_tx::Context) {
        let usb_tx::Resources { usbdev, serial, recvbuf, blip } = ctx.resources;

        usb_poll(
            usbdev,
            serial,
            recvbuf,
            blip,
        );
    }

    #[task(
        binds = USB_LP_CAN_RX0,
        resources = [usbdev, serial, recvbuf, blip]
    )]
    fn usb_rx(ctx: usb_rx::Context) {
        let usb_rx::Resources { usbdev, serial, recvbuf, blip } = ctx.resources;

        usb_poll(
            usbdev,
            serial,
            recvbuf,
            blip,
        );
    }

    // Interrupt used by the tasks
    extern "C" {
        fn USART1();
    }
};

fn usb_poll<B: bus::UsbBus>(
    usbdev: &mut UsbDevice<'static, B>,
    serial: &mut SerialPort<'static, B>,
    buf: &mut Vec<u8, U64>,
    blip: &mut blip::Blip,
) {
    if !usbdev.poll(&mut [serial]) {
        return;
    }

    let mut tmpbuf = [0u8; 32];

    match serial.read(&mut tmpbuf) {
        Ok(count) if count > 0 => {
            let _ = buf.extend_from_slice(&tmpbuf);
        }
        _ => {}
    }

    match from_bytes::<Command>(&buf) {
        Ok(cmd) => match cmd {
            Command::Idle => {
                let _ = hprintln!("cmd idle").ok();
                blip.state = blip::State::Idle;
                usb_send_reply(serial, &Reply::Ok);
            }
            Command::Info => {
                let info: Info = Info {
                    version: VERSION,
                    transmitters: blip::ENABLED_TRANSMITTERS,
                };
                let _ = hprintln!("info").ok();
                usb_send_reply(serial, &Reply::Info { info });
            }
            Command::Capture => {
                let _ = hprintln!("State: capture").ok();
                blip.capturer.reset();

                blip.state = blip::State::CaptureRaw;
            }
            Command::CaptureProtocol(id) => {
                let _ = hprintln!("Not implemented: {}", id);
            }
            Command::RemoteControlSend(cmd) => {
                let _ = hprintln!("sending").ok();
                usb_send_reply(serial, &Reply::Ok);

                blip.txers.load(cmd.txid, cmd.addr, cmd.cmd);
                blip.state = blip::State::IrSend;
            }
        },
        Err(_) => {}
    };

    buf.clear();
}

fn usb_write<B: bus::UsbBus>(serial: &mut SerialPort<'static, B>, towrite: &[u8]) {
    let count = towrite.len();
    let mut write_offset = 0;

    while write_offset < count {
        match serial.write(&towrite[write_offset..]) {
            Ok(read) if read > 0 => write_offset += read,
            _ => {}
        }
    }
}

fn usb_send_reply<B: bus::UsbBus>(serial: &mut SerialPort<'static, B>, reply: &Reply) {
    let replybytes: heapless::Vec<u8, U1024> = to_vec(&reply).unwrap();

    usb_write(serial, &replybytes);
}
