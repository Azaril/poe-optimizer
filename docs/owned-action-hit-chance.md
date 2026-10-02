# Native action hit chance and source configuration

An action's accuracy-based hit chance and its chance to get past enemy block are
separate stages. Both use injected game rules and exact action/actor identities.
The native evaluator consumes owned inputs and compiled programs; source XML,
PoB UI defaults and reference execution belong in Import or offline tests.

The pinned PoE2 source gives ordinary minions a cannot-be-evaded flag. It does not
give them the old monster accuracy curve. An alternate player-accuracy inheritance
branch exists in the source, but requires a separate producer and consumer proof.
Consequently, a zero Accuracy output on an ordinary minion does not imply a zero
chance to hit. The first consumer is the actual Skeletal Sniper Basic Attack;
admission for this exact attack does not admit every minion, spell or action.

The native program first proves the applicable minion policy and derives the
accuracy-based result. Enemy block is then calculated in source order:

`max(min(total enemy block, 100) - action block reduction, 0)`

The cannot-block-attacks flag replaces that value with zero for the selected
attack. The final chance is the accuracy-based chance multiplied by
`1 - effective block / 100`. Values, units, constants and operations are declared
in data. In particular, changing the order of the upper clamp and subtraction
can change results and is not an equivalent simplification.

The configured BlockChance BASE value is one contribution to total enemy block.
Its absence is not proof that every block source is absent. Every reduction or
flag-presence aggregate requires complete incoming membership before using its
declared empty identity. Incomplete input producers and inherited-accuracy cases
remain unresolved; numerical component tests may explicitly close their finite
world, but production coverage stays Partial.

The present rule contract can encode flag presence with nonnegative integer
contributions from explicitly authenticated zero-or-one producers. Its aggregate
guard does not prove each contributor's domain and cannot detect cancellation of
forged positive/negative entries. This is a data-producer contract, not a new Core
type guarantee. A native Boolean reduction or per-contributor domain declaration
can be considered when real modifier families require broader producer admission.
Missing Capability values are not false and cannot substitute for that proof.

## Saved configuration fallback

The source's `countAllowZero` setting uses an explicit numeric Input first, then
a saved numeric Placeholder. Zero is a value in both positions. A bounded opt-in
Import policy records presence and the selected raw quantity with its exact source
provenance. Missing values remain distinguishable from authored zero. Unknown,
ambiguous or malformed rows do not silently become defaults. Existing V1 override
semantics remain unchanged.

This rule does not apply to every configuration control. For example, the source's
enemy-distance `count` control treats zero as absent and has a seeded placeholder.
It requires a different reviewed adapter before a native player-accuracy consumer
can depend on it. Distance is not needed by the currently admitted minion
cannot-be-evaded branch. The source's numerical distance falloff ends at 90 units;
its tooltip says 12 metres. Runtime data and actual consumers are the reference.
The source also carries distinct distance values: Input zero plus Placeholder 80
produces a Config distance contribution of 80 while the raw skill configuration
distance is zero. A future consumer must identify which one it actually reads;
one generic distance assumption cannot safely stand in for both.

## Validation boundary

Fresh complete-source loads must establish unchanged original outputs, actual
MAIN/CALCS pass inputs, input/placeholder precedence, clamping and flag recipients.
Source-parsed controls must retain their real applicability: a player modifier
does not automatically affect minions. Level changes, repeated actors and warm
reloads distinguish occurrence-specific behavior from cached source state.

Native tests compare actual published programs and routes with that evidence,
including missing inputs, alternate policy branches, scratch reuse and parallel
workers. The five original saved selections and 110 queries remain unchanged.
This slice does not decide the open occurrence-input, usage, preparation-readiness
or socket-configuration proposals and does not establish whole-build parity.

The checked source witness covers 31 cases and 32 complete loads in each JIT mode.
Its compact committed projection retains 13 actual Sniper consumers for native
parity and 14 player-distance consumers as reference-only evidence. Default native
tests reconstruct raw inputs and compare outputs; they never use expected hit
chance as an evaluator input. Fresh source tests regenerate the projection in
memory and compare it to the fixture, without rewriting expected values.
