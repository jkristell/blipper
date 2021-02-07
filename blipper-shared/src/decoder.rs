use infrared::{
    protocols::{
        nec::{Nec16Command, NecAppleCommand, NecCommand, NecRawCommand, NecSamsungCommand},
        rc5::Rc5Command,
        rc6::Rc6Command,
        sbp::SbpCommand,
        {Nec, Nec16, NecApple, NecSamsung, Rc5, Rc6, Sbp},
    },
    recv::BufferReceiver,
};

#[derive(Debug)]
pub enum BlipperCommand {
    Nec(NecCommand),
    Nec16(Nec16Command),
    Nes(NecSamsungCommand),
    NecApple(NecAppleCommand),
    NecRaw(NecRawCommand),

    Rc5(Rc5Command),
    Rc6(Rc6Command),

    Sbp(SbpCommand),
}

pub struct Decoders;

impl Decoders {
    pub fn run(&mut self, edges: &[u16], samplerate: u32) -> Vec<BlipperCommand> {
        let receiver = BufferReceiver::new(&edges, samplerate);

        receiver
            .iter::<Nec>().map(BlipperCommand::Nec)
            .chain(receiver.iter::<NecSamsung>().map(BlipperCommand::Nes))
            .chain(receiver.iter::<Nec16>().map(BlipperCommand::Nec16))
            .chain(receiver.iter::<NecApple>().map(BlipperCommand::NecApple))
            .chain(receiver.iter::<Rc5>().map(BlipperCommand::Rc5))
            .chain(receiver.iter::<Rc6>().map(BlipperCommand::Rc6))
            .chain(receiver.iter::<Sbp>().map(BlipperCommand::Sbp))
            .collect()
    }
}
