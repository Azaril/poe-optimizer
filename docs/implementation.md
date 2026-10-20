# Implementation plan and resume point

Updated: 2026-10-20 (EDT).

This is the active delivery plan. The [design](domain-architecture.md) defines
the end state; the [execution overview](data-and-evaluation-overview.md) explains
what actually runs. Older checkpoint reports are preserved in
[implementation history](implementation-history.md), not current instructions.

## Current state

**Current Git backup:** the October 20 checkpoint following `f2680094` adds
the tested usage work-accounting correction, regenerated allocation provenance,
reviewed historical replay fixture repair and these resume notes. It is a
work-in-progress backup, not a fully validated successor release. The previous
commit's full remote hash was verified before this backup; the current commit
and push receipt are available in Git history. Ignored candidate/evidence files
and the nine protected incoming-critical files remain local.

**Validated data baseline:**
`runs/owned-preset-usage-inventory-publication-05/package`, input
`e8df8bdbb4842c637fa18f822f25b32efd332753842a61f464181af2211e062d`.
The 25-row packet preserves all five originals and all 110 queries. Earlier
count/support/skeletal controls and strict CLI/PoB lint pass. No selected
Original05 input obligation has retired; the current five obligations and
remaining native mechanics are detailed below.

**Latest validation:** the Direct adapter and usage work-accounting correction
pass 687 selected Import tests and strict all-target Import lint
(`runs/owned-usage-actual-work-tests-02.log` and
`runs/owned-usage-actual-work-clippy-01.log`). Earlier exact all-five preservation
also passes. The two-row
Direct inventory overlay passes five ordinary checks. All five native movement
controls pass, including unavailable Partial contributors and fresh/reused/Rayon
equivalence; thirteen related campaign/critical checks pass. The three movement
source tests pass, with identical supplier evidence across four fresh JIT-on/off
runs. Source behavior and native component results do not establish complete
movement coverage.

**Fresh Direct publication:** attempt `07` passes all-five unchanged imports,
all 110 queries, whole inverses and deterministic independent publication/import
replays (`runs/owned-manual-direct-publication-08.log`). Its package is
`runs/owned-preset-usage-inventory-publication-07/package`, input
`32a6f01b04397ce857af5101e2a9e9f4fa40ed847ef4ea8733e9ae770186f53a`,
release SHA-256 `14ba4405c7ebfd6f56514cd0f2da627d25ae8a4c15a3e500b153715da7cf28bc`.
Selected issue counts remain 107/118/110/124/5; none retired. The real-group run
passes Manual Direct, primary counts and skeletal controls, but Ice Nova and
support expansion fail the same pinned hash assertion at
`tests/support/owned_preset_usage_inventory_positive.rs:38`
(`runs/owned-manual-direct-real-controls-01.log`). Keep the earlier validated
baseline until these two failures are diagnosed and their controls pass.

**Resume in this order:** diagnose those two exact fixture hash failures.
The temporary diagnostic pinpointed the unchanged
Original05 work-limit failure in attempt `06`; the fix charges actual retained
parameter/destination lengths and avoids repeated whole-binding comparisons.
The original two-million limit, full equality/duplicate checks and atomic
retirement remain intact. The diagnostic edits are removed.
Rerun the historical extension test after the applied item-defence supplied-policy
fixture repair (`runs/owned-extension-replay-03.log` identified the stale schema
binding after the earlier membership repair). It authenticates the whole historical
recipe and both old policies before changing only transitive commitments; production
validation is unchanged. Finish the shared-helper/lint gates
before authoring the movement release against the validated successor. Attempt
`06` is retained as failed evidence, not a release. The configuration inventory
and movement authoring candidates remain unapplied/unpublished; their evidence,
remaining prerequisites and selected-mechanics follow-up are recorded below.
No hosted-green or complete-build claim is made.

