// use std::collections::HashMap;
// faster hashmap

use crate::{
    csr_regs_define::{CsrAddr, Misa, Mstatus, Sie, Sip, Sstatus},
    inst::inst_base::*,
    traptype::TrapType,
};

pub struct CsrMask {
    csr_address: u16,
    redirect_addr: u16,
    read_mask: u64,
    write_mask: u64,
}

impl CsrMask {
    pub fn new(csr_address: u16, redirect_addr: u16, read_mask: u64, write_mask: u64) -> Self {
        CsrMask {
            csr_address,
            redirect_addr,
            read_mask,
            write_mask,
        }
    }
}

pub struct CsrRegs {
    // pub csr_map: HashMap<u64, Box<dyn CsrRW>>,
    pub csr_mask: Vec<CsrMask>,
    pub csr_flag: [bool; 4096],
    pub csr_map: [u64; 4096],
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
        let mstatus = Mstatus::new().with_uxl(misa.mxl()).with_sxl(misa.mxl());

        // println!("{:x}", u64::from(mstatus));

        let csr_list = vec![
            BaseCSR::new(CSR_MTVEC.into(), 0),
            BaseCSR::new(CSR_MTVAL.into(), 0),
            BaseCSR::new(CSR_MCAUSE.into(), 0),
            BaseCSR::new(CSR_MIP.into(), 0),
            BaseCSR::new(CSR_MIE.into(), 0),
            BaseCSR::new(CSR_MEPC.into(), 0),
            BaseCSR::new(CSR_MSTATUS.into(), mstatus.into()),
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
            BaseCSR::new(CSR_CYCLE.into(), 0),
            BaseCSR::new(CSR_MIMPID.into(), 0),
            BaseCSR::new(CSR_MARCHID.into(), 0),
            BaseCSR::new(CSR_MVENDORID.into(), 0),
        ];

        let sstatus_mask: u64 = Sstatus::new()
            .with_sie(true)
            .with_spie(true)
            .with_ube(true)
            .with_spp(true)
            .with_vs(0b11)
            .with_fs(0b11)
            .with_xs(0b11)
            .with_sum(true)
            .with_mxr(true)
            .with_uxl(0b11)
            .with_sd(true)
            .into();
        let sip_mask: u64 = Sip::new()
            .with_ssip(true)
            .with_stip(true)
            .with_seip(true)
            .into();
        let sie_mask: u64 = Sie::new()
            .with_ssie(true)
            .with_stie(true)
            .with_seie(true)
            .into();

        // todo! add more
        let mstatus_mask: u64 = Mstatus::new().with_uxl(0b11).with_sxl(0b11).into();

        let csr_mask: Vec<CsrMask> = vec![
            CsrMask::new(CSR_SSTATUS, CSR_MSTATUS, sstatus_mask, sstatus_mask),
            CsrMask::new(CSR_SIP, CSR_MIP, sip_mask, sip_mask),
            CsrMask::new(CSR_SIE, CSR_MIE, sie_mask, sie_mask),
            // misa is read only,can not modify
            CsrMask::new(CSR_MISA, CSR_MISA, MASK_ALL, !MASK_ALL),
            CsrMask::new(CSR_MSTATUS, CSR_MSTATUS, MASK_ALL, !mstatus_mask),
        ];

        // is csr register exist ?
        let mut csr_flag = [false; 4096];
        let mut csr_map = [0_u64; 4096];

        for csr in csr_list.into_iter() {
            csr_flag[csr.addr as usize] = true;
            csr_map[csr.addr as usize] = csr.read();
        }

        CsrRegs {
            csr_flag,
            csr_map,
            csr_mask,
        }
    }

    // CSR address (csr[11:8]) are used to encode the read and
    // write accessibility of the CSRs according to privilege level as shown in Table 2.1. The top two bits
    // (csr[11:10]) indicate whether the register is read/write (00, 01, or 10) or read-only (11). The next
    // two bits (csr[9:8]) encode the lowest privilege level that can access the CSR.

    // Attempts to access a non-existent CSR raise an illegal instruction exception. Attempts to access a
    // CSR without appropriate privilege level or to write a read-only register also raise illegal instruction
    // exceptions. A read/write register might also contain some bits that are read-only, in which case
    // writes to the read-only bits are ignored.
    fn check_csr(&self, addr: u64, privi: PrivilegeLevels, access_type: AccessType) -> bool {
        let csr_exist = self.csr_flag[addr as usize];

        let csr_addr = CsrAddr::from(addr);
        csr_addr.check_privilege(privi, access_type) & csr_exist
    }

    pub fn read(&self, addr: u64, privi: PrivilegeLevels) -> Result<u64, TrapType> {
        assert!(addr < 4096);

        if !self.check_csr(addr, privi, AccessType::Load) {
            return Err(TrapType::IllegalInstruction);
        };

        match self.read_csr_mask(addr) {
            Some(val) => Ok(val),
            None => Ok(self.csr_map[addr as usize]),
        }
    }

    pub fn write(&mut self, addr: u64, val: u64, privi: PrivilegeLevels) -> Result<u64, TrapType> {
        assert!(addr < 4096);
        if !self.check_csr(addr, privi, AccessType::Store) {
            return Err(TrapType::IllegalInstruction);
        };

        match self.write_csr_mask(addr, val) {
            Some(val) => Ok(val),
            None => {
                self.csr_map[addr as usize] = val;
                Ok(val)
            }
        }
    }

    fn read_csr_mask(&self, addr: u64) -> Option<u64> {
        self.csr_mask
            .iter()
            .find(|item| item.csr_address as u64 == addr)
            .map(|x| self.csr_map[x.redirect_addr as usize] & x.read_mask)
    }

    fn write_csr_mask(&mut self, addr: u64, val: u64) -> Option<u64> {
        self.csr_mask
            .iter_mut()
            .find(|item| item.csr_address as u64 == addr)
            .map(|x| {
                let pre_val = self.csr_map[x.redirect_addr as usize];
                // println!("addr:{:x},val:{:x}", addr, val);
                // println!("pre:{:x}", pre_val);
                let next_val = (pre_val & !x.write_mask) | (val & x.write_mask);
                // println!("next:{:x}", next_val);
                // println!("mask:{:x}", x.wmask);
                self.csr_map[x.redirect_addr as usize] = next_val;
                next_val
            })
    }

    // no privilege check
    pub fn read_raw(&self, csr_addr: u64) -> u64 {
        assert!(csr_addr < 4096);
        self.csr_map[csr_addr as usize]
    }
    // no privilege check
    pub fn write_raw(&mut self, csr_addr: u64, val: u64) -> u64 {
        assert!(csr_addr < 4096);
        self.csr_map[csr_addr as usize] = val;
        val
    }
}

#[derive(Clone, Copy)]
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
    pub fn read(&self) -> u64 {
        self.val
    }
}

#[cfg(test)]
mod test_csr {}
