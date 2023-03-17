use std::{cell::Cell, rc::Rc};

use crate::{
    bus::Bus,
    csr_regs_define::{Mstatus, Stap, StapMode},
    inst::inst_base::{check_aligned, AccessType, PrivilegeLevels},
    sifive_clint::{Clint, DeviceClint},
    sv39::{Sv39PTE, Sv39Pa, Sv39Va},
    traptype::TrapType,
};

pub struct Mmu {
    pub bus: Bus,
    pub access_type: AccessType,
    mstatus: Mstatus,
    stap: Stap,
    cur_priv: Rc<Cell<PrivilegeLevels>>,
    i: i8,
    level: i8,
    pagesize: u64,
    a: u64,
    va: Sv39Va,
    pub pa: Sv39Pa,
    pte: Sv39PTE,
}

impl Mmu {
    pub fn new(privilege: Rc<Cell<PrivilegeLevels>>) -> Self {
        let clint_u = Clint::new();
        let device_clint = DeviceClint {
            start: 0x2000000,
            len: 0xBFFF,
            instance: clint_u,
            name: "Clint",
        };

        let bus_u = Bus::new(device_clint);
        Mmu {
            bus: bus_u,
            access_type: AccessType::Load(0),
            mstatus: 0.into(),
            stap: 0.into(),
            i: 0,
            level: 0,
            pagesize: 0,
            a: 0,
            va: 0.into(),
            pa: 0.into(),
            pte: 0.into(),
            cur_priv: privilege,
        }
    }

    fn get_ptesize(&self) -> u64 {
        8 // todo! sv39 mode only
    }
    // 1. Let a be satp.ppn × PAGESIZE, and let i = LEVELS − 1. (For Sv32, PAGESIZE=2^12 and
    // LEVELS=2.) The satp register must be active, i.e., the effective privilege mode must be
    // S-mode or U-mode.

    // todo! check privilege mode
    fn va_translation_step1(&mut self) -> Result<u8, TrapType> {
        assert_eq!(self.stap.mode(), StapMode::Sv39); // todo!  sv39 mode only
        assert_ne!(self.cur_priv.get(), PrivilegeLevels::Machine); // check privilege mode
        self.pagesize = 4096; // 2 ^ 12
        self.level = 3;
        self.i = self.level - 1;
        self.a = self.stap.ppn() * self.pagesize;
        Ok(2)
    }
    // 2. Let pte be the value of the PTE at address a+va.vpn[i]×PTESIZE. (For Sv32, PTESIZE=4.)
    // If accessing pte violates a PMA or PMP check, raise an access-fault exception corresponding
    // to the original access type.

    // todo! PMA or PMP check
    fn va_translation_step2(&mut self) -> Result<(), TrapType> {
        let pte_size = self.get_ptesize();

        let pte_addr = self.a + self.va.get_ppn_by_idx(self.i as u8) * pte_size;
        // println!("va:{:?}", self.stap);
        // println!("va:{:?}", self.va);
        // assert_eq!(self.stap.ppn() * 4096, self.a);
        // todo! PMA or PMP check
        let pte_data = self.bus.read(pte_addr, pte_size as usize).unwrap();
        self.pte = Sv39PTE::from(pte_data);
        Ok(())
    }
    // 3. If pte.v = 0, or if pte.r = 0 and pte.w = 1, or if any bits or encodings that are reserved for
    // future standard use are set within pte, stop and raise a page-fault exception corresponding
    // to the original access type.
    fn va_translation_step3(&self) -> Result<(), TrapType> {
        if !self.pte.v() || (!self.pte.r() && self.pte.w()) {
            Err(self.access_type.throw_exception())
        } else {
            Ok(())
        }
    }

    //4. Otherwise, the PTE is valid. If pte.r = 1 or pte.x = 1, go to step 5. Otherwise, this PTE is a
    // pointer to the next level of the page table. Let i = i − 1. If i < 0, stop and raise a page-fault
    // exception corresponding to the original access type. Otherwise, let a = pte.ppn × PAGESIZE
    // and go to step 2.
    fn va_translation_step4(&mut self) -> Result<u8, TrapType> {
        if self.pte.r() && self.pte.x() {
            return Ok(5); // go to step 5
        }
        self.i -= 1;
        if self.i < 0 {
            return Err(self.access_type.throw_exception());
        }

        self.a = self.pte.ppn_all() * self.pagesize;

        Ok(2)
    }
    // 5. A leaf PTE has been found. Determine if the requested memory access is allowed by the
    // pte.r, pte.w, pte.x, and pte.u bits, given the current privilege mode and the value of the
    // SUM and MXR fields of the mstatus register. If not, stop and raise a page-fault exception
    // corresponding to the original access type.

