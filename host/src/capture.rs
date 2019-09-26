use std::path::PathBuf;
use std::io;
use std::fs::File;

use common::Command;
use crate::blippervcd::BlipperVcd;

use log::{info};
use crate::link::SerialLink;

pub fn command_capture_raw(link: &mut SerialLink,
                           path: Option<PathBuf>) -> io::Result<()> {
    // Send command to device
    link.send_command(Command::CaptureRaw)?;

    link.reply_ok()?;

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

    loop {
        match link.read_capturerawdata() {
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

