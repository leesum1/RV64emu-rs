use std::{rc::Rc, sync::Mutex};

use core::cell::Cell;

use crate::{
    rv64core::bus::Bus,
    rv64core::csr_regs_define::{CsrShare, SatpIn, StapMode, XstatusIn},
    rv64core::inst::inst_base::{check_aligned, AccessType, PrivilegeLevels},
    rv64core::traptype::TrapType,
};

use super::{
    sv48::{Sv48PA, Sv48PTE, Sv48VA},
    vm_info::{PAenume, PAops, PTEenume, PTEops, VAenume, VAops},
};

const PAGESIZE: u64 = 4096; // 2 ^ 12

pub struct Mmu {
    pub bus: Rc<Mutex<Bus>>,
    pub access_type: AccessType,
    mstatus: CsrShare<XstatusIn>,
    satp: CsrShare<SatpIn>,
    cur_priv: Rc<Cell<PrivilegeLevels>>,
    mmu_effective_priv: PrivilegeLevels,
    satp_mode: StapMode,
    i: i8,
    level: i8,
    a: u64,
    va: VAenume,
    pa: PAenume,
    pte: PTEenume,
}

impl Mmu {
    pub fn new(
        bus: Rc<Mutex<Bus>>,
        privilege: Rc<Cell<PrivilegeLevels>>,
        mstatus: CsrShare<XstatusIn>,
        satp: CsrShare<SatpIn>,
    ) -> Self {
        Mmu {
            bus,
            access_type: AccessType::Load(0),
            mstatus,
            satp,
            cur_priv: privilege,
            mmu_effective_priv: PrivilegeLevels::Machine,
            satp_mode: StapMode::Bare,
            i: 0,
            level: 0,
            a: 0,
            va: Sv48VA::new().into(),
            pa: Sv48PA::new().into(),
            pte: Sv48PTE::new().into(),
        }
    }

    // 1. Let a be satp.ppn × PAGESIZE, and let i = LEVELS − 1. (For Sv32, PAGESIZE=2^12 and
    // LEVELS=2.) The satp register must be active, i.e., the effective privilege mode must be
    // S-mode or U-mode.

    // todo! check privilege mode
    fn va_translation_step1(&mut self) -> Result<u8, TrapType> {
        assert_ne!(self.mmu_effective_priv, PrivilegeLevels::Machine); // check privilege mode
                                                                       // self.pagesize = 4096; // 2 ^ 12
        self.level = self.satp_mode.get_levels() as i8;
        self.i = self.level - 1;
        self.a = self.satp.get().ppn() * PAGESIZE;
        Ok(2)
    }
    // 2. Let pte be the value of the PTE at address a+va.vpn[i]×PTESIZE. (For Sv32, PTESIZE=4.)
    // If accessing pte violates a PMA or PMP check, raise an access-fault exception corresponding
    // to the original access type.

    // todo! PMA or PMP check
    fn va_translation_step2(&mut self) -> Result<(), TrapType> {
        let pte_size = self.satp_mode.get_ptesize() as u64;

        let pte_addr = self.a + self.va.get_ppn_by_idx(self.i as u8) * pte_size;
        // warn!("va:{:?}", self.stap);
        // warn!("va:{:?}", self.va);
        // assert_eq!(self.stap.ppn() * 4096, self.a);
        // todo! PMA or PMP check
        let pte_data = self
            .bus
            .lock()
            .unwrap()
            .read(pte_addr, pte_size as usize)
            .unwrap();
        // self.pte = Sv39PTE::from(pte_data).into();
        self.pte = self.get_pteops(pte_data);

        Ok(())
    }
    // 3. If pte.v = 0, or if pte.r = 0 and pte.w = 1, or if any bits or encodings that are reserved for
    // future standard use are set within pte, stop and raise a page-fault exception corresponding
    // to the original access type.
    fn va_translation_step3(&self) -> Result<(), TrapType> {
        if !self.pte.v() || (!self.pte.r() && self.pte.w()) {
            Err(self.access_type.throw_page_exception())
        } else {
            Ok(())
        }
    }

    //4. Otherwise, the PTE is valid. If pte.r = 1 or pte.x = 1, go to step 5. Otherwise, this PTE is a
    // pointer to the next level of the page table. Let i = i − 1. If i < 0, stop and raise a page-fault
    // exception corresponding to the original access type. Otherwise, let a = pte.ppn × PAGESIZE
    // and go to step 2.
    fn va_translation_step4(&mut self) -> Result<u8, TrapType> {
        if self.pte.r() || self.pte.x() {
            return Ok(5); // go to step 5
        }
        self.i -= 1;
        if self.i < 0 {
            return Err(self.access_type.throw_page_exception());
        }

        self.a = self.pte.ppn_all() * PAGESIZE;

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
            if self.mmu_effective_priv != PrivilegeLevels::Supervisor {
                return true;
            }
            // in S-mode

            self.mstatus.get().sum() || !self.pte.u()
        };

        match self.access_type {
            AccessType::Fetch(_) if !self.pte.x() => {
                return Err(self.access_type.throw_page_exception());
            }
            // When MXR=0, only loads from pages marked readable (R=1 in Figure 4.18) will succeed.
            // When MXR=1, loads from pages marked either readable or executable (R=1 or X=1) will succeed.
            // MXR has no effect when page-based virtual memory is not in effect.
            AccessType::Load(_)
                if !(self.pte.r() || self.pte.x() & self.mstatus.get().mxr())
                    || !sum_bit_check() =>
            {
                return Err(self.access_type.throw_page_exception());
            }
            AccessType::Store(_) | AccessType::Amo(_) if !self.pte.w() || !sum_bit_check() => {
                return Err(self.access_type.throw_page_exception());
            }
            _ => {}
        }

