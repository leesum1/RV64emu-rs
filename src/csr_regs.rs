// use std::collections::HashMap;
use hashbrown::HashMap; // faster hashmap

use crate::{inst_base::*, csr_regs_define::Misa};

pub struct CsrRegs {
    // pub csr_map: HashMap<u64, Box<dyn CsrRW>>,
    pub csr_vec: Vec<Box<dyn CsrRW>>,
    pub csr_flag: [bool; 4096],
}

unsafe impl Send for CsrRegs {}

impl CsrRegs {
    pub fn new() -> Self {
        let misa = Misa::new()
            .with_i(true)
            .with_m(true)
            .with_a(true)
            .with_s(true)
            .with_u(true)
            .with_mxl(2); // 64
        let csr_list = vec![
            BaseCSR::new(CSR_MTVEC.into(), 0),
            BaseCSR::new(CSR_MTVAL.into(), 0),
            BaseCSR::new(CSR_MCAUSE.into(), 0),
            BaseCSR::new(CSR_MIP.into(), 0),
            BaseCSR::new(CSR_MIE.into(), 0),
            BaseCSR::new(CSR_MEPC.into(), 0),
            BaseCSR::new(CSR_MSTATUS.into(), 0),
            BaseCSR::new(CSR_MSCRATCH.into(), 0),
            BaseCSR::new(CSR_MHARTID.into(), 0),
            BaseCSR::new(CSR_MISA.into(), misa.into()),
            BaseCSR::new(CSR_PMPCFG0.into(), 0),
            BaseCSR::new(CSR_PMPADDR63.into(), 0),
            BaseCSR::new(CSR_PMPADDR62.into(), 0),
            BaseCSR::new(CSR_PMPADDR61.into(), 0),
            BaseCSR::new(CSR_PMPADDR60.into(), 0),
            BaseCSR::new(CSR_PMPADDR59.into(), 0),
            BaseCSR::new(CSR_PMPADDR58.into(), 0),
            BaseCSR::new(CSR_PMPADDR57.into(), 0),
            BaseCSR::new(CSR_PMPADDR56.into(), 0),
            BaseCSR::new(CSR_PMPADDR55.into(), 0),
            BaseCSR::new(CSR_PMPADDR54.into(), 0),
            BaseCSR::new(CSR_PMPADDR53.into(), 0),
            BaseCSR::new(CSR_PMPADDR52.into(), 0),
            BaseCSR::new(CSR_PMPADDR51.into(), 0),
            BaseCSR::new(CSR_PMPADDR50.into(), 0),
            BaseCSR::new(CSR_PMPADDR49.into(), 0),
            BaseCSR::new(CSR_PMPADDR48.into(), 0),
            BaseCSR::new(CSR_PMPADDR47.into(), 0),
            BaseCSR::new(CSR_PMPADDR46.into(), 0),
            BaseCSR::new(CSR_PMPADDR45.into(), 0),
            BaseCSR::new(CSR_PMPADDR44.into(), 0),
            BaseCSR::new(CSR_PMPADDR43.into(), 0),
            BaseCSR::new(CSR_PMPADDR42.into(), 0),
            BaseCSR::new(CSR_PMPADDR41.into(), 0),
            BaseCSR::new(CSR_PMPADDR40.into(), 0),
            BaseCSR::new(CSR_PMPADDR39.into(), 0),
            BaseCSR::new(CSR_PMPADDR38.into(), 0),
            BaseCSR::new(CSR_PMPADDR37.into(), 0),
            BaseCSR::new(CSR_PMPADDR36.into(), 0),
            BaseCSR::new(CSR_PMPADDR35.into(), 0),
            BaseCSR::new(CSR_PMPADDR34.into(), 0),
            BaseCSR::new(CSR_PMPADDR33.into(), 0),
            BaseCSR::new(CSR_PMPADDR32.into(), 0),
            BaseCSR::new(CSR_PMPADDR31.into(), 0),
            BaseCSR::new(CSR_PMPADDR30.into(), 0),
            BaseCSR::new(CSR_PMPADDR29.into(), 0),
            BaseCSR::new(CSR_PMPADDR28.into(), 0),
            BaseCSR::new(CSR_PMPADDR27.into(), 0),
            BaseCSR::new(CSR_PMPADDR26.into(), 0),
            BaseCSR::new(CSR_PMPADDR25.into(), 0),
            BaseCSR::new(CSR_PMPADDR24.into(), 0),
            BaseCSR::new(CSR_PMPADDR23.into(), 0),
            BaseCSR::new(CSR_PMPADDR22.into(), 0),
            BaseCSR::new(CSR_PMPADDR21.into(), 0),
            BaseCSR::new(CSR_PMPADDR20.into(), 0),
            BaseCSR::new(CSR_PMPADDR19.into(), 0),
            BaseCSR::new(CSR_PMPADDR18.into(), 0),
            BaseCSR::new(CSR_PMPADDR17.into(), 0),
            BaseCSR::new(CSR_PMPADDR16.into(), 0),
            BaseCSR::new(CSR_PMPADDR15.into(), 0),
            BaseCSR::new(CSR_PMPADDR14.into(), 0),
            BaseCSR::new(CSR_PMPADDR13.into(), 0),
            BaseCSR::new(CSR_PMPADDR12.into(), 0),
            BaseCSR::new(CSR_PMPADDR11.into(), 0),
            BaseCSR::new(CSR_PMPADDR10.into(), 0),
            BaseCSR::new(CSR_PMPADDR9.into(), 0),
            BaseCSR::new(CSR_PMPADDR8.into(), 0),
            BaseCSR::new(CSR_PMPADDR7.into(), 0),
            BaseCSR::new(CSR_PMPADDR6.into(), 0),
            BaseCSR::new(CSR_PMPADDR5.into(), 0),
            BaseCSR::new(CSR_PMPADDR4.into(), 0),
            BaseCSR::new(CSR_PMPADDR3.into(), 0),
            BaseCSR::new(CSR_PMPADDR2.into(), 0),
            BaseCSR::new(CSR_PMPADDR1.into(), 0),
            BaseCSR::new(CSR_PMPADDR0.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER3.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER4.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER5.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER6.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER7.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER8.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER9.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER10.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER11.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER12.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER13.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER14.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER15.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER16.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER17.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER18.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER19.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER20.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER21.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER22.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER23.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER24.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER25.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER26.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER27.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER28.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER29.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER30.into(), 0),
            BaseCSR::new(CSR_MHPMCOUNTER31.into(), 0),
            BaseCSR::new(CSR_MCOUNTEREN.into(), 0),
            BaseCSR::new(CSR_MCOUNTINHIBIT.into(), 0),
            BaseCSR::new(CSR_MENVCFG.into(), 0),
            BaseCSR::new(CSR_SCOUNTOVF.into(), 0),
            BaseCSR::new(CSR_TIME.into(), 0),
            BaseCSR::new(CSR_MTOPI.into(), 0),
            BaseCSR::new(CSR_STIMECMP.into(), 0),
            BaseCSR::new(0x30c, 0),
            BaseCSR::new(CSR_MHPMEVENT3.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT4.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT5.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT6.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT7.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT8.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT9.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT10.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT11.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT12.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT13.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT14.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT15.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT16.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT17.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT18.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT19.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT20.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT21.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT22.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT23.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT24.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT25.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT26.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT27.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT28.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT29.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT30.into(), 0),
            BaseCSR::new(CSR_MHPMEVENT31.into(), 0),
            BaseCSR::new(CSR_SCOUNTEREN.into(), 0),
            BaseCSR::new(CSR_SCOUNTEREN.into(), 0),
            BaseCSR::new(CSR_SATP.into(), 0),
            BaseCSR::new(CSR_MIDELEG.into(), 0),
            BaseCSR::new(CSR_MEDELEG.into(), 0),
            BaseCSR::new(CSR_STVEC.into(), 0),
            BaseCSR::new(CSR_STVAL.into(), 0),
            BaseCSR::new(CSR_SCAUSE.into(), 0),
            BaseCSR::new(CSR_SIP.into(), 0),
            BaseCSR::new(CSR_SIE.into(), 0),
            BaseCSR::new(CSR_SEPC.into(), 0),
            BaseCSR::new(CSR_SSTATUS.into(), 0),
            BaseCSR::new(CSR_SSCRATCH.into(), 0),


            








        ];

        let _csr_map = HashMap::<u64, Box<dyn CsrRW>>::new();

        let mut csr_vec: Vec<Box<dyn CsrRW>> = Vec::new();
        let mut csr_flag = [false; 4096];

        for _i in 0..4096 {
            csr_vec.push(Default::default());
        }

        for csr in csr_list.into_iter() {
            csr_vec[csr.addr as usize] = Box::new(csr);
            csr_flag[csr.addr as usize] = true;
            // csr_map.insert(csr.addr, Box::new(csr));
        }

        CsrRegs { csr_vec, csr_flag }
    }

