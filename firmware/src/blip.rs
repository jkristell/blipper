use common::{Reply, RawData};
use infrared::prelude::*;
use infrared::logging::LoggingReceiver;
use infrared::nec::{NecSamsungTransmitter, NecCommand};
use infrared::rc5::{Rc5Transmitter, Rc5Command};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum State {
    Idle,
    CaptureRaw,
    IrSend,
}

pub struct Txers {
    nes: NecSamsungTransmitter,
    rc5: Rc5Transmitter,
    active: u8,
}

impl Txers {

    fn new(samplerate: u32) -> Self {

        // Remove in next infrared release
        let period: u32 = (1 * 1000) / (samplerate / 1000);

        Self {
            nes: NecSamsungTransmitter::new(period),
            rc5: Rc5Transmitter::new_for_samplerate(samplerate),
            active: 0,
        }
    }

    pub fn load(&mut self, tid: u8, addr: u16, cmd: u8) {

        self.active = tid;

        match tid {
            1 => self.nes.load(NecCommand { addr: addr as u8, cmd: cmd }),
            2 => self.rc5.load(Rc5Command::new(addr as u8, cmd, false)),
            _ => (),
        }
    }

    fn step(&mut self, sample: u32) -> TransmitterState {
        match self.active {
            1 => self.nes.step(sample),
            2 => self.rc5.step(sample),
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

    pub fn irsend(&mut self, samplenum: u32) -> bool {

        let state = self.txers.step(samplenum);
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

