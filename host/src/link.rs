use common::{RawData, Reply, Command};
use postcard::{from_bytes, to_vec};
use serialport::{SerialPort, SerialPortSettings};
use std::io;
use std::path::Path;
use heapless::consts::U64;

pub struct SerialLink {
    port: Box<dyn SerialPort>,
}

impl SerialLink {

    pub fn new(path: &Path) -> Self {
        let settings = SerialPortSettings {
            baud_rate: 115_200,
            ..Default::default()
        };

        let port = serialport::open_with_settings(path, &settings).unwrap();

        Self {port}
    }

    pub fn send_command(&mut self, cmd: Command) -> io::Result<()> {
        let req: heapless::Vec<u8, U64> = to_vec(&cmd).unwrap();
        self.port.write_all(&req)
    }

    pub fn reply_ok(&mut self) -> io::Result<()> {
        let mut sbuf = [0; 1024];
        match self.port.read(&mut sbuf[0..]) {
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

    pub fn read_capturerawdata(&mut self) -> io::Result<RawData> {
        let mut recvbuf = [0; 1024];
        let mut offset = 0;

        //FIXME: Figure out how to do this properly.
        while offset < std::mem::size_of::<Reply>() - 3 {
            match self.port.read(&mut recvbuf[offset..]) {
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

}

/*
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

            if let Ok(Reply::ProtocolData{data}) = from_bytes::<Reply>(&recvbuf) {
                return Ok(data);
            }

            /*
            match from_bytes::<Reply>(&recvbuf) {
                Ok(reply) => match reply {
                    Reply::ProtocolData { data } => return Ok(data),
                    _ => (),
                },
                _ => (),
            }
            */
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
*/