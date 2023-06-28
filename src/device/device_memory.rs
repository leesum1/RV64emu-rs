use alloc::{vec::Vec, boxed::Box};
use log::info;

use crate::device::device_trait::DeviceBase;

pub struct DeviceMemory {
    data: Box<[u8]>,
}

impl DeviceMemory {
    pub fn new(size: usize) -> Self {
        let datavec: Vec<u8> = vec![0; size];
        DeviceMemory {
            data: datavec.into_boxed_slice(),
        }
    }
    pub fn from_boxed_slice(data: Box<[u8]>) -> Self {
        DeviceMemory { data }
    }
    pub fn size(&self) -> usize {
        self.data.len()
    }
    pub fn load_binary(&mut self, slice: &[u8]) {
        self.copy_from_slice(0, slice);
        info!("load binary success: {:#x} bytes", slice.len());
    }
}

impl DeviceBase for DeviceMemory {
    fn do_read(&mut self, addr: u64, len: usize) -> u64 {
        let mut data_bytes = 0_u64.to_le_bytes();

        data_bytes[..(len)].copy_from_slice(&self.data[(addr as usize)..(addr as usize + len)]);

        u64::from_le_bytes(data_bytes)
    }
    fn do_write(&mut self, addr: u64, data: u64, len: usize) -> u64 {
        let data_bytes = data.to_le_bytes();

        self.data[(addr as usize)..(addr as usize + len)].copy_from_slice(&data_bytes[..(len)]);
        data
    }
    fn copy_from_slice(&mut self, addr: u64, slice: &[u8]) {
        self.data[(addr as usize)..(addr as usize + slice.len())].copy_from_slice(slice);
    }

    fn copy_to_slice(&mut self, addr: u64, slice: &mut [u8]) {
        slice.copy_from_slice(&self.data[(addr as usize)..(addr as usize + slice.len())]);
    }

    fn get_name(&self) -> &'static str {
        "memory"
    }
}

#[cfg(test)]
mod tests_dram {
    use super::*;

    #[test]
    fn test_do_read() {
        let mut dram = DeviceMemory::new(1024);
        let data = 0xDEADBEEF;
        let data1 = 0xDEADBEEFDEADBEEF_u128;
        let len = 4;

        // write data to dram
        let addr = 0x100;
        dram.do_write(addr, data, len);
        dram.do_write(addr + 4, data, len);

        // read data from dram
        let result = dram.do_read(addr, len);
        // check if the read data is equal to the written data
        assert_eq!(result, data);

        let result = dram.do_read(addr, 1);
        assert_eq!(result, data & 0xff);
        let result = dram.do_read(addr, 2);
        assert_eq!(result, data & 0xffff);
        let result = dram.do_read(addr, 4);
        assert_eq!(result, data);
        let result = dram.do_read(addr, 8);
        // warn!("{:x}\n{:x}", result, data1);
        assert_eq!(result as u128, data1);
    }
    #[test]
    fn test_do_read_slice() {
        let vec_data = vec![0_u8; 1024];
        let mut dram = DeviceMemory::from_boxed_slice(vec_data.into_boxed_slice());

        // let mut dram = DeviceDram::new(1024);
        let data = 0xDEADBEEF;
        let data1 = 0xDEADBEEFDEADBEEF_u128;
        let len = 4;

        // write data to dram
        let addr = 0x100;
        dram.do_write(addr, data, len);
        dram.do_write(addr + 4, data, len);

        // read data from dram
        let result = dram.do_read(addr, len);
        // check if the read data is equal to the written data
        assert_eq!(result, data);

        let result = dram.do_read(addr, 1);
        assert_eq!(result, data & 0xff);
        let result = dram.do_read(addr, 2);
        assert_eq!(result, data & 0xffff);
        let result = dram.do_read(addr, 4);
        assert_eq!(result, data);
        let result = dram.do_read(addr, 8);
        // warn!("{:x}\n{:x}", result, data1);
        assert_eq!(result as u128, data1);
    }
}
