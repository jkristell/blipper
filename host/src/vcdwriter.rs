use std::io;

use vcd::{self, SimulationCommand, TimescaleUnit, Value};

pub struct BlipperVcd<'a> {
    wires: Vec<vcd::IdCode>,
    writer: vcd::Writer<'a>,
    timestamp: u64,
}

impl<'a> BlipperVcd<'a> {
    pub fn from_writer(w: &'a mut dyn io::Write,
                       timescale: u32,
                       wirenames: &[&str]) -> io::Result<BlipperVcd<'a>> {
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

        Ok(BlipperVcd { wires, writer, timestamp: 0 })
    }


    pub fn write_value(&mut self, wire_id: usize, ts: u64, high: bool) -> io::Result<()> {

        let offseted_ts = self.timestamp + ts;

        self.writer.timestamp(offseted_ts)?;
        let value = if high {Value::V1} else {Value::V0};
        self.writer.change_scalar(self.wires[wire_id], value)?;

        Ok(())
    }

    pub fn write_vec<T: Into<u64>>(&mut self, v: Vec<T>) -> io::Result<()> {

        let v2: Vec<u64> = v.into_iter()
                .map(|v| v.into())
                .scan(0, |state, delta| {
                    *state += delta;
                    Some(*state)
                })
                .collect();


        let mut level = false;
        for ts in &v2 {
            self.write_value(0, *ts, level)?;
            level = !level;
        }

        self.add_offset(v2.last().unwrap_or(&0) + 200);

        Ok(())
    }

    pub fn add_offset(&mut self, offset: u64) {
        self.timestamp += offset;
    }

}
