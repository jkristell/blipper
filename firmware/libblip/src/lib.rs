#![no_std]

use embedded_hal::PwmPin;

//pub use blipper_protocol::{Command, Info, CaptureData, Reply};
//use rtt_target::{rprintln};

use infrared::{protocol::{
    rc5::{Command as Rc5Command},
    nec::{NecCommand, SamsungNec, Nec}
}, ProtocolId};

pub mod capturer;
use crate::capturer::Capturer;
pub use blipper_shared::protocol::{Command, Reply, Info, CaptureData};

const VERSION: u32 = 1;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum State {
    Idle,
    CaptureRaw,
    IrSend,
}
/* 

pub struct Sender<PWMPIN>
where
    PWMPIN: PwmPin,
{
    sender: infrared::MultiSender<PWMPIN>,
    nec: NecSenderState,
    nes: NecSenderState<NecSamsungCommand>,
    rc5: Rc5SenderState,
    rc6: Rc6SenderState,
}

impl<PWMPIN, DUTY> Sender<PWMPIN>
where
    PWMPIN: PwmPin<Duty = DUTY>,
{
    fn new(pin: PWMPIN, samplerate: u32) -> Self {

        let sender = infrared::MultiSender::new(samplerate, pin);
        let nec = sender.create_state();
        let nes = sender.create_state();
        let rc5 = sender.create_state();
        let rc6 = sender.create_state();

        Self {
            sender,
            nec,
            nes,
            rc5,
            rc6,
        }
    }

    pub fn load(&mut self, pid: ProtocolId, addr: u16, cmd: u8) {
        match pid {
            // Nec
            ProtocolId::Nec => self.sender.load::<Nec>(&mut self.nec, &NecCommand {
                addr: addr as u8,
                cmd,
                repeat: false
            }),

            // Nec Samsung
            ProtocolId::NecSamsung => self.sender.load::<NecSamsung>(&mut self.nes, &NecSamsungCommand {
                addr: addr as u8,
                cmd,
                repeat: false
            }),
            // Rc5
            ProtocolId::Rc5 => self.sender.load::<Rc5>(&mut self.rc5, &Rc5Command::new(addr as u8, cmd, false)),
            _ => (),
        };
    }

    fn step(&mut self) {
        self.sender.tick()
    }
}
*/
pub struct Blip
//where
//    PWMPIN: PwmPin<Duty = DUTY>
{
    pub state: State,
    pub capturer: Capturer,
    //pub sender: Sender<PWMPIN>,
    pub samplerate: u32,
}

impl Blip
//where
//    PWMPIN: PwmPin<Duty = DUTY>
{
    pub fn new(samplerate: u32) -> Self {
        Blip {
            state: State::Idle,
            capturer: Capturer::new(samplerate),
            //sender: Sender::new(pwmpin, samplerate),
            samplerate,
        }
    }

    fn irsend(&mut self) {
        //self.sender.step();
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
                        samplerate: 40_000,
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

                //let protocol_id = cmd.pid.into();

                //self.sender.load(protocol_id, cmd.addr, cmd.cmd);
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

