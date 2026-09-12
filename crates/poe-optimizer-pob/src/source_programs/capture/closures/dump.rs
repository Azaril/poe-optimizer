//! Bounded decoder for the attested LuaJIT v2 dump; it never loads bytecode.
use super::*;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Local {
    pub name: String,
    pub hidden: bool,
    pub start: u32,
    pub end: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Proto {
    pub parameters: u8,
    pub frame: u8,
    pub variadic: bool,
    pub first: u32,
    pub last: u32,
    pub instructions: Vec<u32>,
    pub lines: Vec<u32>,
    pub descriptors: Vec<u16>,
    pub names: Vec<String>,
    pub locals: Vec<Local>,
    /// Negative constant index (-1-D) -> earlier decoded child record.
    pub children: BTreeMap<i32, usize>,
    pub gc_constants: u32,
    pub numeric_constants: u32,
    pub sha256: String,
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(n)
            .ok_or_else(|| error("closure dump offset overflow"))?;
        let value = self
            .bytes
            .get(self.at..end)
            .ok_or_else(|| error("truncated closure dump"))?;
        self.at = end;
        Ok(value)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn uint(&mut self, bits: u32) -> Result<u64> {
        let mut value = 0u64;
        for shift in (0..bits).step_by(7) {
            let byte = self.byte()?;
            value |= u64::from(byte & 127) << shift;
            if byte & 128 == 0 {
                if value >> bits != 0 || shift > 0 && byte == 0 {
                    return Err(error("invalid closure dump integer"));
                }
                return Ok(value);
            }
        }
        Err(error("closure dump integer overflow"))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(self.uint(32)? as u32)
    }
    fn name(&mut self) -> Result<String> {
        let length = self.bytes[self.at..]
            .iter()
            .position(|b| *b == 0)
            .ok_or_else(|| error("unterminated closure debug name"))?;
        if length > 4096 {
            return Err(error("closure debug name bound"));
        }
        let value = String::from_utf8(self.take(length)?.to_vec()).map_err(error)?;
        self.byte()?;
        Ok(value)
    }
    fn table_value(&mut self) -> Result<()> {
        match self.u32()? {
            0..=2 => {}
            3 => {
                self.u32()?;
            }
            4 => {
                self.u32()?;
                self.u32()?;
            }
            n => {
                self.take((n - 5) as usize)?;
            }
        }
        Ok(())
    }
}
pub(super) fn decode(bytes: &[u8]) -> Result<Vec<Proto>> {
    if bytes.len() > MAX_DUMP {
        return Err(error("closure dump byte bound"));
    }
    let mut r = Reader { bytes, at: 0 };
    if r.take(4)? != b"\x1bLJ\x02" {
        return Err(error("closure dump magic/version mismatch"));
    }
    let flags = r.u32()?;
    if !matches!(flags, 8 | 24) {
        return Err(error(format!(
            "closure dump profile/strip/flags mismatch: {flags:#x}"
        )));
    }
    let name_length = r.u32()? as usize;
    if name_length > 4096 {
        return Err(error("closure dump source name bound"));
    }
    r.take(name_length)?;
    let mut protos = Vec::new();
    let mut stack = Vec::new();
    let mut words = 0usize;
    loop {
        let length = r.u32()? as usize;
        if length == 0 {
            break;
        }
        if protos.len() >= MAX_CALLBACKS {
            return Err(error("closure prototype count bound"));
        }
        let bytes = r.take(length)?;
        let mut p = Reader { bytes, at: 0 };
        let flags = p.byte()?;
        if flags & !0x83 != 0 {
            return Err(error("closure prototype unsupported flags"));
        }
        let parameters = p.byte()?;
        let frame = p.byte()?;
        let upvalues = p.byte()? as usize;
        if parameters > frame || upvalues > 128 {
            return Err(error("closure prototype frame/capture bound"));
        }
        let gc_constants = p.u32()?;
        let numeric_constants = p.u32()?;
        let count = p.u32()? as usize;
        words = words
            .checked_add(count)
            .ok_or_else(|| error("closure bytecode count overflow"))?;
        if count > 65_535
            || words > MAX_VALUES
            || gc_constants as usize > MAX_VALUES
            || numeric_constants as usize > MAX_VALUES
        {
            return Err(error("closure bytecode/constant bound"));
        }
        let debug_length = p.u32()? as usize;
        if debug_length == 0 {
            return Err(error(
                "closure creation requires complete local debug metadata",
            ));
        }
        let first = p.u32()?;
        let lines_count = p.u32()?;
        let last = first
            .checked_add(lines_count)
            .ok_or_else(|| error("closure line overflow"))?;
        let mut instructions = Vec::with_capacity(count);
        for word in p.take(count * 4)?.as_chunks::<4>().0 {
            instructions.push(u32::from_le_bytes(*word));
        }
        if instructions
            .iter()
            .any(|word| matches!(word & 255, 89..=95))
        {
            return Err(error("closure bit-op bytecode frontier"));
        }
        let mut descriptors = Vec::with_capacity(upvalues);
        for word in p.take(upvalues * 2)?.as_chunks::<2>().0 {
            descriptors.push(u16::from_le_bytes(*word));
        }
        let mut children = BTreeMap::new();
        for index in 0..gc_constants {
            match p.u32()? {
                0 => {
                    let child = stack
                        .pop()
                        .ok_or_else(|| error("closure child stack underflow"))?;
                    children.insert(index as i32 - gc_constants as i32, child);
                }
                1 => {
                    let array = p.u32()? as usize;
                    let hash = p.u32()? as usize;
                    let rows = array
                        .checked_add(
                            hash.checked_mul(2)
                                .ok_or_else(|| error("constant table overflow"))?,
                        )
                        .ok_or_else(|| error("constant table overflow"))?;
                    if rows > MAX_VALUES {
                        return Err(error("constant table bound"));
                    }
                    for _ in 0..rows {
                        p.table_value()?;
                    }
                }
                2..=4 => return Err(error("closure cdata constant frontier")),
                n => {
                    p.take((n - 5) as usize)?;
                }
            }
        }
        for _ in 0..numeric_constants {
            if p.uint(33)? & 1 != 0 {
                p.u32()?;
            }
        }
        let mut debug = Reader {
            bytes: p.take(debug_length)?,
            at: 0,
        };
        let width = if lines_count < 256 {
            1
        } else if lines_count < 65_536 {
            2
        } else {
            4
        };
        let mut lines = Vec::with_capacity(count);
        for row in debug.take(count * width)?.chunks_exact(width) {
            let mut word = [0; 4];
            word[..width].copy_from_slice(row);
            let offset = u32::from_le_bytes(word);
            if offset > lines_count {
                return Err(error("closure instruction line outside prototype"));
            }
            lines.push(first + offset);
        }
        let names = (0..upvalues)
            .map(|_| debug.name())
            .collect::<Result<Vec<_>>>()?;
        let mut locals = Vec::new();
        let mut previous = 0u32;
        loop {
            let tag = *debug
                .bytes
                .get(debug.at)
                .ok_or_else(|| error("missing closure local terminator"))?;
            if tag == 0 {
                debug.byte()?;
                break;
            }
            let hidden = tag < 7;
            let name = if hidden {
                debug.byte()?;
                format!("@hidden:{tag}")
            } else {
                debug.name()?
            };
            let start = previous
                .checked_add(debug.u32()?)
                .ok_or_else(|| error("closure local start overflow"))?;
            let end = start
                .checked_add(debug.u32()?)
                .ok_or_else(|| error("closure local end overflow"))?;
            if start > end || end as usize > count + 1 || locals.len() >= MAX_VALUES {
                return Err(error("closure local lifetime bound"));
            }
            previous = start;
            locals.push(Local {
                name,
                hidden,
                start,
                end,
            });
        }
        if debug.at != debug.bytes.len()
            || p.at != p.bytes.len()
            || (flags & 1 != 0) != !children.is_empty()
        {
            return Err(error("unconsumed/inconsistent closure prototype metadata"));
        }
        // In this pinned fork PROTO_BITOP aliases the top runtime closure-count
        // bit. After proving no bit-op instruction exists, normalize that counter
        // bit; normalized loop words already come from the trusted dump writer.
        let mut digest = Sha256::new();
        digest.update(b"poe-source-prototype-v1\0");
        digest.update([flags & 3]);
        digest.update(&bytes[1..]);
        let sha256 = format!("{:x}", digest.finalize());
        protos.push(Proto {
            parameters,
            frame,
            variadic: flags & 2 != 0,
            first,
            last,
            instructions,
            lines,
            descriptors,
            names,
            locals,
            children,
            gc_constants,
            numeric_constants,
            sha256,
        });
        stack.push(protos.len() - 1);
    }
    if r.at != bytes.len() || stack.len() != 1 || stack[0] + 1 != protos.len() {
        return Err(error("closure dump root/consumption mismatch"));
    }
    Ok(protos)
}

pub(super) fn write(lua: &Lua, function: &Function, limit: usize) -> Result<Vec<u8>> {
    use std::ffi::{c_int, c_void};
    struct Buffer {
        bytes: Vec<u8>,
        limit: usize,
        failed: bool,
    }
    unsafe extern "C-unwind" fn writer(
        _: *mut mlua::ffi::lua_State,
        bytes: *const c_void,
        count: usize,
        data: *mut c_void,
    ) -> c_int {
        // No panic/unwrap/unwind across the C callback; reserve before extending.
        if data.is_null() {
            return 1;
        }
        if count == 0 {
            return 0;
        }
        if bytes.is_null() {
            return 1;
        }
        let buffer = unsafe { &mut *data.cast::<Buffer>() };
        if count > buffer.limit.saturating_sub(buffer.bytes.len())
            || buffer.bytes.try_reserve(count).is_err()
        {
            buffer.failed = true;
            return 1;
        }
        buffer
            .bytes
            .extend_from_slice(unsafe { std::slice::from_raw_parts(bytes.cast::<u8>(), count) });
        0
    }
    let mut buffer = Buffer {
        bytes: Vec::new(),
        limit: limit.min(MAX_DUMP),
        failed: false,
    };
    let mut status = -1;
    // mlua roots the function and guards/restores the stack. lua_dump's own
    // protected writer never executes the function or creates child closures.
    unsafe {
        lua.exec_raw::<()>(function.clone(), |state| {
            status = mlua::ffi::lua_dump(state, writer, (&mut buffer as *mut Buffer).cast(), 0);
            mlua::ffi::lua_settop(state, 0);
        })?;
    }
    if status != 0 || buffer.failed {
        return Err(error("bounded closure dump failed"));
    }
    Ok(buffer.bytes)
}