    // todo!
    fn va_translation_step5(&mut self) -> Result<u8, TrapType> {
        // When SUM=0, S-mode memory accesses to pages that are
        // accessible by U-mode (U=1 in Figure 4.18) will fault.
        // When SUM=1, these accesses are permitted.
        let sum_bit_check = || -> bool {
            if self.cur_priv.get() != PrivilegeLevels::Supervisor {
                return true;
            }
            // in S-mode

            self.mstatus.sum() || !self.pte.u()
        };

        match self.access_type {
            AccessType::Fetch(_) if !self.pte.x() => {
                return Err(self.access_type.throw_exception());
            }
            // When MXR=0, only loads from pages marked readable (R=1 in Figure 4.18) will succeed.
            // When MXR=1, loads from pages marked either readable or executable (R=1 or X=1) will succeed.
            // MXR has no effect when page-based virtual memory is not in effect.
            AccessType::Load(_)
                if !(self.pte.r() || self.pte.x() & self.mstatus.mxr()) || !sum_bit_check() =>
            {
                return Err(self.access_type.throw_exception());
            }
            AccessType::Store(_) | AccessType::Amo(_) if !self.pte.w() || !sum_bit_check() => {
                return Err(self.access_type.throw_exception());
            }
            _ => {}
        }

        Ok(6)
    }
    // 6. If i > 0 and pte.ppn[i − 1 : 0] ̸= 0, this is a misaligned superpage; stop and raise a page-fault
    // exception corresponding to the original access type.
    fn va_translation_step6(&mut self) -> Result<u8, TrapType> {
        let mut is_misalign_superpage = || -> bool {
            for i in 0..self.i {
                if self.pte.get_ppn_by_idx(i as u8) != 0 {
                    return true;
                }
            }
            false
        };

        if (self.i > 0) && is_misalign_superpage() {
            return Err(self.access_type.throw_exception());
        }

        Ok(7)
    }
    // 7. If pte.a = 0, or if the original memory access is a store and pte.d = 0, either raise a page-fault
    // exception corresponding to the original access type, or:
    //     + If a store to pte would violate a PMA or PMP check, raise an access-fault exception
    //       corresponding to the original access type.
    //     + Perform the following steps atomically:
    //       – Compare pte to the value of the PTE at address a + va.vpn[i] × PTESIZE.
    //       – If the values match, set pte.a to 1 and, if the original memory access is a store, also
    //       set pte.d to 1.
    //       – If the comparison fails, return to step 2

    fn va_translation_step7(&mut self) -> Result<u8, TrapType> {
        // choese to raise a exception
        if !self.pte.a()
            || ((!self.pte.d())
                    // only check eume type without data
                && (self.access_type == AccessType::Store(0)
                    || self.access_type == AccessType::Amo(0)))
        {
            Err(self.access_type.throw_exception())
        } else {
            Ok(8)
        }
    }

    // 8. The translation is successful. The translated physical address is given as follows:
    //      + pa.pgoff = va.pgoff.
    //      + If i > 0, then this is a superpage translation and pa.ppn[i − 1 : 0] = va.vpn[i − 1 : 0].
    //      + pa.ppn[LEVELS − 1 : i] = pte.ppn[LEVELS − 1 : i].

    fn va_translation_step8(&mut self) -> Result<u8, TrapType> {
        let pgoff = self.va.offset();

        self.pa.set_offset(pgoff);

        if self.i > 0 {
            for idx in 0..self.i as u8 {
                let va_ppn = self.va.get_ppn_by_idx(idx);
                self.pa.set_ppn_by_idx(va_ppn, idx);
            }
        }

        for idx in self.i..self.level {
            let idx = idx as u8;
            let pte_ppn = self.pte.get_ppn_by_idx(idx);
            self.pa.set_ppn_by_idx(pte_ppn, idx);
        }

        Ok(1)
    }

    pub fn page_table_walk(&mut self) -> Result<u64, TrapType> {
        let ret = self.va_translation_step1();

        assert!(ret.is_ok());

        'step2: loop {
            let ret = self.va_translation_step2();
            assert!(ret.is_ok());

            let ret = self.va_translation_step3();

            ret?;

            let ret = self.va_translation_step4();

            ret?;

            if let Ok(step) = ret {
                if step == 5 {
                    break 'step2;
                }
            }
        }

        let ret = self.va_translation_step5();
        ret?;

        let ret = self.va_translation_step6();
        ret?;

        let ret = self.va_translation_step7();
        ret?;

        let ret = self.va_translation_step8();
        ret?;

        Ok(self.pa.into())
    }

    fn no_mmu(&self) -> bool {
        // if ((cur_priv == M_MODE && (!mstatus->mprv || mstatus->mpp == M_MODE)) ||
        // satp_reg->mode == 0) {
        let x1 = !self.mstatus.mprv() || (self.mstatus.mpp() == 3);
        let x2 = self.cur_priv.get().eq(&PrivilegeLevels::Machine);
        let y1 = self.stap.mode().eq(&StapMode::Bare);

        (x1 && x2) || y1
    }

    pub fn do_read(&mut self, addr: u64, len: u64) -> Result<u64, TrapType> {
        if !check_aligned(addr, len) {
            return Err(TrapType::LoadAddressMisaligned(addr));
        }
        // no mmu
        if self.no_mmu() {
            self.pa = Sv39Pa::from(addr);
            return Ok(self.bus.read(addr, len as usize).unwrap());
        }
        // println!("has mmu,addr:{:x}",addr);
        // has mmu
        self.va = Sv39Va::from(addr);
        self.page_table_walk()?; // err return
        Ok(self.bus.read(self.pa.into(), len as usize).unwrap())
    }

    pub fn do_write(&mut self, addr: u64, data: u64, len: u64) -> Result<u64, TrapType> {
        if !check_aligned(addr, len) {
            return Err(TrapType::StoreAddressMisaligned(addr));
        }
        // no mmu
        if self.no_mmu() {
            return Ok(self.bus.write(addr, data, len as usize).unwrap());
        }
        // has mmu
        self.va = Sv39Va::from(addr);
        self.page_table_walk()?; // err return
        Ok(self.bus.write(self.pa.into(), data, len as usize).unwrap())
    }

    pub fn update_access_type(&mut self, access_type: AccessType) {
        self.access_type = access_type;
    }

    pub fn update_mstatus(&mut self, mstatus_val: u64) {
        self.mstatus = Mstatus::from(mstatus_val);
    }

    pub fn update_stap(&mut self, stap_val: u64) {
        self.stap = Stap::from(stap_val);
    }
}

