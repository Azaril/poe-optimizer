# Native item assembly

The target is a complete native item-preparation pipeline driven by injected game data.
It produces immutable modifier structures that the native build compiler can reuse
across parallel candidate evaluations. PoB remains an optional, pinned differential
reference. Assembly has no Lua runtime, subprocess, network or global mutable state.
See the [implementation record](implementation.md) for the current capability boundary.

## Ownership and data

`poe-optimizer-data` owns immutable source definitions, `poe-optimizer-engine` owns
portable value/list mechanisms, and `poe-optimizer-import` owns ordered item loading and
assembly. Engine code must not depend on import-owned `ItemState`.

An additive `ItemAssemblyData` section and `ItemAssemblyCatalog` describe closed,
source-authenticated policy groups: row collection, range rewriting, local queries,
rune effects, grants, named compatibility, attribute requirements and slot selection.
Game names, patterns, labels, modifier selectors, masks and numeric defaults are data.
Source field structure, arithmetic order, copy semantics and native operation kinds
are code. The catalog is not a general instruction language or a build whitelist.
Its current section schema 6 declares `PolicyOnly`; loading and binding it supplies
validated definitions, never an admitted executable item. Complete-method provenance
also does not imply lowering of every specialized slot branch.

The separate [equipment integration design](native-equipment-integration.md) specifies
whole-item slot validity, ordered item-set activation and actor-effective participation.
Its reusable validity program borrows prepared item owners and explicit tree/flag context;
a validity result alone does not activate equipment or complete a build.

Reuse the existing complete base records, rune definitions, scalability policy,
precision map and keyword definitions. Verify shared source facts during extraction.
In particular, a live keyword flag and a mask captured when a helper was constructed
are separate dependencies; changing one must not silently regenerate the other.
Keep requirement operand order and the order of independent compatibility rules.

Definitions validate required roles, references, provenance and resource bounds.
Pattern and replacement text can include NUL within their bounded UTF-8 data domain;
matching and output operate on bytes. Unused malformed patterns remain lazy source
errors. Catalog availability describes definitions, not completed item assembly or
numerical support for their effects.

## Advanced-copy unique line ordering

Unique advanced copies derive line order from the injected modifier family. Exact lowercased,
newline-flattened keys take precedence over normalized keys. Numeric and range substitutions,
modifier-family/field names, rarity roles and group operands belong to the item-loading policy.
Duplicate lookup keys use the source minimum. A successful finite reduction may be independent
of source table traversal; malformed inputs, ambiguous signed-zero minima and failure prefixes
need their own proof or an explicit unsupported boundary.

Assign or remove each explicit line's order even when the item has only one line. Sorting is
a separate advanced-copy operation: crafted/custom, fractured and ordinary groups follow the
injected group comparison, eligible groups compare order with missing values as infinity, and
remaining ties use original row position. Move the actual line records with their metadata;
never reconstruct their meaning from sorted text. Versioned/grouped unique variants bypass the
lookup as in the source. Earlier rare-affix matching and later magnitude transformations remain
separate operations and cannot be bypassed by supporting this one.

A successful lookup cache belongs to its immutable injected catalog and preparation lifetime.
Do not reuse it after a data-owner change or invent the source's partially constructed cache
on an ambiguous error. The initial native implementation caches within one item machine across
reparses; sharing prepared lookup storage across different item machines remains a performance
follow-up, with explicit owner and resource accounting. That limitation is not shared catalog
compilation or candidate throughput evidence.

## Rune reconstruction order

Rune inference consumes injected definitions and the original numeric/text policy. Preserve
PoB's first minimum-count combination in its candidate order; an alternative equally small
combination does not by itself make the result indeterminate. A finite group with strictly
descending vectors has one order under the original exact lexicographic comparator. Missing
components use the injected default, while search tolerance is a separate predicate. Names
and storage iteration are not evidence of source order.

The native proof is bounded and only needed when search finds alternative minima. Comparator
ties remain unresolved unless the existing unique-result path proves the required consumer
behavior. Grouping must also be order independent: check nonnegative integral contributions
before adding them, because rounding an out-of-range sum back to 2^53 does not prove exactness.
Keep all independent range, type, effect and source-error boundaries.

