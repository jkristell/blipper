use std::{io, path::Path};

use postcard::{from_bytes, to_vec};
use serialport::{SerialPort, SerialPortInfo};

use crate::protocol::{CaptureData, Command, Reply};

pub struct SerialLink {
    port: Option<Box<dyn SerialPort>>,
}

impl SerialLink {
    pub fn new() -> Self {
        SerialLink { port: None }
    }

    pub fn list_ports() -> Result<Vec<SerialPortInfo>, serialport::Error> {
        serialport::available_ports()
        //.map(|portvec| portvec.iter().map(|port| port.port_name.clone()).collect())
    }

    pub fn connect<P: AsRef<Path>>(&mut self, path: P) -> Result<(), serialport::Error> {
        let path = path.as_ref().to_string_lossy();
        let port = serialport::new(path, 115_200).open()?;

        self.port.replace(port);

        Ok(())
    }

    pub fn send_command(&mut self, cmd: Command) -> io::Result<()> {
        let req: heapless::Vec<u8, 64> = to_vec(&cmd).unwrap();
        self.port
            .as_mut()
            .ok_or(io::ErrorKind::NotConnected)?
            .write_all(&req)
    }

    pub fn read_reply(&mut self) -> io::Result<crate::protocol::Reply> {
        let mut recvbuf = [0; 1024];
        let mut offset = 0;

        let port = self.port.as_mut().ok_or(io::ErrorKind::NotConnected)?;
        //info!("port: {:?}", port.name());

        loop {
            match port.read(&mut recvbuf[offset..]) {
                Ok(readlen) => {
                    offset += readlen;
                }
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => continue,
                Err(e) => {
                    eprintln!("{:?}", e);
                    break;
                }
            }

            let reply = from_bytes::<Reply>(&recvbuf);
            //info!("reply: {:?}", reply);

            if let Ok(reply) = reply {
                match reply {
                    Reply::CaptureReply { data } => {
                        let reply_size = 1 + std::mem::size_of::<CaptureData>();
                        if offset < reply_size {
                            continue;
                        } else {
                            return Ok(Reply::CaptureReply { data });
                        }
                    }
                    _ => return Ok(reply),
                }
            } else {
                break;
            }
        }

        Err(io::ErrorKind::InvalidData.into())
    }

    pub fn reply_ok(&mut self) -> io::Result<()> {
        let mut recvbuf = [0; 1024];

        match self
            .port
            .as_mut()
            .ok_or(io::ErrorKind::NotConnected)?
            .read(&mut recvbuf[..])
        {
            Ok(_readlen) => {}
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            Err(e) => eprintln!("{:?}", e),
        }

        if let Ok(Reply::Ok) = from_bytes::<Reply>(&recvbuf) {
            return Ok(());
        }

        Err(io::ErrorKind::InvalidData.into())
    }
}
