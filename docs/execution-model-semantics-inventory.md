# Execution-model semantic boundaries

A1 baseline: published `9c75622`, 2026-09-12. This inventory supports the
[execution-model investigation](implementation.md#execution-model-investigation-a1-a4);
it does not select a replacement runtime. Native evaluation still completes **0/5
supplied original builds**. All five import/inspect and run in pinned PoB. Source
component parity and the existing Spark/Mace numerical paths do not change that
denominator. See the [real-build rollout](real-build-rollout.md) for the retained
selections, output meanings and acceptance gates.

The decision is where to establish an equivalent domain result, then which machinery
to use behind that boundary. A domain DSL, data-driven rules, native Rust families and
an interpreter are not mutually exclusive representations or execution strategies.
Moving interpretation into preparation is a separate question from eliminating it.
This document identifies the proof obligations; it makes no speed or complexity winner
claim.

## Three contracts that must remain distinct

| Boundary | Contract to preserve | What it does not establish |
| --- | --- | --- |
| End-user build/evaluation | Resolve the requested saved views and actor/action identities; retain active effects, source data and scenario; return the specified metric values **and availability**, resource/feasibility outcomes and relevant diagnostics. Controlled changes, saved/exported builds and fresh reimports must agree with the reference. | Equal metrics on one baseline do not prove correct edit behavior, supporting actors, unavailable metrics, exports or the other four originals. Unobserved Lua implementation details need not become public product features. |
| Original public parser compatibility | Preserve the actual public `parseMod` wrapper's result packs, exposed cache, state transitions, writable copies, retained functions and reached error effects for the admitted inputs. These requirements already govern the source/session work. | This API is broader than a pure `text -> modifier records` function. A narrower domain parser would require an explicit compatibility boundary and consumer proof, not relabelling unsupported calls as no-match. Complete native public-parser compatibility is still unfinished. |
| Existing Rust APIs | Keep the admitted contracts of the stateless structural parser, finite item-metadata adapter, preparation reports, opaque owner-bound handles and typed calculators. Their current unsupported cases remain explicit. | The structural parser is not the complete stateful public source parser; the inspector's general item prefix is not the numerical backend's equipment path. A source body that compiles is not automatically an admitted whole build. |

The public source wrapper is in
[ModParser.lua](../vendor/path-of-building-poe2/src/Modules/ModParser.lua) (the final
returned function). It exposes `cache`, keys it by the original line, retries order two
only after a first result containing both modifiers and extra text, and returns
`unpack(copyTable(cache[line]))`. Startup also populates the externally exposed cache
in [Main.lua](../vendor/path-of-building-poe2/src/Modules/Main.lua).
The [parser session contract](parser-sessions.md) records why cached no-match, missing
results, empty text and mutable dictionaries cannot be collapsed into one pure lookup.

The existing [CompiledModifierParser](../crates/poe-optimizer-engine/src/modifier_parser.rs)
implements its admitted structural parsing/retry/copy path with per-request state.
[NativeModifierParserProvider](../crates/poe-optimizer-import/src/item_loading/parser.rs)
then converts supported results into finite item metadata and rejects callbacks,
non-finite numbers and non-UTF-8 text. It preserves Source/Resource/Unavailable
classification. This is an explicit narrower seam, not evidence that those values are
irrelevant to every original consumer.

## Consumer inventory

“Simpler model” below means a proposed domain boundary. Each row states what would have
to be proved before removing the corresponding lower-level behavior.

| Actual consumer and current integration | Observable domain contract | Lower-level dependencies currently encountered | Proof needed for a simpler model |
| --- | --- | --- | --- |
| **Import, saved-view selection and authored loading.** [prepare_view](../crates/poe-optimizer-native/src/preparation.rs) runs authored configuration/skill preparation before checking the narrow numeric profile. [Native configuration](../crates/poe-optimizer-native/src/configuration.rs) explicitly reports the unexecuted activation continuation. | Preserve source instance identity, independently selected sets, authored values, defaults/migrations, load order and the first reached invalid/unavailable dependency. Missing data is not a fabricated default. | Ordered source sections; nil/false/zero/empty distinctions; fresh default tables; instance aliases; failures after an executed prefix. Full original configuration activation is not supplied by these loader stages. | Define a typed load result and ordered transition rules with equivalent effective state and diagnostics. Demonstrate that subsequent consumers cannot distinguish removed intermediate state, including on failed loads and selected-view changes. |
| **Configuration controls and dispatch.** Original [ConfigTab:BuildModList](../vendor/path-of-building-poe2/src/Classes/ConfigTab.lua) calls `UpdateLevel`, then visits `varList` in order using selected inputs/placeholders. The [configuration contract](configuration-preparation.md) includes the preceding controls and loadout lifecycle. Current source tests exercise components; production preparation still stops before full activation. | Same effective configuration, player/enemy modifiers, selected control values and later callback effects. Rendering may be omitted; state written by control notifications cannot be omitted merely because it originates in UI code. | Class lookup/raw shadowing; receiver identity; mutable closures/cells; target-before-argument lookup; input-versus-placeholder rules; ordered callbacks and failure prefixes. | Produce an explicit configuration state machine or dependency plan whose transitions match complete original lifecycles, including shared controls, changed defaults and failures. Prove any reordered independent transitions commute. |
| **Quest and custom modifier blocks.** [ConfigOptions.lua](../vendor/path-of-building-poe2/src/Modules/ConfigOptions.lua) uses `applyModsFromString`/`questModsRewards`; [ConfigTab.lua](../vendor/path-of-building-poe2/src/Classes/ConfigTab.lua) handles enabled custom blocks and the legacy fallback. | Same line splitting/cleanup, parser acceptance/remainder, modifier insertion order and source attribution. Empty, disabled, missing and malformed text have distinct effects. | Escaped `gmatch` callable state; full parser result packs/cache; `#`/`ipairs`; writable returned records; [setSource](../vendor/path-of-building-poe2/src/Modules/ModTools.lua) mutates both outer and nested records before returning the same object. | A typed stream of attributed modifiers must preserve parsing policy, rejected/extra text and nested attribution. Show that lost aliases, callback values or partial writes cannot affect the rest of the preparation, or represent them explicitly. |
| **Item lines, ranges, variants and slots.** Original [Item.lua](../vendor/path-of-building-poe2/src/Classes/Item.lua) parses a ranged line, may try it combined with the next line, and retries the original line if that attempt fails. Slot assembly copies/modifies records. [build_inspect](../src/build_inspect.rs) uses the general [item provider](../crates/poe-optimizer-import/src/item_loading/provider.rs); native numerical preparation uses the narrower [equipment parser](../crates/poe-optimizer-import/src/equipment.rs). | Same consumed lines, range/scalar interpretation, variants, local versus global effects, requirements, source item/slot identity and unavailable assembly dependencies. | Repeated parser calls and their state; writable copies; tag/callback captures; ordered lists and source stamping; different slot consumers. | Prove a stable typed item result is sufficient for **all reached downstream consumers**, not only inspection. Keep local weapon/armour operations separate from global actor effects and revalidate after slot/provider changes. Finite metadata conversion alone is not full item assembly. |
| **Public parser cache and returned copies.** Final wrapper in [ModParser.lua](../vendor/path-of-building-poe2/src/Modules/ModParser.lua), recursive [copyTable](../vendor/path-of-building-poe2/src/Modules/Common.lua). | Same miss/hit/no-match transitions, exposed cache aliases, returned pack arity, independent writable copies and callable identity. On failure, earlier cache/dictionary writes remain visible. | Mutable cache; table/function identity; recursive copy without alias memoization; raw traversal; sparse outer packs and `unpack` length. Equal raw entries do not necessarily determine source length or legal `next` controls. | Either retain this API or explicitly introduce a domain service with demonstrated consumer equivalence. A generic memoizing graph clone changes source copy semantics; a new container order needs proof that all downstream uses are order independent. |
| **Inner parser dictionary scan and factories.** `scan`, the inner parser and the `DOUBLED` branch in [ModParser.lua](../vendor/path-of-building-poe2/src/Modules/ModParser.lua). | Same chosen rule, capture pack/remainder, shared dictionary mutation and behavior of any returned callable/tag. Rule data and skill names must still come from the injected definitions. | `pairs` order can break exact ties after earliest-match/end/pattern-length comparison; selected dictionary values retain identity; `DOUBLED` mutates a selected name table; generated closures share live cells; numeric/string conversion and source operand timing can affect results. | Establish an explicit rule priority and typed result/capture vocabulary that reproduces all admitted choices and future state effects. Prove tie impossibility or preserve observed priority. Replacing a callback with a record requires proving what its eventual callers observe. |
| **Modifier storage and actor assembly.** Original [ModList](../vendor/path-of-building-poe2/src/Classes/ModList.lua) appends, searches, replaces and merges ordered records. The admitted native [actor program](../crates/poe-optimizer-engine/src/actor_program.rs) compiles bounded typed records. | Same conditions/flags, source selection, first-match replacement, parent/layer semantics, numeric order and per-layer rounding. Effects must belong to the correct actor and action. | List order; parent fallback; aliases during replacement/copy; MORE rounding and override precedence. A map/set of equal-looking modifiers need not be equivalent. | Define typed ordered layers and precedence, then prove the retained vocabulary covers each active producer. The current typed implementation is a useful precedent, not proof that the full modifier/tag language or actor graph fits it already. |
| **Candidate admission, calculation and verification.** [ControlledBuildDomain::prepare/admit](../crates/poe-optimizer-import/src/controlled_build.rs), [PreparedBuildCandidates::calculate](../crates/poe-optimizer-native/src/build_candidates.rs), and [build_search](../src/build_search.rs). | Same legal/illegal selection, resulting actor resources, requested metrics and availability; owner-bound inputs; correct document materialization and fresh verification after mutation. | Current admitted calculation uses typed numeric components rather than Lua tables. Candidate preparation still orders and combines fragments, computes actor resources and assesses requirements; finalist verification reconstructs and prepares a document. | Preserve a dependency/invalidation boundary that recomputes everything changed by a candidate. Moving interpretation out of repeated calculation is useful only if its setup and repeated invalidation costs are included, and stale prepared state cannot survive a relevant change. |

The current all-five successful-public-parse frontier is concrete: six positive families
per build write their source-equivalent cache row, then native execution stops in
`Common.copyTable` because traversal of the first modifier record is unavailable.
R2t matched the actual failed table at `cache[line][1][1]` and the exact copy callback;
it did **not** establish the record's allocation origin or complete the returned copy.
See the current [implementation checkpoint](implementation.md). This is a useful
comparison case for alternatives: a domain result could avoid reproducing hash layout,
but must first preserve the consumer-visible copying, ordering and state behavior above.

## What is actually in the repeated candidate path

The present bounded path has three separate cost/lifetime layers:

1. **Request/setup.** [prepare_request_with_lineage / prepare_view](../crates/poe-optimizer-native/src/preparation.rs)
   imports source XML, resolves the view, prepares authored stages and validates that the
   narrow numeric projection agrees. Controlled search constructs its catalog/domain and
   [fixed native preparation](../crates/poe-optimizer-native/src/build_candidates.rs)
   once, including admitted item/passive/support components and fixed scenario checks.
2. **Per-candidate assembly/admission.** [Domain::prepare](../src/build_search.rs) runs
   `ControlledBuildDomain::admit` in Rayon with worker scratch. That call resolves the
   allocation, orders configuration/equipment/passive fragments, recomputes actor
   resources, then assesses requirements. The resulting opaque handle retains the actor
   and selected components. This work and its allocations are part of search, even
   though they precede `calculate`.
3. **Calculation and external result work.**
   [PreparedBuildCandidates::calculate](../crates/poe-optimizer-native/src/build_candidates.rs)
   checks bindings and uses the admitted actor/components for Spark or Mace arithmetic.
   [PreparedEvaluation::calculate](../crates/poe-optimizer-native/src/lib.rs) likewise
   operates on prepared numeric inputs. Search then creates owned assessments; baseline
   and finalist document verification take the full preparation path again. Export also
   materializes source rather than treating a numeric handle as a complete document.

The shared standalone source/session machinery is currently a dependency-development
and component-execution path, not a general activation stage already wired into
`NativeBackend::prepare_view`. Its measured primitive costs cannot be assigned to every
native candidate. Conversely, measuring only `calculate` omits candidate admission,
scoring, verification and setup. The existing
[candidate calculation contract](native-candidate-evaluation.md) and
[passive/equipment assembly contract](passive-equipment-assembly.md) delimit the narrow
zero-allocation calculation claims.

### Reuse and invalidation obligations

| Change or reuse | Current implemented boundary | Requirement for the investigation |
| --- | --- | --- |
| Repeat calculation of the same admitted handle | Reuses immutable prepared inputs and actor state; validates owner/domain bindings. | Keep worker-private scratch and prove no prior candidate contaminates the result. |
| Select another admitted item, support or allocation within the controlled domain | Re-admits the selection and recomputes actor resources/requirements; reuses compiled fragments and fixed preparation. | Count this repeated cost. Compare changed candidate results with fresh document preparation, including non-additive interactions and provider removal. |
| Change source/data owner, selected view, configuration/scenario, or an unsupported candidate axis | Existing handles cannot be rebound to a foreign owner/domain; the closed prepared setup is not a general incremental model. | Specify which plans/caches are invalidated and obtain fresh preparation where reuse is not proved. Do not infer a complete dependency graph from current owner checks. |
| Edit arbitrary modifier/item text, or introduce a new source-defined rule | The inspector/provider and narrow numerical paths have explicit parsing/assembly boundaries. | Include parsing and any interpreted producer work at the frequency of such edits; do not assume every possible user string can be precomputed at data extraction time. |
| Share compiled data across workers or revisit candidate A after B | Immutable owners/components are shared; source session cells/tables and numerical scratch remain private. | Compare fresh A with A → B → A, provider removal/re-addition and shuffled parallel schedules. Serial/parallel agreement alone can share the same stale-state bug. |

An interpreted preparation stage feeding a typed native plan is therefore a candidate
architecture worth measuring. It still needs a clear result schema, provenance and
invalidation policy. Retaining PoB interpretation in a host preparation process also
changes deployment/dependency boundaries relative to the current Lua-free native runtime;
it is an option to evaluate, not an implemented portable-browser path.

## Semantic traps that determine the comparison

- **Observable state versus incidental representation.** Internal table addresses are not
  product output, but aliasing can affect later dictionary edits, source attribution,
  cache hits and returned functions. An abstraction must close over every relevant future
  consumer, not merely serialize an equal snapshot once.
- **Order and sparse length are separate facts.** Dictionary tie-breaking and modifier
  layers can observe order. LuaJIT allocation/JIT history can change sparse length and
  valid absent-key controls despite equal entries. A typed list/map can remove that
  history only after its complete consumer is shown equivalent; sorted traversal is not
  an automatic replacement. [Parser sessions](parser-sessions.md) retains the constructor
  counterexamples and explicit unsupported cases.
- **Errors are transitions too.** Source errors, unavailable semantics and resource
  exhaustion have different meanings. Current source sessions retain successful writes
  before a later error and cumulative work/allocation charges. A transactional domain
  operation would need an explicit changed API or proof that the failed state is never
  reused. It must not turn unsupported into empty/no-match or publish partial metrics as
  a completed build.
- **Closures can sometimes become data, but only with a defined caller.** An environment,
  typed predicate or operation record could replace a function. It must reproduce shared
  cell updates, fresh identity where observed, invocation effects and lifetime across
  repeated calls. Source operand/register timing matters while executing the original
  language; a higher-level plan needs proof at its chosen result boundary instead.
- **Numerical and data contracts survive implementation changes.** Condition/flag masks,
  ordering, rounding, coercion and signed-zero/non-finite policy cannot be replaced by
  convenient host defaults. Keep actual game definitions injected. Matching original
  helpers with native primitives does not justify hardcoding their high-level game
  behavior or silently normalizing transport values.
- **Evidence scope must match the claim.** Component inputs captured at a live source
  entry prove that component, not the preceding preparation. Cold/warm comparisons must
  state the actual source trace scope; source compiler limits are not native passes.
  WASM compilation is not browser execution. None of these establishes full-build speed.

## Representative vertical slices for the investigation

Use unchanged originals and the existing selection contract. Reduced adversarial cases
supplement these slices; they do not replace blocked members of the five-build set.

| Slice | Required mechanism coverage and observations | Status/decision use |
| --- | --- | --- |
| **Configuration → custom/quest text → public parser → attributed modifiers, all five** | Selected inputs/placeholders; complete callback ordering; miss/hit/no-match, changed dictionaries, fresh copies, nested source attribution and failure prefixes. Include the currently failing successful modifier copy. | Compare the current source/session boundary with a typed preparation result or bounded DSL on the same whole consumer. Full activation remains blocked. |
| **Item/provider → actor/action ownership, all five** | Line 1 Kelari/Sand Djinn and allocation provider; line 2 Twister and granted/supporting actions; line 3 Whirling Assault/Living Lightning and parent/armour effects; line 4 Crossbow/Tornado instance and trigger distinctions; line 5 Sniper minion/offering ownership. Retain independent selected sets, item parsing/slots and unresolved identities. | Tests whether an alternative's schema covers contrasting mechanisms rather than another single-skill profile. Full effective preparation is not yet available. |
| **Prepared plan → controlled edit → fresh whole-document result** | Develop line 2 Twister/player and line 5 Skeletal Sniper/minion together; preserve all five model cases. Change supports/items/passives/providers and compare required outputs, availability and feasibility after A → B → A and export/reimport. Add explicitly identified mapping scenarios without changing the original bossing observations. | This is the eventual end-user parity boundary. Keep all five in the report and mark Blocked/Not run rather than substituting existing Spark/Mace numerical wins. |
| **Current admitted hot path as a control** | Existing Spark/Mace setup, candidate admission, repeated calculation, owned results and finalist verification; serial and parallel reuse with injected data changes. | Supplies an implemented cost/reuse comparison while the original-build slices are blocked. It is not a sixth replacement fixture or evidence of general numerical coverage. |

For every candidate implementation, record the boundary it promises, preserved and
replaced mechanisms, source/data dependency, failure policy, setup and invalidation work,
worker storage and repeated evaluation separately. Require the same versioned outputs
and mutation cases before comparing speed. The next architecture decision can then say
which complexity is necessary at the chosen boundary and which complexity can be removed
with evidence; this inventory alone selects neither a runtime nor a numerical winner.