Explicit saved rune names remain authored input even when reconstruction chooses different
internal counts. Those counts constrain later bonded inference; only the header-free repair
path appends inferred names and rebuilds rune lines. Validation must cover both paths and the
resulting requirements and owned assembly. The [implementation record](implementation.md)
tracks the current evidence; these rules do not establish complete build parity.

## Mutable preparation, immutable results

Preparation needs object identity. A duplicate variant can append the same modifier
several times; subsequent source writes must reach every alias. A source copy later
creates independent objects, including independent copies of previously shared children.
Cloning every append, copy-on-write mutation, and graph-preserving deep copies each
change this behavior.

Use a bounded item-local arena with stable handles for tables, modifiers and rows.
Ordered lists retain handles until the original copy operation is reached. Source copy
operations recursively copy values without an identity memo; table keys and external
class identities require their own explicit representation. Charge nodes, references,
bytes, copies and work before allocation. Unsupported cycles or values remain explicit.
Freeze completed reachable state into immutable storage for worker sharing. Mutable
scratch is local to a preparation task and needs no shared lock.

The current `ItemState` transport combines parsed header values with bounded assembly
projections. Its socket groups remain the values parsed from the header; original assembly
rewrites socket groups in the authoritative owned arena. Compare the header groups with the
pre-assembly source witness and owned groups with the post-assembly source graph. Never use
that mixed diagnostic transport as a complete post-assembly object or an equipment admission
token. Keep both witnesses in parity receipts. Separating these stages more clearly in the
transport API belongs in the execution-model investigation, without weakening final output
parity or changing the authoritative arena boundary.

Armour state survives item re-entry. Preserve its complete owned graph, including nested
values, indexed entries and aliases introduced by late overrides. The numeric `armour_data`
map is a diagnostic projection; `armour_data_complete` distinguishes a complete numeric map
from a subset. A subset cannot seed a new assembly graph or authorize registration. Actual
parser header writes, including nil deletions, form a separate ordered-load patch applied to
the retained graph. Retain pending writes across no-base parses that never reach assembly. Consume them after
successful assembly or a successful no-base ParseRaw continuation so final assembly cannot
replay old headers over newer override values. A no-base continuation remains incomplete and
cannot register an item. Flask and charm outputs reset at assembly entry.

Input adapter limits remain distinct from runtime values: current finite UTF-8 metadata
admission is not permission to discard non-finite arithmetic, opaque functions, sparse
positions or unexpected intermediate types. Preserve representable raw values and exact
number bits; otherwise stop at the reached representation boundary.

## Progress and provider boundary

Assembly progress carries the reachable item state, ordered dependency events and one
of Complete, Pending, SourceError or ResourceError. State includes base/single/slot/buff
modifier lists, row-owned Bonded lists, range-row references, grants, requirements,
sockets, restrictions and cache fields. Absent, empty and unchanged are different states.
ModStore named fields are retained until the source converts a list to a plain array.

Earlier mutations survive a later failure. A slot result becomes an assigned item field
only after that call returns, while item mutations made inside the failed slot remain.
Retain sticky fields and opposite slot lists across re-entry exactly as the source does;
clear the active Bonded cache only after all slots succeed. Returning progress does not
promise arbitrary instruction-level resumption: retry must explicitly re-enter the
source operation or start from a retained input snapshot.

Parser and formatter events carry assembly invocation, phase, row/category identity,
optional authored index, rune provenance, slot and ordinal. Record calls before invoking
the provider, including failures and nested formatter precision parses. Preserve actual
argument count and nil positions. Do not invent authored line indices for generated rows.
Explicit caller providers remain supported; legacy atomic complete results retain their
restricted representability. No hidden fallback or restart with a different backend.

## Source-ordered operations

Collection handles disabled/extra/Bonded rows, duplicate variants, source assignment and
range callbacks in the original order. A range parse occurs before the zero-line check;
an empty replacement is different from leaving the old payload unchanged. Reuse existing
variant counting, with an explicit boundary if injected alternate-count policy exceeds
its represented state capacity.

Local queries consume matching modifiers in order. Earlier removals survive an error
on a later value; the failing value is not removed. Lua truthiness, numeric coercion,
first-tag selection, AND64 and MatchKeywordFlags govern selection. Nil-config queries
must filter before requesting EvalMod, so an unrelated tagged modifier does not block
assembly. Reached unsupported evaluation remains a specific dependency.

