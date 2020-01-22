use std::io;

use log::info;

use blipper_protocol::{Command, RemoteControlCmd};
use blipper_utils::SerialLink;

pub fn transmit(link: &mut SerialLink, protocol: u32, addr: u32, cmd: u32) -> io::Result<()> {
    let rc_cmd = RemoteControlCmd {
        txid: protocol as u8,
        addr: addr as u16,
        cmd: cmd as u8,
    };

    info!("Sending command: {:?}", rc_cmd);

    link.send_command(Command::RemoteControlSend(rc_cmd))?;
    link.reply_ok()?;
    info!("Got ok");

    Ok(())
}
