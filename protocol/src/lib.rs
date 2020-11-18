#![no_std]

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Command {
    Idle,
    Info,
    CaptureProtocol(u32),
    Capture,
    RemoteControlSend(RemoteControlCmd),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Reply {
    Ok,
    CaptureReply { data: CaptureData },
    CaptureProtocolReply { data: GenericRemote },
    Info { info: Info },
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct CaptureData {
    pub samplerate: u32,
    pub len: u32,
    pub bufs: [[u16; 32]; 4],
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct GenericRemote {
    pub addr: u16,
    pub cmd: u16,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct RemoteControlCmd {
    pub pid: ProtocolId,
    pub addr: u16,
    pub cmd: u8,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Info {
    pub version: u32,
    /// Bitfield of transmitters
    pub transmitters: u32,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum ProtocolId {
    Nec = 1,
    NecSamsung = 2,
    Rc5 = 5,
}

impl From<u32> for ProtocolId {
    fn from(v: u32) -> Self {
        match v {
            1 => ProtocolId::Nec,
            2 => ProtocolId::NecSamsung,
            5 => ProtocolId::Rc5,
            _ => ProtocolId::Nec,
        }
    }
}
