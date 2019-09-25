use std::io;
use log::info;
use common::{Command, RemoteControlCmd};
use crate::link::SerialLink;

pub fn transmit(link: &mut SerialLink,
                _protocol: u32,
                addr: u32,
                cmd: u32,
) -> io::Result<()> {

    let rc_cmd = RemoteControlCmd {
        addr: addr as u16,
        cmd: cmd as u16
    };

    info!("Irsend");

    link.send_command(Command::RemoteControlSend(rc_cmd))?;
    link.reply_ok()?;

    info!("Got ok");

    Ok(())
}
