# Breadth mechanism inventory, 2026-09-08

This evidence inventory covers the five complete caller imports, every saved item/passive
set, and fresh MAIN calculations from the pinned PoB source. It prioritizes shared native
work. It does not certify game legality or broaden native admission. Current delivery and
validation status belong in the [implementation log](implementation.md).

The source remains `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`; selected portable data remains
schema 13, SHA-256 `91d72da5882d40c822044763e19e9894e26027bb8d97766b4d65b27e8e586acb`.
The five exact imports and six independent numerical goldens are unchanged. The original
corpus is development data; its alternate saved sets are not independent held-out builds.

## Equipment and passive occurrences

| Build line | Authored items | Saved item sets | ModRange records | Saved passive specs |
| --- | ---: | ---: | ---: | ---: |
| 1 | 16 | 1 | 74 | 1 |
| 2 | 34 | 6 | 133 | 6 |
| 3 | 17 | 1 | 89 | 1 |
| 4 | 21 | 1 | 89 | 1 |
| 5 | 28 | 6 | 101 | 7 |
| Total | 116 | 15 | 486 | 16 |

The inventory spans 80 rare, 16 unique, 15 magic and five normal items. All 486 authored
ModRange values are 0.5, spread across 102 items. Original ItemsTab loading processes item
text and these range instructions in order before building modifiers; saved/normalized
item text is a different artifact. Header, tag, range, variant, quality, catalyst, rune,
enchant and affix interpretation must retain the source-defined ordering around modifier
parsing and assembly; some rune and quality processing occurs after parsing.

The saved item sets contain 330 Slot rows, including 169 nonzero item references. Their 36
SocketIdURL records are URL metadata. The actual 21 jewel assignments belong to passive
Spec/Sockets records; these are not interchangeable. Five RuneSlot records name None.
Thirty-seven inventory IDs are reused across saved item sets, and one item is unreferenced
by any authored slot or spec. Preserve inventory identity separately from equipment instances
and saved selections. The authored literal `useSecondWeaponSet="nil"` remains different
from absence even though the source comparison with `"true"` yields false for both.

Twenty-five authored base-position strings exactly equal selected native base names, and
196 unchanged item lines match an injected modifier template's syntax. Neither observation
establishes item parsing, modifier scope or admission. Every item has at least one proven
failure of the current structural/header/rarity/base gates. Adding base names alone would
leave the rest of the item pipeline unimplemented.

Across 16 passive specs, 1,369 authored node occurrences map to **549 supported native node
views, 788 explicitly excluded views and 32 implicit roots**. Across the five active specs,
those counts are **225 / 378 / 10**, totaling 613 occurrences. These are node-level lookup
results; surrounding skills, items, allocation modes and effects remain separate requirements.
No authored allocation ID is missing from the full authenticated 0_5 tree. All 331 explicit
attribute override occurrences reference allocated nodes and have no conflicts. Seven specs
use weapon allocations; the inventory also retains four multiple-choice selections and one
unlock constraint. No mastery selections or unknown Spec attributes were found.

Ordinary, ascendancy and weapon-set point counts must retain their own rules. Character
level, quest/progression state, ExtraPoints and PassivePointsToWeaponSetPoints can affect
budgets. Observed counts or saved-set titles do not supply an entitlement. Inactive specs
may represent earlier progression states; do not infer their intended level from their titles.

## Isolated allocations and provider evidence

Build 1 has six ordinary notables without an ordinary allocated path: 338, 8483, 47441,
49088, 55180 and 59387. Fresh PoB state records each as `connectedToStart=false` and links
each through `intuitiveLeapLikesAffecting=[7960]`. Socket 7960 contains item 16, From Nothing
(Diamond), with the Ritual Cadence allocation provider. The reference has established their
radius-provider relationship; they must not be rejected by plain graph reachability alone.
This does not by itself establish the character's point budget or all gameplay legality.

The raw tree's linked IDs include class-to-ascendancy connectors. Raw connected-component
counts therefore cannot define point categories. Ordinary and ascendancy allocations still
use their respective semantic roots, ownership and budgets. The 14 dangling source target
IDs from the [earlier investigation](tree-topology-investigation.md) remain a different issue:
none of these five builds allocates a source-missing ID.

## Observed actors and dependencies

The read-only reference harness captures the MAIN environment after the original calculation.
It preserves actual action, support, grant and receiver relationships alongside normalized
modifier records and source hashes. Functions remain explicitly opaque descriptors. A
retained predicate or modifier is not proof that a particular query applied it.

| Build line | Prepared player actions | Owned minion representations | Minion actions | Provider-to-group grant links |
| --- | ---: | ---: | ---: | ---: |
| 1 | 22 | 15 | 42 | 4 |
| 2 | 20 | 1 | 1 | 3 |
| 3 | 17 | 1 | 1 | 2 |
| 4 | 19 | 0 | 0 | 2 |
| 5 | 16 | 8 | 24 | 3 |

