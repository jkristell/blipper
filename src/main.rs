use std::{fs::File, io, path::PathBuf};

use env_logger::Env;
use structopt::{self, StructOpt};

mod capture;
mod irsend;
mod playback;
mod vcdutils;

use blipper_support::SerialLink;
use infrared::Protocol;

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
        nodecode: bool,
    },
    /// Use Device as <protocol> receiver
    Protocol { id: u8 },
    /// Transmit
    Transmit { proto: String, addr: u32, cmd: u32 },
}

fn main() -> io::Result<()> {
    let opt = Opt::from_args();

    env_logger::from_env(Env::default().default_filter_or("info")).init();

    let path_serialport = select_serialport(opt.serial, "/dev/ttyACM0");
    let mut link = SerialLink::new();




    match opt.cmd {
        CliCommand::Capture { path, nodecode } => {
            log::info!("Capture");
            link.connect(&path_serialport)?;

            let vcdout = path.and_then(|path| {
                log::info!("Writing vcd to file: {:?}", path);
                File::create(&path).ok()
            });

            capture::command_capture(&mut link, opt.debug, !nodecode, vcdout)
        }
        CliCommand::Playback { proto, path } => {

            let protocol = parse_protocol(&proto).expect("Failed to parse protocol");
            let cmds = playback::command(protocol, &path)?;

            for cmd in cmds {
                println!("{:?}", cmd);
            }

            Ok(())
        },
        CliCommand::Protocol { .. } => Ok(()),
        CliCommand::Transmit { proto, addr, cmd} => {
            link.connect(&path_serialport)?;

            let protocol = parse_protocol(&proto).expect("Failed to parse protocol");

            irsend::transmit(&mut link, protocol, addr, cmd)
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

fn parse_protocol(s: &str) -> Option<infrared::Protocol> {
    match s {
        "nec" => Some(Protocol::Nec),
        "n16" => Some(Protocol::Nec16),
        "sbp" => Some(Protocol::Sbp),
        "nes" => Some(Protocol::NecSamsung),
        "rc5" => Some(Protocol::Rc5),
        "rc6" => Some(Protocol::Rc6),
        _ => None,
    }
}
