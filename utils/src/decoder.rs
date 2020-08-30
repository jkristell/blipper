use infrared::{
    recv::{
        Receiver,
        State,
    },
    protocols::{
        //rc5::Rc5,
        //rc6::Rc6,
        nec::*,
        //sbp::*,
        //denon::Denon,
    },
    //remotes::std::RemoteControlData,
    Command
};
use infrared::recv::ReceiverSM;

/*
#[derive(Debug)]
pub struct DecodedButton {
    pub address: u32,
    pub command: u32,
    pub remotes: Vec<RemoteControlData>,
}

impl DecodedButton {
    fn new(protocol: ProtocolId, address: u32, command: u32) -> Self {
        Self {
            address,
            command,
            remotes: vec![]
        }
    }
}

 */

pub struct Decoder {
    //rc5: Rc5,
    //rc6: Rc6,
    nec: Receiver<Nec>,
    //nes: NecSamsung,
    //sbp: Sbp,
    //denon: Denon,
}

impl Decoder {

    pub fn new(samplerate: u32) -> Self {

        Self {
            //rc5: Rc5::new(samplerate),
            //rc6: Rc6::new(samplerate),
            nec: Receiver::with_samplerate(Nec::new(40_000), 40_000),
            //nes: NecSamsung::new(samplerate),
            //sbp: Sbp::new(samplerate),
            //denon: Denon::for_samplerate(samplerate),
        }
    }

    pub fn decode_data(&mut self, edges: &[u16]) {
        let mut t: u32 = 0;
        let mut rising = false;

        for dist in edges {
            t += u32::from(*dist);
            rising = !rising;

            /*
            if let Some(cmd) = sample(&mut self.rc5, rising, t) {
                return Some(cmd);
            }

            if let Some(cmd) = sample(&mut self.rc6, rising, t) {
                return Some(cmd);
            }

             */

            sample_nec(&mut self.nec, rising, t);
                //return Some(cmd);

            /*
            if let Some(cmd) = sample(&mut self.nes, rising, t) {
                return Some(cmd);
            }

            if let Some(cmd) = sample(&mut self.sbp, rising, t) {
                return Some(cmd);
            }

             */

            /*
            if let Some(cmd) = sample_denon(&mut self.denon, rising, t) {
                return Some(cmd);
            }
            */
        }
        //None
    }
}

/*
fn sample<RECEIVER, CMD>(recv: &mut RECEIVER, edge: bool, t: u32) -> Option<DecodedButton>
where
    CMD: Command,
    RECEIVER: Receiver<Cmd=CMD>,
{
    match recv.event(edge, t) {
        State::Done(cmd) => {
            recv.reset();
            return Some(DecodedButton::new(RECEIVER::ID,
                                           cmd.address().into(),
                                           cmd.data().into()))
        }
        State::Error(_err) => {
            recv.reset();
        }
        _ => {}
    }

    None
}

 */

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
fn sample_nec(recv: &mut Receiver<Nec>, edge: bool, t: u32) {

    if let Some(cmd) = recv.event(edge, t) {
        println!("cmd: {:?}", cmd);
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

