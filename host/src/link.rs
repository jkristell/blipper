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
        let mut recvbuf = [0; 1024];

        match self.port.read(&mut recvbuf[..]) {
            Ok(_readlen) => {}
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            Err(e) => eprintln!("{:?}", e),
        }

        if let Ok(Reply::Ok) = from_bytes::<Reply>(&recvbuf) {
            return Ok(())
        }

        Err(io::ErrorKind::InvalidData.into())
    }

    pub fn read_capturerawdata(&mut self) -> io::Result<RawData> {
        let mut recvbuf = [0; 1024];
        let mut offset = 0;

        //FIXME: Figure out how to do this properly.
        while offset < std::mem::size_of::<Reply>() - 3 {
            match self.port.read(&mut recvbuf[offset..]) {
                Ok(readlen) => { offset += readlen; }
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                Err(e) => eprintln!("{:?}", e),
            }
        }

        if let Ok(Reply::CaptureRawData {rawdata}) = from_bytes::<Reply>(&recvbuf) {
            return Ok(rawdata);
        }

        Err(io::ErrorKind::InvalidData.into())
    }
}