Slot selection uses the source's base fields and separate primary-name/dispatch rules.
Rune caster tags are not interchangeable predicates. Substitute tag strings with the
shared LuaJIT-compatible replacement primitive, including replacement percent semantics.
Preserve source naming quirks, every reached slot and each copy/filter boundary. Rune
scaling, requirements, sockets, crafted quality, Spirit and charm limits complete in
source order or report their exact frontier. Assembled grants are metadata until a
separate actor evaluator supports their effects.

Armour, flask and charm local algorithms consume explicit typed policies for ordered queries,
base/output fields, constants, operand roles and late overrides. Armour includes quality,
block, movement penalties and per-level defences. Flask recovery and charm duration remain
separate formulas; effect-not-removed flags can consume the base modifier list while the slot
copy retains its earlier contents. Fractional charge capacity and floored charge use are distinct.
Preserve query removals, output writes and conversion timing when later arithmetic fails.
Weapon locals use the authenticated captured damage order, explicit channel classifications,
queries, base/output fields, quality divisors, rounding and residual tag predicates. Preserve
publication of each slot's output before its calculations, conditional reload fields, positive
bounds, late arbitrary overrides, hand-specific tagging, and the final DPS sum after overrides.
Both slot outputs remain whole owned graphs. The native producer dispatches generic channel
kinds; injected names do not select hardcoded skill or item behavior.

Jewel locals preserve Grand Spectrum sharing, ordered list values and overrides, repeated
FromNothing queries, and cluster corrections and value-valued validity. Versioned radius
resolution and ParseRaw's header/deferred/override order use injected loading policy and an
explicit item-call context. Actual callback values still require an executable captured-state
owner. The [jewel integration design](native-jewel-integration.md) separates this local
production from selected-tree geometry and actor application. These domain algorithms add no
general instruction language and remain inputs to the execution-model review.

## Completion and proof

Complete means every reached operation, every slot call and final cache mutation has
finished. A collected modifier prefix is not a completed item. Selecting a scope from
source predicates is valid; selecting production support from current item/build IDs
is not. Any representative fixture set is a validation set only.

Differential tests call unchanged complete original methods with an uninstrumented
control. Compare the whole reachable graph, exact numbers/bytes, aliases, source copy
boundaries, row references, requirements, lifecycle state, callback order and failure
prefixes. Include changed injected names/policies, empty and missing values, all common
branches and adversarial resource limits. Warm execution claims require completed live
traces of the actual original methods; repeated calls alone are insufficient evidence.

General actor/action modeling and conditional-parser operations remain separate design
decisions. This item-preparation contract does not approve them or claim full native
build parity. The delivery sequence and measured breadth belong in the
[living implementation record](implementation.md).

## R2y implementation checkpoint

The producer and ordered inventory prefix use the existing injected catalog. Common assembly
and accessory slots are implemented; specialized local data and reached unsupported callback/tag
evaluation remain explicit. Owned assembly graphs, opaque item/catalog/attempt bindings,
preparation report schema 4 and CLI exposure are implemented. All supplied native builds remain
**0/5** complete.

`prepare_authored_items` retains authored occurrences and the source-order registration prefix.
Duplicate numeric IDs update only the lookup winner while retaining both instances and insertion
order. The stage stops before unimplemented item-set/slot/trade instructions or repeated root
sections. Equipment activation, actor effects and completed root loading remain open. Diagnostic
loader completion cannot authorize production registration. All five original public CLI runs
retain their 116 records but stop before the first registration: two at armour, two at
jewel/radius and one at charm dependencies. Accessory component matches below therefore do
not represent completed original inventory prefixes.

Source validation retains all 116 original records: 32 eligible accessories match using an
explicit original-parser dependency and 31 also match through the built-in native parser. The
remaining native case reaches unsupported parser callback 168. The other 84 records are excluded
by the local-family predicate and retained in the denominator; this lane does not execute them. Comparisons cover the declared assembly-field/row-field graph,
including aliases within it; they do not close the whole-object contract above. Five separate
control imports preserve selected views, output scalar bits, inventory order and 114 available
item graphs; two callback-bearing graphs remain unavailable. Directed coverage adds 30 variant
vectors, 15 range/reparse/reassembly steps and five malformed-requirement failure prefixes.

Binding/isolation, graph bounds and retained failure state, all-five inventory denominators,
native-only CLI and five portable WASM libraries pass. The source target's eight tests pass in
41.36 seconds on this Windows release run, including ten reference hosts. This is validation
cost, not hot candidate throughput. Final receipts are in
`runs/r2y-item-assembly-01/source-parity-04/`; the living implementation record lists other checks.

