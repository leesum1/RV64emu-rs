use crate::device_trait::DeviceBase;

pub struct DeviceType {
   pub start: u64,
   pub len: u64,
   pub instance: Box<dyn DeviceBase>,
}

pub struct Bus {
    // devices: Vec<(u64, u64, Rc<dyn DeviceBase>)>,
    devices: Vec<DeviceType>,
}

impl Bus {
    pub fn new() -> Self {
        Bus { devices: vec![] }
    }

    pub fn add_device(&mut self, device: DeviceType) {
        self.devices.push(device);
    }

    pub fn read(&mut self, addr: u64, len: usize) -> u64 {
        Bus::check_aligned(addr, len as u64);
        for device in &mut self.devices {
            if Bus::check_area(device.start, device.len, addr) {
                let r_data = device.instance.do_read(addr - device.start, len);
                return r_data;
            }
        }
        // Handle error when no device is found
        println!("can not find device,read addr{:X}", addr);
        0
    }

    fn check_area(start: u64, len: u64, addr: u64) -> bool {
        (addr >= start) && (addr < start + len)
    }

    fn check_aligned(addr: u64, len: u64) {
        match len {
            1 => (),
            2 => {
                if (addr & 0b1) != 0 {
                    panic!();
                }
            }
            4 => {
                if (addr & 0b11) != 0 {
                    panic!();
                }
            }
            8 => {
                if (addr & 0b111) != 0 {
                    panic!();
                }
            }
            _ => panic!(),
        };
    }

    pub fn write(&mut self, addr: u64, data: u64, len: usize) {
        Bus::check_aligned(addr, len as u64);
        for device in &mut self.devices {
            if Bus::check_area(device.start, device.len, addr) {
                let _w_data = device.instance.do_write(addr - device.start, data, len);
                return;
            }
        }
        // Handle error when no device is found
        println!("can not find device,write addr{:X}", addr);
    }
}

#[cfg(test)]
mod tests_bus {
    use crate::dram::Dram;

    use super::{Bus, DeviceType};

    #[test]
    fn test1() {
        let dram_u = DeviceType {
            start: 0x8000_0000,
            len: 1024,
            instance: Box::new(Dram::new(1024)),
        };

        let mut bus_u = Bus::new();
        bus_u.add_device(dram_u);

        let data = 0xDEADBEEF;
        let data1 = 0xDEADBEEFDEADBEEF_u128;
        let len = 4;

        // write data to dram
        let addr = 0x8000_0000;
        bus_u.write(addr, data, len);
        bus_u.write(addr + 4, data, len);

        // read data from dram
        let result = bus_u.read(addr, len);
        // check if the read data is equal to the written data
        assert_eq!(result, data);

        let result = bus_u.read(addr, 1);
        assert_eq!(result, data & 0xff);
        let result = bus_u.read(addr, 2);
        assert_eq!(result, data & 0xffff);
        let result = bus_u.read(addr, 4);
        assert_eq!(result, data);
        let result = bus_u.read(addr, 8);
        println!("{result:x}\n{data1:x}");
        assert_eq!(result as u128, data1);
    }
}
