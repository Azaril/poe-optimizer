# Staged actor contributions and ordered reductions

Status: proposal only; owner discussion is required before changing shared Core, rule,
build or evaluation contracts. This document does not approve a schema migration or a
new native stage API. The current provider checkpoint is `runs/owned-passive-attributes-01/package`;
complete original-build evaluation remains 0/5. See [attribute resolution](owned-attributes.md),
[domain architecture](domain-architecture.md) and [the implementation plan](implementation.md).

The next reusable calculation boundary is a **stage-local query over actual contributor
occurrences**, preserving their numeric types, order, grouping and activation. A scalar
formula supplied with precomputed BASE/INC/MORE totals cannot establish that boundary.
Ordinary passive providers can continue independently while this contract is reviewed.

## What exists and what is missing

The current package has three Integer Actor contribution channels (`1d2e`, `1d2f`,
`1d30`) populated by class bases, reviewed attribute-choice passives and four formatted
item-attribute families, plus the reviewed ordinary full-list passive inputs. The input
component introduced in `53c9d04` is a transitional
producer seam, not semantic permission to regroup values or treat attributes as resolved.
The package's fourteen registered receivers do not yet calculate final player attributes
or resources. All existing Partial owner closures remain relevant.

Ordinary rules already express explicit reads, lazy selection, comparisons, arithmetic,
rounding, Count-to-Integer conversion and contributions. Separate stage-qualified owned
Stat IDs can represent immutable intermediate values. No new opcode, loop, mutable actor
output dictionary or runtime source interpreter is necessary for two finite passes.

The production resolver currently keys a contribution channel by **entity, stat, kind**.
It folds all matching effects in compiler/provider visitation order. There is no authored
ordered membership, reduction-group identity or query-local shared limit state. A stat
has one final producer; evaluating the same unqualified producer again with a different
implicit attribute snapshot is not supported. Using the final Strength key as both input
and output creates a real dependency cycle rather than a second pass.

These are distinct problems:

| Concern | Existing representation | Additional contract needed |
| --- | --- | --- |
| Six attribute evaluations and two condition snapshots | Distinct Stat IDs, ordinary producer programs and stat receivers | None for explicitly lowered finite stages |
| Stage-specific live reads | Explicit reads of the appropriate earlier Stat IDs | Authoring bindings; a runtime stage context is optional, not required |
| Quantity BASE, percentage INC, factor MORE | Existing types and contribution kinds | No arithmetic opcode change |
| One final value per stage | Existing single-writer validation and acyclic plan | None |
| Complete ordered contribution membership and reduction groups | No general representation | Versioned shared rule/plan binding contract |
| Query-local limits across contributor occurrences | Independent programs and Sum/Product are insufficient | Explicit bounded shared-limit semantics, or retain affected families as unsupported |
| Proven source correspondence | Optional importer/oracle | Keep outside native runtime |

## Finite stages and live snapshots

Use domain-owned stage symbols in authoring data. They identify evaluation points, not
PoB function names, UI groups or mutable tables. A stage manifest can lower these symbols
to ordinary owned Stat IDs and program IDs before publishing the runtime package.

The required order is:

`S1 → D1 → I1 → C1 → S2 → D2 → I2 → C2 → inherent bonuses`

Each modifier query within a stage sees the following immutable input snapshot:

| Evaluated output | Strength read | Dexterity read | Intelligence read | Comparison conditions |
| --- | --- | --- | --- | --- |
| S1 | S0 | D0 | I0 | C0 |
| D1 | S1 | D0 | I0 | C0 |
| I1 | S1 | D1 | I0 | C0 |
| S2 | S1 | D1 | I1 | C1 |
| D2 | S2 | D1 | I1 | C1 |
| I2 | S2 | D2 | I1 | C1 |

C1 uses S1/D1/I1; C2 uses S2/D2/I2. Attribute-comparison reads in D1 or I1 must not see
conditions recomputed immediately after S1. Derived fields such as LowestAttribute also
need explicit snapshot bindings. TotalAttr is produced after the second pass, not silently
made available to an earlier query. Non-attribute reads retain their own reviewed stage.

