
use infrared::remotes::nec::{SpecialForMp3, SamsungTv, };
use infrared::remotes::{RemoteControl, StandardButton, DeviceType, RemoteControlCommand};
use infrared::remotes::rc5::CdPlayer;
use infrared::remotes::std::RemoteControlData;

use common::Protocol;
use infrared::rc5::Rc5Receiver;


pub fn create_remotes() -> Vec<RemoteControlData> {
    vec![
        RemoteControlData::construct::<CdPlayer>(),
        RemoteControlData::construct::<SamsungTv>(),
        RemoteControlData::construct::<SpecialForMp3>(),
    ]
}

