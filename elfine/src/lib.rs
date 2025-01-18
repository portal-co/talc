use std::{collections::BTreeMap, iter::once};

use anyhow::Context;
use goblin::elf::{
    program_header::{PF_R, PF_W, PF_X, PT_LOAD},
    Elf,
};
use talc_common::Input;

pub fn lineup(a: &[u8], i: &mut Input) -> anyhow::Result<Res> {
    let mut exports = BTreeMap::new();
    let e = Elf::parse(a).context("in parsing the elf")?;
    let entry = e.entry;
    let lowest = e
        .program_headers
        .iter()
        .filter(|a| a.p_type == PT_LOAD)
        .map(|a| a.vm_range().start)
        .min()
        .unwrap();

    for s in e.program_headers.iter() {
        if s.p_type == PT_LOAD {
            i.expand(s.vm_range().end - lowest);
            let sc = &a[(s.p_offset as usize..)][..(s.p_filesz as usize)];
            i.code[(s.vm_range().start - lowest)..(s.vm_range().end - lowest)].copy_from_slice(sc);
            let r = s.p_flags & PF_R != 0;
            let w = s.p_flags & PF_W != 0;
            let x = s.p_flags & PF_X != 0;
            for (r, v) in [(&mut i.r, r), (&mut i.w, w), (&mut i.x, x)] {
                for l in s.vm_range() {
                    r.set(l - lowest, v);
                }
            }
        }
    }
    Ok(Res {
        exports,
        entry,
        root_pc: lowest as u64,
    })
}
pub struct Res {
    pub exports: BTreeMap<String, usize>,
    pub entry: u64,
    pub root_pc: u64,
}
