use infrared::{protocols::*, Command, ReceiverSM, Protocol, BufferedReceiver};

#[derive(Debug)]
pub struct DecodedCommand {
    pub address: u32,
    pub command: u32,
    pub kind: Protocol,
}

pub struct Decoders;

impl Decoders {

    pub fn decode_data(&mut self, edges: &[u16], samplerate: u32) -> Vec<DecodedCommand> {
        let edges = edges.iter().map(|v| u32::from(*v)).collect::<Vec<_>>();

        let mut rc5: BufferedReceiver<Rc5> = BufferedReceiver::new(&edges, samplerate);
        let mut rc6: BufferedReceiver<Rc6> = BufferedReceiver::new(&edges, samplerate);
        let mut nec: BufferedReceiver<Nec> = BufferedReceiver::new(&edges, samplerate);
        let mut nes: BufferedReceiver<NecSamsung> = BufferedReceiver::new(&edges, samplerate);
        let mut sbp: BufferedReceiver<Sbp> = BufferedReceiver::new(&edges, samplerate);

        to_cmd(&mut rc5)
            .chain(to_cmd(&mut rc6))
            .chain(to_cmd(&mut nec))
            .chain(to_cmd(&mut nes))
            .chain(to_cmd(&mut sbp))
            .collect()
    }
}

fn to_cmd<'a, SM, CMD>(recv: &'a mut BufferedReceiver<'a, SM>) -> impl Iterator<Item=DecodedCommand> + 'a
    where
        CMD: Command,
        SM: ReceiverSM<Cmd = CMD>,
{
    recv
        .map(|cmd|
            DecodedCommand {
                address: cmd.address(),
                command: cmd.data(),
                kind: cmd.protocol(),
            }
        )
}

/*

fn sample<SM, CMD>(recv: &mut EventReceiver<SM>, edge: bool, t: u32) -> Option<DecodedButton>
where
    CMD: Command,
    SM: ReceiverSM<Cmd = CMD> + Default,
{
    let r = recv.edge_event(edge, t);

    if let Ok(Some(cmd)) = r {
        return Some(DecodedButton {
            address: cmd.address(),
            command: cmd.data(),
            kind: cmd.protocol(),
        })
    }

    None
}


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

// Specialization for NEC
fn sample_nec(recv: &mut EventReceiver<Nec>, edge: bool, t: u32) -> Option<DecodedButton> {
    if let Ok(Some(cmd)) = recv.edge_event(edge, t) {
        println!("cmd: {:?}", cmd);
        Some(DecodedButton {
            command: cmd.data(),
            address: cmd.address(),
            kind: cmd.protocol(),
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
*/
