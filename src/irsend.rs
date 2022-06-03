
use blipper_shared::protocol::{Command, Pid, RemoteControlCmd};
use blipper_shared::SerialLink;

pub fn transmit(
    link: &mut SerialLink,
    protocol: Pid,
    addr: u32,
    cmd: u32,
) -> anyhow::Result<()> {
    let rc_cmd = RemoteControlCmd {
        pid: protocol.as_u8(),
        addr: addr as u16,
        cmd: cmd as u8,
    };

    log::info!("Sending command: {:?}", rc_cmd);

    link.send_command(Command::RemoteControlSend(rc_cmd))?;
    link.reply_ok()?;
    log::info!("Got ok");

    Ok(())
}
