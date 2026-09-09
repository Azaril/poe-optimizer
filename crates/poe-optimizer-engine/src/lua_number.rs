//! Byte-string `tonumber` conversion for the pinned LuaJIT runtime.
//!
//! The scanner accepts LuaJIT's decimal, hexadecimal, binary, and nonfinite
//! spellings, preserving signed zero and rejecting embedded NUL. It performs no
//! allocation and uses time proportional to the input length. Callers bound the
//! input before invoking it. This is the one-argument string operation; explicit
//! bases, Lua coercion of other value kinds, and FFI numeric suffixes are separate.

const MAX_EXPONENT: u32 = 1 << 20;
const SIGN: u64 = 1 << 63;
const INFINITY: u64 = 0x7ff0_0000_0000_0000;

/// Convert a byte string with LuaJIT's one-argument `tonumber` syntax.
///
/// `None` is Lua nil. Nonfinite values are returned as IEEE numbers; NaN uses
/// LuaJIT's canonical negative quiet NaN independently of the authored sign.
pub fn parse_number(text: &[u8]) -> Option<f64> {
    let first = text.iter().position(|b| !is_space(*b))?;
    let last = text.iter().rposition(|b| !is_space(*b))? + 1;
    let text = &text[first..last];
    let negative = text.first() == Some(&b'-');
    let body = if matches!(text.first(), Some(b'+' | b'-')) {
        &text[1..]
    } else {
        text
    };
    if body.eq_ignore_ascii_case(b"inf") || body.eq_ignore_ascii_case(b"infinity") {
        return Some(f64::from_bits(INFINITY | if negative { SIGN } else { 0 }));
    }
    if body.eq_ignore_ascii_case(b"nan") {
        return Some(f64::from_bits(0xfff8_0000_0000_0000));
    }
    if matches!(body.get(..2), Some(b"0x" | b"0X")) {
        let scanned = scan(&body[2..], 16)?;
        return Some(hex_number(scanned, negative));
    }
    if matches!(body.get(..2), Some(b"0b" | b"0B")) {
        let bits = &body[2..];
        if bits.is_empty() || bits.iter().any(|b| !matches!(b, b'0' | b'1')) {
            return None;
        }
        let first = bits.iter().position(|b| *b != b'0').unwrap_or(bits.len());
        let bits = &bits[first..];
        if bits.len() > 64 {
            return None;
        }
        let integer = bits
            .iter()
            .fold(0u64, |n, b| (n << 1) | u64::from(b - b'0'));
        let number = integer as f64;
        return Some(if negative { -number } else { number });
    }
    scan(body, 10)?;
    // The syntax check above guarantees ASCII. Rust's decimal parser supplies
    // correctly rounded conversion, including subnormal and overflow cases.
    std::str::from_utf8(text).ok()?.parse().ok()
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 11 | 12)
}

struct Scanned<'a> {
    mantissa: &'a [u8],
    exponent: i64,
    fraction_digits: usize,
}

fn scan(body: &[u8], base: u8) -> Option<Scanned<'_>> {
    let mut at = 0;
    let mut dot = None;
    let mut any_digit = false;
    let mut last_nonzero = None;
    while let Some(&byte) = body.get(at) {
        if if base == 16 {
            byte.is_ascii_hexdigit()
        } else {
            byte.is_ascii_digit()
        } {
            any_digit = true;
            if byte != b'0' {
                last_nonzero = Some(at);
            }
        } else if byte == b'.' && dot.is_none() {
            dot = Some(at);
        } else {
            break;
        }
        at += 1;
    }
    if !any_digit {
        return None;
    }
    let mantissa = &body[..at];
    let fraction_digits = dot.map_or(0, |dot| at - dot - 1);
    // Source discounts trailing zeros before checking this bound, and entirely
    // zero mantissas have no fractional exponent to check.
    if let (Some(dot), Some(last)) = (dot, last_nonzero)
        && last > dot
        && last - dot >= MAX_EXPONENT as usize
    {
        return None;
    }
    let mut exponent = 0i64;
    if body
        .get(at)
        .is_some_and(|b| b.to_ascii_lowercase() == if base == 16 { b'p' } else { b'e' })
    {
        at += 1;
        let negative = body.get(at) == Some(&b'-');
        if matches!(body.get(at), Some(b'+' | b'-')) {
            at += 1;
        }
        let start = at;
        let mut value = 0u32;
        while let Some(&b) = body.get(at).filter(|b| b.is_ascii_digit()) {
            value = value * 10 + u32::from(b - b'0');
            if value >= MAX_EXPONENT {
                return None;
            }
            at += 1;
        }
        if at == start {
            return None;
        }
        exponent = if negative {
            -i64::from(value)
        } else {
            i64::from(value)
        };
    }
    if at != body.len() {
        return None;
    }
    Some(Scanned {
        mantissa,
        exponent,
        fraction_digits,
    })
}

fn hex_digit(byte: u8) -> u8 {
    if byte <= b'9' {
        byte - b'0'
    } else {
        (byte | 32) - b'a' + 10
    }
}

fn hex_number(scanned: Scanned<'_>, negative: bool) -> f64 {
    let sign = if negative { SIGN } else { 0 };
    let Some(first) = scanned
        .mantissa
        .iter()
        .position(|b| !matches!(b, b'0' | b'.'))
    else {
        return f64::from_bits(sign);
    };
    let high = 7 - hex_digit(scanned.mantissa[first]).leading_zeros() as i64;
    let tail_digits = scanned.mantissa[first + 1..]
        .iter()
        .filter(|b| **b != b'.')
        .count();
    // The difference is bounded by the slice length. i128 prevents overflow
    // even for arbitrary addressable byte slices on a 64-bit host.
    let exponent = i128::from(scanned.exponent)
        + i128::from(high)
        + 4 * (tail_digits as i128 - scanned.fraction_digits as i128);
    if exponent > 1023 {
        return f64::from_bits(sign | INFINITY);
    }
    if exponent < -1075 {
        return f64::from_bits(sign);
    }
    let mut exponent = exponent as i64;
    let precision = (exponent + 1075).clamp(0, 53) as usize;
    let mut significand = 0u64;
    let mut position = 0usize;
    let mut guard = false;
    let mut sticky = false;
    for (index, byte) in scanned.mantissa[first..]
        .iter()
        .filter(|b| **b != b'.')
        .enumerate()
    {
        let digit = hex_digit(*byte);
        let width = if index == 0 { high as usize + 1 } else { 4 };
        for bit in (0..width).rev() {
            let one = digit & (1 << bit) != 0;
            if position < precision {
                significand = (significand << 1) | u64::from(one);
            } else if position == precision {
                guard = one;
            } else {
                sticky |= one;
            }
            position += 1;
        }
    }
    if position < precision {
        significand <<= precision - position;
    }
    if guard && (sticky || significand & 1 != 0) {
        significand += 1;
    }
    let bits = if exponent < -1022 {
        significand
    } else {
        if significand == 1 << 53 {
            significand >>= 1;
            exponent += 1;
        }
        if exponent > 1023 {
            INFINITY
        } else {
            ((exponent + 1023) as u64) << 52 | (significand & 0x000f_ffff_ffff_ffff)
        }
    };
    f64::from_bits(sign | bits)
}
