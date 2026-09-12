//! Conservative pinned-bytecode check for this test's observed post-store seam.
//! No runtime layout or general Lua control-flow admission is provided.
use super::copy_witness::producer::ProducerStore;
use poe_optimizer_pob::source_programs::capture::{
    ConstructorDiagnosticInstruction as Instruction, ConstructorDiagnosticReport,
};
use serde_json::{Value as Json, json};

fn check<'a>(
    window: &'a [Instruction],
    control: &[Instruction],
    allocation: u32,
    continuation: u32,
    line_end: u32,
) -> Result<&'a Instruction, String> {
    let fail = |reason: &str| Err(reason.to_owned());
    if continuation <= allocation + 1 {
        return fail("no constructor-to-store interval");
    }
    let at = |pc| {
        window
            .iter()
            .find(|i| i.pc == pc)
            .ok_or_else(|| format!("missing original instruction {pc}"))
    };
    let new = at(allocation)?;
    if !matches!(new.word & 255, 52 | 53) {
        return fail("selected instruction is not TNEW/TDUP");
    }
    if line_end < continuation {
        return fail("invalid continuation line interval");
    }
    for pc in continuation..=line_end {
        at(pc)?;
    }
    let register = (new.word >> 8) & 255;
    let store = at(continuation - 1)?;
    if store.word & 255 != 60 || (store.word >> 8) & 255 != register {
        return fail(
            "continuation is not immediately after storing the constructor register with TSETV",
        );
    }
    for pc in allocation + 1..continuation - 1 {
        let instruction = at(pc)?;
        let a = (instruction.word >> 8) & 255;
        if matches!(instruction.word & 255, 69 | 70 | 72 | 77..=87) {
            return fail(
                "loop/iterator hidden register effects are not admitted inside the constructor interval",
            );
        }
        match instruction.mode & 7 {
            1 if a == register => return fail("constructor register overwritten before store"),
            2 | 4 if a <= register => {
                return fail("base/range operation may overwrite constructor register");
            }
            _ => {}
        }
    }
    for instruction in control {
        let opcode = instruction.word & 255;
        if matches!(opcode, 78 | 81 | 84 | 87) {
            return fail("trace-patched control flow is not admitted by this diagnostic check");
        }
        let mut destinations = Vec::with_capacity(2);
        if (instruction.mode >> 7) & 15 == 13 {
            destinations
                .push(i64::from(instruction.pc) + 1 + i64::from(instruction.word >> 16) - 0x8000);
        }
        // Conservatively include test skips, even type tests that cannot reach
        // that edge in an admitted execution. This may reject; it cannot omit it.
        if opcode <= 17 {
            destinations.push(i64::from(instruction.pc) + 2);
        }
        for destination in destinations {
            if destination == i64::from(continuation) {
                return fail(
                    "alternate edge can enter the continuation without its immediate store",
                );
            }
            if destination > i64::from(continuation)
                && destination <= i64::from(line_end)
                && !(continuation..=line_end).contains(&instruction.pc)
            {
                return fail(
                    "alternate edge can enter a later instruction on the continuation line",
                );
            }
            if destination > i64::from(allocation)
                && destination < i64::from(continuation)
                && !(allocation..continuation).contains(&instruction.pc)
            {
                return fail("alternate edge can enter the store interval after allocation");
            }
        }
    }
    Ok(store)
}

pub fn post_store(
    report: &ConstructorDiagnosticReport,
    event: &ProducerStore,
) -> Result<Json, String> {
    let continuation = report
        .continuation_pc
        .ok_or("no original continuation PC")?;
    let boundary = report
        .instruction_window
        .iter()
        .find(|i| i.pc == continuation)
        .ok_or("missing continuation instruction")?;
    if boundary.line as usize != event.source_line {
        return Err("observed line differs from original continuation".into());
    }
    let line_end = *report
        .continuation_pcs
        .last()
        .ok_or("missing continuation-line inventory")?;
    if report
        .continuation_pcs
        .iter()
        .copied()
        .ne(continuation..=line_end)
        || report
            .instruction_window
            .iter()
            .filter(|i| (continuation..=line_end).contains(&i.pc))
            .any(|i| i.line as usize != event.source_line)
    {
        return Err("continuation line is not a complete contiguous instruction region".into());
    }
    let store = check(
        &report.instruction_window,
        &report.control_flow_instructions,
        report.instruction.pc,
        continuation,
        line_end,
    )?;
    let table_register = store.word >> 24;
    let index_register = (store.word >> 16) & 255;
    // Original debug-local slots are one-based stack registers (lj_debug.c).
    if table_register as usize + 1 != event.list_slot
        || index_register as usize + 1 != event.index_slot
    {
        return Err("TSETV operands do not bind the observed modList/i local slots".into());
    }
    Ok(
        json!({"allocation_pc":report.instruction.pc,"store_pc":store.pc,"continuation_pc":continuation,"continuation_line_pcs":report.continuation_pcs,
        "constructor_register":(report.instruction.word>>8)&255,"list_register":table_register,"index_register":index_register,
        "constructor_register_preserved":true,"continuation_has_only_immediate_store_predecessor":true,
        "no_alternate_entry_after_allocation":true,"continuation_line_region_dominated_by_store":true,"observed_exact_hook_pc":false,"original_local_slots_bound":true,
        "scope":"pinned original constructor/store interval and next-line event; not a general CFG or physical table-layout proof"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn instruction(pc: u32, word: u32, mode: u32) -> Instruction {
        Instruction {
            pc,
            word,
            mode,
            line: pc,
        }
    }
    fn sample() -> Vec<Instruction> {
        vec![
            instruction(10, 53 | (8 << 8), 1),
            instruction(11, 41 | (9 << 8), 1),
            instruction(12, 60 | (8 << 8) | (7 << 16) | (6 << 24), 3),
            instruction(13, 14, 0),
        ]
    }
    #[test]
    fn rejects_bypassed_store_allocation_and_overwritten_constructor_register() {
        let window = sample();
        assert_eq!(check(&window, &[], 10, 13, 13).unwrap().pc, 12);
        let jump = |pc: u32, target: i64| {
            instruction(
                pc,
                88 | (((target - i64::from(pc) - 1 + 0x8000) as u32) << 16),
                13 << 7,
            )
        };
        assert!(check(&window, &[jump(2, 13)], 10, 13, 13).is_err());
        assert!(check(&window, &[jump(2, 11)], 10, 13, 13).is_err());
        assert!(check(&window, &[instruction(11, 14, 0)], 10, 13, 13).is_err());
        let mut line_region = window.clone();
        line_region.push(instruction(14, 18 | (10 << 8), 1));
        assert!(
            check(&line_region, &[jump(2, 14)], 10, 13, 14).is_err(),
            "later same-line entry also bypasses the store"
        );
        let mut iterator = window.clone();
        iterator[1] = instruction(11, 82 | (9 << 8), 2);
        assert!(
            check(&iterator, &[], 10, 13, 13).is_err(),
            "hidden iterator register writes can clobber A-1"
        );
        let mut overwritten = window.clone();
        overwritten[1] = instruction(11, 18 | (8 << 8), 1);
        assert!(check(&overwritten, &[], 10, 13, 13).is_err());
        assert!(
            check(&window, &[jump(20, 10)], 10, 13, 13).is_ok(),
            "loop entry repeats the actual allocation"
        );
    }
}
