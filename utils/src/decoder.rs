use infrared::{
    ProtocolId, Command, ReceiverStateMachine, ReceiverState,
    rc5::Rc5,
    rc6::Rc6,
    nec::*,
    sbp::*,
    remotes::std::RemoteControlData,
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
    rc5: Rc5,
    rc6: Rc6,
    nec: Nec,
    nes: NecSamsung,
    sbp: Sbp,
}

impl Decoder {

    pub fn new(samplerate: u32) -> Self {

        Self {
            rc5: Rc5::new(samplerate),
            rc6: Rc6::new(samplerate),
            nec: Nec::new(samplerate),
            nes: NecSamsung::new(samplerate),
            sbp: Sbp::new(samplerate),
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

            if let Some(cmd) = sample(&mut self.sbp, rising, t) {
                return Some(cmd);
            }
        }
        None
    }
}

fn sample<RECEIVER, CMD>(recv: &mut RECEIVER, edge: bool, t: u32) -> Option<DecodedButton>
where
    CMD: Command,
    RECEIVER: ReceiverStateMachine<Cmd=CMD>,
{
    match recv.event(edge, t) {
        ReceiverState::Done(cmd) => {

            recv.reset();

            return Some(DecodedButton::new(RECEIVER::ID,
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
fn sample_nec(recv: &mut Nec, edge: bool, t: u32) -> Option<DecodedButton>
{
    match recv.event(edge, t) {
        ReceiverState::Done(_neccmd) => {

            let bits = recv.bitbuf;
            let cmd;
            let proto;

            if NecStandard::verify_command(recv.bitbuf) {
                cmd = NecStandard::decode_command(bits);
                proto = NecStandard::PROTOCOL;
            }
            else if Nec16Variant::verify_command(recv.bitbuf) {
                cmd = Nec16Variant::decode_command(bits);
                proto = Nec16Variant::PROTOCOL;
            }
            else {
                cmd = NecStandard::decode_command(bits);
                proto = NecStandard::PROTOCOL;
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

