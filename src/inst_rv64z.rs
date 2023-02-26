use crate::{inst_base::*, mcsr_regs::Mstatus, traptype::TrapType};

#[allow(unused_variables)]
pub const INSTRUCTIONS_Z: [Instruction; 12] = [
    Instruction {
        mask: MASK_EBREAK,
        match_data: MATCH_EBREAK,
        name: "EBREAK",
        operation: |cpu, inst, pc| {
            cpu.halt();
            Ok(())
        },
    },
    Instruction {
        mask: MASK_ECALL,
        match_data: MATCH_ECALL,
        name: "ECALL",
        operation: |cpu, inst, pc| match cpu.cur_priv {
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
            // mode (U if U-mode is implemented, else M). If xPP̸=M, x RET also sets MPRV=0.
            let mstatus_val = cpu.csr_regs.read_raw_mask(CSR_MSTATUS.into(), MASK_ALL);
            let mut mstatus = Mstatus::from(mstatus_val);

            // todo! only support m mode
            mstatus.set_mie(mstatus.mpie());
            cpu.cur_priv = PrivilegeLevels::from_repr(mstatus.mpp().into()).unwrap();
            mstatus.set_mpie(true);
            mstatus.set_mpp(PrivilegeLevels::Machine as u8);

            // println!("MRET:mstatus_now:{mstatus_val:x}");
            cpu.csr_regs
                .write_raw_mask(CSR_MSTATUS.into(), mstatus.into(), MASK_ALL);
            // println!("MRET:mstatus_now2:{mstatus_val:x}");

            let mepc_val = cpu.csr_regs.read_raw_mask(CSR_MEPC.into(), MASK_ALL);
            // println!("mret->{mepc:x}");
            cpu.npc = mepc_val;

            Ok(())
        },
    },
    Instruction {
        mask: MASK_FENCE_I,
        match_data: MATCH_FENCE_I,
        name: "FENCE_I",
        operation: |cpu, inst, pc| {
            cpu.cpu_icache.clear_inst();
            Ok(())
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
            let t = cpu.csr_regs.read(f.csr);

            let rs1_data = cpu.gpr.read(f.rs1);

            let csr_wb_data = t & !rs1_data;
            // println!("CSRRC:{csr_wb_data:x}");

            cpu.csr_regs.write(f.csr, csr_wb_data);
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
            let t = cpu.csr_regs.read(f.csr);

            let rs1_data = cpu.gpr.read(f.rs1);
            // println!("CSRRS_pre:{t:x}");
            let csr_wb_data = t | rs1_data;
            // println!("CSRRS_now:{csr_wb_data:x}");
            cpu.csr_regs.write(f.csr, csr_wb_data);

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

            let t = cpu.csr_regs.read(f.csr);
            let rs1_data = cpu.gpr.read(f.rs1);
            // println!("CSRRW_pre:{t:x}");
            let csr_wb_data = rs1_data;
            // println!("CSRRW_now:{csr_wb_data:x}");
            cpu.csr_regs.write(f.csr, csr_wb_data);

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
            let t = cpu.csr_regs.read(f.csr);
            let zimm = f.rs1;
            // println!("CSRRCI_zimm:{zimm:x}");
            // println!("CSRRCI_pre:{t:x}");

            let csr_wb_data = t & !zimm;
            // println!("CSRRCI_now:{csr_wb_data:x}");
            cpu.csr_regs.write(f.csr, csr_wb_data);
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
            let t = cpu.csr_regs.read(f.csr);
            let zimm = f.rs1;
            // println!("CSRRSI_pre:{t:x}");
            let csr_wb_data = t | zimm;
            // println!("CSRRSI_now:{csr_wb_data:x}");
            cpu.csr_regs.write(f.csr, csr_wb_data);
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

            let t = cpu.csr_regs.read(f.csr);
            let zimm = f.rs1;
            // println!("CSRRWI_pre:{t:x}");
            let csr_wb_data = zimm;
            // println!("CSRRWI_now:{csr_wb_data:x}");
            cpu.csr_regs.write(f.csr, csr_wb_data);
            cpu.gpr.write(f.rd, t);

            Ok(())
        },
    },
];
