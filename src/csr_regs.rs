
use std::{fmt, str::FromStr};

use strum_macros::{Display, EnumString, FromRepr, IntoStaticStr};

#[derive(EnumString, FromRepr, IntoStaticStr, Display, Debug, PartialEq)]
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
        match idx {
            0 => 0,
            _ => self.regs[idx as usize],
        }
    }
    pub fn write(&mut self, idx: u64, data: u64) {
        assert!(idx < 32);
        match idx {
            0 => (),
            _ => self.regs[idx as usize] = data,
        };
    }
    pub fn get_register_name(num: u64) -> &'static str {
        assert!(num < 32);
        // 数字转枚举
        let name = GprName::from_repr(num as usize).expect("can not get_register_name");
        name.into()
    }

    pub fn get_register_idx(reg_name: &str) -> GprName {
        match GprName::from_str(reg_name) {
            Ok(gpr_emu) => gpr_emu,
            Err(_) => panic!(),
        }
    }

    pub fn read_by_name(&mut self,reg_name: &str) ->u64{
        let idx = Gpr::get_register_idx(reg_name);

        self.read(idx as u64)
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

#[cfg(test)]
mod test_gpr {
    use std::str::FromStr;

    use crate::gpr::Gpr;
    use crate::gpr::GprName;

    #[test]
    fn test1() {
        // 枚举转换字符串
        let test1 = GprName::zero.to_string();
        assert_eq!(test1, "zero");
    }
    #[test]
    fn test2() {
        // 枚举转数字
        let test2 = GprName::zero;
        assert_eq!(test2 as u64, 0);
    }
    #[test]

    fn test3() {
        // 字符串转枚举
        let reg_name_var = GprName::from_str("a0").expect("a0");
        assert_eq!(reg_name_var, GprName::a0);

        let reg_name_var = GprName::from_str("a1").expect("a1");
        assert_eq!(reg_name_var, GprName::a1);
    }

    #[test]
    fn test4() {
        let mut gpr = Gpr::new();
        gpr.write(1, 1);
        gpr.write(0, 1);
        gpr.write(10, 10);
        let x10 = gpr.read(10);
        let x0 = gpr.read(0);
        let x1 = gpr.read(1);
        assert_eq!(x0, 0);
        assert_eq!(x1, 1);
        assert_eq!(x10, 10);
        println!("{gpr}");
    }

    #[test]
    fn test5() {
        let x = Gpr::get_register_idx("a2");
        assert_eq!(x, GprName::a2);
    }
}
