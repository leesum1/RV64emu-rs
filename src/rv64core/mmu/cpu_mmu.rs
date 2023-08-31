use core::cell::Cell;

use alloc::{rc::Rc, vec::Vec};
use hashlink::LruCache;
use log::{info, trace};

use crate::{
    config::Config,
    rv64core::csr_regs_define::{SatpIn, StapMode, XstatusIn},
    rv64core::{
        cache::cache_system::CacheSystem,
        inst::inst_base::{AccessType, PrivilegeLevels},
        traptype::TrapType,
    },
    tools::{check_aligned, RcCell, RcRefCell},
};

use super::{
    sv48::{Sv48PA, Sv48PTE, Sv48VA},
    vm_info::{PAenume, PAops, PTEenume, PTEops, PageSize, TLBEntry, TLBKey, VAenume, VAops},
};

const PAGESIZE: u64 = 4096; // 2 ^ 12

pub struct Mmu {
    pub caches: RcRefCell<CacheSystem>,
    pub access_type: AccessType,
    mstatus: RcCell<XstatusIn>,
    satp: RcCell<SatpIn>,
    cur_priv: Rc<Cell<PrivilegeLevels>>,
    mmu_effective_priv: PrivilegeLevels,
    satp_mode: StapMode,
    config: Rc<Config>,
    tlb: LruCache<TLBKey, TLBEntry>,
    tlb_hit: u64,
    tlb_miss: u64,
    /* tmp val */
    i: i8,
    level: i8,
    a: u64,
    va: VAenume,
    pa: PAenume,
    pte: PTEenume,
}

impl Mmu {
    pub fn new(
        caches: RcRefCell<CacheSystem>,

        privilege: Rc<Cell<PrivilegeLevels>>,
        mstatus: RcCell<XstatusIn>,
        satp: RcCell<SatpIn>,
        config: Rc<Config>,
    ) -> Self {
        Mmu {
            caches,
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
            tlb: LruCache::new(config.tlb_size().unwrap_or(0)),
            config,
            tlb_hit: 0,
            tlb_miss: 0,
        }
    }

    // 1. Let a be satp.ppn × PAGESIZE, and let i = LEVELS − 1. (For Sv32, PAGESIZE=2^12 and
    // LEVELS=2.) The satp register must be active, i.e., the effective privilege mode must be
    // S-mode or U-mode.

