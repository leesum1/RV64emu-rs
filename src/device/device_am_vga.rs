use std::sync::Mutex;

use alloc::sync::Arc;

use crate::device::device_trait::DeviceBase;

const VGA_H: usize = 300;
const VGA_W: usize = 400;
pub const VGA_BUF_SIZE: usize = VGA_H * VGA_W * 4;

pub struct DeviceVGA {
    pix_buff: Arc<Mutex<Box<[u8]>>>,
}

impl DeviceVGA {
    pub fn new(pix_buff: Arc<Mutex<Box<[u8]>>>) -> Self {
        DeviceVGA { pix_buff }
    }

    pub fn get_size() -> usize {
        VGA_BUF_SIZE
    }

    pub fn updata_vga(&mut self) {}
}

impl DeviceBase for DeviceVGA {
    fn do_read(&mut self, addr: u64, len: usize) -> u64 {
        let mut data_bytes = 0_u64.to_le_bytes();

        let binding = self.pix_buff.lock().unwrap();
        data_bytes[..(len)].copy_from_slice(&binding[(addr as usize)..(addr as usize + len)]);

        u64::from_le_bytes(data_bytes)
    }

    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64 {
        let data_bytes = data.to_le_bytes();
        let mut binding = self.pix_buff.lock().unwrap();
        binding[(addr as usize)..(addr as usize + len)].copy_from_slice(&data_bytes[..(len)]);
        drop(binding);
        data
    }

    fn do_update(&mut self) {}

    fn get_name(&self) -> &'static str {
        "AM_VGA_FB"
    }
}
