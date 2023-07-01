#[derive(Default)]
pub struct Config {
    icache_size: Option<usize>,
    dcache_size: Option<usize>,
    decode_cache_size: Option<usize>,
    tlb_size: Option<usize>,
    // mmu_type: Option<String>,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
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