Earlier October 20 backup narratives are preserved in
[implementation history](implementation-history.md#archived-2026-10-20-backup-and-validation-notes).

**Five supplied builds: 0/5 complete native evaluations.** All five import and
run in pinned PoB. Owned native components work, but unresolved inputs and
mechanics still prevent complete native requests. Component parity, catalog size
and deleted code are not substitutes for this gate.

**Delivery priority, confirmed 2026-10-07: finish the closest complete build
first.** Original05 (Skeletal Sniper / Basic Attack, skill set 4) is the current
target: it has five selected input obligations and the most developed relevant
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

An October 19 progression audit found an admission gap to verify before that gate:
Requirement effects can calculate false without making the generic effects report
itself reject the candidate. Complete evaluation/search admission must consume
every selected requirement, distinguish invalid from unavailable, and retain
incomplete requirement coverage. Numeric reward authority tests prove detection;
they cannot alone establish enforcement. Reuse one generic admission seam rather
than adding a progression-only validator. The
[requirement-assessment proposal](owned-requirement-assessment-proposal.md) was
accepted October 19: this evidence now accompanies each native metric report from
the same attempt. Ordinary/support regression tests and strict lint checks pass.
The report preserves known violations alongside incomplete validation
and independently computable measurements. It assesses injected requirements,
not complete game legality; the future owned search adapter must consume it.
Some Action contexts currently exist only when queried. Potential Action Requirement
families without an independent mechanical context census must remain incomplete
regardless of which metrics instantiate them; known effect rows alone cannot close
that coverage. Complete Action census is an explicit follow-up, not part of a
new Action registry or implicit all-Action expansion in this checkpoint.

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

### Structural checkpoint: one Actor model

The owner approved [generic Actor identity and local rules](owned-existing-enemy-rule-ownership-proposal.md)
on October 15. The migration implements the structural dependency for shared
conditions, outgoing critical calculations and Enemy mitigation. It does not add complete
build coverage or change the closest-build priority.

1. Completed: freeze the independent Evasion override publication on the existing
   format. Slot aggregation and override selection are committed and validated.
2. Completed: migrate Core/Data identities, role-specific inputs, shared applicability and
   exact selected Encounter membership. In parallel migrate Engine contexts,
   invocation/value keys and checked query binding. Keep role authority explicit
   over whole inventories and remove all placeholder Player behavior.
3. Completed: migrate Import/CLI and current authored artifacts to the one new contract.
   Rebuild generated packages and portable fixtures, invalidating old hashes.
   Preserve original PoB archives/observations. No runtime compatibility branch.
4. Completed: validate one identical local rule on Player, two supplied occurrences and
   ScenarioEnemy, with independent inputs and provenance; prove unauthorized
   reads/writes, unknown/inactive/unlisted/late sources and obsolete aliases fail.
   Preserve existing numerical controls and fresh/reused/Rayon determinism.
5. Completed: reimport all five originals and retain their exact queries and
   coverage gaps; pass the final CLI/lint gates and validate the new endpoint.
   Resume the closest real calculation
   blocker: shared conditions and configured Enemy contributions feeding critical
   damage/mitigation, alongside final Player defence admission.

Current resume point: the Actor structural migration, full ordinary outgoing-critical
component, configured Weakness activation, separate normal/critical Armour
consumers and joined ordinary physical-DPS component are implemented and published
locally. They pass finite native
execution and all-five preservation, with every production query/owner gap
unchanged. The accepted numeric Choice-reward adapter and exact source accounting
are implemented and pass the focused Import gates. The actual campaign-progression
data publication and all-five preservation now pass. Native Scenario usage intent
is implemented; the importer and exact accounting for configuration intents pass
focused tests and all-five preservation. Source-backed structural discovery and actual control
publication follow that transport checkpoint. Four structural publications are
validated locally: fifteen Gem potential-Skill inventories, direct declarations
on those fifteen Gems, four item templates and six passive definitions now
account for 90 reviewed complete inventories, with all numerical and descendant
gaps preserved. The six remaining passive candidates require missing
buff payloads or Command execution evidence.
Candidate-bound source and special-branch admission remains blocking, as detailed
below; no complete build is claimed.
The latest validated Import package is
`runs/owned-preset-usage-inventory-publication-05/package` (`e8df8bdb…`), built
from the construction cutover `b3b7c381…`. It preserves all five original outputs
except eight exact added count preferences, with all 110 queries and every other
draft/sidecar/report value unchanged. The [checkpoint below](#source-construction-cutover-checkpoint-october-20)
records the source-construction migration and subsequent usage gates.
Existing support/generated-input proofs feed the whole-preset usage gate.
The physical-singleton secondary-switch and group-selector adapters pass their
scoped tests; the injected support inventory and companion data pass real-source
positive and refusal controls. The primary-completion cleanup also passes its
producer/liveness tests and exact comparison with saved pre-change outputs.
The Pain Offering/Frost Bomb count data passes its full scoped gates.
The four skeletal rows now also pass publication and all 31 real-group controls.
Continue with authored Direct Djinn selectors/absent manual source, then generated
routing and the remaining configuration/default accounting. The generated
reverse-catalog proposal remains a separate unresolved decision.
This does not close numerical coverage or any original's inventory.
At the Actor migration checkpoint, the package and portable replay rebuilt
successfully. Exact inverses authenticated the prior artifacts and allowed only
declared format/role edits, checked dependency commitments and one provenance row.
All five originals retained identical drafts after canonical Enemy-address and
lineage conversion, all 110 queries, selections and the then-current issue counts
**106/117/109/123/4**. The later Scenario-intent format added one Pending inventory
per selected Scenario; current selected counts are **107/118/110/124/5**. No
coverage gap was closed. The scoped regression and strict Clippy gates passed.
Core's existing 312 tests and 25 focused role/wire checks pass. Data's 205
focused checks and strict Core/Data all-target Clippy pass. Engine's 718 tests
pass across the library and 73 integration targets, including all 45 portable
Sniper replay tests; seven explicit offline tests remain intentionally ignored.
Strict Engine Clippy passes for that scope. Import's 191 library, 357
normalization, seven recipe, five catalog reproduction, 16 current migration
and ten evaluation tests pass.
Obsolete format commitments and compatibility-only assertions were corrected;
dropping existing evaluation artifacts remains a tested refusal.
Existing offline exporter suites pass (22/16/18/16/14 tests). These counts
describe the migration gates, not build coverage.
The 31 selected CLI regression targets and the full publication-chain target
pass. Strict Import all-target and the 32 selected CLI-target Clippy checks pass;
the chain's final helper corrections were linted again after its successful run.
Final logs: `runs/generic-actor-catalog-recipe-tests-01.log` and
`runs/generic-actor-extension-clippy-01.log`. Hosted CI is tracked separately below.
The owned-library dependency/compiled-source boundary check also passes without
Lua, PoB or Import in either owned Data or owned Engine. The CLI publication
chain exposed stale intermediate item-policy dependencies. Current typed loaders
rebuilt 51 exact dependency fields across 38 authoring files, including source
policies, compiler metadata, membership patches, gem normalization inputs and
derived item-layout bindings.
Authenticated prior snapshots and full inverses preserve every other field and
all source observations. The full ordinary CLI publication chain passes: two
tests, one intentional offline ignore, 746.72 seconds. Its 41 shared helpers
exercise the intermediate publications through selected Actions and supplied
Actor abilities. Log: `runs/generic-actor-extension-tests-04.log`. One prior
attempt failed early with Windows access denied; no numerical
retry policy or weaker validation was introduced.
Proofs: `runs/generic-actor-item-policy-final-audit-01.json` and
`runs/generic-actor-policy-final-reference-audit-02.json`. Four filtered
authoring checks pass and their temporary Cargo targets have been removed.
The final audit finds no stale operational dependency references; retained old
hashes identify historical evidence only.
The [rebuild follow-up](legacy-retirement.md#generic-actor-cutover-and-rebuild-follow-up-2026-10-16)
tracks reproducible regeneration of intermediate dependency identities. It is
offline tooling debt; after this checkpoint, return to the closest-build
calculation rather than starting another storage or runtime architecture.
The current schema is 7, rule package is 4, and full operations are V28. Existing
operation capability subsets use the new identity semantics. Current migration
authoring retains one wire-5 shape; historical schema-upgrade branches and their
compatibility-only tests were removed while declaration changes, allocation
checks, capability upgrades and evaluation preservation remain tested.
Current authored descriptors, bootstrap packages and the portable fixture are
rebuilt. Temporary authoring scripts remain ignored evidence in `runs`; their
Cargo hooks/targets have been removed. No runtime old-format reader was added.
Existing Player final receivers remain unchanged; unsupported ScenarioEnemy
usage/support/Action paths must reject explicitly. Scenario usage-intent
import/accounting is implemented and tested alongside native intent resolution,
progression import and component assessment. Resource-obligation migration,
source-backed structural census and actual stage-control publication remain open.

### Saved output contract

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
| 02, 18, 19: Player ES, Armour, Evasion | Selected item magnitude/local-defence intermediates, global/passive contributions and guarded final Evasion component | Final item properties, complete receiving, transformations and admission of final Actor totals |
| 03–06: four capped Player resistances | Item/passive/reward inputs and configured penalty | Complete contributor inventories, aggregation and cap/override calculations |
| 07–12: total EHP and five maximum hits | Encounter and incoming-damage components | Complete resource/defence dependencies, mitigation and final survivability calculations |
| 14: exact Sniper Basic Attack hit DPS | Prepared skill, intrinsic damage, accuracy, inherited/Offering/Gigantic/quality factors, exact-Action Command, selected physical-hit range, ordinary attack rate, finite outgoing-critical consumer, separate normal/critical Armour and internal ordinary physical DPS | Complete source and mechanic coverage, other damage types and mitigation, whole-plan admission and final DPS binding |
| 13, 15, 16: Player hit DPS/average hit and Sniper average hit | Exact typed query identities | Reviewed unavailable results matching the reference; no fabricated numerical defaults |
| 17: Player Spirit | Actual Amulet/reward inputs, explicit Amulet-copy contribution and guarded final Player consumer | Supplier, suppression and numeric-order admission plus complete-plan binding; unreserved Spirit and legality are separate |
| 20, 21: Player movement/action speed | Movement contribution channel and typed metric declarations | Complete contributors, interactions and final calculations; observed action speed 100 is not a native constant |

Five selected import obligations remain: configuration, preset usage, external
assumptions, exact Scenario usage and the additional Scenario-intent inventory.
Their current exact IDs are **01f2**, **0507**, **0208**, **0209** and **020a**,
respectively. Historical campaign references to preset usage **0506** predate
the format rebuild; that current ID identifies a SkillUse. Format changes
preserve exact correspondence at publication. No evaluation
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

## Current checkpoint: configuration admission

Latest validated local package:
**`runs/owned-scenario-stage-inputs-publication-01/package`**, input
`a0176def7e3666006faa1b95f08709e81b745e15318024fe63dc147aeeb1693f`.
Its release manifest SHA-256 is
`238286b791a2657f63f8d43e51b49e16dc88af4e6e30ca4846c65cf38a5b35c1`;
definition schema SHA-256 is
`6d4c06bb3311c9b4fddc0c2a7f7f82bc3bcb1dcd6b6605249f0d2b69cbd61ddb`.
It retains all 110 queries and selected input counts **107/118/110/124/5**.
The next checkpoint targets complete accounting and admission of Original05's
selected preset-usage inventory. Structural census, complete Scenario applicability and
configuration accounting remain separate blockers below. Complete builds remain **0/5**.

### Configuration-to-intent checkpoint and CI repair, October 20

The existing usage-import policy now has Scenario projection rows; a shared
numeric selector supplies both Reward and usage adapters. Configuration values
stay on their own Scenario, and native preparation remains responsible for exact
recipients. Whole-policy projection is atomic: a missing or invalid parameter
retains the unresolved inventory and source issues. Exact materialized-output
receipts join the existing configuration accounting owner. The current policy is
updated in place with required Scenario rows and an exact usage-policy dependency;
there is no old-format reader or parallel accounting path.

Review identified a concrete shared-leaf risk in scalar destination accounting:
an unresolved scalar projection can have no output link while a usage projection
succeeds. The existing scalar completeness check now covers every declared matching
projection. Automatic raw-placeholder retirement also defers when a declared
usage projection remains responsible for that exact selector. Both paths have
passing regressions; neither relies only on successfully emitted outputs.

All **627 targeted tests pass**: 193 Import library, 371 normalization, 17 Reward,
17 release revision and 29 successor tests. The one-off format rebuild also
passes and its temporary library hook has been removed. Configuration-format predecessor:
**`runs/owned-scenario-config-format-publication-01/package`**, input
`091556fee21e868c21ac3c2a61390c3e90c5900ab4576552c90a46a9fcc63ffd`,
receipt SHA-256
`cbe625eb5c24724657d3688a92ac8222d7ecbd011ba92ed7d0a65cb4cb85813a`.
All five whole drafts are byte-identical to the prior checkpoint, each fresh
repeat agrees, and sidecar inverses allow only policy/tree-policy identities.
Fifteen artifacts, including all 110 query rows and numerical data, are
byte-identical. Selected input obligations and complete-build status remain
unchanged. This rebuild adds empty configuration projection rows; it does not
claim the actual Sigil/Cry controls are supported.

Evidence: `runs/scenario-config-import-tests-03.log`,
`runs/scenario-config-format-authoring-01.log` and the publication's
`format-proof.json`. Formatting, diff checks and strict Import lint pass.
The native library boundary check also passes, retaining no Lua, PoB or Import
dependency in owned Data/Engine (`runs/scenario-config-owned-boundaries-01.log`).
The repaired CLI regression suites pass locally; hosted verification remains
pending publication.

Parallel CI investigation targets remote run `38853276908` at `24d12cd8`.
The previous run `38843640195` at `accd5040` passed all 20 jobs. The failed run has
18 failing CLI targets and three PoB targets; public annotations identify the
shared historical Enemy decoder failure in supplier evidence. The first local CLI
batch then exposed historical normalization being decoded as the current policy,
four stale operation-version assertions and a stale migration-version assertion.
The evidence helper now authenticates original policy bytes without executing or
upgrading them; a regression rejects altered policies, queries and commitments.
Current-contract assertions use the maintained contract while historical Life
proof metadata remains pinned to V27. The eighteen-target rerun passes thirteen;
the other five exposed stale downstream provenance after the Scenario-format
migration. The underlying construction proof is already current. The first
repair's exact byte inverses authenticate 23 files / 40 SHA leaves; ten of twelve
affected proof/component suites then pass. Two further reconstructed reviews
expose only one selected-membership SHA each, including a nested child-lifecycle
commitment. The residual repair and its transitive references have a separate
21-file / 29-leaf proof. No source observations, numbers, formulas or coverage
change. Strict lint and all nine final regression suites pass:
**115 tests, zero failures**, with eight existing explicit offline authoring
tests ignored. Together, batches 02–04 validate all eighteen formerly failing CLI
targets and seven downstream component targets with all features. No tests or
assertions were removed to obtain these results. Final evidence:
`runs/ci-38853276908-cli-repaired-04.log`.
Evidence: `runs/ci-minion-review-rebind/proof.json`,
`runs/ci-review-residuals/proof.json` and
`runs/ci-38853276908-residual-clippy-01.log`. The
[existing rebuild follow-up](legacy-retirement.md#generic-actor-cutover-and-rebuild-follow-up-2026-10-16)
now explicitly covers nested review commitments and complete reconstruction.
All three formerly
failing PoB targets now pass: ten tests, including byte-identical fresh JIT-on/off
source observations. Two component fixtures now declare their actual empty
inventories; the third repair changes only mapping-provenance and observation
pins, with whole-record/byte inverses proving unchanged numerical evidence.
Strict PoB lint passes. Evidence:
`runs/ci-38853276908-pob-repaired-02.log`,
`runs/ci-38853276908-pob-clippy-01.log` and
`runs/ci-intrinsic-minion-{mapping-provenance,source-rebinding}-proof.json`.
A bounded early failure diagnostic window is added because the existing terminal
tail drops earlier failures in long batches; all seven CI-dispatch/annotation
tests pass, including the three new diagnostics regressions. The minion-source,
usage and occurrence-count fixture suites also pass. Strict lint passes for all
twenty-two selected CLI targets in both all-features and native-only profiles
(`runs/ci-38853276908-cli-{clippy,native-clippy}-01.log`).
Full GitHub logs require a signed-in session;
public API/UI checks remain available. Do not retrieve keys or credentials.

A follow-up fixture audit found the same missing explicit inventories in the
formerly optional numeric-modifier oracle. Its isolated fixture is corrected.
All five tests now run ordinarily in the PoB package, using the same pinned
source and local VM setup as existing routine oracle tests. They pass in 1.73s
with zero ignored, as does strict lint
(`runs/ci-modifier-value-oracle-ordinary{,-clippy}-01.log`).
The historical physical
usage cutover is a separate retirement item: its old policy construction cannot
replay the current format. Preserve its original evidence and retire it with the
remaining historical acquisition chain, rather than adding runtime compatibility.
See [the retirement inventory](legacy-retirement.md#current-physical-usage-transport-cleanup-2026-10-06).

The current build-admission checkpoint is a source-backed **structural inventory
census**, separate from numerical coverage. Original05's saved/currently recorded
skills contain neither Sigil nor Corrupting Cry, but that observation alone cannot
prove their Scenario intents dormant. The initial audit found open structural
inventories on 19 Gem definitions, four item templates, twelve passive owners,
five supplied Actors and 22 reachable Skills. The four completed publications
below close potential-Skill lists and reviewed direct declarations on fifteen
Gems, all four templates and six passive owners. These counts describe
this audit's selected scope, not a
production special case or numerical completion.

Start with the fifteen selected Gem families whose potential effect membership
is resolved: twelve singleton supports, Frost Bomb, Pain Offering and Ice Nova.
Four minion Gems instead retain declared command references missing from the
pinned source lookup; do not turn that omission into Complete gameplay coverage.
Two Djinn-related passive nodes also say “Grants Skill” while their abilities
appear under supplied Actors and no modifier is parsed. Resolve the actual
ownership/gating evidence before closing those nodes.

Reuse the authenticated skill identities, `self-flat-selected-bindings`,
`self-flat-attack-damage/selected-domain.json`, Djinn/Actor ability witnesses and
Firebolt's exact item grant. The offline proof must account for full profile
skill lists, `ExtraSkill`, `ExtraMinionSkill`, child factories and exact provider
relations, rejecting missing references and unreviewed branches. Publish ordinary
existing `skills`, `grants`, `actors` and `skill_grants` declarations only.
Preserve Partial numerical programs, parameters, outputs and contribution queries.
This needs no new runtime model or build-name routing. Real Sigil/Cry policies
still require their positive grant/supply applicability; an empty Complete list
would not be a valid shortcut.

The initial source-review handoff is
`runs/resolved-gem-structural-closure-plan-20261020.{md,json}`. It authenticates
the latest release, all seventeen artifacts, twenty source-file pins and both
existing JIT witnesses. Fifteen families span 132 physical Gem records across
fourteen original/preset pairs. Existing evidence establishes their fifteen
potential-skill inventories from PoB; that alone does not establish every gameplay
factory. The follow-up
`runs/resolved-gem-structural-closure-audit-20261020.{md,json}` examines the
forty-five proposed direct grant/Actor/supply fields. Twenty-four fields across
eight fully mapped support families have bounded direct-owner negative evidence,
still requiring the offline proof checker. Twenty-one fields across the other
seven families remain conditional: their thirty-eight null PoB stat mappings do
not establish the gameplay meaning or ownership of those stats. Six explicit
primary grant/supply links are verified members, not exhaustive topology proof.
Pain Offering's described bone-spike Minion remains an unresolved descendant on
its Skill even though PoB has no minion profile. The audit also accounts for
support `addMinionList`, which the older retained metadata omitted.
The eventual patch must change only reviewed completion fields, with no new IDs,
members or numerical programs. Neither trace absence nor unmapped source data
can justify a Complete inventory.

The subsequent scoped description review is
`runs/resolved-gem-null-mapping-semantics-20261020.{md,json}`. Authenticated
game-description bindings establish existing-recipient behavior for the remaining
Encroaching Ground, Last Gasp and Magnified Area I rows. This expands the smallest
candidate proof cohort to eleven support families and thirty-three direct fields.
Twenty-six rows outside that cohort remain semantically unresolved,
and three others describe effect creation or population limits. Description scope
matters: text attached to the same stat ID under another skill cannot prove its
meaning here.

The complementary item handoff is
`runs/item-structural-closure-plan-20261020.{md,json}`. It authenticates all
seventeen package artifacts, ten source files and four matching JIT witness
pairs. Twelve candidate closures concern only the direct intrinsic grants,
Actors and skill supplies on the four selected item templates. Ashen Staff's
Firebolt remains on its exact modifier-owned grant chain; duplicating that grant
on the template would be wrong. Socket, modifier and numerical coverage
stay open, including Original04's independent pending item inputs.

The completed bounded implementation combines
`support-structural-inventories` and `item-structural-inventories`. Eleven support
families span **114 physical Gem records across fourteen original/preset pairs**;
the earlier 132 count describes the larger fifteen-family review. Independent
review also establishes that these eleven Gems have no potential standalone
Skills: their exact winning catalog records contain one primary support effect,
no declared/constructed/resolved additional effects, and no loader-generated
auxiliaries. Full loader and factory boundaries support that conclusion,
independently of enablement or support compatibility.

The packet closes **56 structural inventories**: 33 direct support declarations,
11 potential Skill lists and 12 direct item declarations. It adds no definitions,
members, rules or numerical coverage. Ordinary Rust evidence checks use bounded,
checked-in source archives with upstream licenses, exact source/owner joins and
negative controls for missing references, extra effects, factory routes and
incorrect Firebolt ownership. Full descriptor inverses constrain the delta.
The joint publication test additionally requires whole-recipe preservation,
authenticated Import dependency rebinding, all five unchanged drafts/sidecars
and all 110 query rows. All eighteen ordinary checks and the explicitly invoked
all-five publication pass; strict Clippy and formatting pass. The package rebuild
is byte-identical, and independent review verified all 56 deltas, 25 retained
source archives and the 504 item observations. Evidence:
`runs/owned-structural-inventories-validation-01.log`,
`runs/owned-structural-inventories-tests-final.log`,
`runs/owned-structural-inventories-clippy-02.log` and
`runs/owned-structural-inventories-publication-01/validation.json`.
No Lua code, source interpretation or new backend branch enters native evaluation.
The remaining passive, Skill and supplied-Actor census still blocks proof of
Scenario-intent dormancy.

**Passive structural checkpoint, October 20:** the reviewed
`passive-structural-inventories` packet closes **eighteen direct inventories on six
existing owners**. It completes At your Command (`35560 → 11c2`), the
Puppet/Archon effect nodes (`19318 → 0d0f`, `45333 → 14d8`), Command cost/Puppet
capacity (`62210 → 19ec`), and both Barya suppliers. The positive chains remain
exact: `32705/10d4 → grant32d5 → supply32d4 → Skill032c` and
`13289/0b34 → grant32d3 → supply32d2 → Skill0322`.

Of 55 selected explicit passive definitions, **49 now have complete direct
grant/Actor/skill-supply inventories and six remain Partial**. Numerical rule
ownership remains **43 Complete / 12 Partial**, a separate measure. No IDs,
members, numerical programs, parameters, outputs or descendant coverage changed.
All twelve reviewed candidates are accounted for. Four buff-gain producers
(`53324`, `14686`, `53795`, `26926`) remain unresolved because the pinned exports
omit their complete payloads. Two Command unlocks (`23265/0e15`, `10561/0a6d`)
retain the separate source and execution gaps below. Neither parser rejection
nor current unversioned descriptions prove an empty inventory.

Nine Rust checks pass, including exact whole-release inversion, a byte-identical
rebuild and unchanged all-five drafts/sidecars, 110 queries and selected input
counts. Strict Clippy passes. Independent review verified the authoring seal,
upstream license, nine source archives, sixteen source boundaries, seventeen
predecessor artifacts and negative controls. Evidence:
`runs/owned-passive-structural-validation-01.log`,
`runs/owned-passive-structural-clippy-01.log` and
`runs/owned-passive-structural-inventories-publication-01/validation.json`.

The two structural publishers now share the same test-only whole-release inverse
instead of maintaining duplicate implementations. Replaying the earlier packet
through this helper reproduces all eighteen package files byte-for-byte
(`runs/owned-structural-publication-reuse-proof.json`); its three checks also pass.
Native-only validation additionally passes all ten ordinary checks across the
passive family and both publishers, plus strict Clippy. The two explicit
publication tests are not repeated in that ordinary pass; both already ran
successfully with all features. Logs:
`runs/owned-passive-structural-native-only-01.log` and
`runs/owned-passive-structural-native-clippy-01.log`.
Formatting and diff checks pass. These targets enter CI through its existing
Cargo-metadata census in both feature profiles. This refactor adds no production
runtime path; hosted validation still awaits Git publication.

The pinned GGPK exporter knows `BuffDefinitions` and `BuffTemplates`, but their
raw tables are not retained. A bounded supplemental acquisition should retain
the selected Puppet Master/Archon rows and every referenced stat/description
record, raw hashes and complete field accounting. The current evidence pins
PoB `3887ae68…` / application 0.23.1 and tree generation `0_5`, **not an exact
game/client patch or original content archive**. The tree exporter itself
hardcodes `0_5`; changelog patch mentions cannot supply the missing identity.
A local installation/snapshot path has been requested from the owner. Any
provided snapshot needs independent version evidence and reconciliation; it
must not be represented as the identical original export.

The [Djinn Command investigation](djinn-command-investigation.md) distinguishes
missing Player Commands from supplied-minion abilities. Sixteen selected/unselected
controls and eight independent fresh replays have exactly matching compared
results. This is a deterministic oracle coverage limitation, not evidence that
the unlocks are inert. No parity exception or unconditional native execution is
introduced. The runtime source is independently pinned; the two export text
files are only locally hashed corroboration. Complete data and a reviewed generic
execution-dependency contract remain necessary. The next independent work is the
remaining Gem/Skill/Actor structural census, followed by actual Scenario-control
publication and configuration accounting. Keep the missing-data branch explicit
while advancing evidence-backed work.

**Gem potential-Skill checkpoint, October 20:** four additional inventories are
now validated and published locally:
`06c5 → []`, `0797 → 0149` (Frost Bomb), `07dd → 01df` (Ice Nova) and
`086b → 02a2` (Pain Offering). The `gem-skill-inventories` packet authenticates
the complete catalog/constructor and support-role dispatch using seven existing
source archives and nine reviewed boundaries. Its exact inverse permits only
these four `skills` closure changes. Members, direct grant/Actor/supply and
descendant inventories remain unchanged; no definitions or programs are added.

Ice Nova's two additional-stat-set aliases are accounted for separately from its
two constructed stat sets; neither is an extra Skill. Skeletal Sniper provides
a real negative control: its missing declared/constructed Command reference
blocks completeness even when the resolved effect list contains one Skill.
Frost Nexus's empty potential-Skill list does not remove its ground mechanic.

All nine Rust checks pass, including whole-release inversion, a byte-identical
rebuild and all-five unchanged drafts, sidecars and 110 queries. Strict
all-features Clippy passes. Independent review found no remaining defects in the
seal, source/dependency bindings, exact delta or negative controls. Evidence:
`runs/owned-gem-skill-inventories-validation-01.log`,
`runs/owned-gem-skill-inventories-clippy-01.log` and
`runs/owned-gem-skill-inventories-publication-01/validation.json`.
Native-only validation also passes all eight ordinary checks and strict Clippy.
Its single explicit publication test is intentionally ignored in the ordinary
pass and already passed in the all-features run. Logs:
`runs/owned-gem-skill-inventories-native-only-01.log` and
`runs/owned-gem-skill-inventories-native-clippy-01.log`.
Scoped formatting passes. This checkpoint does not reduce any selected input
count or complete a build.

Sniper Actor `3091` and Basic Attack/Gas Arrow Skills `0021`/`0024` already have
complete direct inventories and must not be counted again.

Frost Bomb and Ice Nova still need their supplied Skills' Infusion/ground/Orb
mechanics, and Offering's described Minion spike remains an explicit descendant gap. In parallel,
actual Sigil/Cry policy transport fits the existing schema. Executable Cry
provider grants expose a separate authority/scheduling gap below; complete
Scenario-intent admission remains blocked.
Include Sigil's Chiming Staff grant, not only Gem `0911`; the three Cry supports
`06eb/06ec/06ed` construct distinct children `05a5/05a6/05a7`. Positive policy
tests cannot certify an exhaustive source domain or an empty recipient set.

**Direct Gem checkpoint, October 20:** `gem-direct-inventories` closes twelve
direct inventories on four existing Gems, preserving every member and every other
field. Frost Nexus retains empty intrinsic grants/Actors/skill supplies. Frost
Bomb, Ice Nova and Pain Offering retain their exact primary grant/supply/Skill
chains. No new identity, numerical program or execution permission is introduced.

The initial three-Gem ownership handoff is
`runs/gem-direct-ownership-review-20261020/handoff.md`, SHA-256
`fc4d18001678dbc6679111de7ff738ac72711720a5406708aef5aa343fcf9ff7`.
The final packet also proves Frost Nexus's receiving ownership: its support type
and modifiers attach to the exact supported Skill. Its unmapped radius/duration
constants and ground topology, lifecycle, trigger and numerical obligations stay
unresolved. The source export and active-Skill dispatch likewise keep the other
three Gems' Orb, Infusion, ground and spike mechanics on their supplied Skills.
The proof preserves external owners when item/tree-created groups reuse Gem IDs
and external modifiers produce extra support effects. It includes Barbs/Thorns,
additional/missing-effect and exact chain-redirection controls; null mappings
are never classified as inert.

All ten checks pass with all features: seven ordinary family checks, two
publisher inverse checks and the explicitly invoked all-five publication. All
eighteen package files reproduce byte-for-byte, and all five unchanged drafts,
sidecars, 110 queries and selected input counts survive. Strict all-features
Clippy and scoped formatting pass. Independent review authenticated the seal,
all actual predecessor artifacts, seven reused source archives, eighteen
ownership boundaries, the upstream license and the exact twelve-field inverse.
Logs: `runs/owned-gem-direct-inventories-validation-01.log`,
`runs/owned-gem-direct-inventories-clippy-01.log` and
`runs/owned-gem-direct-inventories-publication-01/validation.json`.
Native-only validation also passes: seven family checks and two publisher
inverse checks, with the separately executed all-five publication explicitly
ignored in this profile. Strict native-only Clippy passes. Logs:
`runs/owned-gem-direct-inventories-native-only-01.log` and
`runs/owned-gem-direct-inventories-native-clippy-01.log`.
No original build is complete.

The remaining structural census concerns four Gems with unresolved Command
references, six passive owners, supplied Actors and descendant Skills. Keep the
pending game-data request and support-generated-child proposal separate from
completed direct declarations; neither missing evidence nor API permission is
supplied by this checkpoint.

**Actual stage-policy handoff, October 20:** the source review is retained in
`runs/scenario-stage-authoring-20261020/README.md` and `evidence.json`
(evidence SHA-256
`ee8c995ae9a591427d6576e349a1ed17e2d93d7d4d23522c4ae85e6681c99213`).
It authenticates thirteen complete source files, nine current descriptors and
all five unchanged originals. Each original has Sigil Placeholder 1 and Cry
Placeholder 10, with no Input lane for either: ten saved leaves, not defaults.

The existing model can publish two UsagePolicies with required integer parameters,
positive exact supply applicability, Skill-owned raw-stage facts and checked
configuration accounting. Four currently Unmapped Skills need honest partial
schemas; their Gem/item/support routes need real grant and supply declarations.
Chiming Staff's Sigil belongs to an item Modifier, using the existing Firebolt
import/provider seam. Cry tiers map I `06eb → 05a5`, III/Paquate `06ec → 05a6`,
II `06ed → 05a7`; numeric ID order does not imply tier order.

Preserve Sigil zero. Cry skips zero lanes, but the shared selector validates
every lane first: its raw transport must admit zero so Input 0 plus Placeholder
10 remains valid. A positive-domain consumer can reject an exact zero override
without inventing one stack. Missing/both-zero Cry stays Pending. Keep raw
chosen values separate from injected caps, subtraction and downstream effects;
editor width and PoB's MAIN/CALCS `Effective` switch are not native mechanics.
Paquate's five stacks per Warcry are distinct from simultaneous configured stacks
and trigger frequency. Sigil still needs recipient/stacking evidence.

**Immediate executable check before Cry publication:** use an actual physical
SupportAssignment → Grant → SkillGrant → child through support
preparation/finalization. Existing `owned_support_generated` coverage primarily
uses generated *receivers* and does not establish this supplier path. Ordinary
EffectPlan rejects SupportAssignment roots, while support planning limits the
root programs to SupportOrigin and deferred receiving templates. The rule compiler
already rejects `ActivateGrant` and `ProjectSkillParameter` in SupportOrigin
programs. Existing source-final-input assembly permits exact provider projection,
but not grant activation. A physical assignment's reported Player owner also
does not prove the generated child's Actor when the receiving Skill belongs to
a supplied Actor. Positive source inventory is not invocation or trigger authority.

Six isolated current-contract probes now pass under
`runs/scenario-stage-authoring-20261020/support-child-probe-01.log`.
They confirm the two compiler rejections, exact descendant scheduling gaps for
repeated physical assignments, finalizer refusal, unchanged working baselines
and fresh/reused scratch agreement. The temporary Cargo target was removed;
the draft and subsequent `support-child-probe-validation.json` remain local
investigation evidence. These synthetic V12/V13 capability-subset controls do
not prove positive V28 source-property integration or actual Cry execution.
The investigation has one unused-import warning from its unchanged shared
fixture; strict lint is not claimed for that temporary target.

Independent review agrees that widening the execution permissions is a public
contract change. Existing identities describe one child per physical assignment,
not a new child per receiving application. The pinned PoB constructor attaches
its additional effect to Player per enabled Gem; this establishes reference
ownership, not gameplay trigger or duplicate-selection eligibility. Do not use
the support root's Player value or the first receiver as inferred authority.
Prove the actual provider path, participation, exact Actor binding and
retained-selection dependencies before enabling execution. Reuse existing source
assembly where valid. The
[support-generated Skill proposal](owned-support-generated-skills-proposal.md)
is awaiting owner review; it recommends a classified supplier path with explicit
Actor and execution dependencies while retaining SupportOrigin restrictions.
No permission change is implemented. This supersedes the earlier expectation of
an implementation-only change. Positive stage transport must retain incomplete
composed census and all unrelated numerical gaps.

**Cry lifecycle evidence, October 20:** the separate static handoff at
`runs/cry-lifecycle-review-20261020/README.md` authenticates thirteen source files,
all three actual Cry routes and 28 bounded excerpts. Its evidence SHA-256 is
`f3fb10a5d9d59724b32a1be2121182f6890eff65e7d921c8b4e5e673bca697f1`.
It does not contain executed oracle controls or new gameplay authority.
PoB constructs a physical child separately from support retention and Warcry
compatibility. The child can inherit the level recorded from a compatible
active Skill through the exact same physical support source; duplicate selection
and multiple compatible Skills require explicit donor/order controls. That value
is not yet proven to be the donor's final fully supported level. Support-tagged
sources bypass the ordinary active-Gem property collector, so copying that
collection recipe or using raw support level is not a valid preparation proof.

The inspected Cry trigger linkage and Paquate recent-use Life-cost constant lack
executable mappings. Configured stack occupancy still feeds a separate DoT result,
and Full DPS takes a maximum that can mask extra children or wrong levels. No
automatic bug exception or zero-cost assumption is authorized by these findings.
Next evidence must capture exact physical child/retained-support identities,
compatible receiver, level donor, participation and trigger fields before damage:
three tier baselines, source/Warcry disablement, non-Warcry, duplicate/mixed-tier
order and multiple-Warcry controls. Preserve fresh matched-stage replays and
original inputs. These source controls are independent of the pending native
supplier-contract decision.

**Executed Cry controls, October 20:** the reviewed Rust investigation passed
eight controls, each with two fresh replays, three fixed lifecycle checkpoints
and both JIT modes. Both final reports are byte-identical (8,171,912 bytes,
SHA-256 `bc010ae260a1a65b264ef66f6281dd5adb2d074c882087d0fd821655c4b2c12b`).
Rebuilds change only the output revision; no retry or selected-child substitution
was used. The three tier baselines receive level 20 through the retained support
with the identical physical source. Disabling Cry removes its child. Disabling
the Warcry or replacing it with Spark leaves a selected level-1 Cry child with
no compatible level donor, yet PoB reports `CorruptingBloodDPS` of
59.333333333333336. Tied duplicate copies both construct children; only the
first retained support supplies its own child's level, and reordering swaps
those exact identities. Nonselected child output stays unavailable.

Thus potential construction and numerical DoT are demonstrably insufficient
proof of gameplay invocation. No automatic source-bug exception, native trigger
permission or complete original-build coverage follows. Mixed-tier and
multiple-donor controls, final donor-level semantics and gameplay cost/frequency
remain unproven. See the separate
[runtime observations](../runs/cry-lifecycle-review-20261020/runtime-results.md)
and `runs/cry-lifecycle-review-20261020/probe-run-01.log`. The temporary Cargo
target was removed; source/evidence remain local. Independent review confirms
the observations. The existing Windows linker warning remains; strict lint is
not claimed for the temporary investigation.

**Ordinary physical hit-DPS checkpoint:** while the supplier decision and missing game
data remain pending, the native component now composes
Original05's exact Sniper Basic Attack (`reference-14`). It reuses post-armour normal
and critical averages `33b2/33b3`, critical chance `33aa`, after-block hit chance
`3220` and ordinary rate `33a3`. Its bounded source proof covers effective timing,
mechanical quantity, DPS factors and late repeat/hazard writers, extending the
existing timing evidence. It does not establish native candidate-source closure.
No new public contract was needed. Partial source/mitigation and all five input
obligations remain blocking. Unavailable saved average-hit results are preserved;
mechanical quantity is distinct from saved count and Full DPS reporting. The recorded
388.7517825598493 DPS is a reference validation target, never a native constant.

Implementation is in `data/owned/poe2/3887ae68/physical-hit-dps`
and the corresponding Rust family/native/publication tests. The packet
adds internal weighted/post-block physical averages and ordinary physical DPS,
not a saved metric binding. Its sole shared-fixture change exposes the existing
Armour replay to compose existing producers. Publication must run the extended
source-domain checks as well as the data seal and exact append inverse.
The initial capture is unfiltered at construction, but does not capture the
final minion store: late factories and transport need their own complete-source
proof. Repeat FINAL can alter DPS independently of ordinary rate. All 39 retained
comparison rows preserve exact identities: 36 physical-only comparisons agree,
while three conversion/gain controls explicitly differ from all-type DPS.
Finite native composition produces 388.7517825598493 without inserting that
reference value as an operand. Fresh/reused/Rayon, independent Actions, changed
input identity, unsupported domains and unknown/zero/inactive late writers are
covered. The 24 family/native and three ordinary CLI tests pass. The explicit
publication passes exact append inverses, identical rebuild and unchanged
all-five imports/110 queries. No owner completion or evaluation binding is added.

The first explicit publication caught JSON integer-versus-float representation
differences in 31 typed quantity literals. The helper now compares complete
`DefinitionRules` after requiring exactly one matching owner, retaining numeric
values, program order and coverage. A typed roundtrip regression reproduces the
real loader path. Retained package data and its authoring seal did not change.
Final validation passes: **27 ordinary tests in each of the all-features and
native-only profiles**, plus the explicit all-five publication test. Strict
Clippy passes for both profiles; formatting and scoped whitespace checks pass.
The explicit publication is intentionally opt-in during ordinary runs and was
executed separately here. Evidence:
`runs/owned-physical-hit-dps-tests-final.log`,
`runs/owned-physical-hit-dps-native-only-01.log`,
`runs/owned-physical-hit-dps-publication-02.log` and
`runs/owned-physical-hit-dps-{clippy-02,native-clippy-01}.log`.
The optional PoB build retains the existing Windows LNK4098 linker warning;
the native-only run reports none. Initial failed diagnostic runs remain preserved.

**Admission investigation at the physical-DPS checkpoint:** the preset-usage
obligation was `0507` (not the historical `0506`), with 35 known preferences and
20 linked saved-field origins. Account for each field against exact occurrence
participation, usage and reporting consumers, beginning with the retained
manual-Djinn `global2` evidence.
The existing importer attaches preferences but deliberately leaves the inventory
Pending; source-field transport is not exhaustive completion authority. The
following checkpoint implements the finite whole-inventory checker; original
admission still requires complete evidence and the public finalizer. Retire the
entire obligation only when every field and consumer is covered; otherwise identify the precise remaining
row and missing behavior. Do not widen support-generated child execution, infer
empty inputs from consumers that were never called, or substitute a smaller build.
All five originals remain preservation regressions. The other four input
obligations and selected numerical coverage remain independent gates.
The [frozen next-blocker snapshot](../runs/physical-dps-next-blocker-20261020.md)
joins all twenty origins to exact XML locations, owned occurrences, seven retained
evidence families and the code that preserves Pending. Its companion JSON is
bound to the preceding physical-DPS publication and source XML; SHA-256
`36d11ce402db897c099eab740f14b8ecdd32bb14645b30522cf53682c5cf1d35`.
It is investigation evidence, not a completion certificate. If exhaustive
accounting exposes a new public contract, discuss that specific gap before
changing authority.

### Preset-usage admission checkpoint, October 20

The accepted skill-preset usage proposal's gate 5 authorizes a checked finite
inventory in Import. The checker is implemented on the existing usage policy,
with optional inventory authority and no parallel importer or native ownership
change. Absence of that authority leaves the existing Pending behavior intact.
The checker covers the entire SkillSet and compares exact successful
conversion receipts with their current typed destinations and source links. It
validates all linked origins and precharges an atomic retirement plan before
marking a preset inventory Complete. Group fields may be accounted for by their
actual consumers within that group; every occurrence still requires its own
complete field, descendant and destination coverage. No unchanged original has
passed this gate yet.

The 197 Import library tests and ten new public inventory tests pass, including
the four private receipt/retirement tests. These cover independent presets,
late unknown and duplicate occurrences, all declared consumers, override lanes,
stale typed values, detached/foreign origins and late budget failure without
partial retirement. The public checked finalizer retains other obligations
after the finite fixture's usage inventory completes. The earlier 428
Gem-disposition, normalization, release-revision and successor regressions
also passed; final optional-gate corrections did not change the absent-authority
path. Evidence: `runs/owned-preset-usage-inventory-tests-02.log` and the completed
regression targets in `runs/owned-preset-usage-inventory-tests-01.log`.
Independent review found no false-completion or cross-group evidence borrowing.
This initial checker checkpoint did not yet consume support proofs or generated
input proofs. Their follow-up is recorded below; reference/group/global
responsibilities still need their own accounting.

The selected Original05 inventory now contains 36 known preferences and
20 remaining linked origins, but those counts are not the completeness domain.
The review also covers support rows and previously accounted sources. Two
concrete gaps remain: complete group action-selection correspondence, and
global-effect switches whose consumers depend on later modifier processing.
PoB's lazy stat-map processing can set global-effect metadata after initial
loading. Neither an empty early observation nor a consumer that was never
called proves those switches inert. MAIN/CALCS reference selection is separate
from native participation and from aggregate damage reporting.
The pre-count [finite field review](../runs/owned-preset-usage-field-review-20261020.md)
authenticates 23 input/report pins, seven upstream file bodies and 16 bounded
source spans. Its independent XML census confirms 199 attributes on the twenty
remaining origins and no child elements on those ten primary rows. The 35 known
preferences comprise twelve participation, twelve reporting, nine count and two
effect-specific preferences. The companion JSON has SHA-256
`06604e8ab0199635accfd672d50e61716d94f4a1139e04c5390b12b87a3a68cc`.
Import code references identify unpublished working snapshots, independently of
the pinned upstream bodies and immutable reports.
These are investigation results, not certified dispositions. Three global fields
have provider-override candidates, two have singleton/no-Vaal candidates, and
fifteen still require semantic or reachability proof. Group main-action selectors
choose reference state within the selected socket group; export also reads
`mainActiveSkill` for enabled groups. FullDPS initializes its environment before
overriding the main skill for each pass. They are not universally inert. Prefer
proving their actual source reference/export responsibility in Import over adding
an unnecessary native gameplay setting.

The missing [Ice Nova count projection](../data/owned/poe2/3887ae68/ice-nova-count-inputs/README.md)
is implemented and published locally through the existing typed count policy.
Eight ordinary Rust tests and the explicit public CLI publication test pass.
The package rebuilds byte-identically. All-five exact draft/sidecar inverses
allow only four new count bindings in Original05, one in the selected preset,
and their checked dependency/lineage changes. All 110 queries, every other
input and the selected issue counts **107/118/110/124/5** remain unchanged.
No evaluation bundle or complete native build was added.
Retained source controls cover occurrence counts, group overrides and
count-sensitive reporting. This is input transport only; it does not implement
FullDPS aggregation or complete the preset inventory. The selected preset now
has ten count, twelve participation, twelve reporting and two effect-specific
preferences. Its usage issue remains `0507` on preset `01dd`; the current lineage
is `dfbd0361d99aa3a5ccf35cbf2bfd5f99`.
Existing group-override semantics intentionally ignore a malformed losing
occurrence value when a valid override wins. Preserve that behavior and require
separate field accounting before granting completeness.

Evidence: `runs/owned-ice-nova-count-inputs-tests-02.log`,
`runs/owned-ice-nova-count-inputs-publication-02.log` and
`runs/owned-ice-nova-count-inputs-publication-01/validation.json`.
Strict Import all-target and scoped CLI Clippy pass with warnings denied:
`runs/owned-preset-usage-inventory-clippy-final.log` and
`runs/owned-ice-nova-count-inputs-clippy-final.log`.
The ordinary fixture's source-pin shape check and publication wrapper's stale
configuration dependency were corrected without changing source evidence or
weakening admission; the earlier failing logs remain diagnostic records.

CI discovers the new Import tests and ordinary CLI tests automatically on
Windows and Linux; the CLI shards cover both feature profiles. The immutable
publication test remains an explicit offline gate. Review found one checkout
hazard in the reused upstream license: Markdown normalization would change its
authenticated CRLF bytes. An exact `.gitattributes` override now preserves that
file. Read-only raw-versus-filtered Git hashing agrees, without writing Git
objects; evidence is `runs/owned-ice-nova-count-license-portability-01.json`.
The bounded audit found no other text-normalization hazard in this checkpoint.

The latest [source checkpoint snapshot](../runs/secondary-switch-source-next-blocker-20261020.json)
records the tested source components and the then-pending Import artifact decision.
The owner approved that migration October 20, including immediate related cleanup
and current-artifact rebuilding without compatibility support.
The preceding [receipt checkpoint snapshot](../runs/usage-receipts-next-blocker-20261020.json)
records the completed receipt transport, source-component tests and unchanged
five-build replay, together with the remaining source-interpretation work.
The preceding [count checkpoint snapshot](../runs/preset-usage-count-next-blocker-20261020.json)
binds the selected draft, sidecar, finalization report, package and count
publication proof; SHA-256
`651fbed62a6dba2eb0502d4b353e6636762667afaa37daef2b6413bc5989feaf`.
Five independent input obligations remain. The support and generated-input
proof producers are now connected as described below; finish the bounded
group-selector and global-effect source accounting next. Keep original input
obligations and numerical coverage distinct. Gate 5 remains incomplete despite
the tested checker and additional projection. Git publication and hosted CI
remain subject to the previously recorded approval-service usage-limit blocker.

The receipt extensions now use the existing producers. Original05's
selected preset contains sixteen physical support assignments across twelve
definitions, all with Complete Gem parameter lists, and three exact generated
input bindings (Sand, Water and Firebolt), also Complete. Those observations
alone confer no usage-inventory authority. The importer retains successful
support proof tokens through actual SupportAssignment construction, checking
exact Gem, assignment, source/group/preset and live authored-target membership.
Generated receipts are emitted only after existing live-value, exact-provider,
source-frame and reference checks succeed. Final inventory checking revalidates
the exact raw/usage bindings, allocation or equipment/item records, supplying
preset membership and origin links. Archived accounting emits no receipt.
These reuse the existing adapters and add no provider matcher or level default.
The absent-inventory path retains its output, identity and work-budget behavior.
All **590 Import tests pass** (206 library, 374 normalization and ten public
inventory tests), including actual generated-input normalization which retires
only the selected fixture's usage issue. Whole-draft and origin inverses retain
archived Pending state, exact values and spent allocator watermarks. The support
controls cover exact assignments, detached targets, stale outputs, duplicate
membership, origin tampering and atomic budget failure. Actual producer checks
are separate from deliberately constructed private consumer fixtures.
All five optional PoB source-component tests pass with fresh JIT-off/on controls.
Strict Import all-target and source-target Clippy, scoped formatting and Git
whitespace checks pass. The all-five publication replay passes, retaining all
110 queries and selected unresolved counts **107/118/110/124/5**.
No original inventory is claimed complete and the published package remains the
Ice Nova count endpoint above. A separate Rust comparison against the preceding
checkpoint passes for all fifteen draft/sidecar/report documents and all eighteen
package files. Fresh CLI imports intentionally allocate random project lineages,
so raw draft/sidecar/report bytes differ across projects. The comparison
authenticates exact project lineages, draft and sidecar commitments, report
lengths and output directories before normalizing only that metadata; all
remaining fields compare equal. Package files compare byte-identically.
This is import preservation, not numerical evaluation or complete-build parity.
Evidence: `runs/owned-usage-receipts-tests-02.log`,
`runs/owned-preset-reference-selection-tests-02.log`,
`runs/owned-usage-receipts-preservation-01.log` and
`runs/owned-usage-receipts-baseline-01.log`. Initial fixture failures are
retained in `runs/owned-usage-receipts-tests-01.log` and
`runs/owned-usage-receipts-integration-01.log`: duplicate attributes are rejected
at decoding, and the shared fixture must account for one spent retired issue.
Neither correction changed production semantics or weakened inverse checks.

Group fields belong to their actual consumers across the whole group. Every
occurrence still needs exhaustive local-field and descendant accounting; every
declared destination remains conjunctive. Do not require support gems to own a
primary skill's count/reporting preferences. Test multiple supports, repeated
definitions across groups/presets, missing or changed current outputs, detached
origins and unknown/disabled siblings. These are follow-ups within the accepted
Import model, not new native semantics. Reference group selectors and missing
global-effect proofs remain separate blockers after receipt plumbing.

For the current group selectors, investigate a finite source reference/export
receipt admitting only absent, `nil` and `1`. All twenty remaining saved fields
fall in that domain. Existing occurrence-level Action correspondence does not
prove position in the group. Prove the final constructed ordered prefix,
including `Data.lua` display-order rewriting, exact target, enabled/non-support/
non-hidden status and global gate. The current catalog lacks `hideFromSideBar`;
retain bounded complete-definition evidence for that fact. This can avoid
requiring later Command execution coverage when the exact primary remains first,
without declaring those later mechanics supported. Generated Firebolt already
has an exact Skill target and needs no artificial Action adapter for this proof.
Test reordering, duplicate/hidden/disabled sources, failed gates, missing
providers, empty lists, unsupported numeric tokens, independent MAIN/CALCS and
a later unresolved effect. Do not copy the source's upper clamp or malformed
token fallback into native behavior. This is an investigation and next
implementation step, not certified selector accounting. The new
[source-component packet](../data/owned/poe2/3887ae68/preset-reference-selection-source/README.md)
retains authenticated constructor, admission, selector and exporter evidence.
It covers eight primary families and the ten current group/primary pairs, with
fresh JIT-off/on behavioral controls. Its host constructs identity-only active
skill objects; it does not execute full skill preparation, initialize lazy
modifier metadata or prove numerical readiness. In particular, later unresolved
Commands remain unresolved. The packet grants no Import or native authority.

The follow-up global-switch census is **ten primary `true`, seven secondary
`true`, and three secondary `nil`**; the latter are the generated Sand/Water
Djinn and Firebolt secondary switches.
This census covers the twenty remaining primary-source fields. The whole-preset
gate also requires support rows' count/global responsibilities; their new
intrinsic proofs do not discharge those fields.
`nil` decodes false. A true value makes a source admission gate unrestricted,
but does not justify discarding the authored toggle or replacing it with general
participation. Consumers include CalcSetup active-effect admission, BuildExport,
and CalcDefence Vaal count/reservation handling; downstream global modifiers
still need exact recipient/application coverage. The existing
[Frost Bomb typed-preference transport](../data/owned/poe2/3887ae68/frost-bomb-inputs/README.md)
is reusable transport, not evidence of another skill's mechanical intent.
Ice Nova's description establishes Cold Infusion consumption producing Chilled
Ground, but no retained evidence equates its saved primary global switch to
either choice. Do not clone an opaque PoB switch into native data or call it
ordinary participation. First prove the complete singleton effect list and every
secondary-switch consumer, and investigate the primary switch's semantic
destination. The Vaal count branch also requires an actual second list entry;
test singleton/Vaal controls before adding any catalog property. Multiple stat
sets do not establish multiple effects. Any later transport
must preserve exact occurrences, preset ownership, raw exports and Scenario
overrides, with mechanics still Partial until independently supported.
Exact provider reconstruction overwrites three primary switches, not their
secondary switches. Both Djinn Commands exist; lazy metadata and four missing
skeletal Command references remain separate gaps. No new Core model is currently
demonstrated necessary for preference transport.

Secondary field accounting also needs an explicit source-fact transport seam in
the existing Import disposition adapter. Current private producers deliberately
exclude the global switches. Neither an evidence digest nor native `Gem.skills`
membership proves the final ordered source effect list. Establish that authority
and its live source/catalog binding before allowing a private receipt for only
the proved field. Reuse the current physical/generated adapters and atomic
inventory gate; do not add family/hash allowlists, a parallel importer or an
opaque native Boolean. Retain stale-binding, additional/missing-effect,
cross-preset, provider-change and late-budget refusal controls.

The [source-construction proposal](owned-source-effect-construction-proposal.md)
was accepted October 20. The existing offline identity catalog already retains
these observations; preserve a compact compiler-derived projection in the existing
Import-only role artifact. Replace duplicated per-disposition construction claims
with that shared evidence, rebuild the current format and remove related obsolete
interfaces together. The owner explicitly authorized breaking unshipped contracts;
do not keep compatibility readers or duplicate runtime paths. Native schemas do
not acquire these source facts. Independent constructor/secondary-consumer source tests now pass;
they grant no import-completion authority. Primary-switch semantic investigation
continues separately.

The primary-switch audit found a finite neutral case: the exact saved literal
`true` leaves the source admission/export predicate unrestricted for any lazy
`hasGlobalEffect` state. The Vaal helper also returns enabled=true, which
FullDPS consumes; reservation uses the returned count, not that enabled result.
This justifies an exact-field neutral-gate receipt after the existing live
source, provider and output correspondence checks. No effect-index identity is
needed for this one literal: it adds no restriction to whichever effect occupies
index one. Preserve the raw attribute and existing participation destination;
do not enable a disabled skill. Count,
group-count, reporting, reference selection and additional-effect fields remain
independent. Missing, `nil`, false and malformed values gain no authority from
this argument. The private authored/Direct and generated input receipts now
retain this fact; support receipts remain unchanged. Focused validation and all-five
import preservation pass at the checkpoint below. Current published normalization has no usage-inventory policy,
so this capability does not retire any original's obligation or change its inputs.
Broader false-switch inapplicability requires a complete eager/lazy stat-map and
extra-stat reachability proof, including tooltip-triggered metadata, and remains
open. Generated MAIN's overwrite does not prove independent cold CALCS behavior.

**Source-component checkpoint, October 20:** the
[secondary-switch packet](../data/owned/poe2/3887ae68/secondary-effect-switch-source/README.md)
authenticates nine complete source bodies and thirteen source spans. Seven
ordinary Rust tests pass in fresh JIT-off/on VMs, exercising actual constructor,
count/export/visibility consumers and the actual admission predicate with an
explicit identity-only host. Controls include the real missing Sniper Command,
synthetic missing/sparse/late additions, duplicates and display reordering,
multiple stat sets, Vaal states, changing global metadata and independent
enablement/count gates. Full preparation, reservation and FullDPS do not run.
The candidate singleton predicate is test-only; it is not a production matcher.
Strict target Clippy and formatting pass. Logs:
`runs/owned-secondary-effect-switch-source-tests-01.log`,
`runs/owned-secondary-effect-switch-source-clippy-01.log`,
`runs/owned-secondary-effect-switch-source-format-01.log`.
MSVC emitted the existing `LNK4098` default-library warning; linking and tests
succeeded, with no warning waiver added.
Independent review authenticated the packet and source boundaries and found no
actionable issues.
Production data and importer/evaluator behavior are unchanged, so this checkpoint
does not regenerate the package or claim a new all-five numerical replay.
Current native completion remains **0/5**. The existing cleanup register now
tracks the source-shaped Frost Bomb transport label for semantic review rather
than copying it to Ice Nova.

### Selected requirement owners and neutral primary input, October 20

Two private implementation corrections advance the existing model without a
new public data contract. They are implemented and validated locally; Git
publication and hosted CI remain pending.

Requirement assessment previously scanned every packaged Action Requirement,
including Twister while evaluating Sniper. It now uses potential owners from
the existing structural traversal, including inactive providers, potential
Skills, their output owners and explicit supplied outputs. Missing/Partial
inventories and unestablished Skill supply remain blocking, regardless of
queries. Existing Scenario-intent census bookkeeping is shared; no additional
topology graph or Action enumeration was introduced. Actual outcomes and
numerical coverage are unchanged. Ordinary and support metric identities move
to `owned-metric-plan-v3` to invalidate reports with the old assessment semantics.
Selected Sniper reservation and complete Action context discovery remain open.

The exact literal primary `true` receipt described above is now shared by the
authored/Direct and generated input adapters. Independent review found no
authority widening beyond that one field: successful live producer proofs,
current destinations and every declared consumer are still required. Support
receipts, secondary switches and the current inventory-disabled release are
unchanged. This can account for ten of the retained primary fields after a
reviewed inventory publication; it does not retire preset `0507` on its own.

Validation passes: **594 Import tests, 104 Engine tests, 10 CLI tests and two
all-five import preservation checks** (710 total), strict scoped Clippy and
format checks. Initial failures exposed test assumptions: a numeric XML entity
is rejected before generated usage projection, and the resource-limit fixture
has five potential requirement subjects but only three deferred provider owners.
The corrected tests prove retained raw/source obligations, no unintended
retirement, both exact resource bounds and duplicate accounting. No production
limit was loosened.

The replay in `runs/owned-primary-gate-preservation-01` reproduces package bytes
and all five prior draft/sidecar/report contents after authenticating fresh
project identities and output metadata. The current input remains `60827bda…`,
with 110 unchanged queries and selected issue counts **107/118/110/124/5**.
This verifies imports, not a new whole-build numerical replay; native completion
remains **0/5**. Evidence logs use `owned-primary-gate-*`,
`owned-requirement-owner-*` and `owned-requirement-primary-*` in `runs/`.
The resume record is `runs/requirement-primary-next-blocker-20261020.json`.
The candidate publication manifest retains unrelated work separately. Earlier
Git publication was not executed because the automatic approval service reached
its usage limit; the owner's authorization is already present.

The next input work is the approved Import role-artifact migration for
constructed-effect facts, then source selector/secondary-switch receipts and
complete preset accounting. The separate configuration audit finds only two
unaccounted selected ConfigSet leaves: Corrupting Cry stages (`437`, value10)
and Sigil stages (`442`, value1). The subsequent existing-model slice below retains these
as two typed Scenario intents with nonempty, explicitly Partial supplied
applicability. Known execution schemas are not a prerequisite for preserving
intent: unresolved grant/supply dependencies must remain unresolved during
composition. Include Sigil's Gem and Chiming Staff Modifier routes and all three
Cry support routes; do not use an empty Complete applicability set. Reuse the
reviewed source lane selection, preserving Sigil zero and Cry's nonzero-lane
selection without inventing gameplay caps. Prove two added intent records per
original, exact source links and unchanged assumptions, Rewards and queries.
Leaf transport alone cannot certify complete assumptions, Scenario usage,
Scenario intents or configuration inventory. Preserve the 34 existing assumptions,
18 Reward records and all five selected obligations. Actual applicability and
mechanics remain later publication work; executable Cry supply still needs the
[support-generated Skill decision](owned-support-generated-skills-proposal.md).

### Saved stage-intent publication, October 20

The [stage-input packet](../data/owned/poe2/3887ae68/scenario-stage-inputs/README.md)
now retains Sigil stages and Corrupting Cry stacks as typed Scenario-owned intent
through the existing policy and normalizer. It adds no evaluator branch or public
contract. Offline Rust authoring authenticates thirteen reused full source archives,
the unchanged catalog mappings and the current predecessor. All runtime inputs
remain injected data.

Two nonempty Partial applicability lists retain five exact supplying relations:
Sigil's Gem and Chiming Staff Modifier, plus the three Cry support variants.
The four Skills, Sigil Gem and item Modifier remain Unmapped; existing source
inventories and every supplied output inventory remain Partial. Positive
relationships add neither activation programs nor preset-input, Actor or final-level
authority. Native mechanics, complete recipient discovery and Cry execution remain
separate work.

Sigil preserves zero and selects Input before Placeholder. Cry selects nonzero
Input before nonzero Placeholder. Both validate every present lane, including
losing candidates; malformed or duplicate inputs retain unresolved responsibility.
There is no gameplay cap, invented stage default or raw-to-multiplier conversion.
The numeric bound is the existing integer representation limit.

The CLI publication and fresh-repeat checks pass for all five unchanged originals.
Each gains exactly two intent records and exact provenance links; only those two
leaves lose their configuration fallback links. Whole-draft and whole-sidecar
inverses preserve all other values, local identities, allocator state, assumptions,
Rewards and incomplete inventories. All 110 query rows remain byte-identical,
the package rebuild is byte-identical, and selected finalization issues remain
**107/118/110/124/5**. Full intent diagnostics are retained separately.
This is import preservation, not numerical parity or complete applicability:
native completion remains **0/5**.

Nine ordinary packet tests, nine focused Import tests and the explicit
authoring/publication gates pass (20 total). Strict scoped CLI/Import Clippy,
formatting, diff and artifact text-normalization checks pass.
The earlier Import failure was an assertion about a losing Placeholder: checked
accounting correctly preserves it as `SourceOnly("unselected-config-projection")`.
The corrected test requires that exact disposition, the winning Input's original
disposition and equality of the entire origin list. A publication-test macro
syntax error and one redundant explicit dereference in the test helper were also
corrected; no production change or lint waiver was needed.

Evidence logs use `owned-scenario-stage-*` in `runs/`. The resume record is
`runs/scenario-stage-next-blocker-20261020.json`; its publication candidate
manifest keeps unrelated work excluded. Git publication and hosted CI remain
pending under the previously recorded approval-service usage limit.

**Checkpoint handoff (completed by the construction checkpoint below):** implement the accepted
[source-construction migration](owned-source-effect-construction-proposal.md#implementation-and-rebuild-plan),
including related compatibility-branch cleanup and current-artifact rebuilding.
Start from the package above. Reproduce compiler-derived observations offline,
rebind their Import dependencies without inventing a native schema change, and
then add narrowly proved selector/secondary-switch receipts. The separate
support-generated Skill execution proposal remains pending; raw intent transport
does not implement it. Later numeric admission can investigate Player Life's
remaining conversion-order gap using existing checked queries; Partial-empty
conversion queries do not prove zero or safe regrouping.

### Source-construction cutover checkpoint, October 20

The accepted Import contract now requires explicit unavailable or observed source
construction on each Gem role. Fresh compilation and extension share the same
projection; offline publication independently reproduces every row from the
authenticated catalog. Runtime loading checks bindings, nested budgets and exact
mapping joins without loading the catalog or executing Lua. Ambiguous Gem
selectors remain diagnostic during compilation but cannot certify an exact
owned-row pairing at publication. Missing Commands, repeated ordered effects and
additional stat sets remain distinct, with no new native execution authority.

The first 33 catalog regressions pass. Independent review also identified a
retained-declaration collection-budget omission; its correction and regression
are included in the broader selected run. The retired Direct V1 negative fixture
now requires decoding refusal. Catalog recipe (5), compact successor (8) and
Direct dispositions (26) pass. A Gem-catalog negative setup then encountered the
new earlier `ConstructionMismatch` refusal; its updated test checks that boundary
and retains independent downstream compiler refusal using an explicitly
unavailable-observation domain with a passing positive control. The remaining
Import checks now pass for Gem catalog (16), Gem dispositions (11), breadth (3),
release revision (17), construction catalog (27), catalog extension (7) and
successor (29), including the retained-declaration budget regression. The
first normalizer run passed 372 tests and exposed four stale test expectations for
retired tags and whole-frame rejection of raw-only Direct inputs. Those controls
are updated to require current-format round trips, retired-format refusal,
independent Pending inventories and bounded deterministic replay. The final
normalizer run passes all **377 tests** in
`runs/owned-construction-normalize-tests-01.log`. Earlier selected logs are
`runs/owned-construction-import-tests-{03,04}.log`.
No compatibility reader was restored. The explicit Rust cutover authorer
passes: candidate
`runs/owned-construction-cutover-candidate-01/package`, input
`b3b7c3810f659e584f23b6dd69b7bdc6b619cc39fcd4f2b9ae259194bf8d7529`,
preserves the complete native recipe, mapping, Rewards, items, source policy and
110 query rows. Only 966 construction observations and twenty dependent Import
commitments change, plus explicit authoring provenance. Repeated assembly is
byte-identical. The current reconstruction seed preserves eighteen of twenty
artifacts byte-for-byte; only roles and their transition receipt change.

Both maintained seeds are rebuilt through their production compilers. Their
previous raw artifacts are retained under
`runs/owned-construction-cutover-originals-01/`. The dependent fixture authorer
also passes: ten reproduced construction rows are added across three dependency
snapshots, and the old Direct fixture is rebuilt as current V2 with explicit empty
dispositions. Seven files change; the Warrior authoring file stays byte-identical.
Whole-document inverses preserve all native rules and historical commitments.
Candidates and originals are retained in
`runs/owned-construction-fixtures-{candidate,originals}-01/`; the Ice Nova
authoring hard seal is refreshed. The temporary library authoring hook has been
removed.

The dependent fixture/ordinary CLI checks pass (12 tests; nine historical or
explicit publication tests remain ignored). The explicit all-five CLI check also
passes in `runs/owned-construction-all-five-01.log`: ten fresh imports preserve
every draft, sidecar and selected request under authenticated inverses, with
identical repeated imports, unchanged 110 queries and selected Pending counts
**107/118/110/124/5**. The new validated Import endpoint is the `b3b7c381…`
package above. Its receipt is
`runs/owned-construction-cutover-validation-01/validation.json`, SHA-256
`2934bb54117928a1e15bf3687f8dd58dafba44da19da43987b9dae363df04d21`.
The repaired normalizer target also passes, closing this scoped regression
checkpoint.

The CLI proof reproduces all nine compiler artifacts from pinned seed/catalog
inputs and binds `compiled_roles` to that reproduction; its ordinary tamper test
rejects a substituted digest even with recomputed authoring. No migration reader
or public rebinding API remains. This migration retires no input issue, runs no
calculation and establishes no whole-build numerical parity.

Related cleanup removes physical-Gem V1/V2 and Direct V1 reader branches, Direct's
version-dispatch flag and historical support-primary singleton acceptance.
Keep the two live primary-usage proofs and all real additional-effect support
memberships. An explicit empty Direct disposition list represents unresolved
raw-only input; it grants neither completeness nor support-target authority.

The new endpoint is 65,563,069 bytes, leaving 1,545,795 bytes below the existing
64 MiB input ceiling. Record this measured capacity before further catalog growth;
this migration does not raise a limit. Next, after preservation passes, add the
smallest independently proved private receipt: secondary-switch non-consumption
for exact physical singleton construction. Original05 Ice Nova is the immediate
case. Missing skeletal Commands and two-effect Djinn must still refuse; generated
Firebolt additionally needs its exact loader/provider dispatch proved. Ordered
group selection remains a separate, larger proof.

### Completed coverage audit and implementation resume, October 20

The [completed local assessment](../runs/coverage-audit-20261020/assessment.md)
joins the saved-input inventory, dependency-ranked metric backlog and pinned
game-data census. Its scripts reproduce successfully and authenticate all
seventeen artifacts in the last validated `a0176def…` release. This was an
analysis/reporting checkpoint; no missing engine mechanics were implemented.
The audit files live under ignored `runs/`, with hashes and reproduction
instructions; they are not a claim of GitHub publication.

Complete unchanged native builds remain **0/5**. Original05 has five remaining
input inventories, but its 22 requested outputs and whole-selected mechanics
have substantial downstream work. The I-series input packages are entirely
included in the W00–W15 acceptance backlog, not additive. The consolidated
planning range is **73–167 experienced engineer-days** (14.6–33.4 weeks), with
low overall confidence, unbounded external evidence delays excluded, and no
claim of measured velocity. This replaces the earlier rough 8–20-week range.
Full-game family and numerical-support denominators remain unknown; imported
record counts do not measure implemented mechanics.

Retained diagnostic `SpiritUnreserved = -67` makes complete numerical evaluation
and a legal optimizer candidate separate acceptance milestones. Preserve the
unchanged baseline and report its actual legality; do not repair or suppress
obligations to claim success. A first legal candidate integration is separately
estimated at 3–6 days after complete evaluation, not a general optimizer estimate.

**Resume:** the construction migration checkpoint is complete locally: normalizer,
dependent fixture, ordinary CLI and explicit all-five preservation checks pass.
The superseded
`owned_usage_transport_cutover` test and its exclusive helper are removed; their
authenticated historical observations remain retained, as recorded in
[the cleanup ledger](legacy-retirement.md#current-physical-usage-transport-cleanup-2026-10-06).
The reviewed private secondary-switch adapter is now applied, with its focused
tests and strict lint checks passing. The group-selector source proof passes;
the applied adapter's public integration checks pass. The support extension and
actual policy publication are the next gates. Use
the audit's explicit source/consumer dependencies to retire complete-request
blockers. Static census and source evidence can proceed in parallel; actual-plan
reconciliation remains downstream of honest admission. Support-generated Skill
execution and resource-obligation signature/kind/quantum refinement remain open
proposals, not newly approved contracts.

The physical-singleton secondary-switch receipt is **applied, focused validation passing**.
Its reviewed candidate is retained under
`runs/secondary-global-receipt-candidate-20261020/`; original files are retained
under `runs/secondary-global-receipt-originals-20261020/`. Its source-backed domain
excludes missing Commands, other source effects, Direct/generated dispatch and
other usage consumers. Review found a conservative linear lookup charge would
consume 872,298 of the default 2,000,000 work units over Original05's 21 currently
eligible calls; the candidate now uses charged binary search over the existing
immutable sorted rows. That revision is prepared and reviewed:
the static same-identity lookup estimate is 8,514 work units, with a public
normalizer budget regression against 1,024 unrelated roles. These are not yet
executed runtime measurements. The fixture setup failures and an incorrect test
expectation for an unknown source were repaired. All 15 usage tests pass in
`runs/owned-secondary-global-receipt-tests-03.log`, and focused strict Clippy
passes in `runs/owned-secondary-global-clippy-02.log`; no new field responsibility is
published in a real build policy yet.

The next bounded source proof is group-selector neutrality for zero or one
possible active effect. Original05 group 238 (Ice Nova source 239) has literal
`nil` MAIN/CALCS selectors, one construction-complete physical active singleton
and three singleton supports. A joined Rust/mlua source test passes for
zero → one → zero admission, independent MAIN/CALCS and fresh/cached exporter
paths. All six selector and seven secondary-switch source tests pass in
`runs/owned-selector-source-tests-01.log`. Windows linking emitted an existing
`LIBCMT` conflict warning; it did not fail these tests. The reviewed adapter under
`runs/group-selector-receipt-candidate-20261020/` is applied. All 39 private
normalization tests pass in `runs/owned-group-selector-normalize-tests-01.log`;
377 normalization and 20 usage integration tests pass in
`runs/owned-group-selector-integration-tests-01.log`.
It shares the existing charged source-role lookup and singleton construction
predicate with the secondary-switch adapter; role-specific field permissions
remain separate. It uses an existing private input
receipt to account for exact default selectors after checking the whole physical
group. Support companions retain their separate count/global obligations.
This is not positive
admission, native execution or whole-preset completion; additional/missing
effects, another active row and any present generated-source attribute remain
outside the proposed domain. No public contract change is proposed for this step.

**Next actual-build step:** Original05's selected preset contains 16 physical
support occurrences across 12 Gem definitions. All have observed singleton
construction and saved `count=1`, `enableGlobal1=true`, `enableGlobal2=true`.
The current support input receipt intentionally omits these three fields, leaving
48 separate field obligations. Extend that existing private receipt only after
the actual source consumers prove non-consumption for a complete singleton
support. An arbitrary direct query to PoB's count helper is insufficient:
a synthetic Vaal support can read its count even though no active effect was
admitted. Additional active children are a separate, explicitly excluded case.
Three new source-component controls are applied to the existing secondary-switch
test target. All ten tests pass in `runs/owned-support-usage-source-tests-01.log`;
the sealed source packet is unchanged. Four positive tests exposed a producer
coupling: source proof was requested only when the scalar parameter inventory
was incomplete. The existing SupportAssignment path now also requests the same
independent proof for complete scalar parameters, retaining those values if
source validation refuses. No second proof implementation or primary allocation
change was added. The 25-test usage suite includes Complete/Partial equivalence
and preservation controls; its post-fix run passes in
`runs/owned-support-usage-backup-tests-01.log`.
The 39 private normalization checks and 377 normalization integration tests also
pass (`runs/owned-support-usage-normalize-tests-01.log` and
`runs/owned-support-usage-integration-tests-01.log`). Strict Import all-target
lint and the scoped PoB support-source lint also pass
(`runs/owned-usage-receipts-clippy-03.log` and
`runs/owned-support-usage-source-clippy-02.log`). The real policy publication
remains a separate gate.

Then author and validate the real usage-inventory policy in the current package,
including all remaining source fields and independent consumers. Group 238
(Ice Nova 239 and supports 240–242) is the first real joined case. The publication
must preserve unchanged input builds, exact typed values, all other obligations,
all 110 queries and fresh-repeat determinism across all five originals. Do not
stop at synthetic positives or infer whole-preset completion from this group.
Support receipt and publication candidates are retained under
`runs/support-usage-receipt-candidate-20261020/` and
`runs/owned-usage-inventory-publication-candidate-20261020/`; the support adapter
is applied. The first publication run compiled and executed: Originals 1–4
passed whole-output preservation and fresh-repeat checks, then Original05's
successor normalization failed with `normalization exceeds work`
(`runs/owned-usage-inventory-publication-tests-01.log`). The candidate uses
16 existing policy identities and 337 field declarations; other consumers remain
explicitly Deferred. Default limits remain unchanged. The reviewed early-refusal
fix now stops unnecessary proof work once a preset is known to remain Pending,
while preserving whole-source validation, independent presets and atomic
retirement. Its 13 private and 27 public usage tests pass
(`runs/owned-usage-early-refusal-{private-tests,tests}-01.log`).
The second all-five run still failed Original05 at the default work limit after
the first four originals pass. The diagnostic run
`runs/owned-usage-inventory-work-diagnostic-03.log` locates a 48,860-unit
all-receipts-by-draft precharge in `inventory::complete`, raising cumulative work
to 2,004,672 against the unchanged 2,000,000 limit. Temporary caller logging is
removed. This identified the need to charge cheap exact-identity filtering, then
deep validation only for matching receipts, retaining all live checks
and atomicity. Early refusal alone did not solve this measured failure.
Failed output directories remain evidence, not validated endpoints.
The exact-matching correction from
`runs/usage-inventory-matched-receipts-candidate-20261020/` is now applied and
independently reviewed. All receipts pay for identity filtering; only matches
incur the unchanged live output, origin and membership checks. Matching usage
binding comparisons retain the compiled 64-parameter bound. All 18 private
completion and 27 public usage tests pass
(`runs/owned-usage-matching-{private,public}-tests-01.log`).
The fourth real publication run passes at default limits:
`runs/owned-usage-inventory-publication-validation-04/validation.json`, successor
`593fb1facdc2a85825d3deb090a322978c400431b1742f8b19bdc936fdf69a5c`.
Every original retains its whole draft/sidecar/report and independently repeated
output; the package rebuild is byte-identical, all 110 queries remain, selected
Pending counts remain 107/118/110/124/5 and no supplied build becomes complete.
The log is `runs/owned-usage-inventory-publication-tests-04.log`.
The real group238 positive/nonneutral control also passes:
`runs/owned-ice-nova-group-usage-positive-validation-01/validation.json`.
Only the derived group's usage inventory and its exact issue links retire;
all other content and allocation watermarks stay identical. Changing the first
support count from 1 to 2 keeps usage Pending and preserves the entire output
between the prior and successor policies. Checked finalization remains Pending.
Stopping work on an already refused preset does not establish complete-build
scaling. The new complete 16/32-preset controls exclude the removed cubic
precharge, but identity scans and relevant draft membership checks still have
quadratic costs. No claim of arbitrary larger-corpus scaling follows.

The authenticated selected-preset audit under
`runs/selected-usage-next-blockers-20261020/` identified the next data-only step:
nine missing support-family rows cover thirteen selected occurrences and thirty-nine
neutral count/global fields. Their existing physical-support rules, singleton
construction, resolved Gem inputs and exact authored targets are present.
These additions are now applied to the same unshipped maintained packet at
`data/owned/poe2/3887ae68/preset-usage-inventory/`, with 25 identities and 454
field declarations. `tests/owned_preset_usage_inventory_cli.rs` is the sole
maintained target for the packet, all-five publication, original group control
and the new real-byte support expansion controls. The latter compare the same
derived input under the full packet and conservative omissions, including each
of the nine families separately. They test field accounting, not support
eligibility or gameplay legality. The ordinary packet check, all-five publication
and original group control pass, as does scoped CLI lint. The 25-row successor is
`fc0d9af5c0651853142b3f06a94a7eee4f4538d201857bc825da49232ebe0497`
under `runs/owned-preset-usage-inventory-publication-01/`; selected counts and
all 110 queries remain unchanged. The temporary Cargo targets are removed.

The first expanded control failed honestly: its inserted `groupCount="1"` lay
outside the existing physical-support source frame, leaving all sixteen
supports' scalar inventories Pending. Removing that insertion alone is
insufficient: the existing Ice Nova count fallback admits only five reviewed
companion identities. The nine new support families also need explicit
noncompeting count-companion admission through that existing policy. Their
complete singleton constructions justify the added finite tuples as data,
keeping original source files and runtime source-frame checks unchanged.
The correction removes the group-header rewrite and retains both
inventory-omission and count-companion-refusal controls. Failed evidence:
`runs/owned-preset-usage-inventory-support-expansion-01/` and
`runs/owned-preset-usage-inventory-support-expansion-tests-01.log`.

The reviewed correction is applied and all gates pass at default limits.
`companion-additions.json` appends nine exact singleton Support tuples to the
existing five count companions. Tests authenticate current compiler roles and
the complete source consumer; they preserve all thirteen copied Gem byte ranges
and the unchanged group header. Both ordinary checks pass, including malformed
construction/identity controls. All-five publication produces
`e355f2db6ad73810c417006e288a847bede2f9a195f1df471f53fdcd4e95eafb`
in `runs/owned-preset-usage-inventory-publication-02/`: full output preservation,
independent fresh repeats, byte-identical rebuild, all 110 queries and unchanged
selected Pending counts. The original group control passes under
`runs/owned-preset-usage-inventory-group-positive-02/`. The expanded control
passes under `runs/owned-preset-usage-inventory-support-expansion-02/`:
all nine inventory omissions preserve the entire unresolved output; all nine
companion omissions and the known-but-unreviewed sibling preserve Pending count
and usage obligations with independent repeated output checks. This validates
import accounting, not support eligibility or native execution.
Logs: `runs/owned-preset-usage-inventory-{packet,publication,group,support-expansion}-tests-02.log`
and `runs/owned-preset-usage-inventory-clippy-02.log`.

After that, converge private primary completion and final-destination plumbing
using the existing `InputReceipt` and `InputOutput::Authored`. `AwaitUsage` and
`AwaitDisposition` currently carry different proof authority: the former
requires a designated attached usage policy; the latter proves source Action
correspondence. Frost's Partial empty output inventory prevents blindly replacing
the former with an Action-requiring policy. Preserve each actual proof scope and
allocation timing during cleanup. Pain Offering/Frost Bomb requested-count
recipes remain separately absent. Missing minion Commands, two-effect Djinn
selection and generated Firebolt dispatch also remain genuine consumer gaps;
the physical-singleton proof cannot account for them.
The private completion cleanup from
`runs/primary-input-completion-candidate-20261020/` is applied. Revision 2 retains an exact
reserved issue ID in the existing prepared result rather than deriving authority
from a diagnostic code; it preserves the first revision separately. It removes
the Complete-support fallback, retains narrow flat-primary proof scope and the
exact live designated usage binding, and adds six producer plus three private
regression tests. The initial run exposed three incorrect private fixtures and
one real occurrence-only usage-origin failure (18/21 private and 32/33 public
passed). The fixture had removed a contributing group link while retaining its
group-enabled consumer. Separately, the live usage checker required a group link
even when the producing physical adapter read only occurrence values.

The reviewed correction from
`runs/usage-receipt-origin-scope-candidate-20261020/` is also applied. The existing
private receipt now records the adapter's actual group participation. One shared
check verifies exact source/group/set ancestry and source/set preset ownership;
contributing groups require their sole link, while noncontributing groups may be
absent but cannot contradict the preset. It fabricates no links, changes no
public format, and retains exact binding checks and charged traversal.
All 22 private inventory checks, the 10 existing Gem proof checks, 33 public
usage checks and 377 normalization checks pass. Strict Import all-target lint
also passes. The CLI packet, actual all-five preservation, group and support
controls, and strict CLI lint now pass. Logs use the
`runs/owned-primary-completion-*-03.log` prefix; the support controls completed
in 207.39 seconds. The maintained publication test now optionally compares
with a successful previous-code run using
`POE_OPTIMIZER_TEST_USAGE_INVENTORY_BASELINE`; exact package identity and every
saved output are authenticated before comparison. Use the successful
`runs/owned-preset-usage-inventory-publication-02/` baseline for this code cutover.
The successful `publication-03/previous-code-baseline.json` records byte-identical
packages and identical complete checked outputs for all five originals.
Do not infer that this cleanup supplies missing Pain Offering/Frost Bomb count
data.
The next data audit confirms the existing requested-count policy can serve both
sources, but the current flat-primary proof still requires explicit count guards
and rejects a containing `groupCount`. Keep those limits explicit: Pain's
occurrence guard admits 1/3, Frost's admits 1; the generic numeric schema's 0–4
domain does not broaden that source proof or establish gameplay limits.
Retain the guards for the unchanged-build checkpoint and track their replacement
with checked consumer evidence as import breadth/cleanup work.
The source/dependency audit is retained under
`runs/primary-input-guard-debt-audit-20261020/`; the compiler requires named
guards, while the injected data supplies their allowed values. The reviewed
count-data candidate is under `runs/offering-frost-count-inventory-candidate-20261020/`.
The follow-on maintained Rust candidate is frozen and independently reviewed at
`runs/offering-frost-count-stage-candidate-20261020/` (manifest
`4dae67ef58d59febf318998a9d132011212e399350426ada40f246843b42a991`).
It is applied and validated. It folds the two count policies and two
inventory row changes into the current packet/stage, without a parallel
publisher or compatibility path, and checks exactly eight saved count bindings
against their real source occurrences while preserving all other output and
allocation state. The CLI run uses
`runs/owned-primary-count-*-04.log` and publishes to
`runs/owned-preset-usage-inventory-publication-04/`; this validated package now
supersedes `e355f2db…` as the current endpoint.
The ordinary three packet checks and all-five publication pass
(`fd869107…`): eight exact known count bindings are added, with no allocated IDs
and complete preservation of all other output. The first isolated-group gate
found an Ice Nova-specific test expectation, not a producer failure: it required
the usage issue on both group and Gem, while the flat-primary producer attaches
it only to the Gem. The shared test comparison now requires explicit exact
expected origins; Ice Nova/support controls retain both, and the new controls
require the Gem. Full draft/sidecar/report and allocator inverses remain.
The corrected controls pass under `owned-primary-count-controls-tests-05.log`
(106.34 seconds): three exact positives and 26 independent refusals, including
fresh replays and complete output preservation. Their receipt is
`runs/owned-preset-usage-inventory-primary-counts-05/validation.json`.
The successful publication is reused unchanged. Existing group and expanded
support controls pass against the new package (support: 212.09 seconds), as does
strict CLI lint. Native mechanics and all five original Pending issue counts
remain unchanged.

The next generated-source audit is retained under
`runs/generated-firebolt-dispatch-review-20261020/`. Firebolt has one resolved
effect and an additional stat-set alias, not two effects. Its generated-input
receipt already authenticates the exact item provider, inputs and live usage;
secondary-switch and group-selector consumers still need the checked Import
adapter and catalog authority described below. Reuse the current generated
receipt with the validated reconstruction and catalog/direct source evidence.
Shared Djinn construction evidence does not
by itself establish the same selector behavior. The source-component test
candidate is separate from native mechanics. The frozen
candidate at `runs/generated-firebolt-routing-source-candidate-20261020/` has
passed independent source-span and assertion review and is applied. A further review
identified the narrower required proof: selector and secondary-switch meaning
must remain equivalent when PoB replaces the saved generated group, not just
when it retains the same object. Level, quality, count and native provider
correspondence remain independently checked responsibilities.
The independently reviewed supplemental replacement test is also applied.
The first fourteen-test run passed eleven tests and exposed three test-host
failures: the new Lua chunk lacked the existing authenticated export helper's
lexical binding. Binding `activeSkillGems` to that same `exportActive` function
fixes the host without changing any extracted source body. The four reviewed
constructed-selector tests are now also applied; all eighteen source tests pass
in `runs/generated-routing-and-selection-source-tests-02.log`.
Strict target lint also passes in
`runs/generated-routing-and-selection-source-clippy-02.log`. No source-test result itself grants an
Import receipt, native completion or numerical parity.

Generated catalog uniqueness has one additional contract decision:
[explicit reverse-routing facts](owned-generated-catalog-routing-proposal.md).
The loaded role index structurally permits a subset; its caller-supplied count
cannot prove that an omitted primary would not collide in PoB's reverse map.
Full offline reproduction already establishes that fact against the actual
catalog. Prefer carrying the complete compiler-derived candidate observation in
the existing Import construction artifact rather than requiring the source
catalog during ordinary loading. This is proposed, not implemented; the owner
has been prompted for the contract decision. The bounded omission regression
and strict catalog-target lint pass in
`runs/generated-catalog-omission-regression-{tests,clippy}-01.log`.

The [remaining-usage audit](../runs/owned-remaining-usage-blockers-20261020/audit.md)
identifies the next independent path: source-selector accounting over actual
constructed order and literal secondary-`true` accounting, then the existing
inventory destinations. Preserve missing skeletal Commands and native Partial
coverage. Manual Djinn needs two-effect ordering evidence; generated Djinn's
secondary `nil` requires actual global-effect meaning and cannot be translated
to generic disabled participation. Existing minion-reference mappings already
distinguish the Sand unit's MAIN/CALCS actions from a player Command selector.
The independently reviewed four-test source candidate under
`runs/constructed-source-selector-candidate-20261020/` reuses the current source
target, is applied and passes with the other fourteen tests. It leaves
the strict source-construction predicate unchanged. Reuse these bounded
selector facts in the existing private Import path next; a source list and
actual dispatch must not be mistaken for complete native mechanics.

### Authored source selector and neutral-gate follow-up, October 20

The optional source target now passes nineteen tests, including an explicit
secondary-true control over actual constructor order, Vaal/non-Vaal count
handling, absent/false/true global metadata and independent disabled, support
and hidden gates. Counts zero and seven are preserved. This proves the narrow
non-restricting gate, not non-consumption, native activation or missing Command
coverage. Evidence: `runs/owned-secondary-true-source-{tests,clippy}-01.log`.

The applied private Import adapters retain the current public model: primary
selection may use the actual constructed list while preserving declared missing
references; authored physical/Direct inputs may account for an exact secondary
`true` after their existing input proof. Supports keep their stricter construction
proof, and generated sources retain independent reconstruction requirements.
Both candidates passed independent review. Their focused tests retain mapped
addition/alias refusals, exact live recipients, work limits, independent presets
and every other consumer. The first private run passed 24/25: an older negative
case incorrectly required the newly supported secondary `true` to refuse. It now
uses `false`, preserving the independent unread/construction requirement. The
first public compile exposed two test-only API mistakes; the corrected test uses
complete serialized sidecar comparison and the existing checked-finalizer test.
The Import library passes all 224 tests in
`runs/owned-authored-selector-regression-tests-02.log`. That run exposed a missing
`source` declaration in the new Direct test fixture's allowed group frame;
adding the declaration changes neither source data nor production behavior.
The corrected public suites pass all 28 Direct, 377 normalization and 39 usage
tests in `runs/owned-authored-selector-regression-tests-03.log`. No real inventory
destinations have changed yet. Strict Import all-target lint now passes in
`runs/owned-authored-selector-clippy-01.log`. The maintained all-five previous-code
comparison passes in `runs/owned-authored-gates-preservation-01.log`: the exact
`fd869107…` package, complete outputs and independent fresh repeats match the
previous code. Selected issue counts remain 107/118/110/124/5. The nineteen
source tests and their strict lint already pass. Hosted CI remains a separate gate.

The private reference-absence candidate has now been independently reviewed and
applied, along with two actual Direct-producer regression tests. The current
checks pass 713 tests: Import library 227, normalization 377, usage 43,
physical/Direct dispositions 11/30 and the three source-action targets 9/8/8.
The broad log is `runs/owned-reference-absence-regression-01.log`; its sole failure
was a new unsupported-child assertion expecting no usage list. Existing independent
projections correctly survive with Pending completion. The corrected usage suite,
including explicit physical allocator preservation, passes in
`runs/owned-reference-absence-usage-tests-02.log`. Production code needed no fix.
Strict all-target Import lint passes in `runs/owned-reference-absence-clippy-01.log`.
Unchanged all-five preservation passes in
`runs/owned-reference-absence-preservation-01.log`, including exact package bytes,
whole outputs, allocator state and independent fresh repeats.
The reviewed four-row skeletal packet integration is now applied. All four
ordinary checks pass in `runs/owned-skeletal-inventory-ordinary-01.log`; changed
packet publication also passes in `runs/owned-skeletal-inventory-publication-01.log`.
Its byte-identical rebuilds, all-five whole-output inverses and fresh repeats
preserve all 110 queries and selected issue counts 107/118/110/124/5. The eight
earlier count additions remain the only original-build value changes relative
to the fixed predecessor. All four explicit real-group suites pass in
`runs/owned-skeletal-real-controls-01.log`: unchanged Ice Nova, prior primary-count
controls, four skeletal positives with 27 refusals, and expanded support/companion
controls. The 31 skeletal cases preserve complete outputs and independent fresh
repeats; unknown source tokens retain generated-source obligations rather than
creating physical records. Strict scoped CLI and PoB lint pass in
`runs/owned-skeletal-inventory-clippy-01.log` and
`runs/owned-manual-direct-selector-source-clippy-01.log`. The current validated
release is `e8df8bdb…` in `runs/owned-preset-usage-inventory-publication-05/package`.
These follow-ups are included in this backup checkpoint; the nine protected
incoming-critical work-in-progress files remain local. GitHub `main` was
verified at `4aa10b6e` immediately before this follow-up backup, then verified at
the new backup `00d3c62a`. A public GitHub snapshot at 19:43 UTC shows run
`39485418554` for that commit pending with no jobs; the earlier `4aa10b6e` run
`39481892743` was cancelled without jobs. Neither is hosted test evidence.
The snapshot is `runs/ci-00d3c62a-public-review-20261020/`.

The real-packet review also found a subsequent absence-accounting blocker.
Skeletal inventory rows declare `statSetIndex`/`statSetIndexCalcs`, while the
selected original omits them; the current reference receipt records only present
attributes. Original01 also omits a declared group label. Keep these fields in
the identity-wide inventory. Reuse the successful source-reference/default and
presentation proofs to account for absence explicitly before enabling the four
skeletal inventory rows. Merely changing Deferred destinations cannot close
those rows, and neither change closes native numerical coverage.
The exact follow-up is retained in
`runs/skeletal-usage-integration-review-20261020/{README.md,audit.json}`. It covers
21 saved occurrences across Original01 and Original05 and preserves all 25 rows,
456 fields and 20 existing typed destinations. The private absence facts now
reach the four updated rows through the same maintained packet. No additional
publisher or full-release evidence copy was introduced.

The absence transport retains the same checked reference result through the
existing physical/Direct path. Six finite facts distinguish absent overwritten
scalar headers from explicitly permitted missing minion profile/action selectors.
Both reference contexts and all saved descendants must already be accounted for.
Final attachment rechecks exact source, parent, descendants and live recipients.
Missing physical group labels remain a separate presentation fact. Generated
sources retain only their prior descendant scope; flat primary proofs gain no
reference absence authority. No native defaults, public contract or game data
change. Evidence and the independently reviewed candidate are in
`runs/reference-absence-receipt-candidate-20261020/`; the additional actual Direct
controls are in `runs/direct-reference-absence-test-candidate-20261020/`.

After the four skeletal rows, review the two authored Direct Djinn groups next.
Their remaining private gaps are group-selector accounting and exact absent
manual-source accounting. The source-selection proof must first cover the
two-effect ordering and late-hidden-primary boundary; do not infer it from a
catalog role. The bounded investigation is
`runs/manual-direct-selector-priority-20261020/`. Generated reverse-catalog routing
remains a separate public proposal and is not authorized by this Direct scope.
Four source-component tests for that boundary are now applied to the existing
`owned_preset_reference_selection_source` target. All ten tests in that target
pass in `runs/owned-manual-direct-selector-source-tests-01.log`; scoped lint
also passes. They reuse
authenticated full loader/processor spans, actual saved groups and support
declarations, with the existing explicit identity-only active-skill factory.
They check source/effect object identity and retain the deliberate late-hidden
counterexample. They do not establish numerical parity or authorize a Direct
Import adapter by themselves. The independently reviewed candidate is
`runs/manual-direct-selector-source-candidate-20261020/` (manifest `01cfcc0a…`).

The private Direct adapter now passes 683 selected Import tests, strict all-target
lint and unchanged all-five publication against the exact `e8df8bdb…` baseline
(`runs/owned-manual-direct-selector-preservation-01`). The maintained two-row
Direct inventory overlay is applied from
`runs/manual-direct-usage-inventory-candidate-20261020/` (manifest `6782c21b…`).
It changes 50 destination lists while preserving all ten typed consumers, all
25 rows and all 456 fields. Publication, the 17 actual-group controls and all
earlier controls remain required before treating this data checkpoint as validated.

The next tree-Djinn switch audit is retained in
`runs/tree-djinn-secondary-switch-audit-20261020/`. Twenty-four authenticated
baseline observations retain both exact Commands despite saved secondary=false;
their complete raw ExtraSkillStat chains are empty. This is finite input evidence,
not identity-wide absence authority. Existing nested Skill identities and
application activation can distinguish a real effect from local Command/minion
execution, but the current generated usage correspondence targets only the primary
and neither Command has an authored global application. Do not invent one merely
to consume an import field. Water's entering Command grant also needs its actual
tree-root allocation permission checked; Sand already carries that permission.

In parallel, the next native slice is conditional equipment movement penalty.
The source/profile value `0.03` is a dimensionless fraction, but current owned
Stat `29ff` incorrectly declares percentage-point unit `0002`. Correct that
metadata to existing unit `0001` without changing raw numbers, prove the full
inverse, and update the maintained profile assertions. Historical sealed releases
remain historical evidence; do not rewrite their seals or add a compatibility
reader. Reuse the existing Actor slot reads and Boolean contribution graph for
the equipment producer. The candidate begins with the body slot and retains
Partial supplier coverage: it does not prove final MovementSpeed, complete
equipment coverage or an absent ignore-penalty flag. Source, native unit/value,
missing/unknown inputs, query independence and repeated/parallel execution checks
are required before publication. The authored ingredient and tests are applied;
its current-recipe correction check passes in
`runs/owned-movement-penalty-recipe-check-01.log`. Full release authoring reuses
the existing private dependency rebinder, with a temporary test-only registration
and no new production repair API. Its reviewed candidate is
`runs/movement-penalty-release-candidate-20261020/`; no publication has run yet.

The selected-mechanics audit in
`runs/ignore-movement-coverage-audit-20261020/` supplies the next closure order.
It is a static positive-declaration audit of the validated `e8df8bdb…` package,
not a compiled request or a new complete-build claim. Of 57 reached passive
owners, 45 are Complete; the remaining twelve concern Djinn roots/child unlocks,
Puppet Master, Archon of Undeath and Command scaling/resource behavior. All 18
selected Reward owners and the selected Ascendancy are already Complete.
Reconcile existing evidence before reimplementing those completed responsibilities.

All 43 Partial contribution queries in that package have retained readers on the
actual Sniper path: 20 Player Life/Spirit/incoming-critical receiver queries,
19 outgoing-critical/armour queries and four inherited-damage, block or Weakness
queries. The new movement-immunity query adds another selected dependency.
Completing a member list does not close Partial selected owner programs: an
unimplemented owner could still supply a modifier or change a descendant. Keep
that guard. Catalog-wide query relevance is separate possible debt, but orphan
query removal would eliminate none of these current blockers. Scoped omitted-effect
proof remains Proposed, not an approved shortcut to a single known metric.

After input admission, replace the static counts with actual compiled gaps.
Before then, prioritize existing source/admission/delivery evidence for the nine
selected Modifier families and eight ItemTemplate owners, then the selected
Gem/Skill/Actor/Action families and their exact query producers. Do not substitute
new off-hand arithmetic or unrelated catalog cleanup for these dependencies.

The parallel non-usage audit confirms the subsequent configuration checkpoint:
all 42 saved Config leaves in Original05 already have destinations. None of the
four remaining non-usage gates can honestly close by adding another isolated
numeric input. Reuse the accepted complete constructor/default census and the
existing configuration adapters' checked receipts. Current partial-row accounting
intentionally has no permission to complete owner inventories. Begin with the
selected ConfigSet423, while preserving all thirteen linked fallback origins:
the Config/ConfigSet pair and eleven outside the Config subtree (Build,
TimelessData, Tree, Skills, archived generated Firebolt, Items, Calcs and Import).
Each still needs its actual
selection, search, export or configuration responsibility established.

The existing default-input rows do not cover every source-installed default:
the authenticated ordinary-load report also records averaging modes, current
Mana percentage, conditional states/counts, cooldown bypasses and child
buff/curse switches. Reuse
`runs/owned-configuration-inputs-source-01/source-jit-off.json` and its pinned
source evidence; do not infer defaults from absent XML. Thirty-four transported
assumptions, an empty exact-usage list and two typed Scenario stage intents are
still independent incomplete inventories. Dormant intent resolution requires
the complete composed source census; it does not justify implementing a positive
support-child mechanic merely to transport that intent.

The concrete next configuration implementation is retained in
`runs/configuration-completion-audit-20261020/`. The acquired catalogue has 663
ordered rows, 564 named declarations and 563 distinct names. One name has two
different callbacks: preserve both consumers instead of deduplicating by name.
The current leaf loop cannot consume already-checked constructor/absence receipts
whose source origin is absent. Extend this same accounting owner with the accepted
complete control/branch census and one scoped receipt collector; keep local
ConfigSet proof separate from atomic retirement across every live fallback origin.
Reuse checked scalar/reward/Encounter/usage output correspondence, including exact
absence guards. Fresh ConfigSet structure alone does not account for custom blocks.
The 28 installed non-reward inputs are distinct from 563 UI fallback defaults.
The 24 controls beyond the existing scalar and Encounter proofs are now classified
in `runs/configuration-installed-defaults-review-20261020/`: all have mechanical
consumers, and none has a current matching checked receipt. Seven control children
of exact parent summons. This identifies responsibility, not complete applicability
or native coverage. The independently reviewed combined candidate is
`runs/configuration-inventory-candidate-20261020/` (manifest `c17183e9…`),
which supersedes the earlier private collector candidate. It retains the existing
adapter order and no-origin/Encounter-absence receipts, adds an independent
source-control census, and stages whole-issue retirement only after every required
consumer and linked origin is checked. Partial or empty censuses cannot close an
inventory, and its initial domain cannot retire the eleven outside origins.
The current PoB census remains incomplete. The candidate is unapplied and untested;
its fixture census supplies no authority for a real build. Do not install all UI
defaults as native assumptions or infer irrelevant
behavior from hidden controls. Public-normalizer fixtures must exercise absent,
constructor, authored false/zero/None, Placeholder and duplicate-consumer branches,
then prove exact all-origin retirement or unchanged refusal on Original05. No
new Core/Engine model, parallel configuration adapter or compatibility reader is
needed by this accepted milestone.

The installed-default priority map separates fourteen scalar/mode/relation controls
from ten scoped Skill controls. Transport repeat evaluation mode first through the
existing Option-valued Environment projection, then physical-reduction and resource
expectation modes. The repeat-mode authorer must allocate from an authenticated
current release and reuse the maintained default-input stage; its input publication
will not implement repeat mechanics or complete any inventory. Three cooldown
bypasses and seven child switches need Boolean constructor transport in the existing
Scenario-intent adapter, which currently accepts only numeric saved values. Preserve
exact parent/child applicability; global Player flags are not a substitute.
For these child switches, false suppresses the Config producer rather than writing
final participation=false. Also, a Partial applicability declaration still applies
its known matches. The current final GrantSlot/SkillGrantSlot match does not by
itself prove an ancestor summoning Skill, so a broad child allowance cannot be made
safe by adding a Partial label. Investigate existing exact topology evidence or an
explicit ancestor contract before authoring those seven rows. Boolean constructor
transport itself can reuse the existing Boolean value/default and Scenario-intent
types without a new data-transfer object.

The selected Modifier audit found actual remaining delivery work behind some
Partial labels: movement modifiers need grouped equipment-slot bonus copies, while
Charm capacity needs equipment-local aggregation followed by a single Player
delivery. Reuse the existing equipment receivers and contribution graph where
possible, preserving local consumption, grouping and scaling. Do not close an owner
because the current observed modifier value happens to be neutral or already emitted
by its direct path. Candidate authoring and source controls are still in progress.

### Native Scenario intent checkpoint, October 20

The approved [Scenario intent contract](owned-scenario-usage-intent-proposal.md)
is implemented across Core, Data and Engine. Core retains Build
preferences separately from exact Scenario overrides and stores typed Scenario
intents. Data adds required explicit applicability over authored Skills and exact
grant/supply relations. Engine uses shared
structural traversal so disabled/off-loadout sources cannot be mistaken for an
empty recipient set or accidentally enabled.

This local checkpoint is validated. Input/draft envelopes are now 5 and request identities
are v2; definition schema 7 is updated in place with the required UsagePolicy
field. Authored schemas, current replay fixtures and the latest release have been
rebuilt with inverse/preservation checks. The importer conservatively introduces
one additional Pending intent inventory per Scenario until the configuration
adapter and accounting prove it complete. Earlier issue counts below describe
the preceding format, not a claim that the new inventory is already resolved.

Preceding native-contract endpoint (the new Import fields are rebuilt above):
**`runs/owned-scenario-usage-format-publication-02/package`**, input
`7b4205fd92b8f993e0f0be82367785e0534acab5c12f6db132233bc1cffae24f`.
All five unchanged originals now reimport with whole-draft and whole-sidecar
inverse checks and identical fresh repeats. All five query artifacts remain
byte-identical (110 rows). Each Scenario gains exactly one separate Pending
intent inventory; no mechanical coverage is closed by this migration.
The CLI confirms the original saved selections and no schema issues in all five;
selected Pending counts are **107/118/110/124/5**. The three retained typed source
snapshots and their dependent reviews have been reauthored with explicit inverse
checks and unchanged source observations/mechanical values. Sidecars remain below
the existing 2 MiB reader bound. No migration hooks remain in Import's library.
The 321 Core tests, 39 scoped Data tests, 363 normalization tests, 26 usage-policy
tests and 45 Sniper replay tests pass. The 49 scoped preparation, participation
and requirement-assessment regressions also pass. Another 74 CLI/direct-evidence
and 77 downstream-evidence tests pass; ten authoring tests remain intentionally
ignored in those suites. Strict library, CLI and scoped test lint pass with
warnings denied. The owned-library boundary check also passes: Data and Engine
compile without PoB, Lua or Import dependencies. The configuration-to-intent
transport follow-up is now implemented and tested above. Actual control data and
source-backed structural census remain prerequisites for re-evaluating admission.
The campaign endpoint below remains the preceding validated checkpoint.

Evidence: `runs/scenario-format-release-authoring-02.log`,
`runs/owned-scenario-usage-format-publication-02/{format-proof,import-preservation,selected-validation}.json`,
`runs/scenario-format-selected-cli-01.log`,
`runs/scenario-draft-evidence-authoring-03/report.json`,
`runs/scenario-artifact-pin-authoring/report.json`, and
`runs/scenario-review-pin-authoring/report.json`. The latter two repair exact raw
artifact references; semantic review commitments come from the existing checked
reconstructors, not token replacement. That native-contract checkpoint's release
receipt SHA-256 is
`57579994f8eabb12c556e5467181b81a19e32661b67af57916445aad634e60d6`.
Final validation logs:
`runs/scenario-intent-{core-tests-03,data-tests-01,normalize-tests-01,engine-tests-01,engine-tests-02,engine-tests-03}.log`
(the later Engine log supersedes the repaired usage/reporting failures in earlier runs),
`runs/scenario-intent-{cli-evidence-tests-01,dependent-evidence-tests-01}.log`,
`runs/scenario-intent-{library,cli,core,data,engine,import}-clippy-01.log`,
`runs/scenario-intent-owned-boundaries-01.log`,
and `runs/scenario-intent-{format-check-02,diff-check-03}.log`.

Evidence-maintenance finding: some historical source reviews deserialize retained
drafts through the current runtime DTOs. Updating those drafts also changes
transitive review and publication commitments. Preserve their structural checks
for this migration, but include this coupling in the debt review: distinguish
immutable historical observations from current executable fixtures, and avoid
requiring unrelated source evidence to be rewritten for every input-model change.
Do not solve this by accepting obsolete runtime formats or weakening source pins.

### Campaign progression and same-attempt requirement assessment, October 19

Preceding campaign endpoint (superseded by the Scenario-format checkpoints above):
**`runs/owned-campaign-progression-publication-02/package`**.

- Input: `c9d047cc2acae7bec4d6ce384b335d570906fa1d2f32f8b355e7cb2b77a564a5`.
- Registry: `5f7a897104b1fad26e945de64d727c482712e49b42bf40b6cbf53d16ebdbec7e` (last `33b7`).
- Definitions: `d6d8e740b9e749a9a466cf1ea46bfabb9bfa4e20c67038d1ef1e538f5bb9a865`.
- Rules: `a1b95df4ba7573f5d17181c816f778707fc0c4d4748c0e990cfe5f046c3f915e`.
- Compiled: `5e486ce6bb02fcba88a36fc7435e46bbf85b7de5ab07051c8bd0ed5a48b4982f`.

The [campaign packet](../data/owned/poe2/3887ae68/configuration-campaign-progression/README.md)
imports each saved count into the existing Choice Reward model. Actual values
are **55/57/57/56/56**, retained as source evidence, never runtime defaults.
Both present source lanes must validate; missing/all-zero selection remains
Pending. Native rules independently check one exact authority, including two
distinct sources with equal values. A consumer requiring the fact must also
reject absent selection. The supported integer bound is representational, not
a proven maximum campaign count.

Publication preserves all five complete draft/sidecar relationships through an
injective identity correspondence, all 110 query bytes, and selected issues
**106/117/109/123/4**. Byte-identical rebuild and six actual import mutations pass
(32.62s initially; final repeat 35.96s). Original05's configuration issue origins
fall **16 → 15**: ordinal 450
now links to its exact Reward and Choice preset. The four grouped input
obligations remain; no existing owner/query closure or complete evaluation bundle
was added. The package retains all 17 prior reward recipes and adds one numeric
recipe, two programs and two contribution queries.

Metric reports now include exact Requirement outcomes and separate coverage from
the same ordinary/support execution. Prefix violations survive failed support
preparation; unavailable requirements remain incomplete. Measurements are retained.
That checkpoint introduced `owned-metric-plan-v2` to invalidate earlier report
caches; the selected-owner correction above now uses `owned-metric-plan-v3`. New tests
cover exact duplicates, missing inventories, changed queries, and fresh/reused/
Rayon scratch. Independent runtime review found no actionable issues. Action
context census and complete candidate admission remain explicitly unfinished.
The current package contains two known Action Requirement programs:
`intrinsic-action-coefficients` and `sniper-ordinary-flat-reservation`. Thus the
conservative census gap is a real remaining admission gate, not only a future
extension concern. Finish metric-independent mechanical discovery and the already
accepted Skill-owned resource-obligation migration before claiming complete
requirement assessment. Do not omit a rule because its Action was not queried.

Source archive validation caught Windows CRLF checkout bytes; the corrected full
bodies match the independent pinned manifest exactly. No source assertion or
domain boundary was relaxed. Final validation passes:

- 70 scoped Engine checks across ordinary/support metrics, exact authority,
  requirements, binding, required effects and support delivery. The original mixed
  fixture test was split into ordinary/support targets to remove duplicate modules.
- 436 focused Import tests after boxing large enum payloads. Independent wire and
  digest assertions preserve both current finite and numeric policy shapes.
- 19 final CLI/component tests, including ordinary/support reports and actual
  campaign rules, plus the explicitly invoked all-five publication. The repeat
  produces the same input identity and byte-identical receipt as publication01.
- Strict Clippy for the scoped Engine, Import and nine CLI targets; formatting
  for all 50 changed scoped Rust files. The actual owned-only compiled-source and
  dependency check passes with no PoB, Lua or Import in owned Data/Engine.

Final logs: `runs/owned-requirement-assessment-engine-tests-{02,05,06}.log`,
`runs/owned-requirement-assessment-engine-clippy-02.log`,
`runs/owned-numeric-reward-adapter-{tests-03,clippy-04}.log`,
`runs/owned-requirement-campaign-{final-tests-01,cli-clippy-01}.log`, and
`runs/owned-requirement-assessment-boundaries-02.log`. The boundary checker used
the bundled Python executable because the Windows Store alias is inaccessible;
no new Python tooling was introduced.
Publication evidence: `runs/owned-campaign-progression-publication-02/{validation,receipt}.json`
and `runs/owned-campaign-progression-publication-02.log`.

**Current follow-up:** the configuration-to-intent transport is implemented and
tested above. Complete the structural source census and actual control publication,
then resolve remaining configuration accounting before re-evaluating Original05
admission.
Preserve these five unchanged imports. Final weighted hit/DPS still needs the
candidate-bound supplier and special-branch proof described below; more partial
arithmetic alone does not unlock a complete request. Complete builds remain **0/5**.

### Predecessor numerical and input evidence

The [selected physical-base packet](../data/owned/poe2/3887ae68/selected-physical-base/README.md)
publishes the actual pre-conversion Basic Attack formula and two checked
self-flat contribution queries. Its original selected endpoints are **208–387**.
All potential native writers, including zero/inactive and application writers,
are checked against the finite source proof; existing global, owner and input
Partial boundaries remain intact. Four ordinary checks, all-five publication,
102 distinct joined tests, strict Clippy and public replay export pass. The
[packet](../data/owned/poe2/3887ae68/selected-physical-base/README.md#checks)
records the test-helper failures and exact validation scope.

The following [trade-search preference packet](../data/owned/poe2/3887ae68/source-trade-preferences/README.md)
accounts for saved trade weights through the existing opt-in import policy.
It preserves names, duplicate rows and finite signed weights as source
preferences; these do not become native gameplay inputs or inferred optimizer
objectives. Complete loader and trade-entry-point evidence keeps future trade
intent distinct from gameplay. All five originals and 110 queries reproduce
exactly. Original05 configuration origins fall **19 → 16** (five inside Config
unchanged; fourteen → eleven outside). Its **33 known assumptions, 35 usage
preferences, 20 usage links and four Pending input inventories** are unchanged.
Selected issue counts remain **106/117/109/123/4**; complete native builds remain
**0/5**, with no complete evaluation bundle published.

The subsequent [selected physical-hit packet](../data/owned/poe2/3887ae68/selected-physical-hit/README.md)
now computes **574–1068** before critical multipliers and enemy mitigation.
It applies semantic Intimidated/Debilitated conditions through existing Boolean
queries, preserves the single MORE-bucket rounding point, and proves the finite
selected conversion/type/suppression domain from complete source and native
writer censuses. Eight ordinary tests, strict Clippy and all-five publication
pass, including fresh/reused/Rayon equality, duplicate/unknown conditions,
zero/inactive supplier refusals and a fractional-quality rounding control.
Source-call snapshots remain test evidence rather than runtime operands.

The subsequent [Offering Speed and ordinary-rate packet](../data/owned/poe2/3887ae68/offering-speed-rate/README.md)
extends the existing Offering application with an independently selected Speed
effect and calculates the exact Basic Attack rate through `OrdinaryTiming`.
Original05 yields **0.8666666666666667 attacks/s**; disabled Offering yields
0.6666666666666666. The actual 40-row level table, source/recipient scaling and
separate Damage/Speed Maximum winners remain data driven. Complete source and
native writer checks establish the finite ordinary timing domain; all broader
owner, input and query coverage boundaries remain Partial.

The subsequent [Player Evasion component](../data/owned/poe2/3887ae68/player-evasion-final/README.md)
publishes intrinsic BASE 7 and a guarded final receiver. Its finite native graph
uses the actual selected 30% and 15% quest rewards and reproduces Evasion 10.
The final owner remains Partial: native item-property, conditional and
transformation coverage must be completed before the unchanged original can
evaluate. Nine ordinary tests, strict target Clippy and all-five publication/
byte-identical rebuild pass; no complete metric or build is claimed.

The subsequent [item Evasion property](../data/owned/poe2/3887ae68/item-evasion-property/README.md)
publishes actual local arithmetic and the final EquipmentUse property through
four definitions, nine programs, eleven checked queries and two receivers.
Seven exact Modifier deliveries preserve hybrid/Defences contributions and
the prior Armour/ES streams. The retained positive source control reproduces
75; injected base/quality controls, signed rounding bounds, missing/unknown/
late suppliers and demanded unavailability for replacements also pass.
Original template/Modifier coverage stays Partial: numeric family identity
alone cannot establish local applicability.

Item-property predecessor: **`runs/owned-item-evasion-property-publication-02/package`**.

- Input: `d70eb4fefe8e4cefdddf123f545424b83f9247d41e7ac7c8a00429938363cfbf`.
- Registry: `608b251388dca735a38b52ad0b11b7e30ade326c7a7f1c8893c76c4b1e173bb3` (last `33a8`).
- Definitions: `9e2782eeda887808140128f4795040cc5e0892857ffe19000a73de4ef12db48f`.
- Rules: `081b16a6c286e7984352e1afcb65016931695484a40105460fcf5d972ac55531`.
- Compiled: `a067e0767fd0d97d45288b9fca7b7227e157115aad6f50e5dbae40668bc64c56`.

The eighteen files total **64,559,809 bytes**, with 199 provenance rows,
8201 effective programs, 89 checked queries and 59 receivers. Nine ordinary
tests, strict target Clippy and all-five publication/byte-identical rebuild
pass. Publication's exact inverse authenticates 2817 predecessor potential
writers and preserves every original input, query and local identity; selected
issue counts stay **106/117/109/123/4**, with no complete build or evaluation
bundle. Logs: `runs/item-evasion-property-{ordinary-13,clippy-02,publication-02}.log`.
The first publication rejected unsorted receiver-template targets; the repair
uses the existing loader's canonical ordering without changing game semantics.
The following Player slot aggregation is now published; final conditional and
transformation coverage remains incomplete.

Slot predecessor: **`runs/owned-player-evasion-equipment-publication-02/package`**.

- Input: `cb52e5b9de49d5e8aea9e7d8bb603a92e839ee6e5c5635e83f32364c4869cc78`.
- Registry: `5773595cb2c607d0a2bdc419e67d2f6d19f3af529561741e951861e279be91f0` (last `33a9`).
- Definitions: `baa118a44dc6c47216cc5aec2866e7a82f918a72ca48f42b7a59c35e861e76b9`.
- Rules: `db8001412284bcfa9518040853ff80b2552ff73976d9d4000a2b6cec619d830d`.
- Compiled: `a94cdc7faf57507e83a143c8e69771cd36a291019b0a302fa99454924cf9e440`.

The [slot packet](../data/owned/poe2/3887ae68/player-evasion-equipment/README.md)
reads the real final item property in Helmet, Gloves, Boots, Body and offhand,
applies each slot's factors in order and supplies subtotal `33a9` to the final
receiver. Empty slots contribute zero; occupied unsupported items remain
unavailable. The source-only unconstructed Weapon3 term is omitted under the
proved finite arithmetic rules, without introducing a phantom slot.
The eighteen files total **64,569,789 bytes**, with 200 provenance rows,
8202 effective programs, 89 queries and 59 receivers. Original Evasion remains
10; positive joined controls exercise nonzero item values and slot factors.
The new target's 22 tests and predecessor target's ten tests pass (some helpers
are shared), as do strict Clippy and all-five publication/rebuild. Logs:
`runs/owned-player-evasion-equipment-{ordinary-06,clippy-02,publication-02}.log`.
All 110 queries, local identities and selected issues **106/117/109/123/4**
are preserved; final/shared owner coverage is still Partial.

Validated predecessor before Actor migration:
**`runs/owned-player-evasion-override-publication-01/package`**.

- Input: `b3f2bcb73c780c4687fc868dd84a62db782ebe5bb39f0b1711a25e4e0ec5dfff`.
- Registry/definitions: unchanged from the slot predecessor above.
- Rules: `94cbb562efadb840766ca3ca21b7702e34ba5a78833a51046e796a1fd75a18cb`.
- Compiled: `88c6da669e5bc49734b5687165fe9e1bb480177c8dffbf4f2657c799d32e083d`.

The [override packet](../data/owned/poe2/3887ae68/player-evasion-override/README.md)
adds one checked Override query and no definitions/programs. An agreeing selected
value, including zero, replaces only the global base before scaling; item subtotal
and final total retain their existing meaning. Conflicting, unknown and unlisted
suppliers fail explicitly. Actual potential-writer census proves the finite native
empty membership; it does not complete source coverage or the final owner.
Twenty-six ordinary tests (including inherited components), strict Clippy and
all-five publication/byte-identical rebuild pass. Logs:
`runs/owned-player-evasion-override-{ordinary-03,clippy-01,publication-01}.log`.
The 201 provenance entries, 90 queries and 8202 effective programs preserve all
110 saved queries, exact local IDs and selected issues **106/117/109/123/4**.
No input or owner gap is retired. The following Actor migration preserves this
checkpoint's mechanics and coverage with the new-format identities below.

Validated current package after Actor migration:
**`runs/owned-generic-actor-publication-01/package`**.

- Input: `1b2603af5c5a6a698159c717e398a98e478316c9b3069f5310d9ccf35010d511`.
- Registry: unchanged (`5773595c…`, last `33a9`).
- Definitions: `503db6e507609da86faa880164bf6c4dd35ba34a93819dc2aebd6ea9438bb01b`.
- Rules: `b8feb4e6679487da171b5d29016dc104f99caa1c589914264c9eb3bb157bd376`.
- Compiled: `9bd71828333762be26224995600ab7f2a00520ee4078929dc1bb364aeca51509`.

The 18 files total **64,571,852 bytes**, with 202 provenance entries and the
same 90 queries, 59 receivers and 8202 effective programs. No new game rule or
complete evaluation bundle is introduced. Full-release inverse checks validate
91 declared format edits, two release names and 68 admitted dependency pointers
(63 changed); the portable replay validates 51 edits and 16 dependency pointers.
Original source observations and all other semantic fields restore exactly.
Evidence: `runs/generic-actor-format-{preflight,final}-inverse.json`,
`runs/generic-actor-fixtures-rebuild-01.log`,
`runs/generic-actor-originals-inverse-02.log` and the package's parent
`validation.json`. Bootstrap artifacts were regenerated through the existing
offline exporters and production publishers, with no historical-format loader.

The final Evasion receiver predecessor remains at
**`runs/owned-player-evasion-final-publication-03/package`**:

- Input: `e282b4bb142200944d84f30aaf35ccc77eb3fae9d6084be2c0229b9960489af9`.
- Registry: `5874684bd9ac6570c132fada76e8e459a5c11534c248de1dfd4df25d06da41b6` (last `33a4`).
- Definitions: `3dcde6d1c357b3e51bc6deb7e20be3a99393e569304dc887fc4e37071b9d0660`.
- Rules: `9605dbb30b2f471395fcff04efeb058b0da7145fd7975b7d81a33c4f320d812e`.
- Compiled: `911c727858c38537b55c6407643b9b823d5f89ebcf1d5cac87c011dbcdc17628`.

The eighteen files total **64,520,414 bytes**, with 198 provenance rows,
**8192** effective programs, 78 checked queries and 57 receivers. The serialized
endpoint is separately 64,508,858 bytes. All 110 queries, local IDs and selected
issue counts remain unchanged. Production Partial-owner refusal, exact saved
metric binding in the finite graph, reward removal, unsupported numeric-domain
diagnostics, late writes and fresh/reused/Rayon equality are checked. Logs:
`runs/owned-player-evasion-final-{ordinary-04,clippy-01,publication-03}.log`.
Earlier fixture and publication failures remain recorded; their repairs supply
the actual Reward discovery declarations, append the new owner explicitly and
match the normalized receiver by identity, retaining the exact full inverse.

The Speed predecessor remains at
**`runs/owned-offering-speed-rate-publication-01/package`**:

- Input: `b6c757ffc109b2a58bc21b0d185033bfe1553c37419cf46402116dfed09e7a0e`.
- Registry: `f2756a579a7f5d478dc626ab5d62cab5cb597e9cc3c026ceb4a49601c46ff9e2` (last `33a3`).
- Definitions: `ab74ade74def338d6b84e2d9e2c0a17ab680e5da0ddff143761dbb3a028dc099`.
- Rules: `4258bbe67962385b121f03435a08ff93a2414626e4255809308e3c5b6b1b2746`.
- Compiled: `e0148d25324ff034b3fb9f63b3cb67346484d0a63dbcf1fd5f850898edac2f6f`.

Its eighteen files total **64,487,868 bytes**, with 197 provenance rows and
**8190** effective programs (8189 owner programs plus one application). The
publication used the former 8192 ceiling; the subsequent program-only capacity
checkpoint below raises that ceiling to 16384 without changing this package.
The Speed publication adds two definitions, one program and one query, appending
one independently selected effect to the existing application. Fifteen ordinary
tests, all-five publication/rebuild (32.83s) and strict Clippy pass. Native tests
cover missing/unknown producers, separate stacking winners, late writes, actual
tick limits, level-domain refusals and fresh/reused/Rayon equality. All 110 saved
queries, local IDs and selected issue counts remain unchanged. Evidence:
`runs/owned-offering-speed-rate-{final-01,publication-01,clippy-01}.log` and
`runs/owned-offering-speed-rate-publication-01/{validation,receipt}.json`.
The preceding physical-hit package remains reproducible at
`runs/owned-selected-physical-hit-publication-01/package` (input `d935e899…`).
Its eighteen files total 64,474,322 bytes; its serialized endpoint is separately
64,462,764 bytes. These are different measurements.
Trade preferences add no definitions, native programs, slots or tables.
Twenty-one focused Import tests, eight ordinary CLI tests, all-five CLI
publication/rebuild and strict Clippy pass. Evidence:
`runs/owned-trade-preferences-import-{01,clippy-01}.log`,
`runs/owned-trade-and-ci-cli-{01,clippy-01}.log`, and
`runs/owned-trade-preferences-publication-01.log`.
The earlier [Current Energy Shield input](implementation-history.md#archived-2026-10-15-checkpoint-current-energy-shield-input)
and [source boundaries](implementation-history.md#archived-2026-10-15-checkpoint-final-selected-flat-damage-boundaries)
remain preserved with their evidence.

**Next input blockers:** the remaining saved controls have distinct responsibilities:

| Control | Next bounded step |
| --- | --- |
| Campaign bosses defeated | [Implemented and published locally October 19](owned-progression-input-proposal.md). Numeric projection, exact accounting, native unique-authority assessment and all-five correspondence pass. Consumers and future search must enforce the resulting assessment and required-fact availability. No constructor default: neither saved Placeholder 56 nor zero is an absent-value fallback. |
| Sigil and Corrupting Cry stages | [Scenario-owned usage intent accepted October 19](owned-scenario-usage-intent-proposal.md). Native resolution, configuration accounting and actual raw-stage publication pass focused/all-five checks. Two typed intents retain nonempty Partial applicability over five exact supplier relations. Next: complete grant/child discovery and actual mechanics; positive relation data is not execution authority. Exact Scenario overrides follow resolved intent, which overrides preset preferences. Unknown discovery remains blocking. |

The Skill-targeted external-assumption type is not an executable shortcut:
Engine external reads reject Skill targets. Skill-context reads of Player inputs
do not establish exact skill ownership. Reuse checked usage composition
instead of routing skill state through a global Player input. The remaining
outside-Config origins need their own dispositions. Do not infer native
ownership solely from a PoB modifier-list destination.
Warcry/Stunned activation and upper clamps, Ammo/Grenade applicability, time
windows and inventory limits remain downstream work. The source's preview mode
is not a gameplay flag. Complete configuration, usage and assumption inventories
remain mandatory; a smaller source-link count cannot close them.

The current stage-intent endpoint retains thirteen configuration-origin rows,
audited against its unchanged import and source XML. The preceding fifteen-row
census additionally contained ordinals 437 and 442; their raw stage intent is
now accounted for, without claiming complete applicability or mechanics:

| Source ordinals | Remaining responsibility |
| --- | --- |
| 1 | Build level/class/ascendancy, main skill selection and format/view fields |
| 123, 124 | TimelessData variants/search preferences and active Tree selection |
| 166 | Skills active preset, constructor level/quality and browser fields |
| 174, 175 | Archived item-granted Firebolt group/Gem correspondence and deferred usage |
| 422, 423 | Config containers; campaign ordinal 450 and stage-intent ordinals 437/442 are now accounted for |
| 467 | Items active preset, weapon-loadout and display fields |
| 744–746 | Calcs context and its distinct skill/mode selection |
| 797 | Import export-Party behavior and account/league UI fields |

The archived Firebolt pair can eventually reuse existing exact generated-input
and deferred-usage contracts, but retiring those two links would leave all five
selected obligations unchanged. It is not the next numerical milestone. Calcs
selects a distinct reference context and export-Party has a calculation branch;
neither may be dismissed as presentation solely by its UI location. Finish the
selected numerical consumers while preparing a complete remaining configuration
and default-accounting proof with the separate usage/progression decisions.

The apparent Chill breakdown use of Shock stack names remains an investigation
with ailment-effect admission, not an accepted parity exception. The bounded
source review's lack of a direct Scorch-stack reader is not an absence proof.
See the [cleanup register](legacy-retirement.md#ailment-calculation-and-display-separation-2026-10-15).

## Previous checkpoint: Critical Weakness input

[Archived publication and validation](implementation-history.md#archived-2026-10-15-checkpoint-critical-weakness-input):
the raw Enemy count uses checked source selection and constructor 20. All-five
preservation, 22 controls, strict Clippy and immutable source receipt checks pass.
The subsequent ailment packet preserves this input and its downstream gaps.

## Damage checkpoint: critical calculation and ordinary armour

The full outgoing-critical component now executes and publishes on the generic
Actor endpoint. It binds actual Encounter `31d1`, preserves zero/inactive potential
membership, uses definitions `33aa`–`33b0` and reuses intrinsic `3215` and accuracy
`321c`. The 14 source tests, three publication controls and nine full native
controls pass. The all-five publication passes independently, with 110 queries
and issue counts **106/117/109/123/4** unchanged. Strict targeted Clippy passes.
No query or existing owner gap was closed. Logs and exact identities are in the
[component packet](../data/owned/poe2/3887ae68/outgoing-critical/README.md#native-execution-and-publication-october-16).
Outgoing-critical checkpoint: `runs/owned-outgoing-critical-component-publication-02/package`,
input `051c663ef14260a0fd10b7cfedb6f253cb1dfede2efd017831f25b7237904e64`,
last allocation `33b0`, 8204 effective programs.

Configured Weakness activation is now published at
`runs/owned-critical-weakness-activation-publication-01/package`, input
`a7e1cb77b85ecd34ff18ea7b99cf104b41079cd9328715e00553d2785c1f9e3d`.
It imports Boolean `33b1` using the checked false constructor and runs a separate
Encounter FLAG producer before the condition freeze. Seven source/policy and four
new native controls pass, along with the configuration regression batch and strict
Clippy. The explicit all-five publication passes with twelve mutation controls,
identical rebuilds and no changed saved source leaves. An absent checkbox proves
only the configured source is false; it does not close skill-applied conditions
or infer that an enabled Command skill has hit. The Weakness query stays Partial.

The latest validated endpoint is
`runs/owned-enemy-armour-component-publication-01/package`, input
`5b611f43a0eedbf556799bcbd2c295a85d58d3264a7306c24602fd4d41e6e7b6`.
It adds two Stats `33b2/33b3`, one program and four Partial queries for normal and
critical armour-adjusted averages. These use the actual Encounter Armour producer
`31e0`, physical endpoints `33a0/33a1` and critical multiplier `33ab`. Units and
each pass's nonlinear reduction are preserved; no captured Armour or damage value
becomes an operand. Six numerical controls and three publication controls pass.
The actual all-five publication passes in 33.29 seconds, preserving every unrelated
field, local ID, all 110 queries and issue counts **106/117/109/123/4**. Its rebuild
is byte-identical; no owner/query closure or evaluation bundle is added.
Logs: `runs/owned-enemy-armour-component-tests-02.log` and
`runs/owned-enemy-armour-component-publication-01.log`. An earlier raw JSON table
comparison rejected `3` versus `3.0`; full typed table equality corrected that
test representation mismatch without changing source data or numerical rules.

After those real producers, finish per-definition/occurrence source accounting
for admission. Exact native query membership and complete gameplay coverage are
separate: even a completely enumerated query cannot bypass Partial selected
owners, applications or input inventories. The Diamond condition/override factory,
untranslated Sand Djinn on-hit responsibility, positive supplier order and special
branches remain explicit. Do not introduce another public model to replace these
concrete source obligations.

The next bounded damage consumer can reuse normal/critical averages `33b2/33b3`,
critical chance `33aa`, after-block hit chance `3220` (not accuracy `321c`) and
ordinary rate `33a3`. Pinned `CalcOffence.lua:4563–4576` weights the two mitigated
passes, applies hit chance, then effective rate, DPS multiplier and mechanical
quantity. Preserve source arithmetic order. Before treating the last factors as
neutral, extend the actual source-domain proof for hit-time multiplier/override,
per-enemy rate caps, DPS INC/MORE, quantity and late repeat/hazard factories.
The existing rate packet deliberately does not prove those absences. Bind a
final admitted Stat to `selected_hit_dps` only after upstream mitigation and
nonphysical domains are also accounted for. No new public arithmetic API is needed.
The internal weighted hit must not populate saved `selected_average_hit`: source
attack aggregation copies AverageDamage/TotalDPS but leaves AverageHit absent.
Keep saved skill count and FullDPS reporting separate from mechanical quantity.

**October 19 source review:** Diamond's source initializer/helper chain need not
be recreated as native intermediate flags. When a real granting source is
implemented, its active buff can directly contribute the existing Actor critical
chance Override(100), preserving its actual recipient and numeric-selection rules.
The retained selected source domain has no activating supplier; that exclusion
must be attached to admitted definition/provider domains before owner coverage
can close. Do not invent a false configuration input or promote empty native
membership into gameplay absence. The foreign Raging Spirit/Phantasm grants and
Sand Djinn critical-bonus sources retain their distinct recipients.

Physical hits and ordinary attack rate are published for their finite selected
source domains. The rate proof authenticates both original JIT captures (6,981
targeted occurrences / 124 complete records each), complete carrier records,
36 exact Basic constructors, eleven full source bodies and the native writer
census. It establishes all eight timing inputs, including selfCast/ActionSpeed
and the real tick rate of `1 / 0.033`; captured empty flags and matching scalar
outputs alone establish none of these. Raw level 1 resolves to final level 3
in its retained control, so Offering supplies 21%, not level 1's 20%.

Next finish **outgoing critical source admission**, distinct from the
implemented finite consumer and incoming-Player critical calculations, then finish
**Enemy physical mitigation and weighted hit DPS** beyond the ordinary armour
component. The retained reference
normal/critical averages after armour are 414.2081976279727 and
1101.2392272188865; their weighted value is 448.55974910751837. The final saved
Sniper hit-DPS reference is 388.7517825598493. These are validation targets,
never operands or defaults. Multiplying mitigated normal damage by a single
average critical factor is invalid because armour is nonlinear. Keep the
saved unavailable Sniper average-hit query and FullDPS/count reporting separate.
The outgoing-critical preflight must retain the conditional Critical Weakness
supplier even while inactive, and resolve the source-call anomaly recorded in
[cleanup tracking](legacy-retirement.md#other-current-compatibility-investigations).
The isolated `outgoing-critical` draft reserves final Action chance
and multiplier outputs. Intrinsic chance is 3215; accuracy confirmation is 321c, distinct
from after-block hit chance 3220. Shared minion bonus/cap ownership, Critical
Weakness applicability and crit-specific dynamic producers remain unclosed.
Additional supplier definitions/programs must be justified before allocation;
no new runtime contract or source-argument compatibility behavior is approved.

The [critical packet's source evidence](../data/owned/poe2/3887ae68/outgoing-critical/README.md)
now retains ten complete source files, 21 spans and 39 exact recipient/output
comparisons. Eight ordinary offline checks pass, including callback append
inverses, exact descriptor/binding roles and an independently pinned baseline
append. The baseline comparison uses the existing typed RuleProgram after raw
source authentication: six historical `1` quantities serialize as `1.0` in the
current program, with no semantic difference. The initial strengthened check
correctly rejected raw-JSON inequality; its failed log is retained before the
typed repair in `runs/owned-outgoing-critical-source-02.log`. Strict target
Clippy also passes (`runs/owned-outgoing-critical-clippy-01.log`).
The subsequent selected ordinary-critical review joins fifteen complete
manifest-bound source bodies, 24 writer/destination spans, eight retained
constructor/delivery proofs, both original traces, forty selected recipes and
36 exact Basic constructors per trace. Conditional Diamond Shrine FLAG and
Critical Weakness BASE suppliers remain explicit. Foreign recipients, nested
payloads and false/zero candidates cannot become an empty native channel.

Six authored decimal-rounding fragments now execute with native
nearest-ties-positive semantics, checked on their actual scaled operands over
`[-2^52, 0.4999999999999999]` or `[0.5, 2^52]`, followed by division by 100.
The signed bound is distinct from final Evasion's zero-floor proof. Thirteen
boundary controls, lazy bypass and admitted/outside/missing/admitted scratch
reuse pass. The extended target has fourteen passing tests and strict Clippy:
`runs/owned-outgoing-critical-domain-{01,clippy-01}.log`. This executes the
authored rounding fragments. The subsequent full-consumer checkpoint above adds
nine native controls and publication; existing owners and all sixteen query
groups still remain Partial.

The source census also preserves an untranslated gameplay responsibility:
`SummonSandDjinnPlayer` (Kelari) has one on-hit Critical Weakness stack in the
pinned skill data, but the retained selected recipe has no modifier mapping
for that stat. Its Command-only participation and Enemy delivery need review;
neither the absent source modifier nor the observed critical chance establishes
native condition absence. No PoB defect exemption is approved. See the
[cleanup investigation](legacy-retirement.md#sand-djinn-critical-weakness-responsibility-2026-10-15).
The next critical gate is candidate-bound condition/special-branch admission.
The complete native consumer executes through the implemented generic
Actor/Encounter query contract. No additional public API decision is needed.

**Resolved structural blocker, October 16:** checked membership now admits the
exact selected Encounter owner, and the scenario Enemy uses ordinary Actor
queries with explicit role authority. The outgoing-critical packet is rebased
onto that endpoint. `31d1 / configured-critical-weakness / critical-weakness`
uses `ContributionOrigin::Encounter`, retaining zero and inactive membership.
Both full consumer programs execute with intrinsic chance `3215`, accuracy
confirmation `321c`, raw stacks `3390`, the authenticated baseline append and
IDs `33aa`–`33b0`.

Controls validate exact recipients, Weakness true/false/unknown and stack bounds,
critical override presence/conflicts, missing operands and Partial/unlisted/late
suppliers. Finite native results match the retained original 5% chance and 2×
multiplier with exact fresh/reused/Rayon equality. The override control exercises
numeric selection; it does not implement the Diamond factory. These controls do
not replace candidate-bound
condition coverage or resolve the Sand Djinn responsibility above. All sixteen
query groups remain Partial until their actual domains are admitted. The armour
component now reuses configured Armour BASE `31e0` through that same graph for
separate normal and critical mitigation. No filler definitions, alternate aggregator or compatibility
aliases are needed.

The [Enemy physical-mitigation packet](../data/owned/poe2/3887ae68/enemy-physical-mitigation/README.md)
now retains five original source bodies, nine spans and all 39 observed Sniper
pass comparisons. It establishes the next implementation's operation order:
reuse configured Armour `31e0`, preserve Rating versus Damage units, use the
monster cap of 75% rather than the Player cap of 90%, and mitigate normal and
critical averages separately before weighting. Conversion/gain and non-effective
reference contexts remain distinct; source output fields never become runtime
operands. Three ordinary Rust evidence tests and strict targeted Clippy pass; logs:
`runs/owned-enemy-physical-mitigation-{source-01,clippy-01}.log`.
The later native/publication checkpoint above implements the ordinary armour
consumer. Complete supplier proof, extra mitigation/damage-taken branches and
final DPS remain unfinished.

The [selected Enemy supplier review](../data/owned/poe2/3887ae68/enemy-physical-mitigation/source-domain-review.md)
confirms that shared Enemy damage-taken/reduction channels contain conditional
baseline rules. They are independent of Encounter configuration. The
[generic Actor decision](owned-existing-enemy-rule-ownership-proposal.md)
was accepted on October 15: unify the scenario opponent with Player and supplied
units under one Actor identity/local-rule context, retaining explicit role and
source authority. The earlier separate-Enemy-context draft is superseded.
The implemented contract includes selected Encounter origins and shared local
rules for all three roles. Shared baselines retain their own owner. Actual
Maimed and Enemy mitigation publication must now use that contract; the Actor
migration itself does not establish their supplier coverage or numerical rules.
Late Shock can also
produce a zero-valued conditional record: Lua numeric truthiness is source
evidence to classify, not native gameplay behavior to copy. This review narrows
the next checks but does not complete source admission or native membership.

**Current independent saved-metric work:** Player Evasion (query 19, Metric `313e`).
The actual positive sources are shared Player intrinsic BASE 7 and two selected
Reward increases of 30% and 15%; both retained reference contexts yield 10.
Reuse channel `29f1`, Rating `29ee` and shared Player owner `332a`, with final
Stat `33a4` now published. Its checked rounding/zero-floor formula preserves
native nearest-ties-positive semantics and rejects the source helper's
exceptional operand immediately below 0.5; the
[domain proof](legacy-retirement.md#evasion-rounding-domain-2026-10-15) explains
why the Life/Mana bound alone was insufficient. Observed 10 is only a comparison
target. This route avoids waiting for Enemy authority and does not require
finishing all positive Armour/ES item properties first.
The Evasion component deliberately keeps its final owner Partial: retained
source evidence proves a finite test domain, but is not runtime authority for a
candidate's item properties or conditions. Its controlled graph can exercise
the saved metric binding; ordinary complete planning must still refuse it.
Native per-equipment Evasion properties and their Player slot contributions are
published, including checked override selection. Conditional Evasion channels
and transfer coverage remain. The published work uses the existing shared
EquipmentUse local-defence pattern and the accepted shared-Player slot reads;
no new public API is needed. Retained source controls produce local Evasion 75
from positive hybrid inputs, providing a check beyond the original's four zero
items. Keep per-level/property-replacement absence explicit, distinguish an
empty slot from an occupied unsupported item, preserve per-slot multiplication
and addition order, and test nonzero injected base profiles and quality changes.
The reviewed route is `runs/owned-evasion-runtime-route-01.md`. Do not leave a
test-only completion as the final solution or publish observed zero item values
as implicit defaults.

The published item property uses four contiguous definitions (`33a5`–`33a8`)
for local Evasion, its final EquipmentUse property, per-level contributions and
property replacements. The published shared Player subtotal uses `33a9`.
Local modifier eligibility remains a real coverage obligation: a matching
numeric modifier family does not establish local rather than global delivery.
Shared Armour/Evasion, Evasion/ES and Defences contributions must be emitted
once, preserving existing Armour/ES consumers; the predecessor currently uses
direct reductions there. Replacement presence must leave an unsupported final
property unavailable, including a present zero. Do not encode that Boolean as
an invented Rating and run it through a numeric guard. Exact empty-group
membership, missing final-property propagation and the whole-plan coverage gate
remain distinct checks. The Player slot successor omits the source-only Weapon3
zero term under the native finite-number and signed-zero rules.

The associated [direct-reduction audit](legacy-retirement.md#direct-numeric-reductions-and-encounter-membership-2026-10-15)
found 78 older direct numeric reads in 32 current programs, including eleven
Enemy reads. These retain their existing validation but lack explicit checked
query membership. The Actor migration preserves those formulas; migrate the
touched Enemy block/incoming-hit consumers with their real source proofs next.
Do not introduce more direct reads or remove the operation before its other
live contexts are migrated.

The portable Sniper replay now contains the published physical-base program;
all **45 ordinary Engine tests** and **40 tests across six direct CLI consumers**
pass, with strict Clippy for every affected target. The old arithmetic installer
is removed. Life test helpers authenticate the already published guarded bodies
instead of reinstalling them; the Amulet test rejects unsupported inputs even
on identity/bypass paths while preserving lazy arithmetic, all nine source
vectors and recipient/order/zero/missing controls. Historical Partial coverage
survives only as an explicit negative control. Earlier failing logs remain
retained; no production rule or source observation changed for these repairs.

The fixture refresh also includes three previously published Life guards and
29 canonical descriptors; its exact inverse is not physical-only. Build,
Scenario and saved queries remain unchanged. Evidence:
`runs/owned-selected-physical-base-engine-replay-03.log`,
`runs/owned-selected-physical-base-replay-consumers-02.log`,
`runs/owned-selected-physical-base-amulet-replay-01.log`, and corresponding
`engine-replay-clippy-01`, `replay-consumers-clippy-01`, `amulet-clippy-01` logs.
See the [fixture receipt](../tests/fixtures/owned-sniper-replay.md) and
[retirement tracking](legacy-retirement.md#other-current-compatibility-investigations).

## Previous checkpoint: Skill support and exact aggregation

Exact Skill support recipients and the generic exact Integer reduction now run
in native Core/Data/Engine. They are validated prerequisites for the accepted
Skill-owned reservation migration; obligation effects and payer binding are
still unfinished. The reporting package below is the predecessor of the current
Critical Weakness and ailment input publications.

### Predecessor data: reporting inclusion intent

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

### Exact Skill support recipients

Core/Data/Engine now extend the existing receiving contract with a Skill endpoint
and exact `SkillTarget` application key. Semantic Skill targets admit an empty
path; Gem containers require an explicit grant to a generated Skill. Two final
grant aliases cannot duplicate one recipient, while repeated sources, siblings
and ancestor provider paths remain distinct. Cold binding retains the validated
receiver Actor only as private context. It grants no new Skill-to-Actor reads,
writes or payer authority.

Skill support programs retain their assignment's inputs and provenance, write
the exact Current receiver, and use the existing preparation/execution roles.
Complete receiving inventories must classify their Skill programs. Missing or
Partial inventories remain blocking; false execution participation does not
erase preparation supply, unknown participation is not false, and late writes
to frozen Skill channels still reject. The existing graph, scratch and Rayon
paths handle delivery without constructing an Action. No game-specific rule,
new compatibility branch or resource-obligation producer is added.

Validation passes **202 targeted tests**: 90 Core/Data, 66 Engine library and
46 Engine integration tests (readiness 18, participation 9, delivery 15 and
inherited delivery 4). Controls cover direct and generated recipients,
independent child admission, assignment inputs versus receiving inputs, exact
occurrence identity, missing producers, scratch reuse and parallel execution.
Strict targeted Core/Data and Engine Clippy passes; independent review is clean.
Evidence: `runs/owned-skill-receiving-data-03.log`,
`runs/owned-skill-receiving-data-clippy-01.log`,
`runs/owned-skill-receiving-engine-unit-02.log` and the final Engine integration
and Clippy logs under `runs/owned-skill-receiving-*`. Earlier fixture failures
remain recorded; fixes did not relax runtime authority or missing-input checks.

The all-five CLI publication regression passes (52.53s) at
`runs/owned-skill-receiving-publication-01`, independently comparing all 18
artifacts byte-for-byte with the reporting-inclusion checkpoint above. Endpoint
`e1c2abd299aead48ff1da38aa517c4fcbcaacd2671f754deb921364c0cad4b39`,
64,432,702 bytes, 187 provenance entries, 110 queries and selected issue counts
**106/117/109/123/4** are unchanged. See
`runs/owned-skill-receiving-publication-01.log` and
`runs/owned-skill-receiving-artifact-comparison-01.json`.

**Resume:** add the planned typed resource-obligation effect to the existing
Skill owner/program inventory, checked payer aggregation and named exact Action
dependencies, then migrate the current reservation data. The live reservation
consumer remains Action-owned. Skill receiving alone does not make it
query-independent or complete any of the five builds.

### Exact Integer aggregation

The generic `ExactIntegerSum` reduction is implemented in the current V27
contract. It admits checked Add queries over Integer Stats, unordered membership,
integer zero and no ranks. Worker execution accumulates every active occurrence
in checked `i128`, then checks the final public Integer range. The unchanged
65,536-effect cap proves the intermediate bound. Signed cancellation is exact;
local IDs and discovery order carry no numeric authority. Unknown contributors,
Partial membership and final overflow retain explicit failures. Raw reads cannot
bypass the checked query, and ordinary Sum/Product behavior is unchanged.

Validation passes **143 targeted tests**: Core/Data 33, Engine library 70 and
Engine integration 40. The six new integration cases reuse existing repeated
Reward-to-Player authority; they do not pretend that Skill-to-payer delivery is
implemented. Kernel controls cover the full hard effect limit, cancellation,
canonical failures and work budgets. End-to-end checks include distinct duplicate
sources, missing values, inactive occurrences, scratch reuse and Rayon. Strict
targeted Clippy passes; independent code and identity reviews are clean. Earlier
test-authoring errors remain in their failed logs; runtime validation was not
relaxed. Evidence: `runs/owned-exact-integer-sum-data-02.log`,
`runs/owned-exact-integer-sum-data-clippy-01.log`,
`runs/owned-exact-integer-sum-engine-lib-02.log`,
`runs/owned-exact-integer-sum-engine-regression-05.log`,
`runs/owned-exact-integer-sum-engine-actor-07.log` and
`runs/owned-exact-integer-sum-engine-clippy-01.log`.

The actual all-five CLI publication regression also passes (44.85s), rebuilding
all 18 artifacts byte-for-byte from the same inputs into
`runs/owned-exact-integer-sum-publication-01/package`. Input endpoint `e1c2abd299…`,
compiled identity `6401dc0a…`, 64,432,702 bytes, 187 provenance entries, 110 queries
and selected issue counts **106/117/109/123/4** remain unchanged. No game-data
regeneration or source acquisition is required. Evidence:
`runs/owned-exact-integer-sum-publication-01.log` and
`runs/owned-exact-integer-sum-artifact-comparison-01.json`. Complete native
originals remain **0/5**.

**Resume:** obligation amounts must stay separate from ordinary pool contributions.
The [aggregation refinement](owned-resource-obligation-aggregation-proposal.md)
records the proposed resource/kind/quantum signature and amount channel; the
owner question is pending. Do not implement those public fields until resolved.
The current Sniper intrinsic and ordinary flat programs use no Action-specific
read, so they can migrate after the obligation/payer contract and before the later
named Action-dependency extension. Unsupported Action-dependent branches must
stay explicit throughout that cutover. This generic reducer adds no obligation
effect, payer authority, reservation data or complete-build coverage.

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
where mechanically required, and checked Skill support recipients. The receiving
extension is implemented and validated. Complete the obligation/payer contract
and migrate existing reservation data, removing the superseded live
Action-owned path. Prove invariance to unrelated metric-query changes alongside
participation, repeated sources, counts, payer identity, dependency cycles and
Action-sensitive controls. Existing numerical and complete-plan gaps remain
until independently closed. The selected physical-base publication now supplies
the previously missing checked self-flat groups and uses fresh identities.
Conditional/type/conversion damage and final DPS remain downstream.

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
current slot Evasion successor uses **8202/16384** after the bounded capacity
change below, with 59 receivers. Its serialized inventories contain 6549 owners,
23532 effects, 4309 reads, 33051 nodes and 89 checked queries. The measurement is
bound to input `cb52e5b9…` in `runs/owned-player-evasion-equipment-capacity-01.json`; these
are serialized inventory counts, not private compiler edge/work counters.
The current serialized endpoint is **64,558,225/67,108,864 bytes**,
leaving 2,550,639 bytes. Its eighteen package files separately total 64,569,789
bytes; earlier checkpoints reported file totals. These are separate measurements. The common
content digest also enforces the 64 MiB envelope, so increasing only release
input limits would not address package growth.

The five Mana adjustment receivers (`3357`–`335b`) have now been removed after
checking all rule/application readers, saved metrics and retained query contracts.
Their existing guarded Complete-empty membership is unchanged; broader supplier
and whole-plan coverage remains open. Do not split catalogs by the selected build or remove
legitimate per-item programs to meet a budget.

The next Evasion and outgoing-critical consumers would exceed 8192 programs.
The compiler program ceiling is now **16384**, with an explicit independent
checked-query ceiling still at **8192** and lower caller limits still honored.
Owners, nodes, reads, edges, effects, work, wire size and all plan limits remain
unchanged. The pre-change test demonstrates refusal of 8193 otherwise valid
programs. New tests admit exactly 16384, refuse 16385 and invalid custom limits,
retain exact source/execution identities and numerical results at smaller
budgets, and independently test actual Data-valid 8192/8193-query boundaries.
All 34 `owned_rules` tests and strict Clippy pass in
`runs/compiler-program-capacity-{after-03,clippy-01}.log`. The initial fixture
version/error-message failures remain recorded; no gameplay semantics or
coverage requirement was relaxed to make those checks pass.

The public CLI subsequently rebuilt the current real release under the new
budget into `runs/owned-program-capacity-release-01/package`. All eighteen files
are byte-identical to the published Speed package; the full receipt and compiled
identity `e0148d25…` are unchanged. The comparison is recorded in that run's
`validation.json`. The 15 ordinary Speed and three mitigation-evidence checks
also pass against the new compiler; critical draft evidence remains a separate
validation target and cannot establish native critical support.

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

## CI usage-resolution repair (2026-10-20)

Hosted run `39451021127` on `15778b04` exposed three stale Engine expectations
in `owned_frost_bomb_inputs`, `owned_skill_usage_inputs` and
`owned_sniper_reservation`. Core deliberately retains Build preferences and exact
Scenario selections separately; the definition-aware Engine preparation resolves
their precedence. The tests now inspect both authored layers, the complete
prepared resolution and replaced-source provenance, then evaluate that same
plan. Existing numerical and occurrence-isolation assertions remain intact.
No runtime behavior or compatibility path changes. All eighteen ordinary tests
pass across those three targets; two existing optional source-evidence tests
remain intentionally ignored. Strict Clippy passes for all three targets.
Logs: `runs/owned-usage-resolution-ci-tests-01.log` and
`runs/owned-usage-resolution-ci-clippy-01.log`.
The three test repairs were pushed to `main` as `f6dac721` and the remote hash
was verified. Hosted verification remains separate.

At 16:47 UTC the old run had two passing jobs, four known failed jobs and twelve
running jobs. The other failures are the transition receipt and unknown-variant
Import expectation already fixed in `afc607d8`. The `dba0f661` run
`39458332844` remained queued behind the older run. Running durations were within
previous successful ranges; public metadata did not expose live target progress.
No hosted success is claimed and no workflow/run was changed.
The 17:59 UTC refresh shows five passing jobs, the same six known failures
across both platforms, and seven running jobs. Ubuntu CLI shards 0, 1 and 2
have passed; the remaining running jobs are Ubuntu CLI3, four Windows CLI
shards and both PoB jobs. This snapshot describes the older `15778b04` run,
not validation of the newer fixes.
At 18:08 UTC, the same older run still had five passes, six known failures and
seven running jobs. The latest pushed `983cee19` run `39472005429` was pending;
the superseded pending `1ba8c7ce` run was cancelled by the existing workflow.
No new failure or hosted success was inferred from that queue state.
The later public review of `a6b61b3c` finds its run `39477231180` pending with
no jobs started. The older `15778b04` run has six passes, six known failures
and six running jobs; Ubuntu PoB completed successfully at 18:41:17 UTC.
No new failure class appeared. Evidence is retained at
`runs/ci-a6b61b3-public-review-20261020/`; none of these observations establishes
hosted success for the latest code.
At 19:07 UTC the old run has seven passes, six known failures and five running
Windows jobs. Ubuntu CLI3 completed at 18:50:24 UTC, so all four Ubuntu CLI
shards and Ubuntu PoB have passed. The latest run remains pending; no new
failure appeared. Snapshot files are retained in the same evidence directory.
At 19:17 UTC, `4aa10b6e` has run `39481892743` pending with no jobs started.
The old run has eight passes, six unchanged known failures and four running
Windows jobs; Windows CLI2 completed at 19:14:30 UTC. Evidence is retained at
`runs/ci-4aa10b6e-public-review-20261020/`. This remains queue/progress evidence,
not a passing hosted run for the latest code.
At 20:20 UTC, the latest backup `550d7bf4` has run `39490890433` pending with no
jobs; the preceding pending `00d3c62a` run was cancelled by normal concurrency.
The older `15778b04` run has ten passes, six known failures and two Windows CLI
jobs still running. No new failure class or hosted rerun of the fixes is proven.
The credential-free public snapshot is
`runs/ci-550d7bf4-public-review-20261020-201952/`.

The 21:41 UTC review supersedes the older queue snapshots above. Run `39451021127`
on `15778b04` finished: twelve jobs passed, six known test jobs failed and two
aggregate gates failed. Run `39498631482` on `c84e66f3` exposed the already-fixed
movement test macro error plus a new failure in both validation jobs at
`python scripts/tests/test_export_owned_allocations.py`. The exact failing assertion
is still unavailable: the public log API requires authentication, `gh` is absent,
and the browser tool could not initialize. No credentials were accessed and no
protected allocation files were edited. The latest `f2680094` run was pending at
that snapshot; no current hosted-green claim follows. Evidence is in
`runs/ci-public-review-20261020-214103/`.

The subsequent local allocation reproduction authenticated 34 inputs and ran the
existing offline exporter twice. Both runs produced identical rules and exactly
one changed provenance leaf: `bundle_transition_sha256` now matches the current
bundle (`a84e9c57648d6d8108d3291120d0b84e649632d4ae597e63a14aaae818233a7e`).
Complete JSON and byte inverses prove no other source-facts change. The regenerated
`data/owned/poe2/3887ae68/allocations/source-facts.json` is applied and the existing
exporter's `--check-dir` passes. No protected Engine source/tests were edited or
executed. This fixes a reproduced byte-validation defect; the hosted assertion
and hosted green status remain unverified. Evidence:
`runs/owned-allocation-provenance-reproduction-20261020-01/comparison.json` and
`check-dir.log` in that directory.

## CI receipt repair (2026-10-15)

Inspection of main `61205e81`, run `38825000935`, exposed a remaining Import
`items_after_test_module` lint in both validation jobs. The preceding `1ec92252`
run `38722502273` also exposed a Linux-only Demonflame evidence lookup failure.
The test module now follows production functions without changing either body.
The shared offline source checker resolves repository-relative archive components
portably, retaining the exact evidence packet and compressed/decompressed hashes.
No runtime rule, native path or current package identity changes.

Focused validation passes: the exact Import receipt test, strict Import
Clippy, five ordinary Demonflame controls, four Current Energy Shield controls
and strict CLI Clippy. The archive-locator regression reads the same sealed
1,172,527-byte source through either slash convention and rejects unsafe relative
components. Logs: `runs/ci-configuration-receipt-order-{01,clippy-01}.log`
and `runs/owned-trade-and-ci-cli-{01,clippy-01}.log`. Hosted validation of
the repair is pending: commit `885b5d7d` is pushed, and run `38829485175`
was queued behind the still-running predecessor at inspection. Full previous
job logs are retained in
`runs/ci-job-116730283{203,414}.log` and
`runs/ci-job-116410446{698,998,809}.log`. The current hosted run was still active
at inspection, with failed validation; do not claim hosted success.
The [earlier receipt and source-evidence repairs](implementation-history.md#archived-2026-10-15-checkpoint-earlier-ci-receipt-repairs)
remain recorded separately. They are not unresolved failures of this checkpoint.
The subsequent read-only follow-up found no additional failure: completed Import,
workspace and both Engine jobs pass. Ubuntu CLI shards 0, 1 and 2, Ubuntu PoB
and Windows CLI shard 1 also pass; the remaining jobs are still active.
Public job status and annotations are retained under
`runs/ci-followup-20261015-38825000935/`. No runs were cancelled by this task.
The scheduler coalesced pending revisions `885b5d7d`, `247fe562`, `6e1d6eb0`
and `07275d22`. At the latest read-only inspection the Player Evasion revision
`dd524416` has run `38839048945` pending behind the same still-active older run
`38825000935`. The repair and newer checkpoints still await their own hosted
validation; a queued revision is not a green hosted check.
The later completed Ubuntu CLI shard 3 (`116730283816`) fails only the same
Demonflame archive lookup, in both feature profiles, on old `61205e81`.
Authenticated job logs are retained at `runs/ci-job-116730283816.log`; the
failing path and line match the portable-locator fix already in `885b5d7d`.
Windows PoB has also completed successfully. The other active Windows CLI
shards and the queued current revision remain unverified; no new runtime fix
or repeated broad local test run is justified by this old-head failure.
The same log measures 56m04s and 55m04s compilation for its two feature
profiles. The [cleanup register](legacy-retirement.md#other-current-compatibility-investigations)
tracks profiling/cache/shared-helper investigation; preserving both profiles
and all checks is mandatory. This is a maintenance follow-up, not a reason to
interrupt the closest-build implementation.

Read-only follow-up at 2026-10-16 04:03:59 UTC found `accd5040` run
`38843640195` pending with no jobs. Older `61205e81` run `38825000935`
had run for 4h23m, with two Windows CLI shards still active. Its three failures
are the two lint jobs and Ubuntu Demonflame failure already fixed in `885b5d7d`;
13 jobs had passed. No new actionable failure or hosted success is established.
Status/annotation receipts are `runs/generic-actor-ci-{run,jobs,annotations}-*.json`.
No workflow or run was changed by this inspection.

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
The Sniper damage route has pre-conversion base, physical-hit, ordinary-rate,
finite critical, Armour and ordinary physical-DPS consumers. All-type final
binding and complete candidate-bound supplier admission remain downstream.
Complete coverage only where potential
supplier inventories and numerical behavior are proved; source observations
and finite component replays do not replace either requirement.

The selected saved Life query is Player. Its remaining supplier, conversion-order
and whole-plan dependencies are independent of the now-joined six
minion-Life sources and inherent minion zero contribution. Those joins advance
Actor coverage; they do not close the saved Player metric. Do not replace a
missing producer with a supplied scalar or call a finite composition a complete
build.

The current input audit found no existing-authority shortcut to close the
five obligations. Preset usage `0507` retains unresolved global switches,
action selection and FullDPS responsibilities despite known preferences.
`01f2` retains the configuration origins counted at the current checkpoint;
known values and remaining Firebolt correspondence are separate. `0208`'s
known assumptions do not prove the constructor/default catalogue complete;
`0209`'s empty saved usage does not account for seven
default child-effect enables. Scenario intent also requires complete applicability
accounting; it is a separate fifth obligation.
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
  the current `0507` disposition review. All these Djinn rows already have usage-only
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
2. Account for selected preset usage (`0507`) first, alongside configuration,
   external assumptions, exact Scenario usage and Scenario intent through reviewed
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

The earlier audit found no supported shortcut from known members to complete
inventories. Subsequent publications have transported more source fields; use
the counts at the current checkpoint, not the older audit's counts. Twenty
selected usage rows in ten groups still need complete field accounting, and the
28 non-reward effective defaults include seven child buff/curse/aura enables.
Scenario usage remains empty/Pending. Preserve these distinctions when adding
a local scalar recipe.

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

**Input resume:** use the current checkpoint's five selected obligations and
configuration-origin census. Its 101 item-range records already have exact
item/output or unresolved-item ownership. Later field publications narrow the
remaining Config and outside-Config origins without completing their inventories.
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
input `326e`: their source fields and consumers are distinct. Checked support
receivers now include exact Skills as well as Actors and Actions; `ReceivingSkill`
still names a separate admission context. The accepted Skill-owned obligation
migration must use that exact recipient. Do not synthesize primary Summon Actions,
invoke a parent obligation once per child Action, or repurpose source-property
preparation as execution authority. Payer aggregation and amount-channel identity
remain the next contract step described at the current checkpoint.
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
   currently has five selected obligations and 36 selected preferences. The earlier
   generated-usage checkpoint had 35 preferences: four skeletal
   counts, Offering and Frost Bomb switches, five Direct/Tree/Item occurrence
   counts, twelve requested-participation preferences and twelve reporting
   inclusion preferences. The subsequent Ice Nova count publication adds the
   thirty-sixth preference. Remaining global usage, Action/reporting execution
   and full field accounting need the
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
   identified the required Skill support-recipient contract. The owner accepted
   it on October 15; implementation must not manufacture Action demand from
   metric queries. Validate contrasting reservation/cost shapes,
   migrate the existing live consumer, and prove query-independent results.
   The exact Skill support endpoint is now implemented and validated in Core/Data/
   Engine, retaining preparation, participation and provider checks. Next add
   the checked obligation census, payer binding and aggregation, then migrate
   the current Sniper data. Its ordinary flat branch needs no Action dependency;
   add named exact Action bindings before admitting Action-sensitive branches.
   Reuse canonical
   Actor pool Stat identities and the existing occurrence/value graph; no new
   resource catalogue, second evaluator or compatibility path is required.
   Represent obligations as typed effects in the existing complete/Partial
   owner-program inventory, not a second top-level census. Permit emission only
   once per exact Skill owner invocation and qualify the obligation-to-payer
   contribution origin separately; ordinary Skill self-contributions retain
   their current permissions. Exact Action dependencies remain explicit reads.
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
   Sniper reservation must migrate from its current Action-owned programs to
   the accepted Skill-owned obligation model. Its ordinary flat branch reads no
   Action-specific property, so no parent Action or hidden query is needed for
   that migration. Later part/mode/stat-set-dependent branches require explicit
   named dependency binding; metric demand cannot choose or activate them.
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

Enemy distance is now published as explicit per-consumer projections: its
count-style zero fallback differs from the raw override lane. More numeric
controls alone cannot close the current
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
| Closest complete build and native optimization | Original05 is the current first-completion target. Close its five remaining input obligations, publish its real evaluation bundle and complete its selected mechanics/metrics through the public CLI. D4 begins after this first complete build, without waiting for all five: bind objectives to owned actor/action/stat-set requests and verify one legal locked candidate mutation before expanding search. Reassess the shortest complete path at every checkpoint. |
| Saved allocations and effective connectivity | The retained Sniper removal95 control keeps8737 in the saved tree, while PoB prunes8737 from its active set. Current Import preserves that occurrence with Pending allocation access; it does not silently repair the build. Preserve this refusal and test explicit candidate repairs separately. Before admitting passive-tree optimizer mutations, validate connectivity against injected roots, pools, scopes and exceptional access after each mutation; import-time path proof and legacy candidate checks are not a native post-mutation legality gate. |
| Independent breadth and CI | Add provenance-bound holdouts by mechanic intersections and an explicit reproducible integration subset. Existing five examples are development cases. Ordinary CI now executes the current accuracy rule laws, but the joined Sniper graph and its import/membership/staging checks are ignored without their published package. Provision the exact current package and retained source controls reproducibly in CI, run that joined subset explicitly, and fail on missing artifacts; do not treat component tests as an equivalent gate. |
| Historical replay retirement | Reimport with the one current implementation and retain useful stored source evidence. Seventeen older ignored publication harnesses need consumer review: rebase useful assertions to current semantics or remove obsolete scaffolding, rather than add compatibility paths. Declaration integrity and source parity remain required. [Retirement/migration inventory](legacy-retirement.md#historical-publication-replay-migration-2026-10-05). |
| Action-routing format retirement | Inventory the remaining V1/V2 readers and publication/test consumers, migrate useful callers to one current format with explicit selector coverage, then remove obsolete branches. Do not convert unknown selector coverage to Complete-empty. Preserve source witnesses and all-five/routing regressions; this cleanup follows the selected flat-damage work. [Retirement scope](legacy-retirement.md#other-current-compatibility-investigations). |
| Generated usage | Versioned preferences, all-record data-aware proofs and explicit applicability diagnostics pass focused validation. Shared exact-provider proof now serves raw quality and requested-count import independently. Same-definition allocations in different presets remain distinct. Enabled/global/reporting dispositions and usage inventory closure remain open. [Contract and gates](owned-generated-skill-usage-proposal.md). |
| Requested participation | [Common gate](owned-skill-participation-proposal.md), unified typed Import and first Sniper packet are validated. All-five/source/native checks preserve count, effect controls and mechanical availability. Next close genuine parameter inventories for remaining skills before readiness publication; no PoB preview bypass or default is imported. Full-original coverage remains open. |
| Resource ownership and demand | [Accepted October 15](owned-resource-obligations-proposal.md): sustained reservation is independent of queries, with exact Skill/payer/resource identity and explicit Action dependencies where required. Skill support recipients and generic exact Integer aggregation are implemented and validated; [amount-channel signature](owned-resource-obligation-aggregation-proposal.md) awaits review. Obligation effects, payer binding and live reservation migration are still unimplemented. The current Sniper flat branch requires no Action dependency; later cost conversion, mines and stance/toggle branches need their own reviewed declarations. Gigantic's producer/status and isolated Life/Damage-factor packets are published; final reservation delivery remains open. |
| Generated raw inputs | Accepted exact preset bindings now have V6 permissions, V19 producers, source-bound quality joins and checked publication for the reviewed Djinn/Firebolt families. Existing provider levels survive. Archived cross-axis correspondence, other generated families and final mechanics remain open. [Contract and gates](owned-generated-skill-inputs-proposal.md). |
| Generated source-format breadth | Replace finite Item-name frames with proved general derivation over admitted layouts, and add evidence for normalized-only level correspondence. Census competing saved representations before scalar admission. Original04's Firebolt provider still has unresolved rune/layout/grant-line authority; raw quality cannot bypass it. [Bounded adapter follow-up](legacy-retirement.md#bounded-generated-source-import-coverage-2026-10-05). |
| Typed Boolean contributions | Accepted 2026-10-07: add typed Boolean contributions and unordered Any to the existing occurrence/query graph. Separate complete membership from numeric ordering, preserve duplicate source identity and unknown propagation, and migrate actual inherent-attribute flag producers/consumers first. This is also a prerequisite for later critical-hit flags; Enemy recipient authority must be explicit. Core/Data/Engine schema3/operations22 are implemented; real halving/doubling passives and five reducers are published. Their five bounded membership groups are now Complete; global query coverage, donor owners and unrelated passive mechanics remain Partial. No false default or numeric stand-in. [Accepted contract](owned-boolean-contributions-proposal.md). |
| Actor/reward contribution queries | Accepted October 8; Core/Data/Engine membership and canonical Life query data use operations V23. Eight exact writers populate seven groups; six zero/one-effect domains are bounded by semantic tie rejection. Bounded equipment ordering, combined Player BASE accumulation and the guarded final Life consumer are published; magnitude/routing, conversion arithmetic and global/owner coverage remain open. [Contract](owned-actor-reward-contribution-queries-proposal.md). |
| Encounter contributions and numeric Enemy queries | [Implemented within the generic Actor migration](owned-encounter-contribution-queries-proposal.md). Selected Encounter invocations use the existing graph; ScenarioEnemy is an exact Actor recipient with role authority. The conditional Critical Weakness producer, full critical consumer, configured Boolean activation and ordinary Armour component pass native and all-five publication checks. Complete numerical admission remains pending. No separate aggregator or Enemy-specific numeric path. |
| Generic Actor identity and shared local rules | [Accepted October 15 and implemented](owned-existing-enemy-rule-ownership-proposal.md): actual ScenarioEnemy Actor identity, unified local mechanical context, removed Player fallbacks and explicit role/source permissions. Shared applicability binds Player, ScenarioEnemy and discovered supplied Actor slots. Whole-inventory validation and current-format artifact rebuild pass; no compatibility alias. Player final receivers stay unchanged, common baselines remain separate from Encounter ownership, and Partial coverage stays blocking. |
| Composed support discovery | Accepted October 8. The common native gate, current authored-order DTO and source-input accounting are implemented; Original05 `01de` is retired. The Gem packet publishes 937 Known and 29 Unmapped domains with all-five preservation; all 23 selected physical Gem domains are covered. Existing typed identities fixed 111 false stat-set blockers. Pinned generated export metadata classifies eight missing runtime effects as active without implementing their Command mechanics. Continue other owner domains and exact provider exclusions; keep Command action/usage gaps separate. Unsupported positive and archived source relationships remain blocking. [Contract](owned-support-origin-composition-proposal.md). |
| Exact Skill contribution queries | Accepted October 8; Core/Data/Engine V24 implements Current Skill reads and Skill-owned self-contributions with explicit direct-use and/or supplied-slot permission. Exact supply relations survive late support checks. Canonical Offering source writers use four checked empty domains and their activation/application join passes. Inherited Actor and support-delivery memberships remain pending; nonempty game-group meaning requires proof. Actor recipient writers also use guarded empty queries. [Accepted contract](owned-skill-contribution-queries-proposal.md). |
| Application-group contribution queries | Accepted and implemented October 8 in Core/Data/Engine V25. Typed program-effect/application-group addresses preserve exact recipients, potential-source census, stacking, unknowns and explicit numeric order in one checked reduction path. Current artifacts are rebuilt with no old-format compatibility. Mixed inherited/applied damage data now adopts this capability; MORE and final hit consumers remain next. [Accepted contract](owned-application-group-contribution-queries-proposal.md). |
| Final incoming-critical ownership | Accepted and implemented October 14 through existing stat-owned Player receivers. Three final formulas reuse configured inputs and preserve exact Enemy reads, own-Stat writes and shared Actor Partial coverage. All 52 source vectors, native/CLI checks and all-five publication pass. Six numeric producers, four Partial supplier queries and complete-build/EHP binding remain open. [Decision](owned-actor-enemy-reads-proposal.md). |
| Configuration/source dispositions | Presentation, exact item-range ownership and common checked leaf accounting are implemented; full semantic closure is not. Use the [current endpoint and exact counts](#current-checkpoint-configuration-admission), rather than historical publication totals. Configuration, preset usage, assumptions, exact Scenario usage and Scenario intent retain five independent completeness obligations. The owner approved checked disposition authority on October 14; complete catalogue/default and outside-origin accounting remain pending. Partial scalar success or local Config proof cannot clear global fallback. Keep scenario usage/assumptions separate and retire redundant variants through the [consolidation follow-up](legacy-retirement.md#configuration-projection-consolidation-2026-10-07). [Evidence and gates](owned-configuration-dispositions-proposal.md). |
| Scenario-owned usage intent | [Accepted October 19](owned-scenario-usage-intent-proposal.md): native applicability, structural discovery, three-layer resolution and Import projections/receipts are implemented and tested. Configuration-scoped skill state remains separate from independent skill presets; preparation resolves all eligible exact occurrences and retains dormant intent and exact Scenario overrides. Source-backed structural census, actual stage consumers and complete intent inventories remain pending. Do not use Skill external reads or infer source-only status from an absent saved skill. |
| Numeric build progression | [Implemented and published locally October 19](owned-progression-input-proposal.md): the existing reward adapter projects numeric parameters and accounts for exact selected/losing origins. Focused Import tests, actual campaign rules and all-five correspondence checks pass. Campaign boss progress has no constructor default; both count lanes skip zero. Native assessment detects duplicate authority; final candidate admission remains separate. |
| Requirement assessment beside measurements | [Accepted October 19](owned-requirement-assessment-proposal.md): implemented locally for ordinary and support metric attempts, with separate coverage and exact outcomes. Tests retain prefix violations after failed support preparation and computable measurements for invalid candidates; strict lint checks pass. Action-context census and consumption by the future owned search adapter remain explicit follow-ups; injected-requirement assessment is not complete game legality. |
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

**Corpus source added October 20:** use
[poe.ninja PoE2 builds](https://poe.ninja/poe2/builds) for curated independent PoB
imports when expanding B4. Retain each exact unedited export with its source URL,
retrieval/export metadata, game/patch/league identity where supplied and content
hash. Pin local fixtures so CI does not depend on live rankings or site access.
Choose contrasting mechanics and their interactions, including ordinary builds;
popularity and ladder position are not coverage criteria by themselves.

The end-to-end acceptance gate is unchanged import → complete native evaluation
→ legal configurable optimization → export/reimport, with no user repair of
items, skills, passives or saved settings required. Report each stage separately.
Unsupported mechanics or genuinely missing source information remain explicit
failures with an actionable backlog; do not silently remove them, invent inputs
or hand-edit a build to make the test pass. Search changes are separate candidates
and preserve configured locks. A downloaded/decoded build alone is not a working
optimizer case. This adds a future corpus source; no bulk collection is underway,
and completing the closest supplied build remains the current delivery priority.

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

**Local publication status, October 20:** the checkout is still at `24d12cd8`.
Subsequent critical/armour, progression-adapter, requirement-report and
Scenario-intent, configuration projection, CI-repair, structural-inventory and
ordinary physical-DPS work
is local. A commit/push
attempt was not executed because the automatic approval
service reached its usage limit; this was a service limitation, not a safety
rejection or missing owner authorization. Preserve the tested files and evidence;
resume normal Git publication when that service is available. Hosted CI cannot
validate these unpushed changes yet.

Deferred product decisions: distribution license before a public release,
frontend/packaging before GUI delivery, and acquisition sources before live
trade ingestion. They do not block current native build support or cleanup.
