use talc_common::{Arch, Cfg, Funcs, TRegs};
use typenum::Same;
use waffle::{Block, FunctionBody, Module};

pub trait Felf: Arch {
    fn felf1<C: Cfg>(
        f: &mut FunctionBody,
        entry: Block,
        mut code: &[u8],
        funcs: &Funcs,
        module: &mut Module,
    ) -> Option<Block> {
        code = code.strip_prefix(b"FELF0001")?;
        let base;
        (base, code) = code.split_at_checked(16)?;
        let entry2 = u64::from_le_bytes(base[..8].try_into().unwrap());
        let base = u64::from_le_bytes(base[8..].try_into().unwrap());
        let v = Self::go::<C>(f, entry, code, base, funcs, module);
        return v.insts.get(&entry2).cloned();
    }
    fn felf2<C: Cfg>(
        f: &mut FunctionBody,
        entry: Block,
        mut code: &[u8],
        funcs: &Funcs,
        module: &mut Module,
    ) -> Option<Block> {
        code = code.strip_prefix(b"FELF0002")?;
        let base;
        (base, code) = code.split_at_checked(16)?;
        let entry2 = u64::from_le_bytes(base[..8].try_into().unwrap());
        let base = u64::from_le_bytes(base[8..].try_into().unwrap());
        let v = Self::go::<C>(f, entry, code, base, funcs, module);
        return v.insts.get(&entry2).cloned();
    }
}
impl<T: Arch + ?Sized> Felf for T {}
pub trait FelfFuncs: Same<Output = Funcs> + Sized {
    fn init_felf1<R: TRegs, C: Cfg>(
        &self,
        mut code: &[u8],
        f: &mut FunctionBody,
        mut k: Block,
        entry: Block,
        module: &mut Module,
    ) -> Option<Block> {
        code = code.strip_prefix(b"FELF0001")?;
        let base;
        (base, code) = code.split_at_checked(16)?;
        let entry2 = u64::from_le_bytes(base[..8].try_into().unwrap());
        let base = u64::from_le_bytes(base[8..].try_into().unwrap());
        return Some(Funcs::init::<R, C>(
            //SAFETY: same type
            unsafe { std::mem::transmute(self) },
            code,
            base,
            f,
            k,
            entry,
            module,
        ));
    }
    fn init_felf2<R: TRegs, C: Cfg>(
        &self,
        mut code: &[u8],
        f: &mut FunctionBody,
        mut k: Block,
        entry: Block,
        module: &mut Module,
    ) -> Option<Block> {
        code = code.strip_prefix(b"FELF0002")?;
        let base;
        (base, code) = code.split_at_checked(16)?;
        let entry2 = u64::from_le_bytes(base[..8].try_into().unwrap());
        let base = u64::from_le_bytes(base[8..].try_into().unwrap());
        return Some(Funcs::init::<R, C>(
            //SAFETY: same type
            unsafe { std::mem::transmute(self) },
            code,
            base,
            f,
            k,
            entry,
            module,
        ));
    }
}
impl FelfFuncs for Funcs{

}