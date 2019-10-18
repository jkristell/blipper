use std::io;
use log::{info};

use common::{ Command, Reply, };
use libblipperhost::{Decoder, SerialLink};


pub fn command_decode(link: &mut SerialLink) -> io::Result<()> {

    info!("Decode");

    let mut decoder = Decoder::new(40_000);
    link.send_command(Command::CaptureRaw)?;






    loop {

        if let Ok(Reply::CaptureRawData {rawdata}) = link.read_reply() {
            let v = &rawdata.data.concat()[..rawdata.len as usize];
            let decoded = decoder.decode_data(v);

            println!("{:?}", decoded);
        }
        else {
            //eprintln!("Unexpected reply");
        }
    }
}

