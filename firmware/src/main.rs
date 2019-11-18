#![no_main]
#![no_std]
#![allow(deprecated)]

use panic_semihosting as _;
use cortex_m::asm::delay;
use cortex_m_semihosting::hprintln;
use rtfm::app;

use stm32f1xx_hal::{
    prelude::*,
    pac,
    gpio::{gpiob::{PB8, PB9}, Floating, Input},
    gpio::{Alternate, PushPull},
    pwm::{Pins, Pwm, C4},
    stm32::{TIM4},
    timer::{self, Timer, CountDownTimer, },
};

use stm32f1xx_hal::usb::{Peripheral, UsbBus, UsbBusType};

use embedded_hal::digital::v2::{
    InputPin,
    OutputPin
};

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

struct PwmChannels(PB9<Alternate<PushPull>>);
impl Pins<TIM4> for PwmChannels {
    const REMAP: u8 = 0b00;
    const C1: bool = false;
    const C2: bool = false;
    const C3: bool = false;
    const C4: bool = true; // PB9
    type Channels = Pwm<TIM4, C4>;
}

#[app(device = stm32f1xx_hal::pac, peripherals = true)]
const APP: () = {

    struct Resources {
        USB_DEV: UsbDevice<'static, UsbBusType>,
        SERIAL: SerialPort<'static, UsbBusType>,

        TIMER_MS: CountDownTimer<pac::TIM2>,
        IRPIN: PB8<Input<Floating>>,
        PWM: Pwm<TIM4, C4>,

        SERIAL_RECV_BUF: Vec<u8, U64>,
        BLIP: blip::Blip,
    }


    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<bus::UsbBusAllocator<UsbBusType>> = None;

        let device = ctx.device;
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

        let usb_dev =
            UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x16c0, 0x27dd))
                .manufacturer("Blipper Remotes")
                .product("Blipper 010")
                .serial_number("007")
                .device_class(USB_CLASS_CDC)
                .build();

        // Setup the Timer
        let mut timer_ms = Timer::tim2(device.TIM2, &clocks, &mut rcc.apb1)
            .start_count_down(SAMPLERATE.hz());

        timer_ms.listen(timer::Event::Update);

        // Setup the IR input pin
        let mut gpiob = device.GPIOB.split(&mut rcc.apb2);
        let irpin = gpiob.pb8.into_floating_input(&mut gpiob.crh);

        // PWM
        let mut afio = device.AFIO.constrain(&mut rcc.apb2);
        let irled = gpiob.pb9.into_alternate_push_pull(&mut gpiob.crh);

        let mut c4 = Timer::tim4(device.TIM4, &clocks, &mut rcc.apb1)
            .pwm(PwmChannels(irled), &mut afio.mapr, 38.khz());

        // Set the duty cycle of channel 0 to 50%
        c4.set_duty(c4.get_max_duty() / 2);
        c4.disable();

        init::LateResources {
            TIMER_MS: timer_ms,
            IRPIN: irpin,
            PWM: c4,
            USB_DEV: usb_dev,
            SERIAL: serial,
            SERIAL_RECV_BUF: Default::default(),
            BLIP: blip::Blip::new(SAMPLERATE),
        }
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        let _ = hprintln!("Ready!");
        loop {
            continue;
        }
    }

    #[task(resources = [USB_DEV, SERIAL])]
    fn send_reply(ctx: send_reply::Context, reply: Reply) {
        let mut serial = ctx.resources.SERIAL;

        let reply_vec: heapless::Vec<u8, U512> = to_vec(&reply).unwrap();
        usb_write(&mut serial, &reply_vec);
    }

    #[task(
        binds = TIM2,
        spawn = [send_reply],
        resources = [TIMER_MS, IRPIN, BLIP, PWM],
    )]
    fn timer2_interrupt(ctx: timer2_interrupt::Context) {
        static mut TS: u32 = 0;
        let edge = ctx.resources.IRPIN.is_low().unwrap();
        // Ack the timer interrupt
        let timer = ctx.resources.TIMER_MS;

        timer.clear_update_interrupt_flag();

        let blip = ctx.resources.BLIP;

        match blip.state {
            blip::State::Idle => {}
            blip::State::CaptureRaw => {

                if let Some(reply) = blip.sample(edge, *TS) {
                    if ctx.spawn.send_reply(reply).is_err() {
                        hprintln!("Error sending").unwrap();
                    }

                    blip.reset();
                }
            }
            blip::State::IrSend => {
                blip.irsend(*TS, ctx.resources.PWM);
            }
        }

        // Update our timestamp
        *TS = TS.wrapping_add(1);
    }

    #[task(
        binds = USB_HP_CAN_TX,
        resources = [USB_DEV, SERIAL, SERIAL_RECV_BUF, BLIP])]
    fn usb_tx(ctx: usb_tx::Context) {
        let mut buf = ctx.resources.SERIAL_RECV_BUF;
        let blipper_blip = ctx.resources.BLIP;
        usb_poll(ctx.resources.USB_DEV, ctx.resources.SERIAL, &mut buf, blipper_blip);
    }

    #[task(
        binds = USB_LP_CAN_RX0,
        resources = [USB_DEV, SERIAL, SERIAL_RECV_BUF, BLIP])]
    fn usb_rx(ctx: usb_rx::Context) {
        let mut buf = ctx.resources.SERIAL_RECV_BUF;
        let blipper_blip = ctx.resources.BLIP;
        usb_poll(ctx.resources.USB_DEV, ctx.resources.SERIAL, &mut buf, blipper_blip);
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
) {
    if !usb_dev.poll(&mut [serial]) {
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
                let _ = hprintln!("cmd idle");
                blip.state = blip::State::Idle;
                usb_send_reply(serial, &Reply::Ok);
            }
            Command::Info => {
                let info: Info = Info {
                    version: VERSION,
                    transmitters: blip::ENABLED_TRANSMITTERS,
                };
                let _ = hprintln!("info");
                usb_send_reply(serial, &Reply::Info {info});
            }
            Command::CaptureRaw => {
                let _ = hprintln!("State: capture");
                blip.state = blip::State::CaptureRaw;
            }
            Command::CaptureProtocol(id) => {
                let _ = hprintln!("Not implemented: {}", id);
            }
            Command::RemoteControlSend(cmd) => {
                let _ = hprintln!("sending");
                usb_send_reply(serial, &Reply::Ok);

                blip.txers.load(cmd.txid, cmd.addr, cmd.cmd);
                blip.state = blip::State::IrSend;
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