use embedded_hal::PwmPin;

use blipper_protocol::{Command, Info, CaptureData, Reply, ProtocolId};
use rtt_target::{rprintln};

use infrared::{
    protocols::{
        rc5::{Rc5Command},
        nec::{NecCommand, NecSamsung, NecStandard}
    },
    hal::HalSender,
};

const VERSION: u32 = 1;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum State {
    Idle,
    CaptureRaw,
    IrSend,
}

pub struct BlipCapturer {
    pub ts_last_cmd: u32,
    pub timeout: u32,
    pub samplerate: u32,
    pub buf: [u16; 128],
    pub i: usize,
    pub edge: bool,
    pub last_edge: u32,
}

impl BlipCapturer {
    pub fn new(samplerate: u32) -> Self {
        Self {
            ts_last_cmd: 0,
            timeout: samplerate / 10,
            samplerate,
            buf: [0; 128],
            i: 0,
            edge: false,
            last_edge: 0,
        }
    }

    pub fn reset(&mut self) {
        self.ts_last_cmd = 0;
        self.i = 0;
        self.edge = false;
        self.last_edge = 0;
    }

    pub fn sample(&mut self, edge: bool, ts: u32) -> Option<Reply> {

        if edge == self.edge {
            if self.i != 0 && ts.wrapping_sub(self.last_edge) > self.timeout {

                let reply = capture_reply(self.samplerate, &self.buf[0..self.i]);
                self.reset();
                return Some(reply);
            }

            return None;
        }

        self.edge = edge;

        self.buf[self.i] = ts.wrapping_sub(self.last_edge) as u16;
        self.i += 1;
        self.last_edge = ts;

        if self.i == self.buf.len() {
            let reply = capture_reply(self.samplerate, &self.buf);
            self.reset();
            return Some(reply);
        }

        None
    }
}

pub struct Transmitters<PWMPIN, DUTY>
    where
        PWMPIN: PwmPin<Duty = DUTY>,
{
    sender: HalSender<PWMPIN, DUTY>,
}

impl<PWMPIN, DUTY> Transmitters<PWMPIN, DUTY>
where
    PWMPIN: PwmPin<Duty = DUTY>,
{
    fn new(pin: PWMPIN, samplerate: u32) -> Self {
        Self {
            sender: HalSender::new(samplerate, pin),
        }
    }

    pub fn load(&mut self, pid: ProtocolId, addr: u16, cmd: u8) {
        match pid {
            ProtocolId::Nec => self.sender.load(&NecCommand::<NecStandard>::new(addr, cmd)),
            ProtocolId::NecSamsung => self.sender.load(&NecCommand::<NecSamsung>::new(addr, cmd)),
            ProtocolId::Rc5 => self.sender.load(&Rc5Command::new(addr as u8, cmd, false)),
        };
    }

    fn step(
        &mut self,
    ) {
        self.sender.tick()
    }
}

pub struct Blip<PWMPIN, DUTY>
    where
        PWMPIN: PwmPin<Duty = DUTY>,
{
    pub state: State,
    pub capturer: BlipCapturer,
    pub txers: Transmitters<PWMPIN, DUTY>,
    pub samplerate: u32,
}

impl<PWMPIN, DUTY> Blip<PWMPIN, DUTY>
    where
        PWMPIN: PwmPin<Duty = DUTY>,
{
    pub fn new(pwmpin: PWMPIN, samplerate: u32) -> Self {
        Blip {
            state: State::Idle,
            capturer: BlipCapturer::new(samplerate),
            txers: Transmitters::new(pwmpin, samplerate),
            samplerate,
        }
    }

    fn irsend(&mut self) {
        self.txers.step();
    }

    pub fn tick(&mut self, timestamp: u32, level: bool) -> Option<Reply> {
        match self.state {
            State::Idle => None,
            State::IrSend => { self.irsend(); None}
            State::CaptureRaw => self.capturer.sample(level, timestamp)
        }
    }

    pub(crate) fn handle_command(&mut self, cmd: Command) -> Reply {
        match cmd {
            Command::Idle => {
                self.state = State::Idle;
                Reply::Ok
            }
            Command::Info => {
                Reply::Info {
                    info: Info {
                        version: VERSION,
                        transmitters: 0, //blip::ENABLED_TRANSMITTERS,
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
                self.txers.load(cmd.pid, cmd.addr, cmd.cmd);
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
