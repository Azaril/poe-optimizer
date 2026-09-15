# Owned rule components

Status: component APIs implemented in source, 2026-09-15. This is the delivered
storage/compiler/execution boundary under the [domain architecture](domain-architecture.md).
The component API accepts injected owned data and explicit facts. The owned effect plan
now binds complete owned requests to concrete effects without supplied facts. Neither is a
complete build metric evaluator; **0/5 protected originals complete native evaluation**. Validation receipts and
the next resume point belong in [implementation](implementation.md).

## Separate responsibilities

| Layer | Implemented responsibility | What success establishes |
| --- | --- | --- |
| [Core authoring contracts](../crates/poe-optimizer-core/src/owned_rules.rs) | Versioned package, declared reads, expression DAGs and typed effects. | A portable representation. Raw DTO construction is not validation authority. |
| [Data storage](../crates/poe-optimizer-data/src/owned_rules.rs) | `OwnedRulePackage::new`, `decode_rule_package` and `encode_rule_package`; strict bounded JSON, exact schema binding, owner/program/local identities, expression references and partial-membership evidence. | Structurally valid stored rule data. Operation types, target compatibility and cycles require compilation. |
| [Engine components](../crates/poe-optimizer-engine/src/owned_rules.rs) | `CompiledRulePackage::compile`, immutable private indices, `new_scratch` and `evaluate` with caller-supplied facts. | A checked program and the classified results of its demanded expressions. Supplied facts do not prove provider existence, activation, legality or incoming-effect completeness. |

The [owned effect plan](../crates/poe-optimizer-engine/src/owned_plan.rs) is the consumer of
these layers. `OwnedEffectPlan::compile` receives an owned request, schema, compiled rules
and an [action-routing artifact](../crates/poe-optimizer-data/src/owned_routing.rs). It binds
actual item/modifier/skill/provider occurrences, normalizes relative targets and prepares
an effect dependency graph. `evaluate` uses only that immutable plan and worker-owned scratch.
The [metric layer](owned-metrics.md) reads requested final stats through the same executor,
with separate query activation gates and no intermediate diagnostic-report construction.

A gem's explicit skill-supply rules reuse `ActivateGrant` and
`ProjectSkillParameter`; no separate activation interpreter is needed. The gem owns its
physical level/quality. Its supplied skill owns projected computed inputs and an exact
parent/slot identity. Distinct slots remain distinct even when their skill definition is
equal. Required generated-skill inputs gate both rule effects and routed action values.
Potential memberships alone cannot activate an action or redirect a saved root selector.
The plan content digest uses `owned-effect-plan-v6`; the latest rule operation set is v7,
with the v6 subset still accepted unchanged.

These modules contain no source-language parser, Lua interpreter, PoB callback, UI object or
named-build dispatch. Existing source readers and any optional Lua acquisition stay offline
or in the reference adapter. Adding a game coefficient within these operations changes
injected data; adding an operation requires explicit versioned Rust semantics and tests.

`RulePackageInput` carries its namespace, release, semantics version, operation version and
the exact definition-schema `DataIdentity`. Its wire version is 2 with required `receivers`; the implemented operation set
is `owned-domain-operations-v7`. The compiler also accepts v6 with its unchanged operation
set and identities; character identity predicates require v7. Other operation versions are
explicitly rejected by the compiler; regenerate experimental
artifacts rather than silently interpreting them with new semantics. Both storage and compilation check the supplied index's
identity and namespace. Execution checks that binding again. A digest identifies content;
it does not authenticate its source or prove conversion fidelity.

Storage emits deterministic JSON, canonicalizing receiver rows and applicability targets.
Compilation separately canonicalizes finite-table, owner, program, read and node declaration tables. Effect order and boolean
operand order remain significant. Stored-package and compiled-program digests have separate
domains; consumers must not interchange them. Public declaration IDs are distinct from the
private indices used during execution.
Computed value types and read sources use adjacent `kind`/`value` envelopes, matching the
owned schema conventions and retaining strict unknown-field checks for tag-only variants.

