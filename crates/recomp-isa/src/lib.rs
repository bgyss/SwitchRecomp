use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Reg {
    X(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    MovImm {
        dst: Reg,
        imm: i64,
    },
    AddImm {
        dst: Reg,
        src: Reg,
        imm: i64,
    },
    Add {
        dst: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Sub {
        dst: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Cmp {
        lhs: Reg,
        rhs: Reg,
    },
    LslImm {
        dst: Reg,
        src: Reg,
        shift: u8,
    },
    LsrImm {
        dst: Reg,
        src: Reg,
        shift: u8,
    },
    AsrImm {
        dst: Reg,
        src: Reg,
        shift: u8,
    },
    RorImm {
        dst: Reg,
        src: Reg,
        shift: u8,
    },
    LdrImm {
        dst: Reg,
        base: Reg,
        offset: i64,
        size: MemSize,
    },
    StrImm {
        src: Reg,
        base: Reg,
        offset: i64,
        size: MemSize,
    },
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
    #[error("unaligned memory access at {address} size {size}")]
    Unaligned { address: usize, size: usize },
    #[error("memory out of bounds at {address} size {size}")]
    OutOfBounds { address: usize, size: usize },
}

#[derive(Debug, Clone)]
pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0; size],
        }
    }

    pub fn read(&self, address: usize, size: MemSize) -> Result<u64, ExecError> {
        let width = size.bytes();
        if address % width != 0 {
            return Err(ExecError::Unaligned {
                address,
                size: width,
            });
        }
        if address + width > self.data.len() {
            return Err(ExecError::OutOfBounds {
                address,
                size: width,
            });
        }
        let mut value = 0u64;
        for i in 0..width {
            value |= (self.data[address + i] as u64) << (i * 8);
        }
        Ok(value)
    }

    pub fn write(&mut self, address: usize, size: MemSize, value: u64) -> Result<(), ExecError> {
        let width = size.bytes();
        if address % width != 0 {
            return Err(ExecError::Unaligned {
                address,
                size: width,
            });
        }
        if address + width > self.data.len() {
            return Err(ExecError::OutOfBounds {
                address,
                size: width,
            });
        }
        for i in 0..width {
            self.data[address + i] = ((value >> (i * 8)) & 0xFF) as u8;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemSize {
    Byte,
    Half,
    Word,
    DWord,
}

impl MemSize {
    pub fn bytes(self) -> usize {
        match self {
            MemSize::Byte => 1,
            MemSize::Half => 2,
            MemSize::Word => 4,
            MemSize::DWord => 8,
        }
    }
}

pub fn execute_block(
    instructions: &[Instruction],
    regs: &mut RegisterFile,
    mem: &mut Memory,
) -> Result<(), ExecError> {
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
            Instruction::LslImm { dst, src, shift } => {
                let value = regs.get(src) as u64;
                let (result, carry) = shift_left(value, shift);
                regs.set(dst, result as i64);
                regs.set_flags(Flags {
                    n: (result as i64) < 0,
                    z: result == 0,
                    c: carry,
                    v: false,
                });
            }
            Instruction::LsrImm { dst, src, shift } => {
                let value = regs.get(src) as u64;
                let (result, carry) = shift_right_logical(value, shift);
                regs.set(dst, result as i64);
                regs.set_flags(Flags {
                    n: false,
                    z: result == 0,
                    c: carry,
                    v: false,
                });
            }
            Instruction::AsrImm { dst, src, shift } => {
                let value = regs.get(src);
                let (result, carry) = shift_right_arithmetic(value, shift);
                regs.set(dst, result);
                regs.set_flags(Flags {
                    n: result < 0,
                    z: result == 0,
                    c: carry,
                    v: false,
                });
            }
            Instruction::RorImm { dst, src, shift } => {
                let value = regs.get(src) as u64;
                let (result, carry) = rotate_right(value, shift);
                regs.set(dst, result as i64);
                regs.set_flags(Flags {
                    n: (result as i64) < 0,
                    z: result == 0,
                    c: carry,
                    v: false,
                });
            }
            Instruction::LdrImm {
                dst,
                base,
                offset,
                size,
            } => {
                let address = effective_address(regs.get(base), offset)?;
                let raw = mem.read(address, size)?;
                regs.set(dst, raw as i64);
                regs.set_flags(Flags {
                    n: (raw as i64) < 0,
                    z: raw == 0,
                    c: false,
                    v: false,
                });
            }
            Instruction::StrImm {
                src,
                base,
                offset,
                size,
            } => {
                let address = effective_address(regs.get(base), offset)?;
                let value = regs.get(src) as u64;
                mem.write(address, size, value)?;
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

fn shift_left(value: u64, shift: u8) -> (u64, bool) {
    let shift = shift.min(63) as u32;
    if shift == 0 {
        return (value, false);
    }
    let carry = ((value >> (64 - shift)) & 1) == 1;
    (value << shift, carry)
}

fn shift_right_logical(value: u64, shift: u8) -> (u64, bool) {
    let shift = shift.min(63) as u32;
    if shift == 0 {
        return (value, false);
    }
    let carry = ((value >> (shift - 1)) & 1) == 1;
    (value >> shift, carry)
}

fn shift_right_arithmetic(value: i64, shift: u8) -> (i64, bool) {
    let shift = shift.min(63) as u32;
    if shift == 0 {
        return (value, false);
    }
    let carry = (((value as u64) >> (shift - 1)) & 1) == 1;
    (value >> shift, carry)
}

fn rotate_right(value: u64, shift: u8) -> (u64, bool) {
    let shift = (shift as u32) & 63;
    if shift == 0 {
        return (value, false);
    }
    let result = value.rotate_right(shift);
    let carry = (result >> 63) & 1 == 1;
    (result, carry)
}

fn effective_address(base: i64, offset: i64) -> Result<usize, ExecError> {
    let addr = base.wrapping_add(offset);
    if addr < 0 {
        return Err(ExecError::OutOfBounds {
            address: 0,
            size: 1,
        });
    }
    Ok(addr as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mov_and_add() {
        let mut regs = RegisterFile::default();
        let mut mem = Memory::new(0);
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

        execute_block(&block, &mut regs, &mut mem).expect("exec ok");
        assert_eq!(regs.get(Reg::X(2)), 12);
    }

    #[test]
    fn sub_and_cmp_set_flags() {
        let mut regs = RegisterFile::default();
        let mut mem = Memory::new(0);
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

        execute_block(&block, &mut regs, &mut mem).expect("exec ok");
        assert_eq!(regs.get(Reg::X(2)), 0);
        let flags = regs.flags();
        assert!(flags.z);
        assert!(!flags.n);
        assert!(flags.c);
    }

    #[test]
    fn add_immediate_sets_negative_flag() {
        let mut regs = RegisterFile::default();
        let mut mem = Memory::new(0);
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

        execute_block(&block, &mut regs, &mut mem).expect("exec ok");
        assert_eq!(regs.get(Reg::X(1)), -3);
        let flags = regs.flags();
        assert!(flags.n);
        assert!(!flags.z);
    }

    #[test]
    fn shifts_set_carry_and_zero() {
        let mut regs = RegisterFile::default();
        let mut mem = Memory::new(0);
        let block = [
            Instruction::MovImm {
                dst: Reg::X(0),
                imm: i64::MIN,
            },
            Instruction::LslImm {
                dst: Reg::X(1),
                src: Reg::X(0),
                shift: 1,
            },
            Instruction::LsrImm {
                dst: Reg::X(2),
                src: Reg::X(1),
                shift: 1,
            },
            Instruction::LsrImm {
                dst: Reg::X(3),
                src: Reg::X(2),
                shift: 1,
            },
            Instruction::Ret,
        ];

        execute_block(&block, &mut regs, &mut mem).expect("exec ok");
        assert_eq!(regs.get(Reg::X(1)), 0);
        assert_eq!(regs.get(Reg::X(2)), 0);
        assert_eq!(regs.get(Reg::X(3)), 0);
        let flags = regs.flags();
        assert!(flags.z);
        assert!(!flags.c);
    }

    #[test]
    fn rotate_right_sets_negative() {
        let mut regs = RegisterFile::default();
        let mut mem = Memory::new(0);
        let block = [
            Instruction::MovImm {
                dst: Reg::X(0),
                imm: 1,
            },
            Instruction::RorImm {
                dst: Reg::X(1),
                src: Reg::X(0),
                shift: 1,
            },
            Instruction::Ret,
        ];

        execute_block(&block, &mut regs, &mut mem).expect("exec ok");
        assert_eq!(regs.get(Reg::X(1)), i64::MIN);
        let flags = regs.flags();
        assert!(flags.n);
        assert!(!flags.z);
    }

    #[test]
    fn lsr_sets_carry_from_bit0() {
        let mut regs = RegisterFile::default();
        let mut mem = Memory::new(0);
        let block = [
            Instruction::MovImm {
                dst: Reg::X(0),
                imm: 3,
            },
            Instruction::LsrImm {
                dst: Reg::X(1),
                src: Reg::X(0),
                shift: 1,
            },
            Instruction::Ret,
        ];

        execute_block(&block, &mut regs, &mut mem).expect("exec ok");
        assert_eq!(regs.get(Reg::X(1)), 1);
        let flags = regs.flags();
        assert!(!flags.z);
        assert!(flags.c);
    }

    #[test]
    fn load_store_with_alignment() {
        let mut regs = RegisterFile::default();
        let mut mem = Memory::new(16);
        let block = [
            Instruction::MovImm {
                dst: Reg::X(0),
                imm: 8,
            },
            Instruction::MovImm {
                dst: Reg::X(1),
                imm: 0xAABBCCDD,
            },
            Instruction::StrImm {
                src: Reg::X(1),
                base: Reg::X(0),
                offset: 0,
                size: MemSize::Word,
            },
            Instruction::LdrImm {
                dst: Reg::X(2),
                base: Reg::X(0),
                offset: 0,
                size: MemSize::Word,
            },
            Instruction::Ret,
        ];

        execute_block(&block, &mut regs, &mut mem).expect("exec ok");
        assert_eq!(regs.get(Reg::X(2)), 0xAABBCCDD);
    }

    #[test]
    fn load_unaligned_is_error() {
        let mut regs = RegisterFile::default();
        let mut mem = Memory::new(16);
        let block = [
            Instruction::MovImm {
                dst: Reg::X(0),
                imm: 3,
            },
            Instruction::LdrImm {
                dst: Reg::X(1),
                base: Reg::X(0),
                offset: 0,
                size: MemSize::Word,
            },
        ];

        let err = execute_block(&block, &mut regs, &mut mem).unwrap_err();
        assert!(matches!(err, ExecError::Unaligned { .. }));
    }
}
