use std::{
    mem::MaybeUninit,
    rc::Rc,
};

use ringbuf::{LocalRb, Producer, Rb};

use crate::device_trait::DeviceBase;

type VgaCtlSender = Producer<bool, Rc<LocalRb<bool, Vec<MaybeUninit<bool>>>>>;
pub struct DeviceVGACTL {
    // Consumer<bool, Rc<LocalRb<bool, Vec<MaybeUninit<bool>>>>>,
    tx: VgaCtlSender,
}

impl DeviceVGACTL {
    pub fn new(tx: VgaCtlSender) -> Self {
        DeviceVGACTL { tx }
    }
}

impl DeviceBase for DeviceVGACTL {
    fn do_read(&mut self, _addr: u64, _len: usize) -> u64 {
        panic!("should not read !!");
    }

    fn do_write(&mut self, addr: u64, _data: u64, len: usize) -> u64 {
        assert_eq!(addr, 4);
        assert_eq!(len, 4);
        self.tx.push(true).unwrap();
        0
    }

    fn get_name(&self) -> &'static str {
        "VGA_CTL"
    }
}
