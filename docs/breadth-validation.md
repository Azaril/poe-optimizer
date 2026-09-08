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

All five native runs stop at the existing XML compatibility guard for literal tabs/newlines
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

## Next source/configuration seam

The offending values exactly match options in pinned `QuestRewards.lua`. PoB's XML reader
preserves literal whitespace and decodes five named entities; its writer leaves LF/tab
literal in attributes. ConfigTab retains the selected string and the quest consumer splits
it into modifier lines. Normalizing it to spaces would join two effects and change semantics.
The current native compatibility path only has a source-range exemption for `customMods`;
other scalar strings still come from normalized XML attributes.

The next bounded proposal is a shared source-attribute/configuration projection: raw range
and bytes, PoB-decoded text, owning ConfigSet identity/order and active selection. Both import
and native readers should consume it. String-input ranges may admit literal whitespace only
when that projection actually handles them. Keys, scalar kinds, defaults, exact option text
and capability status belong in injected configuration definitions. No Medallion-specific
exception, character-ID branch or normalized-choice fallback is appropriate.

Extend the existing original-parser oracle with LF/CRLF/tab, named entities, active/inactive
sets, renamed injected keys/options and disabled custom blocks. Preserve hashes and exact
round trips; retain failures for lexical ambiguities, duplicate inputs/sets and unsupported
scalar kinds. Generic import can preserve unimplemented configuration while complete native
evaluation still rejects it. Charm duration/charges/limit consumers remain unsupported.

This alone will not admit the corpus: native profiles also require one skill set and one
skill group. The five inputs have multiple groups, and two have six skill sets. General
skill/actor/dependency admission must follow the inventory, independently of XML compatibility.

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
python scripts/intake-build-corpus.py --input example.import.txt --output runs/breadth-check --import-cli target/debug/poe-optimizer.exe --backend native=target/debug/poe-optimizer.exe --backend pob=target/debug/poe-optimizer.exe --pob vendor/path-of-building-poe2 --jobs 2 --deadline-seconds 600
python scripts/test_intake_build_corpus.py
```

`--data` and `--options` select caller-supplied native data and evaluation assumptions;
`--data-sha256` retains the native CLI's external review digest contract. Every source line,
including blank lines and original line endings, is preserved with byte offsets and hashes.
The index retains saved skill/item/tree/config set identity, independent backend results,
exact raw reports and exported XML. It never fills a failed entry with a fixture.

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
