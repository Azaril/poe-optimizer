# Intrinsic minion attack source

This slice adds an owned native rule program for intrinsic profile weapon data:
physical minimum, physical maximum and attack rate. Four exact Basic Attack
routes carry those components and the existing critical chance from the selected
action's actor. The source skill is `MinionMeleeBow`, owned skill `0021`, output
`0022`; the actor is the existing Sniper population slot `001f`.

The program reads injected baseline stats and follows the source operation order:
floor the level curve, apply the profile's damage scale, conditionally multiply by
attack time, apply the spread, then floor each endpoint. Attack rate is the
reciprocal of attack time with explicit time/rate units. No game profile, build,
effective gem level or actor level is selected in native Rust code.

`extension.json` allocates seven stats (`320f`–`3215`) and appends one program.
`routes.json` appends four routes for the exact existing part, mode and stat set.
`dependencies.json` records unchanged input stat/unit definitions. `authoring.json`
records the reference version and completed witness identity when validated.
Both owner and routing closures remain Partial. No metric binding or coverage
completion is added.

These values precede selected weapon replacement and action-local modifiers.
Inherited player weapons, minion equipment replacements, hostile/Spectre curves,
final damage, hit chance, action speed and support effects remain outside this
release slice. The program can evaluate a finite counterexample for the
attack-time branch without admitting its actor profile into the production data.

The Engine fixture retains the actual population-level lookup, actor baseline,
ability supply programs and their data tables. It explicitly supplies a final
parent skill level only inside its finite test world; removing that test producer
must leave the native damage unresolved. Before the parity test completes that
world, it compares its unchanged prerequisite snapshot against the published
package. Independent PoB witness vectors come from fresh full source loads,
including repeated actors and a reused runtime, with both JIT modes authenticated.
The small committed calibration fixture runs numerical parity in default CI;
the reference witness rederives it with a shared test-only Rust projection.

The finite fixture also requests the parent summoning action explicitly, because
its existing owner contains an Action-context reservation program. Current plan
discovery reports an unsupported context when only the child action is requested.
This test scaffolding does not rewrite the original query set or resolve that
production preparation dependency. The original final skill input producer and
coverage gates remain unchanged.

The remaining real-build blockers include final parent input production, selected
weapon-source policy, action calculations and complete contributor membership.
This stage alone does not make an original build evaluable.
