use infrared::{
    protocols::{
        {Nec, Nec16, NecApple, NecSamsung, Rc5, Rc6, Sbp},
        nec::{Nec16Command, NecAppleCommand, NecCommand, NecRawCommand, NecSamsungCommand},
        rc5::Rc5Command,
        rc6::Rc6Command,
        sbp::SbpCommand,
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
        let nec: BufferReceiver<Nec> = BufferReceiver::with_values(&edges, samplerate);
        let nes: BufferReceiver<NecSamsung> = BufferReceiver::with_values(&edges, samplerate);
        let nec16: BufferReceiver<Nec16> = BufferReceiver::with_values(&edges, samplerate);
        let nec_apple: BufferReceiver<NecApple> = BufferReceiver::with_values(&edges, samplerate);

        let rc5: BufferReceiver<Rc5> = BufferReceiver::with_values(&edges, samplerate);
        let rc6: BufferReceiver<Rc6> = BufferReceiver::with_values(&edges, samplerate);
        let sbp: BufferReceiver<Sbp> = BufferReceiver::with_values(&edges, samplerate);

        nec.iter().map(BlipperCommand::Nec)
            .chain(nes.iter().map(BlipperCommand::Nes))
            .chain(nec16.iter().map(BlipperCommand::Nec16))
            .chain(nec_apple.iter().map(BlipperCommand::NecApple))

            .chain(rc5.iter().map(BlipperCommand::Rc5))
            .chain(rc6.iter().map(BlipperCommand::Rc6))
            .chain(sbp.iter().map(BlipperCommand::Sbp))
            .collect()
    }
}
