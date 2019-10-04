
use infrared::nec::remotes::{SpecialForMp3, SamsungTv, };
use infrared::remotecontrol::{RemoteControl, StandardButton, DeviceType, RemoteControlCommand};
use infrared::rc5::remotes::CdPlayer;

use common::Protocol;


#[derive(Debug)]
pub struct RemoteControlData {
    pub model: String,
    pub addr: u16,
    pub protocol: Protocol,
    pub dtype: DeviceType,
    pub mapping: Vec<(u8, StandardButton)>,
}

fn trait_to_data<'a, CMD: RemoteControlCommand, REMOTE: RemoteControl<'a, CMD, Button=StandardButton>>(remote: REMOTE,
    protocol: Protocol) -> RemoteControlData {

    let mapping: Vec<_> = (0..=255)
        .filter_map(|cmdid| Some((cmdid, remote.decode_cmdid(cmdid)?) ))
        .collect();

    RemoteControlData {
        addr: REMOTE::ADDR,
        model: REMOTE::MODEL.to_string(),
        dtype: REMOTE::DEVICE,
        protocol,
        mapping,
    }
}

pub fn create_remotes() -> Vec<RemoteControlData>{
    let mut res = Vec::new();
    let rem = trait_to_data(SamsungTv, Protocol::NecSamsung);
    res.push(rem);
    let rem = trait_to_data(SpecialForMp3, Protocol::Nec);
    res.push(rem);
    let rem = trait_to_data(CdPlayer, Protocol::Rc5);
    res.push(rem);
    res
}

