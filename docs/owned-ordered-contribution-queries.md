# Ordered contribution queries

Status: bounded implementation of the accepted [contribution-stage direction](owned-contribution-stages.md). This framework does not choose a PoE numerical grouping or rounding law, close a game owner, or make another original build executable. Game-specific adoption still requires actual consumer evidence and complete contributor coverage.

The shared contract is an optional `contribution_queries` registry on `RulePackageInput`, introduced by operationsV21 and generalized by operationsV22/V23. Each query names one Stat and contribution kind. Its fixed named groups declare a reduction, typed empty identity, and a `DeclaredSet` of exact owner/program/effect members. Programs read a named group through `RuleReadSource::ContributionQuery { entity, query, group }`. The query is authored data; concrete membership is bound from the current candidate's discovered effect occurrences.

An authored member is not one runtime value. A repeated modifier produces one contribution per actual modifier occurrence. The same rolled item in two equipment uses produces separate occurrences. Removing a passive, changing an item, changing semantic modifier order, or selecting another loadout changes candidate membership. Renaming or rebasing opaque instance IDs must not change numerical order. Existing discovery determines whether alternate-loadout roots are available; the query does not introduce a second participation policy.

Every actual effect on the query's recipient/channel must match exactly one member across the **whole query**, including groups the current consumer does not read. Unassigned effects, unsupported origins, ambiguous rank ties, and unresolved providers fail closed. A Complete group can be empty only after this check; its identity must not conceal an unknown contributor. Partial registry or member inventories retain incomplete-contributor refusal. Complete query declarations do not make their owner programs, input declarations, or incoming-effect coverage Complete.

## Bounded ordering contract

Members name exact definition or admitted Actor-slot owners, program IDs, effect IDs and source origins. Numeric groups require `ordering: ordered`; each member carries a separate `ContributionOrder`, whose equipment-slot ranks exactly cover its equipment origin membership. The authored order is compared lexicographically as:

`(source_rank, slot_rank, modifier_position, program_rank, effect_rank)`

Nonapplicable slot and modifier positions are zero. No instance ID, display name, insertion order, hash order, or discovery index breaks a tie. Rank values have meaning only within the authored group; no implicit game order is inferred from a definition key.

| Origin policy | Admitted owner | Candidate-derived ordering component |
| --- | --- | --- |
| `Character` | Class or Ascendancy | Its singleton character root; explicit source/program/effect ranks |
| `Allocation` | PassiveNode | Its exact allocated definition; explicit source/program/effect ranks |
| `EquipmentUse` | ItemTemplate | Exact declared equipment-slot rank |
| `ItemModifier` | Modifier | Exact declared equipment-slot rank, then position in the rolled item's `modifier_order` |
| `ExistingActor` (V23) | Actor definition with the exact applicability declaration | Explicit source/program/effect ranks; currently the existing Player |
| `Reward` (V23) | Selected Reward definition | Explicit source/program/effect ranks; distinct occurrences remain distinct |
| `SuppliedActor` (V23) | Explicit Actor slot or its declared Actor provider definition | Exact checked supply relation and Actor recipient; explicit source/program/effect ranks |

Direct provider policies require an empty grant path. Supplied Actors instead
use the validated Actor-to-provider relation, also retained for support suffix
checks. Their keys contain the parent provider, so raw path equality or an
assumed grant depth is insufficient. An explicit slot list authorizes only those
Actor supplies; arbitrary generated, socketed, support-assignment and
effect-application origins remain unsupported. A shared Actor application is
not a fabricated provider root.

Numeric occurrences that share a recipient and semantic position reject. This
includes repeated selections of one Reward definition or several supplied
Actors writing the same Player channel without a reviewed ordering law. Separate
minion recipients do not need an ordering between their IDs. Unordered Boolean
groups preserve the full source and invocation identity instead of ranking or
deduplicating those occurrences. These are membership capabilities, not game
grouping laws or complete source inventories.

Within a group, all equipment members of the same origin category share the same source rank and slot map. This makes equipment-slot order and modifier order precede definition/program order. Slot IDs and slot ranks are unique, finite, known declarations. Programs of one owner share its source rank; effects of one program share source/program ranks. Exact runtime ties are errors. These restrictions prevent an author from accidentally using modifier-definition order instead of the selected item's authored modifier sequence.

