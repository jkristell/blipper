use std::io;
use std::io::ErrorKind::InvalidInput;
use std::fs::File;
use std::path::{PathBuf, Path};
use std::convert::{TryFrom};

use serialport;
use serialport::prelude::*;
use serialport::Result as SerialResult;
use serialport::SerialPortSettings;

use structopt;
use structopt::StructOpt;

use vcd::Value;
use log::{info, error};

use common::{Command};

mod vcdwriter;
mod serialpostcard;

use vcdwriter::BlipperVcd;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// Set speed
    #[structopt(short = "s", long = "speed", default_value = "115200")]
    speed: u32,
    /// Serial Device. Defaults to /dev/ttyACM0
    #[structopt(long = "device", parse(from_os_str))]
    serial: Option<PathBuf>,

    #[structopt(short, long)]
    debug: bool,

    // The number of occurrences of the `v/verbose` flag
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[structopt(short, long, parse(from_occurrences))]
    verbose: u8,

    #[structopt(subcommand)]
    cmd: CliCommand
}


#[derive(StructOpt, Debug)]
enum CliCommand {
    #[structopt(name = "vcd")]
    /// Capture ir signals from blipper device to vcd file
    Vcd {
    },
    #[structopt(name = "decode")]
    /// Decode in realtime with protocol
    Decode {
    },
    #[structopt(name = "playback")]
    /// Playback vcd file
    Playback {
        path: Option<PathBuf>,
    },
    /// Read raw data from device
    PostcardRead {
        vcd: bool,
    },
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


fn command_capture_raw(devpath: &PathBuf, vcd: bool) -> io::Result<()> {
    use heapless::{consts::{U64}};
    use postcard::{to_vec};

    dbg!(vcd);

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

    let mut file = File::create("blipper2.vcd")?;
    let mut bvcd = None;
    if vcd {
        bvcd = Some(BlipperVcd::from_writer(&mut file, 25, &["ir"])?);
    }


    loop {
        match serialpostcard::read_capturerawdata(&mut port) {
            Ok(rawdata) => {

                if vcd {
                    let v: Vec<_> = [rawdata.d0, rawdata.d1, rawdata.d2, rawdata.d3].concat();
                    bvcd.as_mut().unwrap().write_vec(v);
                } else {
                    info!("Capture raw");
                    println!("len: {}, samplerate: {}", rawdata.len, rawdata.samplerate);
                    println!("{:?}", rawdata.d0);
                    println!("{:?}", rawdata.d1);
                    println!("{:?}", rawdata.d2);
                    println!("{:?}", rawdata.d3);

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
    femme::start(log::LevelFilter::Info).unwrap();

    let opt = Opt::from_args();
    let devpath = opt.serial.unwrap_or(PathBuf::from("/dev/ttyACM0"));

    match opt.cmd {
        CliCommand::Vcd {} => {
            Ok(())
        },
        CliCommand::Decode {} => {
            command_decode(&devpath)
        },
        CliCommand::Playback {path} => {
            let path = path.unwrap_or(PathBuf::from("philips-bluray.vcd"));
            play_saved_vcd(&path, opt.debug)
        }
        CliCommand::PostcardRead {vcd} => {
            command_capture_raw(&devpath, vcd)
        }
        CliCommand::SetSamplerate {rate} => {
            command_set_samplerate(&devpath, rate)
        }
    }
}

fn play_saved_vcd(path: &Path, debug: bool) -> io::Result<()> {
    use infrared::{Receiver, ReceiverState, rc6::Rc6Receiver};

    let mut recv = Rc6Receiver::new(40_000);

    let vcditer = parse_vcd(path)?
        .into_iter()
        .map(|(t, v)| (u32::try_from(t).unwrap(), v));


    for (t, value) in vcditer {

        let state = recv.event(value, t);

        if debug {
            println!("{} {} {} {:?}", t, recv.rc6_counter, value, recv.last_state);
        }

        if let ReceiverState::Done(ref cmd) = state {
            println!("Cmd: {:?}\n", cmd);
            recv.reset();
        }

        if let ReceiverState::Err(err) = state {
            println!("Error: {:?}", err);
            recv.reset();
        }
    }

    Ok(())
}


fn parse_vcd(path: &Path) -> io::Result<Vec<(u64, bool)>> {

    let file = File::open(path)?;
    let mut parser = vcd::Parser::new(&file);

    // Parse the header and find the wires
    let header = parser.parse_header()?;
    let data = header.find_var(&["top", "ir"])
        .ok_or_else(|| io::Error::new(InvalidInput, "no wire top.data"))?.code;

    let timescale = header.timescale;
    println!("{:?}", timescale);

    // Iterate through the remainder of the file and decode the data
    let mut current_ts = 0;
    let mut res: Vec<(u64, bool)> = Vec::new();

    for command_result in parser {
        use vcd::Command::*;
        let command = command_result?;
        match command {
            ChangeScalar(i, v) if i == data => {
                let one = v == Value::V1;
                res.push((current_ts, one));
            }
            Timestamp(ts) => current_ts = ts,
            _ => (),
        }
    }
    Ok(res)
}
