#![no_std]

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Command {
    Idle,
    CaptureRaw,
    SetSampleRate(u32),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Reply {
    Ok,
    CaptureRawHeader {samplerate: u32},
    CaptureRawData {rawdata: RawData},
    CaptureRemote {addr: u32, cmd: u32},
    Info {info: Info},
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct RawData {
    pub len: u32,
    pub samplerate: u32,
    pub data: [[u16; 32]; 4],
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Info {
    version: u32,
}

