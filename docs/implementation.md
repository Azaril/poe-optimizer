# Implementation plan and resume point

Updated: 2026-10-08 (EDT).

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

## Accepted decisions and next implementation boundary

On October 8 the owner approved all three recently prompted decisions:

- [Actor/reward contributions](owned-actor-reward-contribution-queries-proposal.md):
  extend the existing checked graph to exact shared Actors, supplied Actors and
  selected rewards, preserving recipients, multiplicity, numeric order and
  incomplete-coverage failures. This is the next Player Life dependency.
- [Exact Skill queries](owned-skill-contribution-queries-proposal.md): extend
  query reads and member authority using the same occurrence graph. Repeated
  authored/generated skills remain distinct. This enables Offering scaling.
- [Support composition](owned-support-origin-composition-proposal.md): presets
  retain authored assignments/order; cold native planning derives complete
  support origins from each composed request and injected capabilities. Install
  mandatory direct/staged request coverage before moving Import obligation
  `01de`; unsupported origin families remain blocking.

Actor/reward membership is implemented in Core/Data/Engine operations V23 and
adopted by canonical Life query data. Equipment reduction coverage and the final
Player Life consumer remain pending. Exact Skill reads and self-contribution
membership are implemented in operations V24. Offering's three source writers
adopt that contract for checked empty domains; the joined graph now executes
its activation and non-stacking application with those inputs. Inherited/support
delivery authority and the final damage consumer remain pending. Composed support discovery
now has a **common native coverage gate**. Authored-input accounting is implemented
for the admitted source relationships and closes Original05's `01de` obligation.
Gem-definition source domains now have their first reviewed data publication;
other provider domains and exact exclusions remain pending.
These decisions supersede older checkpoint statements below describing them as
awaiting approval. The latest runtime checkpoint below specifies the immediate
data-adoption step for the same closest build. Life and Skill scaling reuse the
same membership implementation; support-domain data adoption proceeds independently
of the now-accepted application-group query contract. Preserve the general model and incomplete
coverage throughout.
Actor-to-Enemy reads, resource demand, configuration/reporting and scoped
coverage proposals have not been approved by
these answers. Ask when one becomes the next necessary design boundary.

## Current runtime checkpoint: checked application-group producers

The accepted [application-group query contract](owned-application-group-contribution-queries-proposal.md)
is implemented in Core/Data/Engine operations **V25**. Members of the existing
query now name either an ordinary program effect or a result after application
stacking. The Data layer checks the complete potential application/effect census,
channel, recipient capability and numeric order. The shared initial/late-support
binding gate uses the group's exact recipient, family and modifier. Execution
retains the existing reductions and worker scratch; no second aggregation path,
Lua state, fabricated definition owner or game-specific opcode was added.

**105 focused native tests pass**, including eight new generic query tests in
the 22-test application suite. These cover mixed ordinary/application values,
independent families, tied sources, repeated recipients, missing/extra/duplicate
membership, unread/inactive/zero writers, unknowns, Partial coverage, ordering,
work bounds, stages, cycles, storage permutations and fresh/reused/four-worker
execution. Evidence: `runs/application-groups-native-tests-06.log`. These finite
controls do not prove all positive combinations of late support delivery.

The [producer-format packet](../data/owned/poe2/3887ae68/application-group-producers/README.md)
records the rebuild of **2,017 canonical ordinary members**, seven maintained
member-bearing data files, one dependent reference pin and five authoring
commitments. Numeric rules, memberships, source evidence and ordering are
unchanged. The runtime rejects the old development member format; the one-off
Rust rebuild remains offline evidence. Older operation capabilities use the
current grammar, not compatibility parsers. The canonical package still uses
V24 because it contains ordinary producers; real application-query data will
opt into V25 and plan identity domain `owned-effect-plan-v22`.

The successor package is
`runs/owned-application-group-producers-publication-02/package`, input
`802f807e1a7c1687c67f161148619e14430a871e61e90101d04f33c8e13112e2`, rules
`2247acae6be01175f30834ddabc47d4dc114177388c99964bbf53249cac4e5ab`, compiled rules
`94ef07506af0bd6d127bd38d74c9d401d273d13bfa77e4441d9d9bb70c61e999`.
Schema identity is unchanged. Exactly three of 18 artifacts change: rules,
manifest and release. The rebuilt release independently reproduces its bytes.

All-five preservation and byte-pin tests pass in **13.04s**; all **79 joined
Sniper checks pass in 38.25s** against that successor. Evidence:
`runs/application-groups-integration-03.log` and
`runs/owned-application-group-producers-five-builds-03/report.json`.
Fresh CLI imports deliberately mint new project lineages. The comparison
authenticates the draft digests, rebases only fresh lineages and their digest
references, and compares every remaining draft, sidecar and finalization field.
This is import identity behavior, not nondeterministic evaluation. All 110
queries, five query sets and input counts **107/117/109/123/4** are preserved.
Complete native builds remain **0/5**.

The seven maintained authoring/consumer targets also pass **16 ordinary tests**
(artifact-dependent historical tests remain ignored):
`runs/application-groups-authored-tests-04.log`. Strict all-feature Clippy passes
for changed Core/Data/Engine/Import libraries and the selected native/CLI targets:
`runs/application-groups-clippy-01.log`. Changed Rust formatting and whitespace
checks pass. Unrelated untracked incoming-critical drafts remain excluded.

**Resume:** keep Original05 first. Publish the real mixed damage consumer using
the accepted query capability, then continue toward its physical damage endpoint.
The Offering application already emits channel `322d`; the selected passive
component currently reaches the minion through derived stat `1d34`. Do not claim
both are already writers to one channel or manufacture an ordinary owner for the
Offering group. Reuse checked group reads and typed arithmetic, or publish an
explicitly reviewed shared contribution channel. Validate the observed 62 + 68
component subtotal without turning 130 into a rule literal or complete damage
coverage. Preserve all remaining source-domain and input obligations. Composed
support coverage and the final Player Life consumer remain separate pending work.

## Preceding native data checkpoint: shared Life routing

The [flat-Life routing packet](../data/owned/poe2/3887ae68/flat-life-routing/README.md)
adds the existing Talisman exclusion to ordinary Player Life delivery. It reuses
the EquipmentUse predicate and its allocated-Passive/Player retention chain,
with four additional non-Amulet template predicates from the authenticated
catalog. Numeric preparation, exact Life effect identity, all query memberships,
import admission and every owner gap remain unchanged. There is no new runtime
operation, source parser, definition or format branch.

The pinned routing branch handles all Amulet modifiers, independently of stat
name. Publication reuses its original-call routing evidence and the existing
type catalog rather than making another source observer. The numeric native
control adds a Life roll to the selected Amulet and uses the actual published
Talisman program in a finite passive component. Only that ordinary Life effect
becomes inactive; the five other equipment uses retain their values and identities.
Removal, reusable scratch and four-worker Rayon replay agree. A missing predicate
stays unresolved, and an early read is rejected.

**All 79 joined Sniper checks pass in 39.03s**, including the three new routing
controls. Authoring and all-five publication pass in **20.63s**, including current
exact base-to-template mappings. Logs are `runs/owned-flat-life-routing-
publication-02.log` and `runs/owned-flat-life-routing-sniper-01.log`.
Strict Clippy with all features passes for both changed CLI integration targets;
changed Rust formatting and whitespace checks pass. Evidence:
`runs/owned-flat-life-routing-clippy-01.log`. The successor package is
`runs/owned-flat-life-routing-publication-02/package`, input
`06905cc38c8ceadfe27019a843b507bb6d71b210ada9738a45aa1173884f62f9`, rules
`72bbf63447cd6d1260baea5f35cb90f2eaf61a336b1f2322ed94d191395bed5e`, compiled rules
`59c4642a7a3653ea7d4d5815e9169be8c44f8c91b817b2548b71e92813f5c061`.
Schema identity is unchanged. All 18 artifact hashes also agree with the first
publication used by the native checks. Rebuilding is byte-identical; all five imports,
110 queries and selected input counts **107/117/109/123/4** are preserved.
Complete native builds remain **0/5**.

**Resume:** keep Original05 first. Player Life still needs complete item delivery
(including independent copied records), equipment contribution/order coverage and
the final resource formula; its known component subtotal is not final Life.
This checkpoint implements one routing law and intentionally closes no broad
owner gap. Channel `3306`'s historical broad name means only the Talisman exclusion;
do not treat true as proof of every item routing law. Minion receipt is separate.
The application-group query extension is now implemented at the current
checkpoint above; the mixed passive/Offering consumer remains next. Other support-source owners and provider exclusions
remain open despite complete selected physical-Gem classifications.

## Historical acceptance checkpoint: application-group contribution queries

On October 8 the owner explicitly approved the
[application-group producer contract](owned-application-group-contribution-queries-proposal.md).
This supersedes older awaiting-review statements in checkpoint history below.
No further approval is needed for its implementation. At the acceptance-only
checkpoint, no runtime or package change had shipped: the flat-Life-routing package,
operations V24, four selected input obligations and **0/5 complete builds** are
unchanged.

**Implementation scope accepted then, completed above:** extend the existing member contract with typed ordinary
program-effect and application-group producers; validate every potential
application/effect declaration; bind concrete groups by exact recipient/family/
modifier in the common initial and late-support inventory gate. Preserve the
existing reduction execution, application stacking, explicit numeric positions,
Partial coverage and bounded work. Replace the current development representation
and regenerate maintained artifacts; add no compatibility branch or fabricated
definition owner. Verify independent families, missing/extra/duplicate members,
inactive/unread/unknown sources, repeated recipients, ordering, stages/cycles and
fresh/reused/Rayon execution before publishing the real mixed damage consumer.
The observed 62 + 68 = 130 percent subtotal is a component control only.

The intervening Life investigation made no code or data changes. Its full formula
also depends on Extra, Total, conversion, override and Chaos Inoculation inputs,
in addition to the existing BASE/INC/MORE queries. Retained rounding evidence in
`runs/owned-player-life-rounding-source-02` does not prove all broader source
domains or arithmetic bounds. Keep that work pending; do not replace the newly
unblocked damage path with another supplied-scalar Life fixture.

## Preceding native data checkpoint: Gem support-source domains

The existing offline extension/assembler now publishes **966 Gem declarations**:
937 `AuthoredAssignmentsOnly` and 29 Unmapped for actual additional support effects. The
[packet](../data/owned/poe2/3887ae68/gem-support-domains/README.md) reuses the
authenticated complete Gem catalog rather than adding another extractor. Every
source-to-owned binding is checked against the current release. No runtime code,
Lua type, compatibility branch, schema definition or formula was added.

The first packet's offline proof incorrectly treated stat-set metadata as
additional-effect references, falsely blocking 111 Gems. The corrected proof
reuses the existing typed identity catalog, checks every flattened observation
against its reference category and cross-checks actual support classifications.
Ice Nova loses this false blocker. This was our authoring error, not a PoB bug.
The current format was corrected and rebuilt from the same predecessor; no
runtime compatibility or extension-overwrite path was added.

The follow-up traced all eight missing runtime effects to the pinned generated
`SkillGemsExport.txt` inventory. Nine exact export blocks, including duplicate
Sniper rows, retain the generator's non-support branch. The generator reads each
effect's `IsSupport`, independently of its Gem category. Three loaded active/
support controls agree with the typed catalog. Publication authenticates the
template's exact Git blob and all block occurrences, plus the manifest-pinned
generator. This establishes classification without inventing the missing Skills.

All **23 selected Sniper physical Gem occurrences now have Known support-source
domains**. The eight missing runtime identities and their Command mechanics
remain unresolved; Player Commands are not aliases for existing minion-side
actions. Four bounded PoE2DB lookups corroborate that distinction but supply no
pinned numerical data. This is neither a PoB-bug parity waiver nor missing-table
emulation. Generated/manual Djinn and other providers have separate coverage
requirements. The retained proof and lookup references are in the packet README.

The predecessor package at that checkpoint was
`runs/owned-gem-support-domains-publication-03/package`, input
`f3d994ac40afb840606c84a10df7e84fb60bcc216ac5700f2cb07e9a5a955c68`, rules
`5cd23f3e3c3c70e96d131608e9f14a58e346c551b6ef2c0aac93e1a06379b128`, compiled rules
`746863d0d48dad809ad44256fbd7ff6cda4c9223fb8b4b0c0a15f1e29ec65014`.
The schema identity remains unchanged. Publication proves an exact whole-input
inverse allowing only these declarations and one provenance row, plus byte-identical
rebuilding and unchanged fresh imports of all five originals. Selected input
counts remain **107/117/109/123/4**, with 110 queries and **0/5 complete builds**.

Seven ordinary catalog/refusal checks pass, including the full stat-set census,
category/identity corruption, missing export proof, positive-support controls,
and unchanged missing runtime identities. Publication passes in **21.08s**; logs
are `runs/owned-gem-support-domains-{ordinary-04,publication-03}.log`.
The initial `ordinary-01` failure combined genuine missing effects with our
incorrect stat-set classification; it is retained as historical diagnostic evidence.
The publication's `selected-gem-domains-05.json` records the selected occurrence
census. **All 76 joined Sniper checks pass in 37.83s** against the current successor.
All seven common native discovery-gate checks passed at the preceding checkpoint,
including disabled sources,
changed providers, direct/staged entry and reused/Rayon results. Strict Clippy
passes for the new CLI integration target with all features. Evidence:
`runs/owned-gem-support-domains-{sniper-03,native-gate-01,clippy-03}.log`.

**Resume:** the selected physical Gem support-source domain is now covered.
Continue exact support-source domains for the remaining owners and provider-specific
exclusions. Known Gem domains alone cannot bypass the common native gate, close
usage/configuration inputs or establish whole numerical coverage. The separate
application-group contribution-query proposal still awaits owner input; do not
implement that public change while waiting. Prioritize the shortest evidenced
path to unchanged Original05 through the accepted support and Actor/Life seams.
Keep distinct Command input/action/usage obligations when reviewing the selected
mechanics; support classification does not remove them. Do not repeatedly reopen
the resolved Gem identity/classification census as an immediate blocker.

## Current input checkpoint: authored support relationships

Original05 now has **four selected input obligations, down from five**. Import
retires only `01de` after proving the saved physical assignment census, exact
targets and encounter order, and all saved support relationships. Manual groups
must have no unrepresented slot binding. Generated groups must match the existing
exact source resolver's item/tree correspondence; archived syntax, stale or
ambiguous providers, and unsupported sharing remain Pending. The proof runs
after generated input/field accounting and requires that the retired issue has
exactly its one owning preset link, preserving unresolved archived responsibilities.

