# Bounded equipment Life reduction

This packet adds three existing `CheckNumericDomain` expressions to two
existing flat-Life contribution programs. It completes only the equipment group
of `life-base-contributions` for this bounded native calculation domain. The
four Modifier `3100` owner gaps, global query coverage, other owners and all
import obligations remain unchanged. It adds no final Life consumer or
evaluation bundle.

The direct contribution checks its final Life amount. The Amulet copy checks
both its effective Count operand and its final Life amount. Each accepted value
lies in the inclusive interval `[-2^37, 2^37]`. A demanded value outside that
interval is unsupported, rather than clamped or treated as inactive. Existing
eligibility and conditional arithmetic remain lazy.

The unchanged `effective-amount` program rounds to a multiple of one Count and
divides by one. Its actual output is therefore integral. Direct unit conversion
preserves it. An Amulet copy of that integral output is either the unchanged
value or the existing integral copy branch. Synthetic fractional facts supplied
directly to a component test are not this bound producer graph.

The checked engine has a non-raisable maximum of 65,536 effects, including graph
extensions. Query membership admits each effect exactly once. Consequently the
absolute value of any prefix of the equipment sum is at most
`65,536 * 2^37 = 2^53`. Every integer in that interval is exactly representable
in binary64, so every equipment accumulation order gives the same result.
Existing semantic ranks remain deterministic; no claim is made that they
reproduce PoB traversal. This proof applies to this equipment subtotal, not to
cross-group arithmetic or the complete source Life pool.

The effect ceiling is a semantic dependency of this proof. Increasing it
requires re-proving the bounds, rebuilding this data and invalidating the
affected semantic identities and admission; it is not merely performance tuning.

The copy operand check is independently necessary. The retained large-value
numeric discrepancy could otherwise be scaled down into an accepted final copy.
Keeping the operand in this interval excludes that case. Pinned PoB
`ItemTools.lua:45-58` formats these integral values through `tostring`; the
locked LuaJIT `210.7.3+1ee778a` uses `STRFMT_G14`, while `2^37` has only twelve
decimal digits. This bounded transport argument does not implement arbitrary
source encodings, magnitude sequences, routing or missing suppliers. Their
existing owner gaps remain blocking. The shared effective-value program and
its retained large-value diagnostic are unchanged.

Authoring authenticates the exact current owner, unchanged integral producer,
all seven potential Life/Add writer programs and the full query. Publication
requires an exact whole-input inverse, all five unchanged imports, all 110
queries, unchanged selected issue counts and byte-identical rebuilding.

Run the ordinary `owned_life_equipment_domain` target. Publication additionally
sets `POE_OPTIMIZER_TEST_LIFE_EQUIPMENT_DOMAIN_PRIOR` to the current checked
package and `POE_OPTIMIZER_TEST_LIFE_EQUIPMENT_DOMAIN_OUTPUT` to a fresh path,
then runs `publish_bounded_life_equipment_preserving_all_five` with
`--ignored --exact`.

Validation passed: 44 Engine replay tests (seven new bounded-domain cases),
two ordinary CLI tests and the all-five publication gate (22.88 seconds).
Strict Clippy passed for both test targets. The published endpoint is
`fe061a37c795102c8ca025cf52ebca199cb0ba3be42c9fa901e6c42ac5bc6f3f`
at `runs/owned-life-equipment-domain-publication-01/package`. All 110 queries,
local IDs and selected issue counts `[106, 117, 109, 123, 4]` are preserved;
rebuilding the 18 artifacts is byte-identical. No import issues or rule owners
were completed, and no complete native build is claimed.
