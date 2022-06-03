use infrared::{Receiver, protocol::{
    {Nec, Nec16, AppleNec, Rc5, Rc6, Sbp},
}, Protocol};
use infrared::protocol::nec::{AppleNecCommand, Nec16Command, NecCommand, SamsungNecCommand};
use infrared::protocol::rc5::Command as Rc5Command;
use infrared::protocol::rc6::Rc6Command;
use infrared::protocol::{Capture, SamsungNec};
use infrared::protocol::nec::decoder::NecDecoder;
use infrared::protocol::rc5::decoder::Rc5Decoder;
use infrared::protocol::sbp::SbpCommand;
use infrared::receiver::{DecoderFactory, MultiReceiverCommand, ProtocolDecoder};

pub struct Decoders {
    nec: NecDecoder<u32>,
    apple: NecDecoder<u32, AppleNecCommand>,
    samsung: NecDecoder<u32, SamsungNecCommand>,
    rc5: Rc5Decoder<u32>,
}

impl Decoders {

    pub fn new(samplerate: u32) -> Self {
        Decoders {
            nec: Nec::decoder(samplerate),
            apple: AppleNec::decoder(samplerate),
            samsung: SamsungNec::decoder(samplerate),
            rc5: Rc5::decoder(samplerate),
        }
    }

    pub fn run(&mut self, edges: &[u16], samplerate: u32) -> Vec<MultiReceiverCommand> {

        let v = edges.iter().map(|v| *v as u32).collect::<Vec<_>>();

        let r = run_decoder(&v, &mut self.nec).into_iter()
            .chain( run_decoder(&v, &mut self.apple).into_iter())
            .chain( run_decoder(&v, &mut self.samsung).into_iter())
            .collect();

        return r;
    }
}

pub fn run_decoder<Decoder, Proto>(
    vcdvec: &[u32],
    decoder: &mut Decoder,
) -> Vec<MultiReceiverCommand>
    where
        Decoder: ProtocolDecoder<u32, Proto>,
        Proto: Protocol,
        Proto::Cmd: Into<MultiReceiverCommand>,
{
    let mut res = Vec::new();

    let mut prev = 0;
    let mut edge = false;
    for t in vcdvec {
        let dt = t - prev;
        prev = *t;
        edge = !edge;
        //println!("value: {}, t = {}, dt = {}", value, t, dt);

        if let Ok(Some(cmd)) = decoder.event_total(edge, dt) {
            res.push(cmd.into());
        }
    }
    res
}