## Finite integer data tables

`RulePackageInput.tables` is an explicit collection of immutable `IntegerRuleTable`
records. Each table has a package-local ID, inclusive bounded-integer minimum/maximum,
a computed scalar type and one row per integer in that domain. Tables can be shared by
multiple programs and owners. This is owned game data, with no source-language table
layout, callback, build name, UI selection or executable cell content.

`LookupIntegerTable { table, key }` requires an Integer key expression. Storage and
compilation reject duplicate IDs, missing rows, reversed/oversized domains, dangling table
references and cells with mismatched types or exact units. Quantity units and Option IDs
must resolve to known definitions, including in tables that no current program reads.
Empty/sparse tables and interpolation are not supported; incomplete source evidence must
remain an unconverted recipe obligation rather than a fabricated row or fallback.

Compilation validates every table and gives lookup operations shared immutable arrays.
Worker evaluation performs checked indexing after evaluating the demanded key. It needs
no schema lookup, source access or table-map allocation. It never clamps a key or selects
an endpoint. A present zero cell is an applied zero; a missing key remains Unresolved.
An out-of-domain integer returns `UnsupportedDomain` with the original lookup node,
table ID, requested key and supported bounds. This cause survives downstream expressions,
required skill inputs, actor/action reads and activation gates. Lazy guards can avoid the
lookup. A false grant still deactivates its descendants while keeping the parent's
component diagnostic; incomplete contributor closure still withholds final values.

Cell edits change stored/compiled identities while keeping definition, slot and table IDs.
The compiler canonicalizes table declaration order; row order always follows its domain.
Independent table/cell and wire/work bounds apply before secondary allocations. The
[direct table tests](../crates/poe-optimizer-engine/tests/owned_tables.rs) and
[occurrence-plan tests](../crates/poe-optimizer-engine/tests/owned_lookup_plan.rs) cover
extreme keys, exact units, lazy demand, unchanged IDs, partial coverage and worker reuse.
These laws do not establish complete skill mechanics or full-build parity.

## Ordinary direct-action timing

`RuleExpression::OrdinaryTiming` is a closed native numerical operation. Its recipe names
all eight input nodes: base time, speed increase, speed multiplier, added attack and cast
time, actor action speed, repeats and server rate. Precision is injected and bounded to
0–12 decimal places. `ReciprocalTimingUnits` explicitly declares a time unit and its rate
per one time unit; compilation enforces those exact units and matching factor units. Equal
dimensions alone do not choose a unit pair or convert values. Repeats is an Integer.

Each node requests speed multiplier, uncapped rate, capped action rate or action time.
All eight inputs must be available for the atomic operation. Internal infinities/NaN remain
inside its numerical algorithm: an unavailable uncapped rate does not erase a separately
requested finite capped rate or time. The selected nonfinite channel becomes `NonFinite`;
finite quantities use the normal owned signed-zero normalization. Lazy enclosing guards
can still avoid demanding the operation at all.

The pure [timing primitive](../crates/poe-optimizer-engine/src/timing/ordinary.rs) preserves
rounding, reciprocal/addition order and cap semantics. The legacy timing entry point now
delegates to it. This shares numerical behavior but does not migrate the profile-specific
preparation/realization callers. Raw reference tests retain infinity sign, NaN and signed-zero
evidence separately from the finite owned-value contract.

Numerical computability does not certify a valid timing branch or character. Positive tick
rates, legal repeat counts, support applicability and exclusion of trigger/channel/reload
branches belong to injected input schemas and requirement/selection rules. No coefficient,
actor selection or missing-input default comes from a skill name or PoB UI object.

## Typed reads, operations and effects