        Ok(6)
    }
    // 6. If i > 0 and pte.ppn[i − 1 : 0] ̸= 0, this is a misaligned superpage; stop and raise a page-fault
    // exception corresponding to the original access type.
    fn va_translation_step6(&mut self) -> Result<u8, TrapType> {
        let is_misalign_superpage = || -> bool {
            for i in 0..self.i {
                if self.pte.get_ppn_by_idx(i as u8) != 0 {
                    return true;
                }
            }
            false
        };

        if (self.i > 0) && is_misalign_superpage() {
            return Err(self.access_type.throw_page_exception());
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
            Err(self.access_type.throw_page_exception())
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

        loop {
            self.va_translation_step2()?;
            self.va_translation_step3()?;
            if let Ok(step) = self.va_translation_step4() {
                if step == 5 {
                    break;
                }
            }
        }

        self.va_translation_step5()?;
        self.va_translation_step6()?;
        self.va_translation_step7()?;
        self.va_translation_step8()?;

        Ok(self.pa.raw())
    }

    fn no_mmu(&mut self) -> bool {
        let satp_bare_mode = self.satp_mode.eq(&StapMode::Bare);
        let mstatus: XstatusIn = self.mstatus.get();
        self.mmu_effective_priv = self.cur_priv.get();

        // When MPRV=1, load and store memory addresses are translated and protected, and endianness is applied, as though
        //the current privilege mode were set to MPP. Instruction address-translation and protection are
        // unaffected by the setting of MPRV. MPRV is read-only 0 if U-mode is not supported.
        if self.access_type != AccessType::Fetch(0) && mstatus.mprv() {
            self.mmu_effective_priv = mstatus.get_mpp_priv();
        }

        // If the effective privilege level is machine mode or if the satp mode is bare mode, then the MMU is effectively disabled
        // (i.e. no_mmu() returns true)
        let machine_mdoe = self.mmu_effective_priv.eq(&PrivilegeLevels::Machine);
        machine_mdoe || satp_bare_mode
    }

    pub fn do_read(&mut self, addr: u64, len: u64) -> Result<u64, TrapType> {
        //check whether the address is aligned
        if !check_aligned(addr, len) {
            return Err(TrapType::LoadAddressMisaligned(addr));
        }
        // no mmu
        //if the machine is without mmu,then we need not do page table walk,just return the physical address
        if self.no_mmu() {
            return self
                .bus
                .lock()
                .unwrap()
                .read(addr, len as usize)
                .map_or(Err(self.access_type.throw_access_exception()), Ok);
        }
        // has mmu,
        // update the virtual address and physical address
        self.va = self.get_vaops(addr);
        self.pa = self.get_paops(0);

        //do page table walk
        self.page_table_walk()?; // err return

        //read the data from the physical address
        self.bus
            .lock()
            .unwrap()
            .read(self.pa.raw(), len as usize)
            .map_or(Err(self.access_type.throw_access_exception()), Ok)
    }

    pub fn do_write(&mut self, addr: u64, data: u64, len: u64) -> Result<u64, TrapType> {
        if !check_aligned(addr, len) {
            return Err(TrapType::StoreAddressMisaligned(addr));
        }

        // no mmu
        if self.no_mmu() {
            return self
                .bus
                .lock()
                .unwrap()
                .write(addr, data, len as usize)
                .map_or(Err(self.access_type.throw_access_exception()), Ok);
        }

        // has mmu
        // update the virtual address and physical address

        self.satp_mode = self.satp.get().mode();
        self.va = self.get_vaops(addr);
        self.pa = self.get_paops(0);

        self.page_table_walk()?; // err return
        self.bus
            .lock()
            .unwrap()
            .write(self.pa.raw(), data, len as usize)
            .map_or(Err(self.access_type.throw_access_exception()), Ok)
    }

    pub fn update_access_type(&mut self, access_type: AccessType) {
        self.access_type = access_type;
        // update satp mode
        self.satp_mode = self.satp.get().mode();
    }

    fn get_pteops(&self, pte_data: u64) -> PTEenume {
        match self.satp_mode {
            StapMode::Sv39 => PTEenume::Sv39PTE(pte_data.into()),
            StapMode::Sv48 => PTEenume::Sv48PTE(pte_data.into()),
            StapMode::Sv57 => PTEenume::Sv57PTE(pte_data.into()),
            _ => PTEenume::Sv39PTE(pte_data.into()),
        }
    }

    fn get_paops(&self, pa_data: u64) -> PAenume {
        match self.satp_mode {
            StapMode::Sv39 => PAenume::Sv39PA(pa_data.into()),
            StapMode::Sv48 => PAenume::Sv48PA(pa_data.into()),
            StapMode::Sv57 => PAenume::Sv57PA(pa_data.into()),
            _ => PAenume::Sv39PA(pa_data.into()),
        }
    }

    fn get_vaops(&self, va_data: u64) -> VAenume {
        match self.satp_mode {
            StapMode::Sv39 => VAenume::Sv39VA(va_data.into()),
            StapMode::Sv48 => VAenume::Sv48VA(va_data.into()),
            StapMode::Sv57 => VAenume::Sv57VA(va_data.into()),
            _ => VAenume::Sv39VA(va_data.into()),
        }
    }
}
