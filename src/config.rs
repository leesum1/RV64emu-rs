use crate::rv64core::csr_regs_define::StapMode;

pub struct Config {
    icache_size: Option<usize>,
    dcache_size: Option<usize>,
    decode_cache_size: Option<usize>,
    tlb_size: Option<usize>,
    mmu_type: StapMode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            icache_size: Default::default(),
            dcache_size: Default::default(),
            decode_cache_size: Default::default(),
            tlb_size: Default::default(),
            mmu_type: StapMode::Bare,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn set_icache_size(&mut self, size: usize) {
        self.icache_size = Some(size);
    }
    pub fn set_dcache_size(&mut self, size: usize) {
        self.dcache_size = Some(size);
    }
    pub fn set_decode_cache_size(&mut self, size: usize) {
        self.decode_cache_size = Some(size);
    }
    pub fn set_tlb_size(&mut self, size: usize) {
        self.tlb_size = Some(size);
    }
    // sv39 sv48
    pub fn set_mmu_type(&mut self, mmu_type: &str) {
        let mmu_type = mmu_type.to_lowercase();
        match mmu_type.as_str() {
            "sv39" => self.mmu_type = StapMode::Sv39,
            "sv48" => self.mmu_type = StapMode::Sv48,
            "sv57" => self.mmu_type = StapMode::Sv57,
            err => panic!("mmu type err:{err}"),
        }
    }

    pub fn get_mmu_type(&self) -> StapMode {
        self.mmu_type
    }

    pub fn icache_size(&self) -> Option<usize> {
        self.icache_size
    }
    pub fn dcache_size(&self) -> Option<usize> {
        self.dcache_size
    }
    pub fn decode_cache_size(&self) -> Option<usize> {
        self.decode_cache_size
    }
    pub fn tlb_size(&self) -> Option<usize> {
        self.tlb_size
    }
}
