#![no_main]
#![no_std]

use panic_rtt_target as _;
use rtt_target::{rprintln, rprint, rtt_init_print};

use stm32f4xx_hal::{timer::Event, otg_fs::UsbBusType, prelude::*, pwm, stm32::{TIM2, TIM5}, timer::Timer, stm32};
use stm32f4xx_hal::otg_fs::{USB, UsbBus};
use usb_device::prelude::*;
use usbd_serial::{SerialPort, USB_CLASS_CDC};
use embedded_hal::digital::v2::OutputPin;
use heapless::{consts::*, Vec};
use postcard::{from_bytes, to_vec};
use usb_device::bus;
//use usb_device::prelude::*;

use libblip as blip;
use blip::{Reply};

use rtic::app;

static mut EP_MEMORY: [u32; 1024] = [0; 1024];

type PwmPinType = pwm::PwmChannels<TIM5, pwm::C2>;
type BlipType = blip::Blip<PwmPinType, u16>;

#[app(device = stm32f4xx_hal::stm32, peripherals = true)]
const APP: () = {
    struct Resources {
        usbdev: UsbDevice<'static, UsbBusType>,
        serial: SerialPort<'static, UsbBusType>,
        recvbuf: Vec<u8, U64>,
        blip: BlipType,
        timer2: Timer<TIM2>,
    }

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<bus::UsbBusAllocator<UsbBusType>> = None;

        rtt_init_print!();

        // Device specific peripherals
        let device = ctx.device;

        // Enable the clock for the SYSCFG
        // device.RCC.apb2enr.modify(|_, w| w.syscfgen().enabled());
        // Setup the system clock
        let rcc = device.RCC.constrain();

        // Configure clock to 168 MHz (i.e. the maximum) and freeze it
        let clocks = rcc.cfgr
                .sysclk(168.mhz())
                .require_pll48clk()
                .freeze();

        let gpioa = device.GPIOA.split();
        let _gpioc = device.GPIOC.split();
        let gpiod = device.GPIOD.split();

        // Timers
        let mut timer2 = Timer::tim2(device.TIM2,
                                     1.hz(),
                                     clocks);

        //timer2.start(1.hz());
        timer2.listen(Event::TimeOut);

        let channels = gpioa.pa1.into_alternate_af2();
        let irpwm = pwm::tim5(device.TIM5,
            channels,
            clocks,
            38.khz()
        );

        let blip = blip::Blip::new(irpwm, 20_000);

        let usb = USB {
            usb_global: device.OTG_FS_GLOBAL,
            usb_device: device.OTG_FS_DEVICE,
            usb_pwrclk: device.OTG_FS_PWRCLK,
            pin_dm: gpioa.pa11.into_alternate_af10(),
            pin_dp: gpioa.pa12.into_alternate_af10(),
        };

        *USB_BUS = Some(UsbBus::new(usb, unsafe {EP_MEMORY.as_mut()}));
        //let serial = SerialPort::new(USB_BUS.as_ref().unwrap());
        let serial = SerialPort::new(USB_BUS.as_ref().unwrap());

        let usbdev = UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("Blipper Remotes")
            .product("Blipper 010")
            .serial_number("007")
            .device_class(USB_CLASS_CDC)
            .build();

        rprintln!("init done");

        init::LateResources {
            usbdev,
            serial,
            recvbuf: Default::default(),
            blip,
            timer2,
        }
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        rprintln!("In idle");

        loop {
            continue;
        }
    }

    #[task(binds = TIM2,
           spawn = [send_reply],
           resources = [timer2, blip])
    ]
    fn timer2_event(cx: timer2_event::Context) {
        static mut TIMESTAMP: u32 = 0;

        let timer2_event::Resources {
            timer2,
            blip,
        } = cx.resources;

        if let Some(reply) = blip.tick(*TIMESTAMP, false) {
            //let _ = ctx.spawn.send_reply(reply);
        }
        //rprint!("T");

        timer2.clear_interrupt(Event::TimeOut);
    }

    #[task(binds = OTG_FS, resources = [usbdev, serial, recvbuf])]
    fn usbi(cx: usbi::Context) {

        let usbi::Resources {
            usbdev,
            serial,
            recvbuf
        } = cx.resources;

        if !usbdev.poll(&mut [serial]) {
            return;
        }
    
        let mut data = [0u8; 64];
    
        match serial.read(&mut data) {
            Ok(count) if count > 0 => {
                rprintln!("count = {}", count);
                //buf.extend_from_slice(&data).unwrap();
    
                if let Some(cmd) = blip::cmd_from_bytes(&data) {
                    rprintln!("Cmd: {:?}", cmd);
                    //let reply = blip.handle_command(cmd);
                    let reply = Reply::Ok;
                    serial_reply(serial, &reply);
                }
            }
            Ok(_) => (),
            Err(_e) => (), //rprintln!("serial err: {:?}", e),
        }
    
        recvbuf.clear();
    }

    #[task(resources = [serial])]
    fn send_reply(ctx: send_reply::Context, reply: Reply) {
        let mut serial = ctx.resources.serial;
        let reply_vec: heapless::Vec<u8, U512> = to_vec(&reply).unwrap();
        serial_send(&mut serial, &reply_vec);
    }
};

fn serial_reply<B: bus::UsbBus>(serial: &mut SerialPort<'static, B>, reply: &Reply) {
    let d: heapless::Vec<u8, U1024> = to_vec(&reply).unwrap();
    serial_send(serial, &d);
}

fn serial_send<B: bus::UsbBus>(serial: &mut SerialPort<'static, B>, data: &[u8]) {
    let count = data.len();
    let mut offset = 0;

    while offset < count {
        match serial.write(&data[offset..]) {
            Ok(sent) if sent > 0 => offset += sent,
            _ => {}
        }
    }
}