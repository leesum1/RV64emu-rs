use crate::{rv64core::inst::inst_base::*, rv64core::traptype::TrapType};

#[allow(unused_variables)]
pub const INSTRUCTIONS_Z: &[Instruction] = &[
    Instruction {
        mask: MASK_EBREAK,
        match_data: MATCH_EBREAK,
        name: "EBREAK",
        operation: |cpu, inst, pc| {
            #[cfg(feature = "support_am")]
            cpu.halt();
            Err(TrapType::Breakpoint(pc))
        },
    },
    Instruction {
        mask: MASK_ECALL,
        match_data: MATCH_ECALL,
        name: "ECALL",
        operation: |cpu, inst, pc| match cpu.cur_priv.get() {
            PrivilegeLevels::User => Err(TrapType::EnvironmentCallFromUMode),
            PrivilegeLevels::Supervisor => Err(TrapType::EnvironmentCallFromSMode),
            PrivilegeLevels::Machine => Err(TrapType::EnvironmentCallFromMMode),
        },
    },
    Instruction {
        mask: MASK_WFI,
        match_data: MATCH_WFI,
        name: "WFI",
        operation: |cpu, inst, pc| Ok(()),
    },
    Instruction {
        mask: MASK_MRET,
        match_data: MATCH_MRET,
        name: "MRET",
        operation: |cpu, inst, pc| {
            // An MRET or SRET instruction is used to return from a trap in M-mode or S-mode respectively.
            // When executing an xRET instruction, supposing xPP holds the value y, xIE is set to xPIE; the
            // privilege mode is changed to y; xPIE is set to 1; and xPP is set to the least-privileged supported
            // mode (U if U-mode is implemented, else M). If xPP̸=M, xRET also sets MPRV=0.
            // let mstatus_val = cpu.csr_regs.read_raw(CSR_MSTATUS.into());
            // let mut mstatus = Mstatus::from(mstatus_val);
            let mut mstatus = cpu.csr_regs.xstatus.get();

            // supposing xPP holds the value y
            let y = mstatus.get_mpp_priv();
            // xIE is set to xPIE
            mstatus.set_mie(mstatus.mpie());
            // the privilege mode is changed to xPP
            cpu.cur_priv.set(y);
            // xPIE is set to 1;
            mstatus.set_mpie(true);
            // xPP is set to the least-privileged supported mode
            // (U if U-mode is implemented, else M).
            mstatus.set_mpp(PrivilegeLevels::User as u8);
            // (If xPP̸=M, x RET also sets MPRV=0.) Clarify => (If y!=M, x RET also sets MPRV=0.)
            // reference to  https://github.com/riscv/riscv-isa-manual/pull/929
            if y != PrivilegeLevels::Machine {
                mstatus.set_mprv(false);
            }

            // warn!("MRET:mstatus_now:{mstatus_val:x}");
            cpu.csr_regs.xstatus.set(mstatus);
            // warn!("MRET:mstatus_now2:{mstatus_val:x}");

            // let mepc_val = cpu.csr_regs.read_raw(CSR_MEPC.into());
            let mepc = cpu.csr_regs.mepc.get();
            // warn!("mret->{mepc_val:x}");
            cpu.npc = mepc;

            Ok(())
        },
    },
    Instruction {
        mask: MASK_SRET,
        match_data: MATCH_SRET,
        name: "SRET",
        operation: |cpu, inst, pc| {
            //  SRET should also raise an illegal instruction exception when TSR=1 in mstatus
            // An MRET or SRET instruction is used to return from a trap in M-mode or S-mode respectively.
            // When executing an xRET instruction, supposing xPP holds the value y, xIE is set to xPIE; the
            // privilege mode is changed to y; xPIE is set to 1; and xPP is set to the least-privileged supported
            // mode (U if U-mode is implemented, else M). If xPP̸=M, x RET also sets MPRV=0.
            //  xRET sets the pc to the value stored in the xepc register.
            // let mstatus_val = cpu.csr_regs.read_raw(CSR_MSTATUS.into());
            // let mut mstatus = Mstatus::from(mstatus_val);
            let mut mstatus = cpu.csr_regs.xstatus.get();

            // SRET should also raise an illegal instruction exception when TSR=1 in mstatus
            if mstatus.tsr() {
                return Err(TrapType::IllegalInstruction(inst.into()));
            }

            // supposing xPP holds the value y
            let y = mstatus.get_spp_priv();
            // xIE is set to xPIE
            mstatus.set_sie(mstatus.spie());
            // the privilege mode is changed to xPP
            cpu.cur_priv.set(y);
            // xPIE is set to 1;
            mstatus.set_spie(true);
            // xPP is set to the least-privileged supported mode
            // (U if U-mode is implemented, else M). 0 : user mode
            mstatus.set_spp(false);
            // (If xPP̸=M, x RET also sets MPRV=0.) Clarify => (If y!=M, x RET also sets MPRV=0.)
            // reference to  https://github.com/riscv/riscv-isa-manual/pull/929
            if y != PrivilegeLevels::Machine {
                mstatus.set_mprv(false);
            }

            // cpu.csr_regs.write_raw(CSR_MSTATUS.into(), mstatus.into());
            cpu.csr_regs.xstatus.set(mstatus);
            // warn!("MRET:mstatus_now2:{mstatus_val:x}");

            // xRET sets the pc to the value stored in the xepc register.
            // let sepc_val = cpu.csr_regs.read_raw(CSR_SEPC.into());
            let sepc = cpu.csr_regs.sepc.get();
            // warn!("sret->{sepc_val:x}");
            cpu.npc = sepc;

            Ok(())
        },
    },
    Instruction {
        mask: MASK_FENCE_I,
        match_data: MATCH_FENCE_I,
        name: "FENCE_I",
        operation: |cpu, inst, pc| {
            cpu.cache_system.lock().dcache.clear();
            cpu.cache_system.lock().icache.clear_inst();
            Ok(())
        },
    },
    Instruction {
        mask: MASK_SFENCE_VMA,
        match_data: MATCH_SFENCE_VMA,
        name: "SFENCE_VMA",
        operation: |cpu, inst, pc| {
            // if (cur_priv < S_MODE || (cur_priv == S_MODE && mstatus->tvm))
            // return false;
            // require_privilege(get_field(STATE.mstatus->read(), MSTATUS_TVM) ? PRV_M : PRV_S);

            let mstatus = cpu.csr_regs.xstatus.get();
            let cur_priv = cpu.cur_priv.get();

            let require_priv = if mstatus.tvm() {
                PrivilegeLevels::Machine
            } else {
                PrivilegeLevels::Supervisor
            };

            // warn!("SFENCE_VMA:cur_priv:{:?},require_priv:{:?}", cur_priv, require_priv);

            if !require_priv.check_priv(cur_priv) {
                Err(TrapType::IllegalInstruction(inst.into()))
            } else {
                // PASS icache-alias.S
                #[cfg(feature = "inst_cache")]
                cpu.cache_system.lock().icache.clear_inst();
                #[cfg(feature = "data_cache")]
                cpu.cache_system.lock().dcache.clear();

                Ok(())
            }
        },
    },
    Instruction {
        mask: MASK_FENCE,
        match_data: MATCH_FENCE,
        name: "FENCE",
        operation: |cpu, inst, pc| Ok(()),
    },
    Instruction {
        mask: MASK_CSRRC,
        match_data: MATCH_CSRRC,
        name: "CSRRC",
        operation: |cpu, inst, pc| {
            // t = CSRs[csr]; CSRs[csr] = t &∼x[rs1]; x[rd] = t
            let f = parse_format_csr(inst);
            let csr_ret = cpu.csr_regs.read(f.csr, cpu.cur_priv.get());

            let t = match csr_ret {
                Ok(val) => val,
                Err(trap_type) => return Err(trap_type),
            };

            let rs1_data = cpu.gpr.read(f.rs1);

            let csr_wb_data = t & !rs1_data;
            // warn!("CSRRC:{csr_wb_data:x}");
            if t != csr_wb_data {
                let csr_ret = cpu.csr_regs.write(f.csr, csr_wb_data, cpu.cur_priv.get());
                if let Err(trap_type) = csr_ret {
                    return Err(trap_type);
                };
            };
            cpu.gpr.write(f.rd, t);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_CSRRS,
        match_data: MATCH_CSRRS,
        name: "CSRRS",
        operation: |cpu, inst, pc| {
            // t = CSRs[csr]; CSRs[csr] = t | x[rs1]; x[rd] = t
            let f = parse_format_csr(inst);
            let csr_ret = cpu.csr_regs.read(f.csr, cpu.cur_priv.get());

            let t = match csr_ret {
                Ok(val) => val,
                Err(trap_type) => return Err(trap_type),
            };

            let rs1_data = cpu.gpr.read(f.rs1);
            let csr_wb_data = t | rs1_data;

            if t != csr_wb_data {
                let csr_ret = cpu.csr_regs.write(f.csr, csr_wb_data, cpu.cur_priv.get());
                if let Err(trap_type) = csr_ret {
                    return Err(trap_type);
                };
            }

            cpu.gpr.write(f.rd, t);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_CSRRW,
        match_data: MATCH_CSRRW,
        name: "CSRRW",
        operation: |cpu, inst, pc| {
            // t = CSRs[csr]; CSRs[csr] = x[rs1]; x[rd] = t
            let f = parse_format_csr(inst);

            let csr_ret = cpu.csr_regs.read(f.csr, cpu.cur_priv.get());

            let t = match csr_ret {
                Ok(val) => val,
                Err(trap_type) => return Err(trap_type),
            };
            let rs1_data = cpu.gpr.read(f.rs1);
            // warn!("CSRRW_pre:{t:x}");
            let csr_wb_data = rs1_data;
            // warn!("CSRRW_now:{csr_wb_data:x}");
            if t != csr_wb_data {
                let csr_ret = cpu.csr_regs.write(f.csr, csr_wb_data, cpu.cur_priv.get());
                if let Err(trap_type) = csr_ret {
                    return Err(trap_type);
                };
            }
            cpu.gpr.write(f.rd, t);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_CSRRCI,
        match_data: MATCH_CSRRCI,
        name: "CSRRCI",
        operation: |cpu, inst, pc| {
            // t = CSRs[csr]; CSRs[csr] = t &∼zimm; x[rd] =
            let f = parse_format_csr(inst);
            let csr_ret = cpu.csr_regs.read(f.csr, cpu.cur_priv.get());

            let t = match csr_ret {
                Ok(val) => val,
                Err(trap_type) => return Err(trap_type),
            };
            let zimm = f.rs1;
            // warn!("CSRRCI_zimm:{zimm:x}");
            // warn!("CSRRCI_pre:{t:x}");

            let csr_wb_data = t & !zimm;
            // warn!("CSRRCI_now:{csr_wb_data:x}");
            if t != csr_wb_data {
                let csr_ret = cpu.csr_regs.write(f.csr, csr_wb_data, cpu.cur_priv.get());
                if let Err(trap_type) = csr_ret {
                    return Err(trap_type);
                };
            }
            cpu.gpr.write(f.rd, t);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_CSRRSI,
        match_data: MATCH_CSRRSI,
        name: "CSRRSI",
        operation: |cpu, inst, pc| {
            // t = CSRs[csr]; CSRs[csr] = t | zimm; x[rd] = t
            let f = parse_format_csr(inst);
            let csr_ret = cpu.csr_regs.read(f.csr, cpu.cur_priv.get());

            let t = match csr_ret {
                Ok(val) => val,
                Err(trap_type) => return Err(trap_type),
            };
            let zimm = f.rs1;
            // warn!("CSRRSI_pre:{t:x}");
            let csr_wb_data = t | zimm;
            // warn!("CSRRSI_now:{csr_wb_data:x}");
            if t != csr_wb_data {
                let csr_ret = cpu.csr_regs.write(f.csr, csr_wb_data, cpu.cur_priv.get());
                if let Err(trap_type) = csr_ret {
                    return Err(trap_type);
                };
            }
            cpu.gpr.write(f.rd, t);

            Ok(())
        },
    },
    Instruction {
        mask: MASK_CSRRWI,
        match_data: MATCH_CSRRWI,
        name: "CSRRWI",
        operation: |cpu, inst, pc| {
            // x[rd] = CSRs[csr]; CSRs[csr] = zimm
            let f = parse_format_csr(inst);

            let csr_ret = cpu.csr_regs.read(f.csr, cpu.cur_priv.get());

            let t = match csr_ret {
                Ok(val) => val,
                Err(trap_type) => return Err(trap_type),
            };
            let zimm = f.rs1;
            // warn!("CSRRWI_pre:{t:x}");
            let csr_wb_data = zimm;
            // warn!("CSRRWI_now:{csr_wb_data:x}");
            if t != csr_wb_data {
                let csr_ret = cpu.csr_regs.write(f.csr, csr_wb_data, cpu.cur_priv.get());
                if let Err(trap_type) = csr_ret {
                    return Err(trap_type);
                };
            }
            cpu.gpr.write(f.rd, t);

            Ok(())
        },
    },
];
