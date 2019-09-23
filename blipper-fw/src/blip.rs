use common::{Reply, RawData};
use infrared::{Receiver, ReceiverState};
use infrared::trace::{TraceReceiver};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum State {
    Idle,
    CaptureRaw,
}

pub struct Blip {
    pub state: State,
    pub tracer: TraceReceiver,
    pub samplerate: u32,
}

impl Blip {
    pub fn new(samplerate: u32) -> Self {
        Blip {
            state: State::Idle,
            tracer: TraceReceiver::new(samplerate, 1000),
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
}

fn traceresult_to_reply(samplerate: u32, buf: &[u16]) -> Reply {

    let mut rawdata = RawData {
        samplerate,
        data: [[0; 32]; 4],
        len: buf.len() as u32,
    };

    rawdata.data[0].copy_from_slice(&buf[0..32]);
    rawdata.data[1].copy_from_slice(&buf[32..64]);
    rawdata.data[2].copy_from_slice(&buf[64..96]);
    rawdata.data[3].copy_from_slice(&buf[96..128]);

    Reply::CaptureRawData { rawdata }
}

