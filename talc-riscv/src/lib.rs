use std::{
    array,
    collections::BTreeMap,
    mem::{replace, swap},
};

use riscv_decode::{
    types::{BType, IType, RType, SType, ShiftType},
    Instruction,
};
use waffle::{
    Block, BlockTarget, Func, FunctionBody, Memory, MemoryArg, Module, Operator, SignatureData,
    Terminator, Type, Value,
};
use waffle_ast::results_ref_2;

use talc_common::*;

#[derive(Clone)]
pub struct Regs<const CSRS: usize = 4096> {
    pub regs: [Value; 31],
    pub csrs: [Value; CSRS],
}

impl<const CSRS: usize> Regs<CSRS> {
    pub const N: usize = 31 + CSRS;
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
impl<const CSRS: usize> TRegs for Regs<CSRS> {
    const N: usize = Regs::<CSRS>::N;

    fn iter(&self) -> impl Iterator<Item = Value> {
        self.regs.iter().chain(self.csrs.iter()).cloned()
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Value> {
        self.regs.iter_mut().chain(self.csrs.iter_mut())
    }
}
// pub fn ctx(f: &FunctionBody) -> Vec<Value> {
//     let ctx = f.blocks[f.entry].params[(Regs::N)..]
//         .iter()
//         .map(|a| a.1)
//         .collect::<Vec<_>>();
//     ctx
// }
pub fn imm<const CSRS: usize, C: Cfg>(
    i: &IType,
    f: &mut FunctionBody,
    regs: &mut Regs<CSRS>,
    mut k: Block,
    op: Operator,
    load: bool,
    funcs: &Funcs,
    module: &Module,
    entry: Block,
    code: InputRef<'_>,
    root_pc: u64,
    mut bits: impl FnMut(usize) -> Operator,
) -> Block {
    let w = regs.reg::<C>(f, i.rs1() as u8, k);
    if load {
        let (n, v) = talc_common::load::<Regs<CSRS>, C>(
            w, f, regs, k, op, funcs, module, entry, code, root_pc, bits,
        );
        regs.put_reg(i.rd() as u8, v);
        return n;
    }
    let n = f.add_block();
    let v = f.add_op(k, C::const_32(i.imm()), &[], &[C::ty()]);
    // if load {
    //     let k2 = f.add_block();
    //     let b = f.add_op(k, C::const_64(root_pc), &[], &[C::ty()]);
    //     let b = f.add_op(k, cdef!(C => Sub), &[w, b], &[C::ty()]);
    //     let ts = code
    //         .iter()
    //         .enumerate()
    //         .map(|a| bits(a.0))
    //         .map(|o| {
    //             let l = f.add_block();
    //             let v = f.add_op(l, o, &[], &[C::ty()]);
    //             f.set_terminator(
    //                 l,
    //                 Terminator::Br {
    //                     target: BlockTarget {
    //                         block: n,
    //                         args: vec![v],
    //                     },
    //                 },
    //             );
    //             BlockTarget {
    //                 block: l,
    //                 args: vec![],
    //             }
    //         })
    //         .collect();
    //     f.set_terminator(
    //         k,
    //         Terminator::Select {
    //             value: b,
    //             targets: ts,
    //             default: BlockTarget {
    //                 block: k2,
    //                 args: vec![],
    //             },
    //         },
    //     );
    //     k = k2;
    // };
    // let (w, d) = if load {
    //     let mut ctx = Regs::ctx(f, entry).collect::<Vec<_>>();
    //     ctx.push(w);
    //     let SignatureData::Func { params, returns } =
    //         &module.signatures[module.funcs[funcs.resolve].sig()]
    //     else {
    //         todo!()
    //     };
    //     let r = f.add_op(
    //         k,
    //         Operator::Call {
    //             function_index: funcs.resolve,
    //         },
    //         &ctx,
    //         &returns,
    //     );
    //     let mut r = results_ref_2(f, r);
    //     let w = r.pop().unwrap();
    //     (w, Some(r))
    // } else {
    //     (w, None)
    // };
    let r: &[Value] = &[w, v];
    let v = f.add_op(k, op, r, &[C::ty()]);
    // put_reg(regs, i.rd() as u8, v);

    // if let Some(d) = d {
    //     f.add_op(
    //         k,
    //         Operator::Call {
    //             function_index: funcs.finalize,
    //         },
    //         &d,
    //         &[],
    //     );
    // }
    f.set_terminator(
        k,
        Terminator::Br {
            target: BlockTarget {
                block: n,
                args: vec![v],
            },
        },
    );
    let v = f.add_blockparam(n, C::ty());
    regs.put_reg(i.rd() as u8, v);
    return n;
}
pub fn imm32<const CSRS: usize, C: Cfg>(
    i: &IType,
    f: &mut FunctionBody,
    regs: &mut Regs<CSRS>,
    mut k: Block,
    op: Operator,
    load: bool,
    funcs: &Funcs,
    module: &Module,
    entry: Block,
    code: InputRef<'_>,
    root_pc: u64,
    mut bits: impl FnMut(usize) -> Operator,
) -> Block {
    let w = regs.reg::<C>(f, i.rs1() as u8, k);
    if load {
        let (n, v) = talc_common::load32::<Regs<CSRS>, C>(
            w, f, regs, k, op, funcs, module, entry, code, root_pc, bits,
        );
        regs.put_reg(i.rd() as u8, v);
        return n;
    }

    let n = f.add_block();
    let v = f.add_op(k, Operator::I32Const { value: i.imm() }, &[], &[Type::I32]);

    // if load {
    //     let k2 = f.add_block();
    //     let b = f.add_op(k, C::const_64(root_pc), &[], &[C::ty()]);
    //     let b = f.add_op(k, cdef!(C => Sub), &[w, b], &[C::ty()]);
    //     let ts = code
    //         .iter()
    //         .enumerate()
    //         .map(|a| bits(a.0))
    //         .map(|o| {
    //             let l = f.add_block();
    //             let v = f.add_op(l, o, &[], &[C::ty()]);
    //             f.set_terminator(
    //                 l,
    //                 Terminator::Br {
    //                     target: BlockTarget {
    //                         block: n,
    //                         args: vec![v],
    //                     },
    //                 },
    //             );
    //             BlockTarget {
    //                 block: l,
    //                 args: vec![],
    //             }
    //         })
    //         .collect();
    //     f.set_terminator(
    //         k,
    //         Terminator::Select {
    //             value: b,
    //             targets: ts,
    //             default: BlockTarget {
    //                 block: k2,
    //                 args: vec![],
    //             },
    //         },
    //     );
    //     k = k2;
    // };
    // let (w, d) = if load {
    //     let mut ctx = Regs::ctx(f, entry).collect::<Vec<_>>();
    //     ctx.push(w);
    //     let SignatureData::Func { params, returns } =
    //         &module.signatures[module.funcs[funcs.resolve].sig()]
    //     else {
    //         todo!()
    //     };
    //     let r = f.add_op(
    //         k,
    //         Operator::Call {
    //             function_index: funcs.resolve,
    //         },
    //         &ctx,
    //         &returns,
    //     );
    //     let mut r = results_ref_2(f, r);
    //     let w = r.pop().unwrap();
    //     (w, Some(r))
    // } else {
    //     (w, None)
    // };
    let w = if C::MEMORY64 {
        f.add_op(k, Operator::I32WrapI64, &[w], &[Type::I32])
    } else {
        w
    };
    let r: &[Value] = &[w, v];
    let v = f.add_op(k, op, r, &[Type::I32]);
    // put_reg(regs, i.rd() as u8, v);
    let v = if C::MEMORY64 {
        f.add_op(k, Operator::I64ExtendI32S, &[v], &[C::ty()])
    } else {
        v
    };
    // if let Some(d) = d {
    //     f.add_op(
    //         k,
    //         Operator::Call {
    //             function_index: funcs.finalize,
    //         },
    //         &d,
    //         &[],
    //     );
    // };
    f.set_terminator(
        k,
        Terminator::Br {
            target: BlockTarget {
                block: n,
                args: vec![v],
            },
        },
    );
    let v = f.add_blockparam(n, C::ty());
    regs.put_reg(i.rd() as u8, v);
    return n;
}
pub fn store<const CSRS: usize, C: Cfg>(
    i: &SType,
    f: &mut FunctionBody,
    regs: &mut Regs<CSRS>,
    k: Block,
    op: Operator,
    funcs: &Funcs,
    module: &Module,
    entry: Block,
) {
    let v = regs.reg::<C>(f, i.rs2() as u8, k);
    let w = regs.reg::<C>(f, i.rs1() as u8, k);
    return talc_common::store::<Regs, C>(f, v, w, k, op, funcs, module, entry);
    // regs.put_reg(i.rd() as u8, v);
}
pub fn reg_op<const CSRS: usize, C: Cfg>(
    i: &RType,
    f: &mut FunctionBody,
    regs: &mut Regs<CSRS>,
    k: Block,
    op: Operator,
) {
    let v = regs.reg::<C>(f, i.rs2() as u8, k);
    let w = regs.reg::<C>(f, i.rs1() as u8, k);
    let v = f.add_op(k, op, &[w, v], &[C::ty()]);
    regs.put_reg(i.rd() as u8, v);
}
pub fn reg_op32<const CSRS: usize, C: Cfg>(
    i: &RType,
    f: &mut FunctionBody,
    regs: &mut Regs<CSRS>,
    k: Block,
    op: Operator,
) {
    let v = regs.reg::<C>(f, i.rs2() as u8, k);
    let v = if C::MEMORY64 {
        f.add_op(k, Operator::I32WrapI64, &[v], &[Type::I32])
    } else {
        v
    };
    let w = regs.reg::<C>(f, i.rs1() as u8, k);
    let w = if C::MEMORY64 {
        f.add_op(k, Operator::I32WrapI64, &[w], &[Type::I32])
    } else {
        w
    };
    let v = f.add_op(k, op, &[w, v], &[Type::I32]);
    regs.put_reg(
        i.rd() as u8,
        if C::MEMORY64 {
            f.add_op(k, Operator::I64ExtendI32S, &[v], &[C::ty()])
        } else {
            v
        },
    );
}
pub fn shift<const CSRS: usize, C: Cfg>(
    i: &ShiftType,
    f: &mut FunctionBody,
    regs: &mut Regs<CSRS>,
    k: Block,
    op: Operator,
) {
    let v = f.add_op(k, C::const_32(i.shamt()), &[], &[C::ty()]);
    let w = regs.reg::<C>(f, i.rs1() as u8, k);
    let v = f.add_op(k, op, &[w, v], &[C::ty()]);
    regs.put_reg(i.rd() as u8, v);
}
pub fn shift32<const CSRS: usize, C: Cfg>(
    i: &ShiftType,
    f: &mut FunctionBody,
    regs: &mut Regs<CSRS>,
    k: Block,
    op: Operator,
) {
    let v = f.add_op(
        k,
        Operator::I32Const { value: i.shamt() },
        &[],
        &[Type::I32],
    );
    let w = regs.reg::<C>(f, i.rs1() as u8, k);
    let w = if C::MEMORY64 {
        f.add_op(k, Operator::I32WrapI64, &[w], &[Type::I32])
    } else {
        w
    };
    let v = f.add_op(k, op, &[w, v], &[Type::I32]);
    regs.put_reg(
        i.rd() as u8,
        if C::MEMORY64 {
            f.add_op(k, Operator::I64ExtendI32S, &[v], &[C::ty()])
        } else {
            v
        },
    );
}
pub fn br<const CSRS: usize, C: Cfg>(
    i: &BType,
    f: &mut FunctionBody,
    regs: &mut Regs<CSRS>,
    k: Block,
    op: Operator,
    pc: u64,
    shim: Block,
) {
    let v = regs.reg::<C>(f, i.rs2() as u8, k);
    let w = regs.reg::<C>(f, i.rs1() as u8, k);
    let v = f.add_op(k, op, &[w, v], &[C::ty()]);
    let cont = f.add_op(k, C::const_64(pc + 4), &[], &[C::ty()]);
    let go = f.add_op(k, C::const_64(pc + (i.imm() as u64)), &[], &[C::ty()]);
    // let a = f.add_op(k, Operator::Select, &[v, cont, go], &[C::ty()]);

    let l = f.add_block();
    f.set_terminator(
        k,
        Terminator::CondBr {
            cond: v,
            if_true: BlockTarget {
                block: l,
                args: vec![go],
            },
            if_false: BlockTarget {
                block: l,
                args: vec![cont],
            },
        },
    );
    let a = f.add_blockparam(l, C::ty());
    f.set_terminator(
        l,
        Terminator::Br {
            target: BlockTarget {
                block: shim,
                args: vec![a]
                    .into_iter()
                    .chain(regs.regs.into_iter())
                    .chain(regs.csrs.iter().cloned())
                    .collect(),
            },
        },
    );
}
pub struct R5<const CSRS: usize = 4096> {}
impl<const CSRS: usize> Arch for R5<CSRS> {
    type Regs = Regs<CSRS>;

