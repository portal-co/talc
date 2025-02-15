use std::{collections::BTreeMap, mem::replace};

use disarm64::{decoder::{Mnemonic, Operation}, Opcode};
// use riscv_decode::{
//     types::{BType, IType, RType, SType, ShiftType},
//     Instruction,
// };
use waffle::{
    Block, BlockTarget, Func, FunctionBody, Memory, MemoryArg, Module, Operator, SignatureData,
    Terminator, Type, Value,
};
use waffle_ast::results_ref_2;

use talc_common::*;

#[derive(Clone)]
pub struct Regs {
    pub regs: [Value; 31],
    // pub csrs: [Value; 4096],
}

impl Regs {
    pub const N: usize = 31;
    pub fn reg<C: Cfg>(&self, f: &mut FunctionBody, b: u8, k: Block) -> Value {
        if b == 0 {
            f.add_op(k, C::const_32(0), &[], &[C::ty()])
        } else {
            self.regs[(b - 1) as usize]
        }
    }
    pub fn put_reg(&mut self, b: u8, v: Value) {
        if b == 0 {
        } else {
            self.regs[(b - 1) as usize] = v
        }
    }
}
impl TRegs for Regs {
    const N: usize = Regs::N;

    fn iter(&self) -> impl Iterator<Item = Value> {
        self.regs.iter().cloned()
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Value> {
        self.regs.iter_mut()
    }
}

pub struct AArch64 {}
impl Arch for AArch64 {
    type Regs = Regs;

    fn go<C: Cfg, H: Hook<Regs>>(
        f: &mut FunctionBody,
        entry: Block,
        code: InputRef<'_>,
        root_pc: u64,
        funcs: &Funcs,
        module: &mut Module,
        hook: &mut H,
    ) -> ArchRes {
        crate::go::<C, H>(f, entry, code, root_pc, funcs, module, hook)
    }
}
pub fn go<C: Cfg, H: Hook<Regs>>(
    f: &mut FunctionBody,
    entry: Block,
    code: InputRef<'_>,
    root_pc: u64,
    funcs: &Funcs,
    module: &mut Module,
    hook: &mut H,
) -> ArchRes {
    // let code = hook.update_code::<C>(code);
    // let code = code.as_ref();
    let mut w = code
        .code
        .windows(4)
        .map(|w| u32::from_ne_bytes(w.try_into().unwrap()))
        .enumerate();
    let shim = f.add_block();
    let mut v = vec![];
    for _ in 0..4 {
        let mut r = Regs {
            regs: f.blocks[entry].params[..31]
                .iter()
                .map(|a| a.1)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            // csrs: f.blocks[f.entry].params[32..(31 + 4096)]
            //     .iter()
            //     .map(|a| a.1)
            //     .collect::<Vec<_>>()
            //     .try_into()
            //     .unwrap(),
        };
        let mut k = f.add_block();
        let (idx, a) = w.next().unwrap();
        if let Some(i) = disarm64::decoder_full::decode(a) {
            k = process::<C, H>(
                f,
                &i,
                &mut r,
                k,
                TryInto::<u64>::try_into(idx).unwrap().wrapping_add(root_pc),
                shim,
                entry,
                funcs,
                module,
                hook,
                code,
                root_pc,
            );
        }
        v.push((k, r));
    }
    for (idx, a) in w {
        let (k, mut regs) = v[idx - 4].clone();
        let k = match disarm64::decoder_full::decode(a) {
            Some(i) => process::<C, H>(
                f,
                &i,
                &mut regs,
                k,
                TryInto::<u64>::try_into(idx).unwrap().wrapping_add(root_pc),
                shim,
                entry,
                funcs,
                module,
                hook,
                code,
                root_pc,
            ),
            None => f.add_block(),
        };
        v.push((k, regs));
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
                .enumerate()
                .map(|(i, b)| {
                    let v = code.x[i..][..4].iter().all(|a| *a);
                    if v {
                        b
                    } else {
                        BlockTarget {
                            block: rb,
                            args: vec![],
                        }
                    }
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
                .chain(c)
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
pub fn process<C: Cfg, H: Hook<Regs>>(
    f: &mut FunctionBody,
    i: &Opcode,
    regs: &mut Regs,
    k: Block,
    pc: u64,
    shim: Block,
    entry: Block,
    funcs: &Funcs,
    module: &mut Module,
    hook: &mut H,
    code: InputRef<'_>,
    root_pc: u64,
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
    // return new;
    let mut k = new;
    k = hook.hook::<C>(
        f,
        k,
        regs,
        pc,
        funcs,
        module,
        code,
        pc.wrapping_sub(root_pc).try_into().unwrap(),
    );
    return i.mnemonic.process::<C, H>(
        f,
        &i.operation,
        regs,
        k,
        pc,
        shim,
        entry,
        funcs,
        module,
        hook,
        code,
        root_pc,
    );
}
pub trait AArch64Mnemonic<O> {
    fn process<C: Cfg, H: Hook<Regs>>(
        &self,
        f: &mut FunctionBody,
        i: &O,
        regs: &mut Regs,
        k: Block,
        pc: u64,
        shim: Block,
        entry: Block,
        funcs: &Funcs,
        module: &mut Module,
        hook: &mut H,
        code: InputRef<'_>,
        root_pc: u64,
    ) -> Block;
}
impl AArch64Mnemonic<Operation> for Mnemonic{
    fn process<C: Cfg, H: Hook<Regs>>(
        &self,
        f: &mut FunctionBody,
        i: &Operation,
        regs: &mut Regs,
        k: Block,
        pc: u64,
        shim: Block,
        entry: Block,
        funcs: &Funcs,
        module: &mut Module,
        hook: &mut H,
        code: InputRef<'_>,
        root_pc: u64,
    ) -> Block {
        match self{
            _ => {
                f.add_block()
            }
        }
    }
}