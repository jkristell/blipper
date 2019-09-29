use std::io;
use std::path::{PathBuf};
use structopt;
use structopt::StructOpt;

//use log::{error, info};

mod blippervcd;
mod link;
mod capture;
mod decode;
mod irsend;

use link::SerialLink;


#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
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
    PlaybackVcd { path: Option<PathBuf> },
    /// Capture data from device. Optionaly write it to file
    Capture { path: Option<PathBuf> },
    /// Use Device as <protocol> receiver
    Protocol { id: u32 },
    /// Transmit
    Transmit { protocol: u32, addr: u32, cmd: u32 },
}

fn main() -> io::Result<()> {
    let opt = Opt::from_args();

    let loglevel = if opt.debug {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    femme::start(loglevel).unwrap();

    let devpath = opt.serial.unwrap_or_else(|| PathBuf::from("/dev/ttyACM0"));

    match opt.cmd {
        CliCommand::Decode {} => {
            let mut link = SerialLink::new(&devpath);
            decode::command_decode(&mut link)
        },
        CliCommand::PlaybackVcd { path } => {
            let path = path.unwrap_or_else(|| PathBuf::from("philips-bluray.vcd"));
            blippervcd::play_saved_vcd(&path, opt.debug)
        }
        CliCommand::Capture { path } => {
            let mut link = SerialLink::new(&devpath);
            capture::command_capture_raw(&mut link, path)
        },
        CliCommand::Protocol { .. } => {
            Ok(())
        },
        CliCommand::Transmit { protocol, addr, cmd } => {
            let mut link = SerialLink::new(&devpath);
            irsend::transmit(&mut link, protocol, addr, cmd)
        },
    }
}