`CharacterClassIs` and `CharacterAscendancyIs` are Boolean reads of the authored player
character, including inside an owned actor's invocation. Their operands are typed owned
IDs validated against the injected schema. Known absence of an ascendancy yields false;
no class name, source numeric key, selected UI object or synthesized capability is read.
They retain ordinary provider activation, required-input and global completeness gates.
Changing class/ascendancy changes the bound plan and its identity. Existing v6 packages
are not upgraded implicitly, and cannot contain these v7 reads.


The [definition schema](owned-definition-package.md) has 24 standalone descriptor families
and six declared-slot families. `StatDefId` declares a computed value type and permitted
target kinds; `CapabilityDefId` declares a boolean fact's permitted targets. Neither is a
source stat name or a free-form character override. Computed values are Boolean, Integer,
Quantity with an exact `UnitDefId`, or Option. Equal unit dimensions do not authorize
conversion between different unit IDs.

A program declares an Actor, Action, EquipmentUse, Enemy or Environment context. `Current`
uses that kind; relative `Actor` requires Actor/Action context. `Player`, `Enemy` and
`Environment` have their explicit target kinds. These checks establish type compatibility,
not a concrete actor or equipment occurrence. In particular, relative Actor must not become
an implicit player or selected-minion lookup.

Reads name declared parameter/choice slots, character/gem/item levels, item/gem quality,
computed stats, capabilities, external assumptions or incoming contributions. Parameter and
grant ports must belong to the exact rule owner and its declared membership. Gem/item level
and quality reads require the corresponding owner; their declared ranges apply to supplied
values. A choice read uses the owner's exposed choice context. No lookup inherits unrelated
ports from a same-named or adjacent definition.
Owned item records may explicitly leave item level unspecified. The occurrence binder
omits that fact rather than substituting a number; ItemLevel retains its Integer
rule type. Evaluation reports unresolved only when an active effect demands it. Lazy guards
and branches that do not use it can still produce their own component result.

Quality has explicit presence and amount reads:

```text
Select(HasItemQuality(kind), ItemQualityAmount(kind), explicit_absent_value)
```

The same pattern exists for gem quality. The kind must be declared by that item's template
or gem schema and have a known amount schema. A supplied false presence fact can select
the data-authored absent branch without an amount fact. A missing presence fact, or a missing
amount on the selected present branch, is unresolved. Present zero remains a value. The
component executor does not read a build or infer absence; the owned plan binds these
facts from the actual selection. An explicitly supplied malformed amount still rejects
even when its branch would be inactive.

The closed expression vocabulary covers literals/reads, addition/subtraction, min/max,
dimensionless scaling and division, integer scaling, same-unit ratios, percentage-point
conversion, explicit-quantum rounding, comparisons, boolean composition and selection.
Rounding modes are floor, ceiling, truncation and nearest with ties toward positive infinity.
The owned nearest operation uses a floor/fraction comparison that preserves large integral
values and adjacent-half cases. The legacy `floor(x + 0.5)` helper is unchanged; no numerical
reuse or equivalence to that helper is implied. There is no arbitrary unit multiplication
or text-expression fallback.

Effects emit additive/increased/multiplicative contributions, derived stat values,
capabilities, support-applicability decisions, declared grant activation decisions or
requirements. Add uses the stat's exact type/unit; Increase uses percentage points; Multiply
uses a dimensionless factor. A contribution read declares Sum with explicit zero for Add or
Increase, or Product with explicit one for Multiply. Compilation checks that identity and
reduction. Execution still receives a fact: it does not enumerate contributors or infer
that an omitted contribution read means an empty set.

`ActivateGrant` validates the exact declared grant address and produces a boolean decision.
It does not create an actor/action. Likewise, a support-applicability boolean does not attach
a support to every action in a group or establish its cost, trigger or receiver semantics.

