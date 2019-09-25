use std::io;
use log::{info};

use infrared::nec::*;
use infrared::rc5::*;
use infrared::rc6::*;
use infrared::prelude::*;
use common::Command;
use crate::link::SerialLink;

pub fn command_decode(link: &mut SerialLink) -> io::Result<()> {

    let samplerate = 40_000;
    let mut rc5 = Rc5Receiver::new(samplerate);
    let mut rc6 = Rc6Receiver::new(samplerate);
    let mut nec = NecReceiver::new(samplerate);
    let mut nes = NecSamsungReceiver::new(samplerate);

    info!("Decode");

    link.send_command(Command::CaptureRaw)?;

    link.reply_ok()?;

    loop {
        match link.read_capturerawdata() {
            Ok(rawdata) => {
                let v = rawdata.data.concat();
                let s = &v[0..rawdata.len as usize];

                let mut edge = false;
                let mut t: u32 = 0;

                for dist in s {
                    t += u32::from(*dist);
                    edge = !edge;

                    if let Some(cmd) = sample_on_edge(&mut rc5, edge, t) {
                        println!("Cmd: {:?}", cmd);
                        rc5.reset();
                    }

                    if let Some(cmd) = sample_on_edge(&mut rc6, edge, t) {
                        println!("Cmd: {:?}", cmd);
                        rc6.reset();
                    }

                    if let Some(cmd) = sample_on_edge(&mut nec, edge, t) {
                        println!("Cmd: {:?}", cmd);
                        nec.reset();
                    }

                    if let Some(cmd) = sample_on_edge(&mut nes, edge, t) {
                        println!("Cmd: {:?}", cmd);
                        nes.reset();
                    }
                }
            }
            Err(_err) => {}
        }
    }
}

fn sample_on_edge<CMD, ERR>(recv: &mut impl Receiver<Cmd=CMD, Err=ERR>,
                            edge: bool,
                            t: u32) -> Option<CMD> {

    match recv.sample(edge, t) {
        ReceiverState::Done(neccmd) => {
            return Some(neccmd);
        }
        ReceiverState::Error(_err) => {
            recv.reset();
        }
        _ => {}
    }

    None
}
