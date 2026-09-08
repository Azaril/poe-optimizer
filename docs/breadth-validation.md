# Breadth validation and build corpus

This document defines the corpus workflow and records its first intake. The delivery order
and unchecked work are in the [implementation plan](implementation.md#breadth-of-validation-and-data-driven-build-admission--next-phase).
The [design](design.md#data-driven-builds-and-breadth-of-validation) requires caller-provided
builds and injected game definitions throughout the production path.

## Supplied corpus, 2026-09-08

The user supplied five PoB share strings, one per line of [example.import.txt](../example.import.txt).
An immutable `imports.txt` copy sits beside the decoded corpus so future edits to the
user's working file do not invalidate historical evidence. The complete file is preserved
at 68,354 bytes, SHA-256
`3e763f109adb27d48f2cf63a8a95aaea649e5336dcaf37959931725c29f6c745`.
The [intake index](../tests/fixtures/builds/breadth-20260908/index.json) records exact decoded
XML files/hashes, selections, original version attributes, backend/binary identities and
first-pass outcomes. These complete inputs are development examples, not five independent
held-out archetypes. The original archive fixture and six independent numerical goldens
remain separate and unchanged.

| Line | Character | Reference selection | Saved skill / item / tree sets |
| --- | --- | --- | --- |
| 1 | Level 96 Sorceress, Disciple of Varashta | Kelari; selected Sand Djinn action Kelari's Deception | 1 / 1 / 1 |
| 2 | Level 88 Mercenary, Gemling Legionnaire | Twister, active skill set 6 | 6 / 6 / 6 |
| 3 | Level 93 Monk, Martial Artist | Whirling Assault, average-damage mode | 1 / 1 / 1 |
| 4 | Level 96 Mercenary, Gemling Legionnaire | Crossbow Shot, despite many other configured skills | 1 / 1 / 1 |
| 5 | Level 92 Sorceress, Disciple of Varashta | Skeletal Sniper; selected minion Basic Attack, active skill set 4 | 6 / 6 / 7 |

All five decode and complete a fresh pinned PoB evaluation. The source is
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`; observations used existing stable executables,
not the action-timing working tree. Every saved tree spec declares `0_5`, while every
Build element retains `targetVersion="0_1"`. Preserve both fields; do not infer the tree
version from the latter or silently rewrite the import.

None includes any group in Full DPS. A zero or absent roll-up must not become the default
damage objective. Line 3's average-damage selection also requires an explicit usage model
before it can serve as a DPS objective. Line 1 has three unresolved entries in the reference:
Spectre: Powered Zealot, Navira's Well and Kelari's Deception. The selected minion action can
resolve while a separate configured gem entry with the same label remains unresolved.
Keep actor/action selection and gem resolution distinct.

At the initial intake, all five native runs stopped at the XML compatibility guard for literal tabs/newlines
inside attributes. Each has exactly one such attribute: active ConfigSet 1's
`questAct 2Valley of the TitansMedallion` input string. Line 1 encodes Charm Effect Duration
plus a Charm Slot; lines 2–5 encode Charm Charges Gained plus a Charm Slot. This is a semantic mismatch between native XML normalization and PoB's
parser, not proof of malformed inputs. Do not strip or rewrite those fields to manufacture
coverage. Resolve source-preserving import semantics first, then report the next unsupported
mechanics. Minions, grants, multiple sets, conditional support networks, weapon-specific
attacks, triggers, ailments and other represented mechanics remain beyond the small current
Spark/Mace admission boundary. A first error is not an exhaustive coverage report.

The generic runner reproduced all five imports and fresh reference evaluations, preserving
every input and executable hash, in `runs/breadth-validated-20260908/index.json`. All five
native outcomes remain explicit first-stage rejections; exit 1 was expected.
`runs/breadth-runner-validation-summary.json` reconciles those outcomes. Six standalone
runner tests cover bounds, exact lines, set ownership, failed entries/backends and timeouts;
CI runs them on both operating systems with Python 3.13 using
[the official setup action](https://github.com/actions/setup-python).

Local initial raw evidence: `runs/breadth-intake-20260908/manifest.json`, individual `.pob.json`
outputs, decoded XML and `runs/breadth-intake.log`. Numerical results are diagnostic
observations, not newly certified goldens. The checked-in index deliberately records
identities and coverage observations without calibrating native expectations from them.

## Source-preserving configuration inspection

The offending values exactly match options in pinned `QuestRewards.lua`. PoB's XML reader
preserves literal whitespace and decodes five named entities; its writer leaves LF/tab
literal in attributes. ConfigTab retains the selected string and the quest consumer splits
it into modifier lines. Normalizing it to spaces would join two effects and change semantics.
The shared reader now preserves raw ranges, exact bytes and one-pass named-entity decoding
for configuration scalars and custom blocks. Immutable projections retain all source sets,
requested versus resolved active-set identity, fallback provenance, and cross-kind record
order. Input and Placeholder nodes remain distinct authored records; unknown keys and child
fragments remain visible. This is authored-source evidence, before ConfigTab migrations,
defaults, Placeholder destination rules or numerical effects.

`inspect-configuration` requires one caller-supplied XML/share-code input. It works with the
native-only CLI and needs no data package or PoB runtime:

```powershell
cargo run -p poe-optimizer-cli --no-default-features --locked -- inspect-configuration path/to/caller-build.xml --output runs/configuration.json
```

Report schema 1 binds the input and decoded XML hashes to the projection. It explicitly marks
effective configuration and mechanics as not evaluated, legality as not checked and reference
calculation as not run. Output files must be new. The reader has explicit byte/node/set/record
bounds; malformed or ambiguous source is rejected, with its original error location.

Both native admission and controlled-search scalar readers now use this shared source reader.
The native lexical guard exempts literal attribute whitespace only in successfully projected
`Input.string` ranges. It does not exempt names, titles, placeholders or arbitrary unknown
fragments. There is no quest-name or fixture-specific exception. Existing key, mechanic and
structural capability checks still apply afterward.

Independent tests execute the pinned original XML reader/writer and ConfigTab Load/Save/set
methods. They cover all five originals, LF/CRLF/tab, one-pass named entities, exact numeric
bits, active/inactive selection, defaults/fallbacks, cross-kind overwrite order, compatibility
migrations and disabled blocks. They also show the actual quest consumer sees two lines
where normalized XML supplies one. UI callbacks are stubbed and the default-variable list is
empty in the ConfigTab oracle, so these checks do not certify full default resolution.
Original XML compatibility oracles now live with the optional PoB tests, avoiding an
engine-to-import dependency cycle. Portable import/native libraries remain Lua-free.

The configuration-definition audit resolves all **59** corpus keys: **58 Inputs**, **170
Placeholders**, and **one custom block**, across five active ConfigSets and no inactive sets.
The source contains **564 variable definition rows for 563 keys**; ordered duplicates cannot
be collapsed into a key map. Source option lists are UI choices, not a universal import
whitelist. The [injected catalog proposal](configuration-data-proposal.md) specifies separate
default origins, exact typed options and capability-aware resolution. Catalog extraction and
production injection are the next bounded delivery, not implemented by this projection.

A fresh schema-2 run, `runs/config-breadth-validated-20260908/index.json`, reproduces five
imports, five source projections and five fresh PoB successes. All decoded hashes match the
immutable intake index, with no changed input or executable. The next native error for all
five is `Unsupported native build fields or content in PathOfBuilding2`, rather than an
attribute-whitespace error. `runs/config-breadth-summary.json` reconciles source counts and
outcomes; all nine runner tests pass, including malformed reports and modified/deleted
inspection inputs. A mixed run still exits 1 because native exclusions are failures, not
passing parity cases. All 110 fresh PoB measurement records (22 per build, including
availability/status) exactly match the prior intake; evidence is
`runs/config-breadth-reference-repeat.json`. This is reference repeatability, not native
numerical parity.

This source fix does not admit the corpus natively. Charm duration/charges/limit, broader
skills and actor dependencies remain unsupported. The inputs all have multiple groups and
two have six skill sets. Keep complete source files intact and report the next observed
native restriction instead of deleting configuration to force a pass.

## Unresolved labels and actor ownership

A source audit of line 1 explains the three unresolved configured entries without inventing
replacements. Groups 6, 13 and 14 contain name-only `Gem` rows, with no gem, skill, variant or
minion identity. [SkillsTab](../vendor/path-of-building-poe2/src/Classes/SkillsTab.lua)
loads explicit IDs first (`:303`), then uses its gem-name catalog lookup (`:1178`, `:1242`).
That lookup does not resolve arbitrary minion action labels.

- Group 14's **Kelari's Deception** label is separate from the selected action. Valid group
  1 owns `SummonSandDjinnPlayer`, which creates `SandDjinn`; its selected action 2 is
  `ExplosiveTeleportSandDjinn`, with that same display name.
- Group 13's **Navira's Well** is the name of `WaterDjinn` action 3,
  `ESRechargeForceRestartWaterDjinn`. Valid parent entries in groups 9 and 18 select action
  1. A matching label is not authority to change those selections.
- Group 6's **Spectre: Powered Zealot** matches two distinct source monster IDs,
  `VaalZealotSpearLightning` and `VaalZealotDaggersLightning`, with different action lists.
  The build has no saved spectre identity to disambiguate them.

Summoner/minion/action ownership is established by original `CalcActiveSkill.lua:903` and
`:1116–1184`; ordered Djinn action lists are in `Data/Minions.lua:998–1040` and
`:1098–1140`. Action definitions are in `Data/Skills/minion.lua:1703` and `:2302`, and the two
spectre definitions are in `Data/Spectres.lua:12358` and `:12404`. Internal gem catalog keys,
external game IDs, variants, summoning effects, minions and actions are separate identities;
even the Djinn catalog's `Gems` path versus external `Gem` path spelling is intentional.

The existing reference coverage retains these unresolved rows and offers related candidates
only as hints. Full audit evidence and exact source hashes are in
`runs/unresolved-skill-inventory.json`. This audit used existing fresh observations and pinned
source; it neither changed the builds nor evaluated invented replacements. Intended spectre
identity and the purpose of the loose action labels remain unknown. The general build model
must preserve actor/action ownership rather than merge rows with matching display names.

## Next container and selection barriers

All five originals have ten top-level sections and no root attributes or non-whitespace root
text. The native root guard excludes the same four sections: `Import`, `Party`, `Calcs` and
`TreeView`. This is the next observed error, not a reason to remove these source records.
Local evidence is `runs/config-next-root-barriers.json`, with exact XML/source hashes and
record locations. Original dispatch is in `Build.lua:543–604` and `:2691–2729`.

- The observed `TreeView` attributes are viewer state (`PassiveTreeView.lua:59–75`).
- `Import` retains workflow history/preferences (`ImportTab.lua:556–573`); all five have
  `exportParty=false`. Loading this section does not itself import another character.
- `Party` is empty in these five inputs. General `ImportedBuffs` records feed calculations
  (`PartyTab.lua:516–544`, `CalcSetup.lua:912`, `CalcPerform.lua:1219`), so blanket ignoring
  would be unsafe for other builds.
- `Calcs/Section` rows retain display collapse state, while `Calcs/Input` includes
  `skill_number` and `misc_buffMode`. These select CALCS-mode action/effect semantics
  (`CalcsTab.lua:199–237`, `CalcSetup.lua:803–812` and `:1888–1895`). MAIN calculations use
  `Build.mainSocketGroup`, a separate selection. Legacy Calcs-to-Config migration also
  exists at `ConfigTab.lua:1243` onward.

Static inspection after this root guard still finds multiple groups in lines 1/3/4 and six
saved skill sets in lines 2/5; no modified originals were passed off as admitted builds.
Further represented fields include weapon sets, companion actors and Build/Buffs metadata.
A classified source-preserving container projection should precede expanded admission.
Keep display/workflow metadata, authored calculation selections and implemented mechanic
capabilities distinct. The general actor/build model still requires its planned review.

## Runtime input audit

The normal `import`, `evaluate` and `search-build` commands read caller-supplied input.
Search templates and inventory bounds come from the problem data. There is no fallback
character in those paths. However, native admission still uses closed Spark/Mace profiles;
a path argument by itself does not make that architecture sufficiently general.

The developer `search-calibration` command now requires a caller-supplied `--catalog` in
[catalog_search.rs](../src/catalog_search.rs), with no embedded XML or fixture fallback.
Its schema 1 manifest contains bounded, distinct ID/path entries; paths resolve from the
manifest and each input passes through the portable XML/share-code decoder. The runnable
[example manifest](../examples/calibration-catalog.json) selects the four old regression
fixtures explicitly. Other documents are accepted for finite diagnostic comparison without
being projected into canonical mutable candidates.

Report schema 2 separates exact requested input/XML identities from observed PoB summaries,
action/configuration evidence, coverage and normalized-export hashes. Fresh finalist checks
establish numeric consistency under the same backend; generic realization and game legality
remain `unverified`. Exporting the exact requested XML does not establish that PoB preserved
all its semantics. The old `PobCandidateCatalog` four-fixture allowlist and fixed realization
checks remain separately tested library debt, outside this production command path. Fixed
test inputs remain appropriate; production build-ID special cases remain a breadth concern.

## Reproduce corpus intake

The reusable standard-library [runner](../scripts/intake-build-corpus.py) accepts the input
list and executable/backend paths explicitly. The output must be a new directory with an
existing parent. For example, after building a CLI with the optional PoB backend:

```powershell
python scripts/intake-build-corpus.py --input example.import.txt --output runs/breadth-check --import-cli target/debug/poe-optimizer.exe --backend native=target/debug/poe-optimizer.exe --backend pob=target/debug/poe-optimizer.exe --pob vendor/path-of-building-poe2 --inspect-configuration --jobs 2 --deadline-seconds 600
python scripts/test_intake_build_corpus.py
```

`--data` and `--options` select caller-supplied native data and evaluation assumptions;
`--data-sha256` retains the native CLI's external review digest contract. Every source line,
including blank lines and original line endings, is preserved with byte offsets and hashes.
The index retains saved skill/item/tree/config set identity, independent backend results,
exact raw reports and exported XML. It never fills a failed entry with a fixture.

Optional `--inspect-configuration` calls the explicitly selected import CLI for source
inspection before independent backend evaluations. Manifest schema 2 records whether it was
requested, raw report hashes, exact commands and validated per-set source counts. The ordinary XML
summary labels its attributes as standard-XML-normalized; use the configuration projection
for PoB source values. Inspector failures remain separate from numerical backend outcomes;
changed decoded XML is rejected before it can be evaluated as the original import.

Exit 0 means the requested observations completed, not numerical parity or certified legality.
Exit 1 retains per-entry/backend failures or changed-input evidence; exit 2 is a setup error.
Unsupported native builds therefore make a mixed native/reference run exit 1 while successful
reference observations remain available. Deadline exhaustion is explicit per operation,
and running child process trees are reaped. The bound on line count is enforced during
scanning. No executable, default character or data package is selected by this runner.

## Validation gates

A reusable runner must take the corpus path, output directory, backend/data selection and
budgets as arguments. Preserve all entries and fail per entry, not by dropping unsupported
builds. Record exact source/effective selections and settings before comparing metrics.
Classify import fidelity, realization, legality, supported mechanics, measurement availability
and numerical parity independently. Cached XML stats are never the reference calculation.

After inventory, select additional whole-build holdouts across missing mechanism families
before implementing their shared dependencies. Keep derived small reproductions and
perturbations separately traceable to unchanged originals. Compare complete builds against
fresh reference results and inspect intermediate actor/action outputs when discrepancies
appear. Do not relax tolerances, reselect a convenient skill, or delete interacting supports
to turn an excluded build into a passing case.

Ordinary and ascendancy trees use separate roots and point budgets; see the
[tree investigation](tree-topology-investigation.md#ascendancy-components-and-separate-point-budgets).
The 14 missing source targets are a separate diagnostic from valid ascendancy components.
Broader allocation cases must cover weapon sets and provider-dependent exceptional roots.

Progress means visible coverage gains across families, not just additional variants of one
skill. Native benchmarks must disclose admitted/excluded builds and separate preparation,
allocation-free calculation and whole-search costs. Broad numerical parity, useful multicore
execution and source-preserving realization are all required before claiming full native
replacement.
