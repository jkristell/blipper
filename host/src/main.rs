use std::convert::TryFrom;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use serialport;
use serialport::prelude::*;
use serialport::Result as SerialResult;
use serialport::SerialPortSettings;

use structopt;
use structopt::StructOpt;

use log::{error, info};

use common::Command;

mod blippervcd;
mod serialpostcard;

use blippervcd::BlipperVcd;
use infrared::rc6::rc6_multiplier;
use infrared::nec::NecSamsungReceiver;

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
}

fn serial_connect(path: &Path) -> SerialResult<Box<dyn SerialPort>> {
    let settings = SerialPortSettings {
        baud_rate: 115_200,
        ..Default::default()
    };

    serialport::open_with_settings(path, &settings)
}

fn command_protocol(devpath: &PathBuf, id: u32) -> io::Result<()> {
    use heapless::consts::U64;
    use postcard::to_vec;

    let mut port = serial_connect(devpath).expect("Failed to open serial");

    // Send command to device
    let req: heapless::Vec<u8, U64> = to_vec(&Command::CaptureProtocol(id)).unwrap();
    port.write_all(&req).unwrap();

    if serialpostcard::read_ok(&mut port).is_err() {
        error!("Failed to read ok");
    }

    loop {
        match serialpostcard::read_protocoldata(&mut port) {
            Ok(genericremote) => {
                info!("Protocol capture");
                println!("{:?}", genericremote);
            }
            Err(_err) => {
            }
        }
    }

    Ok(())
}

fn command_capture_raw(devpath: &PathBuf, path: Option<PathBuf>) -> io::Result<()> {
    use heapless::consts::U64;
    use postcard::to_vec;

    let mut port = serial_connect(devpath).expect("Failed to open serial");

    // Send command to device
    let req: heapless::Vec<u8, U64> = to_vec(&Command::CaptureRaw).unwrap();
    port.write_all(&req).unwrap();

    if serialpostcard::read_ok(&mut port).is_err() {
        error!("Failed to read ok");
    } else {
        info!("Got ok");
    }

    #[allow(unused_assignments)]
    let mut file = None;
    let mut bvcd = None;
    if let Some(path) = path {
        file = Some(File::create(&path)?);
        bvcd = Some(BlipperVcd::from_writer(
            file.as_mut().unwrap(),
            25,
            &["ir"],
        )?);
    }

    for i in &[1, 2, 3, 6] {
        let r = rc6_multiplier(40_000, *i);
        println!("{} = {:?}", i, r);
    }



    loop {
        match serialpostcard::read_capturerawdata(&mut port) {
            Ok(rawdata) => {
                let v = rawdata.data.concat();
                let s = &v[0..rawdata.len as usize];

                if let Some(ref mut bvcd) = bvcd {
                    bvcd.write_vec(s).unwrap();
                } else {
                    info!("Capture raw");
                    println!("len: {}, samplerate: {}", rawdata.len, rawdata.samplerate);
                    println!("{:?}", &v[0..rawdata.len as usize]);
                }
            }
            Err(_err) => {}
        }
    }
}

