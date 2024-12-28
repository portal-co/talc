use std::{borrow::Cow, collections::BTreeMap, iter::once};

use typenum::{Bit, Unsigned};
pub use waffle::Operator;
pub extern crate paste;
use waffle::{
    Block, BlockTarget, Func, FunctionBody, Memory, MemoryArg, MemoryData, MemorySegment, Module,
    SignatureData, Terminator, Type, Value,
};
use waffle_ast::{results_ref_2, Builder, Expr};
pub trait Hook<R: TRegs> {
    fn hook<C: Cfg>(
        &mut self,
        f: &mut FunctionBody,
        k: Block,
        r: &mut R,
        pc: u64,
        funcs: &Funcs,
        module: &mut Module,
        code: &[u8],
        code_idx: usize,
    ) -> Block;
    fn update_code<'a, C: Cfg>(&mut self, code: &'a [u8]) -> Cow<'a, [u8]> {
        return Cow::Borrowed(code);
    }
}
impl<R: TRegs> Hook<R> for () {
    fn hook<C: Cfg>(
        &mut self,
        f: &mut FunctionBody,
        k: Block,
        r: &mut R,
        pc: u64,
        funcs: &Funcs,
        module: &mut Module,
        code: &[u8],
        code_idx: usize,
    ) -> Block {
        k
    }
}
impl<R: TRegs, A: Hook<R>, B: Hook<R>> Hook<R> for (A, B) {
    fn hook<C: Cfg>(
        &mut self,
        f: &mut FunctionBody,
        mut k: Block,
        r: &mut R,
        pc: u64,
        funcs: &Funcs,
        module: &mut Module,
        code: &[u8],
        code_idx: usize,
    ) -> Block {
        k = self.0.hook::<C>(f, k, r, pc, funcs, module, code, code_idx);
        k = self.1.hook::<C>(f, k, r, pc, funcs, module, code, code_idx);
        k
    }
    fn update_code<'a, C: Cfg>(&mut self, code: &'a [u8]) -> Cow<'a, [u8]> {
        return match self.0.update_code::<'_, C>(code) {
            Cow::Borrowed(a) => self.1.update_code::<'_, C>(a),
            Cow::Owned(b) => Cow::Owned(self.1.update_code::<'_, C>(b.as_slice()).into_owned()),
        };
    }
}
pub fn store<R: TRegs, C: Cfg>(
    // i: &SType,
    f: &mut FunctionBody,
    // regs: &mut Regs,
    a: Value,
    b: Value,
    k: Block,
    op: Operator,
    funcs: &Funcs,
    module: &Module,
    entry: Block,
) {
    let v = a;
    let w = b;
    let mut ctx = R::ctx(f, entry).collect::<Vec<_>>();
    ctx.push(w);
    let SignatureData::Func { params, returns } =
        &module.signatures[module.funcs[funcs.resolve].sig()]
    else {
        todo!()
    };
    let r = f.add_op(
        k,
        Operator::Call {
            function_index: funcs.resolve,
        },
        &ctx,
        &returns,
    );
    let mut r = results_ref_2(f, r);
    let w = r.pop().unwrap();
    let v = f.add_op(k, op, &[w, v], &[]);
    f.add_op(
        k,
        Operator::Call {
            function_index: funcs.finalize,
        },
        &r,
        &[],
    );
    // regs.put_reg(i.rd() as u8, v);
}
pub fn load<Regs: TRegs, C: Cfg>(
    // i: &IType,
    w: Value,

    f: &mut FunctionBody,
    regs: &mut Regs,
    mut k: Block,
    op: Operator,
    // load: bool,
    funcs: &Funcs,
    module: &Module,
    entry: Block,
    code: &[u8],
    root_pc: u64,
    // mut bits: impl FnMut(usize) -> Operator,
) -> (Block, Value) {
    let n = f.add_block();
    // let v = f.add_op(k, C::const_32(i.imm()), &[], &[C::ty()]);
    // let w = regs.reg::<C>(f, i.rs1() as u8, k);
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
    let (w, d) = {
        let mut ctx = Regs::ctx(f, entry).collect::<Vec<_>>();
        ctx.push(w);
        let SignatureData::Func { params, returns } =
            &module.signatures[module.funcs[funcs.resolve].sig()]
        else {
            todo!()
        };
        let r = f.add_op(
            k,
            Operator::Call {
                function_index: funcs.resolve,
            },
            &ctx,
            &returns,
        );
        let mut r = results_ref_2(f, r);
        let w = r.pop().unwrap();
        (w, r)
    };
    let r: &[Value] = &[w];
    let v = f.add_op(k, op, r, &[C::ty()]);
    // put_reg(regs, i.rd() as u8, v);

    // if let Some(d) = d {
    f.add_op(
        k,
        Operator::Call {
            function_index: funcs.finalize,
        },
        &d,
        &[],
    );
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
    // regs.put_reg(i.rd() as u8, v);
    return (n, v);
}
pub fn load32<Regs: TRegs, C: Cfg>(
    // i: &IType,
    w: Value,
    f: &mut FunctionBody,
    regs: &mut Regs,
    mut k: Block,
    op: Operator,
    // load: bool,
    funcs: &Funcs,
    module: &Module,
    entry: Block,
    code: &[u8],
    root_pc: u64,
    // mut bits: impl FnMut(usize) -> Operator,
) -> (Block, Value) {
    let n = f.add_block();
    // let v = f.add_op(k, Operator::I32Const { value: i.imm() }, &[], &[Type::I32]);
    // let w = regs.reg::<C>(f, i.rs1() as u8, k);
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
    let (w, d) = {
        let mut ctx = Regs::ctx(f, entry).collect::<Vec<_>>();
        ctx.push(w);
        let SignatureData::Func { params, returns } =
            &module.signatures[module.funcs[funcs.resolve].sig()]
        else {
            todo!()
        };
        let r = f.add_op(
            k,
            Operator::Call {
                function_index: funcs.resolve,
            },
            &ctx,
            &returns,
        );
        let mut r = results_ref_2(f, r);
        let w = r.pop().unwrap();
        (w, r)
    };
    let w = if C::MEMORY64 {
        f.add_op(k, Operator::I32WrapI64, &[w], &[Type::I32])
    } else {
        w
    };
    let r: &[Value] = &[w];
    let v = f.add_op(k, op, r, &[Type::I32]);
    // put_reg(regs, i.rd() as u8, v);
    let v = if C::MEMORY64 {
        f.add_op(k, Operator::I64ExtendI32S, &[v], &[C::ty()])
    } else {
        v
    };
    // if let Some(d) = d {
    f.add_op(
        k,
        Operator::Call {
            function_index: funcs.finalize,
        },
        &d,
        &[],
    );
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
    // regs.put_reg(i.rd() as u8, v);
    return (n, v);
}
pub struct Funcs {
    pub memory: Memory,
    pub ecall: Func,
    pub resolve: Func,
    pub finalize: Func,
    pub deopt: Func,
    pub can_multi_memory: bool,
}
impl Funcs {
    pub fn init<R: TRegs, C: Cfg>(
        &self,
        code: &[u8],
        root_pc: u64,
        f: &mut FunctionBody,
        mut k: Block,
        entry: Block,
        module: &mut Module,
    ) -> Block {
        let SignatureData::Func { params, returns } =
            &module.signatures[module.funcs[self.resolve].sig()]
        else {
            todo!()
        };
        let vs: [Value; 256] = std::array::from_fn(|v| {
            f.add_op(
                k,
                Operator::I32Const {
                    value: (v & 0xff) as u32,
                },
                &[],
                &[Type::I32],
            )
        });
        let m = if self.can_multi_memory {
            Some(module.memories.push(MemoryData {
                initial_pages: code.len(),
                maximum_pages: Some(code.len()),
                page_size_log2: Some(0),
                memory64: C::MEMORY64,
                shared: false,
                segments: vec![MemorySegment {
                    offset: 0,
                    data: code.to_owned(),
                }],
            }))
        } else {
            None
        };
        let ctx = R::ctx(f, entry).collect::<Vec<_>>();
        let v0 = f.add_op(k, Operator::I32Const { value: 0 }, &[], &[Type::I32]);
        match m {
            None => {
                for (ca, i) in code.iter().enumerate() {
                    let c = ca as u64;
                    let c = c.wrapping_add(root_pc);
                    let v = f.add_op(k, C::const_64(c), &[], &[C::ty()]);
                    let ctx = ctx.iter().cloned().chain(once(v)).collect::<Vec<_>>();
                    let r = f.add_op(
                        k,
                        Operator::Call {
                            function_index: self.resolve,
                        },
                        &ctx,
                        &returns,
                    );
                    let mut r = results_ref_2(f, r);
                    let w = r.pop().unwrap();
                    let v = vs[*i as usize];
                    let v = f.add_op(
                        k,
                        Operator::I32Store8 {
                            memory: MemoryArg {
                                align: 0,
                                offset: 0,
                                memory: self.memory,
                            },
                        },
                        &[w, v],
                        &[],
                    );
                    f.add_op(
                        k,
                        Operator::Call {
                            function_index: self.finalize,
                        },
                        &r,
                        &[],
                    );
                }
            }
            Some(m) => {
                let n = f.add_block();
                let p = f.add_blockparam(n, C::ty());
                let rb = f.add_block();
                let pl = f.add_op(n, C::const_64(root_pc), &[], &[C::ty()]);
                let pl = f.add_op(n, cdef!(C => Sub), &[p, pl], &[C::ty()]);
                let pr = f.add_op(n, C::const_32(1), &[], &[C::ty()]);
                let pr = f.add_op(n, cdef!(C => Add), &[p, pr], &[C::ty()]);
                let ctx = ctx.iter().cloned().chain(once(p)).collect::<Vec<_>>();
                let r = f.add_op(
                    n,
                    Operator::Call {
                        function_index: self.resolve,
                    },
                    &ctx,
                    &returns,
                );
                let mut r = results_ref_2(f, r);
                let w = r.pop().unwrap();
                let v = f.add_op(
                    n,
                    Operator::I32Load8U {
                        memory: MemoryArg {
                            align: 0,
                            offset: 0,
                            memory: m,
                        },
                    },
                    &[pl],
                    &[Type::I32],
                );
                let v = f.add_op(
                    n,
                    Operator::I32Store8 {
                        memory: MemoryArg {
                            align: 0,
                            offset: 0,
                            memory: self.memory,
                        },
                    },
                    &[w, v],
                    &[],
                );
                f.add_op(
                    n,
                    Operator::Call {
                        function_index: self.finalize,
                    },
                    &r,
                    &[],
                );
                f.set_terminator(
                    n,
                    waffle::Terminator::Select {
                        value: pl,
                        targets: vec![BlockTarget {
                            block: rb,
                            args: vec![],
                        }],
                        default: BlockTarget {
                            block: n,
                            args: vec![pr],
                        },
                    },
                );
                let z = f.add_op(k, C::const_32(0), &[], &[C::ty()]);
                f.set_terminator(
                    k,
                    waffle::Terminator::Br {
                        target: BlockTarget {
                            block: n,
                            args: vec![z],
                        },
                    },
                );
                k = rb;
            }
        };
        return k;
    }
}
pub trait Cfg {
    const MEMORY64: bool;
    type BITS: Unsigned;
}
pub trait CfgExt: Cfg {
    fn ty() -> Type {
        if Self::MEMORY64 {
            Type::I64
        } else {
            Type::I32
        }
    }
    fn const_32(a: u32) -> Operator {
        if Self::MEMORY64 {
            Operator::I64Const { value: a as u64 }
        } else {
            Operator::I32Const { value: a }
        }
    }
    fn const_64(a: u64) -> Operator {
        if Self::MEMORY64 {
            Operator::I64Const { value: a }
        } else {
            Operator::I32Const {
                value: (a & 0xfffffff) as u32,
            }
        }
    }
    fn mul2(f: &mut FunctionBody, k: Block, a: Value, b: Value) -> (Value, Value) {
        let (mut low, mut high);
        let bw2 = if Self::MEMORY64 { 32 } else { 16 };
        let lower_mask = f.add_op(k, Self::const_64(!0 >> bw2), &[], &[Self::ty()]);
        let mut e = waffle_ast::Expr::Bind(
            cdef!(Self => Mul),
            vec![
                waffle_ast::Expr::Bind(
                    cdef!(Self => And),
                    vec![a, lower_mask].into_iter().map(Expr::Leaf).collect(),
                ),
                waffle_ast::Expr::Bind(
                    cdef!(Self => And),
                    vec![b, lower_mask].into_iter().map(Expr::Leaf).collect(),
                ),
            ],
        );
        low = e.build(&mut Module::empty(), f, k).unwrap().0;
        e = Expr::Bind(
            cdef!(Self => ShrU),
            vec![
                Expr::Leaf(low),
                Expr::Bind(Operator::I32Const { value: bw2 }, vec![]),
            ],
        );
        let mut t = e.build(&mut Module::empty(), f, k).unwrap().0;
        low = f.add_op(k, cdef!(Self => And), &[low, lower_mask], &[Self::ty()]);
        e = Expr::Bind(
            cdef!(Self => Add),
            vec![
                Expr::Leaf(t),
                waffle_ast::Expr::Bind(
                    cdef!(Self => Mul),
                    vec![
                        waffle_ast::Expr::Bind(
                            cdef!(Self => ShrU),
                            vec![
                                Expr::Leaf(a),
                                Expr::Bind(Operator::I32Const { value: bw2 }, vec![]),
                            ],
                        ),
                        waffle_ast::Expr::Bind(
                            cdef!(Self => And),
                            vec![b, lower_mask].into_iter().map(Expr::Leaf).collect(),
                        ),
                    ],
                ),
            ],
        );
        t = e.build(&mut Module::empty(), f, k).unwrap().0;
        e = Expr::Bind(
            cdef!(Self => Add),
            vec![
                Expr::Leaf(low),
                waffle_ast::Expr::Bind(
                    cdef!(Self => Shl),
                    vec![
                        Expr::Bind(
                            cdef!(Self => And),
                            vec![t, lower_mask].into_iter().map(Expr::Leaf).collect(),
                        ),
                        Expr::Bind(Operator::I32Const { value: bw2 }, vec![]),
                    ],
                ),
            ],
        );
        low = e.build(&mut Module::empty(), f, k).unwrap().0;
        e = waffle_ast::Expr::Bind(
            cdef!(Self => ShrU),
            vec![
                Expr::Leaf(t),
                Expr::Bind(Operator::I32Const { value: bw2 }, vec![]),
            ],
        );
        high = e.build(&mut Module::empty(), f, k).unwrap().0;
        e = Expr::Bind(
            cdef!(Self => ShrU),
            vec![
                Expr::Leaf(low),
                Expr::Bind(Operator::I32Const { value: bw2 }, vec![]),
            ],
        );
        t = e.build(&mut Module::empty(), f, k).unwrap().0;
        low = f.add_op(k, cdef!(Self => And), &[low, lower_mask], &[Self::ty()]);
        e = Expr::Bind(
            cdef!(Self => Add),
            vec![
                Expr::Leaf(t),
                waffle_ast::Expr::Bind(
                    cdef!(Self => Mul),
                    vec![
                        waffle_ast::Expr::Bind(
                            cdef!(Self => ShrU),
                            vec![
                                Expr::Leaf(b),
                                Expr::Bind(Operator::I32Const { value: bw2 }, vec![]),
                            ],
                        ),
                        waffle_ast::Expr::Bind(
                            cdef!(Self => And),
                            vec![a, lower_mask].into_iter().map(Expr::Leaf).collect(),
                        ),
                    ],
                ),
            ],
        );
        t = e.build(&mut Module::empty(), f, k).unwrap().0;
        e = Expr::Bind(
            cdef!(Self => Add),
            vec![
                Expr::Leaf(low),
                waffle_ast::Expr::Bind(
                    cdef!(Self => Shl),
                    vec![
                        Expr::Bind(
                            cdef!(Self => And),
                            vec![t, lower_mask].into_iter().map(Expr::Leaf).collect(),
                        ),
                        Expr::Bind(Operator::I32Const { value: bw2 }, vec![]),
                    ],
                ),
            ],
        );
        low = e.build(&mut Module::empty(), f, k).unwrap().0;
        e = Expr::Bind(
            cdef!(Self => Add),
            vec![
                Expr::Leaf(high),
                waffle_ast::Expr::Bind(
                    cdef!(Self => ShrU),
                    vec![
                        Expr::Leaf(t),
                        Expr::Bind(Operator::I32Const { value: bw2 }, vec![]),
                    ],
                ),
            ],
        );
        high = e.build(&mut Module::empty(), f, k).unwrap().0;
        e = Expr::Bind(
            cdef!(Self => Add),
            vec![
                Expr::Leaf(high),
                Expr::Bind(
                    cdef!(Self => Mul),
                    vec![a, b]
                        .into_iter()
                        .map(|v| {
                            waffle_ast::Expr::Bind(
                                cdef!(Self => ShrU),
                                vec![
                                    Expr::Leaf(v),
                                    Expr::Bind(Operator::I32Const { value: bw2 }, vec![]),
                                ],
                            )
                        })
                        .collect(),
                ),
            ],
        );
        high = e.build(&mut Module::empty(), f, k).unwrap().0;
        return (high, low);
    }
}
impl<T: Cfg + ?Sized> CfgExt for T {}
pub trait TRegs {
    const N: usize;
    fn iter(&self) -> impl Iterator<Item = Value>;
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Value>;
}
pub trait TRegsExt: TRegs {
    fn ctx(f: &FunctionBody, entry: Block) -> impl Iterator<Item = Value> {
        return f.blocks[f.entry].params[(Self::N + 1)..]
            .iter()
            .map(|a| a.1);
    }
}
impl<T: TRegs + ?Sized> TRegsExt for T {}
#[macro_export]
macro_rules! cdef {
    ($c:ident => $a:ident $($x:tt)*) => {
        $crate::paste::paste!{
            if $c::MEMORY64{
                $crate::Operator::[<I64 $a>]$($x)*
            }else{
                $crate::Operator::[<I32 $a>]$($x)*
            }
        }
    };
}
pub struct ArchRes {
    pub insts: BTreeMap<u64, Block>,
    pub shim: Block,
}
pub trait Arch {
    type Regs: TRegs;
    fn go<C: Cfg, H: Hook<Self::Regs>>(
        f: &mut FunctionBody,
        entry: Block,
        code: &[u8],
        root_pc: u64,
        funcs: &Funcs,
        module: &mut Module,
        hook: &mut H,
    ) -> ArchRes;
}
