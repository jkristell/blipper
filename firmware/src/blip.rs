use common::{Reply, RawData, Protocol};
use infrared::prelude::*;
use infrared::logging::LoggingReceiver;
use infrared::nec::{NecTransmitter, NecSamsungTransmitter, NecCommand};
use infrared::rc5::{Rc5Transmitter, Rc5Command};
use embedded_hal::PwmPin;

const NEC_ID: u8 = Protocol::Nec as u8;
const NES_ID: u8 = Protocol::NecSamsung as u8;
const RC5_ID: u8 = Protocol::Rc5 as u8;
#[allow(dead_code)]
const RC6_ID: u8 = Protocol::Rc6 as u8;

pub const ENABLED_TRANSMITTERS: u32 = 1 << NEC_ID | 1 << NES_ID | 1 << RC5_ID;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum State {
    Idle,
    CaptureRaw,
    IrSend,
}

pub struct Txers {
    nec: NecTransmitter,
    nes: NecSamsungTransmitter,
    rc5: Rc5Transmitter,
    active: u8,
}

impl Txers {

    fn new(samplerate: u32) -> Self {

        // Remove in next infrared release
        let period: u32 = (1 * 1000) / (samplerate / 1000);

        Self {
            nec: NecTransmitter::new(period),
            nes: NecSamsungTransmitter::new(period),
            rc5: Rc5Transmitter::new_for_samplerate(samplerate),
            active: 0,
        }
    }

    pub fn load(&mut self, tid: u8, addr: u16, cmd: u8) {

        self.active = tid;

        match tid {
            1 => self.nes.load(NecCommand { addr: addr as u8, cmd: cmd }),
            2 => self.nes.load(NecCommand { addr: addr as u8, cmd: cmd }),
            3 => self.rc5.load(Rc5Command::new(addr as u8, cmd, false)),
            _ => (),
        }
    }

    fn step<PWM: PwmPin<Duty=DUTY>, DUTY>(&mut self, sample: u32, pwm: &mut PWM) -> TransmitterState {
        match self.active {
            1 => self.nec.pwmstep(sample, pwm),
            2 => self.nes.pwmstep(sample, pwm),
            3 => self.rc5.pwmstep(sample, pwm),
            _ => TransmitterState::Idle,
        }
    }
}

pub struct Blip {
    pub state: State,
    pub tracer: LoggingReceiver,
    pub txers: Txers,
    pub samplerate: u32,
}

impl Blip {
    pub fn new(samplerate: u32) -> Self {
        Blip {
            state: State::Idle,
            tracer: LoggingReceiver::new(samplerate, 1000),
            txers: Txers::new(samplerate),
            samplerate,
        }
    }

    pub fn sample(&mut self, edge: bool, ts: u32) -> Option<Reply> {
        if let ReceiverState::Done(()) = self.tracer.sample(edge, ts) {
            return Some(traceresult_to_reply(self.samplerate, self.tracer.data()));
        }
        None
    }

    pub fn reset(&mut self) {
        self.tracer.reset();
    }

    pub fn irsend<D, PWM: PwmPin<Duty=D>>(&mut self, samplenum: u32, pwm: &mut PWM) -> bool {

        let state = self.txers.step(samplenum, pwm);
        match state {
            TransmitterState::Idle => false,
            TransmitterState::Transmit(send) => send,
            TransmitterState::Err => false,
        }
    }
}

fn traceresult_to_reply(samplerate: u32, buf: &[u16]) -> Reply {

    let mut rawdata = RawData {
        samplerate,
        data: [[0; 32]; 4],
        len: buf.len() as u32,
    };

    for i in 0..buf.len() {
        let idx = i % 32;
        let bufidx = i / 32;
        rawdata.data[bufidx][idx] = buf[i];
    }

    Reply::CaptureRawData { rawdata }
}