    pub fn check_csr(&self, addr: u64) -> bool {
        self.csr_flag[addr as usize]
    }

    pub fn read(&self, addr: u64) -> u64 {
        // let t = self.csr_map.get(&addr);
        assert!(addr < 4096);

        if !self.check_csr(addr) {
            panic!("csr:{addr:X},can not find");
        };

        let x = self.csr_vec.get(addr as usize);

        match x {
            Some(csr) => csr.read(),
            None => todo!(),
        }
    }

    pub fn write(&mut self, addr: u64, val: u64) -> u64 {
        assert!(addr < 4096);
        if !self.check_csr(addr) {
            panic!("csr:{addr:X},can not find");
        };

        let x = self.csr_vec.get_mut(addr as usize);

        match x {
            Some(csr) => csr.write(val),
            None => todo!(),
        }
    }

    pub fn read_raw_mask(&self, addr: u64, mask: u64) -> u64 {
        let t = self.csr_vec.get(addr as usize);

        match t {
            Some(csr) => csr.read_raw_mask(mask),
            None => todo!(),
        }
    }

    pub fn write_raw_mask(&mut self, addr: u64, val: u64, mask: u64) -> u64 {
        // let t = self.csr_map.get_mut(&addr);
        let t = self.csr_vec.get_mut(addr as usize);

        match t {
            Some(csr) => csr.write_raw_mask(val, mask),
            None => todo!(),
        }
    }
}

