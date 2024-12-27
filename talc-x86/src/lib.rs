use std::iter::once;
use std::ops::Not;
use std::{collections::BTreeMap, mem::replace};

use iced_x86::Instruction;
use iced_x86::{MemorySize, OpKind, Register};
use talc_common::*;
use waffle::{
    Block, BlockTarget, Func, FunctionBody, Memory, MemoryArg, Module, Operator, Terminator, Type,
    Value,
};
#[derive(Clone)]
pub struct Regs {
    pub gprs: [Value; 16],
    pub flags: Value,
}
impl Regs {
    pub const N: usize = 17;
    pub fn get_flag<C: Cfg>(&self, f: &mut FunctionBody, k: Block, i: u8) -> Value {
        let mask = 1u64 << i;
        let m = f.add_op(k, C::const_64(mask), &[], &[C::ty()]);
        let m = f.add_op(k, cdef!(C => And), &[m, self.flags], &[C::ty()]);
        let v = f.add_op(k, cdef!(C => Eqz), &[m], &[Type::I32]);
        return f.add_op(k, Operator::I32Eqz, &[v], &[Type::I32]);
    }
    pub fn set_flag<C: Cfg>(&mut self, f: &mut FunctionBody, k: Block, i: u8, w: Value) {
        let w = f.add_op(k, Operator::I32Eqz, &[w], &[Type::I32]);
        let w = f.add_op(k, Operator::I32Eqz, &[w], &[Type::I32]);
        let w = if C::MEMORY64 {
            f.add_op(k, Operator::I64ExtendI32U, &[w], &[C::ty()])
        } else {
            w
        };
        let mask = 1u64 << i;
        let ws = f.add_op(k, Operator::I32Const { value: i as u32 }, &[], &[Type::I32]);
        let w = f.add_op(k, cdef!(C => Shl), &[w, ws], &[C::ty()]);
        let v = self.flags;
        let mask = mask.not();
        let nmask = mask.not();

        let mask = f.add_op(k, C::const_64(mask), &[], &[C::ty()]);
        let nmask = f.add_op(k, C::const_64(nmask), &[], &[C::ty()]);
        let v = f.add_op(k, cdef!(C => And), &[v, mask], &[C::ty()]);
        let w = f.add_op(k, cdef!(C => And), &[w, nmask], &[C::ty()]);
        self.flags = f.add_op(k, cdef!(C => Or), &[v, w], &[C::ty()]);
    }
    pub fn get_32<C: Cfg>(&self, a: u8, f: &mut FunctionBody, k: Block) -> Value {
        let v = self.gprs[a as usize];
        let v = if C::MEMORY64 {
            let mask = f.add_op(k, C::const_64(0xffffffffu64), &[], &[C::ty()]);
            let v = f.add_op(k, cdef!(C => And), &[v, mask], &[C::ty()]);
            v
        } else {
            v
        };
        return v;
    }
    pub fn set_32<C: Cfg>(&mut self, a: u8, f: &mut FunctionBody, k: Block, w: Value) {
        let v = self.gprs[a as usize];
        if C::MEMORY64 {
            let mask = 0xffffffffu64.not();
            let nmask = mask.not();

            let mask = f.add_op(k, C::const_64(mask), &[], &[C::ty()]);
            let nmask = f.add_op(k, C::const_64(nmask), &[], &[C::ty()]);
            let v = f.add_op(k, cdef!(C => And), &[v, mask], &[C::ty()]);
            let w = f.add_op(k, cdef!(C => And), &[w, nmask], &[C::ty()]);
            self.gprs[a as usize] = f.add_op(k, cdef!(C => Or), &[v, w], &[C::ty()]);
        } else {
            self.gprs[a as usize] = w;
        }
    }
    pub fn set_low<C: Cfg>(&mut self, a: u8, f: &mut FunctionBody, k: Block, w: Value, m: u64) {
        let v = self.gprs[a as usize];
        let mask = m.not();
        let nmask = mask.not();

        let mask = f.add_op(k, C::const_64(mask), &[], &[C::ty()]);
        let nmask = f.add_op(k, C::const_64(nmask), &[], &[C::ty()]);
        let v = f.add_op(k, cdef!(C => And), &[v, mask], &[C::ty()]);
        let w = f.add_op(k, cdef!(C => And), &[w, nmask], &[C::ty()]);
        self.gprs[a as usize] = f.add_op(k, cdef!(C => Or), &[v, w], &[C::ty()]);
    }
    pub fn get_low<C: Cfg>(&self, a: u8, f: &mut FunctionBody, k: Block, m: u64) -> Value {
        let v = self.gprs[a as usize];
        let mask = f.add_op(k, C::const_64(m), &[], &[C::ty()]);
        let v = f.add_op(k, cdef!(C => And), &[v, mask], &[C::ty()]);
        return v;
    }
    pub fn get<C: Cfg>(&self, a: Register, f: &mut FunctionBody, k: Block) -> Value {
        match a {
            Register::None => todo!(),
            Register::AH => {
                let v = self.get_low::<C>(0, f, k, 0xff00);
                let a = f.add_op(k, Operator::I32Const { value: 8 }, &[], &[Type::I32]);
                f.add_op(k, cdef!(C => ShrU), &[v, a], &[C::ty()])
            }
            Register::CH => {
                let v = self.get_low::<C>(1, f, k, 0xff00);
                let a = f.add_op(k, Operator::I32Const { value: 8 }, &[], &[Type::I32]);
                f.add_op(k, cdef!(C => ShrU), &[v, a], &[C::ty()])
            }
            Register::DH => {
                let v = self.get_low::<C>(2, f, k, 0xff00);
                let a = f.add_op(k, Operator::I32Const { value: 8 }, &[], &[Type::I32]);
                f.add_op(k, cdef!(C => ShrU), &[v, a], &[C::ty()])
            }
            Register::BH => {
                let v = self.get_low::<C>(3, f, k, 0xff00);
                let a = f.add_op(k, Operator::I32Const { value: 8 }, &[], &[Type::I32]);
                f.add_op(k, cdef!(C => ShrU), &[v, a], &[C::ty()])
            }
            Register::AL => self.get_low::<C>(0, f, k, 0xff),
            Register::CL => self.get_low::<C>(1, f, k, 0xff),
            Register::DL => self.get_low::<C>(2, f, k, 0xff),
            Register::BL => self.get_low::<C>(3, f, k, 0xff),
            Register::SPL => self.get_low::<C>(4, f, k, 0xff),
            Register::BPL => self.get_low::<C>(5, f, k, 0xff),
            Register::SIL => self.get_low::<C>(6, f, k, 0xff),
            Register::DIL => self.get_low::<C>(7, f, k, 0xff),
            Register::R8L => self.get_low::<C>(8, f, k, 0xff),
            Register::R9L => self.get_low::<C>(9, f, k, 0xff),
            Register::R10L => self.get_low::<C>(10, f, k, 0xff),
            Register::R11L => self.get_low::<C>(11, f, k, 0xff),
            Register::R12L => self.get_low::<C>(12, f, k, 0xff),
            Register::R13L => self.get_low::<C>(13, f, k, 0xff),
            Register::R14L => self.get_low::<C>(14, f, k, 0xff),
            Register::R15L => self.get_low::<C>(15, f, k, 0xff),
            Register::AX => self.get_low::<C>(0, f, k, 0xffff),
            Register::CX => self.get_low::<C>(1, f, k, 0xffff),
            Register::DX => self.get_low::<C>(2, f, k, 0xffff),
            Register::BX => self.get_low::<C>(3, f, k, 0xffff),
            Register::SP => self.get_low::<C>(4, f, k, 0xffff),
            Register::BP => self.get_low::<C>(5, f, k, 0xffff),
            Register::SI => self.get_low::<C>(6, f, k, 0xffff),
            Register::DI => self.get_low::<C>(7, f, k, 0xffff),
            Register::R8W => self.get_low::<C>(8, f, k, 0xffff),
            Register::R9W => self.get_low::<C>(9, f, k, 0xffff),
            Register::R10W => self.get_low::<C>(10, f, k, 0xffff),
            Register::R11W => self.get_low::<C>(11, f, k, 0xffff),
            Register::R12W => self.get_low::<C>(12, f, k, 0xffff),
            Register::R13W => self.get_low::<C>(13, f, k, 0xffff),
            Register::R14W => self.get_low::<C>(14, f, k, 0xffff),
            Register::R15W => self.get_low::<C>(15, f, k, 0xffff),
            Register::EAX => self.get_32::<C>(0, f, k),
            Register::ECX => self.get_32::<C>(1, f, k),
            Register::EDX => self.get_32::<C>(2, f, k),
            Register::EBX => self.get_32::<C>(3, f, k),
            Register::ESP => self.get_32::<C>(4, f, k),
            Register::EBP => self.get_32::<C>(5, f, k),
            Register::ESI => self.get_32::<C>(6, f, k),
            Register::EDI => self.get_32::<C>(7, f, k),
            Register::R8D => self.get_32::<C>(8, f, k),
            Register::R9D => self.get_32::<C>(9, f, k),
            Register::R10D => self.get_32::<C>(10, f, k),
            Register::R11D => self.get_32::<C>(11, f, k),
            Register::R12D => self.get_32::<C>(12, f, k),
            Register::R13D => self.get_32::<C>(13, f, k),
            Register::R14D => self.get_32::<C>(14, f, k),
            Register::R15D => self.get_32::<C>(15, f, k),
            Register::RAX => self.gprs[0],
            Register::RCX => self.gprs[1],
            Register::RDX => self.gprs[2],
            Register::RBX => self.gprs[3],
            Register::RSP => self.gprs[4],
            Register::RBP => self.gprs[5],
            Register::RSI => self.gprs[6],
            Register::RDI => self.gprs[7],
            Register::R8 => self.gprs[8],
            Register::R9 => self.gprs[9],
            Register::R10 => self.gprs[10],
            Register::R11 => self.gprs[11],
            Register::R12 => self.gprs[12],
            Register::R13 => self.gprs[13],
            Register::R14 => self.gprs[14],
            Register::R15 => self.gprs[15],
            Register::EIP => todo!(),
            Register::RIP => todo!(),
            Register::ES => todo!(),
            Register::CS => todo!(),
            Register::SS => todo!(),
            Register::DS => todo!(),
            Register::FS => todo!(),
            Register::GS => todo!(),
            Register::XMM0 => todo!(),
            Register::XMM1 => todo!(),
            Register::XMM2 => todo!(),
            Register::XMM3 => todo!(),
            Register::XMM4 => todo!(),
            Register::XMM5 => todo!(),
            Register::XMM6 => todo!(),
            Register::XMM7 => todo!(),
            Register::XMM8 => todo!(),
            Register::XMM9 => todo!(),
            Register::XMM10 => todo!(),
            Register::XMM11 => todo!(),
            Register::XMM12 => todo!(),
            Register::XMM13 => todo!(),
            Register::XMM14 => todo!(),
            Register::XMM15 => todo!(),
            Register::XMM16 => todo!(),
            Register::XMM17 => todo!(),
            Register::XMM18 => todo!(),
            Register::XMM19 => todo!(),
            Register::XMM20 => todo!(),
            Register::XMM21 => todo!(),
            Register::XMM22 => todo!(),
            Register::XMM23 => todo!(),
            Register::XMM24 => todo!(),
            Register::XMM25 => todo!(),
            Register::XMM26 => todo!(),
            Register::XMM27 => todo!(),
            Register::XMM28 => todo!(),
            Register::XMM29 => todo!(),
            Register::XMM30 => todo!(),
            Register::XMM31 => todo!(),
            Register::YMM0 => todo!(),
            Register::YMM1 => todo!(),
            Register::YMM2 => todo!(),
            Register::YMM3 => todo!(),
            Register::YMM4 => todo!(),
            Register::YMM5 => todo!(),
            Register::YMM6 => todo!(),
            Register::YMM7 => todo!(),
            Register::YMM8 => todo!(),
            Register::YMM9 => todo!(),
            Register::YMM10 => todo!(),
            Register::YMM11 => todo!(),
            Register::YMM12 => todo!(),
            Register::YMM13 => todo!(),
            Register::YMM14 => todo!(),
            Register::YMM15 => todo!(),
            Register::YMM16 => todo!(),
            Register::YMM17 => todo!(),
            Register::YMM18 => todo!(),
            Register::YMM19 => todo!(),
            Register::YMM20 => todo!(),
            Register::YMM21 => todo!(),
            Register::YMM22 => todo!(),
            Register::YMM23 => todo!(),
            Register::YMM24 => todo!(),
            Register::YMM25 => todo!(),
            Register::YMM26 => todo!(),
            Register::YMM27 => todo!(),
            Register::YMM28 => todo!(),
            Register::YMM29 => todo!(),
            Register::YMM30 => todo!(),
            Register::YMM31 => todo!(),
            Register::ZMM0 => todo!(),
            Register::ZMM1 => todo!(),
            Register::ZMM2 => todo!(),
            Register::ZMM3 => todo!(),
            Register::ZMM4 => todo!(),
            Register::ZMM5 => todo!(),
            Register::ZMM6 => todo!(),
            Register::ZMM7 => todo!(),
            Register::ZMM8 => todo!(),
            Register::ZMM9 => todo!(),
            Register::ZMM10 => todo!(),
            Register::ZMM11 => todo!(),
            Register::ZMM12 => todo!(),
            Register::ZMM13 => todo!(),
            Register::ZMM14 => todo!(),
            Register::ZMM15 => todo!(),
            Register::ZMM16 => todo!(),
            Register::ZMM17 => todo!(),
            Register::ZMM18 => todo!(),
            Register::ZMM19 => todo!(),
            Register::ZMM20 => todo!(),
            Register::ZMM21 => todo!(),
            Register::ZMM22 => todo!(),
            Register::ZMM23 => todo!(),
            Register::ZMM24 => todo!(),
            Register::ZMM25 => todo!(),
            Register::ZMM26 => todo!(),
            Register::ZMM27 => todo!(),
            Register::ZMM28 => todo!(),
            Register::ZMM29 => todo!(),
            Register::ZMM30 => todo!(),
            Register::ZMM31 => todo!(),
            Register::K0 => todo!(),
            Register::K1 => todo!(),
            Register::K2 => todo!(),
            Register::K3 => todo!(),
            Register::K4 => todo!(),
            Register::K5 => todo!(),
            Register::K6 => todo!(),
            Register::K7 => todo!(),
            Register::BND0 => todo!(),
            Register::BND1 => todo!(),
            Register::BND2 => todo!(),
            Register::BND3 => todo!(),
            Register::CR0 => todo!(),
            Register::CR1 => todo!(),
            Register::CR2 => todo!(),
            Register::CR3 => todo!(),
            Register::CR4 => todo!(),
            Register::CR5 => todo!(),
            Register::CR6 => todo!(),
            Register::CR7 => todo!(),
            Register::CR8 => todo!(),
            Register::CR9 => todo!(),
            Register::CR10 => todo!(),
            Register::CR11 => todo!(),
            Register::CR12 => todo!(),
            Register::CR13 => todo!(),
            Register::CR14 => todo!(),
            Register::CR15 => todo!(),
            Register::DR0 => todo!(),
            Register::DR1 => todo!(),
            Register::DR2 => todo!(),
            Register::DR3 => todo!(),
            Register::DR4 => todo!(),
            Register::DR5 => todo!(),
            Register::DR6 => todo!(),
            Register::DR7 => todo!(),
            Register::DR8 => todo!(),
            Register::DR9 => todo!(),
            Register::DR10 => todo!(),
            Register::DR11 => todo!(),
            Register::DR12 => todo!(),
            Register::DR13 => todo!(),
            Register::DR14 => todo!(),
            Register::DR15 => todo!(),
            Register::ST0 => todo!(),
            Register::ST1 => todo!(),
            Register::ST2 => todo!(),
            Register::ST3 => todo!(),
            Register::ST4 => todo!(),
            Register::ST5 => todo!(),
            Register::ST6 => todo!(),
            Register::ST7 => todo!(),
            Register::MM0 => todo!(),
            Register::MM1 => todo!(),
            Register::MM2 => todo!(),
            Register::MM3 => todo!(),
            Register::MM4 => todo!(),
            Register::MM5 => todo!(),
            Register::MM6 => todo!(),
            Register::MM7 => todo!(),
            Register::TR0 => todo!(),
            Register::TR1 => todo!(),
            Register::TR2 => todo!(),
            Register::TR3 => todo!(),
            Register::TR4 => todo!(),
            Register::TR5 => todo!(),
            Register::TR6 => todo!(),
            Register::TR7 => todo!(),
            Register::TMM0 => todo!(),
            Register::TMM1 => todo!(),
            Register::TMM2 => todo!(),
            Register::TMM3 => todo!(),
            Register::TMM4 => todo!(),
            Register::TMM5 => todo!(),
            Register::TMM6 => todo!(),
            Register::TMM7 => todo!(),
            Register::DontUse0 => todo!(),
            Register::DontUseFA => todo!(),
            Register::DontUseFB => todo!(),
            Register::DontUseFC => todo!(),
            Register::DontUseFD => todo!(),
            Register::DontUseFE => todo!(),
            Register::DontUseFF => todo!(),
            _ => todo!(),
        }
    }
    pub fn set<C: Cfg>(&mut self, a: Register, f: &mut FunctionBody, k: Block, v: Value) {
        match a {
            Register::AH => {
                // let v = self.get_low::<C>(0,f,k,0xff00);
                let a = f.add_op(k, Operator::I32Const { value: 8 }, &[], &[Type::I32]);
                let v = f.add_op(k, cdef!(C => Shl), &[v, a], &[C::ty()]);
                self.set_low::<C>(0, f, k, v, 0xff00);
            }
            Register::CH => {
                let a = f.add_op(k, Operator::I32Const { value: 8 }, &[], &[Type::I32]);
                let v = f.add_op(k, cdef!(C => Shl), &[v, a], &[C::ty()]);
                self.set_low::<C>(1, f, k, v, 0xff00);
            }
            Register::DH => {
                let a = f.add_op(k, Operator::I32Const { value: 8 }, &[], &[Type::I32]);
                let v = f.add_op(k, cdef!(C => Shl), &[v, a], &[C::ty()]);
                self.set_low::<C>(2, f, k, v, 0xff00);
            }
            Register::BH => {
                let a = f.add_op(k, Operator::I32Const { value: 8 }, &[], &[Type::I32]);
                let v = f.add_op(k, cdef!(C => Shl), &[v, a], &[C::ty()]);
                self.set_low::<C>(3, f, k, v, 0xff00);
            }
            Register::AL => self.set_low::<C>(0, f, k, v, 0xff),
            Register::CL => self.set_low::<C>(1, f, k, v, 0xff),
            Register::DL => self.set_low::<C>(2, f, k, v, 0xff),
            Register::BL => self.set_low::<C>(3, f, k, v, 0xff),
            Register::SPL => self.set_low::<C>(4, f, k, v, 0xff),
            Register::BPL => self.set_low::<C>(5, f, k, v, 0xff),
            Register::SIL => self.set_low::<C>(6, f, k, v, 0xff),
            Register::DIL => self.set_low::<C>(7, f, k, v, 0xff),
            Register::R8L => self.set_low::<C>(8, f, k, v, 0xff),
            Register::R9L => self.set_low::<C>(9, f, k, v, 0xff),
            Register::R10L => self.set_low::<C>(10, f, k, v, 0xff),
            Register::R11L => self.set_low::<C>(11, f, k, v, 0xff),
            Register::R12L => self.set_low::<C>(12, f, k, v, 0xff),
            Register::R13L => self.set_low::<C>(13, f, k, v, 0xff),
            Register::R14L => self.set_low::<C>(14, f, k, v, 0xff),
            Register::R15L => self.set_low::<C>(15, f, k, v, 0xff),
            Register::AX => self.set_low::<C>(0, f, k, v, 0xffff),
            Register::CX => self.set_low::<C>(1, f, k, v, 0xffff),
            Register::DX => self.set_low::<C>(2, f, k, v, 0xffff),
            Register::BX => self.set_low::<C>(3, f, k, v, 0xffff),
            Register::SP => self.set_low::<C>(4, f, k, v, 0xffff),
            Register::BP => self.set_low::<C>(5, f, k, v, 0xffff),
            Register::SI => self.set_low::<C>(6, f, k, v, 0xffff),
            Register::DI => self.set_low::<C>(7, f, k, v, 0xffff),
            Register::R8W => self.set_low::<C>(8, f, k, v, 0xffff),
            Register::R9W => self.set_low::<C>(9, f, k, v, 0xffff),
            Register::R10W => self.set_low::<C>(10, f, k, v, 0xffff),
            Register::R11W => self.set_low::<C>(11, f, k, v, 0xffff),
            Register::R12W => self.set_low::<C>(12, f, k, v, 0xffff),
            Register::R13W => self.set_low::<C>(13, f, k, v, 0xffff),
            Register::R14W => self.set_low::<C>(14, f, k, v, 0xffff),
            Register::R15W => self.set_low::<C>(15, f, k, v, 0xffff),
            Register::EAX => self.set_32::<C>(0, f, k, v),
            Register::ECX => self.set_32::<C>(1, f, k, v),
            Register::EDX => self.set_32::<C>(2, f, k, v),
            Register::EBX => self.set_32::<C>(3, f, k, v),
            Register::ESP => self.set_32::<C>(4, f, k, v),
            Register::EBP => self.set_32::<C>(5, f, k, v),
            Register::ESI => self.set_32::<C>(6, f, k, v),
            Register::EDI => self.set_32::<C>(7, f, k, v),
            Register::R8D => self.set_32::<C>(8, f, k, v),
            Register::R9D => self.set_32::<C>(9, f, k, v),
            Register::R10D => self.set_32::<C>(10, f, k, v),
            Register::R11D => self.set_32::<C>(11, f, k, v),
            Register::R12D => self.set_32::<C>(12, f, k, v),
            Register::R13D => self.set_32::<C>(13, f, k, v),
            Register::R14D => self.set_32::<C>(14, f, k, v),
            Register::R15D => self.set_32::<C>(15, f, k, v),
            Register::RAX => self.gprs[0] = v,
            Register::RCX => self.gprs[1] = v,
            Register::RDX => self.gprs[2] = v,
            Register::RBX => self.gprs[3] = v,
            Register::RSP => self.gprs[4] = v,
            Register::RBP => self.gprs[5] = v,
            Register::RSI => self.gprs[6] = v,
            Register::RDI => self.gprs[7] = v,
            Register::R8 => self.gprs[8] = v,
            Register::R9 => self.gprs[9] = v,
            Register::R10 => self.gprs[10] = v,
            Register::R11 => self.gprs[11] = v,
            Register::R12 => self.gprs[12] = v,
            Register::R13 => self.gprs[13] = v,
            Register::R14 => self.gprs[14] = v,
            Register::R15 => self.gprs[15] = v,
            _ => {}
        }
    }
    pub fn poke_flags<C: Cfg>(&mut self, f: &mut FunctionBody, k: Block, v: Value) {
        let z = f.add_op(k, cdef!(C => Eqz), &[v], &[Type::I32]);
        let z = if C::MEMORY64 {
            f.add_op(k, Operator::I32WrapI64, &[z], &[Type::I32])
        } else {
            z
        };
        self.set_flag::<C>(f, k, 6, z);
        let s = if C::MEMORY64 {
            let w = f.add_op(k, C::const_64(0x7000000000000000), &[], &[C::ty()]);
            let w = f.add_op(k, cdef!(C => And), &[w, v], &[C::ty()]);
            let w = f.add_op(k, cdef!(C => Eqz), &[w], &[Type::I32]);
            f.add_op(k, Operator::I32Eqz, &[w], &[Type::I32])
        } else {
            let w = f.add_op(k, C::const_32(0x70000000), &[], &[C::ty()]);
            let w = f.add_op(k, cdef!(C => And), &[w, v], &[C::ty()]);
            w
        };
        self.set_flag::<C>(f, k, 7, s);
    }
}
impl TRegs for Regs {
    const N: usize = Regs::N;

