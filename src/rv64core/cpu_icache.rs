use hashbrown::HashMap;

pub struct CpuIcache {
    inst_hash: HashMap<u64, u32>,
}

impl CpuIcache {
    pub fn new() -> Self {
        CpuIcache {
            inst_hash: HashMap::new(),
        }
    }

    pub fn get_inst(&self, pc: u64) -> Option<u32> {
        self.inst_hash.get(&pc).copied()
    }

    pub fn insert_inst(&mut self, pc: u64, inst: u32) {
        self.inst_hash.insert(pc, inst);
    }

    pub fn clear_inst(&mut self) {
        self.inst_hash.clear();
    }
}

impl Default for CpuIcache {
    fn default() -> Self {
        Self::new()
    }
}