The generated-source resolver's existing result is retained for this independent
inventory check. Unknown quality or disabled activation does not erase a known
source relationship or become a default. This adds no source parser, saved group
model, native origin variant or evaluator branch. Current physical-inventory
policies gain the authored completion check; simple order-only policies still
lack census authority. Successful completion changes the draft and sidecar
commitments without allocating, renumbering or refunding an issue ID.

All five originals were reimported with the unchanged checked package. Selected
input counts are **107/117/109/123/4**, with 22 queries each. Exact baseline
comparison against the prior authored-order checkpoint proves that only
Original05's selected `01de` completion and its one provenance link change,
apart from the explicitly bijective fresh import-lineage relabel. All assignment
values/order, other inputs, issue IDs, allocator watermarks and remaining sidecar
data are identical. Other Sniper presets and the other four originals stay
unresolved where the same proof cannot establish their source relationships.
Reports: `runs/owned-support-input-accounting-reimport-01/validation.json` and
its adjacent `migration-comparison.json`.

Validation: **317 Import normalizer checks and three breadth checks pass**, including
six new completion/refusal tests and updated exact retirement assertions
(`runs/owned-support-input-accounting-import-05.log`). All seven common native
discovery-gate checks still pass (`runs/owned-support-input-accounting-native-gate-01.log`).
Strict Clippy passes across all Import targets/features
(`runs/owned-support-input-accounting-clippy-import-01.log`). Real-build CLI and
joined Sniper verification also pass: **76 joined checks in 38.66s**, the shipped
policy check and a second fresh all-five reimport in 7.54s
(`runs/owned-support-input-accounting-cli-native-01.log`). The reimport now
explicitly asserts that Sniper's selected authored inventory is Complete and
that exactly the four remaining input codes below survive. Its independent
report is `runs/owned-support-input-accounting-reimport-02/validation.json`.
The second run's adjacent `migration-comparison.json` independently confirms the
same exact retirement and preservation. Strict Clippy also passes for both
affected CLI integration targets with all features
(`runs/owned-support-input-accounting-clippy-cli-01.log`). In total, the final
targeted suites contain **405 passing checks**; repeated reimports supplement
that count rather than increasing it.

**Resume:** native support-domain game data and exact generated-provider
exclusions are still required. A Complete authored inventory is not permission
to bypass the common cold-planning gate or infer empty extra/linked capabilities.
That input checkpoint kept its package byte-identical; the subsequent native data
checkpoint above adds Gem domains. Complete native evaluations remain **0/5**.
Continue the shortest verified path
for Original05, reconciling the remaining input owners and numerical coverage:

| Selected issue | Remaining responsibility |
| --- | --- |
| `0503` | Complete skill-preset usage preferences and saved usage fields |
| `01f2` | Configuration roles and their source accounting |
| `0207` | External assumptions, including effective defaults |
| `0208` | Scenario usage inventory |

The application-group contribution-query decision still awaits owner input;
independent support/usage work may continue under already accepted contracts.

Checkpoint reports below preserve historical evidence. Their earlier counts and
resume statements are superseded by this current input checkpoint and the live
follow-up register.

## Previous contract checkpoint: authored support order

Preset, Draft and composed Build records now use `authored_support_order` and
`AuthoredSupportOrder { target, assignments }`. The assignments are physical
`SupportAssignmentId` values in explicit order; runtime `SupportOrigin` variants
are no longer a possible saved preset payload. Core retains exact target and
membership validation, duplicate rejection, bounds, Pending candidates and
independent preset composition. Engine derives runtime origins for preparation
and delivery through the existing support index. No execution algorithm, source
discovery capability or provider exclusion was inferred from this format change.

The current wire shape rejects the old field and wrapped-origin values. Existing
test fixtures and callers use the new shape; no compatibility alias or second
build model was added. Optional order still means unknown when absent. Rules and
game-data packages contain no saved build documents and need no content rewrite
for this DTO change; source builds must be normalized afresh.

Validation: **249 Core, 120 Engine and 314 Import checks pass**, plus the new
current-package five-build CLI reimport check. Strict Clippy passes for all
library targets/features and the binary plus all 194 tracked CLI test targets.
The pre-existing untracked incoming-critical draft remains excluded and untouched.
Logs use the `runs/owned-authored-support-order-` prefix:
`core-01.log`, `engine-01.log`, `import-01.log`, `reimport-01.log`,
`clippy-libs-04.log` and `clippy-cli-04.log`.

The reimport report is `runs/owned-authored-support-order-reimport-01/validation.json`.
All five unchanged originals preserve source-derived assignment order across
all presets, retain **107/117/109/123/5** selected input obligations and each
retain 22 queries. The checked data package is byte-identical. The adjacent
`migration-comparison.json` records exact prior/current draft and sidecar
comparison after the intended DTO transformation and a bijective relabel of
the one host-assigned import lineage. Every local ID, relationship, allocator
watermark, issue and provenance row is preserved. The CLI deliberately assigns
a fresh lineage per import; this is identity allocation, not nondeterministic
evaluation. Draft commitments change, and no issue is retired.

The joined Sniper fixture initially exposed an old saved-draft dependency in
its Offering preference reader. Character and Offering inputs now share a fresh
import of the original XML and a source-derived selection, with the current
draft commitment checked. Neither reads sibling drafts beside the data package.
The first failing run and isolated error are retained as `sniper-01.log` and
`sniper-isolate-01.log` under the same prefix; no legacy decoder was added.
After this correction, **all 76 joined Sniper checks pass in 37.62s**
(`runs/owned-authored-support-order-sniper-02.log`), bringing this checkpoint's
passing checks to **760**. This covers the actual imported item inputs,
attributes, Life components, accuracy, Offering activation/non-stacking,
unresolved coverage and reused/Rayon results within the finite component graph;
it does not produce a complete original-build evaluation.
Strict Clippy also passes after the joined-fixture correction
(`runs/owned-authored-support-order-clippy-sniper-02.log`).

**Prior resume (superseded by the input checkpoint above):** finish authored-input accounting using the shared whole-source
census and exact generated-provider bindings, while preserving unresolved saved
relations and their provenance. The former mixed-scope completion issue still
exists at the new authored-order field and remains Pending. It must not be
cleared by the rename, a physical assignment count, or empty observed source
buckets. Review support-domain data and exact provider exclusions for the
closest build through the publication path below. Original05 still has five
selected input obligations; complete native builds remain **0/5**. The separate
application-group contribution-query decision remains awaiting owner input.

## Publication checkpoint: support-source authoring

`OwnedRecipeExtension.support_source_domains` now publishes reviewed support
declarations through the existing offline authoring path. It accepts new exact
definition/slot rows and identical replays, preserves missing coverage on an
empty extension, and rejects duplicate or conflicting authority. Existing
Unmapped declarations cannot silently become Known. Ordinary Data validation
and compilation enforce identities and gap shape; compiled identities include
the declarations. The receipt reports the number of added domains.

Both compact successor and complete release assembly preserve this field.
Their outer validation budgets now include provider rows and gap payloads at
both endpoints where applicable. No new publication pipeline, native source
interpreter or compatibility branch was introduced. The canonical game package
below is unchanged: this checkpoint supplies its authoring route, not reviewed
game coverage, and Original05 still has five selected input obligations.

Validation: **74 Import checks pass** across recipe extension, membership patch,
compact succession, complete release publication and prior successor behavior.
This includes nine new domain-authoring/publication/budget regressions. The
complete release round trip retains all five query sets and 110 query rows;
it is a publication fixture, not an original-build evaluation. Logs:
`runs/owned-support-domain-authoring-tests-01.log`,
`runs/owned-support-domain-publication-tests-01.log` and
`runs/owned-support-domain-preflight-tests-01.log`. Strict Clippy passes for
all Import targets and features
(`runs/owned-support-domain-clippy-import-01.log`).
The actual CLI's five publication checks pass, including a new no-overwrite,
no-output-on-conflict control (`runs/owned-support-domain-cli-tests-01.log`).
All **76 joined Sniper checks pass in 38.64s** against the unchanged canonical
package, including scratch reuse and Rayon
(`runs/owned-support-domain-sniper-01.log`).
Strict Clippy also passes for the binary and all **194 tracked CLI integration
targets**, with the same all-feature check and `-D warnings`
(`runs/owned-support-domain-clippy-cli-01.log`; target list in
`runs/owned-support-domain-tracked-cli-targets.txt`). The unrelated untracked
incoming-critical draft remains untouched. At 21:10:17 UTC, predecessor
`d822ff6`'s hosted run `37843055066` was pending; local validation is not a hosted
success claim (`runs/owned-support-domain-prior-ci-01.json`).

**Next blocker:** review actual support-source domains and exact provider
exclusions, then reconcile authored support order with generated-field
accounting. `skill_input_disposition::pending_generated_responsibilities` still
requires the mixed-scope issue as one of three responsibilities for archived
generated syntax. The strict source frame permits `slot`; source evidence shows
that this field can affect support sharing. Preserve an Import obligation for
every unconverted saved relation instead of clearing `01de` from a complete
physical assignment list or empty observed runtime buckets. Positive
ExtraSupport/LinkedSupport producers exist in pinned `ModParser.lua`; source
behavior does not by itself establish legal game admission.

The pending application-group contribution-query decision is unchanged and
independent of this approved work. Complete native builds remain **0/5**.

## Native checkpoint: composed support coverage

The current rule format carries optional `support_discovery` declarations. Each
exact owned definition/slot has either reviewed `AuthoredAssignmentsOnly` scope
or explicit Unmapped GameRules gaps; missing metadata is unknown. Storage and the
standalone compiler share validation of identity, duplicate rows, gap ownership
and bounds. The compiled rule package canonicalizes and indexes this immutable
data once, sharing it across candidate plans and workers.

The common cold planner checks provider occurrences, shared Actor applications,
selected Encounter/usage and Action owners. Disabled roots and descendants reuse
Core's structural provider resolver solely for coverage; this does not run their
programs or enable participation. Off-loadout equipment is checked when selected.
Repeated providers retain separate diagnostics, even when their programs or
authored support lists are empty. Missing/Unmapped coverage blocks preparation,
retained support delivery and final metric authority. All operation subsets use
the same gate and new semantic plan identity; no compatibility bypass was added.

Synthetic fixtures declare their own finite assignment domains. This is not a
game-data absence proof, an implementation of positive extra/linked origins, or
permission to publish Complete real owners. The canonical package remains
unchanged and intentionally lacks these declarations. Original05 still has five
selected input obligations, and the five-build MVP remains **0/5 complete**.

Validation: **76 joined Sniper checks pass in 38.02s**, **50 focused native
checks pass**, **six owner-order/bounds regressions pass**, and **8 rule-storage
checks pass**. The eleven affected CLI targets
have **29 passing checks and 25 existing opt-in checks ignored**. Evidence:
`runs/owned-support-discovery-sniper-01.log`, `focused-04.log`, `final-01.log`,
`data-01.log` and `cli-01.log` under the same `owned-support-discovery-` prefix.
The final owner-order and new-test Clippy replays are `deferred-final-01.log`
and `clippy-final-01.log` under that prefix.
The wider native run exercised 69 targets (527 passes, two failures); canonical
row ordering and inactive Actor-slot coverage caused those failures. Both are
fixed and their complete targets pass in the focused replay. The initial broad
Engine run was interrupted after finding obsolete private-fixture assumptions;
it is not a full-workspace success claim.

Strict Clippy passes for all library targets/features and the binary plus all
**194 tracked CLI integration targets**, with lists/logs in
`runs/owned-support-discovery-tracked-cli-targets.txt` and
`runs/owned-support-discovery-clippy-{libraries,cli}-01.log`. The unrestricted
all-target command also discovers the pre-existing untracked
`tests/support/owned_incoming_critical_native.rs`, whose separate draft needs the
new constructor field; it remains untouched. Windows optional reference links
retain the known `LNK4098` warning. At 20:50:54 UTC, predecessor `1b63704`'s hosted
run `37836701038` remained pending (`runs/owned-support-discovery-prior-ci-01.json`);
hosted success is unverified.

**Resume:** author reviewed
Original05 support-domain data and exact provider exclusions, reconcile the
persisted sequences as authored order, and move Import `01de` only after that
accounting is proved. Do not infer absence from the PoB census alone. The
application-group contribution-query proposal remains awaiting owner review;
do not implement its dependent damage consumer without that decision.

## Latest source checkpoint: composed support origins

The accepted support-composition work now has an actual source-construction
census. A scoped optional observer records original `CalcSetup` calls, raw
ExtraSupport/LinkedSupport buckets and returns, primary/additional Gem effects,
exact source occurrences and incoming constructor candidates. It reuses the
existing complete PoB bootstrap, JIT runner and XML control helpers; native
Core/Data/Engine and published packages are unchanged.

Original05 has twelve selected source groups and sixteen constructed effects.
The 28 primary-effect calls include sixteen supports; four additional calls are
non-support Djinn Commands. Nine ExtraSupport queries and one LinkedSupport
query have empty buckets and results in both MAIN/CALCS. Four tree-Djinn
constructors exclude supports, while manual Sand receives three candidates.
Its disabled Magnified control receives two. A source-slot control makes
item-generated Firebolt receive three supports from the distinct manual Sand
group; the unchanged original receives none. The control proves source behavior,
not legal game admission or permission to add PoB slot groups to native data.

The census also disproves an observer assumption: PoB attaches `gemData` even
to manual Djinn rows and shares `fromTree` definition metadata. Neither is native
physical-Gem or provider-origin authority. The corrected observer records those
fields literally. The first failed run remains diagnostic; a subsequent XML
control setup error was corrected without changing source methods or fixtures.

**Twelve complete cold loads pass**: baseline, independent repeat, uninstrumented
baseline, disabled support, slot sharing and uninstrumented slot sharing in
each JIT mode. Raw reports are byte-identical at 607,968 bytes, SHA-256
`e34ebf95d61c2ebbddd1300b59582149700468d67e24d6cd059008070407eba8`.
The final driver passes in **48.75s**:
`runs/owned-support-origin-acquisition-03.log`; reports are in
`runs/owned-support-origin-source-03`. Strict workspace/all-feature/all-target
Clippy and changed Rust formatting pass. The optional Windows source-test link
emits `LNK4098` (CRT library conflict); the executable succeeds. Do not describe
that link as warning-free. Clippy evidence is `runs/owned-support-origin-clippy-01.log`.
At **19:55:51 UTC**, predecessor `e8b55db` CI run `37834636120` remained pending
(`runs/owned-support-origin-prior-ci-02.json`); hosted success is unverified.