    fn go<C: Cfg, H: Hook<Self::Regs>>(
        f: &mut FunctionBody,
        entry: Block,
        code: InputRef<'_>,
        root_pc: u64,
        funcs: &Funcs,
        module: &mut Module,
        hook: &mut H,
    ) -> ArchRes {
        crate::go::<CSRS, C, H>(f, entry, code, root_pc, funcs, module, hook)
    }
}
pub fn go<const CSRS: usize, C: Cfg, H: Hook<Regs<CSRS>>>(
    f: &mut FunctionBody,
    entry: Block,
    code: InputRef<'_>,
    root_pc: u64,
    funcs: &Funcs,
    module: &mut Module,
    hook: &mut H,
) -> ArchRes {
    // let code = hook.update_code::<CSRS,C>(code);
    // let code = code.as_ref();
    let mut w = code
        .code
        .windows(4)
        .map(|w| u32::from_ne_bytes(w.try_into().unwrap()))
        .enumerate();
    let shim = f.add_block();
    let mut v = vec![];
    for i in 0..4 {
        let mut r: Regs<CSRS> = Regs {
            regs: f.blocks[entry].params[..31]
                .iter()
                .map(|a| a.1)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            csrs: f.blocks[entry].params[32..(31 + CSRS)]
                .iter()
                .map(|a| a.1)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        };
        let mut k = f.add_block();

        let (idx, a) = w.next().unwrap();
        if let Ok(i) = riscv_decode::decode(a) {
            k = process::<CSRS, C, H>(
                f,
                &i,
                &mut r,
                k,
                TryInto::<u64>::try_into(idx).unwrap().wrapping_add(root_pc),
                shim,
                funcs,
                module,
                entry,
                code,
                root_pc,
                hook,
            );
        }
        v.push((k, r));
    }
    for (idx, a) in w {
        let (k, mut regs) = v[idx - 4].clone();

        let k = match riscv_decode::decode(a) {
            Ok(i) => process::<CSRS, C, H>(
                f,
                &i,
                &mut regs,
                k,
                TryInto::<u64>::try_into(idx).unwrap().wrapping_add(root_pc),
                shim,
                funcs,
                module,
                entry,
                code,
                root_pc,
                hook,
            ),
            Err(_) => f.add_block(),
        };
        v.push((k, regs));
    }
    let pc = f.add_blockparam(shim, C::ty());
    let mut regs: Regs<CSRS> = Regs {
        regs: array::from_fn(|_| f.add_blockparam(shim, C::ty())),
        csrs: array::from_fn(|_| f.add_blockparam(shim, C::ty())),
    };
    if let Some(u) = &funcs.u_deopt {
        let u = *u;
    }
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
                    args: regs.iter().collect(),
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
    let c = Regs::<CSRS>::ctx(f, entry).collect::<Vec<_>>();
    match &funcs.u_deopt {
        None => {
            f.set_terminator(
                rb,
                Terminator::ReturnCall {
                    func: funcs.deopt,
                    args: vec![pc]
                        .into_iter()
                        .chain(regs.iter())
                        .chain(c.into_iter())
                        .collect(),
                },
            );
        }
        Some(u) => {
            let u = *u;
            let new_pc = f.add_op(shim, C::const_64(u), &[], &[C::ty()]);
            // let u = u.wrapping_add(4);
            let new_lr = f.add_op(shim, C::const_64(4), &[], &[C::ty()]);
            let new_lr = f.add_op(shim, cdef!(C => Add), &[pc, new_lr], &[C::ty()]);
            // let mut regs = regs.clone();
            let old_lr = replace(&mut regs.regs[0], new_lr);
            regs.csrs[0] = old_lr;

            f.set_terminator(
                rb,
                Terminator::Br {
                    target: BlockTarget {
                        block: shim,
                        args: vec![new_pc]
                            .into_iter()
                            .chain(regs.iter())
                            .chain(c.into_iter())
                            .collect(),
                    },
                },
            );
        }
    }
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
pub fn process<const CSRS: usize, C: Cfg, H: Hook<Regs<CSRS>>>(
    f: &mut FunctionBody,
    i: &Instruction,
    regs: &mut Regs<CSRS>,
    k: Block,
    pc: u64,
    shim: Block,
    funcs: &Funcs,
    module: &mut Module,
    entry: Block,
    code: InputRef<'_>,
    root_pc: u64,
    hook: &mut H,
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
    match i {
        //2.4.1
        Instruction::Addi(i) => {
            imm::<CSRS, C>(
                i,
                f,
                regs,
                k,
                cdef!(C => Add),
                false,
                funcs,
                module,
                entry,
                code,
                root_pc,
                |_| unreachable!(),
            );
        }
        Instruction::Andi(i) => {
            imm::<CSRS, C>(
                i,
                f,
                regs,
                k,
                cdef!(C => And),
                false,
                funcs,
                module,
                entry,
                code,
                root_pc,
                |_| unreachable!(),
            );
        }
        Instruction::Ori(i) => {
            imm::<CSRS, C>(
                i,
                f,
                regs,
                k,
                cdef!(C => Or),
                false,
                funcs,
                module,
                entry,
                code,
                root_pc,
                |_| unreachable!(),
            );
        }
        Instruction::Xori(i) => {
            imm::<CSRS, C>(
                i,
                f,
                regs,
                k,
                cdef!(C => Xor),
                false,
                funcs,
                module,
                entry,
                code,
                root_pc,
                |_| unreachable!(),
            );
        }
        Instruction::Slti(i) => {
            imm::<CSRS, C>(
                i,
                f,
                regs,
                k,
                cdef!(C => LtS),
                false,
                funcs,
                module,
                entry,
                code,
                root_pc,
                |_| unreachable!(),
            );
        }
        Instruction::Sltiu(i) => {
            imm::<CSRS, C>(
                i,
                f,
                regs,
                k,
                cdef!(C => LtU),
                false,
                funcs,
                module,
                entry,
                code,
                root_pc,
                |_| unreachable!(),
            );
        }
        Instruction::Slli(i) => {
            shift::<CSRS, C>(i, f, regs, k, cdef!(C => Shl));
        }
        Instruction::Srli(i) => {
            shift::<CSRS, C>(i, f, regs, k, cdef!(C => ShrU));
        }
        Instruction::Srai(i) => {
            shift::<CSRS, C>(i, f, regs, k, cdef!(C => ShrS));
        }
        Instruction::Lui(i) => {
            regs.put_reg(
                i.rd() as u8,
                f.add_op(
                    k,
                    C::const_64((i.imm() << 12) as i32 as i64 as u64),
                    &[],
                    &[C::ty()],
                ),
            );
        }
        Instruction::Auipc(i) => {
            regs.put_reg(
                i.rd() as u8,
                f.add_op(
                    k,
                    C::const_64(((i.imm() << 12) as i32 as i64 as u64).wrapping_add(pc)),
                    &[],
                    &[C::ty()],
                ),
            );
        }
        //4.2.1
        Instruction::Addiw(i) => {
            imm32::<CSRS, C>(
                i,
                f,
                regs,
                k,
                Operator::I32Add,
                false,
                funcs,
                module,
                entry,
                code,
                root_pc,
                |_| unreachable!(),
            );
        }
        // Instruction::Andiw(i) => {
        //     imm32::<CSRS,C>(i, f, regs, k, cdef!(C => And), false, funcs, module);
        // }
        // Instruction::Oriw(i) => {
        //     imm32::<CSRS,C>(i, f, regs, k, cdef!(C => Or), false, funcs, module);
        // }
        // Instruction::Xoriw(i) => {
        //     imm32::<CSRS,C>(i, f, regs, k, cdef!(C => Xor), false, funcs, module);
        // }
        // Instruction::Sltiw(i) => {
        //     imm32::<CSRS,C>(i, f, regs, k, cdef!(C => LtS), false, funcs, module);
        // }
        // Instruction::Sltiuw(i) => {
        //     imm32::<CSRS,C>(i, f, regs, k, cdef!(C => LtU), false, funcs, module);
        // }
        Instruction::Slliw(i) => {
            shift32::<CSRS, C>(i, f, regs, k, Operator::I32Shl);
        }
        Instruction::Srliw(i) => {
            shift32::<CSRS, C>(i, f, regs, k, Operator::I32ShrU);
        }
        Instruction::Sraiw(i) => {
            shift32::<CSRS, C>(i, f, regs, k, Operator::I32ShrS);
        }
        //2.4.2
        Instruction::Add(i) => {
            reg_op::<CSRS, C>(i, f, regs, k, cdef!(C => Add));
        }
        Instruction::Sub(i) => {
            reg_op::<CSRS, C>(i, f, regs, k, cdef!(C => Sub));
        }
        Instruction::And(i) => {
            reg_op::<CSRS, C>(i, f, regs, k, cdef!(C => And));
        }
        Instruction::Or(i) => {
            reg_op::<CSRS, C>(i, f, regs, k, cdef!(C => Or));
        }
        Instruction::Xor(i) => {
            reg_op::<CSRS, C>(i, f, regs, k, cdef!(C => Xor));
        }
        Instruction::Slt(i) => {
            reg_op::<CSRS, C>(i, f, regs, k, cdef!(C => LtS));
        }
        Instruction::Sltu(i) => {
            reg_op::<CSRS, C>(i, f, regs, k, cdef!(C => LtU));
        }
        Instruction::Sll(i) => {
            reg_op::<CSRS, C>(i, f, regs, k, cdef!(C => Shl));
        }
        Instruction::Srl(i) => {
            reg_op::<CSRS, C>(i, f, regs, k, cdef!(C => ShrU));
        }
        Instruction::Sra(i) => {
            reg_op::<CSRS, C>(i, f, regs, k, cdef!(C => ShrS));
        }
        //4.2.2
        Instruction::Addw(i) => {
            reg_op32::<CSRS, C>(i, f, regs, k, Operator::I32Add);
        }
        Instruction::Subw(i) => {
            reg_op32::<CSRS, C>(i, f, regs, k, Operator::I32Sub);
        }
        // Instruction::Andw(i) => {
        //     reg_op::<CSRS,C>(i, f, regs, k, cdef!(C => And));
        // }
        // Instruction::Orw(i) => {
        //     reg_op::<CSRS,C>(i, f, regs, k, cdef!(C => Or));
        // }
        // Instruction::Xorw(i) => {
        //     reg_op::<CSRS,C>(i, f, regs, k, cdef!(C => Xor));
        // }
        // Instruction::Sltw(i) => {
        //     reg_op::<CSRS,C>(i, f, regs, k, cdef!(C => LtS));
        // }
        // Instruction::Sltuw(i) => {
        //     reg_op::<CSRS,C>(i, f, regs, k, cdef!(C => LtU));
        // }
        Instruction::Sllw(i) => {
            reg_op32::<CSRS, C>(i, f, regs, k, Operator::I32Shl);
        }
        Instruction::Srlw(i) => {
            reg_op32::<CSRS, C>(i, f, regs, k, Operator::I32ShrU);
        }
        Instruction::Sraw(i) => {
            reg_op32::<CSRS, C>(i, f, regs, k, Operator::I32ShrS);
        }

        //2.5.1
        Instruction::Jal(j) => {
            let a = j.imm();
            let a = a as i32 as i64;
            let a = a + (pc as i64);
            let r = j.rd();

            regs.put_reg(r as u8, f.add_op(k, C::const_64(pc + 4), &[], &[C::ty()]));
            let a = f.add_op(k, C::const_64(a as u64), &[], &[C::ty()]);
            f.set_terminator(
                k,
                Terminator::Br {
                    target: BlockTarget {
                        block: shim,
                        args: vec![a]
                            .into_iter()
                            .chain(regs.regs.into_iter())
                            .chain(regs.csrs.iter().cloned())
                            .collect(),
                    },
                },
            );
        }
        Instruction::Jalr(j) => {
            let a = j.imm();
            let r = j.rd();

            let a = f.add_op(k, C::const_32(a), &[], &[C::ty()]);
            let mut base = regs.reg::<C>(f, j.rs1() as u8, k);
            if let Some(u) = &funcs.u_deopt {
                if u.wrapping_sub(root_pc) <= pc.wrapping_sub(root_pc)
                    && r == 0
                    && j.rs1() == 1
                    && funcs.u_deopt_marker.contains(&pc.wrapping_sub(*u))
                {
                    regs.regs[0] = regs.csrs[0];
                }
            }
            regs.put_reg(r as u8, f.add_op(k, C::const_64(pc + 4), &[], &[C::ty()]));
            let a = f.add_op(k, cdef!(C => Add), &[base, a], &[C::ty()]);
            f.set_terminator(
                k,
                Terminator::Br {
                    target: BlockTarget {
                        block: shim,
                        args: vec![a]
                            .into_iter()
                            .chain(regs.regs.into_iter())
                            .chain(regs.csrs.iter().cloned())
                            .collect(),
                    },
                },
            );
        }
        //2.5.2
        Instruction::Beq(b) => {
            br::<CSRS, C>(b, f, regs, k, cdef!(C => Eq), pc, shim);
        }
        Instruction::Bne(b) => {
            br::<CSRS, C>(b, f, regs, k, cdef!(C => Ne), pc, shim);
        }
        Instruction::Blt(b) => {
            br::<CSRS, C>(b, f, regs, k, cdef!(C => LtS), pc, shim);
        }
        Instruction::Bltu(b) => {
            br::<CSRS, C>(b, f, regs, k, cdef!(C => LtU), pc, shim);
        }
        Instruction::Bge(b) => {
            br::<CSRS, C>(b, f, regs, k, cdef!(C => GeS), pc, shim);
        }
        Instruction::Bgeu(b) => {
            br::<CSRS, C>(b, f, regs, k, cdef!(C => GeU), pc, shim);
        }
        //2.6
        //Loads
        Instruction::Lb(l) => {
            k = imm::<CSRS, C>(
                l,
                f,
                regs,
                k,
                cdef!(C => Load8S{memory: MemoryArg {
                    align: 0,
                    offset: l.imm().into(),
                    memory: funcs.memory,
                }}),
                true,
                funcs,
                module,
                entry,
                code,
                root_pc,
                |i| C::const_64(code.code[i].clone() as i8 as i64 as u64),
            );
        }
        Instruction::Lbu(l) => {
            k = imm::<CSRS, C>(
                l,
                f,
                regs,
                k,
                cdef!(C => Load8U{memory: MemoryArg {
                    align: 0,
                    offset: l.imm().into(),
                    memory: funcs.memory,
                }}),
                true,
                funcs,
                module,
                entry,
                code,
                root_pc,
                |i| C::const_64(code.code[i].clone() as u64),
            );
        }
        Instruction::Lh(l) => {
            k =
                imm::<CSRS, C>(
                    l,
                    f,
                    regs,
                    k,
                    cdef!(C => Load16S{memory: MemoryArg {
                        align: 0,
                        offset: l.imm().into(),
                        memory: funcs.memory,
                    }}),
                    true,
                    funcs,
                    module,
                    entry,
                    code,
                    root_pc,
                    |i| {
                        C::const_64(u16::from_le_bytes(code.code[i..][..2].try_into().unwrap())
                            as i16 as i64 as u64)
                    },
                );
        }
        Instruction::Lhu(l) => {
            k = imm::<CSRS, C>(
                l,
                f,
                regs,
                k,
                cdef!(C => Load16U{memory: MemoryArg {
                    align: 0,
                    offset: l.imm().into(),
                    memory: funcs.memory,
                }}),
                true,
                funcs,
                module,
                entry,
                code,
                root_pc,
                |i| C::const_64(u16::from_le_bytes(code.code[i..][..2].try_into().unwrap()) as u64),
            );
        }
        Instruction::Lw(l) => {
            k =
                imm::<CSRS, C>(
                    l,
                    f,
                    regs,
                    k,
                    if C::MEMORY64 {
                        Operator::I64Load32S {
                            memory: MemoryArg {
                                align: 0,
                                offset: l.imm().into(),
                                memory: funcs.memory,
                            },
                        }
                    } else {
                        Operator::I32Load {
                            memory: MemoryArg {
                                align: 0,
                                offset: l.imm().into(),
                                memory: funcs.memory,
                            },
                        }
                    },
                    true,
                    funcs,
                    module,
                    entry,
                    code,
                    root_pc,
                    |i| {
                        C::const_64(u32::from_le_bytes(code.code[i..][..4].try_into().unwrap())
                            as i32 as i64 as u64)
                    },
                );
        }
        //4.3
        Instruction::Lwu(l) => {
            k = imm::<CSRS, C>(
                l,
                f,
                regs,
                k,
                if C::MEMORY64 {
                    Operator::I64Load32U {
                        memory: MemoryArg {
                            align: 0,
                            offset: l.imm().into(),
                            memory: funcs.memory,
                        },
                    }
                } else {
                    Operator::I32Load {
                        memory: MemoryArg {
                            align: 0,
                            offset: l.imm().into(),
                            memory: funcs.memory,
                        },
                    }
                },
                true,
                funcs,
                module,
                entry,
                code,
                root_pc,
                |i| C::const_64(u32::from_le_bytes(code.code[i..][..4].try_into().unwrap()) as u64),
            );
        }
        Instruction::Ld(l) => {
            k = imm::<CSRS, C>(
                l,
                f,
                regs,
                k,
                if C::MEMORY64 {
                    Operator::I64Load {
                        memory: MemoryArg {
                            align: 0,
                            offset: l.imm().into(),
                            memory: funcs.memory,
                        },
                    }
                } else {
                    Operator::I32Load {
                        memory: MemoryArg {
                            align: 0,
                            offset: l.imm().into(),
                            memory: funcs.memory,
                        },
                    }
                },
                true,
                funcs,
                module,
                entry,
                code,
                root_pc,
                |i| C::const_64(u64::from_le_bytes(code.code[i..][..8].try_into().unwrap())),
            )
        }
        //Stores
        Instruction::Sb(s) => {
            store::<CSRS, C>(
                s,
                f,
                regs,
                k,
                cdef!(C => Store8 {
                    memory: MemoryArg {
                        align: 0,
                        offset: s.imm().into(),
                        memory: funcs.memory,
                    },
                }),
                funcs,
                module,
                entry,
            );
        }
        Instruction::Sh(s) => {
            store::<CSRS, C>(
                s,
                f,
                regs,
                k,
                cdef!(C => Store16 {
                    memory: MemoryArg {
                        align: 0,
                        offset: s.imm().into(),
                        memory: funcs.memory,
                    },
                }),
                funcs,
                module,
                entry,
            );
        }
        Instruction::Sw(s) => {
            store::<CSRS, C>(
                s,
                f,
                regs,
                k,
                if C::MEMORY64 {
                    Operator::I64Store32 {
                        memory: MemoryArg {
                            align: 0,
                            offset: s.imm().into(),
                            memory: funcs.memory,
                        },
                    }
                } else {
                    Operator::I32Store {
                        memory: MemoryArg {
                            align: 0,
                            offset: s.imm().into(),
                            memory: funcs.memory,
                        },
                    }
                },
                funcs,
                module,
                entry,
            );
        }
        //4.3
        Instruction::Sd(s) => {
            store::<CSRS, C>(
                s,
                f,
                regs,
                k,
                if C::MEMORY64 {
                    Operator::I64Store {
                        memory: MemoryArg {
                            align: 0,
                            offset: s.imm().into(),
                            memory: funcs.memory,
                        },
                    }
                } else {
                    Operator::I32Store {
                        memory: MemoryArg {
                            align: 0,
                            offset: s.imm().into(),
                            memory: funcs.memory,
                        },
                    }
                },
                funcs,
                module,
                entry,
            );
        }
        //2.7
        Instruction::Fence(f) => {}
        Instruction::FenceI => {
            f.set_terminator(k, Terminator::Unreachable);
        }
        //2.8
        Instruction::Ecall => {
            let ctx = Regs::<CSRS>::ctx(f, entry).collect::<Vec<_>>();
            let ecall_params = regs
                .regs
                .iter()
                .cloned()
                .chain(ctx.into_iter())
                .collect::<Vec<_>>();
            let tys = regs
                .regs
                .iter()
                .cloned()
                .filter_map(|v| f.values[v].ty(&f.type_pool))
                .collect::<Vec<_>>();
            let a = f.add_op(
                k,
                Operator::Call {
                    function_index: funcs.ecall,
                },
                &ecall_params,
                &tys,
            );
            let r = results_ref_2(f, a);
            regs.regs = r[..31].try_into().unwrap();
        }
        //7.1
        Instruction::Csrrw(c) => {
            let old = regs.csrs[c.csr() as usize];
            regs.csrs[c.csr() as usize] = regs.reg::<C>(f, c.rs1() as u8, k);
            regs.put_reg(c.rd() as u8, old);
        }
        Instruction::Csrrs(c) => {
            let old = regs.csrs[c.csr() as usize];
            let x = regs.reg::<C>(f, c.rs1() as u8, k);
            let x = f.add_op(k, cdef!(C => Or), &[x, old], &[C::ty()]);
            regs.csrs[c.csr() as usize] = x;
            regs.put_reg(c.rd() as u8, old);
        }
        Instruction::Csrrc(c) => {
            let old = regs.csrs[c.csr() as usize];
            let x = regs.reg::<C>(f, c.rs1() as u8, k);
            let fb = f.add_op(k, C::const_64(-1i64 as u64), &[], &[C::ty()]);
            let x = f.add_op(k, cdef!(C => Xor), &[x, fb], &[C::ty()]);
            let x = f.add_op(k, cdef!(C => And), &[x, old], &[C::ty()]);
            regs.csrs[c.csr() as usize] = x;
            regs.put_reg(c.rd() as u8, old);
        }
        //Immediate
        Instruction::Csrrwi(c) => {
            let old = regs.csrs[c.csr() as usize];
            regs.csrs[c.csr() as usize] =
                f.add_op(k, C::const_64(c.zimm() as u64), &[], &[C::ty()]);
            regs.put_reg(c.rd() as u8, old);
        }
        Instruction::Csrrsi(c) => {
            let old = regs.csrs[c.csr() as usize];
            let x = f.add_op(k, C::const_64(c.zimm() as u64), &[], &[C::ty()]);
            let x = f.add_op(k, cdef!(C => Or), &[x, old], &[C::ty()]);
            regs.csrs[c.csr() as usize] = x;
            regs.put_reg(c.rd() as u8, old);
        }
        Instruction::Csrrci(c) => {
            let old = regs.csrs[c.csr() as usize];
            let x = f.add_op(k, C::const_64(c.zimm() as u64), &[], &[C::ty()]);
            let fb = f.add_op(k, C::const_64(-1i64 as u64), &[], &[C::ty()]);
            let x = f.add_op(k, cdef!(C => Xor), &[x, fb], &[C::ty()]);
            let x = f.add_op(k, cdef!(C => And), &[x, old], &[C::ty()]);
            regs.csrs[c.csr() as usize] = x;
            regs.put_reg(c.rd() as u8, old);
        }

        //Fallback
        i => {
            dbg!("instruction not supported: ", i);
        }
    };
    return k;
}
