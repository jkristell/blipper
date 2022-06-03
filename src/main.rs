use std::{fs::File, path::PathBuf};
use std::convert::TryInto;

use env_logger::Env;
use clap::{Parser, Subcommand};
use blipper_shared::SerialLink;

mod capture;
mod irsend;
mod playback;
mod vcdutils;

#[derive(Debug, Parser)]
#[clap(name = "Blipper", about = "Blipper cli tool")]
struct Opt {
    /// Serial Device. Defaults to /dev/ttyACM0
    #[structopt(long = "device", parse(from_os_str))]
    serial: Option<PathBuf>,
    #[structopt(short, long)]
    debug: bool,
    #[structopt(subcommand)]
    cmd: CliCommand,
}

#[derive(Subcommand, Debug)]
enum CliCommand {
    /// Playback vcd file
    Playback {
        /// nec nes rc5 rc5 sbp
        proto: String,
        path: PathBuf,
    },
    /// Capture / Decode data from device. Optionally write it to file
    Capture {
        /// Samplerate
        #[clap(default_value = "40000")]
        sample_rate: u32,
        /// Decode the data live
        #[clap(short, long)]
        decode: bool,
        /// Vcd output
        path: Option<PathBuf>,
    },
    /// Use Device as <protocol> receiver
    Protocol { id: u8 },
    /// Transmit
    Transmit { proto: String, addr: u32, cmd: u32 },
    /// List serial ports
    Listserialports,
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let path_serialport = select_serialport(opt.serial, "/dev/ttyACM0");
    let mut link = SerialLink::new();

    match opt.cmd {
        CliCommand::Capture { sample_rate, path, decode } => {
            log::info!("Capture");
            link.connect(&path_serialport)?;

            let mut vcdout = None;

            if let Some(path) = path {
                log::info!("Writing vcd to file: {:?}", path);
                vcdout.replace(File::create(&path)?);
            };

            capture::setup(&mut link, sample_rate, opt.debug, decode, vcdout)?;
        }
        CliCommand::Playback { proto, path } => {
            let protocol = proto.as_str().try_into().map_err(|_| anyhow::Error::msg("Unknown protocol"))?;

            let cmds = playback::command(protocol, &path)?;

            for cmd in cmds {
                println!("{:?}", cmd);
            }
        }
        CliCommand::Protocol { .. } => {
            log::warn!("Protocol is not implemented");
        }
        CliCommand::Transmit { proto, addr, cmd } => {
            link.connect(&path_serialport)?;

            let protocol = proto.as_str().try_into().map_err(|_| anyhow::Error::msg("Unknown protocol"))?;

            irsend::transmit(&mut link, protocol, addr, cmd)?;
        }
        CliCommand::Listserialports => {
            let ports = SerialLink::list_ports()?;
            for p in &ports {
                println!("{:?}", p);
            }
        }
    };
    Ok(())
}

fn select_serialport(opt: Option<PathBuf>, def: &str) -> PathBuf {
    if let Some(path) = opt {
        return path;
    }

    SerialLink::list_ports()
        .ok()
        .and_then(|ports| {
            ports
                .iter()
                .next()
                .map(|port| PathBuf::from(&port.port_name))
        })
        .unwrap_or_else(|| PathBuf::from(def))
}

