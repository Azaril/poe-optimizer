//! Numeric bit helpers matching PoB's `Data/Global.lua` and LuaJIT.
//!
//! PoB's `OR64` operates on Lua doubles with a 21-bit high mask and a signed
//! 32-bit low result. It is not a conventional unsigned 53-bit OR: the signed
//! low word is preserved when recombining, including when bit 31 is set.

const TWO32: f64 = 4_294_967_296.0;
const TOBIT_BIAS: f64 = 6_755_399_441_055_744.0;

/// The pairwise numeric operation used by PoB's variadic `OR64`.
///
/// Source conversion rounds fractional low words with LuaJIT's ties-to-even
/// conversion. The implementation preserves the source arithmetic order and
/// signed low word instead of casting the input to an unsigned integer.
/// The caller owns Lua nil/boolean/string coercion and variadic arity. Inputs
/// represent values already admitted to Lua: an adapter reproducing LuaJIT API
/// ingress must canonicalize NaNs there rather than pass arbitrary NaN payloads.
pub fn or53(a: f64, b: f64) -> f64 {
    let a_high = (a / TWO32).floor();
    let b_high = (b / TWO32).floor();
    let high = to_bit(a_high) | to_bit(b_high);
    let low = to_bit(a - a_high * TWO32) | to_bit(b - b_high * TWO32);
    f64::from(high & 0x1f_ffff) * TWO32 + f64::from(low)
}

pub(crate) fn to_bit(value: f64) -> i32 {
    // LuaJIT lj_vm_tobit adds 2^52+2^51 and returns the low 32 IEEE bits.
    (value + TOBIT_BIAS).to_bits() as i32
}
