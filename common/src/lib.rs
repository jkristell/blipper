#![no_std]

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Command {
    Idle,
    CaptureRaw,
    /*
    CaptureProtocol,
    CaptureRemote,
    ActAsremote(u32),
    GetInfo,
    */
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Reply<'a> {
    Ok,
    CaptureRawHeader {samplerate: u32},
    CaptureRawData {data: &'a [u8]},
    Info {info: Info},
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Info {
    version: u32,
}