impl Default for CsrRegs {
    fn default() -> Self {
        Self::new()
    }
}

pub trait CsrRW {
    fn read(&self) -> u64;
    fn write(&mut self, val: u64) -> u64;
    fn write_raw_mask(&mut self, data: u64, mask: u64) -> u64;
    fn read_raw_mask(&self, mask: u64) -> u64;
}

#[derive(Clone, Copy)]
pub struct BaseCSR {
    pub addr: u64,
    pub val: u64,
    pub privi_level: PrivilegeLevels,
    pub read_only: bool,
}

unsafe impl Sync for BaseCSR {}

impl BaseCSR {
    pub fn new(addr: u64, val: u64) -> Self {
        let priv_l = get_field(addr, 0x300);
        let read_only = get_field(addr, 0xC00) == 3;

        BaseCSR {
            addr,
            val,
            privi_level: PrivilegeLevels::from_repr(priv_l as usize).unwrap(),
            read_only,
        }
    }
}

impl CsrRW for BaseCSR {
    fn read(&self) -> u64 {
        // println!("read {:x},{:x}",self.val, self.addr);
        self.val
    }
    fn write(&mut self, val: u64) -> u64 {
        // println!("write {:x},{:x}",val, self.addr);
        self.val = val;
        val
    }
    fn write_raw_mask(&mut self, data: u64, mask: u64) -> u64 {
        self.val = set_field(self.val, mask, data);
        self.val
    }
    fn read_raw_mask(&self, mask: u64) -> u64 {
        get_field(self.val, mask)
    }
}

impl Default for Box<dyn CsrRW> {
    fn default() -> Self {
        Box::new(BaseCSR::new(0, 0))
    }
}

#[cfg(test)]
mod test_csr {
    use crate::inst_base::{get_field, set_field, PrivilegeLevels, CSR_MTVEC, CSR_STVEC};

    use super::{BaseCSR, CsrRegs};

    #[test]
    fn test5() {
        let reg = 0b1100_1010_1100_1010;
        let mask = 0b0000_1101_0000_0000;
        let result = get_field(reg, mask);
        assert_eq!(result, 0b1000);
        println!("{result:b}"); // 输出: 0b1010

        let y = set_field(reg, 0xf0, 0b1001);

        println!("{reg:b}");
        println!("{y:b}");
    }
    #[test]
    fn tset1() {
        let x = BaseCSR::new(CSR_MTVEC.into(), 0);
        println!("{}", x.privi_level);
        assert_eq!(x.privi_level, PrivilegeLevels::Machine);
        let x = BaseCSR::new(CSR_STVEC.into(), 0);
        println!("{}", x.privi_level);
        assert_eq!(x.privi_level, PrivilegeLevels::Supervisor);
    }
    #[test]
    fn tset2() {
        let mut csr_bus = CsrRegs::new();

        csr_bus.write(CSR_MTVEC.into(), 100);
        let x = csr_bus.read(CSR_MTVEC.into());
        assert_eq!(x, 100);

        csr_bus.write(CSR_MTVEC.into(), 123124);
        let x = csr_bus.read(CSR_MTVEC.into());
        assert_eq!(x, 123124);

        csr_bus.read(11);

        // println!("{:?}", csr_bus.csr_map.len());
    }
}
