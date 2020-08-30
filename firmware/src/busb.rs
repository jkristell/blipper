use usb_device::bus;
use usb_device::prelude::*;
use usb_device::class_prelude::*;

struct BlipperClass<B: UsbBus> {
    bus: core::marker::PhantomData<B>,
}

impl<B: UsbBus> UsbClass<B> for BlipperClass<B> {

    // Data to device
    fn control_out(&mut self, xfer: ControlOut<B>) {
        let req = xfer.request();

        if req.request_type == control::RequestType::Vendor
            && req.recipient == control::Recipient::Device
            && req.request == 1
        {
            if req.value > 0 {
                self.led.set_low().ok();
            } else {
                self.led.set_high().ok();
            }
        }
    }


}
