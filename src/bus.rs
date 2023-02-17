use crate::{clint::DeviceClint, device_trait::DeviceBase};

pub struct DeviceType {
    pub start: u64,
    pub len: u64,
    pub instance: Box<dyn DeviceBase>,
    pub name: &'static str,
}

pub struct Bus {
    // devices: Vec<(u64, u64, Rc<dyn DeviceBase>)>,
    pub clint: DeviceClint,
    pub devices: Vec<DeviceType>,
}

impl Bus {
    pub fn new(clint: DeviceClint) -> Self {
        Bus {
            devices: vec![],
            clint,
        }
    }
    fn check_area(start: u64, len: u64, addr: u64) -> bool {
        (addr >= start) && (addr < start + len)
    }

    fn check_aligned(addr: u64, len: u64) {
        let mask = match len {
            1 => 0,
            2 => 1,
            4 => 3,
            8 => 7,
            _ => panic!(" addr len err:{len}"),
        };
        // let mask = (1u64 << (len.trailing_zeros())) - 1;
        if (addr & mask) != 0 {
            panic!("misaligned addr{addr:X}");
        }
    }

    pub fn add_device(&mut self, device: DeviceType) {
        self.devices.push(device);
    }

    pub fn read(&mut self, addr: u64, len: usize) -> u64 {
        Bus::check_aligned(addr, len as u64);

        // clint.
        if Bus::check_area(self.clint.start, self.clint.len, addr) {
            return self.clint.instance.do_read(addr - self.clint.start, len);
        }

        self.devices
            .iter_mut()
            .find(|device| Bus::check_area(device.start, device.len, addr))
            .map_or_else(
                || {
                    println!("can not find device,read addr{addr:X}");
                    0
                },
                |device| device.instance.do_read(addr - device.start, len),
            )
    }

    pub fn write(&mut self, addr: u64, data: u64, len: usize) -> u64 {
        Bus::check_aligned(addr, len as u64);

        // clint.
        if Bus::check_area(self.clint.start, self.clint.len, addr) {
            return self
                .clint
                .instance
                .do_write(addr - self.clint.start, data, len);
        }

        self.devices
            .iter_mut()
            .find(|device| Bus::check_area(device.start, device.len, addr))
            .map(|device| device.instance.do_write(addr - device.start, data, len))
            .unwrap_or_else(|| {
                println!("can not find device,read addr{addr:X}");
                0
            })
    }

    pub fn update(&mut self) {
        self.clint.instance.do_update();
        self.devices
            .iter_mut()
            .for_each(|device| device.instance.do_update());
    }
}

impl std::fmt::Display for Bus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = self.devices.iter().map(|device| {
            format_args!(
                "name:{:15} Area:0X{:8X}-->0X{:8X},len:0X{:X}\n",
                device.name,
                device.start,
                device.start + device.len,
                device.len
            )
            .to_string()
        });

        f.write_str("--Device Tree MAP--\n").unwrap();
        x.for_each(|device_str| f.write_str(&device_str).unwrap());

        Ok(())
    }
}

#[cfg(test)]
mod tests_bus {
    

    

    // #[test]
    // fn test1() {
    //     let dram_u = DeviceType {
    //         start: 0x8000_0000,
    //         len: 1024,
    //         instance: Box::new(DeviceDram::new(1024)),
    //         name: "DRAM",
    //     };

    //     let mut bus_u = Bus::new();
    //     bus_u.add_device(dram_u);

    //     let data = 0xDEADBEEF;
    //     let data1 = 0xDEADBEEFDEADBEEF_u128;
    //     let len = 4;

    //     // write data to dram
    //     let addr = 0x8000_0000;
    //     bus_u.write(addr, data, len);
    //     bus_u.write(addr + 4, data, len);

    //     // read data from dram
    //     let result = bus_u.read(addr, len);
    //     // check if the read data is equal to the written data
    //     assert_eq!(result, data);

    //     let result = bus_u.read(addr, 1);
    //     assert_eq!(result, data & 0xff);
    //     let result = bus_u.read(addr, 2);
    //     assert_eq!(result, data & 0xffff);
    //     let result = bus_u.read(addr, 4);
    //     assert_eq!(result, data);
    //     let result = bus_u.read(addr, 8);
    //     println!("{result:x}\n{data1:x}");
    //     assert_eq!(result as u128, data1);
    // }
}