The metadata-tree adapter cannot recover original aliases or executable callbacks. Reference
parser conversion refuses these rather than stripping them. The full dependency event contract
above remains a completion gate: current sequence metadata is local to an assembly attempt after
its retained loader prefix, not a globally monotonic history or proof of actual argument arity.
Diagnostic projection refusal retains the owned graph without final exposure and stops the
machine before stale projected state can be reused; a fresh coherent load is required.

Count the producer, graph/adapters and differential tooling in the
[execution-model investigation](rule-execution-model-investigation.md). It selects no winning
architecture. Specialized locals, item-set activation, equipment/actor integration, lossless
acquisition and dependency events are the next integration gates; A3 remains the discussion
before a significant model migration.

## R2z local-item checkpoint

The owned producer now includes armour, flask and charm local data through injected section
schema 2. Preparation report schema 5 and item inventory report schema 2 expose numeric
projection completeness. Re-entry retains nested armour values and aliases, including header
writes across an intermediate no-base parse; late overrides cannot be overwritten by replayed
headers at final assembly. Weapon and jewel branches remain explicit dependencies.

The actual-item comparison retains all 116 original records. Of 91 eligible items, 87 match
using the original parser and 85 match through the built-in native parser:

| Family | Original records | Original parser + native assembly matches | Built-in native matches |
| --- | ---: | ---: | ---: |
| Accessories | 32 | 32 | 31 |
| Armour | 32 | 29 | 28 |
| Flasks | 12 | 11 | 11 |
| Charms | 15 | 15 | 15 |
| Weapons, excluded in this lane | 7 | — | — |
| Jewels, excluded in this lane | 18 | — | — |

Three eligible armour items stop at ambiguous rune reconstruction and one flask stops at
advanced-copy affix ordering in both lanes. The native parser additionally stops at callbacks
168 and 569 on two build-03 items. Exclusions and unsupported records remain in the denominator;
matching items are component evidence, not completed original inventories or full builds.
Five fresh observed/control import pairs preserve selected views, scalar output bits, inventory
order and available declared item graphs. No callback-bearing graph is silently simplified.

Directed validation adds 30 fresh local-item cases, 35 cross-family reparse steps and five
no-base re-entry histories. Sixty explicitly derived pre-call modifier inputs cover 40 complete
results, 15 source-error prefixes and five reached nonfinite frontiers; those last five do not
claim final graph parity. The previous variant/range/reparse/failure cases remain. These inputs
are labelled separately from actual originals and parser-produced modifiers.

The eight-test source target passes in 43.86 seconds on this Windows release run, including ten
reference hosts. This is validation cost, not candidate throughput. Source receipts are in
`runs/r2z-item-local-01/source-parity-01/`; the living implementation record tracks the public
CLI and remaining release checks. Full native original build evaluation remains **0/5**.

The public native-only CLI retains all 116 items and registers source-order prefixes of
2, 3, 0, 0 and 15 items across originals 01–05 (20 total, previously zero). The next stops
are rune reconstruction, weapon locals and jewel/radius handling. These registrations do
not activate equipment or imply completed item-set/root loading; every public report remains
Incomplete with calculation not run. Reports are in `runs/r2z-item-local-01/native-originals-01/`.

## R2aa weapon-local checkpoint

Schema 31 / item-assembly schema 3 adds the injected weapon policy. All 28 other data
sections and the previous assembly policy are unchanged. The original captured damage list
and live flag bindings are authenticated against source and the injected parser definitions;
an unrelated global list is not a substitute for the actual closure capture.

The comparison retains every saved item from all five originals:

| Family | Saved | Original parser + native assembly | Built-in native parser + assembly |
| --- | ---: | ---: | ---: |
| Accessories | 32 | 32 | 31 |
| Armour | 32 | 29 | 28 |
| Flasks | 12 | 11 | 11 |
| Charms | 15 | 15 | 15 |
| Weapons | 7 | 6 | 6 |
| Jewels | 18 | Not executed in this family lane | Not executed in this family lane |
| Implemented families | 98 | 93 | 91 |

