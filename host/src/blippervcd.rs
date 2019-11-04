use std::io;

use std::fs::File;
use std::io::ErrorKind::InvalidInput;
use std::path::Path;
use vcd::{self, SimulationCommand, TimescaleUnit, Value};

use log::info;
use infrared::nec::Nec16Receiver;

pub struct BlipperVcd<'a> {
    wires: Vec<vcd::IdCode>,
    writer: vcd::Writer<'a>,
    timestamp: u64,
}

impl<'a> BlipperVcd<'a> {
    pub fn from_writer(
        w: &'a mut dyn io::Write,
        timescale: u32,
        wirenames: &[&str],
    ) -> io::Result<BlipperVcd<'a>> {


        let mut writer = vcd::Writer::new(w);

        let mut wires = Vec::new();

        // Write the header
        writer.timescale(timescale, TimescaleUnit::US)?;
        writer.add_module("top")?;

        for name in wirenames {
            let id = writer.add_wire(1, name)?;
            wires.push(id);
        }

        writer.upscope()?;
        writer.enddefinitions()?;

        // Write the initial values
        writer.begin(SimulationCommand::Dumpvars)?;
        for wire in &wires {
            writer.change_scalar(*wire, Value::V0)?;
        }
        writer.end()?;

        Ok(BlipperVcd {
            wires,
            writer,
            timestamp: 0,
        })
    }

    pub fn write_value(&mut self, wire_id: usize, ts: u64, high: bool) -> io::Result<()> {
        let offseted_ts = self.timestamp + ts;

        self.writer.timestamp(offseted_ts)?;
        let value = if high { Value::V1 } else { Value::V0 };
        self.writer.change_scalar(self.wires[wire_id], value)?;

        Ok(())
    }

    pub fn write_vec<T: Copy + Into<u64>>(&mut self, v: &[T]) -> io::Result<()> {
        let v2: Vec<u64> = v
            .iter()
            .map(|v| (*v).into())
            .scan(0, |state, delta: u64| {
                *state += delta;
                Some(*state)
            })
            .collect();

        let mut level = true;
        for ts in &v2 {
            self.write_value(0, *ts, level)?;
            level = !level;
        }

        self.add_offset(v2.last().unwrap_or(&0) + 2000);

        Ok(())
    }

    pub fn add_offset(&mut self, offset: u64) {
        self.timestamp += offset;
    }
}

pub fn vcdfile_to_vec(path: &Path) -> io::Result<(u32, Vec<(u64, bool)>)> {
    let file = File::open(path)?;
    let mut parser = vcd::Parser::new(&file);

    // Parse the header and find the wires
    let header = parser.parse_header()?;
    let data = header
        .find_var(&["top", "ir"])
        .ok_or_else(|| io::Error::new(InvalidInput, "no wire top.data"))?
        .code;

    let timescale: Option<(u32, TimescaleUnit)> = header.timescale;
    println!("{:?}", timescale);

    let samplerate = if let Some((timescale, unit)) = timescale {
        match unit {
            TimescaleUnit::MS => 1_000 / timescale,
            TimescaleUnit::US => 1_000_000 / timescale,
            _ => panic!("unsupported"),
        }
    } else {
        0
    };

    println!("samplerate: {:?}", samplerate);

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

    Ok((samplerate, res))
}

pub fn play_saved_vcd(path: &Path, debug: bool) -> io::Result<()> {
    use infrared::{rc5::Rc5Receiver};

    use infrared::s36::S36Receiver;

    use infrared::prelude::*;
    use std::convert::TryFrom;

    let (samplerate, vcdvec) = vcdfile_to_vec(path)?;

    info!("Replay of vcdfile, samplerate = {}", samplerate);

    let vcditer = vcdvec
        .into_iter()
        .map(|(t, v)| (u32::try_from(t).unwrap(), v));

    let mut recv = S36Receiver::new(samplerate);

    println!("{:?}", recv.tolerances);


    for (t, value) in vcditer {
        let state = recv.sample(value, t);
        //println!("State: {} {} {} {} {:?}", value, recv.delta, recv.prev_sampletime, t, recv.state);

        if let ReceiverState::Done(ref cmd) = state {
            println!("Cmd: {:?} ", cmd);
            recv.reset();
        }

        if let ReceiverState::Error(err) = state {
            println!("--Error: {:?}", err);
            recv.reset();
        }
    }

    Ok(())
}

