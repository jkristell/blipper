#![no_main]
#![no_std]
#![allow(deprecated)]

use panic_halt as _;
use rtfm::app;

use stm32f1xx_hal::{
    gpio::{gpiob::PB8, Floating, Input},
    prelude::*,
    stm32::{TIM2},
    timer::{self, Timer},
};



use infrared::{
    nec::{NecType, NecReceiver},
    Receiver,
};

#[app(device = stm32f1xx_hal::stm32)]
const APP: () = {

    static mut TIMER_MS: Timer<TIM2> = ();
    static mut RECEIVER: NecReceiver<u32> = ();
    static mut IRPIN: PB8<Input<Floating>> = ();

    #[init]
    fn init() -> init::LateResources {

        let mut flash = device.FLASH.constrain();
        let mut rcc = device.RCC.constrain();

        let clocks = rcc.cfgr
            .use_hse(8.mhz())
            .sysclk(48.mhz())
            .pclk1(24.mhz())
            .freeze(&mut flash.acr);

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
        let receiver = NecReceiver::new(NecType::Standard, 20_000);

        init::LateResources {
            TIMER_MS: timer_ms,
            RECEIVER: receiver,
            IRPIN: irpin,
        }
    }

    #[interrupt(
        priority = 2,
        resources = [TIMER_MS, RECEIVER, IRPIN],
    )]
    fn TIM4() {
        static mut TS: u32 = 0;
        // Active low
        let rising = resources.IRPIN.is_low();
        // Ack the timer interrupt
        resources.TIMER_MS.clear_update_interrupt_flag();
        // Step the receivers state machine
        resources.RECEIVER.event(rising, *TS);
        // Update our timestamp
        *TS = TS.wrapping_add(1);
    }
};
