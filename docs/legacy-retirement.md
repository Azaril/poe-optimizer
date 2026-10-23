# Legacy retirement register

Updated 2026-10-23. [Domain architecture](domain-architecture.md) defines the
single end state; [implementation R0-R6](implementation.md#delivery-plan-and-gates)
is the schedule. This register classifies deletions and semantic questions; it is
not another plan. Older detailed audits remain available in the
[pre-migration register](https://github.com/Azaril/poe-optimizer/blob/fa49fcea32f0413cde5f5983e179c34e6c7acbdf/docs/legacy-retirement.md).

## Retention rule

Retain code only for a named live consumer in owned authoring, external-format
Import, native evaluation/search or the actual optional PoB oracle. An old command,
test, receipt or source-file pin is not itself a consumer requirement. Preserve
useful semantic fixtures, then delete redundant machinery with its last consumer.
No shipped-format compatibility is required. Do not maintain V1/V2 readers,
aliases, old profiles or a second evaluator for historical test reproduction.

The cached maintained-release harness now owns scalar, Actor attribute and the
four remaining defence-component numeric laws. Their superseded numeric helpers
and callers are removed. Outer import/layout/publication assertions and Cleric
whole-plan cases remain live until their useful behavior has a maintained
replacement; this is not yet retirement of the historical publication chain.
Item-metadata preamble laws now run against the same cached release; the old
`check_preamble_cases`, its caller and private source helpers are removed.
`metadata_only` and original-build/publication assertions still have live consumers.

Do not let a refactor turn unresolved semantics into default values or complete
coverage. Delete unsupported abandoned features honestly; preserve a precise
unsupported diagnostic when the feature is needed by an actual build.

## Required deletions before the first new-format build

| Closure | Replacement / retained value | Gate |
| --- | --- | --- |
| Source-shaped native Lua parser/control/program/UI stack | Data-defined game rules; small necessary external syntax in Import; original Lua in optional offline host | R4 runtime dependency and data-type audit |
| Default legacy feature paths and embedded `game-data.json` | One injected current runtime package | Native/Wasm package and dependency checks |
| Spark/Mace profiles, orphan native crate/commands and exclusive tests | General evaluator plus independent behavioral fixtures | No name/profile dispatch; no dead compatibility feature |
| Hand-authored rule IR, hex allocation and redundant policy copies | `.rule`, KDL manifests, symbolic bindings and generated canonical output | R2 full live-program migration |
| Test-only publication chains, duplicated stage helpers and stale replay paths | Production release builder + shared harness | Clean-clone CI release without `runs/` inputs |
| Rust source/Cargo manifest/lock hashes used as behavior identity | Operation contract + compiled content; separate provenance | Formatting/refactoring does not invalidate semantics |
| Shape-count/change-detector tests, repeated test-support compilation | Invariant, property, differential and real-build checks | R0 runnable consolidated suite |
| Obsolete format readers and temporary migration utilities | Rebuilt current artifacts and reimports | R4 single-reader cutover |
| Python exporters/checkers with live purposes | Shared Rust authoring/validation implementation, or delete obsolete purpose | No Python required by supported workflows |
| Duplicate concatenated artifacts, unused compressed source copies and stale endpoints | Reproducible manifest and named package artifacts | One current release reference and size/memory guards |

The protected incoming-critical draft is not an active implementation to merge.
Its stale IDs and duplicated configuration modeling must not be revived. Useful
formula/edge-case laws can move into the new harness after reconciliation with
current definitions. Leave its working files unchanged until explicitly resumed;
its backup location is in the implementation plan.

## Lua type and behavior audit

For every surviving compatibility behavior, classify it with a concrete consumer:

| Classification | Destination |
| --- | --- |
| Proven game law: exact rounding/domain/order, source/recipient relation, stacking, resource rule | Typed owned rule or general native operation, with independent behavioral evidence |
| Necessary user-file syntax: external token spelling, absence/default resolution, item text | Bounded Import recipe/parser and sidecar; normalized typed values only |
| Source execution needed to obtain data/reference output | Optional offline PoB/mlua/LuaJIT tooling |
| Lua/UI accident or unused compatibility machinery | Delete, or preserve a narrow explicit reference discrepancy if observable |

Audit nil/truthiness, positional/source arrays, metatables, table aliasing,
callback captures, ordering derived from insertion, source-number parsing and
UI initialization/cache state. Do not reproduce a whole Lua runtime because one
external syntax rule is needed. Conversely, do not delete a numerical behavior
without checking its admitted input domain and consumer.

Resistance terminology must be precise: ordinary/chaos resistance, caps,
penetration and other supported game mechanics are domain concepts. `legacy`
labels the obsolete application path. Useful resistance kernels can survive in
the one native engine; parallel old profiles, loaders and data formats cannot.

## Semantic investigations that remain useful

- **Frost initialization:** cold MAIN/CALCS disagreement is the accepted PoB bug.
  Exclude only the affected comparison and validate others with stable fresh
  replays. Do not implement retry-until-pass or native cache-state emulation.
  See [recorded disposition](owned-frost-bomb-usage-evidence.md#upstream-issue-disposition).
- **Kalandra's Touch title routing:** a synthetic rename of an existing rare Solar
  Amulet changed PoB output. It was not evidence of an obtainable corrupted rare
  unique. Keep names cosmetic natively and the exception restricted to those
  renamed controls; actual unique mechanics and unchanged builds are not excluded.
  See [source discrepancy](https://github.com/Azaril/poe-optimizer/blob/fa49fcea32f0413cde5f5983e179c34e6c7acbdf/docs/legacy-retirement.md#name-based-item-routing-source-discrepancy-2026-10-06).
- **Numeric semantics:** migrate exact behavior first. Review integer prefix
  bounds vs exact sums, modifier transforms, failure precedence, All/Any behavior,
  computed-quantity equality, FloorAddHalf vs existing rounding, and exact decimal
  scale handling with targeted independent cases. Do not silently reinterpret
  existing programs while raising/lowering them.
- **Coverage responsibility:** the five Flask modifier-owner gaps and ten Partial
  template owners remain unresolved. Direct delivery did not cover late grouped
  equipment-slot copies; moving/removing an issue label cannot certify another
  owner covers it. Reuse exact grouped contributions, source eligibility and
  rounding; do not add another per-stat traversal.
- **Actor vs Action eligibility:** actor-wide numbers cannot discard conditions
  depending on the consuming Action. Preserve source ownership, local/inherited
  groups and unknowns. Removed unused Enemy-flat ports need no neutral replacement;
  saved Party modifiers remain a real unresolved input where present.
- **Source selector vs supply:** source UI selectors do not authorize a physical
  source/grant. Preserve known raw input independently from unresolved usage,
  generated-source ambiguity and support-generated children. Pending public
  authority proposals remain decisions when a real required case reaches them.
- **Allocation legality:** preserve saved allocation identity and distinguish
  effective connectivity/implicit roots. The retained Sniper removal control is a
  refusal/regression case, not permission to silently prune the user's build.
- **Tables/curves:** preserve exact domain values and rounding while investigating
  lossless segments; no curve fitting or invented interpolation authority.

## Evidence and documentation cleanup

Retain raw original builds, independent source observations and valuable invariants.
Do not duplicate all historical release blobs or producer implementations to keep
old byte hashes alive. Source pin provenance and generated artifact integrity
remain useful; historical source text is available in Git. Relocate/rebase useful
checks to the shared harness and remove tests whose only assertion is layout or
receipt agreement.

The [owner review](architecture-review-2026-10-22.md) is preserved unchanged.
Authoritative overview/plan documents are consolidated in this checkpoint.
Detailed `owned-*.md` contracts and evidence may still describe historical codec
versions or callers; R4 reconciles/deletes those descriptions with the code. They
must not be treated as compatibility requirements. There will be one final design
and implementation path, with clearly labeled history rather than live alternatives.
