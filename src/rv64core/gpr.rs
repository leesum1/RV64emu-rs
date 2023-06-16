use core::fmt;

#[allow(non_camel_case_types)]
pub enum GprName {
    zero,
    ra,
    sp,
    gp,
    tp,
    t0,
    t1,
    t2,
    s0,
    s1,
    a0,
    a1,
    a2,
    a3,
    a4,
    a5,
    a6,
    a7,
    s2,
    s3,
    s4,
    s5,
    s6,
    s7,
    s8,
    s9,
    s10,
    s11,
    t3,
    t4,
    t5,
    t6,
}
pub struct Gpr {
    regs: [u64; 32],
}

impl Gpr {
    pub fn new() -> Self {
        Gpr { regs: [0; 32] }
    }

    pub fn read(&self, idx: u64) -> u64 {
        assert!(idx < 32);
        if idx == 0 {
            0
        } else {
            self.regs.get(idx as usize).copied().unwrap_or(0)
        }
    }
    pub fn write(&mut self, idx: u64, data: u64) {
        assert!(idx < 32);
        if idx != 0 {
            if let Some(x) = self.regs.get_mut(idx as usize) {
                *x = data;
            }
        }
    }
    pub fn get_register_name(num: u64) -> &'static str {
        assert!(num < 32);
        // 数字转枚举
        match num {
            0 => "zero",
            1 => "ra",
            2 => "sp",
            3 => "gp",
            4 => "tp",
            5 => "t0",
            6 => "t1",
            7 => "t2",
            8 => "s0",
            9 => "s1",
            10 => "a0",
            11 => "a1",
            12 => "a2",
            13 => "a3",
            14 => "a4",
            15 => "a5",
            16 => "a6",
            17 => "a7",
            18 => "s2",
            19 => "s3",
            20 => "s4",
            21 => "s5",
            22 => "s6",
            23 => "s7",
            24 => "s8",
            25 => "s9",
            26 => "s10",
            27 => "s11",
            28 => "t3",
            29 => "t4",
            30 => "t5",
            31 => "t6",
            _ => panic!(),
        }
    }

    pub fn get_register_idx(reg_name: &str) -> GprName {
        match reg_name {
            "zero" => GprName::zero,
            "ra" => GprName::ra,
            "sp" => GprName::sp,
            "gp" => GprName::gp,
            "tp" => GprName::tp,
            "t0" => GprName::t0,
            "t1" => GprName::t1,
            "t2" => GprName::t2,
            "s0" => GprName::s0,
            "s1" => GprName::s1,
            "a0" => GprName::a0,
            "a1" => GprName::a1,
            "a2" => GprName::a2,
            "a3" => GprName::a3,
            "a4" => GprName::a4,
            "a5" => GprName::a5,
            "a6" => GprName::a6,
            "a7" => GprName::a7,
            "s2" => GprName::s2,
            "s3" => GprName::s3,
            "s4" => GprName::s4,
            "s5" => GprName::s5,
            "s6" => GprName::s6,
            "s7" => GprName::s7,
            "s8" => GprName::s8,
            "s9" => GprName::s9,
            "s10" => GprName::s10,
            "s11" => GprName::s11,
            "t3" => GprName::t3,
            "t4" => GprName::t4,
            "t5" => GprName::t5,
            "t6" => GprName::t6,
            _ => panic!(),
        }
    }

    pub fn read_by_name(&mut self, reg_name: &str) -> u64 {
        let idx = Gpr::get_register_idx(reg_name);

        self.read(idx as u64)
    }
}

impl Default for Gpr {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Gpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ret: fmt::Result = Ok(());
        for i in 0..32 {
            ret = f.write_fmt(format_args!(
                "{}:{}\n",
                Gpr::get_register_name(i),
                self.read(i)
            ))
        }
        ret
    }
}
