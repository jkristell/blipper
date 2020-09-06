use embedded_hal::PwmPin;

use infrared::{
    PolledReceiver,
    Capturing,
    //protocols::{
    //    capture::Capturing,
    //    nec::{NecCommand, NecSamsungTransmitter, NecTransmitter},
    //},
    //recv::{self, ReceiverSM},
    //rc5::{Rc5Command, Rc5Transmitter},
    //PwmTransmitter, ReceiverState, ReceiverStateMachine, Transmitter, TransmitterState,
};

use blipper_protocol::{CaptureData, Reply};

//const NEC_ID: u8 = ProtocolId::Nec as u8;
//const NES_ID: u8 = ProtocolId::NecSamsung as u8;
//const RC5_ID: u8 = ProtocolId::Rc5 as u8;
//#[allow(dead_code)]
//const RC6_ID: u8 = ProtocolId::Rc6 as u8;

//pub const ENABLED_TRANSMITTERS: u32 = 1 << NEC_ID | 1 << NES_ID | 1 << RC5_ID;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum State {
    Idle,
    CaptureRaw,
    IrSend,
}

pub struct BlipCapturer {
    pub capture_receiver: PolledReceiver<Capturing>,
    pub last_cmd: u32,
    pub timeout: u32,
    pub samplerate: u32,
}

impl BlipCapturer {
    pub fn new(samplerate: u32) -> Self {
        Self {
            capture_receiver: PolledReceiver::new(samplerate),
            last_cmd: 0,
            timeout: samplerate / 10,
            samplerate,
        }
    }

    pub fn reset(&mut self) {
        self.last_cmd = 0;
        self.capture_receiver.reset();
    }

    pub fn sample(&mut self, edge: bool, ts: u32) -> Option<Reply> {
        let mut res = None;

        if let Ok(Some(_cmd)) = self.capture_receiver.poll(edge, ts) {
            let events = self.capture_receiver.recv.sm.edges();
            res = Some(traceresult_to_reply(1_000_000, events));
            self.last_cmd = ts;
        }

        // Check for timeout

        if self.last_cmd != 0
            && ts.wrapping_sub(self.last_cmd) > self.timeout
            && self.capture_receiver.recv.sm.n_edges > 0
        {
            // Timeout
            let events = self.capture_receiver.recv.sm.edges();
            res = Some(traceresult_to_reply(1_000_000, events));

            self.capture_receiver.reset();
        }

        res
    }
}

/*
pub struct Transmitters {
    nec: NecTransmitter,
    nes: NecSamsungTransmitter,
    rc5: Rc5Transmitter,
    active: u8,
}

impl Transmitters {
    fn new(samplerate: u32) -> Self {
        Self {
            nec: NecTransmitter::new(samplerate),
            nes: NecSamsungTransmitter::new(samplerate),
            rc5: Rc5Transmitter::new(samplerate),
            active: 0,
        }
    }

    pub fn load(&mut self, tid: u8, addr: u16, cmd: u8) {
        self.active = tid;

        /*
        match tid {
            NEC_ID => self.nec.load(NecCommand {
                addr: addr,
                cmd: cmd,
            }),
            NES_ID => self.nes.load(NecCommand {
                addr: addr,
                cmd: cmd,
            }),
            RC5_ID => self.rc5.load(Rc5Command::new(addr as u8, cmd, false)),
            _ => (),
        }

         */
    }

    /*
    fn step<PWM: PwmPin<Duty = DUTY>, DUTY>(
        &mut self,
        sample: u32,
        pwm: &mut PWM,
    ) -> TransmitterState {
        match self.active {
            NEC_ID => self.nec.pwmstep(sample, pwm),
            NES_ID => self.nes.pwmstep(sample, pwm),
            RC5_ID => self.rc5.pwmstep(sample, pwm),
            _ => TransmitterState::Idle,
        }
    }
     */
}
 */

pub struct Blip {
    pub state: State,
    pub capturer: BlipCapturer,
    //pub txers: Transmitters,
    pub samplerate: u32,
}

impl Blip {
    pub fn new(samplerate: u32) -> Self {
        Blip {
            state: State::Idle,
            capturer: BlipCapturer::new(samplerate),
            //txers: Transmitters::new(samplerate),
            samplerate,
        }
    }

    /*
    fn irsend<D, PWM: PwmPin<Duty = D>>(&mut self, samplenum: u32, pwm: &mut PWM) -> bool {
        let state = self.txers.step(samplenum, pwm);
        match state {
            TransmitterState::Transmit(send) => send,
            TransmitterState::Idle | TransmitterState::Error => false,
        }
    }
     */

    pub fn tick<D, PWM: PwmPin<Duty = D>>(&mut self, timestamp: u32, level: bool, pwm: &mut PWM) -> Option<Reply> {
        match self.state {
            State::Idle => None,
            State::IrSend => None, //{ self.irsend(timestamp, pwm); None}
            State::CaptureRaw => self.capturer.sample(level, timestamp)
        }
    }
}

fn traceresult_to_reply(samplerate: u32, buf: &[u16]) -> Reply {
    let mut data = CaptureData {
        samplerate,
        bufs: [[0; 32]; 4],
        len: buf.len() as u32,
    };

    for i in 0..buf.len() {
        let idx = i % 32;
        let bufidx = i / 32;
        data.bufs[bufidx][idx] = buf[i];
    }

    Reply::CaptureReply { data }
}