A conditional contributor is evaluated separately for each relevant stage, with the same
physical provider identity and a distinct program/application identity. For example, a
Dexterity-scaled Strength modifier reads D0 in S1 and D1 in S2. Neither invocation reads
final Dexterity. Finite authoring expansion can produce these two ordinary programs now;
a generic runtime stage-parameterized program would require a separate versioned design.

Each comparison output can be a Boolean stat with its own receiver. Current receivers
require exactly one final Derive to their owned stat; they cannot publish an arbitrary
multi-output condition bundle or emit contributions. The source comparison set includes
six strict pairwise comparisons, three tied-or-highest comparisons, Int/Dex single-highest,
and TwoHighestAttributesEqual. It does not define a matching StrSingleHighest condition
in this function. LowestAttribute is the minimum of the three evaluated attributes.

Cold attribute outputs can be explicit zero seeds only under the reviewed initialization
contract: the source resets player output before this chain and missing GetStat values
then return zero. C0 is a separate contract. Source conditions can come from setup, parent
conditions and condition modifiers. A false comparison write does not necessarily mask a
true parent condition or a condition FLAG. Missing evidence must not become false.

For example, with base Strength=10 and Dexterity=10, one added Strength per Dexterity and
one added Dexterity per Strength, the sequential chain gives S1=10, D1=20, S2=30, D2=40.
A simultaneous vector update gives S2=D2=20. With Strength base 10, Dexterity base 20 and
+100 Strength while Dexterity exceeds Strength, a false C0 gives S1=10, C1=true, S2=110,
C2=false. The calculation stops there; recomputing to convergence changes the mechanic.

## Numeric channels and exact operation order

Stage BASE values must be Count quantities, because conditional scaling can produce a
fraction before the final attribute round. The existing Integer channels remain useful
for their reviewed ordinary providers; they cannot accept arbitrary fractional BASE.

Two migrations are possible:

1. Preserve the Integer channels as historical outputs and add a Count-valued projection
   **at each contributor occurrence**, retaining occurrence order and group membership.
2. Migrate those providers to Count channels through an explicit package transition while
   preserving old artifact compatibility and the tests for their existing integer semantics.

Do not replace the entire ordinary Integer channel by one summed Count contribution unless
its regrouping is proved harmless for the admitted domain.

Plan one explicit cutover for these ordinary providers. During migration, compatibility
checks may retain the old outputs, but the production consumer must use only one selected
path. Remove superseded Integer-to-aggregate adapters and tests that assert obsolete wiring
after their numerical invariants move to the occurrence-preserving Count path. Two permanent
flat/staged calculation pipelines would obscure which one owns attribute semantics.

Exact integer addition alone does not prove equivalence after interleaving fractions.
For binary64 arithmetic:

- `((1,000,000 + 0.49999999997) - 1,000,000)` becomes `0.5`, then source rounding gives 1.
- `(1,000,000 - 1,000,000) + 0.49999999997` rounds to 0.

This is a numerical witness, not a claim about an original build. It demonstrates why an
aggregate lift must not become the general source-parity contract.

For each stage, preserve `base * (increased_factor * grouped_more)`. The source computes
the inner factor first; `(base * increased_factor) * grouped_more` is a different floating
point expression. If BASE is exactly zero, INC/MORE reads are not demanded.

Source final rounding is `floor(value + 0.5)`, followed by `max(..., 0)`. Author those
operations explicitly. Keep the rounded value a quantity until after the clamp, then
convert the nonnegative result to bounded Integer. Converting an extremely negative value
to Integer before clamping can report overflow where the source would return zero.
Positive out-of-domain results still fail explicitly. Do not substitute mathematical tie
rounding or a decimal-quantum division when the reference performs multiply/add/floor/divide.

## Ordered groups are semantic inputs

Propose a bounded owned **contribution query specification** whose evaluated membership
contains exact effect occurrences, partitioned into ordered groups. A query identifies:

- its actor, numeric channel/kind and finite stage;
- group definitions and their composition order;
- an exact order of contributor occurrences within each group;
- the group's reduction, numeric identity and any declared post-reduction arithmetic;
- independently checked membership closure and the applicable occurrence activation gates.

The concrete bound sequence must retain provider identity, program and effect identity,
plus any stage-local application identity. Two equal definitions or equal values remain
two occurrences. Inactive members do not contribute; missing or incomplete members do not
become an empty identity. Ambiguous group assignment, duplicate sequence membership and
unaccounted relevant occurrences fail validation.

