use common::{Reply, RawData, GenericRemote};
use infrared::nec::{NecReceiver, NecTransmitter, NecType, NecCommand};
use infrared::rc5::{Rc5Receiver, Rc5Command};
use infrared::rc6::Rc6Receiver;
use infrared::{Receiver, ReceiverState};
use infrared::trace::{TraceReceiver, TraceResult};

use crate::BlipperState;

pub enum BlipperReceiver {
    Trace(TraceReceiver),
    Nec(NecReceiver<u32>),
    Rc5(Rc5Receiver),
    Rc6(Rc6Receiver),
}

pub enum BlipperTransmitter {
    Nec(NecTransmitter),
    Disabled,
}

pub struct BlipperBlip {
    pub state: BlipperState,
    pub receiver: BlipperReceiver,
    pub transmitter: BlipperTransmitter,
    pub tracer: TraceReceiver,
    pub samplerate: u32,
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
            tracer: TraceReceiver::new(samplerate),
            samplerate,
        }
    }

    pub fn select_receiver(&mut self, id: u32) {
/*
        self.receiver = match id {
            1 => BlipperReceiver::Nec(NecReceiver::new(NecType::Standard, self.samplerate)),
            2 => BlipperReceiver::Rc5(Rc5Receiver::new(self.samplerate)),
            3 => BlipperReceiver::Rc6(Rc6Receiver::new(self.samplerate)),
            _ => BlipperReceiver::Trace(TraceReceiver::new(self.samplerate)),
        }
        */
    }

    pub fn tick(&mut self, edge: bool, ts: u32) -> Option<Reply> {

        if let ReceiverState::Done(tr) = self.tracer.event(edge, ts) {
            Some(traceresult_to_reply(tr))
        } else {
            None
        }

/*

        let reply = match self.receiver {
            BlipperReceiver::Nec(ref mut r) => {
                match r.event(edge, ts) {
                    ReceiverState::Done(cmd) => Some(nec_to_reply(cmd)),
                    ReceiverState::Err(_err) => {
                        r.reset();
                        None
                    }
                    _ => None
                }
            }
            BlipperReceiver::Rc5(ref mut r) => {

                match r.event(edge, ts) {
                    ReceiverState::Done(cmd) => {
                        hprintln!("done").unwrap();
                        Some(rc5_to_reply(cmd))
                    },
                    ReceiverState::Err(_err) => {
                        r.reset();
                        None
                    }
                    _ => None
                }
            }
            BlipperReceiver::Rc6(ref mut r) => {
                r.event(edge, ts);
                None
            }
            BlipperReceiver::Trace(ref mut r) => {
                if let ReceiverState::Done(tr) = r.event(edge, ts) {
                   Some(traceresult_to_reply(tr))
                } else {
                    None
                }
            }
        };

        reply
        */
    }

    pub fn reset(&mut self) {
        self.tracer.reset();
        /*
        match self.receiver {
            BlipperReceiver::Trace(ref mut r) => r.reset(),
            BlipperReceiver::Nec(ref mut r) => r.reset(),
            BlipperReceiver::Rc5(ref mut r) => r.reset(),
            BlipperReceiver::Rc6(ref mut r) => r.reset(),
        }
        */
    }
}

/*
fn rc5_to_reply(rc5cmd: Rc5Command) -> Reply {

    let data = GenericRemote {
        addr: rc5cmd.addr as u16,
        cmd: rc5cmd.cmd as u16,
    };

    Reply::ProtocolData {
        data,
    }
}

fn nec_to_reply(neccmd: NecCommand<u32>) -> Reply {

    let (addr, cmd) = if let NecCommand::Payload(raw) = neccmd {
        let addr = (raw & 0xFF) as u16;
        let cmd = ((raw >> 16) & 0xFF) as u16;
        (addr, cmd)
    } else {
        (0xFFFF, 0xFFFF)
    };

    let data = GenericRemote {
        addr,
        cmd,
    };

    Reply::ProtocolData {data}
}
*/
fn traceresult_to_reply(tr: TraceResult) -> Reply {

    let mut data = RawData {
        len: tr.buf_len as u32,
        samplerate: 40_000,
        data: [[0; 32]; 4],
    };

    data.data[0].copy_from_slice(&tr.buf[0..32]);
    data.data[1].copy_from_slice(&tr.buf[32..64]);
    data.data[2].copy_from_slice(&tr.buf[64..96]);
    data.data[3].copy_from_slice(&tr.buf[96..128]);

    Reply::CaptureRawData { rawdata: data }
}

