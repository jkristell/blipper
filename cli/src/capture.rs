use log::info;
use std::fs::File;
use std::io;

use crate::vcdutils::VcdWriter;
use blipper_protocol::{Command, Reply};
use blipper_utils::{Decoders, SerialLink};

pub fn command_capture(
    link: &mut SerialLink,
    do_decode: bool,
    mut capture_file: Option<File>,
) -> io::Result<()> {
    info!("Capturing");

    let mut decoder = if do_decode { Some(Decoders) } else { None };

    let mut vcd = capture_file.as_mut().map(|file| VcdWriter::new(file));

    if let Some(vcd) = vcd.as_mut() {
        vcd.init()?;
    }

    // Set device in capture mode
    link.send_command(Command::Capture)?;
    link.reply_ok()?;

    loop {
        if let Ok(Reply::CaptureReply { data }) = link.read_reply() {
            let concated = &data.bufs.concat()[..data.len as usize];

            println!(
                "len: {}, samplerate: {}\ndata: {:?}",
                data.len,
                data.samplerate, 
                concated
            );

            // Decode the data and print it
            if let Some(decoder) = decoder.as_mut() {
                let decoded = decoder.decode_data(concated, data.samplerate);
                for cmd in decoded {
                    println!("Decoded: {:?}", cmd);
                }
            }

            // Write vcd data
            if let Some(vcd) = vcd.as_mut() {
                vcd.write_vec(concated)?;
            }
        }
    }
}
