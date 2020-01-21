use std::fmt::Debug;
use std::io;
use std::path::Path;

use log::warn;

use infrared::nec::Nec;
use infrared::rc5::Rc5;
use infrared::rc6::Rc6;
use infrared::{Command, ReceiverState, ReceiverStateMachine};

use crate::vcdutils::vcdfile_to_vec;

pub fn command_playback(name: &str, path: &Path) -> io::Result<()> {
    let (samplerate, v) = vcdfile_to_vec(path).unwrap();
    let debug = false;

    match name {
        "nes" => {
            let mut nec = Nec::for_samplerate(samplerate);
            play_vcd(&v, &mut nec, debug)
        }
        "rc5" => {
            let mut recv = Rc5::new(samplerate);
            play_vcd(&v, &mut recv, debug)
        }
        "rc6" => {
            let mut recv = Rc6::new(samplerate);
            play_vcd(&v, &mut recv, debug)
        }
        _ => {
            warn!("Unknown protocol: {}", name);
            Ok(())
        }
    }
}

pub fn play_vcd<RECV, CMD>(vcdvec: &[(u64, bool)], recv: &mut RECV, _debug: bool) -> io::Result<()>
where
    RECV: ReceiverStateMachine<Cmd = CMD>,
    CMD: Debug + Command,
{
    use std::convert::TryFrom;

    let iter = vcdvec
        .iter()
        .cloned()
        .map(|(t, v)| (u32::try_from(t).unwrap(), v));

    for (t, value) in iter {
        let state = recv.event(value, t);

        if let ReceiverState::Done(ref cmd) = state {
            println!("Cmd: {:?} ", cmd);
            recv.reset();
        }

        if let ReceiverState::Error(err) = state {
            println!("Error: {:?}", err);
            recv.reset();
        }
    }

    Ok(())
}