fn command_decode(devpath: &Path) -> io::Result<()> {
    use heapless::consts::U64;
    use infrared::nec::remotes::SpecialForMp3;
    use infrared::nec::NecCommand;
    use infrared::nec::{NecReceiver};
    use infrared::rc5::Rc5Receiver;
    use infrared::rc6::Rc6Receiver;
    use infrared::Receiver;
    use infrared::ReceiverState;
    use infrared::RemoteControl;
    use postcard::to_vec;

    let mut rc5 = Rc5Receiver::new(40_000);
    let mut rc6 = Rc6Receiver::new(40_000);
    let mut nec = NecReceiver::new(40_000);
    let mut nes = NecSamsungReceiver::new(40_000);
    let mp3remote = SpecialForMp3;

    info!("Decode");
    let mut port = serial_connect(devpath).expect("Failed to open serial");

    // Send command to device
    let req: heapless::Vec<u8, U64> = to_vec(&Command::CaptureRaw).unwrap();
    port.write_all(&req).unwrap();

    if serialpostcard::read_ok(&mut port).is_err() {
        error!("Failed to read ok");
    } else {
        info!("Got ok");
    }

    loop {
        match serialpostcard::read_capturerawdata(&mut port) {
            Ok(rawdata) => {
                let v = rawdata.data.concat();
                let s = &v[0..rawdata.len as usize];

                let mut edge = false;
                let mut t: u32 = 0;

                for dist in s {
                    t += u32::from(*dist);
                    edge = !edge;

                    // Rc5?
                    match rc5.sample(edge, t) {
                        ReceiverState::Done(cmd) => {
                            println!("Got Rc5Cmd: {:?}", cmd);
                            rc5.reset();
                        }
                        ReceiverState::Error(_) => {
                            rc5.reset();
                        }
                        _ => {}
                    }

                    // Rc6?
                    match rc6.sample(edge, t) {

                        ReceiverState::Done(cmd) => {
                            println!("Got Rc6Cmd: {:?}", cmd);
                            rc6.reset();
                        }
                        ReceiverState::Error(err) => {
                            println!("Rc Err: {:?}", err);
                            rc6.reset();
                        }
                        _ => {}
                    }
                    // Nec?
                    match nec.sample(edge, t) {
                        ReceiverState::Done(neccmd) => {
                            println!("neccmd: {:?} {:X?}", neccmd, nec.bitbuf);
                            nec.reset();
                        }
                        ReceiverState::Error(err) => {
                            println!("err: {:?}", err);
                            nec.reset();
                        }
                        _ => {}
                    }

                    // Samsung Nec
                    match nes.sample(edge, t) {
                        ReceiverState::Done(neccmd) => {
                            println!("neccmd: {:?} {:X?}", neccmd, nes.bitbuf);
                            nes.reset();
                        }
                        ReceiverState::Error(err) => {
                            println!("err: {:?}", err);
                            nes.reset();
                        }
                        _ => {}
                    }

/*
                    println!("{:?} -> {:?} {} {} {}",
                        nec.prev_state,
                        nec.state,
                        nec.interval,
                        nec.prev_pinval,
                        nec.prev_timestamp,
                    );
                    */
                }
            }
            Err(_err) => {}
        }
    }
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
        CliCommand::Decode {} => command_decode(&devpath),
        CliCommand::PlaybackVcd { path } => {
            let path = path.unwrap_or(PathBuf::from("philips-bluray.vcd"));
            play_saved_vcd(&path, opt.debug)
        }
        CliCommand::Capture { path } => command_capture_raw(&devpath, path),
        CliCommand::Protocol { id } => command_protocol(&devpath, id),
    }
}

fn play_saved_vcd(path: &Path, debug: bool) -> io::Result<()> {
    use infrared::{rc5::Rc5Receiver, Receiver, ReceiverState};

    let (samplerate, vcdvec) = blippervcd::vcdfile_to_vec(path)?;

    info!("Replay of vcdfile, samplerate = {}", samplerate);

    let vcditer = vcdvec
        .into_iter()
        .map(|(t, v)| (u32::try_from(t).unwrap(), v));

    let sr = 40_000;
    let mut recv = Rc5Receiver::new(sr);

    let mut t_prev = 0;

    if debug {
        println!("T\tRc5\tRising\tDelta\t\tState");
    }
    for (t, value) in vcditer {
        let state = recv.sample(value, t);

        t_prev = t;

        if let ReceiverState::Done(ref cmd) = state {
            println!("Cmd: {:?}", cmd);
            recv.reset();
        }

        if let ReceiverState::Error(err) = state {
            println!("--Error: {:?}", err);
            recv.reset();
        }
    }

    Ok(())
}
