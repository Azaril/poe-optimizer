# Native calculation engine and browser target

This document defines the native replacement track. Delivery status, validation results
and the next resume point belong in [implementation.md](implementation.md). The Rust
engine develops alongside the mlua/PoB evaluator: the latter supplies a versioned oracle
and usable evaluation path while the native engine grows through verified slices.

## Boundaries and intended outcome

`poe-optimizer-engine` owns game calculations that can run without Lua or an operating
system. It accepts resolved numeric inputs, validated numeric modifier layers, and explicit condition contexts now,
and will eventually accept typed build, modifier and game-data models. The optimizer core owns objectives, constraints and search;
the PoB adapter owns Lua runtime hosting, XML import/export and reference execution.
Neither the native engine nor future browser bindings should depend on the native PoB
worker package.

The long-term replacement is a native evaluator behind the same semantic evaluation
contract: actor/skill/scenario selection, metric units, coverage declarations and data
identity must remain comparable. A partially translated engine must report its supported
scope. Do not fill unsupported metrics with zero or silently combine native and Lua
outputs from different candidate states. A deliberate hybrid path needs explicit ownership
of each calculation stage and parity at its boundaries.

Pure calculations neither allocate a thread pool nor perform I/O. The native desktop
runner can schedule independent candidate/scenario evaluations through the shared Rayon
budget. A browser runner supplies its own scheduling, progress, cancellation and data
loading while calling the same Rust calculations. Persistent mutable caches, when added,
belong to one evaluation context and must demonstrate request-order independence.

## First translation boundary: defence kernels

The initial native surface is small enough to compare directly with upstream code, and
useful inside a later defence pipeline. All outputs below are percentage points rather
than 0-1 fractions; rating and raw hit inputs are already resolved by the caller.

| Native function | Pinned upstream function | Contract |
| --- | --- | --- |
| `hit_chance` | `calcs.hitChance` | Player hit formula with a 5% floor and optional removal of the 100% ceiling. Negative accuracy returns 5 immediately. |
| `monster_hit_chance` | `calcs.monsterHitChance` | Separate monster hit formula, rounded and clamped to 5-100%. |
| `deflect_chance` | `calcs.deflectChance` | Deflection formula with the selected data cap; rating below 1 returns 0. |
| `armour_reduction_percent` | `calcs.armourReductionF` | Fractional percent reduction for one raw hit, including negative-armour behavior. Later mitigation caps are outside this helper. |
| `armour_reduction_rounded_percent` | `calcs.armourReduction` | The same reduction with upstream integer rounding. |
| `round_to_integer` | `round`, without `dec` | `floor(value + 0.5)`, including negative half-integers. |

