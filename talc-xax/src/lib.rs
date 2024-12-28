use iced_x86::code_asm::*;

pub fn fi(a: &mut CodeAssembler) -> Result<(), iced_x86::IcedError> {
    a.push(r15)?;
    a.mov(r15, rsp)?;
    Ok(())
}
pub fn fx(a: &mut CodeAssembler) -> Result<(), iced_x86::IcedError> {
    a.xchg(r15, rsp)?;
    a.pop(r15)?;
    Ok(())
}

