# Intrinsic minion attack sources

The native actor baseline and action source are separate numerical stages. An
actor's level selects a data-table value; its profile controls damage scale,
spread, attack time and critical chance. Injected programs produce the intrinsic
weapon values. Explicit routes carry those values to that actor occurrence's
Basic Attack. The evaluator contains neither a named-build branch nor Lua data
interpretation for these steps.

For the pinned source, construction first floors the selected damage-table value,
then applies the profile's damage scale. Attack time multiplies damage unless the
profile explicitly ignores it. Each spread endpoint is floored separately. The
intrinsic attack rate is the reciprocal of attack time. These operations must keep
their order; flooring the final spread endpoints alone is insufficient.

The first published consumer is the existing Skeletal Sniper actor declaration
and Basic Attack output. Its raw parent level, effective summon level and actor
level remain distinct dependencies. A missing producer remains unresolved. Tests
may provide a finite level producer explicitly, but this is not evidence that the
unchanged imported build can supply that level end to end.

The finite test also requests the parent summoning action. Its existing
reservation program needs an Action context, and the current planner cannot
prepare that context from a child-only request. Keep this as a real preparation
dependency: production must derive required parent action contexts from explicit
build state independently of which output metrics a user requests. Adding hidden
queries to the five originals would obscure the gap. This belongs with the
[preparation-readiness design](owned-preparation-readiness-proposal.md).

Other source paths can replace this intrinsic weapon with inherited or equipped
weapons. Those paths need explicit source selection before their actions can
consume a final weapon. Intrinsic values do not establish final hit damage, DPS,
attack timing after modifiers or whole-build parity. Partial actor and action
coverage remain Partial.

Reference validation constructs fresh actors through the complete pinned PoB
source. Mutating a finished actor's level cannot validate construction because
the weapon may retain its old values. Native validation exercises the injected
programs and exact action-actor routes, repeated summon occurrences, unresolved
inputs, scratch reuse and parallel execution. PoB is used only by the reference
tests and offline conversion; native workers have no Lua or subprocess dependency.

Physical support inventory is independent of these numerical stages. The opt-in
Import census can prove that every saved physical assignment has been materialized
while retaining unresolved targets, generated effect discovery and active-skill
input obligations. This is an import completion proof, not calculation coverage.
