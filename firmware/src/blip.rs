use embedded_hal::PwmPin;

use infrared::{PeriodicReceiver};

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
    pub ts_last_cmd: u32,
    pub timeout: u32,
    pub samplerate: u32,
    pub buf: [u16; 128],
    pub i: usize,
    pub edge: bool,
    pub last_edge: u32,
}

impl BlipCapturer {
    pub fn new(samplerate: u32) -> Self {
        Self {
            ts_last_cmd: 0,
            timeout: samplerate / 10,
            samplerate,
            buf: [0; 128],
            i: 0,
            edge: false,
            last_edge: 0,
        }
    }

    pub fn reset(&mut self) {
        self.ts_last_cmd = 0;
    }

    pub fn sample(&mut self, edge: bool, ts: u32) -> Option<Reply> {

        if edge == self.edge {

            if self.i != 0
                && self.last_edge != 0
                && ts.wrapping_sub(self.last_edge) > self.timeout {

                let res = Some(traceresult_to_reply(self.samplerate,
                                                 &self.buf[0..self.i]));
                self.i = 0;
                return res;
            }

            return None;
        }

        self.edge = edge;

        self.buf[self.i] = ts.wrapping_sub(self.last_edge) as u16;
        self.i += 1;
        self.last_edge = ts;

        if self.i == self.buf.len() {

            self.i = 0;

            return Some(traceresult_to_reply(self.samplerate,
                                             &self.buf));
        }

        None

        /*
        if let Ok(Some(cmd)) = self.capture_receiver.poll(edge, ts) {
            let events = &cmd.edges;
            let n_egdes = cmd.n_edges;
            self.ts_last_cmd = ts;
        }

        // Check for timeout

        if self.ts_last_cmd != 0
            && ts.wrapping_sub(self.ts_last_cmd) > self.timeout
            && self.capture_receiver.recv.sm.n_edges > 0
        {
            // Timeout
            let events = self.capture_receiver.recv.sm.edges();
            res = Some(traceresult_to_reply(1_000_000, events));

            self.capture_receiver.reset();
            self.ts_last_cmd = ts;
        }

        res

         */
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
