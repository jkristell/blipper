use embedded_hal::PwmPin;

use blipper_protocol::{Command, Info, CaptureData, Reply};

use infrared::{
    sender::{
        {Sender, PwmPinSender},
        State as SenderState
    },
    protocols::{
        rc5::{Rc5Command, Rc5Sender},
        nec::{NecTransmitter, NecSamsungTransmitter, NecCommand}
    }
};

const NEC_ID: u8 = 1;
const NES_ID: u8 = 2;
const RC5_ID: u8 = 3;

const VERSION: u32 = 1;

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
        self.i = 0;
        self.edge = false;
        self.last_edge = 0;
    }

    pub fn sample(&mut self, edge: bool, ts: u32) -> Option<Reply> {

        if edge == self.edge {

            if self.i != 0
                && self.last_edge != 0 // TODO: Check if this can be removed
                && ts.wrapping_sub(self.last_edge) > self.timeout {

                let reply = capture_reply(self.samplerate, &self.buf[0..self.i]);
                self.reset();
                return Some(reply);
            }

            return None;
        }

        self.edge = edge;

        self.buf[self.i] = ts.wrapping_sub(self.last_edge) as u16;
        self.i += 1;
        self.last_edge = ts;

        if self.i == self.buf.len() {
            let reply = capture_reply(self.samplerate, &self.buf);
            self.reset();
            return Some(reply);
        }

        None
    }
}

pub struct Transmitters {
    nec: NecTransmitter,
    nes: NecSamsungTransmitter,
    rc5: Rc5Sender,
    active: u8,
}

impl Transmitters {
    fn new(samplerate: u32) -> Self {
        Self {
            nec: NecTransmitter::new(samplerate),
            nes: NecSamsungTransmitter::new(samplerate),
            rc5: Rc5Sender::new(samplerate),
            active: 0,
        }
    }

    pub fn load(&mut self, tid: u8, addr: u16, cmd: u8) {
        self.active = tid;

        match tid {
            NEC_ID => self.nec.load(NecCommand {
                addr,
                cmd,
            }),
            NES_ID => self.nes.load(NecCommand {
                addr,
                cmd,
            }),
            RC5_ID => self.rc5.load(Rc5Command::new(addr as u8, cmd, false)),
            _ => (),
        }
    }

    fn step<PWM: PwmPin<Duty = DUTY>, DUTY>(
        &mut self,
        sample: u32,
        pwm: &mut PWM,
    ) -> SenderState {
        match self.active {
            NEC_ID => self.nec.step_pwm(sample, pwm),
            NES_ID => self.nes.step_pwm(sample, pwm),
            RC5_ID => self.rc5.step_pwm(sample, pwm),
            _ => SenderState::Idle,
        }
    }
}

pub struct Blip {
    pub state: State,
    pub capturer: BlipCapturer,
    pub txers: Transmitters,
    pub samplerate: u32,
}

impl Blip {
    pub fn new(samplerate: u32) -> Self {
        Blip {
            state: State::Idle,
            capturer: BlipCapturer::new(samplerate),
            txers: Transmitters::new(samplerate),
            samplerate,
        }
    }

    fn irsend<D, PWM: PwmPin<Duty = D>>(&mut self, samplenum: u32, pwm: &mut PWM) -> bool {
        let state = self.txers.step(samplenum, pwm);
        match state {
            SenderState::Transmit(send) => send,
            SenderState::Idle | SenderState::Error => false,
        }
    }

    pub fn tick<D, PWM: PwmPin<Duty = D>>(&mut self, timestamp: u32, level: bool, pwm: &mut PWM) -> Option<Reply> {
        match self.state {
            State::Idle => None,
            State::IrSend => { self.irsend(timestamp, pwm); None}
            State::CaptureRaw => self.capturer.sample(level, timestamp)
        }
    }

    pub(crate) fn handle_command(&mut self, cmd: Command) -> Reply {
        match cmd {
            Command::Idle => {
                self.state = State::Idle;
                Reply::Ok
            }
            Command::Info => {
                Reply::Info {
                    info: Info {
                        version: 1,
                        transmitters: 0, //blip::ENABLED_TRANSMITTERS,
                    },
                }
            }
            Command::Capture => {
                self.capturer.reset();
                self.state = State::CaptureRaw;
                Reply::Ok
            }
            Command::CaptureProtocol(_id) => {
                rprintln!("CaptureProtocol not implemented");
                Reply::Ok
            }
            Command::RemoteControlSend(cmd) => {
                self.txers.load(cmd.txid, cmd.addr, cmd.cmd);
                self.state = State::IrSend;
                Reply::Ok
            }
        }
    }

}

fn capture_reply(samplerate: u32, buf: &[u16]) -> Reply {
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
