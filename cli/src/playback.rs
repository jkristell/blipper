use std::{io, path::Path};

use log::warn;

use infrared::{
    protocols::{
        nec::{Nec, NecSamsung},
        rc5::Rc5,
        rc6::Rc6,
    },
    Command, EventReceiver, ReceiverSM,
};

use crate::vcdutils::vcdfile_to_vec;
use blipper_utils::decoder::DecodedCommand;

pub fn command(name: &str, path: &Path) -> io::Result<Vec<DecodedCommand>> {
    let (samplerate, v) = vcdfile_to_vec(path)?;

    Ok(match name {
        "nes" => {
            let mut recv: EventReceiver<Nec<NecSamsung>> = EventReceiver::new(samplerate);
            play_vcd(&v, &mut recv)
        }
        "rc5" => {
            let mut recv: EventReceiver<Rc5> = EventReceiver::new(samplerate);
            play_vcd(&v, &mut recv)
        }
        "rc6" => {
            let mut recv: EventReceiver<Rc6> = EventReceiver::new(samplerate);
            play_vcd(&v, &mut recv)
        }
        _ => {
            warn!("Unknown protocol: {}", name);
            vec![]
        }
    })
}

pub fn play_vcd<SM: ReceiverSM>(vcdvec: &[(u64, bool)], recv: &mut EventReceiver<SM>) -> Vec<DecodedCommand>{
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

        if let Ok(Some(cmd)) = recv.edge_event(value, dt) {
            res.push(DecodedCommand {
                address: cmd.address(),
                command: cmd.data(),
                kind: cmd.protocol()
            })
        }
    }
    res
}
