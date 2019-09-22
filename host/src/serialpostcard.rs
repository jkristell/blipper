use common::{RawData, Reply, GenericRemote};
use postcard::from_bytes;
use serialport::SerialPort;
use std::io;

pub fn read_ok(port: &mut Box<dyn SerialPort>) -> io::Result<()> {
    let mut sbuf = [0; 1024];
    match port.read(&mut sbuf[0..]) {
        Ok(_readlen) => match from_bytes::<Reply>(&sbuf) {
            Ok(reply) => match reply {
                Reply::Ok => return Ok(()),
                _ => return Err(io::ErrorKind::InvalidData.into()),
            },
            _ => return Err(io::ErrorKind::InvalidData.into()),
        },
        Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
        Err(e) => eprintln!("{:?}", e),
    }

    Ok(())
}

pub fn read_protocoldata(port: &mut Box<dyn SerialPort>) -> io::Result<GenericRemote> {
    let mut recvbuf = [0; 1024];

    match port.read(&mut recvbuf[0..]) {
        Ok(readlen) => {

            dbg!(readlen);

            match from_bytes::<Reply>(&recvbuf) {
                Ok(reply) => match reply {
                    Reply::ProtocolData { data } => return Ok(data),
                    _ => (),
                },
                _ => (),
            }
        }
        Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
        Err(e) => eprintln!("{:?}", e),
    };

    Err(io::ErrorKind::InvalidData.into())
}


pub fn read_capturerawdata(port: &mut Box<dyn SerialPort>) -> io::Result<RawData> {
    let mut recvbuf = [0; 1024];
    let mut offset = 0;

    //FIXME: Figure out how to do this properly.
    while offset < std::mem::size_of::<Reply>() - 3 {
        match port.read(&mut recvbuf[offset..]) {
            Ok(readlen) => {
                offset += readlen;
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            Err(e) => eprintln!("{:?}", e),
        }
    }

    match from_bytes::<Reply>(&recvbuf) {
        Ok(reply) => match reply {
            Reply::CaptureRawData { rawdata } => Ok(rawdata),
            _ => Err(io::ErrorKind::InvalidData.into()),
        },
        _ => Err(io::ErrorKind::InvalidData.into()),
    }
}
