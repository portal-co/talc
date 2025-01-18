use talc_common::{bitvec::vec::BitVec, Arch, Cfg, Funcs, Hook, InputRef, TRegs};
use typenum::Same;
use waffle::{Block, FunctionBody, Module};
pub trait ToFelf {
    fn to_felf2(&self, entry: u64, root_pc: u64) -> impl Iterator<Item = u8>;
}
impl<'a> ToFelf for InputRef<'a> {
    fn to_felf2(&self, entry: u64, root_pc: u64) -> impl Iterator<Item = u8> {
        return b"FELF0002".iter().cloned().chain(u64::to_le_bytes(entry).into_iter()).chain(u64::to_le_bytes(root_pc).into_iter()).chain(self.code.iter().cloned()).chain(self.r.iter().zip(self.w.iter()).zip(self.x.iter()).map(|((r,w),x)|if *r{
            0x1
        }else{
            0
        } | if *w{
            0x2
        } else {0} | if *x{0x4} else{0}));
    }
}
pub trait Felf: Arch {
    fn felf1<C: Cfg, H: Hook<Self::Regs>>(
        f: &mut FunctionBody,
        entry: Block,
        mut code: &[u8],
        funcs: &Funcs,
        module: &mut Module,
        hook: &mut H,
    ) -> Option<Block> {
        code = code.strip_prefix(b"FELF0001")?;
        let base;
        (base, code) = code.split_at_checked(16)?;
        let entry2 = u64::from_le_bytes(base[..8].try_into().unwrap());
        let base = u64::from_le_bytes(base[8..].try_into().unwrap());
        let v = Self::go::<C, H>(
            f,
            entry,
            InputRef {
                code,
                r: code.iter().map(|_| true).collect::<BitVec>().as_ref(),
                w: code.iter().map(|_| true).collect::<BitVec>().as_ref(),
                x: code.iter().map(|_| true).collect::<BitVec>().as_ref(),
            },
            base,
            funcs,
            module,
            hook,
        );
        return v.insts.get(&entry2).cloned();
    }
    fn felf2<C: Cfg, H: Hook<Self::Regs>>(
        f: &mut FunctionBody,
        entry: Block,
        mut code: &[u8],
        funcs: &Funcs,
        module: &mut Module,
        hook: &mut H,
    ) -> Option<Block> {
        code = code.strip_prefix(b"FELF0002")?;
        let base;
        (base, code) = code.split_at_checked(16)?;
        let entry2 = u64::from_le_bytes(base[..8].try_into().unwrap());
        let base = u64::from_le_bytes(base[8..].try_into().unwrap());
        let (code, perms) = code.split_at_checked(code.len() / 2)?;
        let v = Self::go::<C, H>(
            f,
            entry,
            InputRef {
                code,
                r: perms
                    .iter()
                    .map(|a| a & 0x1 != 0)
                    .collect::<BitVec>()
                    .as_ref(),
                w: perms
                    .iter()
                    .map(|a| a & 0x2 != 0)
                    .collect::<BitVec>()
                    .as_ref(),
                x: perms
                    .iter()
                    .map(|a| a & 0x4 != 0)
                    .collect::<BitVec>()
                    .as_ref(),
            },
            base,
            funcs,
            module,
            hook,
        );
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
            InputRef {
                code,
                r: code.iter().map(|_| true).collect::<BitVec>().as_ref(),
                w: code.iter().map(|_| true).collect::<BitVec>().as_ref(),
                x: code.iter().map(|_| true).collect::<BitVec>().as_ref(),
            },
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
        let (code, perms) = code.split_at_checked(code.len() / 2)?;
        return Some(Funcs::init::<R, C>(
            //SAFETY: same type
            unsafe { std::mem::transmute(self) },
            InputRef {
                code,
                r: perms
                    .iter()
                    .map(|a| a & 0x1 != 0)
                    .collect::<BitVec>()
                    .as_ref(),
                w: perms
                    .iter()
                    .map(|a| a & 0x2 != 0)
                    .collect::<BitVec>()
                    .as_ref(),
                x: perms
                    .iter()
                    .map(|a| a & 0x4 != 0)
                    .collect::<BitVec>()
                    .as_ref(),
            },
            base,
            f,
            k,
            entry,
            module,
        ));
    }
}
impl FelfFuncs for Funcs {}