These are calculated representations and relationships, not counts of simultaneously living
in-game creatures. The five active MAIN environments include 310 support-to-action links, 47
constructed buffs and one observed trigger binding. Equal effect IDs cannot authorize
merging their instances or their support sets.

Concrete requirements include:

- Build 1 has separate manual and tree-granted Djinn instances with different supports.
- Build 3's Living Lightning support creates an owned minion action. Its modifier records
  scale from the parent's Strength and Dexterity, so a player-only stat context is insufficient.
- Build 4 has a Tornado instance bound to MetaCastOnDodge and another independent Tornado.
- Build 5 selects skill set 4 and item set 2. Its selected Sniper's modifier store receives
  Pain Offering damage/speed modifiers; that supporting skill is part of the dependency model.
- Enemy stores in builds 4/5 contain Frost Bomb exposure/resistance records; build 4 also
  contains Elemental Weakness records. Build 2's player store contains Purity of Lightning
  and War Banner records. These are observed receiver records, not isolated effect-size tests.
- Source item grants are collected before inactive weapon equipment is filtered, and later
  generated-group handling applies slot context. An active-equipment-only grant inventory
  would discard necessary provider evidence.

The retained modifier families include Condition, ActorCondition, PerStat, PercentStat,
Multiplier/MultiplierThreshold, StatThreshold, GlobalEffect and skill filters. Build 3
includes a parent-actor PerStat dependency and Energy Shield dependent on body-armour Evasion;
build 2 includes speed dependent on Accuracy. Even an emitted FLAG may have a Life threshold.
Original GetCondition also considers overrides, parent stores and FLAG producers. Do not
flatten these records into a bag of unconditional booleans or independent scalar bonuses.

Fresh live nodes also expose reference coverage limits: IDs 23265/36891/43426 in build 1,
39595 in build 3, and 10561/23265 in build 5 have `unknown=true`, empty `modKey`, and no
recorded granted skills. Their actual parsed `modList` and merged `finalModList` are
present and empty; `baseModList` is absent. This establishes no observed contribution
through that passive-modifier pipeline, without making a claim about unrelated source
consumers. Their descriptions must not be treated as implemented effects merely because
they appear on a valid allocated node. Preserve source/reference limitations
separately from mechanics the Rust evaluator has not implemented.

## Shared delivery priorities

1. Preserve and interpret ordered item text/range instructions, inventory identities and
   independent saved selections against injected item definitions.
2. Resolve item-derived passives, radius providers, weapon allocation modes and point-budget
   producers before claiming legality. Preserve physical IDs separately from effective views.
3. Preserve instance-owned grants, action lists, support application, parent actors and
   cross-actor buff/debuff dependencies through the proposed general build model.
4. Extend native modifier query/producer semantics for those dependencies, with source-derived
   operations and complete context. Record unavailable and source-unknown effects explicitly.
5. Port complete skill pipelines using that shared preparation, then validate interactions
   against complete builds and independent holdouts. Another isolated skill allowlist does
   not resolve the common item/actor/configuration dependencies.

The [B3 model proposal](general-build-input-proposal.md) remains under discussion before its
architecture migration. Item/source parsing and data inventory can proceed independently.
B2 still needs a complete native capability matrix for the retained modifier/dependency
records. B4 still needs held-out whole builds across missing mechanic families. Upstream
system tests supply useful independent synthetic regressions, including ailments, triggers,
reservation and minions; they do not substitute for those whole-build holdouts.

## Reproduction and evidence

The shipped corpus runner accepts the caller's imports, executables, data and budgets:

```powershell
python scripts/intake-build-corpus.py --input example.import.txt --output runs/new-breadth-check --import-cli target/debug/poe-optimizer.exe --backend native=target/debug/poe-optimizer.exe --backend pob=target/debug/poe-optimizer.exe --pob vendor/path-of-building-poe2 --data crates/poe-optimizer-data/data/game-data.json --inspect-build --with-definitions --jobs 2 --deadline-seconds 600
python scripts/test_intake_build_corpus.py
```

Manifest schema 3 records source/identity inspection separately from numerical outcomes.
All source occurrences and lookup records must retain their source ownership and selected
data identity. Inspection failures remain visible; altered XML is never passed on as the
original build. Local `runs/breadth-mechanisms-corpus` repeats all 110 prior reference
measurements and all source/identity counts; native's existing one-Skill/one-SkillSet
rejections remain explicit. The mixed run therefore exits 1.

Local audit artifacts are `runs/breadth-items-inventory.json`,
`runs/breadth-passives-inventory.json` and `runs/breadth-dependencies-analysis-3/summary.json`,
with per-build details and exact source anchors. Parameterized audit scripts and the
read-only reference harness remain under `runs/breadth-items-*`, `runs/breadth-passives-*`
and `runs/breadth-dependencies-*`; they are investigation tools, not native calculation
implementations. The [living checkpoint](implementation.md) records final validation and
publication evidence.
