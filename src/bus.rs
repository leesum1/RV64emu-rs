use crate::{
    device::{
        device_sifive_plic::{DevicePlic, SifvePlic},
        device_trait::{DeviceBase, DeviceEnume},
    },
    inst::inst_base::{check_aligned, check_area},
    sifive_clint::DeviceClint,
};

pub struct DeviceType {
    pub start: u64,
    pub len: u64,
    pub instance: DeviceEnume,
    pub name: &'static str,
}

unsafe impl Send for DeviceType {}

pub struct Bus {
    pub clint: DeviceClint,
    pub plic: DevicePlic,
    pub devices: Vec<DeviceType>,
}
unsafe impl Send for Bus {}

impl Bus {
    pub fn new(clint: DeviceClint) -> Self {
        let plic = DevicePlic {
            start: 0x0C00_0000,
            len: 0x30_0008,
            instance: SifvePlic::new(),
            name: "PLIC",
        };

        Bus {
            devices: vec![],
            clint,
            plic,
        }
    }

    pub fn add_device(&mut self, device: DeviceType) {
        self.devices.push(device);
    }

    pub fn read(&mut self, addr: u64, len: usize) -> Result<u64, ()> {
        if !check_aligned(addr, len as u64) {
            println!("bus read:{:x},{:x}", addr, len);
            return Err(());
        }

        // special devices
        // such as clint
        let special_device = || -> Result<u64, ()> {
            if check_area(self.clint.start, self.clint.len, addr) {
                Ok(self.clint.instance.do_read(addr - self.clint.start, len))
            } else {
                println!("can not find device,read addr{addr:X}");
                Err(())
            }
        };

        // general devices
        // suce as uart mouse vga kb
        let general_device = self
            .devices
            .iter_mut()
            .find(|device| check_area(device.start, device.len, addr))
            .map(|device| device.instance.do_read(addr - device.start, len));

        // first find general devices
        match general_device {
            Some(val) => Ok(val),
            None => special_device(),
        }
    }

    pub fn write(&mut self, addr: u64, data: u64, len: usize) -> Result<u64, ()> {
        if !check_aligned(addr, len as u64) {
            return Err(());
        }

        let mut special_device = || -> Result<u64, ()> {
            if check_area(self.clint.start, self.clint.len, addr) {
                Ok(self
                    .clint
                    .instance
                    .do_write(addr - self.clint.start, data, len))
            } else {
                println!("can not find device,read addr{addr:X}");
                Err(())
            }
        };

        let general_device = self
            .devices
            .iter_mut()
            .find(|device| check_area(device.start, device.len, addr))
            .map(|device| device.instance.do_write(addr - device.start, data, len));

        match general_device {
            Some(val) => Ok(val),
            None => special_device(),
        }
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
                "name:{:15} Area:0X{:08X}-->0X{:08X},len:0X{:08X}\n",
                device.name,
                device.start,
                device.start + device.len,
                device.len
            )
            .to_string()
        });

        f.write_str("-------------Device Tree MAP-------------\n")
            .unwrap();
        f.write_fmt(format_args!(
            "name:{:15} Area:0X{:08X}-->0X{:08X},len:0X{:08X}\n",
            self.clint.name,
            self.clint.start,
            self.clint.start + self.clint.len,
            self.clint.len
        ))
        .unwrap();
        x.for_each(|device_str| f.write_str(&device_str).unwrap());

        Ok(())
    }
}
