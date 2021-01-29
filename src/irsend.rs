use std::io;

use blipper_shared::SerialLink;
use blipper_shared::protocol::{RemoteControlCmd, Command};

pub fn transmit(
    link: &mut SerialLink,
    protocol: infrared::ProtocolId,
    addr: u32,
    cmd: u32,
) -> io::Result<()> {
    let rc_cmd = RemoteControlCmd {
        pid: protocol as u8,
        addr: addr as u16,
        cmd: cmd as u8,
    };

    log::info!("Sending command: {:?}", rc_cmd);

    link.send_command(Command::RemoteControlSend(rc_cmd))?;
    link.reply_ok()?;
    log::info!("Got ok");

    Ok(())
}