`ProjectSkillParameter` transfers a computed value into a parameter declared by the exact
skill behind an owner's declared skill-grant slot. The target parameter has no authored
write sites; it is not a fake Gem level. Types, units and membership are checked at
compilation. Listed members of partial declarations can compile, while required producer
closure remains a later resolution obligation. A computed value outside the supported
schema is reported as `UnsupportedValue`, without clamping or disguising it as a missing
fact. This operation does not activate a grant or choose its receiving equipment use;
The owned plan binds the parent occurrence, target and dependency order. Every required
generated-skill input has an availability gate, even when the current formula does not read
it. A known Boolean false is a present input; an inactive/missing/rejected projection is not.

`ProjectActorStat` transfers a value to the actor declared by the exact supplying provider.
Its destination must admit Actor stats with the same type/unit. It does not activate that
actor or alias two summoners. Parent projection evidence can remain known while a false
grant makes the child consumer inactive.

## Compilation and execution outcomes

Compilation validates every node, including unreachable branches, for references, types,
units, declared scope and cycles. Unsupported operation versions, foreign/missing/unmapped
required schemas, duplicate identities, invalid ports and malformed reductions reject.
The component expression graph is acyclic. The owned plan separately rejects inter-program
cycles, including grants/projections/reductions, and competing concrete final producers. It
uses effect-level dependencies so one independent contribution can feed another effect in
the same program.

Execution uses lazy `Select`, left-to-right short-circuiting `All`/`Any`, and effect guards.
A decisive boolean or false guard avoids demanding later inputs. A missing/erroring operand
encountered before that decision propagates; the executor does not reorder operands to hide
it. All supplied facts are checked for declared identity, duplicate keys, type, unit and
applicable range/option membership before expression demand is evaluated.

The ordered `ProgramEvaluation.effects` ledger retains each effect and its disposition:

- `Applied { value }`: the expression produced a typed value. Applied false is a result;
  for a grant or requirement it does not mean activation or satisfaction.
- `Inactive`: the effect's guard was false; this is distinct from zero or a missing value.
- `Unresolved { input }`: a demanded fact was not supplied.
- `UnsupportedValue { value }`: a projected value lies outside its supported destination
  schema. The computed value is retained; this is not a gameplay-legality verdict.
- `UnsupportedDomain { node, table, key, minimum, maximum }`: a demanded integer lookup
  lies outside the table's supported domain; no output value is fabricated.
- `NumericalError { node, reason }`: division by zero, nonfinite arithmetic or integer overflow.

Malformed facts, package mismatch, unknown programs and resource exhaustion return
`RuleError`. Finite quantities and bounded integers remain enforced; this component does
not turn arithmetic infinity into a valid full-build metric classification.
`owner_programs_closure` preserves Partial versus Complete membership in the report.
Successfully running the known programs cannot close an unconverted owner's rule inventory.

`RuleStorageLimits` bound bytes, owners, programs, tables, table cells, reads, nodes, edges,
effects and aggregate gap evidence. Edge accounting includes expression-to-read and
expression-to-table references as well as key-node dependencies. Engine
`RuleLimits` adds compilation/execution work limits and has separate, smaller component
bounds. Both currently permit tightening their default maxima, not arbitrary increases;
full-game capacity remains to be measured. Immutable compiled packages can be shared;
each worker owns a `RuleScratch`. Scratch is cleared before every evaluation, including
validation failures, and contains no package or provider authority. Independent scratch
and reuse tests are component isolation evidence, not whole-build throughput measurements.

## CLI and present evidence

The thin [CLI adapter](../src/owned_rules.rs) uses the same libraries:

```sh
poe-optimizer check-owned-rules rules.json --definitions definitions.json
poe-optimizer check-owned-rules rules.json --definitions definitions.json --probe probe.json
```

A probe contains `owner`, `program` and `facts` entries with `read` and `value`.
`--canonical-output new-rules.json` writes a new file after successful validation.
The report retains stored/compiled/schema identities, resource counts, partial-owner count
and optional component output. Its verification fields explicitly record provider resolution
as not run and whole-build parity as not established. This command is a component validator;
it does not consume a BuildSpec as an evaluation request.

