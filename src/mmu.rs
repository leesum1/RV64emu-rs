use crate::{
    bus::Bus,
    sv39::{Sv39PTE, Sv39Pa, Sv39Va},
    traptype::TrapType,
};

struct Stap {
    val: u64,
}

impl Stap {
    pub fn new(val: u64) -> Self {
        Stap { val }
    }

    pub fn ppn(&self) -> u64 {
        self.val & 0xfff_ffff_ffff
    }
    pub fn asid(&self) -> u64 {
        (self.val >> 44) & 0xffff
    }
    pub fn mode(&self) -> u64 {
        (self.val >> 60) & 0xf
    }
}
#[derive(PartialEq)]
enum AccessType {
    Load,
    Store,
    Execute,
}

impl AccessType {
    fn throw_exception(&self) -> TrapType {
        match self {
            AccessType::Load => TrapType::LoadPageFault,
            AccessType::Store => TrapType::StorePageFault,
            AccessType::Execute => TrapType::InstructionPageFault,
        }
    }
}

struct Mmu {
    pub bus: Bus,
    pub access_type: AccessType,
    pub i: i8,
    pub level: i8,
    pub pagesize: u64,
    pub a: u64,
    pub va: Sv39Va,
    pub pa: Sv39Pa,
    pub pte: Sv39PTE,
}

impl Mmu {
    pub fn get_ptesize(&self) -> u64 {
        8 // todo! sv39 mode only
    }
    // 1. Let a be satp.ppn × PAGESIZE, and let i = LEVELS − 1. (For Sv32, PAGESIZE=2^12 and
    // LEVELS=2.) The satp register must be active, i.e., the effective privilege mode must be
    // S-mode or U-mode.

    // todo! check privilege mode
    pub fn va_translation_step1(&mut self, stap: u64) -> Result<u8, TrapType> {
        let stap_val = Stap::new(stap);
        assert_eq!(stap_val.mode(), 8); // todo!  sv39 mode only
        self.pagesize = 2 ^ 12; // 4096
        self.level = 3;
        self.i = self.level - 1;
        self.a = stap_val.ppn() * self.pagesize;
        Ok(2)
    }
    // 2. Let pte be the value of the PTE at address a+va.vpn[i]×PTESIZE. (For Sv32, PTESIZE=4.)
    // If accessing pte violates a PMA or PMP check, raise an access-fault exception corresponding
    // to the original access type.

    // todo! PMA or PMP check
    pub fn va_translation_step2(&mut self) -> Result<(), TrapType> {
        let pte_size = self.get_ptesize();

        let pte_addr = self.a + self.va.get_ppn_by_idx(self.i as u8) * pte_size;

        // todo! PMA or PMP check
        let pte_data = self.bus.read(pte_addr, pte_size as usize).unwrap();
        self.pte = Sv39PTE::from(pte_data);
        Ok(())
    }
    // 3. If pte.v = 0, or if pte.r = 0 and pte.w = 1, or if any bits or encodings that are reserved for
    // future standard use are set within pte, stop and raise a page-fault exception corresponding
    // to the original access type.
    pub fn va_translation_step3(&self) -> Result<(), TrapType> {
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
    pub fn va_translation_step4(&mut self) -> Result<u8, TrapType> {
        if self.pte.r() && self.pte.x() {
            return Ok(5); // go to step 5
        }
        self.i -= 1;
        if self.i <= 0 {
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
    pub fn va_translation_step5(&mut self) -> Result<u8, TrapType> {
        match self.access_type {
            AccessType::Load => {
                if !self.pte.r() {
                    return Err(self.access_type.throw_exception());
                }
            }
            AccessType::Store => {
                if !self.pte.w() {
                    return Err(self.access_type.throw_exception());
                }
            }
            AccessType::Execute => {
                if !self.pte.x() {
                    return Err(self.access_type.throw_exception());
                }
            }
        }

        Ok(6)
    }
    // 6. If i > 0 and pte.ppn[i − 1 : 0] ̸= 0, this is a misaligned superpage; stop and raise a page-fault
    // exception corresponding to the original access type.
    pub fn va_translation_step6(&mut self) -> Result<u8, TrapType> {
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

    pub fn va_translation_step7(&mut self) -> Result<u8, TrapType> {
        // choese to raise a exception
        if !self.pte.a() || ((!self.pte.d()) && (self.access_type == AccessType::Store)) {
            Err(self.access_type.throw_exception())
        } else {
            Ok(8)
        }
    }

    // 8. The translation is successful. The translated physical address is given as follows:
    //      + pa.pgoff = va.pgoff.
    //      + If i > 0, then this is a superpage translation and pa.ppn[i − 1 : 0] = va.vpn[i − 1 : 0].
    //      + pa.ppn[LEVELS − 1 : i] = pte.ppn[LEVELS − 1 : i].

    pub fn va_translation_step8(&mut self) -> Result<u8, TrapType> {
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

    pub fn page_table_walk(&mut self, _va: u64, stap: u64) -> Result<u64, TrapType> {
        let ret = self.va_translation_step1(stap);

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
}