    // todo! check privilege mode
    fn va_translation_step1(&mut self) -> Result<u8, TrapType> {
        assert_ne!(self.mmu_effective_priv, PrivilegeLevels::Machine); // check privilege mode
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
            .caches
            .borrow_mut()
            .dcache
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
        // let sum_bit_check = || -> bool {
        //     if self.mmu_effective_priv != PrivilegeLevels::Supervisor {
        //         return true;
        //     }
        //     // in S-mode
        //     self.mstatus.get().sum() || !self.pte.u()
        // };

        match self.access_type {
            AccessType::Fetch(_) if !self.pte.x() => {
                return Err(self.access_type.throw_page_exception());
            }
            // When MXR=0, only loads from pages marked readable (R=1 in Figure 4.18) will succeed.
            // When MXR=1, loads from pages marked either readable or executable (R=1 or X=1) will succeed.
            // MXR has no effect when page-based virtual memory is not in effect.
            AccessType::Load(_)
                if !(self.pte.r() || self.pte.x() & self.mstatus.get().mxr())
                    || !self.check_sum_bit() =>
            {
                return Err(self.access_type.throw_page_exception());
            }
            AccessType::Store(_) | AccessType::Amo(_) if !self.pte.w() || !self.check_sum_bit() => {
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
            // if self.i == 0 this loop do not excute
            // and will return false;
            for i in 0..self.i {
                if self.pte.get_ppn_by_idx(i as u8) != 0 {
                    return true;
                }
            }
            false
        };

        if is_misalign_superpage() {
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
        if !self.pte.a() || ((!self.pte.d()) && self.access_type.is_store()) {
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
        let asid = self.satp.get().asid() as u16;
        let page_size = PageSize::from_i(self.i as usize);

        let tlb_key = TLBKey {
            va: self.va.raw() & page_size.get_mask(),
            asid,
        };
        let entry = TLBEntry::new(self.pte, page_size, asid);

        let pa = entry.get_pa(&self.va);

        self.pa = self.get_paops(pa);

        // debug!("page_size:{:?}", PageSize::from_i(self.i as usize));

        if !self.no_tlb() {
            self.tlb.insert(tlb_key, entry);
            // if (self.va.raw() & !(0xfff_u64)) == 0x1b6000 {
            //     self.debug_tlb();
            //     println!("ppn:{:x},asid:{:x}", self.va.raw(), asid);
            // }
            let res = self.fast_path(self.va.raw()).unwrap();
            assert_eq!(res.asid, entry.asid);
            assert_eq!(res.page_size, entry.page_size);
            assert_eq!(res.pte.raw(), entry.pte.raw());
        }

        Ok(1)
    }

    fn check_sum_bit(&self) -> bool {
        if self.mmu_effective_priv != PrivilegeLevels::Supervisor {
            return true;
        }
        // in S-mode
        self.mstatus.get().sum() || !self.pte.u()
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

        // debug!("translate: {:x} -> {:x}", self.va.raw(), self.pa.raw());

        Ok(self.pa.raw())
    }

    fn no_tlb(&self) -> bool {
        self.tlb.capacity() == 0
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

    pub fn translate(&mut self, addr: u64, len: usize) -> Result<u64, TrapType> {
        if !check_aligned(addr, len) {
            return Err(self.access_type.throw_addr_misaligned_exception());
        }
        if self.no_mmu() {
            return Ok(addr);
        }

        if !self.no_tlb() {
            // todo! refactor!!!!!!!!!!!!!
            if let Some(tlb_entry) = self.fast_path(addr) {
                self.pte = tlb_entry.pte;

                // 1. If accessing pte violates a PMA or PMP check, raise an access-fault exception corresponding to the original access type.

                // 2. If pte.v = 0, or if pte.r = 0 and pte.w = 1, or if any bits or encodings that are reserved for
                // future standard use are set within pte, stop and raise a page-fault exception corresponding
                // to the original access type.
                if !self.pte.v() || (!self.pte.r() && self.pte.w()) {
                    return Err(self.access_type.throw_page_exception());
                }
                // 3. A leaf PTE has been found. Determine if the requested memory access is allowed by the
                // pte.r, pte.w, pte.x, and pte.u bits, given the current privilege mode and the value of the
                // SUM and MXR fields of the mstatus register. If not, stop and raise a page-fault exception
                // corresponding to the original access type.

                let sum_bit = self.check_sum_bit();
                match self.access_type {
                    AccessType::Fetch(_) if !self.pte.x() => {
                        return Err(self.access_type.throw_page_exception());
                    }
                    // When MXR=0, only loads from pages marked readable (R=1 in Figure 4.18) will succeed.
                    // When MXR=1, loads from pages marked either readable or executable (R=1 or X=1) will succeed.
                    // MXR has no effect when page-based virtual memory is not in effect.
                    AccessType::Load(_)
                        if !(self.pte.r() || self.pte.x() & self.mstatus.get().mxr())
                            || !sum_bit =>
                    {
                        return Err(self.access_type.throw_page_exception());
                    }
                    AccessType::Store(_) | AccessType::Amo(_) if !self.pte.w() || !sum_bit => {
                        return Err(self.access_type.throw_page_exception());
                    }
                    _ => {}
                }
                // 4. If pte.a = 0, or if the original memory access is a store and pte.d = 0, either raise a page-fault
                // exception corresponding to the original access type
                if !self.pte.a() || ((!self.pte.d()) && self.access_type.is_store()) {
                    return Err(self.access_type.throw_page_exception());
                }
                let pa = tlb_entry.get_pa(&self.get_vaops(addr));
                // debug!("translate: {:x} -> {:x}", addr, pa);
                return Ok(pa);
            }
        }

        // do page table walk
        self.va = self.get_vaops(addr);
        self.pa = self.get_paops(0);
        self.page_table_walk()
    }

    pub fn update_access_type(&mut self, access_type: &AccessType) {
        self.access_type = access_type.clone();
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

    pub fn fast_path(&mut self, va: u64) -> Option<TLBEntry> {
        // println!("fast_path: {:x}", va);
        // Compute the masks only once
        let asid = self.satp.get().asid() as u16;
        let va_p2m = va & PageSize::P2M.get_mask();
        // Check for P2M page size
        if let Some(entry) = self.tlb.get(&TLBKey { va: va_p2m, asid }).copied() {
            if entry.page_size == PageSize::P2M {
                self.tlb_hit += 1;
                return Some(entry);
            }
        }
        let va_p4k = va & PageSize::P4K.get_mask();

        // Check for P4K page size
        if let Some(entry) = self.tlb.get(&TLBKey { va: va_p4k, asid }).copied() {
            if entry.page_size == PageSize::P4K {
                self.tlb_hit += 1;
                return Some(entry);
            }
        }

        let va_p1g = va & PageSize::P1G.get_mask();
        // Check for P1G page size
        if let Some(entry) = self.tlb.get(&TLBKey { va: va_p1g, asid }).copied() {
            if entry.page_size == PageSize::P1G {
                self.tlb_hit += 1;
                return Some(entry);
            }
        }

        self.tlb_miss += 1;
        None
    }

    pub fn show_perf(&self) {
        info!("tlb hit: {}, tlb miss: {}", self.tlb_hit, self.tlb_miss);
        info!(
            "tlb hit rate: {}",
            self.tlb_hit as f64 / (self.tlb_hit + self.tlb_miss) as f64
        )
    }
    pub fn clear_tlb(&mut self) {
        self.tlb.clear();
    }

    // fn debug_tlb(&self) {
    //     self.tlb
    //         .iter()
    //         .for_each(|item| println!("key:{:x},value:{:?}", item.0, item.1));
    //     println!("------------------");
    // }

    fn fence_vma_trace_log(&self, tlb_entryl: Option<TLBEntry>) {
        if let Some(tlb_entry) = tlb_entryl {
            trace!("fence_vma : {:?}", tlb_entry);
        }else {
            trace!("fence_vma : None");
        }
    }

    pub fn fence_vma(&mut self, va: u64, asid: u16) {
        // self.debug_tlb();


        match (va, asid) {
            (0, 0) => {
                self.clear_tlb();
            }
            (0, asid) => {
                let key_list = self
                    .tlb
                    .iter()
                    .filter(|tlb_item| (tlb_item.1.asid == asid) && !tlb_item.1.pte.g())
                    .map(|(key, _val)| *key)
                    .collect::<Vec<_>>();

                key_list.iter().for_each(|tlb_key| {
                    let res = self.tlb.remove(tlb_key);
                    self.fence_vma_trace_log(res);
                });
            }
            (va, 0) => {
                let tlb_key = self
                    .tlb
                    .iter()
                    .find(|tlb_item| (tlb_item.0.va == tlb_item.1.page_size.get_mask() & va))
                    .map(|(key, _val)| *key);

                if let Some(tlb_key) = tlb_key {
                    let res = self.tlb.remove(&tlb_key);
                    self.fence_vma_trace_log(res);
                }
            }
            (va, asid) => {
                let tlb_key = self
                    .tlb
                    .iter()
                    .find(|tlb_item| {
                        (tlb_item.0.asid == asid)
                            && (tlb_item.0.va == tlb_item.1.page_size.get_mask() & va)
                            && (!tlb_item.1.pte.g())
                    })
                    .map(|(key, _val)| *key);

                if let Some(tlb_key) = tlb_key {
                    let res = self.tlb.remove(&tlb_key);
                    self.fence_vma_trace_log(res);
                }
            }
        }
    }
}
