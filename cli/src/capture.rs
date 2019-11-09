use std::path::{PathBuf, Path};
use std::io;
use std::fs::File;

use common::{
    Command,
    Reply,
};
use crate::blippervcd::BlipperVcd;

use log::{info};
use libblipper::SerialLink;

pub fn command_capture_raw(link: &mut SerialLink,
                           path: Option<PathBuf>) -> io::Result<()> {
    // Send command to device
    link.send_command(Command::CaptureRaw)?;

    if let Some(path) = path {
        return capture_to_vcd(link, &path)
    }

    info!("Capturing");

    loop {
        if let Ok(Reply::CaptureRawData {rawdata}) = link.read_reply() {
            let v = &rawdata.data.concat()[..rawdata.len as usize];

            println!(
                "len: {}, samplerate: {}\ndata:\n{:?}",
                rawdata.len,
                rawdata.samplerate,
                v
            );
        }
    }
}

fn capture_to_vcd(link: &mut SerialLink, path: &Path) -> io::Result<()> {

    info!("Capture to {}", path.display());

    let mut file = File::create(&path)?;
    let mut bvcd = BlipperVcd::from_writer(
        &mut file,
        25,
        &["ir"],
    )?;

    loop {
        if let Ok(Reply::CaptureRawData {rawdata}) = link.read_reply() {
            let v = &rawdata.data.concat()[..rawdata.len as usize];

            bvcd.write_vec(v)?;
        }
    }
}
