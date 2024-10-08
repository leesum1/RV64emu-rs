use core::cmp::max;

use alloc::vec::Vec;
use alloc::{boxed::Box, string::ToString};
use log::warn;

use crate::tools::{check_aligned, check_area};
use crate::{
    device::{
        device_sifive_clint::{Clint, DeviceClint},
        device_sifive_plic::{DevicePlic, SifvePlic},
        device_trait::DeviceBase,
    },
    rv64core::inst::inst_rv64a::LrScReservation,
};

use super::inst::inst_base::RVerr;

pub struct DeviceType {
    pub start: u64,
    pub len: u64,
    pub instance: Box<dyn DeviceBase>,
    pub name: &'static str,
}

unsafe impl Send for DeviceType {}

pub struct Bus {
    pub clint: DeviceClint,
    pub plic: DevicePlic,
    pub devices: Vec<DeviceType>,
    pub lr_sc_set: LrScReservation, // for rv64a inst
}

unsafe impl Send for Bus {}

impl Bus {
    pub fn new() -> Self {
        let plic = DevicePlic {
            start: 0x0C00_0000,
            len: 0x0400_0000,
            instance: SifvePlic::new(),
            name: "PLIC",
        };

        let clint = DeviceClint {
            start: 0x0200_0000,
            len: 0x0001_0000,
            instance: Clint::new(),
            name: "CLINT",
        };

        Bus {
            devices: vec![],
            clint,
            plic,
            lr_sc_set: LrScReservation::new(),
        }
    }

    pub fn add_device(&mut self, device: DeviceType) {
        self.devices.push(device);
    }

    pub fn read(&mut self, addr: u64, len: usize) -> Result<u64, RVerr> {
        if !check_aligned(addr, len) {
            warn!("bus read:{:x},{:x}", addr, len);
            return Err(RVerr::AddrMisalign);
        }

        // special devices
        // such as clint
        let mut special_device = || -> Result<u64, RVerr> {
            if check_area(self.clint.start, self.clint.len, addr) {
                Ok(self.clint.instance.do_read(addr - self.clint.start, len))
            } else if check_area(self.plic.start, self.plic.len, addr) {
                Ok(self.plic.instance.do_read(addr - self.plic.start, len))
            } else {
                warn!("can not find device,read addr{addr:X}");
                // panic!("can not find device,read addr{addr:X}");
                Err(RVerr::NotFindDevice)
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

    pub fn write(&mut self, addr: u64, data: u64, len: usize) -> Result<u64, RVerr> {
        if !check_aligned(addr, len) {
            return Err(RVerr::AddrMisalign);
        }

        let mut special_device = || -> Result<u64, RVerr> {
            if check_area(self.clint.start, self.clint.len, addr) {
                Ok(self
                    .clint
                    .instance
                    .do_write(addr - self.clint.start, data, len))
            } else if check_area(self.plic.start, self.plic.len, addr) {
                Ok(self
                    .plic
                    .instance
                    .do_write(addr - self.plic.start, data, len))
            } else {
                warn!("can not find device,read addr{addr:X}");
                // panic!("can not find device,read addr{addr:X}");

                Err(RVerr::NotFindDevice)
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

    pub fn copy_from_slice(&mut self, addr: u64, data: &[u8]) -> Result<(), RVerr> {
        let mut special_device = || -> Result<(), RVerr> {
            if check_area(self.clint.start, self.clint.len, addr) {
                self.clint
                    .instance
                    .copy_from_slice(addr - self.clint.start, data);
                Ok(())
            } else if check_area(self.plic.start, self.plic.len, addr) {
                self.plic
                    .instance
                    .copy_from_slice(addr - self.plic.start, data);
                Ok(())
            } else {
                warn!("can not find device,read addr{addr:X}");
                // panic!("can not find device,read addr{addr:X}");

                Err(RVerr::NotFindDevice)
            }
        };

        let general_device = self
            .devices
            .iter_mut()
            .find(|device| check_area(device.start, device.len, addr))
            .map(|device| device.instance.copy_from_slice(addr - device.start, data));

        match general_device {
            Some(val) => Ok(val),
            None => special_device(),
        }
    }

    pub fn copy_to_slice(&mut self, addr: u64, data: &mut [u8]) -> Result<(), RVerr> {
        let general_device = self
            .devices
            .iter_mut()
            .find(|device| check_area(device.start, device.len, addr))
            .map(|device| device.instance.copy_to_slice(addr - device.start, data));

        let mut special_device = || -> Result<(), RVerr> {
            if check_area(self.clint.start, self.clint.len, addr) {
                self.clint
                    .instance
                    .copy_to_slice(addr - self.clint.start, data);
                Ok(())
            } else if check_area(self.plic.start, self.plic.len, addr) {
                self.plic
                    .instance
                    .copy_to_slice(addr - self.plic.start, data);
                Ok(())
            } else {
                warn!("can not find device,read addr{addr:X}");
                // panic!("can not find device,read addr{addr:X}");

                Err(RVerr::NotFindDevice)
            }
        };

        match general_device {
            Some(val) => Ok(val),
            None => special_device(),
        }
    }

    pub fn update(&mut self, interval_cycle: usize) {
        self.devices
            .iter_mut()
            .for_each(|device| device.instance.do_update());
        self.clint.instance.tick(max(interval_cycle / 10, 1));
        self.plic.instance.tick();
    }
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Display for Bus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
        f.write_fmt(format_args!(
            "name:{:15} Area:0X{:08X}-->0X{:08X},len:0X{:08X}\n",
            self.plic.name,
            self.plic.start,
            self.plic.start + self.plic.len,
            self.plic.len
        ))
        .unwrap();

        x.for_each(|device_str| f.write_str(&device_str).unwrap());
        Ok(())
    }
}
