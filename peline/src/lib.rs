use std::collections::BTreeMap;

use anyhow::Context;
use goblin::pe::symbol::SymbolTable;

pub fn peline(src: &[u8], buf: &mut Vec<u8>, ld: &mut impl Loader) -> anyhow::Result<Res> {
    let pe = goblin::pe::PE::parse(src)?;
    let sym = SymbolTable::parse(
        src,
        pe.header.coff_header.pointer_to_symbol_table as usize,
        pe.header.coff_header.number_of_symbol_table as usize,
    )?;
    let mut ss = BTreeMap::new();
    // let mut ssi = vec![];
    let mut rvas = BTreeMap::new();
    for (i, s) in pe.sections.iter().enumerate() {
        // let i = s.name;
        ss.insert(i, buf.len());
        // ssi.push(i);
        rvas.insert(s.virtual_address as usize, (i, s.virtual_size));
        buf.extend_from_slice(
            &src[(s.pointer_to_raw_data as usize)..][..(s.virtual_size as usize)],
        );
    }
    for (si, s) in pe.sections.iter().enumerate() {
        let r = s.relocations(src)?;
        for (i, r) in r.enumerate() {
            let addrv = r.virtual_address.wrapping_sub(s.virtual_address) as usize;
            let addr = addrv + ss.get(&si).unwrap().clone();
            let (symval, s) = sym
                .iter()
                .find_map(|(i, n, s)| {
                    if i != r.symbol_table_index as usize {
                        return None;
                    }
                    let va = *ss.get(&(s.section_number as usize))?;
                    let va = va + (s.value as usize);
                    Some((va, s))
                })
                .context(format!(
                    "in relocating symbol {} in relocation {} for section {}",
                    r.symbol_table_index, i, si
                ))?;
            use goblin::pe::relocation::*;
            match (r.typ,ld._64bit()){
                (IMAGE_REL_I386_DIR32,false) => {
                    buf[addr..][..4].copy_from_slice(&u32::to_le_bytes((symval & 0xffffffff) as u32));
                }
                (IMAGE_REL_AMD64_ADDR32,true) => {
                    buf[addr..][..4].copy_from_slice(&u32::to_le_bytes((symval & 0xffffffff) as u32));
                }
                (IMAGE_REL_AMD64_ADDR64,true) => {
                    buf[addr..][..8].copy_from_slice(&u64::to_le_bytes(symval as u64));
                }
                (IMAGE_REL_I386_DIR32NB,false) => {
                    buf[addr..][..4].copy_from_slice(&u32::to_le_bytes(s.value));
                }
                (IMAGE_REL_I386_REL32,false) => {
                    buf[addr..][..4].copy_from_slice(&u32::to_le_bytes(((symval & 0xffffffff) as u32).wrapping_sub((addr & 0xffffffff) as u32)));
                }
                _ => anyhow::bail!("invalid relocation type {} in relocating symbol {} in relocation {} for section {}",r.typ,r.symbol_table_index,i,si)
            }
        }
    }
    for i in pe.imports.iter() {
        let (sid, size) = rvas
            .get(&i.rva)
            .cloned()
            .context(format!("in gettting rva for {}/{}", i.dll, i.name))?;
        let soff = ss.get(&sid).cloned().context(format!(
            "in gettting section offset for {}/{}",
            i.dll, i.name
        ))?;
        let ld = ld
            .load(i.dll, buf)
            .context(format!("in resolving {}", i.dll))?;
        let sym = ld
            .exports
            .get(i.name.as_ref())
            .cloned()
            .context(format!("in getting {} from dll {}", i.name, i.dll))?;
        let s = buf[sym..][..(i.size)].to_vec();
        buf[soff..][..(i.size)].copy_from_slice(&s);
    }
    Ok(Res { exports: todo!() })
}
pub struct Res {
    pub exports: BTreeMap<String, usize>,
}
pub trait Loader {
    fn _64bit(&mut self) -> bool;
    fn load(&mut self, x: &str, buf: &mut Vec<u8>) -> anyhow::Result<Res>;
}