The [test helper](../crates/poe-optimizer-engine/tests/support/owned_rule_fixture.rs) directly
authors small synthetic local-item, attribute-condition, support and grant programs.
[Engine tests](../crates/poe-optimizer-engine/tests/owned_rules.rs) exercise these, changed data,
quality absence versus missing facts, lazy demand, exact target/declaration checks, bounds
and scratch reuse. Their constants and explicit input facts are component fixtures, not a
generated semantic package for Twister, Sniper or the other protected originals. Retained
PoB reports and old numerical matrices remain independent evidence to migrate; this slice
does not claim fresh original-build parity or replace a legacy numerical consumer.

Real level/quality components now ship as persisted data through the
[offline recipe assembler](owned-offline-data.md). The source-free assembler and emitted
artifacts use these same APIs; all unconverted mechanics and routes remain partial.

## Next integration and retirement gate

D2 still needs reproducible offline conversion of real local-item, effective-attribute,
support and provider-grant semantics into these owned declarations and rules, with source
pins, conversion diagnostics and independent numerical evidence. Identity-only catalogs and
fixed reward input schemas do not establish effect coverage.

D3 now binds supported provider, actor and action occurrences and rejects conflicting
producers/cycles. It conservatively withholds final values whenever discovered coverage is
incomplete; the effect ledger retains component evidence. Potential gem/actor skills do not
activate merely by appearing in a schema list. Unsupported support, payload, usage and granted
allocation relations remain gaps, including dependent action/route contexts. Generated
Skill versus traversed Provider choices use declared scopes; ambiguous scopes remain
unsupported. Support/cost/legality semantics and the requested metric ledger remain open.
Develop Twister and Sniper together, retaining the other originals as model stress cases.
Neither raw facts nor a successful component result may become a bypass around that plan.

Real inputs already require typed generated-skill parameter transfer from item grants and
transport from equipment-local stats to an action's selected attack source. Do not represent
an item-granted skill as a physical gem to reuse `GemLevel`. During offline conversion,
keep a parameter's supported computable domain separate from gameplay roll bounds. An
explicit item effect that disagrees with crafting metadata must retain its value and
provenance; assess the legality discrepancy separately instead of silently clamping,
double-counting, or making a computable original structurally unbindable.
The public component API still validates/hashes supplied facts. The plan uses a private
prepared single-effect ABI sharing that same executor; evaluation performs no source/index
lookup, serialization or hashing. Plan preparation binds and hashes an exact request snapshot.
A changed candidate currently requires preparation again. Reusable candidate input slots and
incremental invalidation remain a separate measured performance gate; no search throughput
claim follows from repeated evaluation of one snapshot.

`resolve-owned-effects --input REQUEST --schema SCHEMA --rules RULES --routing ROUTES`
exposes this boundary without PoB. An optional `--output REPORT` writes a new report. The CLI
requires explicit input/data paths and keeps typed effects/values distinct from MetricDef
results. Routes are injected data: exact output and selection, with either a player equipment
slot or the selected action's actor as source. Missing/partial routing is not an empty route
set; multiple routes to a final target reject. No weapon is selected by position or name.

The next numerical migration must move a named legacy consumer and preserve its useful
numeric/reference laws in the same checkpoint, then delete the replaced request/profile
path and dependency closure. See [retirement inventory](legacy-retirement.md). No third
profile, mandatory source-program package or parallel default evaluator is introduced here.

The optional [item source tests](../crates/poe-optimizer-engine/tests/owned_item_reference.rs)
now compare the owned rule engine with independently executed pinned local-item arithmetic.
They vary quality and speed, preserve absent damage channels, and exercise missing-input
recovery. The granted-skill comparison uses explicitly supplied decoded Integer levels;
source range decoding and receiving-provider resolution are not established by that test.
Neither the oracle outputs nor the legacy Spark/Mace evaluator supply native results.
