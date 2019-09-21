use common::{Reply, RawData};
use infrared::nec::{NecReceiver, NecTransmitter, NecType};
use infrared::rc6::Rc6Receiver;
use infrared::{Receiver, ReceiverState};
use infrared::trace::{TraceReceiver, TraceResult};
use heapless::{
    consts::*,
    Vec,
};

use crate::BlipperState;
use postcard::to_vec;

pub enum BlipperReceiver {
    Nec(NecReceiver<u32>),
    Rc6(Rc6Receiver),
    Trace(TraceReceiver),
}

pub enum BlipperTransmitter {
    Nec(NecTransmitter),
    Disabled,
}

pub struct BlipperBlip {
    pub state: BlipperState,
    pub receiver: BlipperReceiver,
    pub transmitter: BlipperTransmitter,
    pub samplerate: u32,
}

struct GenericRemoteCommand {
    addr: u32,
    cmd: u32,
}

impl BlipperBlip {

    pub fn new() -> Self {

        let samplerate = 40_000;
        let receiver = BlipperReceiver::Trace(TraceReceiver::new(samplerate));
        let transmitter = BlipperTransmitter::Disabled;

        BlipperBlip {
            state: BlipperState::Idle,
            receiver,
            transmitter,
            samplerate,
        }
    }

    pub fn tick(&mut self, edge: bool, ts: u32) -> Option<Reply> {

        let reply = match self.receiver {
            BlipperReceiver::Nec(ref mut n) => {
                n.event(edge, ts);
                None
            },
            BlipperReceiver::Rc6(ref mut r) => {
                r.event(edge, ts);
                None
            },
            BlipperReceiver::Trace(ref mut r) => {
                if let ReceiverState::Done(tr) = r.event(edge, ts) {
                   Some(traceresult_to_reply(tr))
                } else {
                    None
                }
            }
        };

        reply
    }

    pub fn reset(&mut self) {
        match self.receiver {
            BlipperReceiver::Trace(ref mut r) => r.reset(),
            _ => (),
        }
    }

}

fn traceresult_to_reply(tr: TraceResult) -> Reply {

    let mut data = RawData {
        len: tr.buf_len as u32,
        samplerate: 40_000,
        d0: [0; 32],
        d1: [0; 32],
        d2: [0; 32],
        d3: [0; 32],
    };

    data.d0.copy_from_slice(&tr.buf[0..32]);
    data.d1.copy_from_slice(&tr.buf[32..64]);
    data.d2.copy_from_slice(&tr.buf[64..96]);
    data.d3.copy_from_slice(&tr.buf[96..128]);

    let mut reply_cmd = Reply::CaptureRawData { rawdata: data };

    reply_cmd
}

