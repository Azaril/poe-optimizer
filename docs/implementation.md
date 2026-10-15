# Implementation plan and resume point

Updated: 2026-10-15 (EDT).

This is the active delivery plan. The [design](domain-architecture.md) defines
the end state; the [execution overview](data-and-evaluation-overview.md) explains
what actually runs. Older checkpoint reports are preserved in
[implementation history](implementation-history.md), not current instructions.

## Current state

**Five supplied builds: 0/5 complete native evaluations.** All five import and
run in pinned PoB. Owned native components work, but unresolved inputs and
mechanics still prevent complete native requests. Component parity, catalog size
and deleted code are not substitutes for this gate.

**Delivery priority, confirmed 2026-10-07: finish the closest complete build
first.** Original05 (Skeletal Sniper / Basic Attack, skill set 4) is the current
target: it has four selected input obligations and the most developed relevant
native components. This is a prioritization judgment, not a completion estimate;
input counts do not measure remaining numerical work. At each checkpoint choose
the shortest evidenced path to one complete unchanged build, accounting for
request admission, selected mechanics, final metrics and reference validation.
Keep all five originals as regressions, but do not develop them in lockstep or
require a second complete build before starting owned optimizer integration.

The first complete build unlocks D4: bind a configurable objective to its exact
owned metrics, evaluate a legal locked candidate change, compare fresh/reused/
Rayon results and verify export/reimport. Expand candidate coverage from that
working baseline while completing the other builds. A baseline calculation is
not proof that its mutations work, and this gate does not reduce the eventual
six-dimension optimizer or independent breadth requirements. Original05 is a
development priority only; production admission remains caller/data driven.

The owner confirmed that **SQLite, DuckDB and an ORM are outside the plan**.
Use generated owned artifacts and immutable loaded Rust indexes. New features
belong in one native path:

| Subsystem | What belongs here |
| --- | --- |
| Owned Core | Source-independent build, occurrence, scenario and request contracts |
| Owned Data | Injected definitions, tables, typed rules, identities and compilation |
| Owned Engine | General native preparation, effects and metrics; reusable worker scratch |
| Owned Import | External build decoding, source interpretation and checked normalization |
| Offline acquisition | Convert pinned source facts to reviewed owned data artifacts |
| Optional PoB | `mlua`/LuaJIT reference execution and data acquisition |
| Legacy Import/Data/Engine | Only named remaining acquisition, inspection or numerical-reference consumers; retirement work |

`evaluate-owned` is the native CLI entry point. `evaluate` and `metrics` are
optional PoB reference commands behind `--features pob`. Native evaluation has
no PoB subprocess or Lua state. Some legacy Rust modules still compile through
Import's dependencies; full legacy distribution isolation is unfinished.

The obsolete native CLI, NativeBackend crate, class/UI experiments and orphaned
profile-template coordinator are removed. Keep useful independent source and
numerical tests. Delete unused paths together with their exclusive scaffolding;
an exported API or its self-tests alone do not justify retention. The
[retirement inventory](legacy-retirement.md) names the remaining consumers.

**Compatibility direction, 2026-10-06:** the owner does not require compatibility
with earlier development formats or behavior. Prefer updating the current
contract and rebuilding/reimporting data to carrying old and new branches.
Preserve useful source evidence, not obsolete runtime paths. Any semantic cutover
must invalidate affected result identities and caches; verify the regenerated five-build
corpus. This supersedes preservation requirements in older proposals where they
exist only for backward compatibility.

## Route to the first complete build

The current Original05 package preserves **22 queries with 20 metric names**.
The retained reference contract expects **19 finite values and three unavailable
results**, not 22 invented numbers. The query list is
`queries-original-05.json` in the current package; the availability manifest is
`tests/fixtures/breadth-expectations/originals-v1.json`, case Original05.
These reference observations are not evidence of native metric completion.

| Saved query references | Native work to reuse | Remaining final producer |
| --- | --- | --- |
| 00: Player Life | Intrinsic/Strength/item/reward contributions, checked Life queries, bounded exact BASE accumulation and guarded final consumer with conversion/override/CI precedence | Complete supplier/magnitude admission, prove conversion accumulation domains, finalize plan and evaluation binding |
| 01: Player Mana | Intrinsic/Intelligence contributions, direct checked adjustment queries, override selection and published guarded consumer with checked numeric domain | Transformation supplier admission, whole-plan completion and evaluation binding |
| 02, 18, 19: Player ES, Armour, Evasion | Selected item magnitude/local-defence intermediates and global/passive contributions | Final item properties, complete receiving, transformations and final Actor totals |
| 03–06: four capped Player resistances | Item/passive/reward inputs and configured penalty | Complete contributor inventories, aggregation and cap/override calculations |
| 07–12: total EHP and five maximum hits | Encounter and incoming-damage components | Complete resource/defence dependencies, mitigation and final survivability calculations |
| 14: exact Sniper Basic Attack hit DPS | Prepared skill, intrinsic damage/rate, accuracy, inherited/Offering/Gigantic/quality factors and exact-Action Command | Pre-conversion damage assembly, remaining modifier domains, critical/speed/enemy mitigation and final DPS binding |
| 13, 15, 16: Player hit DPS/average hit and Sniper average hit | Exact typed query identities | Reviewed unavailable results matching the reference; no fabricated numerical defaults |
| 17: Player Spirit | Actual Amulet/reward inputs, explicit Amulet-copy contribution and guarded final Player consumer | Supplier, suppression and numeric-order admission plus complete-plan binding; unreserved Spirit and legality are separate |
| 20, 21: Player movement/action speed | Movement contribution channel and typed metric declarations | Complete contributors, interactions and final calculations; observed action speed 100 is not a native constant |

Four selected import obligations remain: **01f2** configuration, **0505** preset
usage, **0207** external assumptions and **0208** scenario usage. Earlier evidence
uses `0503` for usage; the current allocator identity is `0505`. No evaluation
bundle or complete final metric bindings are published. Selected passive rule
owners are 43 Complete / 12 Partial; item, class/shared Actor, supplied Actor and
global query/application coverage retain their own gaps.

**Minion Life is not a saved metric or an established numerical dependency of
these 22 queries.** Its current work advances selected-Actor coverage under the
existing complete-plan contract. The bounded transformation witness is complete;
do not expand positive transfers ahead of direct Player resource, Sniper damage
or input-admission work. Checked configuration accounting and numeric domains
were approved on October 14. Numeric checks now run in the existing engine;
configuration has common checked leaf accounting and multiple source-backed
input publications. Complete inventory proof remains pending. Other selected
skills still require honest plan coverage; query selection cannot silently prune
them. Once the complete unchanged request, evaluation bundle and reference
availability/value checks pass, begin D4 immediately.

## Current checkpoint: reporting inclusion intent

The [reporting-inclusion packet](../data/owned/poe2/3887ae68/reporting-inclusion/README.md)
preserves saved group inclusion as a typed Boolean on each eligible exact Skill
occurrence. It reuses preset ownership, source applicability and scenario
replacement. UsagePolicy `338e` and required parameter `338f` add an explicitly
Complete-empty ordinary-gameplay owner and checked support-source domain. There
is no new runtime API, mechanical program, aggregate metric or evaluation bundle.
True/false changes request identity without changing ordinary damage, activation,
population or query selection. Literal `nil` maps to false at Import; absent or
unreviewed values remain Pending.

The actual CLI publication and byte-identical rebuild pass for all five original
imports and every archived preset. The only new preferences number **8/0/0/1/40**,
each joined to its exact original group, source, eligible rule and existing
target/applicability. The inverse preserves all prior values, local IDs, source
dispositions, 110 query bytes and selected issue counts **106/117/109/123/4**.
No issue closes; complete native originals remain **0/5**. Reporting aggregation,
global switches, Action choices and full usage accounting remain separate work.

Published package: **`runs/owned-reporting-inclusion-publication-01/package`**.

- Input: `e1c2abd299aead48ff1da38aa517c4fcbcaacd2671f754deb921364c0cad4b39`.
- Registry: `0936f8f33162aafecf52011a70d971fbf9c1e325d50fe8aba0fd2e63b4f5babd` (last `338f`).
- Definitions: `86764ee48af08a18ca08ad7eac3e2443f38b66ff948021a3b6ef673ff1d24211`.
- Rules: `7d7ecf94119f30cb6209f0c3a91b3e167f5088efb96285944d88350b4b3049ce`.
- Compiled: `6401dc0a0122ae43414c52051c70acc83361c4edaab7b9c2a775f93523fa21a1`.

The package contains 18 files / **64,432,702 bytes**, 187 provenance entries and
110 queries. Its source archives remain offline evidence. No mechanical programs
or receiver declarations were added; the program cap remains 8187/8192.

Focused Engine tests pass **15/15**, Import tests **349/349**, ordinary CLI checks
**3/3**, and explicit all-five publication **1/1** (45.32s). Native checks include
scratch reuse/Rayon, exact generated targets, scenario replacement and missing
support-domain refusal. Strict targeted Clippy passes for Engine, Import and CLI;
independent review is clean. Final ordinary CLI checks pass again on the exact
publication helper. Evidence:
`runs/owned-reporting-inclusion-{native-04,import-02,cli-02,publication-01}.log`
and the publication's `validation.json`.

**Resume:** implement the accepted exact Skill support-receiving endpoint in
Core/Data/Engine, then the checked resource-obligation contract, payer aggregation
and reservation migration described below. The current live reservation consumer
remains Action-owned; metadata publication does not make it query-independent.

## Previous checkpoint: Player Spirit pool

The [Spirit packet](../data/owned/poe2/3887ae68/player-spirit-pool/README.md)
adds two injected data programs: an Amulet-copy contribution on the existing
modifier and a final consumer on Player Stat `3166`. Five adjustment channels,
nine checked queries and one Player receiver use existing native contracts.
The consumer handles conversions, Extra/Total, Increase/More, the accepted
rounding domain and a present override, including zero. It adds no runtime API,
Lua behavior or native source dependency.

The joined component imports the actual Amulet modifier's 24 rolls and the three
selected reward occurrences. It binds saved `reference-17` / metric `313c` to
final Spirit **113** and matches the retained range controls **110/115**. The
separate zero Amulet-copy contribution remains present. Source initialization
zero is represented by the native sum's additive identity, without claiming
identical source provenance. Four retained copy probes belong to the original
case only; positive conversion/override controls are explicitly synthetic.
Fresh/reused/Rayon results agree. Missing contributors, incomplete inventories,
conflicting overrides and invalid freeze order remain blocking.

All nine supplier inventories and the relevant owner gaps remain Partial.
Equipment suppression, untranslated suppliers and numeric-order domains are
unproved. Reservation and the observed **-67** unreserved Spirit are separate.
The finite component graph does not make this metric admitted in a complete
original request. All five imports preserve their 110 queries, exact local IDs
and selected issue counts **106/117/109/123/4**; no issue or whole build closes.

The published package is **`runs/owned-player-spirit-pool-publication-02/package`**:

- Input: `dc3371b68bc3ce5f870b6b0c44efe3dad16a0f1c37d2cb39a63492050f9d0d56`.
- Definitions: `05c469e54550ae6853a9b69feaea98eacca196f966681c68a7a5077a5c61a56f`.
- Rules: `5e6cb4dc31c83ceec9450672d4898bf0d49cd7c267d8d93588b18dce950d2ce1`.
- Compiled: `3afba22c8b49ce8fbb6c126bde5419e566348b9c3412fabe12ac3d20c0232e2b`.
- Registry: `2b54cb7610709f9101b0b2560e9efa8a23c648c9f7c220b23c9a3fadd73e4ad1`, last issued `338d`.

All eighteen artifacts rebuild identically. All-five publication and two ordinary
source/authoring tests pass (34.84s); five focused joined tests pass (52.31s).
All 95 joined component tests pass (65.15s), followed by the eleven ordinary
joined tests and two ordinary Spirit source-proof tests. The separate explicit
replay exporter was not run. Strict Clippy passes for both affected targets
(4.73s); independent review is clean. Evidence uses
`runs/owned-player-spirit-pool-{publication-02,joined-01,joined-02,ordinary-01,clippy-01}.log`.

## Previous checkpoint: bounded Player Life BASE accumulation

The [Life BASE packet](../data/owned/poe2/3887ae68/life-base-domain/README.md)
bounds the existing shared-Player Strength-to-Life contribution using the
accepted numeric-domain operation. It adds no program, receiver, definition or
runtime API. Seven distinct BASE producer addresses span intrinsic, inherent,
equipment and reward groups. Exact producer and receiver censuses establish
that admitted Player-targeted terms are integral and bounded by `2^37`;
one global 65,536-effect ceiling bounds the
whole union by `2^53`. Every prefix, subgroup and regrouping is therefore exact
in binary64. This proves why native grouping can differ from PoB insertion order
within the admitted domain.

The proof checks all eight Class level schemas, the sole Strength-derived Life
writer and its receivers, the complete equipment writer, all ten potential Life
writers and the shared-Actor application. Inactive and non-Player writers remain
in the census. Player INC has only the fixed reward contribution; minion INC and
MORE stay on their exact supplied Actor. Active missing or unsupported amounts
remain unavailable, and inactive effects do not demand unused amounts.

Final Life's remaining gap is now `unproved-life-conversion-order-domain`.
PoB carries one accumulator across conversion-name buckets; independent bucket
sums are not equivalent for unrestricted floating values. All seven adjustment,
CI and override supplier queries remain Partial. Upstream source/magnitude,
Actor and global coverage and the four selected input obligations are unchanged.
A local two-owner inverse authenticates older component evidence; current
native fixtures execute the guarded programs without rebuilding an old release.

Its published package is **`runs/owned-life-base-domain-publication-01/package`**:

- Input: `121dc6bf9980fef3301d18e6f6cb4148884f50a792e38b7599f1db60318ab748`.
- Definitions: `cf65959955725dde2758ea79cb5542dca1fe9a9601a352344f8f3fad24ad837b` (unchanged).
- Rules: `5eeef2c9a96a7fbba84e04295080ac217c508b7775acfdb1568fdf5836f02813`.
- Compiled: `a035113017081daf0c7135a12c271bf22861ef8164490334b3a7e0d1e734e8c9`.
- Registry: `6e97db58bec5855fa1a5b8e0b5253fc3ab74c3192a48e29262cce59cd47b076a`, last issued `3388` (unchanged).

All six focused controls, three minion-inherent regression tests and 101 joined
native tests pass; the joined component
still derives Player Life 1320 from its actual producers. All-five publication
passes in 22.11s and rebuilds all eighteen artifacts identically. Only rules,
manifest and release change. All 110 saved queries, exact IDs and selected issue
counts **106/117/109/123/4** are preserved. Strict Clippy across six affected
targets passes (4.21s), as does the existing engine hard-effect-ceiling regression
(one test, 0.19s). The joined run takes 60.34s. Evidence uses
`runs/owned-life-base-domain-{authoring-04,final-ordinary-01,joined-02,publication-01,clippy-02,hard-limit-01}.log`.
Complete native builds remain **0/5**.

## Source admission checkpoints

### Selected quest/reward reconciliation

The [configuration source review](../data/owned/poe2/3887ae68/mana-supplier-sources/configuration-review.md)
now reconciles all **34 selected quest callbacks per independent observation**
against the retained 17-control, 39-value reward witness. Defaults produce nine
records and the saved configuration produces 22; they remain separate. Exact
control keys, typed Boolean/Option codecs, source identities, complete ordered
records and native delivery establish the correspondence. The current report
and sidecar bind ConfigSet source ordinal 423 to choice preset `01f1`, then to
17 exact native reward occurrences and their Player-targeted programs.

The selected parser-boundary count falls **34 → 0** in each observation. General
quest parsing remains unproved: PoB accepts arbitrary text, while this proof
refuses values outside the finite admitted token domain. Global reward and
choice inventories remain Pending. These leaves were already input-accounted,
so no selected import issue closes and complete builds remain **0/5**. The native
package and all five originals are unchanged; bounded archives serve offline
authentication, with no historical evaluator or reimport path.

