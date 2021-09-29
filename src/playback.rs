use std::{io, path::Path};

use infrared::{
    protocol::{
        {Nec, Nec16, NecSamsung},
        Rc5,
        Rc6,
    },
    Receiver,
    ProtocolId,
};

use crate::vcdutils::vcdfile_to_vec;
use blipper_shared::decoder::BlipperCommand;
use infrared::receiver::{Event, DefaultInput, DecoderStateMachine};

pub fn command(protocol: ProtocolId, path: &Path) -> io::Result<Vec<BlipperCommand>> {
    let (samplerate, v) = vcdfile_to_vec(path)?;
    let samplerate = samplerate as usize;

    Ok(match protocol {
        ProtocolId::Nec => {
            let mut recv: Receiver<Nec, Event, DefaultInput> = Receiver::new(samplerate, DefaultInput);
            play_vcd(&v, &mut recv)
                .into_iter()
                .map(BlipperCommand::Nec)
                .collect()
        }
        ProtocolId::Nec16 => {
            let mut recv: Receiver<Nec16, Event, DefaultInput> = Receiver::new(samplerate, DefaultInput);
            play_vcd(&v, &mut recv)
                .into_iter()
                .map(BlipperCommand::Nec16)
                .collect()
        }
        ProtocolId::NecSamsung => {
            let mut recv: Receiver<NecSamsung, Event, DefaultInput> = Receiver::new(samplerate, DefaultInput);
            play_vcd(&v, &mut recv)
                .into_iter()
                .map(BlipperCommand::Nes)
                .collect()
        }
        ProtocolId::Rc5 => {
            let mut recv: Receiver<Rc5, Event, DefaultInput> = Receiver::new(samplerate, DefaultInput);
            play_vcd(&v, &mut recv)
                .into_iter()
                .map(BlipperCommand::Rc5)
                .collect()
        }
        ProtocolId::Rc6 => {
            let mut recv: Receiver<Rc6, Event, DefaultInput> = Receiver::new(samplerate, DefaultInput);
            play_vcd(&v, &mut recv)
                .into_iter()
                .map(BlipperCommand::Rc6)
                .collect()
        }
        _ => {
            log::warn!("Unhandled protocol: {:?}", protocol);
            Vec::default()
        }
    })
}

pub fn play_vcd<SM: DecoderStateMachine>(
    vcdvec: &[(u64, bool)],
    recv: &mut Receiver<SM, Event, DefaultInput>,
) -> Vec<SM::Cmd> {
    use std::convert::TryFrom;

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

        if let Ok(Some(cmd)) = recv.event(dt as usize, value) {
            res.push(cmd);
        }
    }
    res
}
