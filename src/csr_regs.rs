// use std::collections::HashMap;
// faster hashmap

use crate::{
    csr_regs_define::{CsrAddr, Mcounteren, Misa, Mstatus, Sie, Sip, Sstatus, Stap},
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
    pub cur_priv: PrivilegeLevels,
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
        /************** Mathine Level CSRs *****************/
        // Machine Base CSRs
        let mbase_csrs = vec![
            // read only zero
            BaseCSR::new(CSR_MISA.into(), misa.into()),
            BaseCSR::new(CSR_MVENDORID.into(), 0),
            BaseCSR::new(CSR_MARCHID.into(), 0),
            BaseCSR::new(CSR_MIMPID.into(), 0),
            BaseCSR::new(CSR_MHARTID.into(), 0),
            // important m csrs
            BaseCSR::new(CSR_MSTATUS.into(), mstatus.into()),
            BaseCSR::new(CSR_MTVEC.into(), 0),
            BaseCSR::new(CSR_MEDELEG.into(), 0),
            BaseCSR::new(CSR_MIDELEG.into(), 0),
            BaseCSR::new(CSR_MIE.into(), 0),
            BaseCSR::new(CSR_MIP.into(), 0),
            BaseCSR::new(CSR_MSCRATCH.into(), 0),
            BaseCSR::new(CSR_MEPC.into(), 0),
            BaseCSR::new(CSR_MCAUSE.into(), 0),
            BaseCSR::new(CSR_MTVAL.into(), 0),
        ];
        // Machine Configuration CSRs
        let m_conf_csrs = vec![
            BaseCSR::new(CSR_MCONFIGPTR.into(), 0),
            BaseCSR::new(CSR_MENVCFG.into(), 0),
            BaseCSR::new(CSR_MSECCFG.into(), 0),
        ];

        // Hardware Performance Monitor CSRs
        let mut m_monitor_csrs = vec![
            BaseCSR::new(CSR_MCOUNTEREN.into(), 0),
            BaseCSR::new(CSR_MCOUNTINHIBIT.into(), 0),
            BaseCSR::new(CSR_MCYCLE.into(), 0),
            BaseCSR::new(CSR_MINSTRET.into(), 0),
        ];

        for mhpmcounter in (CSR_MHPMCOUNTER3..=CSR_MHPMCOUNTER31).step_by(2) {
            println!("mhpmcounter: {:x}", mhpmcounter);
            m_monitor_csrs.push(BaseCSR::new(mhpmcounter.into(), 0));
        }
        for mhpmevent in (CSR_MHPMEVENT3..=CSR_MHPMEVENT31).step_by(2) {
            println!("mhpmevent: {:x}", mhpmevent);
            m_monitor_csrs.push(BaseCSR::new(mhpmevent.into(), 0));
        }

        // Physical Memory Protection CSRs
        let mut m_pmp_csrs: Vec<BaseCSR> = Vec::new();
        for pmpcfg in (CSR_PMPCFG0..=CSR_PMPCFG15).step_by(2) {
            println!("pmpcfg: {:x}", pmpcfg);
            m_pmp_csrs.push(BaseCSR::new(pmpcfg.into(), 0));
        }
        for pomaddr in CSR_PMPADDR0..=CSR_PMPADDR63 {
            println!("pomaddr: {:x}", pomaddr);
            m_pmp_csrs.push(BaseCSR::new(pomaddr.into(), 0));
        }

        let m_csrs = mbase_csrs
            .into_iter()
            .chain(m_conf_csrs.into_iter())
            .chain(m_monitor_csrs.into_iter())
            .chain(m_pmp_csrs.into_iter())
            .collect::<Vec<BaseCSR>>();
        /************** Supervisor Level CSRs *****************/

        let sbase_csrs = vec![
            BaseCSR::new(CSR_SSTATUS.into(), 0),
            BaseCSR::new(CSR_STVEC.into(), 0),
            BaseCSR::new(CSR_SIP.into(), 0),
            BaseCSR::new(CSR_SIE.into(), 0),
            BaseCSR::new(CSR_SSCRATCH.into(), 0),
            BaseCSR::new(CSR_SEPC.into(), 0),
            BaseCSR::new(CSR_SCAUSE.into(), 0),
            BaseCSR::new(CSR_STVAL.into(), 0),
            BaseCSR::new(CSR_SATP.into(), 0),
            // BaseCSR::new(CSR_SEDELEG.into(), 0), // not exist
            // BaseCSR::new(CSR_SIDELEG.into(), 0),
        ];
        // Supervisor Timers and Performance Counters
        let mut s_monitor_csrs = vec![
            BaseCSR::new(CSR_SCOUNTEREN.into(), 0),
            BaseCSR::new(CSR_TIME.into(), 0),
            BaseCSR::new(CSR_CYCLE.into(), 0),
            BaseCSR::new(CSR_INSTRET.into(), 0),
        ];

        for shpmcounter in (CSR_HPMCOUNTER3..=CSR_HPMCOUNTER31).step_by(2) {
            println!("shpmcounter: {:x}", shpmcounter);
            s_monitor_csrs.push(BaseCSR::new(shpmcounter.into(), 0));
        }
        // Supervisor cfg CSR
        let s_cfg_csrs = vec![BaseCSR::new(CSR_SENVCFG.into(), 0)];

        let s_csrs = sbase_csrs
            .into_iter()
            .chain(s_monitor_csrs.into_iter())
            .chain(s_cfg_csrs.into_iter())
            .collect::<Vec<BaseCSR>>();

        /************** ALL Level CSRs *****************/
        let csr_lists = m_csrs
            .into_iter()
            .chain(s_csrs.into_iter())
            .collect::<Vec<BaseCSR>>();

        // is csr register exist ?
        let mut csr_flag = [false; 4096];
        let mut csr_map = [0_u64; 4096];

        for csr in csr_lists.into_iter() {
            csr_flag[csr.addr as usize] = true;
            csr_map[csr.addr as usize] = csr.read();
        }

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
        let counten_mask: u64 = Mcounteren::new()
            .with_ir(true)
            .with_tm(true)
            .with_cy(true)
            .into();

        let mut csr_mask: Vec<CsrMask> = vec![
            CsrMask::new(CSR_SSTATUS, CSR_MSTATUS, sstatus_mask, sstatus_mask),
            CsrMask::new(CSR_SIP, CSR_MIP, sip_mask, sip_mask),
            CsrMask::new(CSR_SIE, CSR_MIE, sie_mask, sie_mask),
            // misa is read only,can not modify
            CsrMask::new(CSR_MISA, CSR_MISA, MASK_ALL, MASK_NONE),
            CsrMask::new(CSR_MSTATUS, CSR_MSTATUS, MASK_ALL, !mstatus_mask),
            CsrMask::new(CSR_SATP, CSR_SATP, MASK_ALL, MASK_ALL),
            //
            CsrMask::new(CSR_MCOUNTEREN, CSR_MCOUNTEREN, counten_mask, counten_mask),
            CsrMask::new(CSR_SCOUNTEREN, CSR_SCOUNTEREN, counten_mask, counten_mask),
            CsrMask::new(CSR_MCOUNTINHIBIT, CSR_MCOUNTINHIBIT, MASK_NONE, MASK_NONE),
            CsrMask::new(CSR_CYCLE, CSR_MCYCLE, MASK_ALL, MASK_ALL),
            CsrMask::new(CSR_INSTRET, CSR_MINSTRET, MASK_ALL, MASK_ALL),
        ];

        for i in 0..15 {
            csr_mask.push(CsrMask::new(
                CSR_HPMCOUNTER3 + i * 2,
                CSR_MHPMCOUNTER3 + i * 2,
                MASK_ALL,
                MASK_ALL,
            ));
        }

        CsrRegs {
            csr_flag,
            csr_map,
            csr_mask,
            cur_priv: PrivilegeLevels::Machine,
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
        if addr == CSR_CYCLE.into() {
            println!("cycle is not implemented");
        }
        csr_addr.check_privilege(privi, access_type) & csr_exist
    }

    pub fn read(&mut self, addr: u64, privi: PrivilegeLevels) -> Result<u64, TrapType> {
        assert!(addr < 4096);
        self.cur_priv = privi;
        if !self.check_csr(addr, privi, AccessType::Load(0)) {
            return Err(TrapType::IllegalInstruction(0));
        };

        match self.read_csr_mask(addr) {
            Some(val) => Ok(val),
            None => Ok(self.csr_map[addr as usize]),
        }
    }

    pub fn write(&mut self, addr: u64, val: u64, privi: PrivilegeLevels) -> Result<u64, TrapType> {
        assert!(addr < 4096);
        self.cur_priv = privi;
        if !self.check_csr(addr, privi, AccessType::Store(0)) {
            return Err(TrapType::IllegalInstruction(0));
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
                if x.csr_address == CSR_SATP {
                    let stap_val = Stap::from(next_val);
                    if stap_val.unsupport_mod() {
                        return 0;
                    }
                }

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
}

impl BaseCSR {
    pub fn new(addr: u64, val: u64) -> Self {
        BaseCSR { addr, val }
    }
    pub fn read(&self) -> u64 {
        self.val
    }
}

#[cfg(test)]
mod test_csr {}