All ten ordinary Rust tests pass (4.82s), including independent source/native
mutation refusals. Explicit offline retention passes (6.30s), strict Clippy
passes (0.67s), and independent review is clean. Evidence uses
`runs/owned-mana-configuration-rewards-{retain-03,test-03,clippy-02}.log`.
The nine bounded archives total 1,221,115 compressed bytes; their raw and
compressed hashes are independently checked. They do not enlarge the native
release package. Review receipt SHA-256:
`929e8805a7e1ec72ef9d022354309fde61b1e85c12032173fda6550e23ddcccf`.

### Selected radius-provider source proof

The [radius packet](../data/owned/poe2/3887ae68/mana-supplier-sources/radius-review.md)
now reconciles the selected empty passive-jewel constructor boundary. Source02
omitted invocation overrides, reuse/acceleration inputs and actual slot candidates;
a fixed four-load acquisition supplies those inputs. Two independent observed
JIT-off runs agree, and their captured scalar snapshots exactly match plain
JIT-off/on controls. Original functions remain unchanged.

Each trace has 21 fresh environments, 113 ordered slots with 112 actual selections
per environment, all nineteen empty socket selections, and 42 node-list passes
with 2,394 node visits. Exact source membership preserves 55 allocations and two
separate roots. Complete constructor bodies and the absent item/override inputs
exclude both radius insertion branches. Empty final lists alone are insufficient.
The proof refuses occupied/unknown sockets, injected providers, changed membership,
unrecognized overrides, acceleration and reused environments. It does not claim
to implement generic radius mechanics or complete native supplier coverage.

The source acquisition passes in 81.25s; offline retention passes in 15.49s;
seven ordinary mutation tests pass in 20.72s. Both strict Clippy targets and
independent review pass. The first retention attempt failed because the verifier
incorrectly required the same keystone argument on both node-list calls. It now
requires the actual first-pass `true` and second-pass omitted argument. Source
capture was unchanged and was not repeated. Four raw loads and four acquisition
scripts retain 57,973,796 raw bytes in 1,441,004 compressed bytes. Compact metadata
reconstructs the exact accepted report, avoiding a duplicate 57 MB archive.
Review SHA-256: `6cfff1148386ee1d5b98eaab1d334b5eb745194bd623c2e1a725d9335ce760a2`.
Native package, queries, four input obligations and complete-build count remain
unchanged.

Evidence: `runs/owned-mana-radius-source-{ordinary-01,acquire-01,clippy-01}.log`
and `runs/owned-mana-radius-{retain-01,retain-02,ordinary-01,offline-clippy-01}.log`.

### Selected passive constructor reconciliation

The [supplier packet](../data/owned/poe2/3887ae68/mana-supplier-sources/README.md)
now reconciles ordinary parsed-record transport using the existing source02
observations and radius proof. Ten authenticated source spans cover scratch-list
reset, complete node/list constructors and the original ModList/ModStore
transport. The keyed catalogue has 57 selected nodes and 55 distinct complete
bodies. Counted full-body correspondence accounts for 2,394 anonymous returns
across six mode/keystone-argument groups, including 504 empty returns. No node
identity is inferred from cross-capture iteration order.

Each of 42 node-list outputs retains the full 57-record union and duplicate
multiplicity. Exact per-node sequences and separate raw/within-name order
commitments prevent the multiset comparison from granting arithmetic reorder
authority. Recursive checks exclude all fifteen Mana adjustment names from this
finite parsed-record domain. Changed source bodies, transform gates, nested zero
suppliers, lost/duplicated bodies and altered generated roots fail directly.

This does not prove arbitrary parsing or that empty parsed output means no
mechanics. Of twelve empty source declarations, nine have text and three are
blank roots/socket; this is a different inventory from the twelve native Partial
owners and does not revoke separate intrinsic proofs. The two exact `ExtraSkill`
roots, generated/support descendant delivery, later transformations, numeric
ordering and all twelve Partial passive owners remain open. It changes no native
package, query, input obligation or complete-build count. The next bounded source
step was selected equipment recipe and constructed-list reconciliation, now
recorded below. Exact native source dispositions remain required; another zero
aggregate or completeness-label change is insufficient.

Explicit retention passes (16.20s). All seven new ordinary tests (17.21s) and
seven affected radius tests (20.87s) pass; strict Clippy passes for both targets
(0.78s), and independent review is clean. The first Clippy run exposed duplicate
test-helper module imports; the new proof now reuses the radius helper's existing
authenticated supplier module. No production API changed. Evidence uses
`runs/owned-mana-passive-{retain-01,ordinary-01,clippy-01,radius-ordinary-02,radius-clippy-02}.log`.
Retained review SHA-256:
`007e491a5188ecd36d9673839b3fd382e7bffa0c746ff1b5d7604548ad85d2f1`.

### Selected equipment constructor reconciliation

The same [supplier packet](../data/owned/poe2/3887ae68/mana-supplier-sources/README.md)
now authenticates eight Item loads, nine exact uses and eighteen active-list
recipes across the two bonded settings. It joins retained raw XML, ranges,
variants and complete parsed/base/buff records through exact owned bindings,
actual radius-capture slot selections and source02 item inventories. Complete
Item, ItemsTab and ItemTools bodies remain offline evidence; no new source VM
run or runtime adapter is introduced.

Each independent source02 report has 189 exact item/base/buff joins and 126
direct active-list joins across 21 environments. Both ring uses and Weapon 1
have slot-specific active lists absent from that capture. Their retained
original construction recipes are checked separately; absent `item.modList`
is never treated as an empty active list or a delivery observation. The actual
jewel-data key is omitted, not JSON null; zero socket count is separately
observed. The three source archives add 67,639 compressed bytes.

Recursive checks exclude the fifteen Mana adjustment names from this finite
construction domain. A generated-skill declaration remains an explicit
boundary. Item-to-store delivery/scaling, focus/Amulet-copy/replacement paths,
inherited stores, generic parsing, numerical order, generated/support delivery
and native/global coverage are unproved. No package, input issue or whole-build
gate changes.

Retention passes (22.34s), all seven ordinary tests pass (21.18s), and strict
Clippy passes (1.10s); independent review is clean. The first two retention failures identified incorrect
verifier assumptions about omitted jewel-data fields; the exact immutable
capture is now checked without null/absence equivalence. The first Clippy run
identified a helper type alias simplification. Failed logs remain; no source
capture was repeated. Evidence uses
`runs/owned-mana-equipment-{retain-01,retain-02,retain-03,ordinary-01,clippy-01,clippy-02}.log`.
Review SHA-256:
`240521343222f840c91a129126ff5b630fc04917da66cc5de30ab2a64a7d495b`.

The final Spirit component is now published above. Its source suppression,
supplier and numeric-order admission remain explicit gaps. Capacity is distinct
from reservation. The owner accepted
[Skill-owned resource obligations](owned-resource-obligations-proposal.md) on
October 15: explicit payer/resource identity, exact Action dependencies only
where mechanically required, and checked Skill support recipients. After the
completed equipment-source and reporting-intent checkpoints, implement the bounded
contract and migrate existing reservation data, removing the superseded live
Action-owned path. Prove invariance to unrelated metric-query changes alongside
participation, repeated sources, counts, payer identity, dependency cycles and
Action-sensitive controls. Existing numerical and complete-plan gaps remain
until independently closed. Sniper physical-base assembly remains behind its missing self-flat supplier
authority, and its unpublished draft IDs must be reconciled before adoption.

Native-only writer census
could make Life's adjustment groups Complete-empty, as Mana's already are, but
would not prove absence of untranslated suppliers or unlock this build. Do not
prioritize a coverage-label cleanup over actual source admission. Remaining
resource source domains include outgoing conversions, incoming donor-to-Life
conversion/gain, post-scaling additions, CI and overrides. Keep the separate
conversion-order proof explicit; zero observations cannot establish any of these
absence claims. Query-scoped coverage would be a separate design decision, not
an implicit way around the current complete-plan contract.

## Measured catalog capacity and next cleanup

The Life predecessor's read-only capacity audit found programs to be the only
nearly exhausted compiler collection, at 8190/8192 including one application.
The Mana cleanup reduced that to 8185/8192 and receivers from 60 to 55. The
Spirit component now uses **8187/8192**, leaving five program slots, with 56
receivers. Other measured Life-predecessor compiler counts are
6548 owners, 23517 effects, 4249 reads, 32814 nodes and 41600 edges, comfortably
within their separate caps.
The current reporting-intent release package is **64,432,702/67,108,864 bytes**,
leaving 2,676,162 bytes. These are separate limits. The common
content digest also enforces the 64 MiB envelope, so increasing only release
input limits would not address package growth.

The five Mana adjustment receivers (`3357`–`335b`) have now been removed after
checking all rule/application readers, saved metrics and retained query contracts.
Their existing guarded Complete-empty membership is unchanged; broader supplier
and whole-plan coverage remains open. Do not split catalogs by the selected build or remove
legitimate per-item programs to meet a budget.

Track compiler/schema validation work and release byte totals at subsequent
publications; private aggregate work counters need bounded diagnostics before
claiming exact headroom. If simplification is insufficient, make a measured
catalog-capacity adjustment with boundary tests. **Keep the independent
`PlanLimits.max_effects = 65536` unchanged:** the equipment and combined BASE
exact-sum proofs use
`65536 × 2^37 = 2^53`. Limits are acceptance/execution gates and are not hashed
into result identities, so a changed numerical domain requires an explicit
proof and artifact/cache invalidation rather than relying on a limit change to
invalidate results.

## Previous checkpoint: direct Mana queries

