use std::{io, path::Path};

use infrared::{recv::EventReceiver, ProtocolId, recv::ReceiverSM, protocols::{nec::{Nec, Nec16, NecSamsung}, rc5::Rc5, rc6::Rc6}};

use crate::vcdutils::vcdfile_to_vec;
use blipper_shared::decoder::{BlipperCommand};

pub fn command(protocol: ProtocolId, path: &Path) -> io::Result<Vec<BlipperCommand>> {
    let (samplerate, v) = vcdfile_to_vec(path)?;

    Ok(match protocol {
        ProtocolId::Nec => {
            let mut recv: EventReceiver<Nec> = EventReceiver::new(samplerate);
            play_vcd(&v, &mut recv).into_iter().map(BlipperCommand::Nec).collect()
        }
        ProtocolId::Nec16 => {
            let mut recv: EventReceiver<Nec16> = EventReceiver::new(samplerate);
            play_vcd(&v, &mut recv).into_iter().map(BlipperCommand::Nec16).collect()
        }
        ProtocolId::NecSamsung => {
            let mut recv: EventReceiver<NecSamsung> = EventReceiver::new(samplerate);
            play_vcd(&v, &mut recv).into_iter().map(BlipperCommand::Nes).collect()
        }
        ProtocolId::Rc5 => {
            let mut recv: EventReceiver<Rc5> = EventReceiver::new(samplerate);
            play_vcd(&v, &mut recv).into_iter().map(BlipperCommand::Rc5).collect()
        }
        ProtocolId::Rc6 => {
            let mut recv: EventReceiver<Rc6> = EventReceiver::new(samplerate);
            play_vcd(&v, &mut recv).into_iter().map(BlipperCommand::Rc6).collect()
        }
        _ => {
            log::warn!("Unhandled protocol: {:?}", protocol);
            Vec::default()
        }
    })

}

pub fn play_vcd<SM: ReceiverSM>(vcdvec: &[(u64, bool)], recv: &mut EventReceiver<SM>) -> Vec<SM::Cmd>{
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
            res.push(cmd);
        }
    }
    res
}
