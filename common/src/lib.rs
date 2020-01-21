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
    CaptureProtocolReply {data: GenericRemote},
    Info {info: Info},
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct CaptureData {
    pub samplerate: u32,
    pub len: u32,
    pub data: [[u16; 32]; 4],
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct GenericRemote {
    pub addr: u16,
    pub cmd: u16,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct RemoteControlCmd {
    pub txid: u8,
    pub addr: u16,
    pub cmd: u8,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Info {
    pub version: u32,
    /// Bitfield of transmitters
    pub transmitters: u32,
}

