use infrared::{
    protocols::*, 
    ReceiverSM,
    AsRemoteControlButton,
    Protocol, bufrecv::BufferReceiver, protocols::nec::NecSamsung
};

#[derive(Debug)]
pub struct DecodedCommand {
    pub address: u32,
    pub command: u32,
    pub kind: Protocol,
}

pub struct Decoders;

impl Decoders {

    pub fn decode_data(&mut self, edges: &[u16], samplerate: u32) -> Vec<DecodedCommand> {

        let mut rc5: BufferReceiver<Rc5> = BufferReceiver::with_values(&edges, samplerate);
        let mut rc6: BufferReceiver<Rc6> = BufferReceiver::with_values(&edges, samplerate);
        //let mut nec: BufferReceiver<Nec> = BufferReceiver::with_values(&edges, samplerate);
        //let mut nes: BufferReceiver<Nec<NecSamsung>> = BufferReceiver::with_values(&edges, samplerate);
        let mut sbp: BufferReceiver<Sbp> = BufferReceiver::with_values(&edges, samplerate);

        decmd_iter(&mut rc5)
            .chain(decmd_iter(&mut rc6))
            //.chain(decmd_iter(&mut nec))
            //.chain(decmd_iter(&mut nes))
            .chain(decmd_iter(&mut sbp))
            .collect()
    }
}

fn decmd_iter<'a, SM, CMD>(recv: &'a mut BufferReceiver<SM>) -> impl Iterator<Item=DecodedCommand> + 'a
    where
        SM: ReceiverSM<Cmd = CMD>,
        CMD: AsRemoteControlButton,
{
    recv
        .iter()
        .map(|cmd|
            DecodedCommand {
                address: cmd.address(),
                command: cmd.data(),
                kind: cmd.protocol(),
            }
        )
}
