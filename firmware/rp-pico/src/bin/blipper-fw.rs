#![no_std]
#![no_main]

use defmt::warn;
use embedded_hal::digital::v2::InputPin;
use rp_pico::hal::usb::UsbBus;
use usbd_serial::SerialPort;

use heapless::{Vec};
use postcard::{from_bytes, to_vec};

use rp_pico_examples as _;

#[rtic::app(device = rp_pico::hal::pac, peripherals = true, dispatchers = [XIP_IRQ])]
mod app {
    use defmt::*;
    use libblip::Blip;
    use rp2040_monotonic::*;
    use rp_pico::{hal::{clocks::init_clocks_and_plls, watchdog::Watchdog, usb::UsbBus}, hal, XOSC_CRYSTAL_FREQ};
    use core::mem::MaybeUninit;
    use rp_pico::hal::gpio::{Pin, PinMode};
    use rp_pico::hal::Sio;
use embedded_hal::digital::v2::InputPin;

    use usb_device::{class_prelude::*, prelude::*};
    use usbd_serial::SerialPort;
    use crate::{cmd_from_buf, serial_reply};

    #[monotonic(binds = TIMER_IRQ_0, default = true)]
    type Monotonic = Rp2040Monotonic;

    #[shared]
    struct Shared {
        blip: Blip,
    }

    #[local]
    struct Local {
        usb_dev: UsbDevice<'static, UsbBus>,
        serial: SerialPort<'static, UsbBus>,
        ir_pin: Pin<hal::gpio::bank0::Gpio12, hal::gpio::Input<hal::gpio::Floating>>,
        ts: u32,
    }

    #[init(local = [usb_bus: MaybeUninit<UsbBusAllocator<UsbBus>> = MaybeUninit::uninit()])]
    fn init(c: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut resets = c.device.RESETS;
        let mut watchdog = Watchdog::new(c.device.WATCHDOG);
        let clocks = init_clocks_and_plls(
            XOSC_CRYSTAL_FREQ,
            c.device.XOSC,
            c.device.CLOCKS,
            c.device.PLL_SYS,
            c.device.PLL_USB,
            &mut resets,
            &mut watchdog,
        )
            .ok()
            .unwrap();


        let usb_bus = c.local.usb_bus;
        let usb_bus = usb_bus.write(UsbBusAllocator::new(UsbBus::new(
            c.device.USBCTRL_REGS,
            c.device.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut resets,
        )));
        let serial = SerialPort::new(usb_bus);

        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("Fake company")
            .product("Serial port")
            .serial_number("TEST")
            .device_class(2)
            .build();

        let sio = Sio::new(c.device.SIO);
        // Set the pins to their default state
        let pins = hal::gpio::Pins::new(
            c.device.IO_BANK0,
            c.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut resets,
        );

        // Setup the IR stuff
        let ir_pin = pins.gpio12.into_floating_input();

        let blip = Blip::new(40_000);

        let mono = Monotonic::new(c.device.TIMER);
        tick::spawn().ok();
        (Shared {blip}, Local {usb_dev, serial, ir_pin, ts: 0 }, init::Monotonics(mono))
    }

    #[task(shared = [blip], local = [ir_pin, ts])]
    fn tick(cx: tick::Context) {
        let mut blip = cx.shared.blip;
        let ir_pin = cx.local.ir_pin;
        let ts = cx.local.ts;

        *ts += 1;

        let level = ir_pin.is_low().unwrap();

        blip.lock(|blip| {
            if let Some(reply) = blip.tick(*ts, level) {

            }
        });

        tick::spawn_after(25_u64.micros()).ok();
    }


    #[task(binds=USBCTRL_IRQ, local = [serial, usb_dev], shared = [blip])]
    fn on_usb(ctx: on_usb::Context) {
        let serial = ctx.local.serial;
        if !ctx.local.usb_dev.poll(&mut [serial]) {
            return;
        }
        let mut buf = [0u8; 64];
        match serial.read(&mut buf) {
            Ok(count) if count > 0 => {
                info!("Received: {}", core::str::from_utf8(&buf[..]).unwrap());

                if let Some(cmd) = cmd_from_buf(&data) {
                    rprintln!("Cmd: {:?}", cmd);
                    let reply = blip.handle_command(cmd);
                    serial_reply(serial, &reply);
                }
                /*
                buf.iter_mut().take(count).for_each(|b| {
                    b.make_ascii_uppercase();
                });
                // Echo back to the host
                let mut wr_ptr = &buf[..count];
                while !wr_ptr.is_empty() {
                    let _ = serial.write(wr_ptr).map(|len| {
                        wr_ptr = &wr_ptr[len..];
                    });
                }

                 */
            }
            _ => {}
        }
    }
}

fn serial_send(serial: &mut SerialPort<'static, UsbBus>,
    data: &[u8]
) {
    let count = data.len();
    let mut offset = 0;

    while offset < count {
        match serial.write(&data[offset..]) {
            Ok(sent) if sent > 0 => offset += sent,
            _ => {}
        }
    }
}


fn serial_reply(serial: &mut SerialPort<'static, UsbBus>, reply: &libblip::Reply) {
    let d: heapless::Vec<u8, 1024> = to_vec(&reply).unwrap();
    serial_send(serial, &d);
}


fn cmd_from_buf(buf: &[u8]) -> Option<libblip::Command> {
    match from_bytes::<libblip::Command>(buf) {
        Ok(cmd) => Some(cmd),
        Err(err) => {
            warn!("Cmd parse error: {}", err);
            None
        },
    }
}