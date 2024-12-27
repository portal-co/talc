use std::collections::BTreeMap;

use id_arena::Id;

use crate::{
    ops::{crz_op, enc, rotr},
    Block, Cfg, Func, Target, Term, Value,
};

pub struct Melder {
    pub blocks: BTreeMap<Id<Block>, BTreeMap<Vec<Option<usize>>, Id<Block>>>,
}
impl Melder {
    pub fn transform(
        &mut self,
        src: &Cfg,
        dst: &mut Cfg,
        k: Id<Block>,
        args: Vec<Option<usize>>,
    ) -> Id<Block> {
        loop {
            if let Some(a) = self.blocks.get(&k).and_then(|x| x.get(&args)) {
                return *a;
            }
            let b = dst.blocks.alloc(Default::default());
            self.blocks.entry(k).or_default().insert(args.clone(), b);
            let mut k = k;
            let mut args = args.clone();
            let mut state = src.blocks[k]
                .params
                .iter()
                .cloned()
                .zip(args.iter().cloned())
                .map(|(a, v)| {
                    (
                        a,
                        match v {
                            None => dst.add_blockparam(b),
                            Some(a) => dst.append_to_block(b, Value::Const(a)),
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>();
            let mut load_map: BTreeMap<Id<Value>, Id<Value>> = BTreeMap::new();
            'a: loop {
                for i in src.blocks[k].insts.iter().cloned() {
                    let v = match &src.values[i] {
                        Value::BlockParam(id, _) => todo!(),
                        Value::Load(id) => {
                            let v = state.get(id).cloned().unwrap();
                            match load_map.get(&v) {
                                None => dst.append_to_block(b, Value::Load(v)),
                                Some(w) => *w,
                            }
                        }
                        Value::Store(id, id1) => {
                            let v = state.get(id).cloned().unwrap();
                            let w = state.get(id1).cloned().unwrap();
                            load_map.insert(v, w);
                            dst.append_to_block(b, Value::Store(v, w))
                        }
                        Value::Add(id, id1) => {
                            let v = state.get(id).cloned().unwrap();
                            let w = state.get(id1).cloned().unwrap();
                            let w = match (&dst.values[v], &dst.values[w]) {
                                (Value::Const(k), Value::Const(l)) => {
                                    Value::Const((*k + *l) % 59049)
                                }
                                _ => Value::Add(v, w),
                            };
                            dst.append_to_block(b, w)
                        }
                        Value::Mod94(id) => {
                            let v = state.get(id).cloned().unwrap();
                            let w = match &dst.values[v] {
                                Value::Const(k) => Value::Const(*k % 94),
                                _ => Value::Ror1(v),
                            };
                            dst.append_to_block(b, w)
                        }
                        Value::Ror1(id) => {
                            let v = state.get(id).cloned().unwrap();
                            let w = match &dst.values[v] {
                                Value::Const(k) => Value::Const(rotr(*k)),
                                _ => Value::Ror1(v),
                            };
                            dst.append_to_block(b, w)
                        }
                        Value::Add1(id) => {
                            let v = state.get(id).cloned().unwrap();
                            let w = match &dst.values[v] {
                                Value::Const(k) => Value::Const((*k + 1) % 59049),
                                _ => Value::Ror1(v),
                            };
                            dst.append_to_block(b, w)
                        }
                        Value::Crazy(id, id1) => {
                            let v = state.get(id).cloned().unwrap();
                            let w = state.get(id1).cloned().unwrap();
                            let w = match (&dst.values[v], &dst.values[w]) {
                                (Value::Const(k), Value::Const(l)) => Value::Const(crz_op(*k, *l)),
                                _ => Value::Crazy(v, w),
                            };
                            dst.append_to_block(b, w)
                        }
                        Value::Encrypt(id) => {
                            let v = state.get(id).cloned().unwrap();
                            let w = match &dst.values[v] {
                                Value::Const(k) => Value::Const(enc(*k)),
                                _ => Value::Encrypt(v),
                            };
                            dst.append_to_block(b, w)
                        }
                        Value::Print(id) => {
                            dst.append_to_block(b, Value::Print(state.get(id).cloned().unwrap()))
                        }
                        Value::Input => dst.append_to_block(b, Value::Input),
                        Value::Const(k) => dst.append_to_block(b, Value::Const(*k)),
                    };
                    let v = match &dst.values[v] {
                        Value::Const(k) => dst.blocks[b]
                            .insts
                            .iter()
                            .cloned()
                            .find(|i| dst.values[*i] == Value::Const(*k))
                            .unwrap(),
                        _ => v,
                    };
                    state.insert(i, v);
                }
                let term = match &src.blocks[k].term {
                    crate::Term::SwitchMod { val, targets } => {
                        let val = state.get(val).cloned().unwrap();
                        match &dst.values[val] {
                            Value::Const(kn) => {
                                let target = &targets[(*kn) % targets.len()];
                                k = target.block;
                                state = src.blocks[k]
                                    .params
                                    .iter()
                                    .cloned()
                                    .zip(target.params.iter().filter_map(|v| state.get(v).cloned()))
                                    // .zip(args.iter().cloned())
                                    .map(|(a, b)| (a, b))
                                    .collect::<BTreeMap<_, _>>();
                                continue 'a;
                            }
                            _ => Term::SwitchMod {
                                val,
                                targets: targets
                                    .iter()
                                    .map(|t| {
                                        let mut params = vec![];
                                        let mut args = vec![];
                                        for p in
                                            t.params.iter().filter_map(|x| state.get(x)).cloned()
                                        {
                                            match &dst.values[p] {
                                                Value::Const(k) => {
                                                    args.push(Some(*k));
                                                }
                                                _ => {
                                                    params.push(p);
                                                    args.push(None);
                                                }
                                            }
                                        }
                                        Target {
                                            block: self.transform(src, dst, t.block, args),
                                            params,
                                        }
                                    })
                                    .collect(),
                            },
                        }
                    }
                    crate::Term::Jmp { target } => {
                        k = target.block;
                        state = src.blocks[k]
                            .params
                            .iter()
                            .cloned()
                            .zip(target.params.iter().filter_map(|v| state.get(v).cloned()))
                            // .zip(args.iter().cloned())
                            .map(|(a, b)| (a, b))
                            .collect::<BTreeMap<_, _>>();
                        continue 'a;
                    }
                    crate::Term::None => Term::None,
                };
                dst.blocks[b].term = term;
                break 'a;
            }
        }
    }
}