The initial read scope is Player, Actor from Actor/Action programs, or Current from Actor/EquipmentUse programs, with the Stat admitting that target kind. Skill, property-owner, support and application-relative source scopes are not inferred. Expanding this domain must preserve their existing authority and readiness contracts.

## Arithmetic, scheduling and coverage

Groups reuse the existing typed reductions: Add and Increase use Sum with typed zero; Multiply uses Product with a dimensionless factor identity of one. Add retains the Stat's exact numeric type/unit. Increase uses percentage points. A group read exposes an ordinary typed value; existing rule nodes perform any explicit postprocessing or quantization. No Lua values, source tables, callback evaluator, arbitrary expression language, or implicit source-bucket arithmetic is introduced.

The framework supplies fixed named groups, not a dynamically created rounded group per item or provider. If a demonstrated game law needs such a boundary, it must be modeled explicitly rather than simulated by changing discovery order. This checkpoint does not settle the separately pending numerical policy for source-local factor multiplication or rounding.

The existing staged DAG remains authoritative. A query read accesses its underlying Stat/contribution channel for frozen-stage and readiness proofs. Those proofs remain conservative over the whole channel; naming separate groups does not authorize preparation and execution writers to share a channel with conflicting readiness roles. It cannot bypass a producer/read cycle, cross a frozen channel too early, acquire preparation authority, or read a final support-output channel as a contribution. Multi-stage computations continue to use ordinary programs and typed intermediate Stats, with explicit stage/readiness metadata. Conditions remain conditions on potential effects; a currently false guard is not permission to omit a member from the declaration. If future admitted cases require groups with different preparation dependencies, extend and validate those proofs explicitly before admitting them.

Data validates the registry's exact references, origin/owner/context agreement, slot maps, group identities, units, contribution kinds, read scopes, closures, and bounded sizes. The same reusable validator runs when Engine compiles raw rule inputs. Engine performs concrete occurrence binding and contributor coverage during candidate-plan compilation, then stores immutable ordered effect-index vectors for evaluation. Parallel workers evaluate ordinary native typed operations over their own candidate state; PoB is not involved.

## Current format and validation

Rule packages use schema3. There is no old/new query DTO parser. OperationsV21
introduces numeric queries; V22 adds typed Boolean contributions and unordered
Any, using effect-plan domainv19. V23 adds Actor/reward membership and uses
effect-plan domainv20. Explicit older operation subsets still reject
unsupported operations, but do not preserve historical package/hash formats.
Maintained acquisition/test artifacts were rebuilt with rule storage/compiler
hash domainsv3. The [Boolean contract](owned-boolean-contributions-proposal.md)
uses the same registry and separates exact origins from optional numeric ranks.

Storage budgets bound queries, groups, members, slots and validation work. Engine
bounds candidate expansion and comparisons. Declared ranks are part of package
and plan identities; changing a numeric rank is a semantic change. Tests cover
numeric order/ties, typed Boolean identities, explicit membership including
unread and inactive effects, Partial coverage, nonempty support suffixes, frozen
stages, scratch recovery, rebasing and Rayon determinism. These establish the
framework contract, not a game-specific aggregation law or item obtainability.

The canonical [Life packet](../data/owned/poe2/3887ae68/life-contribution-queries/README.md)
now uses V23 for eight known writers in seven groups. Six groups deliberately
give every member the same semantic position. Tie rejection proves that any
successfully bound recipient has at most one potential effect in such a group;
this gives a bounded empty/singleton reduction without an associativity claim.
Equipment retains its separate unresolved multiple-amount group. Neither this
partition nor Complete bounded membership closes global/owner coverage or
supplies the final Life formula.

The attribute consumer checkpoint binds real Original05 class/choice occurrences
to the same six receiver bodies in a finite fixture, using authenticated source
record order. The guarded empty-MORE packet now supplies six explicit factors
for a proved empty Multiply domain. The increased-attribute packet completes
six INC groups with 34 memberships across all nine current donor owners. Their
integral literals have maximum sums 29/24/22, so every permutation is exact;
explicit ranks do not claim source table iteration semantics. Publication scans
owner programs and rejects matching application programs. BASE and global
inventories remain Partial. Complete current INC membership alone does not
certify a complete original build.
