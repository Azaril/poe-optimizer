# Ordered contribution queries

Status: bounded implementation of the accepted [contribution-stage direction](owned-contribution-stages.md). This framework does not choose a PoE numerical grouping or rounding law, close a game owner, or make another original build executable. Game-specific adoption still requires actual consumer evidence and complete contributor coverage.

The shared contract is an optional `ordered_contributions` registry on `RulePackageInput`, enabled only by `owned-domain-operations-v21`. Each query names one Stat and contribution kind. Its fixed named groups declare a reduction, typed empty identity, and a `DeclaredSet` of exact definition-owner/program/effect members. Programs read a named group through `RuleReadSource::OrderedContributions { entity, query, group }`. The query is authored data; concrete membership is bound from the current candidate's discovered effect occurrences.

An authored member is not one runtime value. A repeated modifier produces one contribution per actual modifier occurrence. The same rolled item in two equipment uses produces separate occurrences. Removing a passive, changing an item, changing semantic modifier order, or selecting another loadout changes candidate membership. Renaming or rebasing opaque instance IDs must not change numerical order. Existing discovery determines whether alternate-loadout roots are available; the query does not introduce a second participation policy.

Every actual effect on the query's recipient/channel must match exactly one member across the **whole query**, including groups the current consumer does not read. Unassigned effects, unsupported origins, ambiguous rank ties, and unresolved providers fail closed. A Complete group can be empty only after this check; its identity must not conceal an unknown contributor. Partial registry or member inventories retain incomplete-contributor refusal. Complete query declarations do not make their owner programs, input declarations, or incoming-effect coverage Complete.

## Bounded ordering contract

Members name exact definition owners, program IDs and effect IDs. Their authored order is compared lexicographically as:

`(source_rank, slot_rank, modifier_position, program_rank, effect_rank)`

Nonapplicable slot and modifier positions are zero. No instance ID, display name, insertion order, hash order, or discovery index breaks a tie. Rank values have meaning only within the authored group; no implicit game order is inferred from a definition key.

| Origin policy | Admitted direct owner | Candidate-derived ordering component |
| --- | --- | --- |
| `Character` | Class or Ascendancy | Its singleton character root; explicit source/program/effect ranks |
| `Allocation` | PassiveNode | Its exact allocated definition; explicit source/program/effect ranks |
| `EquipmentUse` | ItemTemplate | Exact declared equipment-slot rank |
| `ItemModifier` | Modifier | Exact declared equipment-slot rank, then position in the rolled item's `modifier_order` |

Initial membership is restricted to direct roots with an empty grant path. Granted, socketed, support-assignment, reward and effect-application origins have no implicit fallback ordering. Their eventual policies need explicit design and evidence. This is a limit on the new read, not a second build model or a removal of those existing occurrences.

Within a group, all equipment members of the same origin category share the same source rank and slot map. This makes equipment-slot order and modifier order precede definition/program order. Slot IDs and slot ranks are unique, finite, known declarations. Programs of one owner share its source rank; effects of one program share source/program ranks. Exact runtime ties are errors. These restrictions prevent an author from accidentally using modifier-definition order instead of the selected item's authored modifier sequence.

The initial read scope is Player, Actor from Actor/Action programs, or Current from Actor/EquipmentUse programs, with the Stat admitting that target kind. Skill, property-owner, support and application-relative source scopes are not inferred. Expanding this domain must preserve their existing authority and readiness contracts.

## Arithmetic, scheduling and coverage

Groups reuse the existing typed reductions: Add and Increase use Sum with typed zero; Multiply uses Product with a dimensionless factor identity of one. Add retains the Stat's exact numeric type/unit. Increase uses percentage points. A group read exposes an ordinary typed value; existing rule nodes perform any explicit postprocessing or quantization. No Lua values, source tables, callback evaluator, arbitrary expression language, or implicit source-bucket arithmetic is introduced.

The framework supplies fixed named groups, not a dynamically created rounded group per item or provider. If a demonstrated game law needs such a boundary, it must be modeled explicitly rather than simulated by changing discovery order. This checkpoint does not settle the separately pending numerical policy for source-local factor multiplication or rounding.

The existing staged DAG remains authoritative. A query read accesses its underlying Stat/contribution channel for frozen-stage and readiness proofs. Those proofs remain conservative over the whole channel; naming separate groups does not authorize preparation and execution writers to share a channel with conflicting readiness roles. It cannot bypass a producer/read cycle, cross a frozen channel too early, acquire preparation authority, or read a final support-output channel as a contribution. Multi-stage computations continue to use ordinary programs and typed intermediate Stats, with explicit stage/readiness metadata. Conditions remain conditions on potential effects; a currently false guard is not permission to omit a member from the declaration. If future admitted cases require groups with different preparation dependencies, extend and validate those proofs explicitly before admitting them.

Data validates the registry's exact references, origin/owner/context agreement, slot maps, group identities, units, contribution kinds, read scopes, closures, and bounded sizes. The same reusable validator runs when Engine compiles raw rule inputs. Engine performs concrete occurrence binding and contributor coverage during candidate-plan compilation, then stores immutable ordered effect-index vectors for evaluation. Parallel workers evaluate ordinary native typed operations over their own candidate state; PoB is not involved.

## Compatibility and validation

V6–V20 packages omit the registry and retain their exact wire form, rule identity domain, receipt fields, and existing `Contributions` behavior. They reject an explicit registry or an ordered-group read. V21 requires an explicit registry, even for a deliberately empty finite fixture, and retains all earlier required effect-application/readiness metadata. Its plan identity uses `owned-effect-plan-v18`. The later [attribute consumer packet](../data/owned/poe2/3887ae68/attribute-step-consumers/README.md) opts into V21 through full release assembly; the historical release-migration V5 contract remains unchanged.

Storage budgets separately bound queries, groups, member references, declared slots and validation work. New counters are omitted when zero, preserving historical receipts. Engine additionally bounds candidate expansion and comparisons using its plan resources. Authored ordering data is included in package/plan identity; changing a rank is a semantic change.

Core/Data tests cover the closed V21 capability, historical omitted-field bytes and digests, explicit inventory admission, exact reference and unit checks, unread-group validation, duplicate membership, origin/slot policies, Partial evidence, bounds, and frozen-stage access. Engine tests cover candidate expansion, rank-based arithmetic, repeated occurrences, edits/rebases, inactive loadouts, missing membership, unsupported origins and incomplete coverage. These tests demonstrate the framework's contract, not obtainability of synthetic fixture items or a game-specific aggregation law.

The attribute consumer checkpoint binds real Original05 class/choice occurrences
to the same six receiver bodies in a finite fixture, using authenticated source
record order. Production memberships remain Partial and contain no ranks;
effective MORE remains an explicit unresolved Stat dependency. The next coverage
checkpoint must establish the admitted candidate domain and every relevant member
before making a final metric available. Structural tests and a finite fixture do
not certify a complete original build.
