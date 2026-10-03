# Skeletal physical inputs and first-child topology

This packet uses the existing schema V5 / operations V17 release migration and
V3 physical-input disposition contracts. Its exact predecessor is
`runs/owned-sniper-inventory-01/package`, input
`2c215cce13539ec9a3f9d35fae361f22cf5bef1a0e935f4d852adfe180600ddf`.

The migration preserves the physical Arsonist, Frost Mage and Reaver Gems and
their six corruption inputs. It appends primary supply/grant members, promotes
three summon Skills and three first-child Skills from Unmapped to Known with
Partial declarations, and declares three separate actor providers. Existing
ordinary action part `0007` and mode `0008` are reused; five distinct stat sets
represent the actual constructed first-child alternatives.

| Family | Gem / summon | Actor | First child / output | Constructed stat sets |
| --- | --- | --- | --- | --- |
| Arsonist | 0914 / 0323 | 3287 | 0131 / 328c | 328d Fire Bomb, 328e Hidden |
| Frost Mage | 0917 / 0326 | 3291 | 0148 / 3296 | 3297 Projectile, 3298 Explosion |
| Reaver | 0918 / 0327 | 329b | 0292 / 32a0 | 32a1 Basic Attack |

Exactly 29 addresses, `3285` through `32a1`, are allocated: three Actors, five
stat sets and 21 structural slots. There are no new mappings, tables, metrics,
scalar values or numerical programs. Nine ordinary ActivateGrant programs
connect Gem to summon, summon to actor, and actor to first child. They express
potential structural supply; they do not compute participation, activity,
effective count, reservation or damage.

All existing Gem fields and Partial closures survive. New summon and child
parameter inventories remain Partial. The existing native Actor schema has no
authored parameter or socket sites, so those two domains are Complete empty;
actor stats require producers under Partial rule coverage. The remaining actor
declaration inventories stay Partial. Additional actor
children (Explosive Demise, Ice Armour and Enrage), missing Command references,
final level/quality producers, action choices and numerical mechanics remain
unconverted. Only the reviewed output part, mode and constructed stat-set
lists are Complete. There is no evaluation bundle.

`dispositions.json` contains three predecessor-bound authoring templates.
After checked migration, the helper rebinds only their definition and role
identities to the endpoint and appends them to the existing V3 inventory.
The shared singleton-minion adapter resolves the exact actor and first-child
paths. Missing singleton selections select the reviewed first child/table1;
malformed values, second-child selections and foreign maps remain Pending.
Both first-child tables are represented for Arsonist and Frost Mage. Neither
an empty flag table nor the label Hidden removes a constructed alternative.

The five deferred recipes validate count, both global switches, group count
and full-DPS syntax without creating usage values. A physical list completes
only alongside its actual containing preset's Pending usage inventory. Existing
Ice Nova/Sniper dispositions, 514 support rows, two legacy primary rows, all
scalar/usage recipes, all 110 queries and all unrelated content survive except
their checked dependency identity rebinding. The staging helper compares those
fields and preserves prior schema/rule/registry contents explicitly.

V3 reserves the physical Pending issue during ordinary materialization and
defers attachment until existing Gem/Direct usage consumers have run. A newly
admitted earlier occurrence therefore cannot move the existing usage issue or
intervening SkillUse IDs. Attachment checks the actual same-preset obligation
before retiring only the physical issue. V1/V2 ordering and persisted wire
meaning remain intact; a fresh V3-only preset with no ordinary usage consumer
can create its real Pending usage issue later in this pass.

The shared complete-source witness passed both JIT modes: 62 complete loads
and three lifecycle snapshots per mode, covering 15 original occurrences
(six Arsonists, four Frost Mages, five Reavers), archived preset activation and
independent MAIN/CALCS child/stat-set selections. Both reports in
`runs/owned-skeletal-actor-action-source-01/source-jit-{off,on}.json` are
33,855,014 bytes with SHA-256
`7a30005f1c59f0bde7344236beea633c6beaeae957d794db1538eb2adaf43474`.
`authoring.json` pins both complete reports, all five original XML hashes
through those reports, and 13 source files. The loader observer is removed
before calculation, and the requested JIT mode is verified. Existing Sniper
source evidence remains byte-identical.

To reproduce the optional source evidence, run the ignored
`skeletal_families::complete_skeletal_families_preserve_actor_action_correspondence`
test in the `owned_sniper_actor_actions` PoB test target with both
`POE_SNIPER_ACTOR_ACTION_SOURCE_CHILD` and
`POE_SKELETAL_ACTOR_ACTION_SOURCE_CHILD` unset and `--test-threads=1`. The default authored-data check
does not require local reports; publication must authenticate both reports
before the helper stages an endpoint.

Publication passed in `runs/owned-skeletal-inputs-02/`. The endpoint input is
`1d58362e0a808164fa7d554b665c60abb52d943a17f1a4c66cd6b8edb141d42e`.
All fifteen intrinsic lists complete: three in Original01 and twelve in
Original05. Selected unresolved counts are **115 / 116 / 108 / 121 / 14**.
The test checks thirty original MAIN/CALCS resolutions, nine mutation controls,
all 110 unchanged query rows, eighteen byte-identical rebuilt files and the
unchanged predecessor. Values, local IDs, existing usage records and all unrelated
obligations survive. Usage remains Pending, mechanics remain Partial and there is
no evaluation bundle; complete native originals remain **0/5**.

To reproduce publication from the repository root, supply the exact predecessor
and both authenticated source reports, and choose an output path that does not
already exist:

```powershell
$env:POE_OPTIMIZER_TEST_SKELETAL_INPUTS_PRIOR = 'C:\code\poe-optimizer\runs\owned-sniper-inventory-01\package'
$env:POE_OPTIMIZER_TEST_SKELETAL_INPUTS_OUTPUT = 'C:\code\poe-optimizer\runs\owned-skeletal-inputs-repeat'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_OPT_LEVEL = '2'
$env:CARGO_PROFILE_TEST_DEBUG_ASSERTIONS = 'true'
$env:CARGO_PROFILE_TEST_OVERFLOW_CHECKS = 'true'
$env:CARGO_INCREMENTAL = '0'
cargo test -p poe-optimizer-cli --test owned_skeletal_inputs_cli --locked -- --include-ignored --test-threads=1
```

The test writes the endpoint, package/rebuild, normalized original/control
drafts and `validation.json`. This publication gate does not establish numerical
parity or report a final lint, portability or hosted-CI result.
