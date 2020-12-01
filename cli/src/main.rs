use std::{fs::File, io, path::PathBuf};

use env_logger::Env;
use structopt::{self, StructOpt};

mod capture;
mod irsend;
mod playback;
mod vcdutils;

use blipper_utils::SerialLink;

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
    Playback {
        /// nec nes rc5 rc5 sbp
        proto: String,
        path: PathBuf,
    },
    /// Capture / Decode data from device. Optionally write it to file
    Capture {
        path: Option<PathBuf>,
        #[structopt(short, long)]
        decode: bool,
    },
    /// Use Device as <protocol> receiver
    Protocol { id: u8 },
    /// Transmit
    Transmit { proto: u8, addr: u32, cmd: u32 },
}

fn main() -> io::Result<()> {
    let opt = Opt::from_args();

    env_logger::from_env(Env::default().default_filter_or("info")).init();

    let path_serialport = select_serialport(opt.serial, "/dev/ttyACM0");
    let mut link = SerialLink::new();

    match opt.cmd {
        CliCommand::Capture { path, decode } => {
            log::info!("Capture");
            link.connect(&path_serialport)?;

            let vcdout = path.and_then(|path| {
                log::info!("Writing vcd to file: {:?}", path);
                File::create(&path).ok()
            });

            capture::command_capture(&mut link, decode, vcdout)
        }
        CliCommand::Playback { proto, path } => {
            let cmds = playback::command(&proto, &path)?;

            for cmd in cmds {
                println!("{:?}", cmd);
            }

            Ok(())
        },
        CliCommand::Protocol { .. } => Ok(()),
        CliCommand::Transmit { proto, addr, cmd} => {
            link.connect(&path_serialport)?;
            irsend::transmit(&mut link, infrared::Protocol::from(proto), addr, cmd)
        }
    }
}

fn select_serialport(opt: Option<PathBuf>, def: &str) -> PathBuf {
    if let Some(path) = opt {
        return path;
    }

    // Use the first one available
    serialport::available_ports()
        .ok()
        .and_then(|ports| {
            ports
                .iter()
                .next()
                .map(|port| PathBuf::from(&port.port_name))
        })
        .unwrap_or_else(|| PathBuf::from(def))
}
