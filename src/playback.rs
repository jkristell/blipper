use std::{io, path::Path};

use blipper_shared::protocol::Pid;
use infrared::{
    protocol::{nec::NecCommand, Nec, Nec16, Rc5, Rc6, SamsungNec},
    receiver::{DecoderFactory, MultiReceiverCommand, ProtocolDecoder},
    Protocol, ProtocolId,
};

use crate::vcdutils::vcdfile_to_vec;

/*
pub fn try_all(path: &Path) -> io::Result<Vec<MultiReceiverCommand>> {
    let (samplerate, v) = vcdfile_to_vec(path)?;

    let mut nec: NecDecoder<u32> = Nec::decoder(samplerate);
    let mut apple: NecDecoder<u32, AppleNecCommand> = AppleNec::decoder(samplerate);

    let r = play_vcd(&v, &mut nec).into_iter()
        .chain( play_vcd(&v, &mut apple).into_iter())
        .collect();

    Ok(r)
}

 */

pub fn command(protocol: Pid, path: &Path) -> io::Result<Vec<MultiReceiverCommand>> {
    let (samplerate, v) = vcdfile_to_vec(path)?;

    Ok(match protocol.as_ref() {
        ProtocolId::Nec => {
            // NOTE: The default doesn't seem to resolve here
            let mut decoder = Nec::<NecCommand>::decoder(samplerate);
            play_vcd(&v, &mut decoder)
        }
        ProtocolId::Nec16 => {
            let mut decoder = Nec16::decoder(samplerate);
            play_vcd(&v, &mut decoder)
        }
        ProtocolId::NecSamsung => {
            let mut decoder = SamsungNec::decoder(samplerate);
            play_vcd(&v, &mut decoder)
        }
        ProtocolId::Rc5 => {
            let mut decoder = Rc5::decoder(samplerate);
            play_vcd(&v, &mut decoder)
        }
        ProtocolId::Rc6 => {
            let mut decoder = Rc6::decoder(samplerate);
            play_vcd(&v, &mut decoder)
        }
        _ => {
            log::warn!("Unhandled protocol: {:?}", protocol);
            Vec::default()
        }
    })
}

pub fn play_vcd<Decoder, Proto>(
    vcdvec: &[(u64, bool)],
    decoder: &mut Decoder,
) -> Vec<MultiReceiverCommand>
where
    Decoder: ProtocolDecoder<u32, Proto>,
    Proto: Protocol,
    Proto::Cmd: Into<MultiReceiverCommand>,
{
    let mut res = Vec::new();

    let iter = vcdvec
        .iter()
        .cloned()
        .map(|(t, v)| (u32::try_from(t).unwrap(), v));

    let mut prev = 0;
    for (t, value) in iter {
        let dt = t - prev;
        prev = t;
        //println!("value: {}, t = {}, dt = {}", value, t, dt);

        if let Ok(Some(cmd)) = decoder.event_total(value, dt) {
            res.push(cmd.into());
        }
    }
    res
}