**Follow-up (common native gate now implemented above):** retain authored assignment/order on the preset and
adopt reviewed provider-capability data in the common cold planner before
support selection/source-property collection. Bind exclusions to exact supplying
occurrences, not shared Skill metadata. Complete declaration lists or the empty
source queries alone cannot establish this support inventory. Direct/staged
requests must share the gate; then move Import `01de` after its authored-source
accounting is proved. Positive extra/linked and additional-support families stay
unsupported until reviewed. Detailed evidence and integration points are in the
[accepted support contract](owned-support-origin-composition-proposal.md#source-census-and-implementation-implications-2026-10-08).

Native discovery has the common coverage gate above; game-data adoption remains
pending. The application-group
query proposal is still awaiting owner review; no approval is inferred from
elapsed time. Current package, all-five issue counts, selected Original05's five
input obligations and **0/5 complete builds** are unchanged.

## Latest integration checkpoint: Offering activation and application

The same finite Sniper graph now binds the imported selected preset's known
Offering preference through Core's current intent proof and checked project
composition APIs. Scenario controls use the existing whole-record replacement
semantics. It loads the actual UsagePolicy `3259`, parameter `325a` and
`skill-effect-activation` program, rather than supplying an activation Stat.
The real preset's enclosing usage inventory remains Pending. A second Offering
occurrence is an explicit duplicate test control, not an additional original
source or a claim that the full preset has been finalized.

The shared fixture accepts the published application declarations and schedules
them after source and recipient scaling. Item-prepared final level22 and the
checked scaling queries now produce **62% increased damage per exact Sniper
recipient**. Equal copies retain both candidate/winner identities but emit one
group result; independent raw levels select the stronger copy. Retained source
controls also agree at final level3 (24%) and level32 (80%). The application
formula, runtime graph and published data are unchanged.

Five new integration checks cover exact overrides, disabled versus absent
activation, missing scaling, lazy inactive sources, Partial application
coverage, early/missing scheduling, non-stacking and fresh/reused
A–unknown–B–A/four-worker Rayon execution. **All 76 joined checks pass**
(66 integration and ten ordinary) in **38.80s**. Strict workspace/all-feature/
all-target Clippy, changed Rust formatting and `git diff --check` pass.
Evidence: `runs/owned-offering-application-{joined-03,full-01,clippy-01}.log`.
The first focused run found fixture type/key spelling errors, corrected before
these passing runs; no production validation was relaxed.

The package remains `runs/owned-buff-sources-publication-01/package`, operations
V24, with all 110 queries and unchanged original issue counts
107/117/109/123/5. This checkpoint changes test composition only; it does not
publish an evaluation bundle, close the real application/owner inventories or
retire any of Original05's five selected input obligations. **0/5 builds complete.**
No source VM was rerun; comparison reuses authenticated independent source
reports. Ordinary standalone application tests remain useful automatic coverage
until CI provisions the joined release.

At **19:45:43 UTC**, predecessor `46c11ce` CI run `37831867065` remained pending;
`runs/owned-offering-application-prior-ci-02.json` records the snapshot. Local
validation is not a hosted CI success claim.

**Next numerical boundary:** checked queries currently name definition program
effects, while application stacking emits a distinct group contribution. The
[application-group query proposal](owned-application-group-contribution-queries-proposal.md)
recommends typed producer addresses in the existing membership model. It is
**awaiting owner review**, not implemented or implied by the earlier Actor/Skill
approval. Do not impersonate the group with a fabricated definition program,
sum its candidates, or supply the expected 62 + 68 subtotal. After approval,
bind the group and selected passive contributions through reviewed data and
continue to the physical damage endpoint.

While that public-contract choice is reviewed, implement the already accepted
composed-support discovery and mandatory direct/staged request coverage gate.
The equipment Life reduction proof and final Player Life consumer remain
independent numerical work; their broader native bounds cannot be silently
narrowed to the current importer's values.

## Previous data checkpoint: exact Offering source scaling

The [source-scaling packet](../data/owned/poe2/3887ae68/buff-effect-sources/README.md)
adds three canonical Skill writers (`3228`–`322a`) and four checked incoming
queries. Buff increase and MORE read their exact occurrence's query; magnitude
combines `(1 + increased / 100) * more` with existing typed operations. The one
literal is the formula's dimensionless identity, not a supplied resolved factor.
No new definition, receiver, runtime formula, interpreter or source-specific
Engine behavior is introduced. Current offline migration V5 now admits V24;
there is no new migration format or runtime compatibility branch.

All four groups have an explicitly empty supported domain. The ordinary census
rejects any matching potential contributor before guards or values, including
neutral and unread sources. Unknown/Partial inventory is unavailable, not zero.
The actual Skill owner and global query registry retain their Partial closures.
Inherited and support origins, nonempty game source admission and complete
support discovery are not established by this packet.

Evidence reuses the existing 22 case/mode vectors rather than copying their
reports. They contain 23 source invocations: 17 empty and six nonempty controls.
Original05's source domains are empty. CalcTools' magnitude composition is
additionally pinned to the source manifest; Lua vararg/JIT dispatch is confined
to offline provenance. Synthetic native members exercise the nonneutral scalar
cases through exact supplied-Skill authority, without admitting those custom
sources into game data.

The current package is `runs/owned-buff-sources-publication-01/package` (V24).
Publication and identical rebuild pass in **29.59s**, preserving all five
unchanged originals, 110 query rows, and selected issue counts 107/117/109/123/5.
There is no evaluation bundle and **0/5 complete builds**. The joined graph
authenticates the source programs already retained by its physical-source loader,
adds the four unchanged query groups, and uses explicit execution stages after
the contribution freeze on two independently prepared Offering occurrences.

Validation passes: **71 joined checks** (61 integration and ten ordinary) in
**37.11s**, plus the packet's ordinary check and full publication/reimport check
in **29.26s**, and six current migration checks in **2.04s**. All 18 files from
the two independent publications are byte-identical. New cases exercise exact
copies, raw-level changes, nonneutral values through explicit synthetic members,
inactive/neutral/unknown unregistered writers, Partial inventories, missing
values, early reads and fresh/reused A–unknown–B–A/four-worker Rayon execution.

During development, source pinning initially used Windows checkout bytes rather
than the source manifest's canonical LF bytes; this was corrected without
changing the source. The fixture also attempted to duplicate programs already
loaded by the existing source loader. Two refusal expectations then needed the
actual early-readiness boundary and a Stat-owned contribution gap. All fixes
were confined to authoring/test setup; production guards were not weakened.
The full final run is `runs/owned-buff-sources-final-01.log`; migration evidence
is `owned-buff-sources-migration-01.log`, and repeat evidence is
`owned-buff-sources-independent-repeat-01.json`. Earlier failed/narrowed runs
remain diagnostics. No new source VM was run; publication authenticates retained
independent JIT-mode reports.
Strict workspace/all-feature/all-target Clippy passes after replacing a duplicate
test module declaration with a shared evidence import; see
`runs/owned-buff-sources-clippy-02.log`. The retained evidence helper is compiled
once per test target. Changed Rust formatting and `git diff --check` also pass.
Both affected targets' ordinary evidence checks pass after that import cleanup
(`runs/owned-buff-sources-shared-evidence-01.log`).
At **19:06:07 UTC**, predecessor `744cff9` CI run `37827149055` remained pending;
`runs/owned-buff-sources-prior-ci-01.json` records that snapshot. This is not a
hosted-success claim for the current checkpoint.

**Next delivery at that checkpoint (now delivered above):** join the existing typed usage-policy activation and published
Offering application to this same graph. Establish final level22 → application62
per exact recipient, duplicate non-stacking, disabled/scenario controls and
missing-input refusal. Then join the selected passive68 contribution through a
reviewed consumer and proceed to the physical damage endpoint. The old separate
application fixture remains useful parity evidence. Correction to earlier plan
wording: the joined item/Sniper graph previously observed Offering's table/final
inputs; it did not execute that application with supplied scaling factors.
No application boundary was removed by adding these source writers.
Reuse UsagePolicy `3259` / parameter `325a` / program `skill-effect-activation`
for Stat `3227`; that checkpoint's fixture Scenario usage list was empty. Bind real
preset preferences and scenario overrides through the existing Core composition
seam. The shared test `PlanComponents` then installed an empty application
inventory; extend that seam explicitly rather than copying the application
formula or supplying an activation literal. Check any additional contribution
origin authority needed by the later combined consumer before changing its
public contract.

Composed support discovery remains accepted follow-on work. The Life equipment
domain and final Player consumer retain their independent numerical/coverage
obligations. These changes do not require a new owner decision.

## Previous runtime checkpoint: exact Skill queries and self-contributions

Operations V24 extends the existing query graph with `Current` Skill reads and
`ContributionOrigin::Skill { authored, supplies }`. A membership can explicitly
admit direct authored uses, generated uses through named Skill supply slots, or
both for the same definition/program/effect. This avoids duplicating a Skill
definition merely because it can be acquired in different ways. Programs remain
Skill-owned and may contribute only to their exact current Skill; this boundary
does not authorize inherited Actor adjustments, support deliveries, Gem-owned
source-property programs or arbitrary related-Skill writes.

Data checks the owner, context, target Stat, direct-selectability permission,
known unique supply slots, exact supplied definition and absence of equipment
ranks. Engine binds direct uses against their exact source record and generated
uses against the existing validated Skill-to-provider map. The parent Skill key
is not confused with its entered grant path. That map is retained for late
support-suffix membership checks alongside the Actor map. All checks use the
existing bounded cold planner; workers and reduction execution are unchanged.
Schema3 remains current and V24 uses effect-plan identity domain21.

Validation passes: **10 new Engine checks**, the existing **7 Actor/reward** and
**13 ordered-contribution** checks, **18 Data** checks and **3 Core** capability
checks. Positive cases include four generated siblings/copies and a single Skill
definition used by two direct copies plus two item-supplied copies. Negative
cases cover wrong supplies/owners/context/recipient, absent direct permission,
semantic ties, unavailable inputs, Partial/empty inventories, unread/inactive/
neutral members, late support additions, raw-compilation admission and bounded
work. Fresh/reused A/B/A, storage permutations and four Rayon workers agree.

All **65 joined Sniper checks** also pass on the unchanged V23 package: 56
integration checks in **35.70s** and nine ordinary checks in **0.03s**. Strict
workspace/all-feature/all-target Clippy passes in **39.52s**, along with changed
Rust formatting and `git diff --check`. The mixed-acquisition test initially
omitted its item inventory and then its early grant-readiness declaration; both
were corrected in the fixture. Production coverage/readiness guards were not
weakened. No source VM or new game-law parity claim is involved.

Evidence: `runs/owned-skill-queries-{engine-04,engine-06,core-02,data-01,
joined-01,joined-ordinary-01,clippy-01,format-01}.log`. `engine-04` retains the
passing 20 previous Engine tests; `engine-06` is the final ten-test Skill run.
At **18:28:37 UTC**, predecessor `5c5ab1f` CI run `37822708160` remained pending;
`runs/owned-skill-queries-prior-ci-01.json` is the snapshot. Local validation is
not a hosted CI success claim.

Numeric positions remain explicit. Separate Skill recipients do not require an
ordering between their IDs, while ties within one recipient still reject.
Boolean Any keeps its unordered semantics. Complete-empty, Partial, unread,
inactive and zero-valued contribution domains retain the ordinary graph checks.
The runtime admits reusable positive self-contributions, not only empty queries.
No canonical game artifact, numerical formula or definition allocation changes
in this runtime checkpoint; the active package remains
`runs/owned-life-queries-publication-02/package` (V23), with 0/5 complete builds.

**Follow-up:** the source-scaling checkpoint above adopts this boundary and
rebuilds the originals. The application join and nonempty/inherited delivery
proofs remain separate work. Composed support discovery needs mandatory
complete-request coverage before retiring input obligation `01de`.

**Life investigation:** the native raw Life slot `3101` permits 0–1,000,000, but
the corruption-factor slot `3117` independently permits 0–1,000,000. The exact
plain-Life importer emits factor 1; its narrower admission is not a bound on all
valid native requests. Current magnitude/catalyst producers and future admitted
transforms also need a composed bound or reviewed order. No input range was
silently tightened and no equipment query was promoted to Complete. The existing
Life reduction, routing/copy, source-coverage and final-resource obligations stay
open. This concrete obstacle prompted work on the already-approved Skill
connection in the same closest build rather than claiming Life completion.

## Previous runtime checkpoint: checked Actor and reward contributions

Operations V23 adds `ExistingActor { application }`, `Reward` and
`SuppliedActor { slots }` to the existing contribution membership contract. Data
authenticates exact owners, applicability declarations, declared Actor slots and
provider definitions. Engine follows the validated Actor supply relation, also
retained for late support-suffix checks; it does not equate a parent provider
address with the entered grant path. Direct provider roots remain constrained
to empty paths. Worker reduction/execution is unchanged: no separate aggregator,
Lua behavior, build names, Life formulas or source-specific runtime code.

Full occurrence and recipient identity survives binding. Numeric same-recipient
ties reject, including repeated numeric reward selections and multiple supplied
Actors writing the same Player channel without an ordering law. Boolean Any
preserves duplicate sources without assigning a numeric rank. Inactive, unread,
zero and Partial contributors retain existing coverage checks. Schema3 remains
current; V23 uses effect-plan domain20 to invalidate prior semantic identities.
At this runtime-only checkpoint the game package remained V22. The subsequent
Life query publication below adopts V23; five-build admission is unchanged.

The real joined Sniper fixture now has opt-in checked Life reduction probes over
its existing imported equipment, rewards, shared Player and two distinct minion
occurrences. They emit test-only diagnostic Stats, never final canonical Life.
The test order covers a finite exact integral Player BASE domain; it is not a
general game ordering law. Remaining minion INC, complete inventories, resource
formula, overrides and conversions are explicitly outside that fixture closure.
The first compile caught a test field-name typo. The first integration run kept
all 59 prior checks passing and rejected the new fixture's incorrect use of two
source ranks for one shared owner; it now uses one source rank and two program
ranks. The next run rejected diagnostic receivers left in the fixture's default
early-readiness phase; these now explicitly execute after contribution stages.
No correction weakens a production guard.

The three new joined checks pass in **28.24s**: actual Player BASE **1257**, INC
**5**, empty Player MORE **1**, and separate minion BASE/MORE. Existing individual
producer/source checks remain in that same graph. A level change, quest removal,
ring roll change and removal of one ring use produce BASE **1243** without changing
minions. Fresh/reused A/B/A and four Rayon workers agree exactly; storage reversal,
unread missing minion membership and Partial coverage are checked. The original
59 joined checks passed in the earlier combined run. These are component results;
the unchanged final-Life target remains **1320**, not yet supplied natively.

Regression validation passes: **3 Core**, **28 Data** and **20 Engine** tests
(seven new Actor/reward checks plus 13 existing contribution checks). These cover
raw/stored validation, round trips, wrong origin/applicability, duplicate rewards,
unread/inactive members, typed Boolean uncertainty, Partial coverage, work bounds
and deterministic fresh/reused/Rayon execution. The existing Engine test's
checked empty-support/readiness construction is now shared, removing a duplicate
fixture rather than bypassing staging. A leftover unused import was removed.
Changed-file formatting and strict workspace/all-feature/all-target Clippy pass;
Clippy finished in **36.09s**.

Evidence: `runs/owned-actor-reward-joined-{02,03,04}.{jsonl,log}` (final joined
run `04`), `{core-01,data-01,engine-04,format-01,clippy-01}.log` with the same
prefix. Source/game artifacts and the five-build input report remain unchanged.

**Follow-up:** the query publication below supplies canonical membership. Finish
equipment's reduction domain and the actual Player consumer. Preserve remaining producer/owner
obligations, prove bounds for the composed arithmetic and account for
ExtraLife/Total/post-clamp conversions, overrides and Chaos Inoculation. Do not
promote the fixture's empty or finite domains to production completeness. Exact
Skill queries and composed support discovery remain approved follow-on work.

## Previous native data checkpoint: canonical Life contribution queries

The [Life query packet](../data/owned/poe2/3887ae68/life-contribution-queries/README.md)
adopts operations V23 and adds three canonical `311a` queries with seven groups
and eight exact known writers. Five full donor owners are authenticated. No
producer, receiver, definition, input, selected build or owner closure changes.
The equipment policy includes all twenty known native equipment slots rather
than specializing to Original05's currently occupied Life-bearing slots.

Six groups have Complete **bounded membership**: intrinsic, inherent, rewards,
received minion increase and the one current MORE source. Every member in each
of these groups has the same all-zero semantic position and a non-equipment
origin. A successful candidate binding therefore has at most one potential
effect per recipient; duplicates tie and reject even before activation. Empty
or singleton Sum/Product needs no associativity or source insertion-order claim.
This proof is independent of saved-build values. It does not close global or
owner inventories, or declare other Life sources absent.

Equipment is the seventh group and stays Partial: it may contain several
effective amounts. Its provisional slot ranks do not establish a general
ordering/rounding law. The publication census checks every current potential
Life contributor, including inactive effects and effect applications; adding a
writer or changing a donor's guard/recipient requires a renewed audit. Group
coverage does not authorize a final resource result.

The joined Sniper fixture now reads this packet's exact query data and removes
its local member/rank constructor and obsolete empty-identity constants. It
keeps one explicit finite exclusion for the received-minion-Life program not
yet installed in that graph. Only its tested equipment domain is closed in the
fixture. All **65 joined checks pass in 33.76s**, including Player BASE1257 / INC5,
changed-input BASE1243, separate minions, packet coverage refusals, duplicate
reward ties, storage reversal and fresh/reused/four-worker execution. No final
canonical Life is supplied by those diagnostic probes.

Two ordinary packet checks and publication pass in **23.38s**. The V5 metadata
migration reuses the current importer and identity rebinding; after it, the exact
inverse removes only three queries and restores the authoring receipt.
Byte-identical rebuilds and all-five import/selection/110-query preservation
pass. Issue counts remain **107/117/109/123/5**. All **six V5 migration tests**
pass in **2.06s**, including V23 preservation, repeated rebuilds and downgrade
refusal. Changed-file formatting and strict workspace/all-feature/all-target
Clippy pass; the final Clippy run took **3.08s** after removing an unused test
module import. No source VM ran and no new PoB numerical law was inferred.

Package at this checkpoint: `runs/owned-life-queries-publication-02/package`. Evidence uses
`runs/owned-life-queries-{ordinary-01,publication-02,joined-01}` plus the publication
directory's receipt and validation, plus `{migration-01,format-01,clippy-02}.log`
with the same prefix. The earlier `publication-01` output records
the pre-proof all-Partial groups; it is immutable evidence, not the active package.

**Life resume:** equipment's effective-Life amount domain is the next Life
dependency. Its raw amount is bounded, but remaining magnitude transforms,
canonical input admission, routing/copy and external-contributor obligations
prevent a blanket exact-sum proof. Reuse the existing numeric compiler and
retained item evidence to retire those actual blockers. Then connect the
original final-Life arithmetic, conversions/extra/total/override/CI inputs and
operand bounds. Do not insert known-empty values from Original05 observations.
The later V24 checkpoint above implements Skill query authority and identifies
its immediate data adoption; composed support discovery remains approved work.
No new public-model decision was introduced in this Life packet. At **18:12:49 UTC**,
the preceding `bfcb877` CI run `37818578183` remained pending; local checks are
not evidence of hosted success. Snapshot: `runs/owned-life-queries-prior-ci-01.json`.

## Previous native data checkpoint: all selected equipment placement

The [current packet](../data/owned/poe2/3887ae68/selected-equipment-placement/README.md)
closes five existing template inventories: Tattered Robe, Rope Cuffs, Sapphire
Ring, Fine Belt and Ashen Staff. Combined with earlier data, **all nine selected
uses / eight templates** have Complete character-slot destinations. The Ring
retains three known slots and two distinct equipped uses; Staff's two source
weapon-set labels map to one typed slot with existing loadout scope. No item
name dispatch, new definitions, numerical programs or alternate slot model is
added. Complete placement does not establish sockets, stock, requirements,
support origins, weapon attack profiles or complete item mechanics.

This supersedes the three-item packet and its source/binding helpers in place,
rebuilt from the same flat-Life predecessor. The obsolete current paths are
removed; prior reports remain immutable. The joined Life fixture now requires
published Complete inventories for every relevant template, retiring its last
finite Ring closure without replacing it with a new fixture allowance.

Validation:

- Three optional source-target checks pass in **16.79s**. Five real items each
  cover 113 registered slots, seven equipment contexts and nine flag settings:
  **35,595 calls per observed VM**. Exact repeats, fresh observed replays,
  no-call controls and JIT-off/on reports agree. A pinned branch/type/tag audit
  establishes destination completeness beyond this finite census. Lua return
  distinctions and unregistered labels remain reference-only evidence.
- Three ordinary packet checks pass. Publication passes in **28.41s**, including
  full-report authentication, exact recipe inverse and byte-identical rebuild.
  All five drafts, saved selections and 110 queries survive; selected input
  issues remain **107/117/109/123/5**, with no evaluation bundle.
- Public binding passes in **4.44s**: 21 actual saved uses, 37 allowed bindings,
  383 other-slot refusals and 21 wrong-scope refusals. Ring multiplicity, both
  Staff loadouts, restored Partial data, removed uses and unrelated diagnostics
  are checked. The first run incorrectly compared whole reports across changed
  release identities; the repaired check independently validates identities
  and compares full binding outcomes. No production binding behavior changed.
- All **59** joined Sniper checks pass in **33.69s**, including source evidence,
  mutation/coverage refusals, fresh/reused/Rayon execution and distinct minions.
  Changed-file formatting and strict workspace/all-feature/all-target Clippy
  pass. Clippy's single clone-to-slice style finding is corrected.

Checkpoint package: `runs/owned-selected-equipment-placement-publication-01/package`.
Evidence: `runs/owned-selected-equipment-placement-source-01/`, parent source
log, `{ordinary-01,publication-01,binding-03,joined-01,clippy-02,format-01}.log`
with the same prefix, and publication `{receipt,validation}.json`. Each full
source report is 61,444,068 bytes at SHA-256
`17b24c4937313b1d35495210ca0f1d73af193246bef5a2c6ae524b4cb6a1619d`.

**Resume:** Original05 is still closest, with five selected input obligations
and **0/5** complete originals. Implement the now-approved contribution-query
boundary above, then the real final-Life consumer with checked ordering and
coverage. Offering scaling and composed support discovery follow their accepted
contracts. Existing numerical and source components are prerequisites, not a
complete build or permission to inject measured scalar results.

## CI throughput checkpoint: complete CLI target partitioning

The CLI now has four target shards on each CI platform. Cargo metadata selects
every binary and integration target exactly once; each shard runs both
`--all-features` and `--no-default-features`. The native-only test step moves
out of validation into these shards. Formatting, Clippy, boundary/dependency
checks, WASM, other package suites and the final matrix gate remain required.
The actual Cargo build-script target is separately accounted for as a build
dependency, executed by Cargo rather than selected as a test. There is no
maintained target allowlist. Unsupported future target kinds,
required features or disabled test targets fail planning rather than disappear.

Each shard attempts both profiles even if the first fails, retains the first
nonzero exit code and emits the existing bounded failure annotations. Four Rust
tests pass in both feature modes (**3.81s / 3.80s**), covering complete disjoint
coverage, source-order independence, discovery of a new target, unsupported
metadata, success/failure propagation and both profile invocation orders.
The actual metadata plan initially exposed the build-script classification gap;
the corrected partition accounts for all testable targets plus that dependency.
An initial test driver normalized PowerShell's nested exit 7 to process exit 1;
the corrected driver explicitly propagates `LASTEXITCODE`, as the hosted runner
does. Production failure handling needed no further change. The failed log is
retained alongside `runs/ci-cli-shards-{all-features-03,native-02}.log`. After the
lint-only array-chunk change, all four tests pass again in the native profile in
**3.77s** (`runs/ci-cli-shards-native-03.log`). The actual-metadata proof is
`runs/ci-cli-shards-plan-summary-02.json`: 195 testable local targets, one
Cargo-managed build dependency and a maximum command length of 1,751 characters.
That census includes two protected untracked test drafts; the committed checkout
has 192 integration targets plus one binary. The dispatcher discovers this
difference automatically and has no hardcoded target count.
Hosted completion and speedup remain unproven until the new workflow runs.

## CI repair checkpoint: downstream Boolean cutover checks

**October 8 follow-up:** the older `6a3aae3` run subsequently completed Ubuntu's
CLI job with failures in `owned_configuration_resistance_penalty_native` and
`owned_extension_cli`. Both PoB jobs passed; Windows CLI was still running, and
`0ca44ee` run `37731323865` was pending. Detailed job logs still return 403, so the
public annotations establish failing targets rather than their assertion causes.
Current local reproduction in `runs/ci-owned-cli-reproduction-01.log` found a
historical rules-schema header in the finite resistance fixture and a stale
16 MiB default-budget assertion in the extension chain. The repair constructs
the resistance fixture in the current format and checks the shared content
budget constant; historical source pins and the explicit 8 MiB refusal test
remain. The resistance target now passes **six** ordinary checks with one retained
source-dependent ignored check (`runs/ci-owned-resistance-repair-01.log`).

The full extension replay then exposed a stale `before.rules` commitment in the
current item-attribute membership artifact; a backtrace identifies that exact
publication step. These are active offline authoring inputs, so the repair
regenerates the three affected attribute/rarity-chaos/defence membership patches,
rather than adding historical rebinding to the test or loader. The existing Rust
`assemble-owned-recipe` command generated current typed identities. Exact inverse
checks prove registry/schema/routing bytes and extension commitments unchanged;
rule bodies differ only in the current package header, and each patch changes
only `before.rules`. Stale-binding refusal tests remain. Verification is in
`runs/ci-scalar-membership-regeneration-01/{prepared,regeneration-verification}.json`.
The full extension pipeline now passes: **two** ordinary checks, one retained
source-dependent ignored check, **715.94s**
(`runs/ci-owned-extension-repair-03.log`). This traverses the actual regenerated
predecessors and retains numerical checks, all-five/query preservation and
stale-before/extension rejection. Earlier failed reproduction and backtrace logs
remain. Strict workspace/all-feature/all-target Clippy passes in **2.08s**
(`runs/owned-player-life-clippy-02.log`), and affected Rust formatting passes.
The bounded follow-up audit found no additional live rule-schema-2 constructors
or stale compiler-default assertions; unrelated 16 MiB limits remain valid.
The historical D3 wording now distinguishes its old bound from the current one.
Both identified CLI targets are locally green; hosted green remains unproven.
At **06:23:44 UTC** on October 8, current `5629e2c` run `37737284986` was
pending with no jobs started. The older `6a3aae3` Windows CLI job remained
running; Engine, Import and PoB jobs passed on both platforms. No failure
annotation identified a new target beyond the four already repaired locally.
Step status alone does not establish execution progress in the long CLI job.

The public job annotations for run `37715752732` identify two failures on both
Windows and Ubuntu: allocation export reproduction and Data's
`owned_support_outputs` target. Full log downloads return HTTP 403; both failures
were reproduced locally against `288bfe8`, so the repair is based on actual
failing assertions rather than inferred job titles. The successor run
`37723486352` was still pending at that inspection.

- The Boolean migration changed the current bundle's `transition.json` but left
  allocation `source-facts.json` with its previous digest. The unchanged exporter
  regenerated the receipt; only `bundle_transition_sha256` changed, to
  `72002ae25afc70a51e22c22f968b3bf366200b94cad657f0635447f39b88e878`.
  Allocation rules remain byte-identical at SHA-256
  `30a79be0d556a022ac30e60edf2beabb6d8e32bd16a101e0c10ecb40ca6898b3`.
  All fourteen existing exporter tests and strict `--check-dir` reproduction pass.
- The support-output test constructed an unchecked Boolean collection read.
  Rule admission now correctly rejects it before support-output validation.
  The Rust test asserts that exact earlier refusal, while retaining all four
  later output-boundary checks. All twelve tests pass. No production validator,
  fixture semantics or historical compatibility path was changed.

Evidence: `runs/ci-support-outputs-{reproduction,repair}-01.log`,
`runs/ci-owned-allocations-reproduction-01.log`,
`runs/ci-owned-allocations-repair-{tests,check}-01.log` and regenerated review
files in `runs/ci-owned-allocations-refresh-01`.

**Validation complete:**
`cargo test -p poe-optimizer-data --all-features --locked --no-fail-fast` exits
successfully: **587 tests pass, none fail or are ignored**. Its 60 result blocks
include an empty doctest target; the tests report 612.22s plus 58.26s compilation.
Execution session `19524` is terminal and must not be restarted as unfinished
work. Strict workspace/all-feature/all-target Clippy and changed Rust formatting
also pass. Evidence: `runs/ci-owned-data-regression-01.log`,
`runs/ci-owned-data-regression-summary-01.json` and
`runs/ci-owned-cutover-{clippy,format}-01.log`.

The fixes are pushed in `7cd9060`. At 03:51:53 UTC on October 8, its hosted run
`37724617730` was pending behind the earlier live CLI/PoB jobs. No further
completed failure was visible. Local validation is complete; hosted green is
unproven. At 05:08:49 UTC, current `648e7fb` run `37730656512` was pending with
no jobs. The older `6a3aae3` run's Ubuntu PoB job had just passed after 156 minutes;
Windows PoB and both CLI jobs still ran with no new failure annotations. The API
does not distinguish compilation from execution, so a hang is unproven. Main's
configured concurrency explains the pending/coalesced runs. Original05 remains
the target, with five selected input obligations and **0/5** complete builds.

## Preceding coverage checkpoint: flat-Life component scalability

The selected Life modifier `3100` had one stale component-metadata obligation.
The retained original-source reports and the current injected numeric compiler
already prove its sole component scalable. The new
[packet](../data/owned/poe2/3887ae68/flat-life-scalability/README.md) removes only
`numeric-component-scalability-unproved`, taking seven owner gaps to six.
All five programs, raw inputs, the sole emitting item rule, source guard and
every other coverage declaration remain unchanged. Component scalability is
distinct from the separate unscalable input and does not prove arbitrary
numeric domains, routing, Amulet copies, contributor coverage or final Life.

Two ordinary checks pass, including rejection of an additional emitter, changed
guard and changed component. Publication passes in **22.30s**. It authenticates
the retained seven-case/23-control source reports without starting PoB, recompiles
the numeric binding to the exact existing writer, and proves a whole-input
inverse allowing only the retired gap plus provenance. All five drafts/local
identities, 110 queries and selected issue counts **107/117/109/123/5** survive;
the rebuilt package is byte-identical. The existing joined graph passes all
**60** checks in **34.16s** against the successor, including actual Partial-owner
refusal, duplicate physical ring uses, mutation/order controls and reused/Rayon
execution. No numerical formula or new fixture was added.

The current integration package is
`runs/owned-flat-life-scalability-publication-01/package`, input
`dafd7c805646bc91dfa6c7394d1a44110fb3c26a0998ce4b154fc5c476a53784`, rules
`69d122ac6640a5b93377b9728e1d21b8a8741961cfc1d5ba4bc32fe89899c74c`, compiled rules
`ffac0a8c5117bd02e1446cf8c0f3968a495f9e57c707392bcfe5b74bab8deaba`.
Evidence: `runs/owned-flat-life-scalability-{ordinary-02,publication-01,native-01}.log`
and publication `{receipt,validation}.json`. The initial helper compile caught a
moved-value borrow in the inverse assertion; binding the restore target before
moving the owner fixed it. The failed build log remains. Changed Rust formatting
passes, and strict workspace/all-feature/all-target Clippy passes in **26.95s**
(`runs/owned-flat-life-scalability-{format,clippy}-01.log`).

**Resume:** Original05 remains closest, with five selected input obligations and
**0/5 complete builds**. Its next native aggregation still needs the pending
Actor/reward contribution-query decision; Offering needs the separate exact
Skill query decision. Both were re-prompted after the source checkpoint. The
archived Warrior preflight found a separate Import contract boundary, recorded
below; it produced no authority or source-link reduction. Do not re-open the
completed attribute/flag memberships or treat this metadata retirement as a
complete modifier owner.

## Native checkpoint: real Player Life contributions in the joined graph

The shared Player Actor `332a` now has one ordinary authored program that reads
the pure inherent amount `331a` and emits it once to canonical Life `311a`.
The existing Boolean disable flags guard emission: disabled means no record,
whereas enabled zero Strength emits one zero record. The independent thirteen-case
source witness and compiled-rule checks preserve that distinction. There is no
new formula, measured constant, receiver or public model extension.

Publication passes in **20.75s**, preserving all five imported drafts, origins,
selections, 110 queries and issue counts **107/117/109/123/5**. Its exact inverse
allows only the appended Actor program and provenance receipt; a rebuild is
byte-identical. Actual Actor/global coverage remains Partial, and no evaluation
bundle or final-Life metric is added. The current integration package is
`runs/owned-inherent-life-contribution-publication-01/package`, input
`77647de61486b91c2de2d4e73547c6fd2e5ef40f506c947d84bf82a04836726b`, rules
`3e650238f83dcf35e0d868f766743575d6e36d634e62660bdf5a4b987919a008`, compiled rules
`8ae86b988ca39964b7ee8f12089cd6a2bdf5caa702d4bb4556d82fa7b19e7fcc`.

The joined fixture now loads the imported character's actual level instead of
retaining the earlier component fixture's level 100. It also imports four actual
Life-bearing item records with five equipped uses, including two distinct uses
of the same Sapphire Ring record, and the two selected quest rewards. Numeric
preparation, catalyst inputs and delivery use existing current rule bodies.
These finite checks exclude unrelated item mechanics and do not certify complete
equipment or reward inventories. All **60** joined checks pass in **34.61s**:
53 artifact-backed integration checks plus seven ordinary checks. They establish
intrinsic Life **1120** at imported level 92, calculated inherent Life **54**, the
five item deliveries **17/16/10/10/10**, and reward Add **20** / Increase **5** as
distinct contributions, without supplying final Life. Controls cover actual
source identities, ring mutation/removal, query/input order, missing inputs,
stage constraints, incomplete inventories and fresh/reused/Rayon equivalence.

The failed runs exposed fixture assumptions, not production fixes: global
catalogs are Pending; three equipment templates have Partial slot inventories;
removing all requested attacks invalidates the retained Action programs; and
Partial early-preparation owners reject before plan construction. Repairs retain
the actual catalog/placement/owner obligations, admit only exact imported slots
in the finite fixture, preserve required attack contexts for the query-order
check, and assert the earlier preparation refusal. Complete production coverage
is not inferred from these projections. The Crown/Amulet origin check now scopes
its existing exact assertions to its modifier family, alongside the separate
five-Life-use checks. Failed logs remain available.

Evidence: `runs/owned-inherent-life-contribution-{authoring,publication}-01.log`,
publication `{validation,receipt}.json`, and
`runs/owned-player-life-native-complete-05.log`. Strict workspace/all-feature/
all-target Clippy passes in **3.01s** and changed-file formatting passes in
`runs/owned-player-life-{clippy-01,format-02}.log`. The joined integration checks
remain ignored in ordinary CI until artifact provisioning is implemented.

**Resume:** the Player Life reference validation below passes. Final aggregation still awaits the
[Actor/reward query decision](owned-actor-reward-contribution-queries-proposal.md);
Offering's source scaling awaits the separate
[exact Skill query decision](owned-skill-contribution-queries-proposal.md).
Both were sent for owner review on October 8. Neither authorizes unrelated
support composition or Actor-to-Enemy reads. Keep Original05 first and preserve
all five originals; complete native builds remain **0/5**.

## Player Life reference checkpoint

The existing optional Rust witness now observes the **actual argument** passed
to original `round` from `CalcDefence.lua:96`, bound to the same original Player
consumer frame, Actor, store and output. The round function is not wrapped, no
modifier query is added, and no operand formula is reconstructed. The original
round return remains unobserved; line 97 supplies the independently observed
result after the minimum of 1, before later Chaos Inoculation handling.

**Validation complete:** eleven cases pass the fixed protocol of two observed
JIT-off acquisitions and two uninstrumented references in each JIT mode:
**66 fresh loads in 514.43s**. All acquired repeats, declared reference projections
and acquisition-to-uninstrumented projections agree exactly. The two acquisitions
per case retain 72 original Player invocations each: **1,584** actual operands
replay through the production native Round and Maximum operations with matching
checkpoint results. These scalar operations use synthetic test definitions;
they are not a new resource formula or a replacement build request.

| Control | Observed operand | After rounding and minimum | Final Life |
| --- | ---: | ---: | ---: |
| Unchanged Original05 | 1319.8500000000001 | 1320 | 1320 |
| Robe Life 9 | 1311.45 | 1311 | 1311 |
| Robe Life 10 | 1312.5 | 1313 | 1313 |
| Robe Life 11 | 1313.55 | 1314 | 1314 |
| Custom zero base | 0 | 1 | 1 |
| Custom negative base | -1.05 | 1 | 1 |
| Custom Chaos Inoculation | 1319.8500000000001 | 1320 | 1 |

The previous character-level, Robe Life 18, increased-Life and more-Life controls
remain. Custom controls prove no obtainable supplier. The first expanded run
failed because tiny fractional increased/reduced text was not accepted by the
pinned generic modifier parser, leaving the operand unchanged. Those controls
were replaced with integer robe rolls; the failed source report is retained.
This was a witness assumption, not nondeterministic source output. An initial
test-helper compilation error was also fixed; failed build logs remain.

A separate authenticated extraction of original `Common.lua:722–728` compares
**198** finite scalar inputs per configured JIT mode, including adjacent floats,
signed zero and large integers. It retains three differences masked by the
minimum and two differences that survive it. Reverse scratch reuse and exact
cross-JIT helper results pass. The [rounding audit](legacy-retirement.md#player-life-rounding-bound-2026-10-08)
also derives an analytical combined-operation bound of finite x <= 2^52. This
does not prove that game inputs stay in that domain, or change native rounding.

Evidence: `runs/owned-player-life-rounding-source-02/{comparison,rounding-helper-comparison,measured-cases-summary}.json`,
separate complete acquisition/reference reports and raw per-case files. The
parent log is `runs/owned-player-life-rounding-source-02.log`. Eight ordinary
checks pass (`runs/owned-player-life-rounding-ordinary-03.log`); strict workspace/
all-feature/all-target Clippy passes in 0.52s, and changed Rust formatting passes.
The source report pins the native test helper and production executor alongside
its source/observer/harness identities. The original six-case checkpoint is
preserved in [history](implementation-history.md#archived-2026-10-08-checkpoint-first-player-life-consumer-witness).

**Resume:** use these observed operands and the justified arithmetic domain when
authoring final Life after the accepted Actor/reward query implementation. Operand
generation, positive ExtraLife/Total/conversion/override suppliers, raw conversion
Sum, complete owner/recipient coverage and whole-build parity remain unproved.
No production schema, rules, import policy, evaluation bundle or original input
changed in that source checkpoint; its flat-Life baseline is now superseded by
the selected placement package above. Keep
Original05 first, with five selected input obligations and **0/5 complete builds**.


## Native checkpoint: real Strength and checked flags reach inherent Life

The five existing Boolean Any queries `3315`–`3319` now have checked bounded
membership. Two authenticate the exact direct passive writers for doubling and
halving inherent Life; the other three reject every actual matching contributor
as outside their complete-empty domain. All current owner and application bodies
are censused without evaluating guards. The five reducers and pure derived-amount
receiver `331a` are unchanged. Donor program inventories and the global query
registry remain Partial; this does not claim all game flag sources are supported.

The existing Sniper graph now uses real second-pass Strength `1d2e`, all five
checked flag receivers and the actual Strength-Life receiver. A fresh import of
all 55 saved allocations proves neither flag-producing passive is selected in
Original05. It derives **Strength 27 → inherent Life 54**, with source evidence
used only for comparison. No measured Strength, default Boolean or test-local
flag writer supplies that result. The receiver still emits only the inherent
amount, not a contribution to canonical Life or a final Life pool.

All **44** joined ignored checks pass: baseline **28.17s**, remaining 43 checks
**33.85s**. The six new checks include actual halving/doubling bodies and all four
combinations, duplicate Any sources with retained identities, missing producers,
early stages, unknown/inactive/false sources, actual Partial inventories and
fresh/reused/Rayon restoration. Counterfactual passive controls explicitly cover
their flag bodies only; finite fixture closure does not establish tree legality
or their other mechanics. The two prior authoring checks plus two new ones pass
in the joined target; all three packet authoring checks pass.

Publication passes in **21.04s** with an exact whole-input inverse and byte-identical
rebuild. Only five membership closures and one provenance receipt change. All
five imported drafts/origins/selections, 110 queries and issue counts
**107/117/109/123/5** are preserved. There are no new definitions, programs,
receivers, schema identities or evaluation bundle. Complete original builds
remain **0/5**. The current integration package is
`runs/owned-attribute-flags-publication-01/package`, input
`2ad4c943b6b6d79e4421746e5318212c69245f885deb1ec2a46e293a87d1d887`, rules
`352538bb060c1ca80704d9a8946c36514d513ecd164522e7d2563160ec3f4858`, compiled rules
`18461019d6aeb3a54e871aa3644257cf615c5d1eb843b54ca22a10d7ad692d9c`.

Evidence: `runs/owned-attribute-flags-{authoring,joined-ordinary,publication}-01.log`,
`runs/owned-attribute-flags-native-{baseline,regressions}-01.log`, and publication
`{validation,receipt}.json`. Strict workspace/all-feature/all-target Clippy passes
in **2.63s**, and changed Rust formatting passes (`runs/owned-attribute-flags-{clippy,format}-01.log`).
Independent review found no correctness issue. The source authenticator reuses the retained independent 13-case
Strength-Life witness rather than adding another observer or copied formula.

Retain the two ordinary `owned_inherent_attribute_flags` tests for now. Their
behaviors are covered by the joined tests, but the latter require local artifacts
and are ignored in ordinary CI. Their current value is automatic execution,
not a second production path. Retire that finite supplied-Strength fixture after
the current joined package is reproducibly provisioned in CI, or transfer its
ordinary execution coverage first. Keep the independent Strength-Life witness
and receiver tests for disabling, zero-versus-absence and lazy missingness.

**Follow-up:** the checkpoint above adds actual Life-bearing equipment and
selected rewards, retaining distinct physical uses.
Final Life query origin authority is pending in the
[Actor/reward proposal](owned-actor-reward-contribution-queries-proposal.md);
Offering's source scaling still needs the
[exact Skill query decision](owned-skill-contribution-queries-proposal.md).
Both questions were sent for owner review on October 8. Those decisions do not
authorize unrelated support composition or Actor-to-Enemy reads. Keep `331a`
pure and deliver its derived amount through the existing shared Player Actor
owner. Do not bypass the candidate-wide census, revive the retired Life alias,
or declare complete metrics before real receiving, source and owner coverage.

The final-pool audit retained Original05 Life records totaling 1257 Add and
5 Increase, with saved MAIN/CALCS Life 1320. These are diagnostics, not a native
recipe or proof of absent families. The subsequent
[Player consumer witness](#player-life-reference-checkpoint) addresses the
missing original-local evidence separately from uninstrumented reference runs;
the earlier detailed observer covered only Sniper. In
`CalcDefence.lua:74`–`134`, conversion has an upper
cap but no lower clamp, Extra precedes scaling, Total follows it, overrides bypass
ordinary rounding/clamping, and Chaos Inoculation follows the pool calculation.
The source `floor(x+0.5)` and native mathematical tie rounding also need bounded
parity evidence rather than a blanket Lua-compatibility claim. Keep the current
item Modifier `3100` and incoming minion-Life `32e5` coverage explicit.

## Native checkpoint: bounded class/passive BASE membership

The next numerical prerequisite is the selected Player's attribute calculation,
which feeds final Life and other Player metrics. The current six BASE channels
contain 48 Class and 1,924 Passive literal Add effects, plus twelve item
effects outside this bounded domain. The class/passive literals are integers from
3 to 25. Their complete per-channel inventory sums are 1711/1707/1743, repeated
for the second pass. Exact membership and unique semantic positions can bound a
successful candidate to a subset of that inventory; this is not an argument for
reordering the signed/scaled item contributors described in the BASE audit.

The 347 passive owners include one ascendancy node; the positive-literal proof
does not depend on which point pool supplies the allocation. Original05's 22
selected attribute allocations are ordinary passives.

Six checked memberships now cover that reusable class/passive domain. Candidate
binding rejects every actual unmatched contributor, including inactive, socket,
copied or dynamic contributions. Class/Actor owners and the global query registry
remain Partial. The six already-published guarded empty-MORE producers and their
Complete empty memberships are reused unchanged. No example identifier selects
production behavior, and no test-local factor or coverage certificate enters the
published package.

Original05's twelve retained BASE stages (MAIN/CALCS, three attributes, two
passes) contain only Base/Tree records; none contains an Amulet-copy zero. The
other originals still provide the item/copy contrasts. The selected class and
actual imported attribute allocations now join the existing Sniper graph, using
the published consumer bodies and current guarded empty-MORE producers. The
baseline produces 27/7/105 in both passes and passes in **27.91s**. This remains a
component prerequisite, not complete Player or build evaluation. The numerical
fixture does not certify tree legality and explicitly retains production Class
and global Partial refusals. Independent review found no correctness or scope
blocker.

Publication passes in **22.76s**: exact whole-release inverse, byte-identical
rebuild, all five imported drafts/origins/selections and all 110 queries. Only six
memberships and provenance change; there are no new definitions, programs,
receivers, schema identities or evaluation bundle. Selected issues remain
**107/117/109/123/5**, with **0/5** complete originals. This checkpoint's checked
package is `runs/owned-attribute-base-publication-03/package`, input
`168da7cecbdabe052e1608bd621617cc25722ca6ef5b72ccfcdc1f3aa380929d`, rules
`2f14b8d3e91939ea4f127539b068eaa11c63322579431b4fa8cf7bfc6ff2c6ba`, compiled rules
`747064b9429ac1af3886f17c3b63085e7ab02869d81af1fddeea3864814f0030`.

The complete membership inventory crossed the compiler's former 16 MiB default
by 67,482 bytes. Its default now uses Core's existing 64 MiB content ceiling,
matching owned storage/import, while caller-selected lower budgets and every
structural/work limit remain enforced. All **25** compiler tests pass, including
exact-size acceptance, one-byte-short/zero/over-ceiling refusal and unchanged
compiled identity/results across sufficient budgets. No format or compatibility
branch was introduced. Initial compile, byte-limit and historical expected-count
failures remain in their original logs; the new publication uses the existing
explicit-count API to check the predecessor's actual issue counts.

All **38** joined ignored tests pass across the baseline, 36 regression checks
and corrected attribute-choice control (**28.16s**). The latter initially
searched for `AttributeOverride` directly under `Spec`; the actual saved XML
nests it under `Overrides`. The corrected test still requires one exact
container/record, the retained control hash, a byte-identical inverse, all 55
imported allocations, exactly one changed choice and 22/12/105 in both passes.
Reordered inputs, reused/Rayon scratch, duplicates, unsupported sources and real
Partial coverage refusals pass. Two ordinary joined authoring checks and three
packet checks also pass. Strict workspace/all-feature/all-target Clippy passes
in **26.91s**. The joined target remains ignored in ordinary CI until reproducible
package/source provisioning is implemented.

Evidence: `runs/owned-attribute-base-publication-03/{validation,receipt}.json`,
`runs/owned-attribute-base-publication-03.log`,
`runs/owned-attribute-base-native-baseline-01.log`, and
`runs/owned-attribute-base-rule-limits-01.log`,
`runs/owned-attribute-base-native-regressions-01.log`,
`runs/owned-attribute-base-native-choice-02.log` and
`runs/owned-attribute-base-clippy-01.log`. The regression log preserves the initial
choice-test failure; only its focused corrected replay is green. The optional
prepared-hand source witness is complete at the separate checkpoint below.

**Follow-up completed above:** the five inherent-attribute Boolean memberships
now bind to their current producer census, and real second-pass Strength reaches
the existing inherent-Life receiver `331a`. Three guarded-empty flag domains and
two actual passive producers supply the checked inputs. Final Life needs real
item/reward sources and
reviewed query authority for shared/generated Actors and Rewards. The current
direct-provider-only query boundary cannot admit those sources; do not move their
ownership or bypass membership validation merely to obtain a number.
The [Actor/reward query proposal](owned-actor-reward-contribution-queries-proposal.md)
documents that pending public boundary. Pure receiver `331a` must remain a derived
amount; deliver it through the existing shared Player rule owner rather than
expanding receiver responsibilities.

The subsequent Warrior preflight disproved the proposed private-reuse path for
eight archived origins: the Physical selector requires actual Gem supply, which
the item-only Warrior does not have. The
[selector/supply separation](owned-source-action-root-separation-proposal.md)
is proposed, not accepted; no source links or unresolved owners were removed.
Keep it behind the selected numerical prerequisite. The archived Firebolt pair
has Complete-empty usage, so it cannot use a Pending-responsibility proof.

## Source checkpoint: prepared hands and original condition writes

The optional prepared-hand extension passes in **262.41s**. Eight controls each
use a fresh observed load, independent observed replay and fresh unobserved load
in both LuaJIT modes: **48 complete cold loads**. Reports are byte-identical at
24,635,377 bytes each, SHA-256
`608f0579f3cc59bad4dae8827ce43c65730cdfc8c3ac481b2d45dcb8860c140a`.
Original methods, exact environment/Actor/store identities, raw line events and
source inputs are preserved. The unchanged Original05 default observation also
matches the authenticated historical off-hand report. The old observer file is
unchanged; this is an opt-in extension, not a second runtime evaluator.

The controls distinguish occupied caster main hand, empty hand, ordinary mace,
glove presence, synthetic Facebreaker text and actual off-hand filtering of the
main item. Original05 retains the Ashen Staff but receives PoB's intrinsic
`None` attack profile and `Unarmed=true`. Removing gloves changes the observed
`Unencumbered` state. The Facebreaker-text control preserves the observed raw
`actor.Gloves` absence separately from equipped gloves. These are source facts,
not a native law or proof that synthetic items are obtainable. Positive
`DisableWeapons`, Hollow Palm and warm/rebuilt behavior remain unproved; captured
Player scalars do not cover the entire minion/output graph. Shared Actor/Class
closure remains withheld. Any Lua-specific behavior needs its own domain or
source-defect decision before native authoring.

Source01 failed before VM execution because one new verifier hashed raw CRLF
checkout bytes. It now uses the existing pinned normalized-text verifier with
path guards; no source pin changed. Source02 then exposed a witness bug:
LuaJIT can repeat the compound assignment's line event within one function call.
The observer now tracks actual call/return frames, retains every entry revisit,
and still rejects missing/reentrant/unfinished frames. Source03 passes without
retry-until-match, event normalization or calculation changes. The ordinary
control-isolation test and formatting pass; strict workspace Clippy also passed
with this source code. Evidence: `runs/owned-player-prepared-hands-source-03`,
`runs/owned-player-prepared-hands-source-03.log`,
`runs/owned-player-prepared-hands-ordinary-02.log` and
`runs/owned-attribute-base-clippy-01.log`. Failed directories remain immutable.

## Previous native checkpoint: Boolean Gigantic and intrinsic Life in the Sniper graph

The [Gigantic cutover](../data/owned/poe2/3887ae68/gigantic-flags/README.md)
replaces integer-presence Stat `3307` with a Boolean Flag at the same identity.
Actual passive `1532` (source node `46365`) contributes true; the existing recipient
resolves status `3308` with a checked, unordered Any query. Duplicate sources keep
their identities and produce one status and one pair of Life/Damage factors.
Complete empty membership yields false; unknown or Partial coverage does not.
This uses the accepted Boolean graph without new definitions or runtime authority.

The same item-driven Sniper graph now joins that freshly imported selected
allocation, its paired reservation-efficiency contribution, the actual intrinsic
Life table/program and the existing Gigantic benefit program. Exact source hashes
authenticate original and removal controls. Only test occurrence IDs are remapped;
the finite numerical graph deliberately does not certify whole-tree connectivity.
Intrinsic Life is calculated through item inputs, source preparation and Actor
level projection, never supplied as a measured value. Canonical Life `311a`
receives both intrinsic Add and the separate Gigantic Multiply; retired `330a`
has no live reference. Final Life, damage aggregation and reservation delivery
remain unfinished.

Validation covers the baseline and removal, duplicate/false/unknown flags,
missing producers, Partial owners/receivers/membership, stage order, unrelated
Actors, disabled roots, independent inputs and fresh/reused/four-worker Rayon.
All **32 joined tests pass across the regression and focused runs**: baseline
**24.72s**, thirty regressions **34.05s**, and corrected refusal checks. The
missing-owner case correctly fails structural validation before execution; its
focused replay passes. Tightening the numeric-Flag diagnostic first expected a
generic type error; the actual Boolean-Flag diagnostic is now asserted and passes
in **24.84s**. Neither correction changed production validation. Strict workspace,
all-feature/all-target Clippy and changed Rust formatting pass. The joined target
remains ignored in ordinary CI until reproducible package/source provisioning is
implemented. Independent review found no lost meaningful assertion or injected
calculated output.

Two obsolete Gigantic native helpers and their redundant test entry points are
removed. Useful canonical-Life checks now run in the joined graph. The five
independent Life numerical tests remain, rebased onto authenticated current
population/activation fragments; they pass in **40.39s**. The first attempted
regression exposed a retired normalization artifact (`gems` field); no old-format
parser or compatibility branch was added. Historical publication/source packets
remain evidence, not alternate production execution paths.

Two ordinary cutover authoring checks, retained-source authentication and the
three retained family authoring checks pass. Publication passes in **37.04s**:
exact whole-release inverse, byte-identical rebuild, all-five reimports and all
110 query identities. It replaces one descriptor and two programs, adds one
query and closes no input/owner inventory. The current package is
`runs/owned-gigantic-flags-publication-01/package`. Actual flag membership, global
query registry and Sniper Actor remain Partial. Selected issues are unchanged at
**107/117/109/123/5**; complete native originals remain **0/5**.

**Resume: complete Offering delivery and selected-input accounting.** Activation
`3227` and recipient scaling `322b/322c` have real producers. Source-Skill scaling
`3228`–`322a` still needs implementation of the accepted [exact Skill query contract](owned-skill-contribution-queries-proposal.md).
Do not infer approval from this Allocation-to-Actor Boolean cutover. The Offering
and Djinn source controls below are now complete; the next independent numerical
work is the bounded BASE slice above, now published with joined numerical
validation in progress. No observed 62%
Offering bonus, neutral factor or precomputed final input may substitute for its
producer. The five selected obligations and final metric/coverage gates remain.
Original05 stays the closest-completion target; D4 starts after the first complete
unchanged build, without waiting for all five.

Evidence: `runs/owned-gigantic-flags-{authoring,source,publication}-01.log`,
`runs/owned-gigantic-join-baseline-01.log`,
`runs/owned-gigantic-join-regression-01.log`,
`runs/owned-gigantic-join-refusals-{02,03}.log`,
`runs/owned-gigantic-life-retirement-regression-02.log`,
`runs/owned-gigantic-flags-format-02.log` and
`runs/owned-gigantic-flags-clippy-03.log`. The `-01` regression and `-02` refusal
logs retain initial failures; their named corrected cases pass in the later
focused runs. There is no claim that those initial invocations were all green.

## Source checkpoint: Offering grouping and exact support delivery

The focused Offering witness adds thirteen controls to the existing optional
PoB harness, with 28 loads per JIT mode: fourteen instrumented and fourteen
independent uninstrumented loads, including warm restoration. It observes
original multiplication steps, pre-round products, local rounding and original
returns. Saved MAIN's Sniper and CALCS' Arsonist remain distinct in the original;
paired controls explicitly bind two separate Sniper recipients. Source selectors
are relocated by exact group identity after inserting another Offering.

| Control | Observed result and scope |
| --- | --- |
| Two distinct 1% MORE custom lines | Original local product `1.0201`, rounded to `1.02`; reversed identified sources agree. |
| 1% MORE / 1% LESS; two zero lines | `0.9999` rounds to `1`; zeros remain `1` with both records retained. |
| Danse on either Offering, disabled, or source order reversed | BuffEffect INC30 follows only its supported occurrence; recipient scaling is unchanged. |
| Duplicate Offering merge | Supported source emits 80 versus ordinary 62; final merge stays 80, or 62 without Danse, never the sum of duplicate copies. |

The custom MORE records occupy an inherited Player store. They establish a
reachable source grouping/rounding boundary, not a generic owned group model or
an admitted legal game producer. Danse's separate Damage MORE30 is not BuffEffect
MORE. Its description requires an additional consumed skeleton; the captured
stat map and support delivery do not prove gameplay activation. No native query,
support-origin or contributor coverage closes from these observations.

The first run's child checks passed under both JIT modes, but the parent correctly
failed exact comparison after **623.51s**. Full comparison found only two
differences: a rejected INC record's absolute position in the mixed ModList was
5 versus 4. Every record, execution step, result, identity and output matched.
The original MORE consumer checks name and type before evaluation, so this
non-MORE position has no arithmetic role. A different unrelated neighbor's
insertion order is an inference, not an observed source fact.

The corrected evidence view retains full raw reports and separate hash receipts.
Only non-MORE ModList candidate positions may be omitted, after validating the
pinned function/caller, query name, strictly increasing positive raw indices and
actual MORE-only execution steps. Candidate order/multiplicity, complete records,
accepted MORE positions, ModDB positions, arithmetic, recipients and output
availability remain exact. Three negative-control tests and independent review
cover those limits. This changes offline observation semantics, not source
execution or native determinism. The fresh `-02` run **passes in 427.23s**:
JIT-off/on checked reports are byte-identical at 8,363,147 bytes, SHA-256
`5245263a3cbbc14daebbafed0c5f26f8702899213f87cb5c4c34e7d6aa7fea35`.
Raw reports retain differing rejected-position metadata; each is 8,363,345 bytes.
Their receipts pin both hashes and the six exact omitted metadata paths. All
thirteen independent uninstrumented comparisons, the fresh repeat and warm
restoration pass. No retry-until-pass or numerical tolerance was introduced for
determinism; the failed first reports remain available.

The existing source suite passes **37 cases in both JIT modes** in **777.55s**;
every case field matches the retained Source02 baseline. This checks the default
observer view after adding the opt-in witness. All fifteen authored-membership
ordinary tests and eleven final Offering ordinary tests pass; strict workspace
all-feature/all-target Clippy passes. Both focused source witnesses are ignored
in ordinary CI and were run explicitly here; they are not a hosted complete-build
gate. Evidence is retained in
`runs/owned-offering-scaling-source-{01,02}`, their adjacent logs,
`runs/owned-offering-djinn-ordinary-regression-01.log`,
`runs/owned-offering-default-view-comparison-01.json`,
`runs/owned-offering-scaling-authoring-03.log` and
`runs/owned-offering-djinn-{clippy,format}-02.log`.

**Next native dependency remains `3228`–`322a`.** The
[Skill-query proposal](owned-skill-contribution-queries-proposal.md) now includes
the minimal implementation audit. Its read/member authority is still pending;
the first checked empty-domain packet can reuse current exact Skill binding,
staging and reductions after approval. Positive support membership/group meaning
remain explicit additional gates. Do not inject measured 62%, neutral factors or
precomputed final inputs to bypass these producers. Original05 still has five
input obligations; complete native originals remain **0/5**.

## Source checkpoint: manual Djinn admission accounting

The focused Original05 Sand/Water witness passes in **131.37s**. Its seven
observed cases cover the unchanged build, each manual global2 switch, both
switches, each disabled manual source and an independent fresh repeat. Per JIT
mode it performs 13 fresh VM loads and 26 fixed normal rebuilds, including six
independent uninstrumented comparisons. JIT-off/on reports are byte-identical:
9,839,040 bytes, SHA-256
`50f1f16913fc32818a98aef3f7246a597a56d19b495cdf67e3ee5264f921765c`.

The observer follows original `ExtraSkillStat` queries, filters and transport into
the original consumer. It retains all **3,204 calls per JIT mode**, including
312 calls in final MAIN/CALCS environments and 2,892 in explicitly identified
ancillary environments. Every queried list in this finite domain is empty.
Disabled manual Summon/Command sources are explicitly **not called**, not
classified as empty; separate tree-granted copies remain present. Effective and
raw `hasGlobalEffect` lookups agree and are absent, with no metatable fallback.
Fresh repeats and instrumented/uninstrumented outputs agree exactly through all
three lifecycle stages. This is evidence for selected-field accounting, not
native disposition authority, nonempty transport laws, dormant-preset coverage
or a complete supplier inventory. It closes none of the five input obligations.

The first capture exceeded the original 8 MiB report bound and exposed an
overstrict final-environment-only validator. The corrected 16 MiB bound retains
every observed call; ancillary calls are validated without pretending they are
in the final census. New refusal tests reject missing transport, unqueried-as-empty
claims, incorrect final-environment identity and raw-only metadata evidence.
No source business method is replaced. This observer is opt-in in the existing
test harness; it introduces no native path, source-specific runtime model or
new import policy.

Evidence: `runs/owned-manual-djinn-admission-source-02.log` and its adjacent
directory's `source-jit-{off,on}.json`; the failed `-01` capture is retained.
The subsequent join of all 799 current source origins to the original XML shows
that manual Sand `215/216`, manual Water `228/229`, generated Sand `208/209` and
generated Water `226/227` already retain only `0503`. No Djinn pair remains on
`01f2`; that issue's ten unaccounted generated rows are Warrior and archived
Firebolt. The new witness therefore supplies evidence for usage review, not an
existing-authority configuration-link retirement.

Existing `inert_fields` declarations are syntax guards, not semantic
non-applicability certificates: compilation reserves the usage fields, and
deferred-usage accounting deliberately retains the same-preset Pending issue.
A bounded field-consumer proof still needs the reviewed Import authority
described in the [configuration proposal](owned-configuration-dispositions-proposal.md).
It must preserve participation, reference-selection and FullDPS responsibilities.
Do not repeat the same source capture, reinterpret a guard as that authority or
infer a complete inventory from unchanged final metrics.

## In progress: incoming critical source proof and Actor read decision

The independent complete-PoB witness now **passes 26 cases under both JIT modes**,
including MAIN/CALCS agreement, a separate fresh replay and a changed-to-original
warm load. These are 54 complete loads. The parent test passed in **332.56s**;
both 1,575,783-byte reports have SHA-256
`f70cf672f6bf4ca39f16c275ddc2839813c2554bb9bd5d695375ace7bccd8d86`.
The [compact source packet](../data/owned/poe2/3887ae68/incoming-critical-effect/source-vectors.json)
retains 52 measured MAIN/CALCS vectors and pins the harness, observer, original
XML, source manifest and full reports. It proves actual flags, numeric operands,
lazy branch behavior, defaults, authored zero/fractional/negative values, extra
damage reduction's upper-only cap, evasion, and overwritten placeholders.
No source method is replaced and no synthetic modifier database is injected.

The source's lowercase `enemyCritChance` override bucket is empty and has no
result in every observed ordinary XML/custom-modifier load. Static acquisition
also found no producer in the pinned supported source paths. This is bounded
evidence, not permission to ignore a future real override mechanic or arbitrary
direct modifier injection. The native draft deliberately has no Lua-shaped
override-presence/value model; it retains a resolved configured-chance dependency.

**The native draft is not published or usable yet.** Its shared Player owner
reads Enemy computed Stats and Boolean queries. Native tests expose an existing
contract restriction: `existing actor read requires unsupported provider
authority`. Shared Actor rules currently admit Enemy external inputs but not
these computed reads. The [Actor-to-Enemy read proposal](owned-actor-enemy-reads-proposal.md)
is awaiting the owner's decision; do not relocate the formula, copy Enemy facts
onto Player Stats, remove the real Actor binding from tests, or change validation
to get around that decision. Support-origin composition was separately accepted
October 8; it does not authorize Actor-to-Enemy reads.

Uncommitted preparatory files are `incoming-critical-effect/{extension,policy,
native-inputs,queries,README}` and the two new root critical test targets and
their helpers. They now provisionally reserve `3353`–`3364` for 18 definitions
(moved after the independent Sand packet's five IDs), preserve Partial
Actor/Encounter ownership, add three Partial flag inventories, and leave six
numeric dependencies unresolved. The normal native target currently has one
passing authoring test and five tests stopped by the explicit authority guard;
the measured replay has not run. Keep those unapproved native drafts out of a
published green checkpoint. No production format, validation or runtime code
was changed in this source-evidence checkpoint.

Resume after the design decision: extend only the approved shared-Actor read
authority and its storage/raw-compiler guards, prove wrong scope/type, missing
producers, Partial membership, stages/cycles and unchanged write boundaries;
then execute the seven actual authored native tests, authenticate the compact
vectors against the full reports, and run the prepared all-five publication
inverse. First rebase the draft's predecessor expectations from the Boolean
package to the current checked baseline; its reserved `3353`–`3364` IDs remain free.
The draft's input policy should add two absence/value channels and
account for exactly two overwritten placeholders per original; **these changes
have not been published or counted as progress in input closure**. Current
package identities, issue counts and 0/5 completion remain unchanged.

Evidence: `runs/owned-critical-source-test-01.log`,
`runs/owned-incoming-critical-source-01/source-jit-{off,on}.json`,
`runs/owned-critical-native-tests-02.log`. Strict workspace/all-feature/all-target
Clippy passed in `runs/owned-critical-clippy-01.log`; it does not establish native
execution. The ordinary critical authoring/source-pin check and the exact compact
vector/full-report authentication pass in `runs/owned-critical-cli-tests-02.log`
and `runs/owned-critical-source-authentication-01.log`. No all-five publication
was attempted for this unapproved draft. At 21:54:56 UTC, `bfa28bb` CI was pending, `303adb2` was cancelled as
pending runs coalesced, and `f93d551` had four successful/eight running jobs with
no failed steps. No new hosted success is claimed.

## Independent progress: default-encounter damage-roll evidence

While the Actor-read and support-origin decisions remain pending, the next
configuration source investigation passed. The optional Rust target
`owned_damage_roll_source` performs twelve cases per JIT mode, including one
extra warm-up load per mode: **26 complete loads in 133.31s**. Ten no-boss cases
cover the original, raw Input zero/100/fractional values, changed/missing numeric
Placeholders, explicit `None`, a separate fresh replay and changed-to-original
reuse. All top-level scalar Player and selected-minion outputs and top-level
availability/type maps match their unchanged baseline within each mode. Nested
output tables are not covered by that equality claim.

The observer traces the original preset callback without replacing methods or
injecting modifier records. The no-boss branch never reads the roll. A pinned
`Shaper Ball` contrast reads 0/100 and changes Cold incoming damage from 4127 to
6190 in both modes. This is a PoB callback contrast, not native admission of a
boss preset or a claim about current PoE2 boss content. MAIN preserves saved
Sniper selection and CALCS preserves saved Arsonist selection; the first local
run failed because our new assertion incorrectly assumed both selected Sniper.
The corrected test changes no source selection and establishes no new PoB bug.

Both 339,448-byte reports have SHA-256
`c20f9c8992fa062f59b42bcc1da77abc17581bf9e1a7133c6fef20251e27d2b6`.
Evidence: `runs/owned-damage-roll-source-02/source-jit-{off,on}.json` and
`runs/owned-damage-roll-test-02.log`. Two evidence-helper regressions, strict
targeted Clippy and formatting passed. To reproduce, build the
`poe-optimizer-pob` test target, set `POE_DAMAGE_ROLL_SOURCE_OUT` to a fresh
directory and run its ignored exact test
`damage_roll_range_is_inactive_without_selected_boss_skill`.

**No Import disposition or native data changed.** The current encounter policy
cannot express the field-to-branch relationship. A hardcoded field exception,
presentation classification or unused synthetic input would violate the intended
data boundary. The [existing configuration proposal](owned-configuration-dispositions-proposal.md#default-encounter-branch-dispositions-2026-10-07-investigation)
records a narrow injected permission for future review. Even after that proof,
only one source leaf would be accounted for; all five selected obligations remain.
Do not prioritize this bookkeeping over a ready numerical consumer.

The approved Sand preparation remains in the current package after this
source-only investigation. It retains Partial contributor/readiness coverage and removes no
selected input obligations. The unapproved incoming-critical packet provisionally
uses `3353`–`3364`, after Sand's five allocations; its source evidence is unchanged.

CI snapshot during this checkpoint: `f93d551` run
[37684118252](https://github.com/Azaril/poe-optimizer/actions/runs/37684118252)
completed successfully with all fourteen jobs passing. Recipient checkpoint
`6a3aae3` run [37715752732](https://github.com/Azaril/poe-optimizer/actions/runs/37715752732)
is now in progress. Earlier queued revisions were coalesced. This checkpoint's
local validation is not hosted success for its eventual commit.

## Checked baseline and original-build results

Use `runs/owned-buff-sources-publication-01/package` and its neighboring
`original-01` through `original-05` imports. Current sidecar contracts are
preserved; definition schema V6, rule schema3 and operationsV24 are independent
contracts. The immediate predecessor is `runs/owned-life-queries-publication-02/package`,
input `c37eea7e87864407494c4b42574e3cfa4521afbdddd32a87096b266acc476b71`.
Historical endpoint paths authenticate stored evidence; retired formats require
explicit offline reauthoring and are not alternate production evaluator modes.

| Identity | Current value |
| --- | --- |
| Source revision | `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` |
| Release input | `7239b9aaf761e5a1c756785ba973e709a06233d6d3e832d8cd32a0f989c700a0` |
| Registry | `8a38b9fa386105c5cefb3f045763f9e46b214c1a11533c5100fd97f643e1adea` |
| Definitions content | `a5a13af506ff524d01db2f0ed4592bf728b8ba93dade938009a7bd1753d44a7a` |
| Rules | `fa5d62760dabaf7be911c4c530bf13d3746ed6dabfd4bce79681cfadcbed6fc4` |
| Compiled rules | `b85c487d8d41ac90dbd52d739910eed9007b05d2e09df8718d0aeb1951453366` |
| Routing | `2b73de7d4720263e6c2d9fc9dffae7ea48ec5889da41a674eb54353ce304be8f` |
| Mapping | `50769317179236c4be8766a07ce570da55203c0c1c2999370006606c9c2c0736` |
| Skill roles | `3006faf9c2938b1adc3fe483dc313896b8ab9f21cd6b193855d3172e00d89f55` |
| Normalization | `2de39f0d9a4d12f01445f932b3754dee8f6dff262b0adbbe1771ed88fdf2e4f1` |
| Rewards | `391f86e6edac5cf65e3e7a2cb705733b6d13eed70d793250e694edc07353e779` |
| Items | `5b706407496bd088bda444230c5d08e348f8ac6d75d7795a58ac257c141044d5` |
| Item source | `5bf5aab130bf55249cae1d9aa35a462b0cc02175e4059896df221d2ff8da11d7` |
| Tree policy | `29d98d0a20978f780e77e198842972bf8225e06309f23d4b5d560c542b3afa3a` |
| Source-scaling authoring commitment | `8efaefde19a9502dbed0ffd19361dd7b105268ad6d3c65bf6054a15abec95d20` |
| Definitions / rules / operations | V6 / V3 / `owned-domain-operations-v24` |

The eighteen package files total **63,903,859 bytes**, with 150 provenance rows.
Definitions and rules use `pob-3887ae68-buff-effect-source-queries-v1`. The registry ends at
`3352`; the latest publication allocates no IDs and adds three programs/four contribution queries.
Mechanics/integration remain Partial, with no evaluation bundle. Authoring stage,
source-property and readiness fragments are not independently complete evaluators.

| Original | Unchanged saved selection | Selected unresolved issues | Native evaluation |
| --- | --- | ---: | --- |
| 01 | Kelari / Sand Djinn, Kelari's Deception | 107 | Not run: Pending |
| 02 | Twister, skill set 6 | 117 | Not run: Pending |
| 03 | Whirling Assault, average-damage mode | 109 | Not run: Pending |
| 04 | Crossbow Shot | 123 | Not run: Pending |
| 05 | Skeletal Sniper, Basic Attack, skill set 4 | 5 | Not run: Pending |

Original05 retains support-origin discovery `01de`, preset usage `0503`,
configuration `01f2`, external assumptions `0207`, and scenario usage `0208`.
Resolving these input obligations will not by itself complete numerical owners.
Crossbow's saved reference selection has no hit-damage output; preserve its
availability result rather than choosing another skill. Original source bytes
and all 110 query identities remain unchanged. Local `runs/` files are
reproducible evidence, not distributed game data.

## Next executable work

The selected-passive, recipient-buff, Gigantic and inherent-Life joins above pass.
The actual Player Life contributor join and independent final-Life consumer
witness now pass. Actor/reward membership is implemented in operations V23;
its actual Life query data and finite reductions now pass. Prove the remaining
equipment reduction domain, then final Player Life arithmetic using the retained source witness.
Publish complete coverage only where supplier domains and numerical bounds are
proved. V24 now provides exact Skill query/self-membership and canonical Offering
source scaling for checked empty domains. The typed activation/application join
now passes in the same graph. The next combined damage consumer requires the
new [application-group query contract](owned-application-group-contribution-queries-proposal.md),
accepted October 8 and now the next implementation step. Composed support
discovery remains an independent accepted path. Application-group and Actor/Skill
membership and support composition need no repeat approval; unrelated proposals
retain their separate review gates.

The follow-up audit found no established final-Life shortcut: the selected saved
Life query is Player, whose BASE/ordering and class/shared-Actor coverage remain
open. Joining the already-published six minion-Life sources would add component
coverage but would not close that metric. Do not replace the current dependency
with another supplied scalar or count that test composition as a complete build.

The checkpoint's input audit found no existing-authority shortcut to close the
other four obligations. `0503` retains twenty Skill/Gem source rows despite 23
known preferences; global switches, action selection and FullDPS responsibilities
remain. `01f2` retains 44 origins (21 Config, 23 elsewhere), including 19
Placeholders and ten unaccounted generated Warrior/Firebolt rows. `0207`'s 17
known assumptions do not prove the constructor/default catalogue complete;
`0208`'s empty saved usage does not account for seven default child-effect enables.
Known scalar values cannot certify these inventories. Configuration-disposition
and reporting-ownership proposals remain decisions, not implemented authority.
In particular, encounter identity alone cannot authorize ignoring
`enemyDamageRollRange`. Continue numerical dependencies and source accounting
without inventing completion or adding scalar-only patches that leave every
selected obligation unchanged.

**Narrow source follow-ups under existing authority:**

- The [focused Offering witness](#source-checkpoint-offering-grouping-and-exact-support-delivery)
  now records original local products/rounding and exact Danse delivery. Use it
  to review owned group membership and support applicability; do not rerun the
  same probes or mistake an inherited source store for native ownership. The
  additional-skeleton activation condition remains unresolved. Do not widen
  native Skill or support-origin authority before the accepted contracts are implemented.
- The [manual Djinn admission witness](#source-checkpoint-manual-djinn-admission-accounting)
  now records the actual Original05 global2 controls through original consumer
  admission. Next use its exact queried-empty versus not-called distinction in
  the `0503` disposition review. All these Djinn rows already have usage-only
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
2. In parallel, account for selected preset usage (`0503`), configuration (`01f2`),
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

**Archived Warrior preflight (2026-10-08):** eight remaining `01f2` source links
belong to four item-granted Warrior pairs in presets 3/5/6/1. Exact generated,
usage and support Pending owners already exist, but the current selector adapter
requires either physical Gem supply or a manually selectable root. Warrior is
item-only; private adapter reuse would invent that authority. No data was
authored, the exploratory private reuse was removed, and the count remains 44.
The [selector/supply separation proposal](owned-source-action-root-separation-proposal.md)
records the structural boundary, the genuine reusable child action and the
conditional 44-to-36 accounting target. This is a pending Import decision;
do not add a fake physical Gem or weaken root checks. The immediate native
priority is Life-consumer adoption of the implemented Actor/reward graph,
followed by the accepted exact Skill query extension.

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
   a separate pending design decision.

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
   Keep the -25 contributor available while the ownership decision below is
   pending; migrate or deliver it only under the reviewed resource contract.

   The [resource-obligation proposal](owned-resource-obligations-proposal.md)
   recommends exact Skill-owned sustained reservation with explicit Action
   dependencies where a mechanic needs a selected part/mode/stat set. It also
   identifies the missing Skill support-recipient contract. This is pending
   owner review, not an implemented migration or authorization to manufacture
   Action demand. After approval, validate contrasting reservation/cost shapes,
   migrate the existing live consumer, and prove query-independent results.

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

The remaining enemy critical chance, critical bonus and attack/cast-time inputs
can reuse the checked numeric-override seam. Source `ConfigOptions.lua:1984–1986`
resets their placeholders, and boss selection at `2235–2240` can overwrite them.
They therefore need injected encounter defaults and explicit override presence,
not saved-placeholder fallback. Before delivery, test stale placeholders, zero,
fractional values and boss changes against the actual consumers at
`CalcDefence.lua:2268`, `2273` and `3412`. This is input work, not completed EHP or
configuration-inventory coverage; no public model change is currently needed.

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
2026-10-05. Accepted direction, implemented components, pending integration and
unresolved design contracts are distinct states. The [retirement inventory](legacy-retirement.md)
remains the single cleanup register; the historical checkpoint text in other
documents must not reopen accepted decisions or promote unvalidated work.

| Work | State and next action |
| --- | --- |
| Shared-contract generality | Initial review complete. Before the next public extension, exercise contrasting authored/generated sources, exact repeated occurrences and applicable stacking/receiver shapes. [Generality gates](build-generality-review.md) remain open. |
| Generated source-property owners | Accepted contract implemented: exact generated Skills own properties and their exact declaring provider can project final inputs. Current native contract tests and component regressions pass. Sand incoming source evidence is captured; real numerical authoring remains downstream of Original05 request admission. Ownership support does not establish complete mechanic coverage. |
| Effect-application breadth | General application model accepted and Maximum component implemented. Select a real contrasting stacking/grouping case and prove duplicate, cap, source and recipient semantics before extending its bounded policies. Do not replace the shared graph with effect-specific paths. |
| Physical-input/reference separation | Design refinement pending: classify fields without requiring successful MAIN/CALCS selection or a live Pending usage issue to preserve known intrinsic facts. Preserve independent unknown-field/usage obligations. |
| Generated selector/supply separation | [Proposal](owned-source-action-root-separation-proposal.md) pending. Archived item-only Warrior exposes physical/manual root coupling in the source-selector adapter. Separate selector interpretation from checked supply, reuse existing child topology, and preserve all unresolved owners. No 44-to-36 source-link reduction or native authority is published. |
| Immutable normalization preparation | Compile existing checked policies once per exact release; separate construction/traversal budgets and prove identical outputs. Implementation pending; no storage-layer change. |
| Scoped numerical coverage | [Proposal](owned-coverage.md) pending concrete reviewed proof contract. Unknown effect reach remains blocking; no known-edge-only pruning or relaxation of original-build acceptance. |
| Closest complete build and native optimization | Original05 is the current first-completion target. Close its four remaining input obligations, publish its real evaluation bundle and complete its selected mechanics/metrics through the public CLI. D4 begins after this first complete build, without waiting for all five: bind objectives to owned actor/action/stat-set requests and verify one legal locked candidate mutation before expanding search. Reassess the shortest complete path at every checkpoint. |
| Saved allocations and effective connectivity | The retained Sniper removal95 control keeps8737 in the saved tree, while PoB prunes8737 from its active set. Current Import preserves that occurrence with Pending allocation access; it does not silently repair the build. Preserve this refusal and test explicit candidate repairs separately. Before admitting passive-tree optimizer mutations, validate connectivity against injected roots, pools, scopes and exceptional access after each mutation; import-time path proof and legacy candidate checks are not a native post-mutation legality gate. |
| Independent breadth and CI | Add provenance-bound holdouts by mechanic intersections and an explicit reproducible integration subset. Existing five examples are development cases. Ordinary CI now executes the current accuracy rule laws, but the joined Sniper graph and its import/membership/staging checks are ignored without their published package. Provision the exact current package and retained source controls reproducibly in CI, run that joined subset explicitly, and fail on missing artifacts; do not treat component tests as an equivalent gate. |
| Historical replay retirement | Reimport with the one current implementation and retain useful stored source evidence. Seventeen older ignored publication harnesses need consumer review: rebase useful assertions to current semantics or remove obsolete scaffolding, rather than add compatibility paths. Declaration integrity and source parity remain required. [Retirement/migration inventory](legacy-retirement.md#historical-publication-replay-migration-2026-10-05). |
| Generated usage | Versioned preferences, all-record data-aware proofs and explicit applicability diagnostics pass focused validation. Shared exact-provider proof now serves raw quality and requested-count import independently. Same-definition allocations in different presets remain distinct. Enabled/global/reporting dispositions and usage inventory closure remain open. [Contract and gates](owned-generated-skill-usage-proposal.md). |
| Requested participation | [Common gate](owned-skill-participation-proposal.md), unified typed Import and first Sniper packet are validated. All-five/source/native checks preserve count, effect controls and mechanical availability. Next close genuine parameter inventories for remaining skills before readiness publication; no PoB preview bypass or default is imported. Full-original coverage remains open. |
| Resource ownership and demand | [Resource-obligation proposal](owned-resource-obligations-proposal.md) awaits owner review. Ordinary sustained reservation should be independent of queries, with exact Skill/payer/resource identity and explicit Action dependencies where required. Define Skill support recipients and contrast cost conversion, mines and stance/toggle selection; then migrate the current Action reservation bodies and validate query invariance. Gigantic's producer/status and isolated Life/Damage-factor packets are published; final reservation delivery remains open. |
| Generated raw inputs | Accepted exact preset bindings now have V6 permissions, V19 producers, source-bound quality joins and checked publication for the reviewed Djinn/Firebolt families. Existing provider levels survive. Archived cross-axis correspondence, other generated families and final mechanics remain open. [Contract and gates](owned-generated-skill-inputs-proposal.md). |
| Generated source-format breadth | Replace finite Item-name frames with proved general derivation over admitted layouts, and add evidence for normalized-only level correspondence. Census competing saved representations before scalar admission. Original04's Firebolt provider still has unresolved rune/layout/grant-line authority; raw quality cannot bypass it. [Bounded adapter follow-up](legacy-retirement.md#bounded-generated-source-import-coverage-2026-10-05). |
| Typed Boolean contributions | Accepted 2026-10-07: add typed Boolean contributions and unordered Any to the existing occurrence/query graph. Separate complete membership from numeric ordering, preserve duplicate source identity and unknown propagation, and migrate actual inherent-attribute flag producers/consumers first. This is also a prerequisite for later critical-hit flags; Enemy recipient authority must be explicit. Core/Data/Engine schema3/operations22 are implemented; real halving/doubling passives and five reducers are published. Their five bounded membership groups are now Complete; global query coverage, donor owners and unrelated passive mechanics remain Partial. No false default or numeric stand-in. [Accepted contract](owned-boolean-contributions-proposal.md). |
| Actor/reward contribution queries | Accepted October 8; Core/Data/Engine membership and canonical Life query data use operations V23. Eight exact writers populate seven groups; six zero/one-effect domains are bounded by semantic tie rejection. Equipment ordering, global/owner coverage and the final Life consumer remain open. [Contract](owned-actor-reward-contribution-queries-proposal.md). |
| Composed support discovery | Accepted October 8. The common native gate, current authored-order DTO and source-input accounting are implemented; Original05 `01de` is retired. The Gem packet publishes 937 Known and 29 Unmapped domains with all-five preservation; all 23 selected physical Gem domains are covered. Existing typed identities fixed 111 false stat-set blockers. Pinned generated export metadata classifies eight missing runtime effects as active without implementing their Command mechanics. Continue other owner domains and exact provider exclusions; keep Command action/usage gaps separate. Unsupported positive and archived source relationships remain blocking. [Contract](owned-support-origin-composition-proposal.md). |
| Exact Skill contribution queries | Accepted October 8; Core/Data/Engine V24 implements Current Skill reads and Skill-owned self-contributions with explicit direct-use and/or supplied-slot permission. Exact supply relations survive late support checks. Canonical Offering source writers use four checked empty domains and their activation/application join passes. Inherited Actor and support-delivery memberships remain pending; nonempty game-group meaning requires proof. Actor recipient writers also use guarded empty queries. [Accepted contract](owned-skill-contribution-queries-proposal.md). |
| Application-group contribution queries | Accepted and implemented October 8 in Core/Data/Engine V25. Typed program-effect/application-group addresses preserve exact recipients, potential-source census, stacking, unknowns and explicit numeric order in one checked reduction path. Current artifacts are rebuilt with no old-format compatibility. Real mixed damage-consumer data adoption remains next. [Accepted contract](owned-application-group-contribution-queries-proposal.md). |
| Shared Actor reads of Enemy results | Proposed after the incoming-critical native draft exposed the existing same-Actor read restriction. Allow resolved Enemy Stats and checked Boolean Flag queries, while preserving Actor-only writes and refusing numeric Enemy queries. No new runtime path or compatibility branch. Source controls pass, but the authority change and native execution remain pending the owner's decision. [Proposal](owned-actor-enemy-reads-proposal.md). |
| Configuration/source dispositions | Presentation and exact item-range ownership are implemented; full semantic closure is not. Original05 issue `01f2` retains 21 Config-local and 23 non-Config origins after selected/generated accounting, cached-Buffs, archived responsibility and overwritten-placeholder proofs. Fourteen archived rows retain their exact existing raw-input, usage and support-origin Pending owners; they gain no provider or known value. Account for generated-group fields and remaining actual consumers before retirement; partial scalar success or local Config proof cannot clear global fallback. Typed constructor defaults now reach Player/Enemy/Environment inputs; the real Player resistance-penalty consumer is published, but no global inventory closes. Keep scenario usage/assumptions separate and retire redundant configuration variants through the [consolidation follow-up](legacy-retirement.md#configuration-projection-consolidation-2026-10-07). [Evidence and gates](owned-configuration-dispositions-proposal.md). |
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
| CI regression, timeout and infrastructure | At 23:26 UTC on 2026-10-07, [37646478601](https://github.com/Azaril/poe-optimizer/actions/runs/37646478601) at `47f3560` had passed all fourteen jobs. [37684118252](https://github.com/Azaril/poe-optimizer/actions/runs/37684118252) at `f93d551` had seven successful/five active matrix jobs and no failed steps; Sand checkpoint [37702132448](https://github.com/Azaril/poe-optimizer/actions/runs/37702132448) was pending. The preceding successful Windows native CLI step took 314 minutes: separately profile/shard that critical path while retaining coverage. Job logs require authenticated access, so current compilation versus test progress is unverified. Pending revisions may coalesce; duration alone does not establish a hung test. No hosted success for the latest checkpoint is claimed. |
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
   After changing a current bundle or transition, regenerate its dependent
   provenance receipts and run their strict reproduction checks, even when the
   dependent numerical artifact is unchanged. A rule-admission change also needs
   the downstream test suites that construct those rules; Clippy alone cannot
   verify which boundary rejects a malformed fixture.
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
