use std::iter::{empty, once};

// use arena_traits::Arena;
use id_arena::{Arena, Id};

use crate::{Block, Func, Target, Term, Value};

impl cfg_traits::Func for Func {
    type Block = Id<Block>;

    type Blocks = id_arena::Arena<Block>;

    fn blocks(&self) -> &Self::Blocks {
        &self.cfg.blocks
    }

    fn blocks_mut(&mut self) -> &mut Self::Blocks {
        &mut self.cfg.blocks
    }

    fn entry(&self) -> Self::Block {
        self.entry
    }
}
impl cfg_traits::Block<Func> for Block {
    type Terminator = Term;

    fn term(&self) -> &Self::Terminator {
        &self.term
    }

    fn term_mut(&mut self) -> &mut Self::Terminator {
        &mut self.term
    }
}
impl cfg_traits::Term<Func> for Term {
    type Target = Target;

    fn targets<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Self::Target> + 'a>
    where
        Func: 'a,
    {
        match self {
            Term::SwitchMod { val, targets } => Box::new(targets.iter()),
            Term::Jmp { target } => Box::new(once(target)),
            Term::None => Box::new(empty()),
        }
    }

    fn targets_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &'a mut Self::Target> + 'a>
    where
        Func: 'a,
    {
        match self {
            Term::SwitchMod { val, targets } => Box::new(targets.iter_mut()),
            Term::Jmp { target } => Box::new(once(target)),
            Term::None => Box::new(empty()),
        }
    }
}
impl cfg_traits::Term<Func> for Target {
    type Target = Target;

    fn targets<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Self::Target> + 'a>
    where
        Func: 'a,
    {
        Box::new(once(self))
    }

    fn targets_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &'a mut Self::Target> + 'a>
    where
        Func: 'a,
    {
        Box::new(once(self))
    }
}
impl cfg_traits::Target<Func> for Target {
    fn block(&self) -> <Func as cfg_traits::Func>::Block {
        self.block
    }

    fn block_mut(&mut self) -> &mut <Func as cfg_traits::Func>::Block {
        &mut self.block
    }
}
impl ssa_traits::Func for Func {
    type Value = Id<Value>;

    type Values = Arena<Value>;

    fn values(&self) -> &Self::Values {
        &self.cfg.values
    }

    fn values_mut(&mut self) -> &mut Self::Values {
        &mut self.cfg.values
    }
}
impl ssa_traits::HasValues<Func> for Value {
    fn values<'a>(
        &'a self,
        f: &'a Func,
    ) -> Box<dyn Iterator<Item = <Func as ssa_traits::Func>::Value> + 'a> {
        match self {
            Value::Load(id) => Box::new(once(*id)),
            Value::Store(id, id1) => Box::new([*id, *id1].into_iter()),
            Value::Add(id, id1) => Box::new([*id, *id1].into_iter()),
            Value::Mod94(id) => Box::new(once(*id)),
            Value::Ror1(id) => Box::new(once(*id)),
            Value::Add1(id) => Box::new(once(*id)),
            Value::Crazy(id, id1) => Box::new([*id, *id1].into_iter()),
            Value::Encrypt(id) => Box::new(once(*id)),
            Value::Print(id) => Box::new(once(*id)),
            _ => Box::new(empty()),
        }
    }

    fn values_mut<'a>(
        &'a mut self,
        g: &'a mut Func,
    ) -> Box<dyn Iterator<Item = &'a mut <Func as ssa_traits::Func>::Value> + 'a>
    where
        Func: 'a,
    {
        match self {
            Value::Load(id) => Box::new(once(id)),
            Value::Store(id, id1) => Box::new([id, id1].into_iter()),
            Value::Add(id, id1) => Box::new([id, id1].into_iter()),
            Value::Mod94(id) => Box::new(once(id)),
            Value::Ror1(id) => Box::new(once(id)),
            Value::Add1(id) => Box::new(once(id)),
            Value::Crazy(id, id1) => Box::new([id, id1].into_iter()),
            Value::Encrypt(id) => Box::new(once(id)),
            Value::Print(id) => Box::new(once(id)),
            _ => Box::new(empty()),
        }
    }
}
impl ssa_traits::Block<Func> for Block {
    fn insts(&self) -> impl Iterator<Item = <Func as ssa_traits::Func>::Value> {
        self.insts.iter().cloned()
    }

    fn add_inst(func: &mut Func, key: <Func as cfg_traits::Func>::Block, v: <Func as ssa_traits::Func>::Value) {
        func.cfg.blocks[key].insts.push(v);
    }
}
impl ssa_traits::Value<Func> for Value {}
impl ssa_traits::HasValues<Func> for Target {
    fn values<'a>(
        &'a self,
        f: &'a Func,
    ) -> Box<dyn Iterator<Item = <Func as ssa_traits::Func>::Value> + 'a> {
        Box::new(self.params.iter().cloned())
    }

    fn values_mut<'a>(
        &'a mut self,
        g: &'a mut Func,
    ) -> Box<dyn Iterator<Item = &'a mut <Func as ssa_traits::Func>::Value> + 'a>
    where
        Func: 'a,
    {
        Box::new(self.params.iter_mut())
    }
}
impl ssa_traits::Target<Func> for Target {
    fn push_value(&mut self, v: <Func as ssa_traits::Func>::Value) {
        self.params.push(v);
    }

    fn from_values_and_block(
        a: impl Iterator<Item = <Func as ssa_traits::Func>::Value>,
        k: <Func as cfg_traits::Func>::Block,
    ) -> Self {
        Target {
            block: k,
            params: a.collect(),
        }
    }
}
impl ssa_traits::HasValues<Func> for Term {
    fn values<'a>(
        &'a self,
        f: &'a Func,
    ) -> Box<dyn Iterator<Item = <Func as ssa_traits::Func>::Value> + 'a> {
        match self {
            Term::SwitchMod { val, targets } => {
                Box::new(once(*val).chain(targets.iter().flat_map(|a| a.params.iter()).cloned()))
            }
            Term::Jmp { target } => target.values(f),
            Term::None => Box::new(empty()),
        }
    }

    fn values_mut<'a>(
        &'a mut self,
        g: &'a mut Func,
    ) -> Box<dyn Iterator<Item = &'a mut <Func as ssa_traits::Func>::Value> + 'a>
    where
        Func: 'a,
    {
        match self {
            Term::SwitchMod { val, targets } => {
                Box::new(once(val).chain(targets.iter_mut().flat_map(|a| a.params.iter_mut())))
            }
            Term::Jmp { target } => target.values_mut(g),
            Term::None => Box::new(empty()),
        }
    }
}
impl ssa_traits::TypedFunc for Func{
    type Ty = ();

    fn add_blockparam(&mut self, k: Self::Block, y: Self::Ty) -> Self::Value {
        self.cfg.add_blockparam(k)
    }
}
impl ssa_traits::TypedBlock<Func> for Block{
    fn params(&self) -> impl Iterator<Item = (<Func as ssa_traits::TypedFunc>::Ty, <Func as ssa_traits::Func>::Value)> {
        self.params.iter().cloned().map(|a|((),a))
    }
}
impl ssa_traits::TypedValue<Func> for Value{
    fn ty(&self, f: &Func) -> <Func as ssa_traits::TypedFunc>::Ty {
        ()
    }
}