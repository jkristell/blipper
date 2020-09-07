use infrared::{protocols::nec::*, Command, Receiver, ReceiverSM, Rc5, Nec, ReceiverKind, Rc6, Sbp};

#[derive(Debug)]
pub struct DecodedButton {
    pub address: u32,
    pub command: u32,
    pub kind: ReceiverKind,
}

pub struct Decoder {
    rc5: Receiver<Rc5>,
    rc6: Receiver<Rc6>,
    nec: Receiver<Nec>,
    nes: Receiver<NecSamsung>,
    sbp: Receiver<Sbp>,
    //denon: Denon,
}

impl Decoder {
    pub fn new(samplerate: u32) -> Self {
        Self {
            rc5: Receiver::new(samplerate),
            rc6: Receiver::new(samplerate),
            nec: Receiver::new(samplerate),
            nes: Receiver::new(samplerate),
            sbp: Receiver::new(samplerate),
        }
    }

    pub fn decode_data(&mut self, edges: &[u16]) -> Vec<DecodedButton> {
        let mut t: u32 = 0;
        let mut rising = false;

        let mut res = Vec::new();

        for dist in edges {
            t += u32::from(*dist);
            rising = !rising;

            if let Some(cmd) = sample(&mut self.rc5, rising, t) {
                res.push(cmd);
            }
            if let Some(cmd) = sample(&mut self.rc6, rising, t) {
                res.push(cmd);
            }
            if let Some(cmd) = sample_nec(&mut self.nec, rising, t) {
                res.push(cmd);
            }
            if let Some(cmd) = sample(&mut self.nes, rising, t) {
                res.push(cmd);
            }
            if let Some(cmd) = sample(&mut self.sbp, rising, t) {
                res.push(cmd);
            }
        }
        res
    }
}

fn sample<SM, CMD>(recv: &mut Receiver<SM>, edge: bool, t: u32) -> Option<DecodedButton>
where
    CMD: Command,
    SM: ReceiverSM<Cmd = CMD> + Default,
{
    let r = recv.edge_event(edge, t);

    if let Ok(Some(cmd)) = r {
        return Some(DecodedButton {
            address: cmd.address(),
            command: cmd.data(),
            kind: SM::KIND,
        })
    }

    None
}


/*
fn sample_denon(recv: &mut Denon, edge: bool, t: u32) -> Option<DecodedButton>{

    match recv.event(edge, t) {
        State::Done(cmd) => {
            recv.reset();

            println!("Denon: {:X} {:#b}", cmd.raw, cmd.raw);

            return Some(DecodedButton::new(Denon::ID,
                                           cmd.address(),
                                           cmd.command()));
        }
        ReceiverState::Error(_err) => {
            recv.reset();
        }
        _ => {}
    }

    None
}
*/

// Specialization for NEC
fn sample_nec(recv: &mut Receiver<Nec>, edge: bool, t: u32) -> Option<DecodedButton> {
    if let Ok(Some(cmd)) = recv.edge_event(edge, t) {
        println!("cmd: {:?}", cmd);
        Some(DecodedButton {
            command: cmd.data(),
            address: cmd.address(),
            kind: ReceiverKind::Nec,
        })
    } else {
        None
    }

    /*
    match recv.event(edge, t) {
        State::Done => {

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
                                           cmd.address().into(),
                                           cmd.data().into()))
        }
        State::Error(_err) => {
            recv.reset();
        }
        _ => {}
    }
     */
}
