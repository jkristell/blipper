
use infrared_remotes::nec::{SpecialForMp3, SamsungTv, };
use infrared_remotes::{RemoteControl, StandardButton, DeviceType, RemoteControlCommand};
use infrared_remotes::rc5::CdPlayer;

use common::Protocol;
use infrared_remotes::extra::RemoteControlData;
use infrared::rc5::Rc5Receiver;


pub fn create_remotes() -> Vec<RemoteControlData> {
    vec![
        RemoteControlData::construct::<CdPlayer>(),
        RemoteControlData::construct::<SamsungTv>(),
        RemoteControlData::construct::<SpecialForMp3>(),
    ]
}