    fn iter(&self) -> impl Iterator<Item = Value> {
        self.gprs.iter().cloned().chain(once(self.flags))
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Value> {
        self.gprs.iter_mut().chain(once(&mut self.flags))
    }
}
pub fn reg_size(r: Register) -> MemorySize {
    match r {
        Register::None => todo!(),
        Register::AL => MemorySize::UInt8,
        Register::CL => MemorySize::UInt8,
        Register::DL => MemorySize::UInt8,
        Register::BL => MemorySize::UInt8,
        Register::AH => MemorySize::UInt8,
        Register::CH => MemorySize::UInt8,
        Register::DH => MemorySize::UInt8,
        Register::BH => MemorySize::UInt8,
        Register::SPL => MemorySize::UInt8,
        Register::BPL => MemorySize::UInt8,
        Register::SIL => MemorySize::UInt8,
        Register::DIL => MemorySize::UInt8,
        Register::R8L => MemorySize::UInt8,
        Register::R9L => MemorySize::UInt8,
        Register::R10L => MemorySize::UInt8,
        Register::R11L => MemorySize::UInt8,
        Register::R12L => MemorySize::UInt8,
        Register::R13L => MemorySize::UInt8,
        Register::R14L => MemorySize::UInt8,
        Register::R15L => MemorySize::UInt8,
        Register::AX => MemorySize::UInt16,
        Register::CX => MemorySize::UInt16,
        Register::DX => MemorySize::UInt16,
        Register::BX => MemorySize::UInt16,
        Register::SP => MemorySize::UInt16,
        Register::BP => MemorySize::UInt16,
        Register::SI => MemorySize::UInt16,
        Register::DI => MemorySize::UInt16,
        Register::R8W => MemorySize::UInt16,
        Register::R9W => MemorySize::UInt16,
        Register::R10W => MemorySize::UInt16,
        Register::R11W => MemorySize::UInt16,
        Register::R12W => MemorySize::UInt16,
        Register::R13W => MemorySize::UInt16,
        Register::R14W => MemorySize::UInt16,
        Register::R15W => MemorySize::UInt16,
        Register::EAX => MemorySize::UInt32,
        Register::ECX => MemorySize::UInt32,
        Register::EDX => MemorySize::UInt32,
        Register::EBX => MemorySize::UInt32,
        Register::ESP => MemorySize::UInt32,
        Register::EBP => MemorySize::UInt32,
        Register::ESI => MemorySize::UInt32,
        Register::EDI => MemorySize::UInt32,
        Register::R8D => MemorySize::UInt32,
        Register::R9D => MemorySize::UInt32,
        Register::R10D => MemorySize::UInt32,
        Register::R11D => MemorySize::UInt32,
        Register::R12D => MemorySize::UInt32,
        Register::R13D => MemorySize::UInt32,
        Register::R14D => MemorySize::UInt32,
        Register::R15D => MemorySize::UInt32,
        Register::RAX => MemorySize::UInt64,
        Register::RCX => MemorySize::UInt64,
        Register::RDX => MemorySize::UInt64,
        Register::RBX => MemorySize::UInt64,
        Register::RSP => MemorySize::UInt64,
        Register::RBP => MemorySize::UInt64,
        Register::RSI => MemorySize::UInt64,
        Register::RDI => MemorySize::UInt64,
        Register::R8 => MemorySize::UInt64,
        Register::R9 => MemorySize::UInt64,
        Register::R10 => MemorySize::UInt64,
        Register::R11 => MemorySize::UInt64,
        Register::R12 => MemorySize::UInt64,
        Register::R13 => MemorySize::UInt64,
        Register::R14 => MemorySize::UInt64,
        Register::R15 => MemorySize::UInt64,
        Register::EIP => todo!(),
        Register::RIP => todo!(),
        Register::ES => todo!(),
        Register::CS => todo!(),
        Register::SS => todo!(),
        Register::DS => todo!(),
        Register::FS => todo!(),
        Register::GS => todo!(),
        Register::XMM0 => todo!(),
        Register::XMM1 => todo!(),
        Register::XMM2 => todo!(),
        Register::XMM3 => todo!(),
        Register::XMM4 => todo!(),
        Register::XMM5 => todo!(),
        Register::XMM6 => todo!(),
        Register::XMM7 => todo!(),
        Register::XMM8 => todo!(),
        Register::XMM9 => todo!(),
        Register::XMM10 => todo!(),
        Register::XMM11 => todo!(),
        Register::XMM12 => todo!(),
        Register::XMM13 => todo!(),
        Register::XMM14 => todo!(),
        Register::XMM15 => todo!(),
        Register::XMM16 => todo!(),
        Register::XMM17 => todo!(),
        Register::XMM18 => todo!(),
        Register::XMM19 => todo!(),
        Register::XMM20 => todo!(),
        Register::XMM21 => todo!(),
        Register::XMM22 => todo!(),
        Register::XMM23 => todo!(),
        Register::XMM24 => todo!(),
        Register::XMM25 => todo!(),
        Register::XMM26 => todo!(),
        Register::XMM27 => todo!(),
        Register::XMM28 => todo!(),
        Register::XMM29 => todo!(),
        Register::XMM30 => todo!(),
        Register::XMM31 => todo!(),
        Register::YMM0 => todo!(),
        Register::YMM1 => todo!(),
        Register::YMM2 => todo!(),
        Register::YMM3 => todo!(),
        Register::YMM4 => todo!(),
        Register::YMM5 => todo!(),
        Register::YMM6 => todo!(),
        Register::YMM7 => todo!(),
        Register::YMM8 => todo!(),
        Register::YMM9 => todo!(),
        Register::YMM10 => todo!(),
        Register::YMM11 => todo!(),
        Register::YMM12 => todo!(),
        Register::YMM13 => todo!(),
        Register::YMM14 => todo!(),
        Register::YMM15 => todo!(),
        Register::YMM16 => todo!(),
        Register::YMM17 => todo!(),
        Register::YMM18 => todo!(),
        Register::YMM19 => todo!(),
        Register::YMM20 => todo!(),
        Register::YMM21 => todo!(),
        Register::YMM22 => todo!(),
        Register::YMM23 => todo!(),
        Register::YMM24 => todo!(),
        Register::YMM25 => todo!(),
        Register::YMM26 => todo!(),
        Register::YMM27 => todo!(),
        Register::YMM28 => todo!(),
        Register::YMM29 => todo!(),
        Register::YMM30 => todo!(),
        Register::YMM31 => todo!(),
        Register::ZMM0 => todo!(),
        Register::ZMM1 => todo!(),
        Register::ZMM2 => todo!(),
        Register::ZMM3 => todo!(),
        Register::ZMM4 => todo!(),
        Register::ZMM5 => todo!(),
        Register::ZMM6 => todo!(),
        Register::ZMM7 => todo!(),
        Register::ZMM8 => todo!(),
        Register::ZMM9 => todo!(),
        Register::ZMM10 => todo!(),
        Register::ZMM11 => todo!(),
        Register::ZMM12 => todo!(),
        Register::ZMM13 => todo!(),
        Register::ZMM14 => todo!(),
        Register::ZMM15 => todo!(),
        Register::ZMM16 => todo!(),
        Register::ZMM17 => todo!(),
        Register::ZMM18 => todo!(),
        Register::ZMM19 => todo!(),
        Register::ZMM20 => todo!(),
        Register::ZMM21 => todo!(),
        Register::ZMM22 => todo!(),
        Register::ZMM23 => todo!(),
        Register::ZMM24 => todo!(),
        Register::ZMM25 => todo!(),
        Register::ZMM26 => todo!(),
        Register::ZMM27 => todo!(),
        Register::ZMM28 => todo!(),
        Register::ZMM29 => todo!(),
        Register::ZMM30 => todo!(),
        Register::ZMM31 => todo!(),
        Register::K0 => todo!(),
        Register::K1 => todo!(),
        Register::K2 => todo!(),
        Register::K3 => todo!(),
        Register::K4 => todo!(),
        Register::K5 => todo!(),
        Register::K6 => todo!(),
        Register::K7 => todo!(),
        Register::BND0 => todo!(),
        Register::BND1 => todo!(),
        Register::BND2 => todo!(),
        Register::BND3 => todo!(),
        Register::CR0 => todo!(),
        Register::CR1 => todo!(),
        Register::CR2 => todo!(),
        Register::CR3 => todo!(),
        Register::CR4 => todo!(),
        Register::CR5 => todo!(),
        Register::CR6 => todo!(),
        Register::CR7 => todo!(),
        Register::CR8 => todo!(),
        Register::CR9 => todo!(),
        Register::CR10 => todo!(),
        Register::CR11 => todo!(),
        Register::CR12 => todo!(),
        Register::CR13 => todo!(),
        Register::CR14 => todo!(),
        Register::CR15 => todo!(),
        Register::DR0 => todo!(),
        Register::DR1 => todo!(),
        Register::DR2 => todo!(),
        Register::DR3 => todo!(),
        Register::DR4 => todo!(),
        Register::DR5 => todo!(),
        Register::DR6 => todo!(),
        Register::DR7 => todo!(),
        Register::DR8 => todo!(),
        Register::DR9 => todo!(),
        Register::DR10 => todo!(),
        Register::DR11 => todo!(),
        Register::DR12 => todo!(),
        Register::DR13 => todo!(),
        Register::DR14 => todo!(),
        Register::DR15 => todo!(),
        Register::ST0 => todo!(),
        Register::ST1 => todo!(),
        Register::ST2 => todo!(),
        Register::ST3 => todo!(),
        Register::ST4 => todo!(),
        Register::ST5 => todo!(),
        Register::ST6 => todo!(),
        Register::ST7 => todo!(),
        Register::MM0 => todo!(),
        Register::MM1 => todo!(),
        Register::MM2 => todo!(),
        Register::MM3 => todo!(),
        Register::MM4 => todo!(),
        Register::MM5 => todo!(),
        Register::MM6 => todo!(),
        Register::MM7 => todo!(),
        Register::TR0 => todo!(),
        Register::TR1 => todo!(),
        Register::TR2 => todo!(),
        Register::TR3 => todo!(),
        Register::TR4 => todo!(),
        Register::TR5 => todo!(),
        Register::TR6 => todo!(),
        Register::TR7 => todo!(),
        Register::TMM0 => todo!(),
        Register::TMM1 => todo!(),
        Register::TMM2 => todo!(),
        Register::TMM3 => todo!(),
        Register::TMM4 => todo!(),
        Register::TMM5 => todo!(),
        Register::TMM6 => todo!(),
        Register::TMM7 => todo!(),
        Register::DontUse0 => todo!(),
        Register::DontUseFA => todo!(),
        Register::DontUseFB => todo!(),
        Register::DontUseFC => todo!(),
        Register::DontUseFD => todo!(),
        Register::DontUseFE => todo!(),
        Register::DontUseFF => todo!(),
        _ => todo!(),
    }
}
pub fn ctx(f: &FunctionBody) -> Vec<Value> {
    let ctx = f.blocks[f.entry].params[(Regs::N)..]
        .iter()
        .map(|a| a.1)
        .collect::<Vec<_>>();
    ctx
}
pub struct X86 {}
impl Arch for X86 {
    type Regs = Regs;

