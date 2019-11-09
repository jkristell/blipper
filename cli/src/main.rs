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
use libblipper::{
    SerialLink,
};
use infrared::ProtocolId;
use crate::blippervcd::{playback_rc5, playback_rc6, playback_nes};

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

    let path_serialport =
        if let Some(path) = opt.serial {
            path
        } else if let Ok(ports) = serialport::available_ports() {
            ports
                .first()
                .map(|port| PathBuf::from(&port.port_name))
                .unwrap()
        } else {
            PathBuf::from("/dev/ttyACM0")
        };

    match opt.cmd {
        CliCommand::Capture { path } => {
            let mut link = SerialLink::new();
            link.connect(&path_serialport)?;
            capture::command_capture_raw(&mut link, path)
        },
        CliCommand::Decode {} => {
            let mut link = SerialLink::new();
            link.connect(&path_serialport)?;
            decode::command_decode(&mut link)
        },
        CliCommand::PlaybackVcd { protocol_string, path } => {

            if let Some(proto) = protocol_from_str(&protocol_string) {
                use ProtocolId::*;
                match proto {
                    Nec | Nec16 => playback_nes(&path, debug),
                    //NecSamsung => play_
                    Rc5 => playback_rc5(&path, debug),
                    Rc6 => playback_rc6(&path, debug),
                    _ => playback_rc5(&path, debug),
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
            link.connect(&path_serialport)?;
            irsend::transmit(&mut link, protocol, addr, cmd)
        },
    }
}

fn protocol_from_str(s: &str) -> Option<ProtocolId> {
    match s {
        "nec" => Some(ProtocolId::Nec),
        "n16" => Some(ProtocolId::Nec16),
        "nes" => Some(ProtocolId::NecSamsung),
        "sbp" => Some(ProtocolId::Sbp),
        "rc5" => Some(ProtocolId::Rc5),
        "rc6" => Some(ProtocolId::Rc6),
        _ => None,
    }
}

