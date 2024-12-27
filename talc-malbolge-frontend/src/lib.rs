use id_arena::{Arena, Id};
pub mod ops;
pub mod impls;
pub mod meld;
pub struct Cfg{
    pub values: Arena<Value>,
    pub blocks: Arena<Block>
}
pub struct Func{
    pub cfg: Cfg,
    pub entry: Id<Block>,
}
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum Value{
    BlockParam(Id<Block>,usize),
    Load(Id<Value>),
    Store(Id<Value>,Id<Value>),
    Add(Id<Value>,Id<Value>),
    Mod94(Id<Value>),
    Ror1(Id<Value>),
    Add1(Id<Value>),
    Crazy(Id<Value>,Id<Value>),
    Encrypt(Id<Value>),
    Print(Id<Value>),
    Input,
    Const(usize)
}
#[derive(Default)]
pub struct Block{
    pub params: Vec<Id<Value>>,
    pub insts: Vec<Id<Value>>,
    pub term: Term
}
#[derive(Default)]
pub enum Term{
    SwitchMod{
        val: Id<Value>,
        targets: Vec<Target>,
    },
    Jmp{
        target: Target
    },
    #[default]
    None,
}
#[derive(Clone)]
pub struct Target{
    pub block: Id<Block>,
    pub params: Vec<Id<Value>>
}
impl Cfg{
    pub fn add_blockparam(&mut self, k: Id<Block>) -> Id<Value>{
        let p = self.values.alloc(Value::BlockParam(k, self.blocks[k].params.len()));
        self.blocks[k].params.push(p);
        return p;
    }
    pub fn append_to_block(&mut self, k: Id<Block>, v: Value) -> Id<Value>{
        let r = self.values.alloc(v);
        self.blocks[k].insts.push(r);
        return r;
    }
}
pub fn go(cfg: &mut Cfg) -> Id<Block>{
    let shim = cfg.blocks.alloc(Default::default());
    let c = cfg.add_blockparam(shim);
    let a = cfg.add_blockparam(shim);
    let d = cfg.add_blockparam(shim);

    let arr_c = cfg.append_to_block(shim, Value::Load(c));
    let arr_c = cfg.append_to_block(shim, Value::Add(c, arr_c));
    let encc = cfg.append_to_block(shim, Value::Encrypt(arr_c));
    let cao = cfg.append_to_block(shim, Value::Add1(c));
    let dao = cfg.append_to_block(shim, Value::Add1(d));
    let bs = Target{block: shim, params: vec![cao,a,dao]};
    let nop = cfg.blocks.alloc(Default::default());
    cfg.blocks[nop].term = Term::Jmp { target: bs.clone() };
    cfg.append_to_block(nop, Value::Store(c, encc));
    let mut targets = vec![Target{block: nop, params: vec![]}; 94];
    let dl = cfg.append_to_block(shim, Value::Load(d));
    let i4 = cfg.blocks.alloc(Default::default());
    let enc = cfg.append_to_block(shim, Value::Encrypt(dl));
    cfg.append_to_block(i4, Value::Store(dl, enc));
    let dlao = cfg.append_to_block(shim, Value::Add1(dl));
    cfg.blocks[i4].term = Term::Jmp { target: Target { block: shim, params: vec![dlao,a,dao] } };
    targets[4] = Target{block: i4, params: vec![]};
    let i5 = cfg.blocks.alloc(Default::default());
    cfg.blocks[i5].term = Term::Jmp { target: bs.clone() };
    cfg.append_to_block(i5, Value::Store(c, encc));
    cfg.append_to_block(i5, Value::Print(a));
    targets[5] = Target{block: i5, params: vec![]};
    let i23 = cfg.blocks.alloc(Default::default());
    let i = cfg.append_to_block(i23, Value::Input);
    cfg.blocks[i23].term = Term::Jmp { target: Target { block: shim, params: vec![cao,i,dao] } };
    cfg.append_to_block(i23, Value::Store(c, encc));
    targets[23] = Target{block: i23, params: vec![]};
    for (i,op) in [(39,(|_,d|Value::Ror1(d)) as fn(Id<Value>,Id<Value>) -> Value),(62,Value::Crazy)]{
        let k = cfg.blocks.alloc(Default::default());
        let j = op(a,dl);
        let j = cfg.append_to_block(shim, j);
        cfg.append_to_block(k, Value::Store(d, j));
        cfg.blocks[k].term = Term::Jmp { target: Target { block: shim, params: vec![cao,j,dao] } };
        targets[i] = Target{block: k, params: vec![]};
    }
    let i40 = cfg.blocks.alloc(Default::default());
    cfg.blocks[i40].term = Term::Jmp { target: Target { block: shim, params: vec![cao,i,dlao] } };
    cfg.append_to_block(i40, Value::Store(c, encc));
    targets[40] = Target{block: i40, params: vec![]};

    cfg.blocks[shim].term = Term::SwitchMod { val: arr_c, targets };
    return shim;
}