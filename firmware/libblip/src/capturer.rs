use crate::capture_reply;
use blipper_support::protocol::Reply;

pub struct Capturer {
    pub ts_last_cmd: u32,
    pub timeout: u32,
    pub samplerate: u32,
    pub buf: [u16; 128],
    pub i: usize,
    pub edge: bool,
    pub last_edge: u32,
}

impl Capturer {
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

