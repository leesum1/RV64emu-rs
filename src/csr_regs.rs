use std::collections::HashMap;

use crate::inst_base::{
    get_field, set_field, PrivilegeLevels, CSR_MCAUSE, CSR_MEPC, CSR_MHARTID, CSR_MIE, CSR_MIP,
    CSR_MSCRATCH, CSR_MSTATUS, CSR_MTVAL, CSR_MTVEC,
};

pub struct CsrRegs {
    pub csr_map: HashMap<u64, Box<dyn CsrRW>>,
}

// enum {
//     mtvec, mie, mip, mtval,mepc, mstatus, mcause, MSCRATCH, mhartid
//   };

impl CsrRegs {
    pub fn new() -> Self {
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
        ];

        let mut csr_map = HashMap::<u64, Box<dyn CsrRW>>::new();
        for csr in csr_list.into_iter() {
            csr_map.insert(csr.addr, Box::new(csr));
        }
        CsrRegs { csr_map }
    }

    pub fn read(&self, addr: u64) -> u64 {
        let t = self.csr_map.get(&addr);

        match t {
            Some(csr) => csr.read(),
            None => todo!(),
        }
    }

    pub fn write(&mut self, addr: u64, val: u64) -> u64 {
        let t = self.csr_map.get_mut(&addr);

        match t {
            Some(csr) => csr.write(val),
            None => todo!(),
        }
    }

    pub fn read_raw_mask(&self, addr: u64, mask: u64) -> u64 {
        let t = self.csr_map.get(&addr);

        match t {
            Some(csr) => csr.read_raw_mask(mask),
            None => todo!(),
        }
    }

    pub fn write_raw_mask(&mut self, addr: u64, val: u64, mask: u64) -> u64 {
        let t = self.csr_map.get_mut(&addr);

        match t {
            Some(csr) => csr.write_raw_mask(val, mask),
            None => todo!(),
        }
    }
}

pub trait CsrRW {
    fn read(&self) -> u64;
    fn write(&mut self, val: u64) -> u64;
    fn write_raw_mask(&mut self, data: u64, mask: u64) -> u64;
    fn read_raw_mask(&self, mask: u64) -> u64;
}

#[derive(Clone)]
pub struct BaseCSR {
    pub addr: u64,
    pub val: u64,
    pub privi_level: PrivilegeLevels,
    pub read_only: bool,
}

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

        println!("{:?}", csr_bus.csr_map.len());
    }
}
