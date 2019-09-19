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
use log::info;

use common::{Reply, Command};

use blipper_host::vcdwriter::BlipperVcd;


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
    Postcard {},
    PostcardRead {},
}


fn serial_connect(path: &Path) -> SerialResult<Box<dyn SerialPort>> {
    let settings = SerialPortSettings {
        baud_rate: 115_200,
        ..Default::default()
    };

    serialport::open_with_settings(path, &settings)
}


fn parse_blipper_data(input: &str) -> Vec<u64> {

    let mut iter = input.split(' ');

    if Some("DATA") != iter.next() {
        return vec![];
    }

    iter
        .filter_map(|s| s.parse::<u64>().ok())
        .scan(0, |state, delta| {
            *state += delta;
            Some(*state)
        })
        .collect()
}


fn write_vcd(vcdwriter: &mut BlipperVcd, bytes: &[u8]) -> io::Result<()> {

    let inputline = std::str::from_utf8(bytes).unwrap();
    let l = inputline.trim();

    let mut level = true;

    println!("LINE: {}", l);
    let v = parse_blipper_data(&l);
    println!("bd: {:?}", v);

    for ts in &v {
        vcdwriter.write_value(0, *ts, level)?;
        level = !level;
    }

    vcdwriter.add_offset(v.last().unwrap_or(&0) + 200);

    Ok(())
}

fn command_postcard(devpath: &PathBuf) -> io::Result<()> {
    use heapless::{
        consts::{U64},
    };

    use postcard::to_vec;
    let mut port = serial_connect(devpath).expect("Failed to open serial");

    let cmd_send = common::Command::CaptureRaw;
    let req: heapless::Vec<u8, U64> = to_vec(&cmd_send).unwrap();
    println!("{:?}", req);
    port.write_all(&req).unwrap();

    Ok(())
}


fn command_postcard_read(devpath: &PathBuf) -> io::Result<()> {
    use heapless::{
        consts::{U64},
    };
    use postcard::{
        to_vec, from_bytes
    };
    use std::convert::TryInto;

    let mut port = serial_connect(devpath).expect("Failed to open serial");

    // Send command to device
    let req: heapless::Vec<u8, U64> = to_vec(&Command::CaptureRaw).unwrap();
    port.write_all(&req).unwrap();


    loop {
        let mut sbuf = [0; 1024];

        match port.read(&mut sbuf[0..]) {
            Ok(_readlen) => {

                match from_bytes::<Reply>(&sbuf) {
                    Ok(reply) => match reply {
                        Reply::CaptureRawHeader {samplerate} => {
                            info!("CaptureRawHeader: {}", samplerate);
                        }
                        Reply::CaptureRawData {data} => {
                            info!("Capture raw");

                            let v: Vec<_> = data.chunks(4)
                                .map(|chunk|
                                    u32::from_le_bytes(chunk.try_into().unwrap()))
                                .collect();

                            println!("{:?}", v);

                            // clear buffer
                            for elem in sbuf.iter_mut() { *elem = 0; }
                        },
                        _ => println!("Unhandled Reply"),
                    }
                    //TODO: Implement chunked read if needed.
                    _ => println!("Failed to read Reply"),
                };
            },
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            Err(e) => eprintln!("{:?}", e),
        }
    }

    Ok(())
}



fn command_vcd(devpath: &PathBuf) -> io::Result<()> {

    let mut port = serial_connect(devpath).unwrap();

    let mut file = File::create("blipper.vcd")?;
    let mut bvcd = BlipperVcd::from_writer(&mut file, 25, &["ir"])?;

    let mut buf = [0; 1024];
    let mut start = 0;
    let mut end = 0;

    loop {
        match port.read(&mut buf[start..]) {
            Ok(readlen) => {

                end += readlen;

                if let Some(newlinepos) = buf[..end].iter().position(|elem| *elem == b'\n') {

                    write_vcd(&mut bvcd, &buf[..newlinepos])?;

                    for i in 0..(buf.len() - newlinepos) {
                        buf[i] = buf[newlinepos + i];
                    }

                    start = 0;
                    end = 0;
                } else {
                    start += readlen;
                }
            },
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            Err(e) => eprintln!("{:?}", e),
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
            command_vcd(&devpath)
        },
        CliCommand::Decode {} => {
            command_decode(&devpath)
        },
        CliCommand::Playback {path} => {
            let path = path.unwrap_or(PathBuf::from("philips-bluray.vcd"));
            play_saved_vcd(&path, opt.debug)
        }
        CliCommand::Postcard {} => {
            command_postcard(&devpath)
        }
        CliCommand::PostcardRead {} => {
            command_postcard_read(&devpath)
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