    fn go<C: Cfg>(
        f: &mut FunctionBody,
        entry: Block,
        code: &[u8],
        root_pc: u64,
        funcs: &Funcs,
        module: &mut Module,
    ) -> ArchRes {
        crate::go::<C>(f, entry, code, root_pc, funcs, module)
    }
}
pub fn go<C: Cfg>(
    f: &mut FunctionBody,
    entry: Block,
    code: &[u8],
    root_pc: u64,
    funcs: &Funcs,
    module: &mut Module,
) -> ArchRes {
    // let mut w = code
    //     .windows(4)
    //     .map(|w| u32::from_ne_bytes(w.try_into().unwrap()))
    //     .enumerate();
    let mut ic = iced_x86::Decoder::new(if C::MEMORY64 { 64 } else { 32 }, code, 0);
    let shim = f.add_block();
    let mut v: Vec<(Block, Regs)> = vec![];
    let mut back: BTreeMap<usize, usize> = BTreeMap::new();
    for idx in 0..(ic.max_position()) {
        let (mut k, mut r) = match back.get(&idx).cloned() {
            Some(a) => v[a].clone(),
            None => {
                let mut r = Regs {
                    gprs: f.blocks[entry].params[..16]
                        .iter()
                        .map(|a| a.1)
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap(),
                    flags: f.blocks[entry].params[16].1,
                };
                let mut k = f.add_block();
                (k, r)
            }
        };
        // let (idx, a) = w.next().unwrap();
        if let Ok(_) = ic.set_position(idx) {
            // if let Ok(i) = riscv_decode::decode(a) {
            k = process::<C>(
                f,
                &ic.decode(),
                &mut r,
                k,
                TryInto::<u64>::try_into(idx).unwrap().wrapping_add(root_pc),
                shim,
                funcs,
                module,
                code,
                root_pc,
                entry,
            );
        }
        // }
        v.push((k, r));
        back.insert(ic.position(), idx);
    }
    let pc = f.add_blockparam(shim, C::ty());
    let regs: [Value; Regs::N] = std::array::from_fn(|i| {
        let p = f.blocks[f.entry].params[i].0;
        f.add_blockparam(shim, p)
    });
    let inv_root = 0u64.wrapping_sub(root_pc);
    let pc_shifted = f.add_op(shim, C::const_64(inv_root), &[], &[C::ty()]);
    let pc_shifted = f.add_op(
        shim,
        if C::MEMORY64 {
            Operator::I64Add
        } else {
            Operator::I32Add
        },
        &[pc_shifted, pc],
        &[C::ty()],
    );
    let rb = f.add_block();
    f.set_terminator(
        shim,
        Terminator::Select {
            value: pc_shifted,
            targets: v
                .iter()
                .map(|(a, _)| BlockTarget {
                    block: *a,
                    args: regs.iter().cloned().collect(),
                })
                .collect(),
            default: BlockTarget {
                block: rb,
                args: vec![],
            },
        },
    );
    let c = Regs::ctx(f, entry);
    f.set_terminator(
        rb,
        Terminator::ReturnCall {
            func: funcs.deopt,
            args: vec![pc]
                .into_iter()
                .chain(regs.iter().cloned())
                .chain(c.into_iter())
                .collect(),
        },
    );
    return ArchRes {
        insts: v
            .into_iter()
            .map(|a| a.0)
            .enumerate()
            .map(|(a, b)| (root_pc.wrapping_add(a as u64), b))
            .collect(),
        shim,
    };
}
pub fn process<C: Cfg>(
    f: &mut FunctionBody,
    i: &Instruction,
    regs: &mut Regs,
    mut k: Block,
    pc: u64,
    shim: Block,
    funcs: &Funcs,
    module: &mut Module,
    code: &[u8],
    root_pc: u64,
    entry: Block,
) -> Block {
    let new = f.add_block();
    let mut t = BlockTarget {
        block: new,
        args: vec![],
    };
    for r in regs.iter_mut() {
        let ty = f.values[*r].ty(&f.type_pool).unwrap();
        t.args.push(replace(r, f.add_blockparam(new, ty)));
    }
    if f.blocks[k].terminator == Terminator::None {
        f.set_terminator(k, waffle::Terminator::Br { target: t });
    }
    let mut k = new;
    fn loader<C: Cfg>(funcs: &Funcs, i: &Instruction) -> Operator {
        match i.memory_size() {
            MemorySize::Int8 => cdef!(C => Load8S{memory: MemoryArg {
                align: 0,
                offset: 0,
                memory: funcs.memory,
            }}),
            MemorySize::UInt8 => cdef!(C => Load8U{memory: MemoryArg {
                align: 0,
                offset: 0,
                memory: funcs.memory,
            }}),
            MemorySize::Int16 => cdef!(C => Load16S{memory: MemoryArg {
                align: 0,
                offset: 0,
                memory: funcs.memory,
            }}),
            MemorySize::UInt16 => cdef!(C => Load16U{memory: MemoryArg {
                align: 0,
                offset: 0,
                memory: funcs.memory,
            }}),
            MemorySize::Int32 => {
                if C::MEMORY64 {
                    Operator::I64Load32S {
                        memory: MemoryArg {
                            align: 0,
                            offset: 0,
                            memory: funcs.memory,
                        },
                    }
                } else {
                    cdef!(C => Load{memory: MemoryArg {
                        align: 0,
                        offset: 0,
                        memory: funcs.memory,
                    }})
                }
            }
            MemorySize::UInt32 => {
                if C::MEMORY64 {
                    Operator::I64Load32U {
                        memory: MemoryArg {
                            align: 0,
                            offset: 0,
                            memory: funcs.memory,
                        },
                    }
                } else {
                    cdef!(C => Load{memory: MemoryArg {
                        align: 0,
                        offset: 0,
                        memory: funcs.memory,
                    }})
                }
            }
            MemorySize::Int64 => cdef!(C => Load{memory: MemoryArg {
                align: 0,
                offset: 0,
                memory: funcs.memory,
            }}),
            MemorySize::UInt64 => cdef!(C => Load{memory: MemoryArg {
                align: 0,
                offset: 0,
                memory: funcs.memory,
            }}),
            _ => todo!(),
        }
    }
    fn storer<C: Cfg>(funcs: &Funcs, i: &Instruction) -> Operator {
        match i.memory_size() {
            MemorySize::Int8 => cdef!(C => Store8{memory: MemoryArg {
                align: 0,
                offset: 0,
                memory: funcs.memory,
            }}),
            MemorySize::UInt8 => cdef!(C => Store8{memory: MemoryArg {
                align: 0,
                offset: 0,
                memory: funcs.memory,
            }}),
            MemorySize::Int16 => cdef!(C => Store16{memory: MemoryArg {
                align: 0,
                offset: 0,
                memory: funcs.memory,
            }}),
            MemorySize::UInt16 => cdef!(C => Store16{memory: MemoryArg {
                align: 0,
                offset: 0,
                memory: funcs.memory,
            }}),
            MemorySize::Int32 => {
                if C::MEMORY64 {
                    Operator::I64Store32 {
                        memory: MemoryArg {
                            align: 0,
                            offset: 0,
                            memory: funcs.memory,
                        },
                    }
                } else {
                    cdef!(C => Store{memory: MemoryArg {
                        align: 0,
                        offset: 0,
                        memory: funcs.memory,
                    }})
                }
            }
            MemorySize::UInt32 => {
                if C::MEMORY64 {
                    Operator::I64Store32 {
                        memory: MemoryArg {
                            align: 0,
                            offset: 0,
                            memory: funcs.memory,
                        },
                    }
                } else {
                    cdef!(C => Store{memory: MemoryArg {
                        align: 0,
                        offset: 0,
                        memory: funcs.memory,
                    }})
                }
            }
            MemorySize::Int64 => cdef!(C => Store{memory: MemoryArg {
                align: 0,
                offset: 0,
                memory: funcs.memory,
            }}),
            MemorySize::UInt64 => cdef!(C => Store{memory: MemoryArg {
                align: 0,
                offset: 0,
                memory: funcs.memory,
            }}),
            _ => todo!(),
        }
    }
    macro_rules! fetch {
        ({regs: $regs:expr, func: $f:expr, k: $k:expr, i: $i:expr, funcs: $funcs:expr} $n:ident) => {
            paste::paste! {
                match $i.[<$n _kind>](){
                    OpKind::Register => {
                        let l = $regs.get::<C>($i.[<$n _register>](),$f,$k);
                        l
                    },
                    OpKind::Memory => {
                        let base = $regs.get::<C>($i.memory_base(),$f,$k);
                        let index = $regs.get::<C>($i.memory_index(),$f,$k);
                        let sc = $f.add_op($k,C::const_32($i.memory_index_scale()),&[],&[C::ty()]);
                        let index = $f.add_op($k,cdef!(C => Mul),&[index,sc],&[C::ty()]);
                        let disp = $f.add_op($k,C::const_64($i.memory_displacement64()),&[],&[C::ty()]);
                        let index = $f.add_op($k,cdef!(C => Add),&[index,disp],&[C::ty()]);
                        let index = $f.add_op($k,cdef!(C => Add),&[index,base],&[C::ty()]);
                        let op = match $funcs{
                            funcs => loader::<C>(funcs,$i)
                        };
                        let v;
                        ($k,v) = talc_common::load::<Regs,C>(index,$f,$regs,$k,op,funcs,module,entry,code,root_pc);
                        v
                    }
                    OpKind::Immediate64 => {
                        $f.add_op($k,C::const_64($i.immediate64()),&[],&[C::ty()])
                    },
                    OpKind::Immediate32 => {
                        $f.add_op($k,C::const_32($i.immediate32()),&[],&[C::ty()])
                    },
                    OpKind::Immediate16 => {
                        $f.add_op($k,C::const_64($i.immediate16() as u64),&[],&[C::ty()])
                    },
                    OpKind::Immediate8 => {
                        $f.add_op($k,C::const_64($i.immediate8() as u64),&[],&[C::ty()])
                    },
                    OpKind::Immediate8to16 => {
                        $f.add_op($k,C::const_64($i.immediate8to16() as u16 as u64),&[],&[C::ty()])
                    },
                    OpKind::Immediate8to32 => {
                        $f.add_op($k,C::const_64($i.immediate8to32() as u32 as u64),&[],&[C::ty()])
                    },
                    OpKind::Immediate8to64 => {
                        $f.add_op($k,C::const_64($i.immediate8to64() as u64),&[],&[C::ty()])
                    },
                    OpKind::Immediate32to64 => {
                        $f.add_op($k,C::const_64($i.immediate32to64() as u32 as u64),&[],&[C::ty()])
                    },
                    _ => todo!()
                }
            }
        };
    }
    macro_rules! size {
        ({regs: $regs:expr, func: $f:expr, k: $k:expr, i: $i:expr, funcs: $funcs:expr} $n:ident) => {
            paste::paste! {
                match $i.[<$n _kind>](){
                    OpKind::Register => {
                        reg_size($i.[<$n _register>]())
                    },
                    OpKind::Memory => {
                        $i.memory_size()
                    }
                    OpKind::Immediate64 => {
                        MemorySize::UInt64
                    },
                    OpKind::Immediate32 => {
                        MemorySize::UInt32
                    },
                    OpKind::Immediate16 => {
                        MemorySize::UInt16
                    },
                    OpKind::Immediate8 => {
                        MemorySize::UInt8
                    },
                    OpKind::Immediate8to16 => {
                        MemorySize::Int16
                    },
                    OpKind::Immediate8to32 => {
                        MemorySize::Int32
                    },
                    OpKind::Immediate8to64 => {
                        MemorySize::Int64
                    },
                    OpKind::Immediate32to64 => {
                        MemorySize::Int64
                    },
                    _ => todo!()
                }
            }
        };
    }
    macro_rules! stor {
        ({regs: $regs:expr, func: $f:expr, k: $k:expr, i: $i:expr, v: $v:expr, funcs: $funcs:expr} $n:ident) => {
            paste::paste! {
                match $i.[<$n _kind>](){
                    OpKind::Register => {
                        $regs.set::<C>($i.[<$n _register>](),$f,$k,$v);
                        $k
                    },
                    OpKind::Memory => {
                        let base = $regs.get::<C>($i.memory_base(),$f,$k);
                        let index = $regs.get::<C>($i.memory_index(),$f,$k);
                        let sc = $f.add_op($k,C::const_32($i.memory_index_scale()),&[],&[C::ty()]);
                        let index = $f.add_op($k,cdef!(C => Mul),&[index,sc],&[C::ty()]);
                        let disp = $f.add_op($k,C::const_64($i.memory_displacement64()),&[],&[C::ty()]);
                        let index = $f.add_op($k,cdef!(C => Add),&[index,disp],&[C::ty()]);
                        let index = $f.add_op($k,cdef!(C => Add),&[index,base],&[C::ty()]);
                        let op = match $funcs{
                            funcs => storer::<C>(funcs,$i)
                        };
                        talc_common::store::<Regs,C>($f,index,$v,$k,op,funcs,module,entry);
                        $k
                    },
                    _ => todo!()
                }
            }
        };
    }
    macro_rules! opcodes {
        ({
            func: $f:expr,
            k: $k:ident,
            regs: $regs:expr,
            pc: $pc:expr,
            shim: $shim:expr,
            i: $i:expr,
            funcs: $funcs:expr,
            module: $module:expr
        } [arith: $($ariths:ident over($over:expr)),*] [jumps: $($jump:ident flag($flag:expr)),*]) => {
            paste::paste!{
            match $i {
                i => match i.mnemonic() {
                    $(iced_x86::Mnemonic::$ariths => {
                        let a = fetch!({regs: $regs, func: $f, k: $k, i: i, funcs: $funcs} op0);
                        let b = fetch!({regs: $regs, func: $f, k: $k, i: i, funcs: $funcs} op1);
                        let v = $f.add_op($k,cdef!(C => $ariths),&[a,b],&[C::ty()]);
                        $regs.poke_flags::<C>($f,$k,v);
                        let mut carry = true;
                        let mut over = $over;
                        for i in [0,11]{
                            let over = over($f,$k,a,b,v,std::mem::replace(&mut carry,false));
                            $regs.set_flag::<C>($f,$k,i,over);
                        }
                        stor!({regs: $regs, func: $f, k: $k, i: i, v: v, funcs: $funcs} op0)
                    }),*,
                    iced_x86::Mnemonic::Cmp => {
                        let a = fetch!({regs: $regs, func: $f, k: $k, i: i, funcs: $funcs} op0);
                        let b = fetch!({regs: $regs, func: $f, k: $k, i: i, funcs: $funcs} op1);
                        let v = $f.add_op($k,cdef!(C => Sub),&[a,b],&[C::ty()]);
                        $regs.poke_flags::<C>($f,$k,v);
                        let mut carry = true;
                        for i in [0,11]{
                            let over = f.add_op(k,if carry{
                                cdef!(C => LeU)
                            }else{
                                cdef!(C => LeS)
                            },&[v,a],&[Type::I32]);
                            carry = false;
                            $regs.set_flag::<C>($f,$k,i,over);
                        }
                        $k
                    }
                    iced_x86::Mnemonic::Mov => {
                        let b = fetch!({regs: $regs, func: $f, k: $k, i: i, funcs: $funcs} op1);
                        let v = b;
                        stor!({regs: $regs, func: $f, k: $k, i: i, v: v, funcs: $funcs} op0)
                    }
                    $(iced_x86::Mnemonic::$jump => {
                        let flag = $flag;
                        let a = fetch!({regs: $regs, func: $f, k: $k, i: i, funcs: $funcs} op0);
                        let flag = flag($f,$k,$regs);
                        let b = $f.add_block();
                        $f.set_terminator($k,Terminator::CondBr{
                            cond: flag,
                            if_true: BlockTarget{
                                block: shim,
                                args: vec![a]
                                .into_iter().chain($regs.iter()).collect()
                            },
                            if_false: BlockTarget{
                                block: b,
                                args: vec![]
                            }
                        });
                        b
                    }),*,
                    iced_x86::Mnemonic::Lea => {
                        let base = $regs.get::<C>($i.memory_base(),$f,$k);
                        let index = $regs.get::<C>($i.memory_index(),$f,$k);
                        let sc = $f.add_op($k,C::const_32($i.memory_index_scale()),&[],&[C::ty()]);
                        let index = $f.add_op($k,cdef!(C => Mul),&[index,sc],&[C::ty()]);
                        let disp = $f.add_op($k,C::const_64($i.memory_displacement64()),&[],&[C::ty()]);
                        let index = $f.add_op($k,cdef!(C => Add),&[index,disp],&[C::ty()]);
                        let index = $f.add_op($k,cdef!(C => Add),&[index,base],&[C::ty()]);
                        let v = index;
                        stor!({regs: $regs, func: $f, k: $k, i: i, v: v, funcs: $funcs} op0)
                    },
                    iced_x86::Mnemonic::Mul => {
                        let b = fetch!({regs: $regs, func: $f, k: $k, i: i, funcs: $funcs} op0);
                        let size = size!({regs: $regs, func: $f, k: $k, i: i, funcs: $funcs} op0);
                        match size.size(){
                            1 => {
                                let a = $regs.get::<C>(Register::AL,$f,$k);
                                let v = $f.add_op($k,cdef!(C => Mul),&[a,b],&[C::ty()]);
                                $regs.set::<C>(Register::AX,$f,$k,v);
                                let w = f.add_op($k,Operator::I32Const {value: 8},&[],&[Type::I32]);
                                let v = $f.add_op($k,cdef!(C => ShrU),&[v,w],&[C::ty()]);
                                let z = f.add_op(k, cdef!(C => Eqz), &[v], &[Type::I32]);
                                let z = f.add_op(k,Operator::I32Eqz,&[v],&[Type::I32]);
                                for i in [11,0]{
                                    $regs.set_flag::<C>($f,$k,i,z);
                                }
                            }
                            2 => {
                                let a = $regs.get::<C>(Register::AX,$f,$k);
                                let v = $f.add_op($k,cdef!(C => Mul),&[a,b],&[C::ty()]);
                                $regs.set::<C>(Register::AX,$f,$k,v);
                                let w = f.add_op($k,Operator::I32Const {value: 8},&[],&[Type::I32]);
                                let v = $f.add_op($k,cdef!(C => ShrU),&[v,w],&[C::ty()]);
                                $regs.set::<C>(Register::DX,$f,$k,v);
                                let z = f.add_op(k, cdef!(C => Eqz), &[v], &[Type::I32]);
                                let z = f.add_op(k,Operator::I32Eqz,&[v],&[Type::I32]);
                                for i in [11,0]{
                                    $regs.set_flag::<C>($f,$k,i,z);
                                }
                            }
                            4 if C::MEMORY64 => {
                                let a = $regs.get::<C>(Register::EAX,$f,$k);
                                let v = $f.add_op($k,cdef!(C => Mul),&[a,b],&[C::ty()]);
                                $regs.set::<C>(Register::EAX,$f,$k,v);
                                let w = f.add_op($k,Operator::I32Const {value: 16},&[],&[Type::I32]);
                                let v = $f.add_op($k,cdef!(C => ShrU),&[v,w],&[C::ty()]);
                                $regs.set::<C>(Register::EDX,$f,$k,v);
                                let z = f.add_op(k, cdef!(C => Eqz), &[v], &[Type::I32]);
                                let z = f.add_op(k,Operator::I32Eqz,&[v],&[Type::I32]);
                                for i in [11,0]{
                                    $regs.set_flag::<C>($f,$k,i,z);
                                }
                            }
                            n if if C::MEMORY64{
                                n == 8
                            }else{
                                n == 4
                            } => {
                                let a = $regs.get::<C>(Register::RAX,$f,$k);
                                let (w,v) = C::mul2($f,$k,a,b);
                                $regs.set::<C>(Register::RAX,$f,$k,w);
                                let w = f.add_op($k,Operator::I32Const {value: 2u32.pow(n as u32)},&[],&[Type::I32]);
                                let v = $f.add_op($k,cdef!(C => ShrU),&[v,w],&[C::ty()]);
                                $regs.set::<C>(Register::RDX,$f,$k,v);
                                let z = f.add_op(k, cdef!(C => Eqz), &[v], &[Type::I32]);
                                let z = f.add_op(k,Operator::I32Eqz,&[v],&[Type::I32]);
                                for i in [11,0]{
                                    $regs.set_flag::<C>($f,$k,i,z);
                                }
                            }
                            _ => todo!()
                        };
                        $k
                    },
                    _ => todo!(),
                },
            }
        }
        };
    }
    return opcodes!({func: f, k: k, regs: regs, pc: pc, shim: shim, i: i, funcs: funcs, module: module} [arith:
    Add over(|f: &mut FunctionBody,k,a,b,v,carry|{
        f.add_op(k,if carry{
            cdef!(C => GeU)
        }else{
            cdef!(C => GeS)
        },&[v,a],&[Type::I32])
    }), Sub over(|f: &mut FunctionBody,k,a,b,v,carry|{
        f.add_op(k,if carry{
            cdef!(C => LeU)
        }else{
            cdef!(C => LeS)
        },&[v,a],&[Type::I32])
    }), And over(|f: &mut FunctionBody,k,a,b,v,carry|{
        f.add_op(k,Operator::I32Const {value: 0},&[v,a],&[Type::I32])
    }), Or over(|f: &mut FunctionBody,k,a,b,v,carry|{
        f.add_op(k,Operator::I32Const {value: 0},&[v,a],&[Type::I32])
    }), Xor over(|f: &mut FunctionBody,k,a,b,v,carry|{
        f.add_op(k,Operator::I32Const {value: 0},&[v,a],&[Type::I32])
    })] [jumps:
    Jo flag(|f: &mut FunctionBody,k,regs: &mut Regs|{
        regs.get_flag::<C>(f,k,11)
    }),
    Js flag(|f: &mut FunctionBody,k,regs: &mut Regs|{
        regs.get_flag::<C>(f,k,7)
    }),
    Jno flag(|f: &mut FunctionBody,k,regs: &mut Regs|{
        let v = regs.get_flag::<C>(f,k,11);
        f.add_op(k,Operator::I32Eqz,&[v],&[Type::I32])
    }),
    Jns flag(|f: &mut FunctionBody,k,regs: &mut Regs|{
        let v = regs.get_flag::<C>(f,k,7);
        f.add_op(k,Operator::I32Eqz,&[v],&[Type::I32])
    }),
    Je flag(|f: &mut FunctionBody,k,regs: &mut Regs|{
        regs.get_flag::<C>(f,k,6)
    }),
    Jne flag(|f: &mut FunctionBody,k,regs: &mut Regs|{
        let v = regs.get_flag::<C>(f,k,6);
        f.add_op(k,Operator::I32Eqz,&[v],&[Type::I32])
    }),
    Jb flag(|f: &mut FunctionBody,k,regs: &mut Regs|{
        regs.get_flag::<C>(f,k,0)
    }),
    Jae flag(|f: &mut FunctionBody,k,regs: &mut Regs|{
        let v = regs.get_flag::<C>(f,k,0);
        f.add_op(k,Operator::I32Eqz,&[v],&[Type::I32])
    }),
    Jbe flag(|f: &mut FunctionBody,k,regs: &mut Regs|{
        let a = regs.get_flag::<C>(f,k,0);
        let b = regs.get_flag::<C>(f,k,6);
        f.add_op(k,Operator::I32Or,&[a,b],&[Type::I32])
    }),
    Ja flag(|f: &mut FunctionBody,k,regs: &mut Regs|{
        let a = regs.get_flag::<C>(f,k,0);
        let b = regs.get_flag::<C>(f,k,6);
        let v = f.add_op(k,Operator::I32Or,&[a,b],&[Type::I32]);
        f.add_op(k,Operator::I32Eqz,&[v],&[Type::I32])
    }),
    Jl flag(|f: &mut FunctionBody,k,regs: &mut Regs|{
        let a = regs.get_flag::<C>(f,k,7);
        let b = regs.get_flag::<C>(f,k,11);
        f.add_op(k,Operator::I32Ne,&[a,b],&[Type::I32])
    }),
    Jge flag(|f: &mut FunctionBody,k,regs: &mut Regs|{
        let a = regs.get_flag::<C>(f,k,7);
        let b = regs.get_flag::<C>(f,k,11);
        f.add_op(k,Operator::I32Eq,&[a,b],&[Type::I32])
    }),
    Jle flag(|f: &mut FunctionBody,k,regs: &mut Regs|{
        let a = regs.get_flag::<C>(f,k,7);
        let b = regs.get_flag::<C>(f,k,11);
        let a = f.add_op(k,Operator::I32Ne,&[a,b],&[Type::I32]);
        let b = regs.get_flag::<C>(f,k,6);
        f.add_op(k,Operator::I32Or,&[a,b],&[Type::I32])
    }),
    Jg flag(|f: &mut FunctionBody,k,regs: &mut Regs|{
        let a = regs.get_flag::<C>(f,k,7);
        let b = regs.get_flag::<C>(f,k,11);
        let a = f.add_op(k,Operator::I32Ne,&[a,b],&[Type::I32]);
        let b = regs.get_flag::<C>(f,k,6);
        let v = f.add_op(k,Operator::I32Or,&[a,b],&[Type::I32]);
        f.add_op(k,Operator::I32Eqz,&[v],&[Type::I32])
    })
    ]);
}