[Archived publication and validation](implementation-history.md#archived-2026-10-15-checkpoint-direct-mana-queries):
five forwarding programs and Player receivers removed; current fixtures use the
final direct-query consumer and the obsolete collector installer is deleted.
All 24 focused tests, all-five publication/rebuild and strict Clippy pass. Pushed
as `0ca256a4`. Coverage and typed channels remain unchanged.

## Previous checkpoint: guarded Player Life consumer

[Archived publication and validation](implementation-history.md#archived-2026-10-15-checkpoint-guarded-player-life-consumer):
actual joined producers calculate Player Life 1320; all 101 joined tests, two
source tests, all-five publication and strict Clippy pass. Supplier and numerical
domain gaps remain explicit. Pushed as `ba33003d`.

## CI receipt repair (2026-10-15)

Hosted run `38675971601` for older revision `c4a62464` exposed another stale
derived receipt in `owned_catalog_recipe`. The compiler-domain change updated
the active tree seed but missed `import/compiled/manifest.json`. The existing
`extend-owned-skill-catalog` CLI reproduces all nine artifacts from unchanged
inputs: seven are identical, while the manifest's compiled identity and its
transition hash change. The expected compiled identity is
`18ac3accd9995e8006a11707adbdd11827d0bb8b039b7814fb0232c515fa722f`.
No source mechanics, test expectations or allocation code change. Independent
inverse review confirms exactly those two digest changes and unchanged LF bytes.
All five Import `owned_catalog_recipe` tests pass (0.35s), using the unchanged
strict artifact comparisons. Hosted success is not yet confirmed.
The subsequent job-log audit explains four more failing CLI targets in that old
run: `owned_item_bases_cli`, `owned_attribute_cli` and `owned_extension_cli`
reject the former current-manifest receipt; `owned_catalog_cli` rejects the
former generated catalog manifest. Those are covered by `47fa0592` and
`7aebbde1` respectively. All 19 current and eight import/compiled artifact
receipts now match their exact hash and size. The later Ubuntu cli-0/cli-1 logs
show the same old current-manifest failure in `owned_weapon_profiles_cli`,
`owned_class_bases_cli`, `owned_intrinsic_cli` and `owned_passive_views_cli`, plus
the old `current/transition.json` byte mismatch in `owned_tree_cli`. These are
the receipt family regenerated by `47fa0592`; no additional code defect was
found. A fresh local `owned_tree_cli` run passes all three tests in 6.98s,
including exact twenty-artifact reproduction, all-five XML censuses and failure
controls (`runs/ci-owned-tree-receipts-01.log`).
The completed Windows cli-0 job reports the same old weapon-profile manifest
receipt failure in both feature profiles, with no new defect
(`runs/ci-job-116230401591.log`). The latest inspected push `6c45804e` has
pending run `38688262637`;
main concurrency preserves the running older revision and coalesces pending revisions. Recent
jobs have completed, but live job logs are unavailable, so individual long steps
have not been diagnosed as hung. Do not report the queued revision as green.
The subsequent Windows cli-2 job repeats only the already-repaired item-base
manifest receipt failure in both profiles (`runs/ci-job-116230401646.log`).
The older revision's Windows PoB job has completed successfully; this is not a
hosted result for the current main revision.
Evidence: `runs/ci-import-c4a62464-job116230401602.log` and
`runs/ci-catalog-republication-01-diff.json`, plus
`runs/ci-job-116230401634.log` and `runs/ci-job-116230401669.log`; validation:
`runs/ci-owned-catalog-recipe-receipts-01.log`.

## Previous checkpoint: configured Player Withered input

[Archived publication and validation](implementation-history.md#archived-2026-10-14-checkpoint-configured-player-withered-input):
one raw typed Player assumption, preserve-zero selection and checked receipt
for both saved lanes. Original05 configuration-origin links decreased 31 → 30;
all five selected issue counts and 110 queries remain unchanged. Nine source
tests, nineteen new controls, all existing scalar/distance publication matrices
and strict Clippy pass. Native applicability, scaling and stack limits remain
separate calculations.

## Previous checkpoint: empty intrinsic passive socket

[Archived proof and validation](implementation-history.md#archived-2026-10-14-checkpoint-empty-intrinsic-passive-socket):
Passive `1b48` and its seven intrinsic declaration inventories are complete;
Socket `320e`, topology and external jewel/radius obligations remain intact.
Selected partial passive owners decreased 13 → 12. The separate CI receipt
repair is pushed as `47fa0592`; all fourteen existing allocation-export tests
pass locally, with hosted confirmation pending.

## Previous checkpoint: configured distance inputs

[Archived publication and controls](implementation-history.md#archived-2026-10-14-checkpoint-configured-distance-inputs):
two typed Enemy assumptions preserve zero differently for skill range and
modifier candidates. All-five publication, sixteen controls and strict Clippy
pass. Native applicability and effects remain separate work.

The reports below are historical. Their former pending decisions, counts and
resume instructions are superseded by the current checkpoint and delivery gates.

## Previous checkpoint: bounded equipment Life reduction

[Historical report](implementation-history.md#archived-2026-10-15-previous-checkpoint-bounded-equipment-life-reduction). The active resume point is above.

### Configuration projection implementation

[Historical report](implementation-history.md#archived-2026-10-15-configuration-projection-implementation). The active resume point is above.

## Previous checkpoint: Player incoming-critical receivers

[Historical report](implementation-history.md#archived-2026-10-15-previous-checkpoint-player-incoming-critical-receivers). The active resume point is above.

## Previous checkpoint: archived generated usage responsibilities

[Historical report](implementation-history.md#archived-2026-10-15-previous-checkpoint-archived-generated-usage-responsibilities). The active resume point is above.

## Previous input checkpoint: configured enemy critical inputs

[Historical report](implementation-history.md#archived-2026-10-15-previous-input-checkpoint-configured-enemy-critical-inputs). The active resume point is above.

## Previous input checkpoint: configured enemy attack/cast interval

[Historical report](implementation-history.md#archived-2026-10-15-previous-input-checkpoint-configured-enemy-attackcast-interval). The active resume point is above.

## Previous numerical checkpoint: guarded Player Mana publication

[Historical report](implementation-history.md#archived-2026-10-15-previous-numerical-checkpoint-guarded-player-mana-publication). The active resume point is above.

## Accepted decisions and next implementation boundary

[Historical report](implementation-history.md#archived-2026-10-15-accepted-decisions-and-next-implementation-boundary). The active resume point is above.

## Implemented contract: checked configuration accounting

[Historical report](implementation-history.md#archived-2026-10-15-implemented-contract-checked-configuration-accounting). The active resume point is above.

## CI repair checkpoint (2026-10-09)

[Historical report](implementation-history.md#archived-2026-10-15-ci-repair-checkpoint-2026-10-09). The active resume point is above.

## Numeric-domain checkpoint

[Historical report](implementation-history.md#archived-2026-10-15-numeric-domain-checkpoint). The active resume point is above.

## Final Mana arithmetic and remaining supplier admission

[Historical report](implementation-history.md#archived-2026-10-15-final-mana-arithmetic-and-remaining-supplier-admission). The active resume point is above.

## Previous data checkpoint: actual Mana override selection

[Historical report](implementation-history.md#archived-2026-10-15-previous-data-checkpoint-actual-mana-override-selection). The active resume point is above.

## Preceding native checkpoint: coherent optional numeric selection

[Historical report](implementation-history.md#archived-2026-10-15-preceding-native-checkpoint-coherent-optional-numeric-selection). The active resume point is above.

## Preceding data checkpoint: Command joins the Action damage consumer

[Historical report](implementation-history.md#archived-2026-10-15-preceding-data-checkpoint-command-joins-the-action-damage-consumer). The active resume point is above.

## Previous native checkpoint: exact Action contribution queries

[Historical report](implementation-history.md#archived-2026-10-15-previous-native-checkpoint-exact-action-contribution-queries). The active resume point is above.

## Previous evidence checkpoint: canonical Life numeric boundary

[Historical report](implementation-history.md#archived-2026-10-15-previous-evidence-checkpoint-canonical-life-numeric-boundary). The active resume point is above.

## Latest data checkpoint: fixed-Life canonical input admission

[Historical report](implementation-history.md#archived-2026-10-15-latest-data-checkpoint-fixed-life-canonical-input-admission). The active resume point is above.

## Preceding data checkpoint: native Amulet Life copying

[Historical report](implementation-history.md#archived-2026-10-15-preceding-data-checkpoint-native-amulet-life-copying). The active resume point is above.

## Preceding data checkpoint: reward support-source coverage

[Historical report](implementation-history.md#archived-2026-10-15-preceding-data-checkpoint-reward-support-source-coverage). The active resume point is above.

## Preceding data checkpoint: Mana pool adjustment inputs

[Historical report](implementation-history.md#archived-2026-10-15-preceding-data-checkpoint-mana-pool-adjustment-inputs). The active resume point is above.

## Preceding data checkpoint: checked Mana contributor domains

[Historical report](implementation-history.md#archived-2026-10-15-preceding-data-checkpoint-checked-mana-contributor-domains). The active resume point is above.

## Preceding data checkpoint: Intelligence-derived Player Mana

[Historical report](implementation-history.md#archived-2026-10-15-preceding-data-checkpoint-intelligence-derived-player-mana). The active resume point is above.

## Preceding data checkpoint: shared Player intrinsic Mana

[Historical report](implementation-history.md#archived-2026-10-15-preceding-data-checkpoint-shared-player-intrinsic-mana). The active resume point is above.

## Preceding data checkpoint: inherited and applied minion damage

[Historical report](implementation-history.md#archived-2026-10-15-preceding-data-checkpoint-inherited-and-applied-minion-damage). The active resume point is above.

## Current validation checkpoint: reproducible joined native replay

[Historical report](implementation-history.md#archived-2026-10-15-current-validation-checkpoint-reproducible-joined-native-replay). The active resume point is above.

## Preceding runtime checkpoint: checked application-group producers

[Historical report](implementation-history.md#archived-2026-10-15-preceding-runtime-checkpoint-checked-application-group-producers). The active resume point is above.

## Preceding native data checkpoint: shared Life routing

[Historical report](implementation-history.md#archived-2026-10-15-preceding-native-data-checkpoint-shared-life-routing). The active resume point is above.

## Historical acceptance checkpoint: application-group contribution queries

[Historical report](implementation-history.md#archived-2026-10-15-historical-acceptance-checkpoint-application-group-contribution-queries). The active resume point is above.

## Preceding native data checkpoint: Gem support-source domains

[Historical report](implementation-history.md#archived-2026-10-15-preceding-native-data-checkpoint-gem-support-source-domains). The active resume point is above.

## Current input checkpoint: authored support relationships

[Historical report](implementation-history.md#archived-2026-10-15-current-input-checkpoint-authored-support-relationships). The active resume point is above.

## Previous contract checkpoint: authored support order

[Historical report](implementation-history.md#archived-2026-10-15-previous-contract-checkpoint-authored-support-order). The active resume point is above.

## Publication checkpoint: support-source authoring

[Historical report](implementation-history.md#archived-2026-10-15-publication-checkpoint-support-source-authoring). The active resume point is above.

## Native checkpoint: composed support coverage

[Historical report](implementation-history.md#archived-2026-10-15-native-checkpoint-composed-support-coverage). The active resume point is above.

## Latest source checkpoint: composed support origins

[Historical report](implementation-history.md#archived-2026-10-15-latest-source-checkpoint-composed-support-origins). The active resume point is above.

## Latest integration checkpoint: Offering activation and application

[Historical report](implementation-history.md#archived-2026-10-15-latest-integration-checkpoint-offering-activation-and-application). The active resume point is above.

## Previous data checkpoint: exact Offering source scaling

[Historical report](implementation-history.md#archived-2026-10-15-previous-data-checkpoint-exact-offering-source-scaling). The active resume point is above.

## Previous runtime checkpoint: exact Skill queries and self-contributions

[Historical report](implementation-history.md#archived-2026-10-15-previous-runtime-checkpoint-exact-skill-queries-and-self-contributions). The active resume point is above.

## Previous runtime checkpoint: checked Actor and reward contributions

[Historical report](implementation-history.md#archived-2026-10-15-previous-runtime-checkpoint-checked-actor-and-reward-contributions). The active resume point is above.

## Previous native data checkpoint: canonical Life contribution queries

[Historical report](implementation-history.md#archived-2026-10-15-previous-native-data-checkpoint-canonical-life-contribution-queries). The active resume point is above.

## Previous native data checkpoint: all selected equipment placement

[Historical report](implementation-history.md#archived-2026-10-15-previous-native-data-checkpoint-all-selected-equipment-placement). The active resume point is above.

## CI throughput checkpoint: complete CLI target partitioning

[Historical report](implementation-history.md#archived-2026-10-15-ci-throughput-checkpoint-complete-cli-target-partitioning). The active resume point is above.

## CI repair checkpoint: downstream Boolean cutover checks

[Historical report](implementation-history.md#archived-2026-10-15-ci-repair-checkpoint-downstream-boolean-cutover-checks). The active resume point is above.

## Preceding coverage checkpoint: flat-Life component scalability

[Historical report](implementation-history.md#archived-2026-10-15-preceding-coverage-checkpoint-flat-life-component-scalability). The active resume point is above.

## Native checkpoint: real Player Life contributions in the joined graph

[Historical report](implementation-history.md#archived-2026-10-15-native-checkpoint-real-player-life-contributions-in-the-joined-graph). The active resume point is above.

## Player Life reference checkpoint

[Historical report](implementation-history.md#archived-2026-10-15-player-life-reference-checkpoint). The active resume point is above.

## Native checkpoint: real Strength and checked flags reach inherent Life

[Historical report](implementation-history.md#archived-2026-10-15-native-checkpoint-real-strength-and-checked-flags-reach-inherent-life). The active resume point is above.

## Native checkpoint: bounded class/passive BASE membership

[Historical report](implementation-history.md#archived-2026-10-15-native-checkpoint-bounded-classpassive-base-membership). The active resume point is above.

## Source checkpoint: prepared hands and original condition writes

[Historical report](implementation-history.md#archived-2026-10-15-source-checkpoint-prepared-hands-and-original-condition-writes). The active resume point is above.

## Previous native checkpoint: Boolean Gigantic and intrinsic Life in the Sniper graph

[Historical report](implementation-history.md#archived-2026-10-15-previous-native-checkpoint-boolean-gigantic-and-intrinsic-life-in-the-sniper-graph). The active resume point is above.

## Source checkpoint: Offering grouping and exact support delivery

[Historical report](implementation-history.md#archived-2026-10-15-source-checkpoint-offering-grouping-and-exact-support-delivery). The active resume point is above.

## Source checkpoint: manual Djinn admission accounting

[Historical report](implementation-history.md#archived-2026-10-15-source-checkpoint-manual-djinn-admission-accounting). The active resume point is above.

## Retained incoming-critical proof and superseded ownership draft

[Historical report](implementation-history.md#archived-2026-10-15-retained-incoming-critical-proof-and-superseded-ownership-draft). The active resume point is above.

## Independent progress: default-encounter damage-roll evidence

[Historical report](implementation-history.md#archived-2026-10-15-independent-progress-default-encounter-damage-roll-evidence). The active resume point is above.

## Checked baseline and original-build results

[Historical report](implementation-history.md#archived-2026-10-15-checked-baseline-and-original-build-results). The active resume point is above.

## Complete-build delivery gates

These gates remain current; the chronological checkpoints above are evidence,
not competing implementation queues. The top resume point names the current
package and immediate work. Actor/reward, exact Skill, application-group and
exact Action contribution queries, coherent numeric selection and composed
support discovery are implemented. Offering and Command already use those
contracts. Do not implement them again or seek repeat approval.

The remaining route is actual final metric production and complete input/owner
admission. Reuse the checked Player Life contributors, implemented numeric-domain
operation and retained final-resource source witnesses. Complete the equipment
Life magnitude/routing and adjustment sources, then admit the already-published
guarded resource consumers with their complete plan and metric dependencies.
The Sniper damage route next needs pre-conversion base production, followed by
the remaining downstream consumers. Complete coverage only where potential
supplier inventories and numerical behavior are proved; source observations
and finite component replays do not replace either requirement.

The selected saved Life query is Player. Its remaining supplier, conversion-order
and whole-plan dependencies are independent of the now-joined six
minion-Life sources and inherent minion zero contribution. Those joins advance
Actor coverage; they do not close the saved Player metric. Do not replace a
missing producer with a supplied scalar or call a finite composition a complete
build.

The current input audit found no existing-authority shortcut to close the
four obligations. Preset usage `0505` retains unresolved global switches,
action selection and FullDPS responsibilities despite known preferences.
`01f2` retains 30 origins after the published correspondence and configuration
accounting steps; known values and remaining Firebolt correspondence are separate.
`0207`'s 23
known assumptions do not prove the constructor/default catalogue complete;
`0208`'s empty saved usage does not account for seven default child-effect enables.
Known scalar values cannot certify these inventories. Checked configuration
accounting and per-consumer projections are implemented. Reporting inclusion
uses accepted exact preset preferences and a Complete-empty gameplay owner;
it requires no new runtime API, aggregate metric or change to selected-action
DPS. The packet retains typed Boolean intent without a mechanical producer.
Actual aggregate execution, any new
report target and origin-sensitive tie semantics remain separate design gates.
The damage-roll packet accounts for the exact numeric
Placeholder only under its proved absent-boss guards; broader encounter branches
remain unadmitted. Continue numerical dependencies and source accounting
without inventing completion or adding scalar-only patches that leave every
selected obligation unchanged.

**Narrow source follow-ups under existing authority:**

- The [focused Offering witness](#source-checkpoint-offering-grouping-and-exact-support-delivery)
  now records original local products/rounding and exact Danse delivery. Use it
  to review owned group membership and support applicability; do not rerun the
  same probes or mistake an inherited source store for native ownership. The
  additional-skeleton activation condition remains unresolved. Exact Skill and
  composed-support contracts are implemented; reuse their checked authority
  without assuming an untranslated supplier or applicability domain is complete.
- The [manual Djinn admission witness](#source-checkpoint-manual-djinn-admission-accounting)
  now records the actual Original05 global2 controls through original consumer
  admission. Next use its exact queried-empty versus not-called distinction in
  the current `0505` disposition review. All these Djinn rows already have usage-only
  responsibility; they cannot remove any further `01f2` link. Preserve unresolved
  supplier and archived responsibilities; neither unchanged numbers nor this
  finite witness certifies an entire inventory. Existing other-SkillName and
  Purifying controls need not be duplicated.

**Original05 completion path.** The public finalizer returns Pending while any
selected input obligation remains, before constructing an owned request. The CLI
therefore cannot export the complete request; `evaluate-owned` also requires an
evaluation bundle, absent from the current package. Work in this order:

1. The authored-support input issue (`01de`) is resolved and the common native
   discovery gate is implemented. Finish reviewed source domains and exact
   provider exclusions through that cold-composition seam. All 23 selected physical
   Gems now have reviewed support domains; other owner domains remain unknown.
   Missing Command mechanics are independent of their now-proved classification;
   do not invent missing source definitions or use preset-only certificates.
2. In parallel, account for selected preset usage (`0505`), configuration (`01f2`),
   external assumptions (`0207`) and scenario usage (`0208`) through reviewed
   consumers, preserved responsibilities or justified non-applicability. Reuse
   retained source evidence; do not equate known values with complete inventories.
3. Produce the unchanged Original05 request through the public finalizer with
   zero selected input obligations and all 22 query identities preserved. No
   handwritten replacement request or patched completion flags.
4. Publish the actual evaluation bundle and resolve its selected owner, preparation,
   receiving and metric dependencies. Both selected Sand forms still need genuine
   coverage: the current Engine checks the complete selected plan and does not
   authorize pruning them just because the selected damage query is Sniper.
   Each numerical slice must name the Original05 dependency it removes.
5. Validate the full CLI path against the fixed metric/availability manifest,
   independent stable reference runs and fresh/reused/parallel native execution.
   Preserve genuine infeasibility, including negative unreserved Spirit. Once
   this gate passes, start D4 immediately while continuing the remaining builds.

Only change the priority build if a checkpoint demonstrates a shorter complete
path for another original; record the reason. Closely related contrast tests and
regression repairs remain required, but unrelated breadth/performance/cleanup
work must not displace this path.

**Sand priority evidence.** Retained `owned-djinn-provider-source-01` controls
disable manual Sand, reduce its raw level to one or disable its Bidding support;
each preserves all 785 Player and 645 selected Sniper MAIN output fields in both
JIT reports. `owned-selected-participation-source-01` case11 removes only the tree
Sand provider plus UI focus indices and preserves the same outputs at all three
lifecycle stages. These controls do not prove global noncontribution: the other
Sand occurrence remains, and persistent-minion-type counts have real consumers.
The current selected metrics contain no Sand damage/Command or combined-DPS
query. Thus Sand's own combat/population formulas are downstream complete-plan
coverage work, not established numerical dependencies of the selected Sniper
outputs. Do not remove the occurrences, invent completeness or silently adopt
the still-proposed [scoped coverage contract](owned-coverage.md). Reassess the
actual remaining plan gaps after the request is admitted.

**Remaining generated preparation.** Sand's actual ordinary/source assembly,
Command input projection and override-free actor-level projection are now authored
and component-tested in the latest packet. Its source/effect/channel/external/
support inventories remain Partial, and no receiving/readiness bundle is published.
Complete incoming Minion-level contributors, actual support composition, actor
attacks, Command mechanics and four population override modes remain separate
coverage work. The generated provider's noSupports fact does not suppress external
supported properties or manual supports. Actor children are not extra source owners.

Item-generated Firebolt `0134` still has no authored ordinary/final assembly. Its
raw grant domain includes zero; retained Original04 controls show raw17/quality0
becoming prepared26/21, with a fractional-quality contrast. Do not copy Sand's
Minion channel or physical-Gem corruption/recovery rules. Firebolt is an
Original04 breadth blocker, not a reason to switch away from Original05.
The accepted [generated-source contract](owned-generated-source-properties-proposal.md)
already provides exact source ownership and provider projection. Use existing
typed inputs and actual consumers; no second input store or new public model is
currently justified.

Saved participation transport already uses existing `332b` for all twelve
selected roots in Original05, including manual Sand. Remaining work is actual
readiness and mechanics coverage, not a second enabled-flag import packet.
Generated bindings retain `WhenExactSourceSelected`; Original04's unresolved
Firebolt provider and archived cross-axis joins stay unresolved. Reuse the
[generated evidence packet](../data/owned/poe2/3887ae68/generated-participation/README.md)
and retained source controls. A selected PoB group can still preview a disabled
generated effect; this supplies no native force-enable authority. Global switches,
FullDPS/reporting intent and the other selected issues remain separate; do not
close all usage `0503` on the strength of one enabled Boolean.

The next input evidence work should classify the retained
`owned-extra-stat-consumption-source-05` suppliers and filters, which already
include inherited-node and Amulet-copy transport. Do not repeat that acquisition
or treat a new scalar grammar as progress on these selected obligations.

**Warrior selector/supply status:** the October 8 preflight is
[historical](implementation-history.md#archived-2026-10-15-warrior-selector-preflight-from-october-8).
The subsequent [Warrior correspondence packet](../data/owned/poe2/3887ae68/warrior-generated-correspondence/README.md)
reuses an exact selector/deferred-field proof for eight archived source rows,
retiring their configuration fallback while preserving their same-preset raw,
usage and support obligations. That publication reduced source links 44 → 36;
later accounting brings the current count to 30. It does not fabricate physical
supply or implement the broader proposed Import refactor. The
[selector/supply separation proposal](owned-source-action-root-separation-proposal.md)
remains a distinct pending design decision, not an instruction to repeat the
completed bounded correspondence work.

**Accepted composition work: support-origin discovery.** The selected Original05
preset currently has sixteen physical support assignments in eight known
sequences, but those are not the complete runtime source inventory. The source
also discovers additional effects per Gem, ExtraSupport, generated item slot
sharing, LinkedSupport and provider-specific noSupports behavior. Existing
`support_inventory::Census/complete` proves physical membership only. A preset's
former `support_origins` field survived independent equipment/tree changes, so
an absence certificate derived only from today's selected Item28 or tree is unsafe. Keep
authored support assignments distinct from complete runtime origins. Review the
existing `support_origin_order`/shared-source/provider-resolution seam and cold
composition validation before retiring `01de`; the
[public ownership/completeness proposal](owned-support-origin-composition-proposal.md)
was accepted October 8. The current checkpoint above implements the authored-order
DTO and common native gate; source accounting and game-data adoption remain pending.
Complete provider coverage can prove absence only
where it accounts for every relevant capability. No new runtime origin variant
is needed merely to describe a sound Assignment-only domain, but Partial rules
cannot be interpreted as absence of extra supports.

The same audit found no supported shortcut for the other four selected
obligations: 23 imported preferences do not account for every field across twenty usage
rows in ten groups; configuration `01f2` still covers 44 origins (21 inside
Config and 23 elsewhere); seventeen known external assumptions do not account for
28 non-reward effective defaults, including seven child buff/curse/aura enables.
Scenario usage remains empty/Pending. Preserve these distinctions rather than
closing global inventories after adding another local scalar recipe.

**Configuration/default transport and next consumer accounting.** The retained
`owned-configuration-inputs-source-01/source-jit-{off,on}.json`,
`cases[4].state.saved`, identifies all 28 non-reward defaults and 34 Placeholders.
Typed constructor-default projection is now implemented for declared Player,
Enemy and Environment external inputs. The real Player resistance-penalty rule
consumes its first new published default. Exact source presence, type, default
and Placeholder semantics remain an Import concern; the native evaluator reads
ordinary injected inputs. This does not complete either source-field inventory.

Seven true defaults have actual source child-effect consumers: Spectre buffs/
curses, Companion buffs/curses and Elemental Relic Anger/Hatred/Wrath auras.
Pinned `ConfigOptions.lua:544-549,644-659` emits their tagged enable records;
`CalcPerform.lua:2592,2634,2735` consumes them. Config dispatch does not require a
visible widget. These observations preclude an empty scenario-usage proof based
only on Original05's absent authored controls or unselected child families.
However, the current owned package has no corresponding child consumer programs;
adding seven pass-through inputs alone would not remove a native blocker.

For real global usage consumers, reuse `ScenarioInput.usage` with the existing
`UsageTarget::Actor(Player)` and exact child/application rules reading
`RuleEntity::Player`. This does not need a new Core owner, copied skill presets,
scenario fanout or persistent child selectors. Establish actual selected
applicability and native consumers before publishing preferences; a usage rule
cannot synthesize child grants. Continue the reviewed whole-Config source-
disposition census, including direct readers, Placeholder semantics and custom
blocks. Support-discovery ownership remains the outstanding owner question;
configuration accounting can proceed through the accepted contracts independently.

**Deferred coherent item-layout work.** Original04's selected Gold Amulet needs
Paragon's granted passive, remaining Rarity accounting and all-skills quality;
its global ES, Life and Spirit category inputs stay Pending. Original05's
selected Solar Amulet has no anoint and already has Proven layout. Dominion is
on its dormant Stellar Amulet, so completing it will not solve the current five
selected issues. Paragon (`20686`/owned `0d5e`) and Dominion (`26214`/`0ef4`) are
anoint-only disconnected nodes, not ascendancy nodes. Current
`GrantTarget::AllocationAccess { pools }` authorizes a pool without an exact
node, and Engine rejects granted allocations/access providers as
`UnsupportedRelation`. A future anoint checkpoint must review exact target
permission, provider lifetime, deduplication and point accounting before
implementation; it is not a data-only grammar extension. Paragon's +5% skill
quality and the separate +5% item line are distinct contributions.

**Accepted generality checkpoint (2026-10-04).** The [review and gates](build-generality-review.md)
record concrete limits in generated-source ownership, application stacking,
physical/reference import coupling, coverage scope, normalization compilation,
owned objective binding and legacy isolation. The initial read-only review is
complete; the cross-family, integration, holdout and CI gates remain open.
The generated raw-input publication passes for its reviewed families. Prioritize
remaining preset/scenario usage and actual contributor coverage next. Reuse the
checked exact-provider join rather than adding another source matcher. Before
changing a public contract, use contrasting real cases and record
its data-only extension boundary. Preserve the five original requests and their
110 query rows at every publication. Neither this review nor producer coverage
changes the 0/5 complete-build result.

**Player-Life dependency status:** the Strength-to-Life receiver, Count cutover,
six ordinary attribute consumers, guarded empty-MORE producers, six BASE/INC
memberships and five inherent-flag memberships are implemented and tested. The
current package uses rule schema 3 and operations V24. Its six BASE query groups
are Complete (328/328/330 members in each pass); the five flag groups are Complete
with 0/0/0/1/1 declared sources. Global query/owner coverage remains Partial.
The existing conservative validator still rejects an unmatched potential effect
even if inactive; membership completion does not admit unsupported donors.

The BASE publication follows the bounded Class/passive donor law described in
the checkpoint above. Broader item donors and post-passive bonus copies still
need destination-complete ordering and grouping proofs before expanding that
domain. Reuse the importer's explicit modifier-order proof; raw saved line order
is not source modifier-list order. The retained signed-cancellation counterexample
still rules out the old class/item-prefix plus passive-suffix shortcut. Nonempty
MORE grouping remains unresolved. Use the actual published producers, not
fixture literals or a cache-dependent native mode. Inherent flags and their
Life contribution are finished prerequisites; final pool aggregation now needs
the accepted origin-authority implementation and its own coverage/arithmetic law.

**Class coverage follow-up:** Original05's Class `0a23` still has its inherited
`tree-game-rules-not-converted` marker. Its Count bases and intrinsic attack row remain class-owned; intrinsic Life
now belongs to shared Actor `332a`. The shared initialization inventory stays
Partial. The complete residual class-specific inventory and all separate gates
must be reviewed before the Class marker changes. Shared action selection, resources and conditions retain their
own coverage. The shared implicit root `1790` is now complete for its default
intrinsic inventory; do not reopen its proven empty lists or use that result to
close Class behavior, neighboring modifiers or external transformations.

The owner accepted [existing-actor rule ownership](owned-existing-actor-rule-ownership-proposal.md)
on 2026-10-06. Actor `332a` now binds explicitly to the existing Player, and shared
intrinsic Life has been removed from all eight Class owners. Keep the
unimplemented shared initialization inventory Partial. This supplies honest
ownership without duplicating universal rules across classes or manufacturing
a Player grant. Contract implementation, the first data migration and source/native
validation pass; no additional whole-build closure is claimed.

**Class audit result:** [closure is withheld](owned-class-coverage-audit.md).
All eight residual class program inventories and the real separate action gates
were reviewed. The actual loaded class-table branch and all eight rows are now
authenticated, and the accepted slot relation has published structural off-hand producers.
Before a data-only closure, give the remaining effective equipment-derived Actor
conditions explicit producer ownership and source evidence. Reuse existing equipment delivery
and Actor rules through the [Player slot relation](owned-equipment-slot-state-proposal.md); do not
substitute Action query selection or template identity for hand state. Keep all
eight Class markers and shared Actor `332a` Partial. The audit records exact
source readers, current consumers and the smallest proof/control work.

The six Add and six Increase queries and the five inherent-control queries have
the bounded membership described above. Neither flag-producing passive is selected
in Original05. The real selected class/passive stream now gives Strength 27 and
inherent Life 54 through the
[typed Boolean aggregation contract](owned-boolean-contributions-proposal.md).
Class/global-owner coverage and final Player Life remain open; do not reopen
finished query memberships or confuse them with these distinct blockers.

**Input resume:** keep Original05's five selected issues and 44 origins linked
to configuration issue `01f2` open. Its 101 item-range records already have exact
item/output or unresolved-item ownership; the remaining origins are 21 Config
and 23 outside Config, with all 23 outside origins retaining only this issue.
The [generated-source consumer census](owned-configuration-dispositions-proposal.md#generated-group-consumer-census-next-blocker-2026-10-05)
must distinguish execution/reporting intent from PoB reference selectors for
the remaining Skill/Gem origins. The selected-output proof retired six generated rows from fallback; the
archived proof now accounts for fourteen more through their actual raw-input,
usage and support-discovery owners without completing those inventories. Reuse the shared resolver and field-level proofs;
selected-context evidence cannot close dormant preset rows.

The declared-stat/global-switch census and actual-parser SkillName controls
already pass. The [item SkillId control](owned-configuration-dispositions-proposal.md#item-granted-skillid-control-2026-10-05)
now adds exact item/grant/receiver transport, mapped output, supplier-line removal
and strict repeat/JIT comparison with retained raw diagnostics. It supplies no
native closure authority. The [actual consumer witness](owned-configuration-dispositions-proposal.md#actual-extra-stat-consumer-witness-2026-10-05)
now captures the extra-stat argument at `CalcActiveSkill.lua:795`, bound to unchanged
Original05's source bytes, selected axes and contemporaneous ancestry. It preserves
nine equipped uses over eight items, attribute-override projections, support/skill
records and Config input/default evidence. Source04 corrects inherited passive modifiers and
adds actual original node-builder returns; use that effective view rather than
source03's local-only node lists. Source05 additionally proves exact Item23 Amulet ScaleAddMod/AddMod transport,
including both retained zero-valued records and their receiver/slot identity.
Next classify the captured supplier/filter dispositions and their transport to
the actual consumer before authoring field exclusions. Complete NodeModifier and Amulet-copy laws,
empty-source exclusions and dormant presets are not proved by the selected frame.
Preserve unknown nested records and transforms. Post-load absence is insufficient;
an exact mandatory filter must survive transport before it can justify an exclusion.
Keep this finite original-context proof separate from arbitrary candidate coverage.

The Magnified Area I/II contributor slice and its actual Action eligibility
producer now pass source, publication and native checks for the reviewed fourteen
contexts. Use the published producer rather than fixture-supplied Area facts in
new integration work. Primary Summon delivery, final radius/resource-cost/damage
consumers and complete receiving remain open. Continue that native integration
alongside the bounded Original05 input proof and participation coverage
for remaining skill families.
Do not expand generic source-proof infrastructure as a substitute for native progress.
Rapid Casting I/II is now published on physical Ice Nova's output `3280`
across both stat sets, with unchanged prepared-input prefixes, cast-specific
Increase channel `32fc`, exact physical support controls and parallel replay.
Spell membership is not the source's Cast predicate; no general timing classifier
or final cast rate is supplied. Neither tier authors a cost/reservation entry.

Encroaching Ground Gem `0740`/Skill `03f8` now publishes factor 1.1 through
ordinary-cost channel `32fa`, with exact source and native composition controls.
Its ground growth remains unresolved. The source's 1.43 support subtotal and the
native unrounded Product are explicitly different contracts; do not hide this
rounding question behind tolerance or generic source emulation.

**Selected-source checkpoint, 2026-10-06:** the [generated-source accounting contract](owned-generated-skill-dispositions-proposal.md)
uses the existing Import policy and one current implementation. Its private
proof accounts for admitted fields against the exact provider and emitted
raw/count bindings while retaining existing Pending usage. All 287 normalization,
184 library, 16 Direct-disposition and eight Direct-source tests pass. The CLI
reimport of all five unchanged originals passes: four origins change in Original01,
six in Original05, and none in the other three. Draft values, IDs, allocations,
selections and other proof dependencies remained exact. Sidecar21 and exact CLI
proof hashes identify that historical evidence. The latest archived-accounting
checkpoint above uses sidecar23 for configuration-input policies. No old importer branch is retained.
Unknown fields keep configuration fallback without discarding successful values.
Evidence: `runs/owned-generated-field-accounting-01/validation.json`.

Numerical integration should use the published real Offering output/recipient
topology and complete actual incoming contributor and selected-owner coverage
below. Settle ordinary-cost reduction and rounding with contrasting valid-domain
inputs. Do not invent a parent Action or use source absence to close ground
growth. The native participation consumer and first Sniper input/rule packet
are implemented; other skills retain their explicit inventory gates. Other new
recipient contracts require review.

Selected Pain Offering uses Prolonged Duration **I**, while II occurs in saved
alternatives. Its duration/cost factors now use real player output `32fd`;
final duration/cost consumers and complete contributor inventories remain open. Final OrdinaryTiming requires
all eight explicit inputs, branch/rounding authority and complete contributors.
These are broader integration gates, not defaults supplied by Rapid's channel.

The Offering final-input source, publication and seven-test native component
now pass, together with the public Engine, historical native, authoring,
strict workspace Clippy, formatting and WASM compilation checks recorded above.
The real item programs,
canonical rolls, shared source census and final projections `3223/3224` execute
in one plan; do not restore final22/0 literals or pass intermediate values through
a second evaluator. Stages V4's bounded local facts preserve production ownership
and dependency checks.

The [Amulet snapshot reducer](../data/owned/poe2/3887ae68/amulet-bonus-snapshot/README.md)
is now published and integrated. The next numerical blocker is **actual selected
item/source contributor and owner coverage**, not another producer for 32e4.
Keep using the real reducer, existing copy program and explicit stage boundary.
Establish ordinary item, external and supported-property inventories before
replacing finite test closure with actual release authority. The global
selected-owner gate still refuses scalar/reduction reads if another selected
owner is Partial; a measured empty source bucket cannot override that gate.

Selected Modifier `30ca` now has the ordinary Talisman recipient guard. Prioritize
the remaining branch/applicability certificate and actual template closure. Focus preprocessing at `CalcSetup.lua:1473–1484` now has a passing original-call witness. The late non-Amulet copy witness proves
bounded delivery after preparation; Talisman's dedicated witness proves ordinary
Amulet diversion separately from early copying. Use these consumer observations;
final ModDB presence alone does not establish an earlier level bonus. Completing
a real selected owner is more useful now than
adding an absent jewellery-bonus family. Final Offering duration, payable cost
and recipient scaling remain subsequent consumers with their own original-call,
incoming-coverage and rounding evidence. Any new public contract remains a
reviewed design decision.

Historical `3225/3226` descriptors now have no executable consumers or writers.
The published structural primary-supply activation remains exact; post-census
source assembly projects final parameters directly from computed `30ac/30ad`.
Preserve historical receipt replay and allocated IDs without restoring that
redundant intermediate path. The proved level domain is exact integral lookup
1–40; source quality input preservation supplies no additional quality-effect
recipe or supported-quality producer.

Classify `validateGemLevel` recovery separately under the Lua cleanup gate.
Original05's integral `20 + 0 + 1 + 1 + 0 = 22` path does not justify natural-20
fallback, arbitrary table-entry selection or treating table extent as a game cap.
Use supported integral lookup and explicit coverage refusal until broader valid
candidate semantics are established. Fractional controls test reachability;
they do not establish game legality merely because the wire type represents them.

Do not route Magnified's ordinary-cost factor `32fa` into Spirit reservation
input `326e`: their source fields and consumers are distinct. Current support
receivers are Actors or Actions; `ReceivingSkill` names an admission context,
not a third receiver kind. Do not synthesize primary Summon Actions, invoke a
parent contribution once per child Action, or repurpose source-property
preparation as execution authority. A genuine parent demand/recipient extension
needs contrasting real cases and review before changing the public contract.
The valid-but-unreviewed part/stat-set control now passes: the existing guard
produces Inactive and the demanding contribution reports UpstreamUnavailable,
without a Boolean default. MissingProducer remains the separate absent-program
case. Future receiving/selection expansion still needs actual producer evidence;
these test-only declarations do not extend the published game domain.

Do not reuse Frost switches as general activation. Existing usage programs inherit
activation gates, so activation writing also needs a dependency-order review.
The [participation proposal](owned-skill-participation-proposal.md) now makes that
decision concrete: retain mechanical supply, derive requested participation from
existing typed usage, and declare its execution consumers on the same graph.
The native consumer and first Sniper input/rule packet are validated; other
families still need their own complete input and source authority.
It also records PoB's selected-group enabled/loadout bypass and default-skill
fallback. Preserve exact reference identities and prove selected/nonselected
controls; native query selection must not silently activate a skill.
The [bounded generated-source accounting follow-up](owned-configuration-dispositions-proposal.md#bounded-generated-source-accounting-follow-up-2026-10-05)
is implemented for the admitted generated Skill/Gem pairs. It proves every field
against the exact provider, actual emitted quality/count bindings, reference
correspondence and existing same-preset Pending usage. Six Original05 rows now
carry the existing usage obligation instead of configuration fallback; 73 origins
and all five selected issues remained at that checkpoint. The subsequent cached-
Buffs proof reduced this to 72, archived responsibility accounting to 58,
and overwritten configuration placeholders to the current 44. Reuse the same proof seams for further
coverage and retain fallback if any field or reference remains unaccounted;
source05 cannot certify a universal absence of extra stats. No new public
participation contract or global-switch non-applicability follows from this plan.

Dormant set 2 currently has Complete empty usage,
so it cannot receive an invented Pending link. Known quality or count alone does
not establish enabled/global/reporting/action semantics. Handle mixed containers through their
actual consumers under the [proposal's](owned-configuration-dispositions-proposal.md)
private correspondence and global retirement rules. Preserve every live selected
obligation until its actual inventory/consumer proof passes.

The breadth replay also identifies five Original02 overlays with Proven source
line targets but Partial modifier-roll declarations (Fire, Lightning and Evasion).
They deliberately keep fallback; source layout alone is not complete output
authority. A later ownership refinement must bind their actual pending roll
obligations, or supply the missing declaration proof, without weakening the
known-output gate. Overwritten/out-of-bounds and nonmodifier-emission range
ownership also remain explicitly unimplemented. These are Import coverage
limits, not separate native item systems.

Local Config coverage is still needed over the full pinned catalogue, not just
Original05's eight authored reward Inputs. PoB constructs 28 non-reward defaults;
seven hidden child-effect enables emit actual modifiers. Reuse reward, encounter,
level and scalar proofs with exact selector/output correspondence. A partially
successful scalar token is not proof for every channel. The full configuration
policy remains proposed; the implemented integrity gate only rejects dangling or
missing issue correspondence. Do not move every unknown to assumptions or usage
to manufacture closure. Preserve all five remaining selected obligations until their
actual consumer/inventory proofs pass.

Then return to generated activation, global switches and reporting inclusion.
Requested counts already share exact provider joins and typed decoding. The
original-function [Full DPS witness](owned-full-dps-aggregation-proposal.md)
authenticates contributor and display-row identity; native aggregation and its
tie/actor/action contract remain separate work. Selected-action `TotalDPS` keeps
its meaning. No fake SkillUse, synthetic union build, default quality/count or
implicit retargeting may close an input obligation. At each publication rerun
all five unchanged selections and fix the next measured blocker.

The [second retirement audit](legacy-retirement.md#second-pass-audit-resistance-terminology-and-end-state-ownership-2026-10-05)
distinguishes old application adapters from shared resistance arithmetic and
older owned modifier representations. Fixed Elemental still has a live early
conversion rule; migrate that semantic path before deletion. Genuinely orphaned
Import equipment/Mace parsing is now removed and its retained library tests pass. Preserve shared
source loading, formatter laws and useful numerical/reference tests.

**Lua cleanup gate (requested 2026-10-05).** Run the
[behavior/type inventory and classification](legacy-retirement.md#lua-compatibility-cleanup-gate-requested-2026-10-05)
alongside D0/D5 and the original-build integration work. Prioritize source
truthiness/defaults, numeric formatting, rounding/min/max, ordering/aliasing and
cache lifecycle in shared helpers. Retain only justified domain laws; isolate
external format conversion and optional reference behavior. Each proposed removal
needs its valid-input impact and evidence recorded. Do not reproduce a Lua quirk
merely because a compatibility test asserts it, and do not silently change
numerical outputs when game intent remains unresolved.

1. **Complete generated usage and remaining source accounting.** Original05
   now has five selected obligations and 23 selected preferences: four skeletal
   counts, Offering and Frost Bomb switches, five Direct/Tree/Item occurrence
   counts, and twelve requested-participation preferences. Remaining global/
   reporting usage and full field accounting need the
   [generated applicability contract](owned-generated-skill-usage-proposal.md).
   The versioned Core contract, all-stored-row proofs, V6 permissions and V19
   input producers pass focused validation. Use that checked boundary;
   do not relax the legacy field's supplying-preset scope. Preserve unknown
   source fields and all existing query identities.
   The reviewed Tree Djinn and item-granted Firebolt now have exact selected
   saved-input/provider correspondence. A source tree-node ID is not a project occurrence: independent
   allocation presets can contain distinct allocations of that same definition.
   Prove the saved selection's exact provider join before projecting preferences;
   never choose the first match, fan out to every allocation or silently retarget
   archived combinations. Generated quality12.5 survives source reconstruction; a
   provider-wide zero is invalid. On 2026-10-05 the owner accepted
   [preset-owned exact generated input bindings](owned-generated-skill-inputs-proposal.md):
   raw quality uses shared typed slots and explicit producer permission, separately
   from usage. Tree Djinn and item Firebolt raw levels already have provider
   projections; preserve those writers instead of adding redundant level rules.
   Generated usage alone cannot fill the quality it requires to execute.
   The source join investigation confirms that Original05's `Tree:13289`
   occurs in Specs 2–6 and `Tree:32705` in Specs 3–6; SkillSets contain no
   implicit Spec association. Its active axes are `Tree.activeSpec=3`
   (position), `Skills.activeSkillSet=4` (key) and `Items.activeItemSet=2`
   (key). Bind the saved active SkillSet only through those independently
   verified axes. Archived combinations need explicit source correspondences
   or remain Pending; titles and coincident set IDs confer no relationship.
   Firebolt's Item28 is equipped in ItemSet2/Weapon1; its provider must join the
   actual EquipmentUse and unique granting Modifier, including source slot and
   item identity. The shared importer now performs these joins after real records
   exist and rejects stale, ambiguous or missing correspondence. Reuse this
   validated boundary when adding usage dispositions.

   The published slice authorizes only quality on tree supplies `32d2/32d4`
   and shared slots `3262/3264`, plus Firebolt Skill`0134`/supply`31c9`/quality`32ef`.
   Level`31ca` remains provider-produced. Opt-in normalization explicitly converts
   existing usage rows to the versioned envelope while preserving unresolved
   completion/issue IDs. It does not close the independent usage inventory.
   Requested primary counts now resolve through UsageV3 for the reviewed Direct,
   Tree and Item occurrences; only the source-proved generated `nil` spelling
   maps to one. Count/group transport does not complete enabled/global/Full DPS
   semantics. Account for those settings across authored and generated contexts
   without unreviewed defaults. The configuration-role issue retained 79 origins
   after presentation/range accounting (237 in the original census); the generated-field proof reduced that to 73 and the cached-output proof
   to 72; archived responsibility accounting left 58, and raw-placeholder
   accounting now leaves 44, including global
   fallback records outside Config. Their complete
   disposition census and actual consumers must be proved; additional numeric
   controls alone cannot certify an empty scenario-usage inventory.

2. **Connect selected receiving and property contributors.** The two single-line
   Minion Damage nodes, four Life/Damage nodes, two plain Life nodes and twelve flat/permanent reward
   owners now have complete default producer behavior. The Command slice also
   closes six selected passive owners and exercises Player-to-Actor-to-Action
   semantics using existing contracts. The Fire/Lightning
   input channels `32e6/32e7` are separate from final resistance metrics; completed
   reward producers do not supply their final reducers or contributor closure.

   The additional selected Reward owners AilmentThreshold `0036`,
   FlaskLifeRecovery `003b` and Charm `002d` now have complete producer programs
   through channels `32f0..32f3`. Both Charm effects remain together, with exact
   option/value provenance and Reward roots. Their receiving, final formulas
   and other incoming contributors remain open; producer closure does not make
   whole-plan reductions available. Growing Swarm now closes its complete default
   passive owner with distinct unconditional cooldown and Area inputs. Resume
   remaining selected passive/item/support owner gaps next.
   Component sums never authorize full pools, absent contributors or defaults.

   **Published native component: Magnified Area I/II.** The selected manual
   Sand and Ice Nova supports are Gem`082a`/Skill`047f` and
   Gem`082b`/Skill`0480`. The authored packet converts both Area increase (35/45)
   and support resource-cost multiplier (1.3); area alone is not the complete
   producer. The source's mana-oriented name does not limit its resource consumer.
   The pinned
   `sup_int.lua:6684/6716` families differ in the zero damage key: I has no map,
   while II maps an Area-conditioned zero MORE modifier. Preserve that distinction
   without inventing an I-tier producer. The packet reuses ordinary support
   preparation and receiving programs, with Action channels `32f9/32fa` and
   Boolean eligibility input `32fb`; `32f7` remains Actor-scoped. The shared Bidding
   fixture now proves exact receiving, removal, duplicate-family selection and
   occurrence isolation, including physical Ice Nova and fourteen source-observed
   action/stat-set contexts. Keep owner coverage Partial until every actual producer/receiver
   inventory is proven; final radius and cost formulas are separate gates.
   The actual Area eligibility producer is now published for eleven outputs and
   fourteen stat sets, using the reviewed base/support/part/map writer census.
   Use those programs instead of fixture facts; other domains need their own
   producer evidence. Primary Summons have no owned Action
   output in this fragment; their delivery and reservation consumers remain open.
   This uses existing public contracts. Defer Muster's participating-minion
   population and Frost Nexus's ground-effect semantics until their dependencies
   have deliberate ownership.

   The published Command receiving slice uses existing APIs: six conditional
   cooldown passives -> Player carrier -> exact Sniper actor slot
   `Skill0012/ActorSlot001f` -> guarded Basic/Gas Action rules. Its subtotal is
   Gas 72 / Basic 0. The independent source04 witness observes the full seven-source
   inventory 92 / 20; Growing Swarm now contributes the separate unconditional 20
   through channels `32f4/32f5` into existing Action channel `32ea`, retaining the
   conditional path. Area20 reaches the Actor input through `32f6/32f7`.
   Its default declaration inventories close, while both actual Action owners
   remain Partial. Basic's 20 is a diagnostic store query,
   not an observed original cooldown call. Do not compare the conditional native
   subtotal to the unfiltered source total or publish a full cooldown formula.

   Gas output `0025` now uses its three authenticated alternatives (Impact,
   Poison Cloud, Explosion), replacing placeholder `0009` through the checked
   migration. No extra Command root or Direct Skill is introduced, and no existing
   query targets Gas. The seven IDs are `32e8..32ee`. Exact Actor receivers and
   ordinary guarded Action rules implement the path; effect applications do not
   target Actions. Existing Gas/Basic Partial mechanics, parent final-input
   requirements and whole-plan completeness gates remain. Audit:
   `runs/owned-command-cooldown-native-audit-01.md`. Existing source04 and Sniper
   actor-action receipts cover these facts; no new runtime model is proposed.

   The ordinary Minion-level Amulet copy consumer now has a proved local
   contract, including finite arithmetic, exact eligibility and snapshot staging.
   Its owner remains Partial with five separate admission/routing/producer gaps.
   Source02 now extends the historical snapshot census to all five originals
   and actual Ritualist controls. The native reducer is published and joins
   item arithmetic to source preparation in the finite component; actual
   contributor and selected-owner coverage still gate real-build authority.
   The current compiler uses a whole-selected-plan completeness flag for scalar
   Stat reads, contribution reductions and modifier transforms. Any selected
   owner/topology gap makes these reads `IncompleteContributors`; completing the
   Amulet contributors alone cannot unlock the real Sniper request. Audit and
   close actual selected owner behavior alongside input work. Do not remove a
   Partial label merely because one of its programs now exists.
   Audit the selected Sniper request first: its two Minion item occurrences are
   already canonical. Neither absence of explicit Mystic Attunement nor an
   observed zero proves all implicit, item, configuration or earlier-copy paths
   absent. Earlier Kalandra/Quiver copies can affect the general snapshot. The
   real 50% source control is one physical rings-and-amulets modifier line
   emitting four stat effects (Ring1/2/3 and Amulet). Existing source-line
   admission can represent that without a new parser or four physical records.
   Implement its full owned meaning and source/numeric admission before claiming
   a complete factor item. Keep its own copy out of the pre-Amulet aggregate.
   Generic non-Amulet slot copies run later in `CalcPerform.lua:1490–1539`;
   its dynamically named query includes Ring bonuses. That later phase groups
   numeric BASE/INC records before scaling and is not the Amulet per-record
   algorithm. It must not be included in the earlier snapshot census.
   The follow-up source audit separates three routing questions for Modifier
   `30ca`. Focus's `CalcSetup.lua:1473–1484` branch retains the original LIST
   property but drops its scaled duplicate through `MergeMod(..., true)`;
   static inspection therefore does not justify another native level multiplier.
   The later `CalcPerform.lua:1490–1539` copies follow ordinary GemProperty
   consumption. The passing late-slot Source02 witness proves actual copy
   transport without re-preparation in its bounded controls; final ModDB
   records alone do not prove pre-support level contributions. Ring controls
   are diagnostic only because Ring templates are outside this family's
   six-template admission domain. They do not prove the Focus branch or
   establish intended game behavior for every possible slot-copy mechanic.

   **Published routing correction:** the original Amulet/Talisman exclusion now
   guards direct delivery through `3305/3306`, independently of `32e3/32e4`.
   Crown/Amulet, allocation removal, independent copies, missing facts and actual
   Partial-owner controls pass in one native graph. Generic later granted
   Keystones have no unproved early authority. Keep the minion consumer, intended
   copy semantics and remaining routing/placement inventories separate.

   **Source routing findings:** the Focus witness now passes for the actual
   Sacred Focus and Instruments of Power control. Its original +2 property reaches
   preparation unchanged; the scaled duplicate is rejected. Preserve this bounded
   evidence rather than implementing another level multiplier. The name-only
   Kalandra discrepancy has a concrete native-admission and source-execution
   counterexample, with an accepted reference-defect disposition limited to
   synthetic title controls. It is not evidence of an obtainable corrupted unique.

   The current six-template modifier membership is not complete equipment
   admission: only Crown/Solar are in the equipment-membership policy. Four
   Helmet/Amulet placements are Complete; Sceptre/Focus placements remain Partial.
   Crown and Solar intrinsic skill-grant inventories are now Complete; this does
   not close modifier-supplied grants or other templates' Partial inventories.
   Energy Blade's weapon-data fallback
   retains these non-weapon Sceptre/Focus lists; exact base suffixes cannot match
   Iron Mass, whose branch also keeps Player delivery. The existing constructed
   skill catalogue executes original data loading and contains 1,436 Skills and
   966 Gems, with no UniqueAnimateWeapon or Energy Blade. Use that authenticated
   catalogue, but include cached ExtraSkill producers before certifying Dervish
   absence. Do not replace producer coverage with a failed text search or a
   native presentation-name rule. Full fresh item construction and actual
   other-hand placement predicates remain independent proof obligations.

   **Published minion Life components:** intrinsic base and passive Increase now use
   canonical Life `311a`; Gigantic contributes its Multiply to the same stat.
   Its grant `3307` now uses typed Flag/Any instead of numeric presence, and both
   contributions execute in the current item-driven Sniper graph. Actual producer
   membership remains Partial; individual factors do not settle final grouping.
   Historical `330a` has no live references, while Damage uses `330b`. The exact
   injected 100-row allied table and 0.55 profile scale produce base1615 at Actor
   level44, and original delivery validates the six-passive Increase44 stream.
   Native integration uses actual physical final-input assembly and population
   programs. Exact dense tables remain valid during the lossless segment study;
   hostile/replacement profiles still need separate admission and data.

   Player intrinsic Life is now published once on shared Actor `332a`, with
   applicability independent of the selected Class.
   Original05's saved Life query targets Player, so final Player attributes and
   inherent bonuses take priority over a minion-only final total. Bounded attribute
   memberships and inherent controls now pass; broader donor and nonempty-MORE
   grouping domains remain open.

   Complete Life must account for extra/total contributions, Increase, More,
   conversion, overrides, relevant branches, rounding/minimum and full ownership.
   Observed zeros alone do not establish universal absence. Solar's seven intrinsic
   declaration inventories are now Complete, while numerical programs and modifier
   inventories remain Partial. The shared local Armour/ES composition
   is now published for Crown and Leggings, before per-level/override stages.
   Prioritize actual selected-item declarations and remaining Solar/Modifier30ca
   numerical ownership next. Crown/Leggings socket layouts require the accepted
   configuration model; the nineteen defence families are absent from these
   selected items, as detailed in the latest
   checkpoint and the item-ownership retirement audit. Physical
   final-input programs exist; unresolved selected providers and incoming coverage
   still prevent a real complete request. Query-independent resource demand is
   a separate accepted migration that remains to be implemented.

   Before final damage aggregation, settle the concrete precision/grouping
   question: Source02's Gigantic plus quality-one factors yield 1.21, while
   unrounded multiplication is 1.212. Individual contributions and native
   Product arithmetic are separate evidence. The [Lua/numerical cleanup gate](legacy-retirement.md#lua-compatibility-cleanup-gate-requested-2026-10-05)
   requires a valid-domain rounding law rather than source-store ancestry or
   a tolerance that masks the difference. No new PoB-defect exception is approved.

   A targeted 2026-10-06 lookup of [PoE2DB's Gigantic entry](https://poe2db.tw/us/Gigantic)
   corroborates the two 20% More benefits and also lists increased size of 20%.
   This is supplemental, unpinned game-description evidence, not a bulk import
   or proof of numerical grouping. Keep size/geometry applicability as a separate
   breadth obligation; do not infer melee range or area from model size. The
   Life/Damage packet does not claim to implement every effect of the status.

   Reservation input `3271` remains Action-owned and unresolved. Parent Action
   discovery follows query/usage/choice/support demand; no Action StatReceiver
   target exists. Do not fabricate a query or support to reach that consumer.
   Keep the -25 contributor available; migrate or deliver it through the
   accepted resource contract below.

   The [resource-obligation proposal](owned-resource-obligations-proposal.md)
   now authorizes exact Skill-owned sustained reservation with explicit Action
   dependencies where a mechanic needs a selected part/mode/stat set. It also
   identifies the missing Skill support-recipient contract. The owner accepted
   it on October 15; implementation must not manufacture Action demand from
   metric queries. Validate contrasting reservation/cost shapes,
   migrate the existing live consumer, and prove query-independent results.
   Implement the exact Skill support endpoint first in the existing Core/Data/
   Engine receiving contract, retaining preparation, participation and provider
   checks. Then add the checked obligation census, exact payer/Action bindings
   and aggregation before migrating the live reservation data. Reuse canonical
   Actor pool Stat identities and the existing occurrence/value graph; no new
   resource catalogue, second evaluator or compatibility path is required.
   Authored Gem containers must remain distinct from their generated Skill
   recipients. See the proposal's implementation contract for concrete checks.

   PoB applies the derived Gigantic Life/Damage bonuses only in its full combat
   calculation scope. Existing buffed/unbuffed diagnostic frames can retain the
   flag without those derived records. This does not prove a game condition of
   being recently in combat, and must not become a native PoB display-mode input.
   Bind numerical comparisons to the appropriate source scope, preserving the
   diagnostic differences and deliberate native scenario semantics.
   Original05's actual MAIN and saved CALCS both use the full combat calculation
   scope. The concrete audit and source locations are preserved in
   `runs/owned-gigantic-native-contract-audit-01.md`.

   Keep the nine parser-limited defaults explicit: seven Puppet/Archon owners and
   two Djinn unlocks. Djinn child-action presence
   needs selected/unselected unlock controls; rejected Puppet/Archon text cannot
   become empty Complete native owners. At your Command and Muster should share
   a real distinct participating-type census. Jewel Socket 7960 / owned `1b48`
   still owns Socket `320e`; lack of a selected jewel does not erase that mechanic.
   A final minion Life pool also needs base-life and incoming coverage, not the
   finite 44-point producer subtotal. Complete builds remain 0/5.

   Keep ordinary `GemProperty` before support admission and actor-provided
   `SupportedGemProperty` after census. Named unkeyed supported records retain
   their value even at zero copy scale, unlike keyed level records. Spell/Nova/
   Fire item families and the actual Original04 Rune source need separate proofs.
   No unchanged original contains the plain untagged Spell-level family; do not
   add it merely to improve a controlled Ice case while a real selected blocker
   has a clear fix. Evidence: `runs/owned-item-property-import-audit-01.md`,
   `runs/owned-pre-amulet-contributor-audit-01.md` and the checked packet README.
   The newer audit corrects the older ignored scope audit's false assertion
   that no Ring bonus-copy consumer exists; the published packet remains Partial
   and its Amulet arithmetic is unaffected.
   The selected owner census and justified closure boundaries are in
   `runs/owned-minion-property-owner-closure-audit-01.md`.

   Prove complete real external/support inventories and all relevant owner
   programs before publishing receiving V3/stages V3 or claiming a working original.
   The current operations V24 Partial release does not weaken that gate. Establish real
   admission predicates and supporting mechanics for all six observed Ice support
   candidates; the finite arithmetic component's already-admitted positions are
   not that proof. Supported/final quality and broader ordinary property families
   remain explicit. Validate nonzero real item/support mutations, independent
   sources, exact requests and unchanged originals at the next checkpoint.

3. **Connect the remaining numerical consumers.** Real source-property
   integration is step 2. Existing minion-level item producers alone do not prove
   completeness of incoming properties or all final inputs.

   The three selected Command Skill Damage defaults (`0edd/139d/10e4`) are
   now published and validated through the actual native contribution programs.
   Continue from channels `3302`–`3304` and existing Command eligibility; do not
   rebuild their parser/producer or substitute unconditional damage, cooldown
   or More channels. Final damage reduction must join actual unconditional and
   conditional contributions with complete receiving/owner coverage. The finite
   Basic 0/Gas 55 component does not provide that closure. Prioritize the ordinary
   remaining routing proof above and the next measured selected-owner/input blocker;
   further source harness work must enable a concrete native consumer.

   The Djinn preparation sidecar already proves ten supports and 1,296 admission
   contexts, including Original01. Integrate it only with complete real owners,
   inputs, stages and receiving metadata. Exact Direct source correspondence
   now retires the six selected target-identity issues: Bidding II twice,
   Magnified Area I, Muster twice and Frost Nexus. This is separate from complete
   mechanics. Bidding II/III have bounded numerical programs in the release
   and a checked receiving authoring fragment. Magnified Area I/II now also have
   published Partial numerical owners; Muster and Frost Nexus numerical owners
   remain absent. Admission and finite delivery do not supply
   complete owner, contributor, stages or receiving inventories. Integrate the
   [Bidding packet](../data/owned/poe2/3887ae68/bidding-support-delivery/README.md)
   only after those actual inventories are proved. Final damage/cooldown
   metrics remain separate work. The [support catalogue/supply correction](legacy-retirement.md#support-catalogue-entries-versus-executable-supply-2026-10-05)
   is now published across 568 Known Gems, with mixed active/support and unresolved
   counterexamples retained. Bidding tests consume the corrected actual release.
   Complete potential-supply declarations and real activation remain open; do not
   copy finite fixture closures into production. Rebase the older Ice Nova finite
   source-property fixture before retiring its historical membership isolation.
   A new preparation-scoped completeness model would need
   a design discussion. Muster still needs actual parent PersistentMinionTypes
   authority. Magnified Area I's Area Increase and resource-cost factor are
   implemented together; its zero damage stat has no matching I-tier map and
   supplies no damage producer. Final consumers and full closure remain open. Frost Nexus also
   adds ground-effect type and mana cost, while its ground radius/duration need
   separate consumer/gameplay evidence. A missing PoB handler is not proof that
   the game mechanic is inert. Keep these gaps explicit rather than closing a
   support from admission alone.
   Sniper reservation needs build-driven parent Action contexts without
   hidden reference queries. Current Action contexts come from real queries,
   usage/choices and receiving declarations; merely owning a parent Action program
   does not request that context. Establish genuine build-derived demand and review
   its selection contract rather than adding fake usage or queries to make it run.
   Quality, infusion, Gigantic, physical-range damage
   and complete offence/defence follow their measured input dependencies. Finalize
   and evaluate the exact unchanged request before claiming a working build.

4. **Continue bounded retirement and Lua semantic cleanup.** The orphaned Import
   equipment/Mace closure and its nineteen exclusive tests are removed in the
   earlier retirement checkpoint; all six general formatter tests remain and all 175 Import
   library tests pass. Next audit shared skill identity and source-number
   formatting consumers before removing broader profile preparation. Migrate
   the live fixed-Elemental early representation to canonical raw data before
   deleting its owned definition path. Retain independent numerical references
   and acquisition consumers; no compatibility facades for abandoned APIs.
   Classify Lua-specific behavior by domain purpose and evidence using the debt
   inventory, with reviewed numerical changes and the five-build gates intact.

Enemy critical chance, critical bonus and attack/cast-time inputs use the existing
typed constructor-default recipe, with one effective raw value per setting.
Source `ConfigOptions.lua:1984–1986` resets their placeholders; excluded boss
selection branches can replace defaults. Do not recreate the older draft's
separate presence/value model. Explicit authored zero wins, overwritten saved
Placeholders do not supply the value, and actual source consumers remain at
`CalcDefence.lua:2268`, `2273` and `3412`. The current checkpoint records each
publication and its mutation controls. This is input work, not completed critical
effect, EHP or configuration-inventory coverage; no public model change is needed.

Enemy distance is a distinct future input: its count-style zero fallback differs
from the raw override lane. More numeric controls alone cannot close the current
configuration or usage inventories. Do not add unrelated catalog families while
an identified selected-request blocker has a clear fix.

The usage implementation reuses `UsagePolicySelection`/`UsagePolicyDraft`, shared
validators and `RuleOrigin::Usage`. Preserve this single composition/execution
path when extending Import projection. Bind each actual source row to its newly
allocated SkillUse and containing preset only for authored skill roots. Generated
preferences bind to already materialized Allocation, EquipmentUse or ItemModifier
providers and their exact declared effect paths; they do not create a replacement
SkillUse or Gem. Policies must validate typed slots, source/domain guards and
work limits before traversal. Omission must preserve prior normalization bytes;
known preferences may coexist with Pending inventories. No inventory closure
follows from persistence alone.

The earlier source audit in `runs/owned-usage-import-next-audit.md` identifies the
Original05 rows. New complete-source witnesses now cover the four skeletal
families' occurrence/action/count inputs and Ice Nova's actual constructed stat
sets, in both JIT modes. See `runs/owned-minion-occurrence-inputs-01/` and
`runs/owned-spell-stat-set-source-01/`. Minion count has a real reservation
consumer; Full DPS uses separate aggregation rules. The minions' global switches
are inert in this source domain, so do not repurpose Offering's Boolean policy
as minion activation. Ice Nova has two constructed stat sets, not three inferred
from aliases. Ice and Sniper action selection/physical disposition, plus Sniper count/reservation, have
native consumers or checked projection. Other typed consumers remain unfinished; none of this
evidence closes scenario usage by itself.

### Accepted and pending owner decisions

The owner accepted these contracts:

- **Accepted:** [skill-preset usage composition](owned-skill-usage-proposal.md), typed
  preferences on the supplying skill preset composed with exact scenario overrides.
  Composition, native execution and initial Boolean source projection are
  implemented and tested. Full inventory proofs still do not follow from them.
- **Accepted:** [preparation versus execution readiness](owned-preparation-readiness-proposal.md),
  one occurrence topology/effect graph with explicit phase dependencies. Its
  versioned declarations, compiler gates and positive public support-plan proof
  are implemented and the component, compatibility and portable checks passed.
  The bounded [stages V4 local-item extension](owned-preparation-item-facts.md)
  is implemented and passes the current real-item integration, compatibility,
  dependency, scratch/Rayon and WASM compilation checks. Complete contributor
  inventories remain a separate gate.

- **Accepted:** [Direct SkillUse inputs](owned-skill-occurrence-input-proposal.md),
  shared typed slots with explicit authored/provider authority. Complete/draft
  persistence, binding and native reads pass component tests. Reviewed manual
  Djinn raw inputs and complete intrinsic inventories now import into the V5/V17
  release, with declared Command/Actor/child topology. Final input producers,
  support admission and complete numerical coverage remain unfinished.

- **Accepted:** [source-property preparation](owned-source-property-preparation-proposal.md),
  reuse existing Skill occurrences with exact source membership and aggregation
  permissions. Bounded original-source evidence and the executable native relation
  component pass. Complete real property producers and final assembly remain open.

**Accepted 2026-10-04:** [generated-skill usage ownership](owned-generated-skill-usage-proposal.md).
The chosen model keeps intent in the skill preset and adds explicit applicability
for exact tree/item providers selected by other build axes. Proven nonselection
can leave a preference dormant; stale, unknown or partial providers remain
obligations. Versioned storage, full-content data-aware validation of all stored
resolved preferences, including overridden and dormant records, and checked
composition are implemented and tested. One bounded source-to-provider resolver
now serves raw inputs and requested-count preferences. Enabled/global/reporting
dispositions and whole usage closure remain open. The legacy preference field
retains its stricter local ownership contract.

**Owner decision 2026-10-05; determinism required:** classify the Frost cold
MAIN/CALCS discrepancy as an upstream PoB initialization bug. Exclude only its
affected comparison for this build, preserve the evidence and validate other
originals with independent fresh replays. An exclusion is not a parity pass or
permission to omit unrelated queries. Do not change the backend to the proposed
rebuilt protocol, encode source cache state in native rules, retry until matching
outputs or accept flaky parity. See the [exact source issue](owned-frost-bomb-usage-evidence.md#upstream-issue-disposition).
The [determinism contract](execution-and-interfaces.md#reproducibility-and-throughput)
separates native calculation, exact oracle replay, fresh import identity,
deterministic search, timed search and telemetry. Current Frost evidence shows
repeatable source lifecycle differences, not demonstrated same-protocol randomness.

**Accepted 2026-10-04:** separate [socket configurations](owned-socket-configurations.md)
from rolled descriptors and physical inventory copies. Versioned records,
projection, host-local receiving and migration remain unimplemented; the choice
does not certify existing socket inputs or effects.

**Accepted 2026-10-05:** [generated raw-input ownership](owned-generated-skill-inputs-proposal.md).
Use preset-owned bindings to exact generated Skill occurrences, shared typed
slots and explicit supplying-declaration permission. Preserve provider-projected
levels and unique producers within the selected preset; alternative presets can
store different values. Reuse usage applicability checks without turning raw
quality into usage or adding another graph. Concrete codecs, binding and native
input producers pass focused validation. Bounded source-bound Import
correspondence and game-data publication now cover the reviewed Djinn/Firebolt
families; other generated sources and full usage/mechanics remain open. Source preservation is distinct from game
legality and permitted optimization edits.

**Accepted and implemented 2026-10-06:** [existing Actor rule ownership](owned-existing-actor-rule-ownership-proposal.md).
Shared Player rules bind once through injected Actor applicability; no actor is
allocated and no class-specific default is inferred. The checked game packet
moves intrinsic Life out of eight Classes, preserving remaining Partial coverage.
Future non-Player targets still require ancestry, readiness and activation proof.

**Accepted and implemented 2026-10-06:** [shared Player equipment-slot reads](owned-equipment-slot-state-proposal.md).
Expose validated occupancy and the exact selected EquipmentUse's computed
Stat/Capability to existing-Player Actor programs. Reuse one bounded relation
resolver with Action routing. Empty, unresolved and ambiguous slots stay distinct;
raw item parameters and game equipment legality retain their existing owners.
Core/Data/Engine validation passes, including actual invocation authority, stage
and cycle dependencies, query independence and parallel worker reuse. Game-state
producer publication and source evidence remain the next shared-state work.

**Accepted and first real packet validated 2026-10-06:** [requested Skill participation](owned-skill-participation-proposal.md).
The consumer preserves mechanical supply for preparation and uses typed usage
to gate execution and descendants on the same graph. Physical-primary usage now
shares one multiple-policy compiler with Direct/generated occurrences; the old
usage variants and Boolean/numeric split are removed and all five inputs rebuilt.

Published Sniper policy `332b`, parameters `332c/332d` and Stat `332e` derive an
early conjunction for Skill `0012`. Count `326a/326b/326c` and physical supply
`0011` → `0016` → `0012` are unchanged. All-five input/source inverses, explicit
source controls and typed preset/scenario native integration pass. Readiness is
an authenticated authoring fragment; the release still has no evaluation bundle.

**Next participation blocker:** Djinn, Firebolt, Offering, Frost Bomb and other
skeletal skills retain Partial parameter inventories. Resolve actual missing
inputs before adding execution readiness; importing enabled flags alone is not
closure. Extend family integration without changing the common gate or native
query semantics. Keep broader generated-provider dormancy/source controls and
complete-original evaluation as separate acceptance gates.

**Accepted 2026-10-06:** [explicit generated-source field accounting](owned-generated-skill-dispositions-proposal.md).
Use the existing Import policy and a separate private proof pass, preserving
actual successful raw/count outputs and unresolved usage obligations. The owner
subsequently clarified that backward compatibility is unnecessary: prefer one
current behavior and rebuilt data over parallel V1/V2 paths. The checked cutover
keeps declarative policy identities and reports the actual proof-file hash.
Current configuration-input policies emit sidecar23; generated-input policies
without configuration accounting emit sidecar22. Reimport all five
against stored previous evidence; no old importer mode is needed. Account for every
field of an admitted source pair; partial evidence retains its fallback.

**Accepted 2026-10-06:** classify the reproduced rare-item title routing as a
[PoB presentation-name defect](legacy-retirement.md#name-based-item-routing-source-discrepancy-2026-10-06).
The owner first clarified whether this was a corrupted unique; it is a synthetic
rename of an existing rare Amulet, with no obtainable-item claim. Retain a narrow
exception for the affected comparisons, never a parity pass. Native names remain
presentation. No unchanged original, independent copy behavior, actual unique
semantics or numerical owner closure is covered by this exception.

## Delivery plan and gates

### Session follow-up register (2026-10-04/05)

This register captures the review work without creating another competing resume
plan. Details and evidence live in the linked design/debt documents; completion
requires their gates, not merely adding the task here.

Reconciled against the recent owner decisions and implementation audit on
2026-10-09. Accepted direction, implemented components, pending integration and
unresolved design contracts are distinct states. The [retirement inventory](legacy-retirement.md)
remains the single cleanup register; the historical checkpoint text in other
documents must not reopen accepted decisions or promote unvalidated work.

| Work | State and next action |
| --- | --- |
| Shared-contract generality | Initial review complete. Before the next public extension, exercise contrasting authored/generated sources, exact repeated occurrences and applicable stacking/receiver shapes. [Generality gates](build-generality-review.md) remain open. |
| Generated source-property owners | Accepted contract implemented: exact generated Skills own properties and their exact declaring provider can project final inputs. Current native contract tests and component regressions pass. Sand incoming source evidence is captured; real numerical authoring remains downstream of Original05 request admission. Ownership support does not establish complete mechanic coverage. |
| Effect-application breadth | General application model accepted and Maximum component implemented. Select a real contrasting stacking/grouping case and prove duplicate, cap, source and recipient semantics before extending its bounded policies. Do not replace the shared graph with effect-specific paths. |
| Computed source selection | [Investigation](owned-computed-source-selection-proposal.md) deferred: pinned Sniper source review found no normal imported-data supplier for the motivating bow-inheritance branch. Use the existing Fixed selector only after authenticating its actual domain. Revisit a computed Boolean policy when a demonstrated supported mechanic needs it; do not copy an orphan PoB branch or infer approval from the separate Action-query decision. |
| Enemy ownership versus Action applicability | The completed bounded supplier proof permits removal of the unused Enemy-flat ports; no empty collector or new API replaces them. Any future real conditioned writer must prove Action independence or gain reviewed delivery to exact Actions. Shared numeric Enemy reads alone cannot preserve consuming-Action eligibility. Saved Party modifiers remain an explicit input obligation. |
| Self-flat source ownership and eligibility | The nonempty supplier audit distinguishes canonical modifier objects, wrapper delivery and actual stat-map consumers; observed zero is not absence. For proved Actor-invariant families, reuse exact Actor contributions/checked queries and ordinary Action routing. FullLife needs an explicit Actor condition plus absence of Action-specific overrides; no owned production FullLife binding exists yet. Preserve source ownership, separate local/inherited numeric groups and incomplete coverage. New source-to-Action authority is deferred until a required concrete supplier demonstrates the need; support and post-stacking origins cannot impersonate Action self-sources. |
| Physical-input/reference separation | Design refinement pending: classify fields without requiring successful MAIN/CALCS selection or a live Pending usage issue to preserve known intrinsic facts. Preserve independent unknown-field/usage obligations. |
| Generated selector/supply separation | [Proposal](owned-source-action-root-separation-proposal.md) pending. Archived item-only Warrior exposes physical/manual root coupling in the source-selector adapter. Separate selector interpretation from checked supply, reuse existing child topology, and preserve all unresolved owners. The narrow eight archived Warrior correspondences are published, reducing configuration source links from 44 to 36; this does not implement the broader separation or confer native authority. |
| Immutable normalization preparation | Compile existing checked policies once per exact release; separate construction/traversal budgets and prove identical outputs. Implementation pending; no storage-layer change. |
| Scoped numerical coverage | [Proposal](owned-coverage.md) pending concrete reviewed proof contract. Unknown effect reach remains blocking; no known-edge-only pruning or relaxation of original-build acceptance. |
| Closest complete build and native optimization | Original05 is the current first-completion target. Close its four remaining input obligations, publish its real evaluation bundle and complete its selected mechanics/metrics through the public CLI. D4 begins after this first complete build, without waiting for all five: bind objectives to owned actor/action/stat-set requests and verify one legal locked candidate mutation before expanding search. Reassess the shortest complete path at every checkpoint. |
| Saved allocations and effective connectivity | The retained Sniper removal95 control keeps8737 in the saved tree, while PoB prunes8737 from its active set. Current Import preserves that occurrence with Pending allocation access; it does not silently repair the build. Preserve this refusal and test explicit candidate repairs separately. Before admitting passive-tree optimizer mutations, validate connectivity against injected roots, pools, scopes and exceptional access after each mutation; import-time path proof and legacy candidate checks are not a native post-mutation legality gate. |
| Independent breadth and CI | Add provenance-bound holdouts by mechanic intersections and an explicit reproducible integration subset. Existing five examples are development cases. Ordinary CI now executes the current accuracy rule laws, but the joined Sniper graph and its import/membership/staging checks are ignored without their published package. Provision the exact current package and retained source controls reproducibly in CI, run that joined subset explicitly, and fail on missing artifacts; do not treat component tests as an equivalent gate. |
| Historical replay retirement | Reimport with the one current implementation and retain useful stored source evidence. Seventeen older ignored publication harnesses need consumer review: rebase useful assertions to current semantics or remove obsolete scaffolding, rather than add compatibility paths. Declaration integrity and source parity remain required. [Retirement/migration inventory](legacy-retirement.md#historical-publication-replay-migration-2026-10-05). |
| Action-routing format retirement | Inventory the remaining V1/V2 readers and publication/test consumers, migrate useful callers to one current format with explicit selector coverage, then remove obsolete branches. Do not convert unknown selector coverage to Complete-empty. Preserve source witnesses and all-five/routing regressions; this cleanup follows the selected flat-damage work. [Retirement scope](legacy-retirement.md#other-current-compatibility-investigations). |
| Generated usage | Versioned preferences, all-record data-aware proofs and explicit applicability diagnostics pass focused validation. Shared exact-provider proof now serves raw quality and requested-count import independently. Same-definition allocations in different presets remain distinct. Enabled/global/reporting dispositions and usage inventory closure remain open. [Contract and gates](owned-generated-skill-usage-proposal.md). |
| Requested participation | [Common gate](owned-skill-participation-proposal.md), unified typed Import and first Sniper packet are validated. All-five/source/native checks preserve count, effect controls and mechanical availability. Next close genuine parameter inventories for remaining skills before readiness publication; no PoB preview bypass or default is imported. Full-original coverage remains open. |
| Resource ownership and demand | [Accepted October 15](owned-resource-obligations-proposal.md): sustained reservation is independent of queries, with exact Skill/payer/resource identity and explicit Action dependencies where required. Define Skill support recipients and contrast cost conversion, mines and stance/toggle selection; then migrate the current Action reservation bodies and validate query invariance. Public contract and migration are not yet implemented. Gigantic's producer/status and isolated Life/Damage-factor packets are published; final reservation delivery remains open. |
| Generated raw inputs | Accepted exact preset bindings now have V6 permissions, V19 producers, source-bound quality joins and checked publication for the reviewed Djinn/Firebolt families. Existing provider levels survive. Archived cross-axis correspondence, other generated families and final mechanics remain open. [Contract and gates](owned-generated-skill-inputs-proposal.md). |
| Generated source-format breadth | Replace finite Item-name frames with proved general derivation over admitted layouts, and add evidence for normalized-only level correspondence. Census competing saved representations before scalar admission. Original04's Firebolt provider still has unresolved rune/layout/grant-line authority; raw quality cannot bypass it. [Bounded adapter follow-up](legacy-retirement.md#bounded-generated-source-import-coverage-2026-10-05). |
| Typed Boolean contributions | Accepted 2026-10-07: add typed Boolean contributions and unordered Any to the existing occurrence/query graph. Separate complete membership from numeric ordering, preserve duplicate source identity and unknown propagation, and migrate actual inherent-attribute flag producers/consumers first. This is also a prerequisite for later critical-hit flags; Enemy recipient authority must be explicit. Core/Data/Engine schema3/operations22 are implemented; real halving/doubling passives and five reducers are published. Their five bounded membership groups are now Complete; global query coverage, donor owners and unrelated passive mechanics remain Partial. No false default or numeric stand-in. [Accepted contract](owned-boolean-contributions-proposal.md). |
| Actor/reward contribution queries | Accepted October 8; Core/Data/Engine membership and canonical Life query data use operations V23. Eight exact writers populate seven groups; six zero/one-effect domains are bounded by semantic tie rejection. Bounded equipment ordering, combined Player BASE accumulation and the guarded final Life consumer are published; magnitude/routing, conversion arithmetic and global/owner coverage remain open. [Contract](owned-actor-reward-contribution-queries-proposal.md). |
| Composed support discovery | Accepted October 8. The common native gate, current authored-order DTO and source-input accounting are implemented; Original05 `01de` is retired. The Gem packet publishes 937 Known and 29 Unmapped domains with all-five preservation; all 23 selected physical Gem domains are covered. Existing typed identities fixed 111 false stat-set blockers. Pinned generated export metadata classifies eight missing runtime effects as active without implementing their Command mechanics. Continue other owner domains and exact provider exclusions; keep Command action/usage gaps separate. Unsupported positive and archived source relationships remain blocking. [Contract](owned-support-origin-composition-proposal.md). |
| Exact Skill contribution queries | Accepted October 8; Core/Data/Engine V24 implements Current Skill reads and Skill-owned self-contributions with explicit direct-use and/or supplied-slot permission. Exact supply relations survive late support checks. Canonical Offering source writers use four checked empty domains and their activation/application join passes. Inherited Actor and support-delivery memberships remain pending; nonempty game-group meaning requires proof. Actor recipient writers also use guarded empty queries. [Accepted contract](owned-skill-contribution-queries-proposal.md). |
| Application-group contribution queries | Accepted and implemented October 8 in Core/Data/Engine V25. Typed program-effect/application-group addresses preserve exact recipients, potential-source census, stacking, unknowns and explicit numeric order in one checked reduction path. Current artifacts are rebuilt with no old-format compatibility. Mixed inherited/applied damage data now adopts this capability; MORE and final hit consumers remain next. [Accepted contract](owned-application-group-contribution-queries-proposal.md). |
| Final incoming-critical ownership | Accepted and implemented October 14 through existing stat-owned Player receivers. Three final formulas reuse configured inputs and preserve exact Enemy reads, own-Stat writes and shared Actor Partial coverage. All 52 source vectors, native/CLI checks and all-five publication pass. Six numeric producers, four Partial supplier queries and complete-build/EHP binding remain open. [Decision](owned-actor-enemy-reads-proposal.md). |
| Configuration/source dispositions | Presentation, exact item-range ownership and common checked leaf accounting are implemented; full semantic closure is not. Original05 issue `01f2` now retains 30 source origins after damage-roll accounting, the default-input publications, distance projections and the Player Withered input; older origin counts are historical. Preset usage `0505` has 23 known preferences, assumptions `0207` has 23 known members, and scenario usage `0208` remains empty/Pending. Known members do not complete an inventory. The owner approved full checked disposition authority on October 14; complete catalogue/default and outside-origin accounting remain pending. Partial scalar success or local Config proof cannot clear global fallback. Typed defaults reach Player/Enemy/Environment inputs and the real Player resistance penalty is published. Keep scenario usage/assumptions separate and retire redundant configuration variants through the [consolidation follow-up](legacy-retirement.md#configuration-projection-consolidation-2026-10-07). [Evidence and gates](owned-configuration-dispositions-proposal.md). |
| Global-switch source reachability | Declared-stat and two actual-parser negative controls pass cold/lifecycle/repeat and JIT checks with exact generated/manual occurrence identity. Independent review and execution disproved the suggested missing-name alias. Full `ExtraSkillStat` producer/filter/transform reach remains unproved; no field non-applicability certificate or inventory closure is published. |
| Reward contributor coverage | Three additional selected producer inventories and four typed Actor channels are published and tested, including both Charm effects. Receiving and final threshold/recovery/capacity formulas remain open. Continue actual selected owner/contributor gaps; no whole-build closure follows from these producer programs. |
| Selected passive receiving | Growing Swarm's complete default two-effect owner and Sniper receiving are published through existing contracts. Conditional/unconditional cooldown sums stay distinct; final duration, area/radius, other minion receivers and complete contributor coverage remain open. [Packet and limits](../data/owned/poe2/3887ae68/growing-swarm/README.md). |
| Selected support delivery | Magnified Area I/II contributions and actual per-Action Area eligibility are published and validated for fourteen contexts, with scratch/Rayon isolation. Rapid Casting I/II now publishes cast-speed contributions and passes both-stat-set/independent-root checks using the same contracts. Encroaching now delivers factor 1.1 on existing channel 32fa with exact mixed-family source/native checks; ground growth and source subtotal-rounding authority remain unresolved. Prolonged I/II now supplies duration/cost factors on real Offering and Ice actions; Offering final-input production is published; complete contributor coverage, other final-input domains, final timing and application-lifetime transfer remain open. Primary Summon delivery, final radius/resource costs and complete owner/receiving inventories remain open. Keep ordinary resource-cost factors separate from reservation factors. [Delivery](../data/owned/poe2/3887ae68/magnified-area-support-delivery/README.md) and [eligibility](../data/owned/poe2/3887ae68/action-area-eligibility/README.md). |
| Support catalogue/supply boundary | Corrected one offline producer and its validators; published 568 Known Gem replacements from a full 966-Gem census. Primary associations stay in Import, non-support candidates and Partial closures survive. Bidding consumes actual corrected data; the historical Ice Nova component retains named finite isolation until rebased. Hidden-helper semantics and complete genuine supply remain open. [Retirement/integration gate](legacy-retirement.md#support-catalogue-entries-versus-executable-supply-2026-10-05). |
| Exact Direct support targets | Shared V2 source proof resolves 40 saved targets/eleven order sequences, retiring eight selected Original01 and six selected Original05 issues. Absent source is required; explicit-empty raw records stay Pending. Input values, usage, activation, full origin inventory and numerical readiness remain independent. [Evidence](owned-djinn-provider-evidence.md#exact-manual-support-targets). |
| Socket configurations | Separate descriptors/configurations/copies accepted. Implement identity, persistence, per-host projection, migration and exact local receiving; independently prove host-local aggregation before rounding, saved-effect reconciliation exactly once, physical-copy/current-setup feasibility and locks. Unused item setups must survive. [Contract and gates](owned-socket-configurations.md). |
| Determinism/source exception | Strict per-candidate determinism required. Frost initialization behavior is an upstream bug with a narrow excluded comparison. All five unchanged originals pass independent fresh replay across both JIT modes; no warm/retry canonical protocol adopted. |
| Legacy and resistance cleanup | Orphaned Import equipment/Mace closure removed; 175 Import library tests and strict Clippy pass. Fixed Elemental still needs canonical raw migration, and Engine profile adapters/default legacy dependencies remain. Follow the [retirement inventory](legacy-retirement.md). |
| Lua behavior/type audit | Requested and planned: classify source-language behavior by game/format/reference purpose and valid-input impact, then isolate or remove it. Preserve justified numerical laws; discuss uncertain semantic changes. |
| Attribute grouping and source caches | Six ordinary attribute receivers and guarded complete-empty MORE producers are published. Every potential Multiply effect rejects that bounded domain before activation. Six INC and six bounded Class/Passive BASE memberships are Complete; unlisted BASE donors reject before activation. The global query inventory and Class/Actor owners remain Partial. The BASE audit proves a signed-scaling counterexample, real post-passive bonus copies and missing socket-origin ordering; supporting those item/copy donors still needs these domain gaps resolved. Typed Boolean flag aggregation is implemented separately and does not close final attributes. Nonempty MORE grouping still needs a reviewed domain law: the synthetic 1.02/1.0201 setup discrepancy needs legal-input reachability. Earlier cache-mode options are historical; no native cache-dependent mode or extension of the Frost exception is approved. [Evidence](owned-attribute-setup-evidence.md). |
| Reference reporting order | PoB `Build.lua:2393` mutates the calculated `SkillDPS` array into display order. Record exact row identity across that reorder in the optional witness; preserve calculation contribution identity and keep presentation sorting outside native numerical semantics. [Reporting evidence](owned-full-dps-aggregation-proposal.md). |
| CI regression, timeout and infrastructure | The complete native CLI target partitioning is implemented; retained source-oracle costs still warrant profiling without dropping coverage. Current repair evidence is in the [CI receipt checkpoint](#ci-receipt-repair-2026-10-15). Compiler-domain changes must regenerate both maintained manifest families and dependent receipts. Local passes and hosted completion remain separate; pending main revisions can coalesce while an older run finishes. |
| Source validation diagnostics | Accumulate bounded per-original failures and run both JIT modes before failing overall; the successful current run skipped no comparisons. Investigate the optional Windows PoB test's `LNK4098` CRT link warning as reference-runtime packaging debt. |
| Authoring/execution simplification | Retain [A1-A4](rule-execution-model-investigation.md): measure maintenance/update cost and real evaluation workloads before choosing compact DSL, declarative, injected-native or compiled execution changes. Work inside the accepted owned boundary; no revival of Lua/UI emulation. |
| Tables, curves and segmented storage | [S1–S4 investigation](owned-scaling-data-investigation.md) added at the owner's request. Initial scan covers 16 tables/835 cells: Offering has two exact affine segments, while several damage tables do not compact usefully that way. Distinguish resolved samples, actual formulas and item-tier thresholds. Prototype lossless encodings in Rust, benchmark against compact arrays/compression, review any format change, then migrate with exhaustive domain and native/WASM parity. No runtime or artifact change yet. |

The authoritative architecture phases are [D0-D6](architecture-migration.md#phases-and-exit-gates).
Do not reuse the older archived D1-D5 profile milestones as current instructions.

| Phase | Current state and next gate |
| --- | --- |
| D0: boundaries and retirement | Owned architecture accepted; delete obsolete paths continuously, retaining named useful references |
| D1: semantic input | Owned model/import exists; unresolved original-build input contracts remain |
| D2: offline data/compiler | Owned packages and typed programs work; mechanic coverage/conversion is incomplete |
| D3: native evaluation | Component execution works; closest complete original first (currently Original05), then remaining originals |
| D4: search integration | Start owned integration after the first complete original; verify bounded mutations, then joint search across all six dimensions |
| D5: retirement and breadth | Active alongside D1-D4; finish legacy isolation and independent whole-build holdouts |
| D6: performance and applications | Measure real native workloads, then shared reports/events, UI and web delivery |

Stable product milestones remain: M0 bootstrap complete; M1 evaluator/fixtures
in progress; M2 generic search contracts partly implemented; M3 first usable
joint optimizer unfinished; M4 broader catalogs/upgrades and M5 richer objective
policies follow. Retired Spark/Mace profile benchmarks do not satisfy current
native performance or optimizer gates.

The first usable optimizer must jointly search classes, ascendancies, passives,
equipment, support gems and supporting skills within explicit finite catalogs.
Preserve 1..N required skills and exact item locks, configurable objectives and
constraints, bossing/mapping scenarios and configurable 5-30 minute thorough
runs. Test coordinated changes, tiny exhaustive domains, deterministic parallel
execution, cancellation, recovery, deduplication and verified exports. CLI/core
contracts come before the GUI; Tauri remains a candidate. PoE1 is a later distinct
versioned adapter.

M4 must rank conditional upgrades and bundles with explicit inventory, cost and
baseline semantics. M5 adds unit-checked derived/composite/ordered objectives,
soft preferences, Pareto selection and explicit robust aggregation while
preserving hard constraints. The GUI must reproduce CLI results through the
same libraries, support responsive cancellation and package the native evaluator
reproducibly; optional reference tooling stays separately installable.

### M1 checklist: native calculation and optional reference parity

- [x] Pin original PoB, preserve caller sources and retain optional reference hosting.
- [ ] First complete unchanged original through the public CLI (currently Original05):
  request admission, real evaluation bundle, complete selected mechanics, fixed
  metrics/availability and independent stable reference parity. This unlocks D4.
- [ ] Verify the first legal locked mutation through the same owned evaluator,
  with fresh/reused/Rayon parity and export/reimport; expand optimizer integration
  alongside the remaining M1 work, rather than waiting for all five originals.
- [ ] Complete the owned request/scenario boundary across all five originals.
- [ ] Produce final metrics with explicit units, availability and contributor coverage.
- [ ] Match complete native originals and independent component references.
- [ ] Match legal mutations and interacting changes in every search dimension,
  including export/reimport and multiple exact locks.
- [ ] Prove fresh/reused/parallel isolation, bounded failures and cancellation.
- [ ] Measure real native preparation, candidate evaluation and memory separately
  from optional reference-worker costs.
- [ ] Complete versioned data conversion, update/reconciliation and broader mechanics.

### M2 and M3 acceptance details

Objectives are generic registered metrics with explicit units and coverage;
unsupported policies, contradictory constraints and nonfinite thresholds reject.
Richer policy implementations must not require redesigning candidate search.
M3 still requires all of the following through the owned evaluator:

- One joint run across all six dimensions, with legality/repair and every requested lock.
- Complete-candidate verification independent of optimization caches, with reserved
  verification budget and exact realized export/reimport comparisons.
- Shared CPU/memory limits, bounded queues, in-flight deduplication, cancellation,
  bounded retries and consistent attempt accounting.
- Preflight assumptions/coverage, baseline comparisons, grouped changes, constraint
  shortfalls, distinct alternatives, JSON artifacts and offline HTML reports.
- Atomic recovery with actual verification state; recovered seeds are freshly
  checked. A warm start gets a new budget; exact search-state continuation is separate.
- Quality against random, greedy and alternating-domain baselines across several
  seeds and equal budgets. Measure scaling and 5/15/30-minute end-to-end quality
  on recorded hardware for bossing and documented mapping proxies.

Saved-result reranking is analysis of recorded values, not a new search. Legal
infeasible exploration remains distinct from verified feasible incumbents.
The M2 synthetic suite must include an exhaustively checkable domain where every
single-change improvement path stalls but a coordinated change wins.

## Fixture ledger and technical unknowns

The [fixture ledger](../tests/fixtures/builds/README.md) preserves source identities;
the [five-build inventory](breadth-validation.md) preserves the wider working set.
Original and independently hosted numerical fixtures remain useful after profile
code retirement. Preserve selected minion/action identity, Full DPS membership,
count/usage assumptions, grants, separate point pools and nonfinite availability.
Do not silently repair unknown skills or interpret labels/tree versions as verified
game-patch identity. Ask for product direction when evidence leaves a real choice;
source-internal IDs and mechanics audits are implementation work.

## Breadth of validation and data-driven build admission — next phase

[Breadth validation](breadth-validation.md) and the
[mechanism inventory](breadth-mechanism-inventory.md) define the corpus.
All production inputs are caller supplied and data driven; fixture names must
never select calculation behavior.

- B1 caller-configured intake and provenance: implemented; preserve exact originals.
- B2 coverage inventory: partial; report mechanism gaps separately from first errors.
- B3 general input seam: owned model accepted/implemented in part; finish real inputs.
- B4 independent corpus: add held-out attacks, spells, minions, DoT, conversion,
  triggers, recovery/reservation and supporting-skill interactions with explicit scenarios.
- B5 identity/legality/interactions: preserve export/reimport, view changes, multiple
  locks and separate ordinary/ascendancy roots and point pools. Source-missing nodes
  are distinct from disconnected ascendancy trees and exceptional allocations.
- B6 usefulness/performance: measure native preparation, parallel evaluation and
  complete search on admitted real builds. Unsupported cases remain visible.

The five development originals are not holdouts. Never simplify them to manufacture
parity, regenerate expected outputs from native results or count unknown outcomes
as matching numbers. Store perturbations separately.

## Execution-model investigation: A1-A4

The [investigation brief](rule-execution-model-investigation.md) remains a follow-up
to the owner's concern about loader/parser/interpreter cost. The later owned-domain
decision controls current work; this investigation does not reopen PoB UI emulation.
A1 inventories current consumers and maintenance/runtime costs; A2 compares bounded
DSL, declarative, injected-native, interpreter and offline-lowering approaches;
A3 discusses measured results and a proposed decision with the owner; A4 implements
the agreed simplification and removes superseded paths. Compare the same real
mechanics, cold preparation, changing candidates, steady evaluation and update effort.
Keep the native/WASM path and independent parity. Unneeded experiments can be
deleted now; total retirement does not require completing every prototype.

## Definition storage and UI search

Storage decision complete: generated owned packages plus loaded Rust indexes;
no database/ORM milestone. The [storage assessment](definition-storage.md) is
historical context. At the discovery/UI phase, add a bounded query API over the
selected snapshot with stable IDs, labels, filters, coverage and a derived
prefix/substring index. Measure latency/memory before adding specialized search.

The owner requested a [table/curve/segment investigation](owned-scaling-data-investigation.md)
on 2026-10-05. Its initial pinned-source and sixteen-table audit is complete;
the broader S1 census, S2 Rust prototype/benchmarks, S3 reviewed decision and S4
migration remain open. Separate semantic functions from file encoding and hot-path
layout: lossless segments may compile once to shared dense arrays. Fitted curves
must not replace exact values or create interpolation/extrapolation authority.
Storage optimization remains D6 work unless a real D3 mechanic requires a curve.

## Seeded jewel opportunity: J1-J5 follow-up

The [design opportunity](seeded-jewel-search.md) and
[feasibility study](seeded-jewel-feasibility.md) remain opt-in future work.
J1 must establish game/version applicability, compatible seed data, provenance
and costs; the pinned PoE2 seed branches and available PoE1 data are not assumed
interchangeable. J2 adds an injected provider with radius/overlap/removal parity;
J3 adds bounded parallel seed/socket/path search with explicit budgets and caches;
J4 adds reproducible item/tree reporting and trade handoff; J5 validates independent
families and compares search quality/cost under equal budgets. These gates do not
displace the five-build MVP. No native seed provider/search is implemented.

## Tooling and checkpoint workflow

New maintained tools and tests use Rust. Existing Python exporter/intake tests
remain useful; the owner explicitly deferred converting them. Their eventual
coherent replacement is [T1](architecture-migration.md#t1-rust-tooling-and-test-consolidation).

At session start, read this resume point and inspect Git state. At each coherent
checkpoint:

1. Name the closest complete-build target and its next blocking dependency. State
   how the proposed task advances that complete request/result; counts of new
   definitions, component tests or retired code do not establish completion.
   Reassess the shortest complete path and explain any target change. After the
   first build passes, begin D4 while selecting the next closest incomplete build.
2. Rerun unchanged saved selections for all five originals; evaluate when admitted.
3. Record exact data/source identities, commands, results and retained failures.
   Distinguish component evidence from whole-build and hosted-CI results.
   Format evidence-producing Rust files before capture, including shared parent
   modules. A later formatting change still changes a pinned file's bytes;
   reproduce the witness or preserve its exact historical source before rebinding.
   After changing a current bundle or transition, regenerate its dependent
   provenance receipts and run their strict reproduction checks, even when the
   dependent numerical artifact is unchanged. A rule-admission change also needs
   the downstream test suites that construct those rules; Clippy alone cannot
   verify which boundary rejects a malformed fixture.
   Compiler-domain changes must rebuild both maintained manifest families
   (`current` and `import/compiled`) and their transitive receipts. Run the
   existing Import `owned_catalog_recipe` target and allocation-export checks;
   do not update only the latest canonical release's identity.
4. Update this compact current state and next action. Append detailed completed
   checkpoints to history; do not accumulate competing current plans here.
   Reconcile new discussion findings with the session register and the existing
   design/debt entry. Every unfinished finding needs a next action, acceptance
   evidence and any actual decision dependency; an accepted recommendation is
   not an implemented feature. Replace stale "awaiting owner" wording when a
   decision arrives, keeping the historical context explicitly historical.
5. Update design only for changed architecture/contracts, discuss significant
   direction changes, review the diff and publish a coherent tested checkpoint.

Keep Cargo invocations serialized in the shared workspace; independent tests
within a target can use bounded parallelism unless they share mutable fixtures.
For heavy legacy numerical oracles, use CI's optimized test profile while retaining
debug assertions and overflow checks; snapshot reconstruction is very slow in an
unoptimized test build. Format per package on Windows to avoid command-length
limits. Do not edit
`crates/poe-optimizer-engine/src/owned_allocations.rs` or its tests during this work.
The owner authorizes commits/pushes to main and CI-log inspection.

Deferred product decisions: distribution license before a public release,
frontend/packaging before GUI delivery, and acquisition sources before live
trade ingestion. They do not block current native build support or cleanup.