Definition data can state which group role and ordering policy apply to a provider effect.
The owned binder resolves that policy against candidate occurrences. If semantic order is
not derivable from existing owned relationships, add explicit typed order input rather than
infer it from allocation IDs, object-map iteration, names or source traversal. A fixed list
of instance IDs imported once is insufficient for optimization: candidate edits must produce
a newly bound, validated sequence. The sequence and its policy must participate in plan
identity. This is shared-contract work, not an unchecked sidecar accepted by the executor.

Do not decide the final DTO names or a broad selector language in this document. Before
implementation, demonstrate the policy on class, passive, equipment and repeated-modifier
occurrences, including changes introduced by search. Provenance describing how source
insertion order was observed belongs to the adapter/oracle, separate from the owned order.

For the reference behavior, the required reductions include:

- **BASE/INC:** a left fold inside each local group; combine that result with its parent
  query result afterward. A flattened sum does not preserve the grouping of floating values.
- **MORE:** form each eligible factor as `1 + percent / 100`, multiply in group order,
  round the complete group product, then multiply it by the separately processed parent.
  Parent chains retain their declared association; do not assume one unrounded product
  across every provider is equivalent.

Two groups each containing a 1.6% MORE effect produce `1.02 * 1.02 = 1.0404` after local
rounding to two decimals. Flattening them and rounding once gives `1.03`. Applied to a base
of 100 these yield final integers 104 and 103 respectively. This illustrates group boundaries,
not permission to assign one group per item or per modifier.

Group role must not be confused with actor ownership: source parent stores can contain
cached player contributions, not a parent actor. The cold and accelerated source paths
can partition stores differently. Investigate and document that behavior with the oracle
before adopting a canonical grouping policy. A cold-only correspondence must be labeled;
a warm/cold difference requires an explicit parity decision, not hidden source-cache entities
in the native model.

Some source queries also share a limit across multiple modifiers and reset that limit for
each local query. Independent producer evaluation followed by Sum cannot reproduce this
state. Decide whether a bounded ordered-fold limit operation is needed for the admitted
families; otherwise keep those families Partial. The stage proposal does not authorize a
general mutable interpreter or silently discard shared limits.

## Smallest reusable implementation boundary

Prefer a compact authoring manifest that expands the six evaluations and condition snapshots
into existing typed rules. Validate all stage dependencies, earlier-snapshot substitutions,
units and unique final writers before publication. The native worker continues executing
an immutable compiled DAG with private scratch. A stage must not mutate a shared actor
object or create an implicit feedback edge.

Separately, review the ordered contribution query binding above. Most arithmetic can remain
ordinary typed expressions after the binder supplies a group aggregate. A group-specific
Stat channel can represent the result, but allocating that channel does not by itself solve
concrete member assignment or ordering. Reusing existing Sum/Product is safe only when their
bound sequence and required grouping have been proved equivalent for that query.

Inherent Life/Mana/Accuracy bonuses consume final attributes and final condition resolution.
Keep global/per-attribute disable flags, doubled bonuses, halved Life-from-Strength and the
Accuracy override/default explicit. Zero is a valid override and cannot mean absent.
Publish these as derived numeric inputs to the later resource-stage recipe. This fits the
current one-Derive receiver contract and does not require contributor-producing receivers.
Such a receiver extension is a separate option only if a later consumer establishes a need.
Do not duplicate the same actor mechanics under each class simply because class providers
currently instantiate Actor-context programs.
Resource baselines and final resource values remain separate from inherent bonus amounts.

The meaningful production milestone is an owned request whose actual class/passive/item
occurrences flow through stage-specific evaluation, ordered membership, condition snapshots
and dependent bonus inputs using the same resolver as search. Its evidence must include a
conditional contributor that sees different snapshots, an activated/inactive provider, a
fractional BASE value and multiple rounding groups. Supplying scalar totals directly to a
helper or marking incomplete real owners Complete is not that milestone.

## Alternatives and tradeoffs

- **Finite stage-qualified stats plus an ordered-query seam (preferred proposal):** reuses
  existing expressions, single-writer checks and DAG execution. Published data is larger,
  but all dependencies are inspectable. A bounded authoring compiler avoids hand-copying
  many programs. Shared changes are limited to membership/order/group semantics.
