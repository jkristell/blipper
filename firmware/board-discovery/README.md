

        /*
        let channels = (
            gpiod.pd12.into_alternate_af2(),
            gpiod.pd13.into_alternate_af2(),
            gpiod.pd14.into_alternate_af2(),
            gpiod.pd15.into_alternate_af2(),
        );


        let pwm = pwm::tim4(device.TIM4, 
            channels, 
            clocks, 
            20u32.khz());

        let (mut ch1, 
            mut ch2, 
            mut ch3, 
            mut ch4) = pwm;

        let max_duty = ch1.get_max_duty();
        ch1.set_duty(max_duty / 64);
        ch1.enable();

        let max_duty = ch2.get_max_duty();
        ch2.set_duty(max_duty / 32);
        ch2.enable();


        let max_duty = ch3.get_max_duty();
        ch3.set_duty(max_duty / 16);
        ch3.enable();

        let max_duty = ch4.get_max_duty();
        ch4.set_duty(max_duty / 128);
        ch4.enable();

         */

