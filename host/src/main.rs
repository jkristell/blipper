use std::io;
use std::path::{PathBuf};
use structopt;
use structopt::StructOpt;

//use log::{error, info};

mod blippervcd;
mod capture;
mod decode;
mod irsend;

//use link::SerialLink;
use libblipperhost::{
    SerialLink,
};
use infrared::ProtocolId;
use crate::blippervcd::{play_rc5, play_rc6, play_nec};

#[derive(Debug, StructOpt)]
#[structopt(name = "Blipper", about = "Blipper cli tool")]
struct Opt {
    /// Serial Device. Defaults to /dev/ttyACM0
    #[structopt(long = "device", parse(from_os_str))]
    serial: Option<PathBuf>,
    #[structopt(short, long)]
    debug: bool,
    #[structopt(subcommand)]
    cmd: CliCommand,
}

#[derive(StructOpt, Debug)]
enum CliCommand {
    /// Decode in realtime with protocol
    Decode {},
    /// Playback vcd file
    PlaybackVcd {
        /// nec nes rc5 rc5 sbp
        protocol_string: String,
        path: PathBuf,
    },
    /// Capture data from device. Optionaly write it to file
    Capture { path: Option<PathBuf> },
    /// Use Device as <protocol> receiver
    Protocol { id: u32 },
    /// Transmit
    Transmit { protocol: u32, addr: u32, cmd: u32 },
}

fn main() -> io::Result<()> {
    let opt = Opt::from_args();

    let debug = opt.debug;

    let loglevel = if debug {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    femme::start(loglevel).unwrap();

    let devpath = opt.serial.unwrap_or_else(|| PathBuf::from("/dev/ttyACM0"));

    match opt.cmd {
        CliCommand::Capture { path } => {
            let mut link = SerialLink::new();
            link.connect(&devpath)?;
            capture::command_capture_raw(&mut link, path)
        },
        CliCommand::Decode {} => {
            let mut link = SerialLink::new();
            link.connect(&devpath)?;
            decode::command_decode(&mut link)
        },
        CliCommand::PlaybackVcd {
            protocol_string,
            path,
        } => {
            use ProtocolId::*;

            if let Some(proto) = protocol_from_str(&protocol_string) {

                match proto {
                    Nec => play_nec(&path, debug),
                    Rc5 => play_rc5(&path, debug),
                    Rc6 => play_rc6(&path, debug),
                    _ => play_rc5(&path, debug),
                }
            } else {
                println!("Protocol: {} not found", protocol_string);
                Ok(())
            }
        }
        CliCommand::Protocol { .. } => {
            Ok(())
        },
        CliCommand::Transmit { protocol, addr, cmd } => {
            let mut link = SerialLink::new();
            link.connect(&devpath)?;
            irsend::transmit(&mut link, protocol, addr, cmd)
        },
    }
}

fn protocol_from_str(s: &str) -> Option<ProtocolId> {
    match s {
        "nec" => Some(ProtocolId::Nec),
        "nes" => Some(ProtocolId::NecSamsung),
        "n16" => Some(ProtocolId::Nec16),
        "sbp" => Some(ProtocolId::Sbp),
        "rc6" => Some(ProtocolId::Rc6),
        _ => None,
    }
}