Source is [CalcDefence.lua:33-69](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/CalcDefence.lua#L33-L69)
and [Common.lua:722-728](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Common.lua#L722-L728).
`DefenceConstants` keeps data inputs explicit. The pinned defaults are armour ratio 10
and deflection cap 95 from [Modules/Data.lua](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Data.lua#L251-L261).
The upstream MIT attribution is retained in the crate's
[NOTICE.md](../crates/poe-optimizer-engine/NOTICE.md).

These helpers do not collect modifiers, apply item/passive rules, compute complete EHP,
or evaluate a build. Their constants are passed explicitly rather than being a second
independent game-data database.

## Second translation boundary: numeric ModDB queries

`modifiers::ModifierDatabase` is an immutable numeric aggregation slice with explicit
`QueryContext` and `MorePrecision` inputs. It models the pinned `ModDB` methods, not the
entire modifier store and not `ModList` interchangeability. A `ModifierInput` preserves
its source, modifier/value kinds and tag names until validation. Construction rejects the
entire input if any layer contains an unsupported entry, even if a particular query
would not select that entry. This keeps partial extraction from silently changing a build.
No automatic Lua extraction or full-build adapter is attached to this slice yet.

| Surface | Supported semantics |
| --- | --- |
| `sum(Base / Increased)` | BASE/INC addition by query-name then insertion order; each local result adds its recursively grouped parent result. |
| `more` | MORE percentages become multiplicative factors. Default rounding occurs per local stat bucket at two decimal places; explicit high precision truncates the accumulated result. Precision carries across query names within a layer and resets for each parent. |
| `override_value` | First matching local value in name/insertion order, then parent layers. Zero is a present override; absence returns `None`. |
| Modifier flags | All required bits must occur in the query. Exactly representable nonnegative 53-bit masks are supported except bit 31, whose upstream signed-low-word behavior requires a separate extension. Every currently declared pinned `ModFlag` fits the supported domain. |
| Keyword flags | Any matching keyword by default, all keywords when the modifier carries `MatchAll`. Empty requirements match. The `MatchAll` control bit is removed from both masks before matching; masks are bounded to bits 0-30. |
| Source provenance | Strings remain attached to modifiers. BASE/INC accept the exact source or its first nonempty colon-delimited component. MORE/OVERRIDE match only that component. A selected modifier with absent source and a source-filtered MORE/OVERRIDE query produces an explicit error, matching the upstream error boundary. |
| Parent layers | Layer zero is the queried DB and following layers are its successive parents. Layer and insertion order are semantic inputs, retained for rounding, floating-point cancellation and override priority. |
| Query names | Zero through eight names, preserving order and repeated names. More than eight is rejected. |

`MorePrecision::pinned()` contains the two MORE entries from pinned `Modules/Data.lua`:
`SupportManaMultiplier` and `ReservationMultiplier`, both at four decimal places.
`try_new` accepts an explicit alternate precision table with decimal places 0-15. The
empty/default table means ordinary two-decimal rounding everywhere; it must not be
mistaken for the pinned game data. Precision inputs concern MORE aggregation only;
modifier scaling has separate rules outside this slice.

The legacy `try_new` constructor still rejects every raw tag name. The explicit typed
condition path below handles a declared subset of `EvalMod`; item, skill, multiplier,
threshold, global-limit and other tags remain unsupported. FLAG/LIST/MAX and unknown
modifier kinds, nonnumeric values including functions/tables/booleans/nil, unsupported
flag masks and out-of-range precision fail explicitly. A future importer must carry
unsupported metadata to validation rather than constructing an untagged approximation.
This module does not derive a final stat by assuming a universal combination of BASE,
INC, MORE and OVERRIDE.

The independent differential harness executes the actual pinned `ModStore` public query
wrappers and `ModDB` implementation. It loads the actual `Data/Global.lua` bit/keyword
helpers and extracts the actual `Common.lua` class library/rounding and `Data.lua` precision
table. All five full normalized source hashes are checked. There is no translated Lua
formula oracle. Cases cover interpreted and warmed LuaJIT, interacting flag/keyword masks,
53-bit boundaries, parent layers, duplicate and reordered stat names, source variations,
zero overrides, precision carry, negative and near-rounding-boundary factors, grouped
cancellation, nonfinite values and fail-closed unsupported input. Finite comparisons use
`1e-12 * max(1, abs(reference))`; signed zero/infinity and NaN classification are checked
separately. Captured real modifier contexts and whole-build mutation parity remain gates
before native evaluator integration.

Sources: [ModDB.lua](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Classes/ModDB.lua),
[ModStore.lua](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Classes/ModStore.lua),
[Global.lua](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Data/Global.lua#L122-L332),
[MORE precision data](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/blob/3887ae68a6a6b8bb7b41d1b61998f1aa184201e4/src/Modules/Data.lua#L597-L603).

## Third translation boundary: explicit conditional modifier contexts

`ModifierDatabase::try_new_tagged` accepts `TaggedModifierInput` without changing the
legacy untagged constructor or query signatures. Each input keeps its complete numeric
modifier, source, flags and an ordered vector of typed `ModifierTag` values. Unknown tags
or unrepresented fields must become `Unsupported` metadata and reject the complete input,
including entries no current query would select. Tagged databases require the explicit
`sum_with_conditions`, `more_with_conditions` or `override_with_conditions` query path;
calling an old query on them returns `MissingConditionContext`.

`ConditionEnvironment::try_new` validates an immutable environment. It contains the
queried store's local/parent condition tables, an actor index graph, per-actor condition
tables and weapon metadata, and the query's override/skill conditions and actor name.
References must resolve; every store condition layer corresponds to a modifier layer.
An absent actor link is distinct from an invalid link. Contexts retain unsupported-feature
markers so extraction cannot discard condition-producing modifiers or unknown behavior.
They do not own Lua objects, mutable caches, a thread pool or operating-system services.

| Supported behavior | Pinned semantics |
| --- | --- |
| `Condition` with one variable or a list | A list is OR, ordered tags are AND; negation applies to the complete tag. An empty list is false before negation. |
| `GetCondition` table lookup | A present `overrideCond` value wins, including false. Otherwise local or any parent true is sufficient; local false does not mask a true parent. |
| Skill-local conditions | `Condition` checks `skillCond` after `GetCondition`, so a true skill condition can satisfy even a false override. `ActorCondition` ignores `skillCond`. |
| `ActorCondition` | Supports an explicit actor or the current queried store, optional one/list variables, negation, and the upstream `cfg.actor` fallback when no condition target is present. Enemy conditions use this tag with `actor = "enemy"`; the pin has no `EnemyCondition` tag. |
| Player actor lookup | Direct player reference first, then the parent actor's player, then the enemy actor's player. Other roles use direct links. Parent actor references and parent modifier layers are separate inputs. |
| Parent modifier evaluation | A modifier inherited from a parent DB still evaluates against the original queried store and actor context, preserving the `context` argument threaded through `ModDB`. |
| All-one-handed weapon exception | Negated `Condition` tags retain `countsAsAll1H` and `Added<condition>` behavior, including first qualifying weapon precedence and list order. |
| Disabled conditional values | BASE/INC contribute zero; MORE uses zero but still influences precision selection; OVERRIDE is absent. Active numeric zero remains a present override. Source errors occur before tag evaluation. |

This is an **explicit condition-table subset**. `GetCondition` also consults
`Condition:<name>` FLAG modifiers upstream. FLAG inputs still reject at numeric DB
construction, and condition-producing FLAG entries in actor/context extraction must be
retained as unsupported features. They must not be pre-resolved into apparently complete
booleans unless a separate extraction contract proves equivalence across the exact query
flags, source and overrides. No real-build modifier extractor is provided yet.

Multiplier and scaling tags remain a separate stage: `GetMultiplier` combines explicit
values, parent values, BASE modifiers and OVERRIDE queries, introducing dependencies and
possible recursion. Adding a scalar multiplier field would not implement those semantics.
Global limits, item/skill predicates, modifier functions and condition-producing FLAG
queries need their own typed input and parity scope before activation.

The differential suite uses the same source-hashed, actual upstream `ModStore`/`ModDB`
harness as numeric aggregation; it does not replace `EvalMod` or `GetCondition` with a
copied oracle. It exercises interpreted and warmed LuaJIT across truthy inheritance,
false overrides, skill-local conditions, missing actors, player fallback precedence,
weapon exceptions, source and flag filtering, inactive MORE precision, zero overrides,
nonfinite values and explicit rejection. Captured real modifier contexts and complete
candidate mutation parity remain required before integrating this slice into a native
build evaluator. No throughput improvement is claimed.

## Numeric behavior and parity

Use `f64` with the original operation ordering and no fast-math reassociation. Upstream
rounding differs from Rust's `f64::round()` at negative half-integers. Negative armour uses
its positive magnitude in the denominator and negates the result; its rounded form need
not be symmetric at a half-integer. Preserve these details until a separately reviewed
game-mechanics correction intentionally changes the contract.

Low-level kernels retain upstream exceptional arithmetic. The explicit zero-armour,
zero-hit case returns zero; singular denominators and nonfinite inputs can otherwise
produce infinities or NaN. Upstream x64 LuaJIT ordered min/max clamps can turn an intermediate
NaN into a finite boundary. Native comparisons model that ordering explicitly rather than
relying on Rust or WASM min/max NaN behavior. These semantics are a compatibility contract,
not proof that such an input is a valid game state. Candidate validation must reject invalid
inputs independently. Keep NaN, positive infinity and negative infinity distinct in metric
serialization and feasibility policies. NaN payload bits have no semantic guarantee.

Each translated slice must pass:

1. A source identity check tied to the submodule revision and normalized source hashes.
2. Differential tests executing the actual upstream functions, with the minimum necessary
   environment and data. A copy of the Rust formula rewritten in Lua is not an oracle.
3. Boundary cases, branch/override cases, singular arithmetic and relevant property checks.
4. Differential comparisons in interpreted and warmed LuaJIT execution where applicable.
5. Integration comparisons using real candidate states before activating the slice inside
   the native build evaluator.

The kernel differential harness loads the actual pinned defence module with a minimal
`Modules.CalcBase` table, and extracts the actual upstream `round` definition. It checks
CRLF-normalized SHA-256 hashes for the defence, common and data modules; changing the pin
requires an explicit translation review. Checks include rating grids, half-integer and
percentage-threshold neighbours, early returns, negative armour, alternate constants,
positive-domain monotonicity, nonfinite classifications and signed zero. Finite fractional
results use a declared `1e-11 * max(1, abs(reference))` tolerance; infinities retain their
signs and NaN is compared by classification. No performance claim follows from these tests.

The Rust production library has no external dependencies. Lua and source hashing are
native-only development dependencies used by the test oracle. Maintain this separation
as the native evaluator expands; shared data types must not introduce a transitive Lua
runtime dependency.

## Expansion order and hard boundaries

The native track can advance in parallel with evaluator hardening and search development.
Prioritize cohesive stages with explicit inputs/outputs over arbitrary lines of Lua:

| Stage | Boundary to preserve | Main parity evidence |
| --- | --- | --- |
| Numeric kernels | Units, rounding, constants, exceptional arithmetic | Direct differential grids and branch cases |
| Modifier aggregation | BASE/INC/MORE/OVERRIDE, flags, tags, scopes, conditions and source provenance | Small exhaustive modifier sets and captured real modifier contexts |
| Resource and resistance pipeline | Equipment conversions, reservation, maximum/current pools, caps, overrides and actor inheritance | Complete intermediate outputs and controlled candidate mutations |
| Active skill/minion pipeline | Skill/stat-set identities, support compatibility, granted effects, triggers, actor and action selection | Cross-skill/support interactions, exact selections and fixture deltas |
| Full defence/offence pipeline | Damage conversions, ailments, costs, recovery, avoidance, maximum hits and encounter assumptions | Independent build/scenario parity across representative mechanics |
| Native evaluator integration | One complete candidate, versioned data, coverage declaration, export semantics | Full contract parity and search-result re-evaluation |

Modifier DB semantics and normalized game data are substantial prerequisites. PoB uses
mutable global tables, modifier functions/closures, private classes, conditional tags and
version-specific preprocessing; translating the arithmetic alone does not remove them.
Design a typed, validated, versioned data snapshot with stable IDs, source hashes and
unsupported-field reporting. Extraction can initially use the reference adapter as a
build-time tool; the deployed native/browser evaluator must consume data without Lua.
Separate extraction-tool requirements from runtime requirements and retain applicable
upstream notices with generated/derived data.

Keep the reference adapter after a native path becomes useful. It supplies differential
checks for upstream updates and mechanisms not yet translated. A new game patch changes
both calculations and data; record capabilities by game/data/adapter identity and rerun
affected parity fixtures rather than silently comparing incompatible revisions.

## WebAssembly deployment

Target `wasm32-unknown-unknown` for the portable calculation library. The kernel dependency
graph contains no Lua/C runtime or OS calls. Compilation is only the first gate: browser
bindings, downloadable data, input limits, cancellation, memory budgets and execution tests
are additional work. The Rust target supports `core`/`alloc` and portions of `std`, while
filesystem and thread APIs do not provide normal native behavior. Keep those services in
the host. [Rust target documentation](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html)

Start browser execution in a Web Worker, with a portable sequential path and bounded work
batches. Future parallel execution may use several independent workers or shared-memory
Rayon support. Rayon ordinarily falls back to sequential operation for WebAssembly without
thread support. [Rayon documentation](https://github.com/rayon-rs/rayon#usage-with-webassembly)
Shared-memory browser Rayon requires an adapter, worker initialization and cross-origin
isolation for SharedArrayBuffer; check the deployment environment before committing to
that mode. [wasm-bindgen-rayon documentation](https://github.com/RReverser/wasm-bindgen-rayon#setting-up)

Keep browser scheduling outside the math crate so desktop multicore performance and
browser compatibility can evolve independently. Feature-detect the selected browser
execution mode and record it in results. Do not promise equivalent throughput, memory
capacity, or wall-clock determinism across native and browser hosts.

The portable compilation gate is:

```powershell
cargo check -p poe-optimizer-engine --lib --target wasm32-unknown-unknown --locked
```

Native differential validation is:

```powershell
cargo test -p poe-optimizer-engine --locked
```

Record actual command results and unverified targets in the living implementation log.
