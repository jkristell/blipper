use std::fs::File;
use std::io;
use std::path::PathBuf;
use structopt;
use structopt::StructOpt;

use log::info;

mod capture;
mod irsend;
mod playback;
mod vcdutils;

use crate::playback::command_playback;
use libblipper::SerialLink;

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
    /// Playback vcd file
    PlaybackVcd {
        /// nec nes rc5 rc5 sbp
        protocol_string: String,
        path: PathBuf,
    },
    /// Capture / Decode data from device. Optionaly write it to file
    Capture {
        path: Option<PathBuf>,
        #[structopt(short, long)]
        decode: bool,
    },
    /// Use Device as <protocol> receiver
    Protocol { id: u32 },
    /// Transmit
    Transmit { protocol: u32, addr: u32, cmd: u32 },
}

fn main() -> io::Result<()> {
    let opt = Opt::from_args();

    env_logger::init();

    let path_serialport = select_serialport(opt.serial, "/dev/ttyACM0");

    match opt.cmd {
        CliCommand::Capture { path, decode } => {
            let mut link = SerialLink::new();
            link.connect(&path_serialport)?;

            let vcdout = path.and_then(|path| {
                info!("Writing to path: {:?}", path);
                File::create(&path).ok()
            });

            capture::command_capture(&mut link, decode, vcdout)
        }
        CliCommand::PlaybackVcd {
            protocol_string,
            path,
        } => command_playback(&protocol_string, &path),
        CliCommand::Protocol { .. } => Ok(()),
        CliCommand::Transmit {
            protocol,
            addr,
            cmd,
        } => {
            let mut link = SerialLink::new();
            link.connect(&path_serialport)?;
            irsend::transmit(&mut link, protocol, addr, cmd)
        }
    }
}

fn select_serialport(opt: Option<PathBuf>, def: &str) -> PathBuf {
    if let Some(path) = opt {
        path
    } else if let Ok(ports) = serialport::available_ports() {
        ports
            .first()
            .map(|port| PathBuf::from(&port.port_name))
            .unwrap()
    } else {
        PathBuf::from(def)
    }
}
