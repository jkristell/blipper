use std::fs::File;
use std::io;

use crate::vcdutils::VcdWriter;

use blipper_shared::protocol::{Command, Reply};
use blipper_shared::{SerialLink, Decoders};

pub fn command_capture(
    link: &mut SerialLink,
    verbose: bool,
    do_decode: bool,
    mut capture_file: Option<File>,
) -> io::Result<()> {
    log::info!("Capturing");

    let mut decoder = if do_decode { Some(Decoders) } else { None };

    let mut vcd = capture_file
        .as_mut()
        .map(|file| VcdWriter::new(file));

    if let Some(vcd) = vcd.as_mut() {
        vcd.init()?;
    }

    // Set device in capture mode
    link.send_command(Command::Capture)?;
    link.reply_ok()?;

    loop {
        if let Ok(Reply::CaptureReply { data }) = link.read_reply() {
            let concated = &data.bufs.concat()[..data.len as usize];

            log::debug!(
                "Got CaptureReply data: {:?}",
                data,
            );

            if verbose {
                println!(
                    "CaptyreReply len: {}, samplerate: {}\ndata: {:?}",
                    data.len,
                    data.samplerate,
                    concated
                );
            }

            // Decode the data and print it
            if let Some(decoders) = decoder.as_mut() {

                let cmds = decoders.decode_data(concated, data.samplerate);

                if cmds.is_empty() {
                    println!("No command decoded");
                } else {
                    for cmd in cmds {
                        println!(
                            "{:?}\tAddr: {}\tCmd: {}",
                            cmd.kind,
                            cmd.address,
                            cmd.command,
                        );
                    }
                }
            }

            // Write vcd data
            if let Some(vcd) = vcd.as_mut() {
                vcd.write_slice(concated)?;
            }
        }
    }
}

