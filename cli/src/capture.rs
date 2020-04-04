use std::fs::File;
use std::io;

use log::info;

use blipper_protocol::{Command, Reply};
use blipper_utils::{Decoder, SerialLink};
use crate::vcdutils::VcdWriter;

pub fn command_capture(
    link: &mut SerialLink,
    do_decode: bool,
    mut capture_file: Option<File>,
) -> io::Result<()> {
    info!("Capturing");

    let mut decoder = if do_decode {
        Some(Decoder::new(40_000))
    } else {
        None
    };

    let mut vcd = capture_file.as_mut().map(|file| VcdWriter::new(file));

    if let Some(vcd) = vcd.as_mut() {
        vcd.init()?;
    }

    // Set device in capture mode
    link.send_command(Command::Capture)?;

    loop {
        if let Ok(Reply::CaptureReply { data }) = link.read_reply() {
            let v = &data.bufs.concat()[..data.len as usize];

            println!(
                "len: {}, samplerate: {}\ndata: {:?}",
                data.len,
                data.samplerate,
                v
            );

            // Decode the data and print it
            if let Some(decoder) = decoder.as_mut() {
                let decoded = decoder.decode_data(v);
                if let Some(data) = decoded {
                    println!("Decoded: {:?}", data);
                } else {
                    println!("Decoded: None");
                }
            }

            // Write vcd data
            if let Some(vcd) = vcd.as_mut() {
                vcd.write_vec(v)?;
            }
        }
    }
}
