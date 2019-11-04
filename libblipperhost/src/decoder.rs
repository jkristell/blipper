use infrared::ProtocolId;
use infrared::rc5::Rc5Receiver;
use infrared::rc6::Rc6Receiver;
use infrared::nec::*;
use infrared::prelude::*;

use infrared::remotes::{
        std::RemoteControlData,
        RemoteControlCommand,
};


#[derive(Debug)]
pub struct DecodedButton {
    pub protocol: ProtocolId,
    pub address: u16,
    pub command: u8,
    pub remotes: Vec<RemoteControlData>,
}

impl DecodedButton {
    fn new(protocol: ProtocolId, address: u16, command: u8) -> Self {
        Self {
            protocol,
            address,
            command,
            remotes: vec![]
        }
    }
}


pub struct Decoder {
    rc5: Rc5Receiver,
    rc6: Rc6Receiver,
    nec: NecReceiver,
    nes: NecSamsungReceiver,
    n16: Nec16Receiver,
}

impl Decoder {

    pub fn new(samplerate: u32) -> Self {

        Self {
            rc5: Rc5Receiver::new(samplerate),
            rc6: Rc6Receiver::new(samplerate),
            nec: NecReceiver::new(samplerate),
            nes: NecSamsungReceiver::new(samplerate),
            n16: Nec16Receiver::new(samplerate),
        }
    }

    pub fn decode_data(&mut self, edges: &[u16]) -> Option<DecodedButton> {
        let mut t: u32 = 0;
        let mut rising = false;

        for dist in edges {
            t += u32::from(*dist);
            rising = !rising;

            if let Some(cmd) = sample(&mut self.rc5, rising, t) {
                return Some(cmd);
            }

            if let Some(cmd) = sample(&mut self.rc6, rising, t) {
                return Some(cmd);
            }

            if let Some(cmd) = sample_nec(&mut self.nec, rising, t) {
                return Some(cmd);
            }

            if let Some(cmd) = sample(&mut self.nes, rising, t) {
                return Some(cmd);
            }
        }
        None
    }
}



fn sample<RECEIVER, CMD, ERR>(recv: &mut RECEIVER, edge: bool, t: u32) -> Option<DecodedButton>
where
    CMD: RemoteControlCommand,
    RECEIVER: infrared::Receiver<Cmd=CMD, Err=ERR>,
{
    match recv.sample(edge, t) {
        ReceiverState::Done(cmd) => {

            recv.reset();

            return Some(DecodedButton::new(RECEIVER::PROTOCOL_ID,
                                           cmd.address(),
                                           cmd.command()))
        }
        ReceiverState::Error(_err) => {
            recv.reset();
        }
        _ => {}
    }

    None
}

// Specialization for NEC
fn sample_nec(recv: &mut NecReceiver, edge: bool, t: u32) -> Option<DecodedButton>
{
    match recv.sample(edge, t) {
        ReceiverState::Done(neccmd) => {

            let bits = recv.bitbuf;
            let cmd;
            let proto;

            if StandardType::verify_command(recv.bitbuf) {
                cmd = StandardType::decode_command(bits);
                proto = StandardType::PROTOCOL;
            }
            else if Nec16Type::verify_command(recv.bitbuf) {
                cmd = Nec16Type::decode_command(bits);
                proto = Nec16Type::PROTOCOL;
            }
            else {
                cmd = StandardType::decode_command(bits);
                proto = StandardType::PROTOCOL;
            }

            recv.reset();

            return Some(DecodedButton::new(proto,
                                           cmd.address(),
                                           cmd.command()))
        }
        ReceiverState::Error(_err) => {
            recv.reset();
        }
        _ => {}
    }

    None
}