The additional weapon stop is build-03 Item 17, whose rune annotation has ambiguous minimum
count vectors under unavailable source traversal order. It joins the three existing armour
rune stops. Advanced-copy flask affix ordering and native parser callbacks 168/569 retain their
previous stops. The 18 excluded jewels remain in the 116-item denominator; they are not
counted as executed assembly failures. Production support uses injected data and source
predicates, never original build/item identifiers.

Whole `weaponData` joins the predeclared comparison fields. Five observed/control import
pairs agree. Across those hosts, new directed coverage includes 20 fresh parser-driven weapons,
30 cross-family reparse steps, and 60 separately labelled derived modifier inputs: 35 complete
results, 20 source-error prefixes, and five explicit nonfinite reload frontiers. One derived
case per host includes 122 residual modifier records across exact flag, keyword and tag
combinations. Existing accessory/local-family and re-entry cases remain. Source failures compare
reachable owned prefix state; unreturned slot-local frames are not observed. Nonfinite cases
make no final-graph parity claim.

The eight source/observer/graph tests pass in 45.02 seconds on one Windows release run, excluding
37.66 seconds of compilation. This includes ten reference hosts and is validation cost, not
candidate throughput. Actual dependency argument-count/order parity, arbitrary callbacks,
metatables and the omitted source fields remain outside this finite declared graph contract.
Full native original evaluations remain **0/5**. Public inventory counts and final validation
are maintained in the [implementation record](implementation.md).

Receipts are under `runs/r2aa-weapon-local-01/`. The source output override was relative and
resolved against two process working directories; after successful completion the artifacts
were moved into `source-parity-01` and their original locations recorded in
`source-artifact-relocation.json`. The pinned submodule is unchanged.


## R2ab jewel and radius checkpoint

Package schema 32 adds item-assembly policy schema 4 and item-loading policy schema 5.
The native producer now covers all six local families through injected policies, including
jewel lists, overrides and cluster operations. Radius loading retains original version
selection, scaling and header/finalization order. Its source constants come from the original
Misc import; extraction records that dependency without running the radius setter.

All 116 saved items are represented in the component comparison, with no family exclusions:

| Family | Saved | Original parser + native assembly | Built-in native parser + assembly |
| --- | ---: | ---: | ---: |
| Accessories | 32 | 32 | 31 |
| Armour | 32 | 29 | 28 |
| Flasks | 12 | 11 | 11 |
| Charms | 15 | 15 | 15 |
| Weapons | 7 | 6 | 6 |
| Jewels | 18 | 16 | 16 |
| Total | 116 | 109 | 107 |

The two jewel stops are build-01 Item 14 and build-03 Item 1. The original-parser lane
refuses callback-bearing output outside its finite metadata contract; the built-in lane
reports a pending jewel capture factory. Four rune-reconstruction stops and one advanced-copy
affix-ordering stop remain in both lanes. Native parser callbacks 168 and 569 account for
the two additional built-in stops. All 21 build-04 items and all 28 build-05 items match in
both lanes; these are local-item results, not complete inventory lifecycle or build results.

Five observed/control import pairs agree within the declared finite graph contract.
The comparison now includes whole `jewelData`, represented `clusterJewel` and radius fields.
Existing omissions and dependency-order/argument-arity limits remain explicit. Directed jewel
coverage adds 20 fresh parser-driven items, 25 cross-family reparse steps and 80 separately
labelled finite pre-call modifier cases: 60 complete results and 20 matching source-error
prefixes. Cluster metadata is caller-supplied in these cases; the original pin does not
initialize it. Five actual callback-ingress probes retain the original source function and
refuse its opaque descriptor natively, with no native callback execution or graph-parity claim.

Sixty further radius-history steps use actual original setter calls and matching explicit
native contexts. They cover static, unmatched and absent labels, deferred Variable selection,
nil removal, repeated headers, time-lost overrides and cross-family persistence. Five no-base
histories retain the old jewel graph through ParseRaw's post-assembly tail and remain
unregistered and incomplete; five missing-context cases remain explicit dependencies.
These post-import histories do not establish initial Build lifecycle parity; that observation
is recorded separately in the [jewel integration record](native-jewel-integration.md).

Counts and source receipts are in `runs/r2ab-jewel-local-01/corpus-counts.json` and
`runs/r2ab-jewel-local-01/source-parity-02/`. They measure validation breadth, not candidate
throughput. Full native original build evaluation remains **0/5**. Selected-tree application,
actor effects and the remaining input dependencies stay open; no replacement execution model
has been selected.
