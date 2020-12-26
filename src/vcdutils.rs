use std::fs::File;
use std::io;
use std::io::ErrorKind::InvalidInput;
use std::path::Path;

use vcd::{self, SimulationCommand, TimescaleUnit, Value};

pub struct VcdWriter<'a> {
    vcd: vcd::Writer<&'a mut File>,
    timestamp: u64,
    wire_id: vcd::IdCode,
}

impl<'a> VcdWriter<'a> {
    /// Create a new vcd writer
    pub fn new(file: &'a mut File) -> Self {
        let vcd = vcd::Writer::new(file);

        Self {
            vcd,
            timestamp: 0,
            wire_id: vcd::IdCode::FIRST,
        }
    }

    pub fn init(&mut self) -> io::Result<()> {
        let writer = &mut self.vcd;

        // Write the header
        //TODO: Timescale
        writer.timescale(25, TimescaleUnit::US)?;
        writer.add_module("top")?;

        // Add the wire
        let id = writer.add_wire(1, "data")?;
        self.wire_id = id;

        writer.upscope()?;
        writer.enddefinitions()?;

        // Write the initial values
        writer.begin(SimulationCommand::Dumpvars)?;
        writer.change_scalar(id, Value::V0)?;
        writer.end()?;

        Ok(())
    }

    pub fn write_slice<T: Copy + Into<u64>>(&mut self, v: &[T]) -> io::Result<()> {
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
            self.write_value(*ts, level)?;
            level = !level;
        }

        self.add_offset(v2.last().unwrap_or(&0) + 2000);

        Ok(())
    }

    pub fn write_value(&mut self, ts: u64, high: bool) -> io::Result<()> {
        let offseted_ts = self.timestamp + ts;

        self.vcd.timestamp(offseted_ts)?;
        let value = if high { Value::V1 } else { Value::V0 };
        self.vcd.change_scalar(self.wire_id, value)?;

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
