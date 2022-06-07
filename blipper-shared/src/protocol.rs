use infrared::ProtocolId;
use infrared;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Command {
    Idle,
    Info,
    CaptureProtocol(u32),
    /// Start a capture
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
    pub pid: u8,
    pub addr: u16,
    pub cmd: u8,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Info {
    pub version: u32,
    /// Bitfield of transmitters
    pub transmitters: u32,
    /// Samplerate
    pub samplerate: u32,
}

/// Protocol Id
#[derive(Debug)]
pub struct Pid(infrared::ProtocolId);

impl Pid {
    pub fn as_u8(&self) -> u8 {
        self.0 as u8
    }
}

impl From<infrared::ProtocolId> for Pid {
    fn from(protocol_id: ProtocolId) -> Self {
        Pid(protocol_id)
    }
}

impl AsRef<infrared::ProtocolId> for Pid {
    fn as_ref(&self) -> &infrared::ProtocolId {
        &self.0
    }
}

impl TryFrom<&str> for Pid {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.as_ref() {
            "nec" => Ok(ProtocolId::Nec.into()),
            "n16" => Ok(ProtocolId::Nec16.into()),
            "nes" => Ok(ProtocolId::NecSamsung.into()),
            "apple" => Ok(ProtocolId::NecApple.into()),
            "rc5" => Ok(ProtocolId::Rc5.into()),
            "rc6" => Ok(ProtocolId::Rc6.into()),
            "sbp" => Ok(ProtocolId::Sbp.into()),
            _ => Err(()),
        }
    }
}