- **Runtime stage contexts and parameterized contributor evaluation:** can share compiled
  programs across snapshots and reduce package size. It requires stage-aware invocation,
  cache and contribution keys, explicit read rebinding and more complex cycle validation.
  Consider it only after finite lowering proves the required semantics and costs.
- **One dedicated Rust attribute algorithm receiving pre-aggregated totals:** fast and small,
  but does not solve contributor scope, grouping, order or conditions. It may be a reusable
  numerical kernel; it cannot replace the production query boundary or certify real builds.
- **Canonical unordered/exact reductions:** potentially simpler and more stable, but a
  numerical semantics change relative to pinned binary64 operation order and group rounding.
  Adopt only after an explicit owner decision about parity, with recorded differences.
- **Source-shaped mutable stores or a source interpreter:** retain the dependency and cache
  behavior the owned design is intended to remove. They are optional reference tooling,
  not the proposed native data model.

## Migration and validation gates

1. Audit source cold/accelerated grouping and initialization with authenticated optional Rust
   reference tests, including noncommuting arithmetic witnesses and actual modifier records.
   Inventory admitted and unsupported stage-dependent families without altering the originals.
2. Review and version the minimal ordered-query contract, including candidate-edit ordering,
   exact occurrence membership, group closure, bounds and identity. Preserve legacy package
   bytes/digests when the new contract is absent. Do not weaken whole-plan coverage gates.
3. Publish a finite stage recipe and per-occurrence Count projections through the checked
   package transition. Keep prior class/item/passive closures and all original inputs intact.
   Do not edit protected allocation accounting or use imported text as runtime dispatch.
4. Exercise the real owned resolver with conditional and repeated occurrences, snapshot
   changes, signed/fractional values, ties, zero-base lazy reads, grouped MORE, overflow,
   missing seeds, unsupported limits, inactive paths and incomplete collections. Check cycle
   rejection and competing writers, then changed candidates and independent worker scratch.
5. Compare intermediate stage values, condition flags, contributor sequences and inherent
   bonuses with the optional oracle before comparing final build metrics. Preserve the five
   originals and 110 queries; add held-out builds rather than tailoring APIs to Twister/Sniper.
   Real builds remain unresolved until all required providers and coverage obligations close.

## Code and source evidence

Five optional Rust tests in `crates/poe-optimizer-pob/tests/owned_actor_attributes_reference.rs`
now exercise authenticated complete source functions with controlled actor/modifier inputs.
They cover sequential reads, two-pass conditions, rounding, group boundaries and inherent
bonus flags/overrides with JIT disabled/enabled. They do not establish compiled traces,
actual cold/accelerated setup equivalence, native stage execution or whole-build parity.

- `owned_rules.rs` defines the current RuleReadSource, contribution kinds and mathematical
  expressions. `owned_rules/compile.rs` checks Add against the exact stat type; Increase
  uses PercentagePoints and Multiply uses DimensionlessFactor independently of that type.
  Its receiver validation requires exactly one final Derive to the receiver-owned stat.
- `owned_plan.rs` defines ProgramOccurrenceKey, PlanValueKey and ContributionKey.
  `owned_plan/compile/reads.rs` binds reads directly to those keys. `owned_plan/compile.rs`
  records contributions in visitation order, applies global completeness to reductions,
  rejects competing final producers and validates the effect DAG. `owned_plan/graph.rs`
  performs the stable left fold; it has no authored group or rank input today.
- Pinned optional source `CalcPerform.lua:233` supplies the six sequential attribute steps
  and comparison snapshots; `:496` supplies inherent bonuses; `:1214` resets player output.
  `CalcTools.lua:16` and `:50` fix factor association and the zero-base fast path.
  `Common.lua:722` fixes source rounding. `ModDB.lua:137` and `:214` show local/parent Sum
  and MORE grouping. `ModStore.lua:409`, `:469`, `:605` and `:1074` show condition fallback,
  live stat reads, per-stat scaling and query-local limits. `CalcSetup.lua:914` and
  `Common.lua:542` show the cached-parent boundary requiring reference investigation.

Source names above are review references. They are not fields or dispatch keys proposed for
the native runtime.
