use std::io;
use log::{info};

use common::{ Command, Reply, };
use libblipper::{Decoder, SerialLink};


pub fn command_decode(link: &mut SerialLink) -> io::Result<()> {

    info!("Decode");

    let mut decoder = Decoder::new(40_000);
    link.send_command(Command::CaptureRaw)?;

    loop {

        let reply = link.read_reply();

        if let Ok(Reply::CaptureRawData {rawdata}) = reply {
            let v = &rawdata.data.concat()[..rawdata.len as usize];
            let decoded = decoder.decode_data(v);

            println!("{:?}", decoded);
        }
        else {
            info!("Unexpected reply: {:?}", reply);
            return Ok(());
        }
    }
}
