# Frost Bomb saved usage and source lifecycle evidence

The optional complete-source Frost Bomb witness passed on 2026-10-03 with
LuaJIT disabled and enabled. The reports are byte-identical. It proves exact
physical-source ownership of the saved global-effect preference and records
how the original calculation lifecycle consumes it. It does **not** prove a
phase-independent activity gate, native action or exposure calculations, or
whole-build numerical parity.

**Owner decision, 2026-10-05:** classify the discrepancy below as an upstream
PoB initialization bug; exclude its affected Frost comparison for this build
from parity certification and validate the other builds' stability. Do not
change the canonical backend to a warm/retry protocol. This evidence preserves
fresh and rebuilt results separately and adds no lazy-cache state to the native
model. See [implementation.md](implementation.md) for publication/build status.

The owner reaffirmed strict determinism on 2026-10-04. The observed discrepancy
is repeatable across different PoB initialization stages; no repeated
identical-protocol randomness or native evaluator variation was demonstrated.
The complete JIT-on/off reports match, the repeated original matches at every
stage, and the two rebuilds match except their explicitly checked revision.
Not every control was independently repeated within each JIT mode, so this does
not prove determinism for arbitrary inputs, platforms or histories. A future
oracle protocol must use a fixed sequence, independent fresh replays and a hard
failure on disagreement; it must never warm or retry until agreement. See the
[determinism boundaries](execution-and-interfaces.md#reproducibility-and-throughput).

## Scope and preservation

The [Rust witness](../crates/poe-optimizer-pob/tests/owned_frost_bomb_usage.rs)
uses the unchanged complete PoB load and calculation lifecycle. It reuses the
authenticated lifecycle and physical-occurrence observers, with a
[read-only supplement](../crates/poe-optimizer-pob/tests/support/frost_bomb_usage_source.lua)
that joins every saved Frost Bomb to its exact physical object, source ordinal,
containing group and preset. Original functions are authenticated and retained;
no business method, global-effect flag or stat-map entry is patched.

Each JIT mode runs 15 complete loads: all five original XMLs, nine Original05
controls, and a repeated Original05. Every load records four stages:

1. Fresh complete load.
2. One passive original frame.
3. One explicitly requested original frame rebuild.
4. A second explicitly requested original frame rebuild.

There are 30 complete loads and 120 stage snapshots across both JIT modes.
Requested rebuilds set only `build.buildFlag`, as original UI edits do. The
authenticated original frame clears its own caches, advances its output
revision, and invokes the original MAIN and CALCS calculations. The passive
frame does not rebuild. Repeated requested rebuilds produce identical observed
outputs and ownership, apart from the expected output revision increment.

For all five unchanged originals, the reported MAIN/CALCS scalar output maps
also match between fresh load and the first requested rebuild. This comparison
is limited to those captured scalar maps; it does not establish equality of
every source field or any native numerical result.

All five fixture files remain byte-identical. The original corpus has five saved
Frost Bomb occurrences: one in Original04 and four in Original05. Archived
presets remain distinct from the selected preset. The test independently joins
their source ordinals to Import's `SourceProjectEvidence`, and preserves the
same physical objects across all four stages. Duplicate controls retain the
original source234 and independently identify the added source235.

Original05 retains SkillSet4, Spec3, ItemSet2 and ConfigSet1. MAIN remains
Sniper/group3 and CALCS remains Arsonist/group1. Each constructed Frost action
belongs to its exact physical Gem and group, uses the player actor, and resolves
the singleton `FrostBombPlayer` effect and constructed stat-set table1.

## Observed global-switch behavior

The controls are `global-1-false`, `global-2-false`, `disabled-gem`,
`disabled-group`, `duplicate-first-active`, `duplicate-second-active`,
`archived-only`, `global-1-missing`, and `global-1-malformed`.

| Selected Frost input | Fresh and passive MAIN / CALCS | Requested rebuild1 and rebuild2 MAIN / CALCS |
| --- | --- | --- |
| Original explicit global1=true | Present / present | Present / present |
| Explicit global1=false | Present / absent | Absent / absent |
| Malformed global1=`bad` | Present / absent | Absent / absent |
| Missing global1 | Present / present | Present / present |
| Global2=false, global1=true | Present / present | Present / present |
| Disabled physical Gem | Absent / absent | Absent / absent |
| Disabled non-main Frost group | Absent / absent | Absent / absent |

The original loader converts missing global1 to true and explicit malformed
global1 to false. Those source defaults do not broaden the importer: its finite
recipe accepts explicit `true`/`false`; missing or malformed input remains
Pending. Global2 has no second effect to address for this singleton Gem. Its
false control preserves observed actions and outputs in every corresponding
lifecycle stage.

Both independent-duplicate controls expose the same lifecycle distinction.
Fresh MAIN contains both exact physical copies, including the copy with
global1=false. CALCS contains only the true copy. After each requested rebuild,
both modes contain only that copy. Reversing which physical copy holds true
reverses the retained source. This proves occurrence isolation, not duplicate
effect stacking or exposure arithmetic.

Changing only archived global1 flags preserves the selected observations and
outputs in every stage. The repeated original reproduces all four original
snapshots. Every physical source remains explicitly count1 in both XML and the
loaded object; no wider count or group-count behavior is admitted.

## Why fresh MAIN differs

At this pin, `Modules/Data.lua` sets `hasGlobalEffect` when `processMod` visits a
global-effect tag. Shared stat maps can resolve lazily through their metatable.
`CalcSetup.lua:1999` checks that flag before creating the active effect, while
`CalcActiveSkill.lua` can initialize the flag later when it consumes the stat
map. The fresh MAIN pass can therefore construct Frost Bomb before the flag is
known. The subsequent CALCS pass sees the initialized flag and applies the
saved global1 condition.

`CalcsTab.lua:504–507` runs MAIN before CALCS. Initial build loading calculates
once and clears `buildFlag`; an idle frame leaves those outputs intact. A normal
requested rebuild calculates both contexts with the now-initialized metadata.
The witness records this difference rather than warming the reference silently.

With the physical Gem or its non-main group disabled, neither mode constructs
Frost Bomb, and the shared `hasGlobalEffect` metadata remains absent even after
the requested rebuilds. The observer records absent versus true without forcing
initialization. Every action that is actually constructed has the observed flag
true by the end of that calculation.

### Upstream issue disposition

The issue is the pinned source's first-load `hasGlobalEffect` initialization
ordering, not a game mechanic or a native special case. The demonstrated affected
domain is Original05's `global-1-false`, `global-1-malformed` and the two
independent-duplicate controls. Their fresh MAIN effect presence is not an
authoritative native parity expectation. Retain the full observations and label
that comparison **excluded: upstream initialization bug**, never passed. No
unrelated metric or another original inherits this exclusion.

All five unchanged originals' captured scalar maps agree across fresh and
rebuilt stages in the original witness. The supplemental stability gate below
now explicitly repeats every original within each JIT mode. Retain
every original and ordered query; no runtime branch may inspect these fixture
names to change calculation behavior. Revisit the exclusion after upstream
updates rather than perpetuating it as accepted semantics.

### Independent original stability checkpoint, 2026-10-05

The new ignored Rust test
`unchanged_originals_have_strict_source_lifecycle_stability` reuses the existing
authenticated observer and a read-only generic semantic snapshot. It loads each
of the five unchanged originals twice in independent fresh Lua states, in fixed
01..05 / 01..05 order, for both JIT modes: **20 complete loads and 80 snapshots**.
It uses the fixed fresh/passive/rebuild1/rebuild2 sequence without retries or
selecting a new canonical lifecycle.

All same-stage independent replays and recorded cross-stage comparisons pass.
Captured state includes exact saved sources, selected presets, player/minion
action identities and stat sets, scalar outputs, FullDPS and output availability.
Output revision increments are checked separately; no other snapshot fields are
scrubbed. No unchanged original is excluded. Four source-bug exclusions refer
only to authenticated mutation XML hashes/false-copy ordinals in the older Frost
reports; an exclusion never changes a new comparison or becomes a pass.

Reports: `runs/owned-original-lifecycle-stability-01/source-jit-{off,on}.json`,
both **9,184,331 bytes**, SHA256
`17bf4a477928aabb94e5c8def8dab948895078cb6dcac68a5143276821be741a`.
The existing Frost reports and all source inputs remain unchanged. Validation:
`runs/owned-permanent-reward-effects-original-stability-04.log` (44.95 seconds).
This proves repeatability of the captured semantic projection on this machine,
not arbitrary platform/history determinism or native whole-build parity.

The current failure path stops at its first comparison failure and stops the
second JIT mode if the first child fails. No comparison was skipped in this
successful run. Improve failure reporting to accumulate bounded per-original
results and both independent modes before failing overall; retain strict failure
and do not read stale reports after a child fails before publication.

The disabled-group result is scoped to the preserved Sniper selection.
`CalcSetup.lua` permits the selected main group to bypass the ordinary group
enabled check; this witness does not generalize its result to a selected,
disabled Frost group.

## Native input boundary

The reviewed input meaning is an occurrence-owned **requested global-effect
preference**. An owned Boolean can preserve and query that preference under the
existing usage-policy model. It must not claim actual effect presence, exposure
magnitude, support interaction, damage, or native action readiness from this
input witness alone. Physical inventory completion remains a separate finite
proof tied to exact scalar and usage records; preset usage and unimplemented
mechanics coverage retain their Pending/Partial gates.

Future calculation work must implement Frost Bomb's own consumers and report the
known source exception explicitly. Nothing here authorizes borrowing
Pain Offering's activation, recipient, stacking or numerical rules.

## Evidence and reproduction

Pinned source revision:
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`.
Manifest SHA256:
`8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675`.
Skill-identity catalog digest:
`b22849f6afaef20b49a578c2ed88314e014b893a71b7919c7b83ca95c6faa7ea`.
The reports also record the pinned hashes of all ten directly relevant source
files, plus the immutable source identity of every original and control.

Both local reports are **11,285,477 bytes** with SHA256
`5c6afc8d9f48b826e2c26f0e289c1276f57c86eee4ff6654ef303ed7167c2e65`:

- `runs/owned-frost-bomb-usage-source-01/source-jit-off.json`
- `runs/owned-frost-bomb-usage-source-01/source-jit-on.json`

The passing centralized run is recorded in
`runs/owned-frost-inputs-source-03.log` and took 62.64 seconds. Earlier failed
runs exposed the unconditional metadata assumption and then the fresh MAIN
activity assumption; the final reports retain the contradictory source facts.

Run explicitly from the repository root with the complete pinned source present:

```powershell
Remove-Item Env:POE_FROST_BOMB_USAGE_CHILD -ErrorAction SilentlyContinue
$env:CARGO_PROFILE_TEST_OPT_LEVEL='2'
$env:CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true'
$env:CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true'
cargo test -p poe-optimizer-pob --test owned_frost_bomb_usage -- --ignored
```

The ignored test runs bounded child processes for JIT off and on, gives each a
300-second deadline, limits each report to 32 MiB, and compares the complete
serialized reports byte-for-byte. It is optional source evidence and does not
add a PoB runtime requirement to normal native CI.
