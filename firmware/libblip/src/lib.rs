#![no_std]

use embedded_hal::PwmPin;

pub use blipper_protocol::{Command, Info, CaptureData, Reply};
//use rtt_target::{rprintln};

use infrared::{
    protocols::{
        rc5::{Rc5Command},
        nec::{NecCommand, NecSamsung, NecStandard}
    },
    Protocol,
    hal::HalSender,
};

pub mod capturer;
use crate::capturer::Capturer;

const VERSION: u32 = 1;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum State {
    Idle,
    CaptureRaw,
    IrSend,
}

pub struct Sender<PWMPIN, DUTY>
where
    PWMPIN: PwmPin<Duty = DUTY>,
{
    sender: HalSender<PWMPIN, DUTY>,
}

impl<PWMPIN, DUTY> Sender<PWMPIN, DUTY>
where
    PWMPIN: PwmPin<Duty = DUTY>,
{
    fn new(pin: PWMPIN, samplerate: u32) -> Self {
        Self {
            sender: HalSender::new(samplerate, pin),
        }
    }

    pub fn load(&mut self, pid: Protocol, addr: u16, cmd: u8) {
        match pid {
            Protocol::Nec => self.sender.load(&NecCommand::<NecStandard>::new(addr, cmd)),
            Protocol::NecSamsung => self.sender.load(&NecCommand::<NecSamsung>::new(addr, cmd)),
            Protocol::Rc5 => self.sender.load(&Rc5Command::new(addr as u8, cmd, false)),
            _ => Ok(()),
        }.ok();
    }

    fn step(&mut self) {
        self.sender.tick()
    }
}

pub struct Blip<PWMPIN, DUTY>
where
    PWMPIN: PwmPin<Duty = DUTY>
{
    pub state: State,
    pub capturer: Capturer,
    pub sender: Sender<PWMPIN, DUTY>,
    pub samplerate: u32,
}

impl<PWMPIN, DUTY> Blip<PWMPIN, DUTY>
where
    PWMPIN: PwmPin<Duty = DUTY>
{
    pub fn new(pwmpin: PWMPIN, samplerate: u32) -> Self {
        Blip {
            state: State::Idle,
            capturer: Capturer::new(samplerate),
            sender: Sender::new(pwmpin, samplerate),
            samplerate,
        }
    }

    fn irsend(&mut self) {
        self.sender.step();
    }

    pub fn tick(&mut self, timestamp: u32, level: bool) -> Option<Reply> {
        match self.state {
            State::Idle => None,
            State::IrSend => { self.irsend(); None}
            State::CaptureRaw => self.capturer.sample(level, timestamp)
        }
    }

    pub fn handle_command(&mut self, cmd: Command) -> Reply {
        match cmd {
            Command::Idle => {
                self.state = State::Idle;
                Reply::Ok
            }
            Command::Info => {
                Reply::Info {
                    info: Info {
                        version: VERSION,
                        transmitters: 0,
                    },
                }
            }
            Command::Capture => {
                self.capturer.reset();
                self.state = State::CaptureRaw;
                Reply::Ok
            }
            Command::CaptureProtocol(_id) => {
                Reply::Ok
            }
            Command::RemoteControlSend(cmd) => {
                self.sender.load(Protocol::from(cmd.pid), cmd.addr, cmd.cmd);
                self.state = State::IrSend;
                Reply::Ok
            }
        }
    }

}

fn capture_reply(samplerate: u32, buf: &[u16]) -> Reply {
    let mut data = CaptureData {
        samplerate,
        bufs: [[0; 32]; 4],
        len: buf.len() as u32,
    };

    for i in 0..buf.len() {
        let idx = i % 32;
        let bufidx = i / 32;
        data.bufs[bufidx][idx] = buf[i];
    }

    Reply::CaptureReply { data }
}


pub fn cmd_from_bytes(buf: &[u8]) -> Option<Command> {
    postcard::from_bytes::<Command>(buf).ok()
}

