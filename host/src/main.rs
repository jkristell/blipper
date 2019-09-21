use std::io;
use std::fs::File;
use std::path::{PathBuf, Path};
use std::convert::{TryFrom};

use serialport;
use serialport::prelude::*;
use serialport::Result as SerialResult;
use serialport::SerialPortSettings;

use structopt;
use structopt::StructOpt;

use log::{info, error, };

use common::{Command};

mod vcdwriter;
mod serialpostcard;

use vcdwriter::BlipperVcd;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// Serial Device. Defaults to /dev/ttyACM0
    #[structopt(long = "device", parse(from_os_str))]
    serial: Option<PathBuf>,

    #[structopt(short, long)]
    debug: bool,

    #[structopt(subcommand)]
    cmd: CliCommand
}


#[derive(StructOpt, Debug)]
enum CliCommand {
    /// Decode in realtime with protocol
    Decode {
    },
    /// Playback vcd file
    PlaybackVcd {
        path: Option<PathBuf>,
    },
    /// Capture data from device. Optionaly write it to file
    Capture {
        path: Option<PathBuf>,
    },
    /// Set the samplerate of the receiver
    SetSamplerate {rate: u32},
}


fn serial_connect(path: &Path) -> SerialResult<Box<dyn SerialPort>> {
    let settings = SerialPortSettings {
        baud_rate: 115_200,
        ..Default::default()
    };

    serialport::open_with_settings(path, &settings)
}

fn command_set_samplerate(devpath: &PathBuf, rate: u32) -> io::Result<()> {
    use heapless::{consts::{U64}};
    use postcard::{to_vec};

    let mut port = serial_connect(devpath).expect("Failed to open serial");

    // Send command to device
    let req: heapless::Vec<u8, U64> = to_vec(&Command::SetSampleRate(rate)).unwrap();
    port.write_all(&req).unwrap();

    if serialpostcard::read_ok(&mut port).is_err() {
        error!("Failed to read ok");
    }

    Ok(())
}


fn command_capture_raw(devpath: &PathBuf, path: Option<PathBuf>) -> io::Result<()> {
    use heapless::{consts::{U64}};
    use postcard::{to_vec};

    let mut port = serial_connect(devpath).expect("Failed to open serial");

    // Send command to device
    let req: heapless::Vec<u8, U64> = to_vec(&Command::CaptureRaw).unwrap();
    port.write_all(&req).unwrap();

    if serialpostcard::read_ok(&mut port).is_err() {
        error!("Failed to read ok");
    } else {
        info!("Got ok");
    }

    // Get the CaptureRawDataHeader
    if serialpostcard::read_ok(&mut port).is_err() {
        error!("Failed to read ok");
    } else {
        info!("Got capturerawHeader");
    }

    #[allow(unused_assignments)]
    let mut file = None;
    let mut bvcd = None;
    if let Some(path) = path {
        file = Some(File::create(&path)?);
        bvcd = Some(BlipperVcd::from_writer(file.as_mut().unwrap(), 25, &["ir"])?);
    }

    loop {
        match serialpostcard::read_capturerawdata(&mut port) {
            Ok(rawdata) => {

                let v = [rawdata.d0, rawdata.d1, rawdata.d2, rawdata.d3].concat();

                if let Some(ref mut bvcd) = bvcd {
                    bvcd.write_vec(v).unwrap();
                } else {
                    info!("Capture raw");
                    println!("len: {}, samplerate: {}", rawdata.len, rawdata.samplerate);
                    println!("{:?}", v);
                }
            },
            Err(_err) => {}
        }
    }
}




fn command_decode(_devpath: &Path) -> io::Result<()> {
    //use infrared;
    //use infrared::philips::PhilipsReceiver;

    //let receiver = PhilipsReceiver::new();

    Ok(())
}

fn main() -> io::Result<()> {
    let opt = Opt::from_args();

    let loglevel = if opt.debug {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    femme::start(loglevel).unwrap();

    let devpath = opt.serial.unwrap_or(PathBuf::from("/dev/ttyACM0"));

    match opt.cmd {
        CliCommand::Decode {} => {
            command_decode(&devpath)
        },
        CliCommand::PlaybackVcd {path} => {
            let path = path.unwrap_or(PathBuf::from("philips-bluray.vcd"));
            play_saved_vcd(&path, opt.debug)
        }
        CliCommand::Capture {path} => {
            command_capture_raw(&devpath, path)
        }
        CliCommand::SetSamplerate {rate} => {
            command_set_samplerate(&devpath, rate)
        }
    }
}

fn play_saved_vcd(path: &Path, debug: bool) -> io::Result<()> {
    use infrared::{Receiver, ReceiverState, rc6::Rc6Receiver};

    let (samplerate, vcdvec) = vcdwriter::vcdfile_to_vec(path)?;

    info!("Replay of vcdfile, samplerate = {}", samplerate);

    let vcditer = vcdvec
        .into_iter()
        .map(|(t, v)| (u32::try_from(t).unwrap(), v));

    let mut recv = Rc6Receiver::new(samplerate);

    for (t, value) in vcditer {

        let state = recv.event(value, t);

        if debug {
            println!("{} {} {} {:?}", t, recv.rc6_counter, value, recv.last_state);
        }

        if let ReceiverState::Done(ref cmd) = state {
            println!("Cmd: {:?}", cmd);
            recv.reset();
        }

        if let ReceiverState::Err(err) = state {
            println!("Error: {:?}", err);
            recv.reset();
        }
    }

    Ok(())
}


