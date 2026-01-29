use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Reg {
    X(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    MovImm { dst: Reg, imm: i64 },
    AddImm { dst: Reg, src: Reg, imm: i64 },
    Add { dst: Reg, lhs: Reg, rhs: Reg },
    Sub { dst: Reg, lhs: Reg, rhs: Reg },
    Cmp { lhs: Reg, rhs: Reg },
    Ret,
}

#[derive(Debug, Default, Clone)]
pub struct RegisterFile {
    regs: BTreeMap<Reg, i64>,
    flags: Flags,
}

impl RegisterFile {
    pub fn get(&self, reg: Reg) -> i64 {
        *self.regs.get(&reg).unwrap_or(&0)
    }

    pub fn set(&mut self, reg: Reg, value: i64) {
        self.regs.insert(reg, value);
    }

    pub fn flags(&self) -> Flags {
        self.flags
    }

    fn set_flags(&mut self, flags: Flags) {
        self.flags = flags;
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    pub n: bool,
    pub z: bool,
    pub c: bool,
    pub v: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error("unsupported instruction")]
    Unsupported,
}

pub fn execute_block(instructions: &[Instruction], regs: &mut RegisterFile) -> Result<(), ExecError> {
    for inst in instructions {
        match *inst {
            Instruction::MovImm { dst, imm } => regs.set(dst, imm),
            Instruction::AddImm { dst, src, imm } => {
                let src_val = regs.get(src);
                let (result, flags) = add_with_flags(src_val, imm);
                regs.set(dst, result);
                regs.set_flags(flags);
            }
            Instruction::Add { dst, lhs, rhs } => {
                let lhs_val = regs.get(lhs);
                let rhs_val = regs.get(rhs);
                let (result, flags) = add_with_flags(lhs_val, rhs_val);
                regs.set(dst, result);
                regs.set_flags(flags);
            }
            Instruction::Sub { dst, lhs, rhs } => {
                let lhs_val = regs.get(lhs);
                let rhs_val = regs.get(rhs);
                let (result, flags) = sub_with_flags(lhs_val, rhs_val);
                regs.set(dst, result);
                regs.set_flags(flags);
            }
            Instruction::Cmp { lhs, rhs } => {
                let lhs_val = regs.get(lhs);
                let rhs_val = regs.get(rhs);
                let (_, flags) = sub_with_flags(lhs_val, rhs_val);
                regs.set_flags(flags);
            }
            Instruction::Ret => return Ok(()),
        }
    }
    Ok(())
}

fn add_with_flags(lhs: i64, rhs: i64) -> (i64, Flags) {
    let (_result, carry) = (lhs as u64).overflowing_add(rhs as u64);
    let signed = lhs.wrapping_add(rhs);
    let overflow = ((lhs ^ signed) & (rhs ^ signed)) < 0;
    let flags = Flags {
        n: signed < 0,
        z: signed == 0,
        c: !carry,
        v: overflow,
    };
    (signed, flags)
}

fn sub_with_flags(lhs: i64, rhs: i64) -> (i64, Flags) {
    let (_result, borrow) = (lhs as u64).overflowing_sub(rhs as u64);
    let signed = lhs.wrapping_sub(rhs);
    let overflow = ((lhs ^ rhs) & (lhs ^ signed)) < 0;
    let flags = Flags {
        n: signed < 0,
        z: signed == 0,
        c: !borrow,
        v: overflow,
    };
    (signed, flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mov_and_add() {
        let mut regs = RegisterFile::default();
        let block = [
            Instruction::MovImm {
                dst: Reg::X(0),
                imm: 7,
            },
            Instruction::MovImm {
                dst: Reg::X(1),
                imm: 5,
            },
            Instruction::Add {
                dst: Reg::X(2),
                lhs: Reg::X(0),
                rhs: Reg::X(1),
            },
            Instruction::Ret,
        ];

        execute_block(&block, &mut regs).expect("exec ok");
        assert_eq!(regs.get(Reg::X(2)), 12);
    }

    #[test]
    fn sub_and_cmp_set_flags() {
        let mut regs = RegisterFile::default();
        let block = [
            Instruction::MovImm {
                dst: Reg::X(0),
                imm: 10,
            },
            Instruction::MovImm {
                dst: Reg::X(1),
                imm: 10,
            },
            Instruction::Cmp {
                lhs: Reg::X(0),
                rhs: Reg::X(1),
            },
            Instruction::Sub {
                dst: Reg::X(2),
                lhs: Reg::X(0),
                rhs: Reg::X(1),
            },
            Instruction::Ret,
        ];

        execute_block(&block, &mut regs).expect("exec ok");
        assert_eq!(regs.get(Reg::X(2)), 0);
        let flags = regs.flags();
        assert!(flags.z);
        assert!(!flags.n);
        assert!(flags.c);
    }

    #[test]
    fn add_immediate_sets_negative_flag() {
        let mut regs = RegisterFile::default();
        let block = [
            Instruction::MovImm {
                dst: Reg::X(0),
                imm: -2,
            },
            Instruction::AddImm {
                dst: Reg::X(1),
                src: Reg::X(0),
                imm: -1,
            },
            Instruction::Ret,
        ];

        execute_block(&block, &mut regs).expect("exec ok");
        assert_eq!(regs.get(Reg::X(1)), -3);
        let flags = regs.flags();
        assert!(flags.n);
        assert!(!flags.z);
    }
}
