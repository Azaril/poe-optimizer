# Saved source settings and owned action selections

`resolve-owned-action` converts one caller-selected physical or manual Direct
source occurrence and one reference context into an existing `ImportQueryTarget`. It runs native
Rust in Import. It does not load Lua or execute PoB, materialize a build, select
which build to evaluate, activate an ability, or certify numerical coverage.

## Boundary

The injected `PobPhysicalPrimaryStatSetsV1` correspondence binds to the exact
owned definitions, role package, source pins and catalog. Construction checks the
physical Gem's primary Skill, supply, entering grant, player output and declared
part/mode/stat-set membership. Source strings and table ordinals exist only in
Import. The Core action retains owned IDs and an occurrence-specific provider.

The request contains an `ImportSkillUseLocator` (source hash, occurrence ordinal,
expected physical Gem) and `main` or `calcs`. These contexts name PoB reference
settings, not native game state. The adapter accepts repeated or
archived physical occurrences without claiming they are active.

Matching `StatSetIndex` or `StatSetCalcsIndex` children are decoded with the shared
typed value recipe. A reviewed `absent_stat_set` is the only source absence
mapping. Missing indices on present rows, malformed values, duplicates, unknown
indices, unsupported child grammar, namespaces and stale source locators remain
unresolved. The pinned loader overwrites legacy scalar headers; the report
retains their attribute origins without using them as fallback selections.

Correspondence construction and resolution have bounded bytes, work, map rows,
stat sets, value decoding and output. The immutable adapter is reusable. Reports
retain the exact request, source snapshot, correspondence identity and selected
attribute, or a specific Pending reason. The CLI only supplies bounded I/O and
validated package loading.

## Summoned actors and child actions

`PobPhysicalSingletonMinionActionsV1` is an additional Import correspondence.
It binds one reviewed source minion identity to an existing owned Actor and
population, then maps declared child actions to existing Skill supplies, grants
and outputs. Construction checks the complete provider path and the output's
provider-actor role. The source minion identity belongs to the reviewed, pinned
correspondence; it does not introduce a source-name lookup in Core or Engine.

PoB uses its alternate CALCS minion name only when the summon is the main skill
in that context. Its child-action and child-stat-set settings have different
selection rules. This adapter admits a singleton actor only when both possible
CALCS name branches converge on that reviewed actor. MAIN checks its own name.
Missing names or action/stat-set indices need an explicitly authored absence
mapping. Unknown names and unsupported choices remain Pending even where PoB
would fall back or clamp them. Native gameplay does not acquire a CALCS mode.

Nested `MinionSkillIndexLookup` and `MinionSkillIndexLookupCalcs` maps are checked
independently, including every contained `MinionSkillIndexMap` entry. Duplicate
keys, foreign effects, unknown fields and unsupported action/stat-set mappings
cannot be silently discarded. Reports retain actor attributes, child-action
selection and every accounted map occurrence in addition to the selected stat
set. Physical V3 completion requires these origins to cover every present
minion selector and every descendant of the physical Gem.

The [Sniper packet](../data/owned/poe2/3887ae68/sniper-inventory/README.md)
admits Basic Attack. Gas Arrow's three source
stat sets require their own reviewed owned topology; they are not aliases for
Basic Attack's stat set. The separately unresolved Command effect remains a
catalog and mechanics obligation. Reference-action correspondence proves neither
that a summon is active nor that its numerical evaluation is complete.

### Arsonist, Frost Mage and Reaver

The [three-family packet](../data/owned/poe2/3887ae68/skeletal-inputs/README.md)
reuses this correspondence and physical V3 dispositions. It declares three
separate actors and first-child action stat sets: Fire Bomb/Hidden for Arsonist,
Projectile/Explosion for Frost Mage, and Basic Attack for Reaver. Ordinary grant
activation connects the declared providers; parameter, additional-child, Command
and mechanics coverage remains Partial. Source and publication verification have
passed: fifteen intrinsic lists complete, thirty original MAIN/CALCS resolutions
and nine mutation controls pass, all 110 queries remain unchanged, and eighteen
files rebuild byte-identically. At that historical checkpoint, selected issue counts
were **115 / 116 / 108 / 121 / 14** in `runs/owned-skeletal-inputs-02/`;
see the [living plan](implementation.md) for the current baseline.
The deferred V3 attachment below preserves existing local IDs and real Pending
usage obligations. These declarations add no final-input or numerical authority;
complete native builds remain **0/5**.

## Manual Direct sources and intrinsic inputs

`PobManualDirectSingletonMinionActionsV1` reuses the same minion/action decoder
with an explicit `ImportDirectSkillUseLocator`: source hash, occurrence ordinal,
catalog identity and expected owned Skill. The catalog entry must be a reviewed
nonphysical provider. It does not acquire physical Gem supply or an entering Gem
grant. Correspondence construction checks the declared root, actor and child
paths; normalization requires the exact materialized Direct SkillUse from that
source. `ImportQueryTarget::DirectAction` keeps this authority distinct from the
existing physical wire contract.

`PobManualDirectSkillV2` adds an injected intrinsic-field disposition. The original
V1 flat-row contract remains unchanged. V2 accounts for both reference contexts,
all nested selectors, declared raw parameter recipes, finite neutral guards and
five deferred usage recipes. Already declared required authored input slots must
have recipes; projected-only slots remain the generated provider's responsibility.
Partial schema declarations stay Partial even when a saved occurrence's intrinsic
input list is complete.

Physical V3 and Direct V2 share private field accounting, deferred syntax decoding
and containing-preset proof. Normalization consumes a private typed result from
the same semantic inspection used by the public resolver; it avoids constructing
and serializing an unused query target and response. The public resolver retains
its complete report, byte limits and work accounting. Both paths charge the work
they actually perform. They resolve MAIN before CALCS and preserve its failure
short circuit. Their bounded attachment passes run after
ordinary input consumers to preserve existing local identities. Direct attachment
requires the actual Direct SkillUse, its own reserved input issue and the same
preset's live Pending usage obligation. A completed or foreign obligation cannot
supply that proof. No physical Gem is fabricated.

The [Djinn packet](../data/owned/poe2/3887ae68/djinn-actions/README.md) declares one
manual root per source, a separate Command, one Actor and its child actions. A
reference selection is diagnostic; it does not itself express gameplay usage,
choose support recipients, establish support admission or complete final inputs.
Those remain separate preparation and execution gates on the same graph.

## CLI and existing normalization

```powershell
poe-optimizer resolve-owned-action BUILD.xml --release RELEASE_DIRECTORY `
  --correspondence ADAPTER.json --request REQUEST.json
```

The command prints a report with `source_execution`, `calculation` and
`whole_build_parity` explicitly false. An application can put `report.target`
into an explicit query template and use `normalize-owned`. That normalizer must
still find the actual materialized SkillUse and its exact source identity
(including its Gem for physical sources), preserve their source links, and retain
every unresolved inventory. This command does not
silently rewrite query files or the selected build.

Physical Gem materialization permits child maps. The V1/V2 physical-input
inventories retain their flat-row contracts; an additive V3 disposition can
account for reviewed maps using this same correspondence. Action selection by
itself does not complete an input inventory.

## Physical inputs and retained usage obligations

`PobFreshPhysicalV3` separates three destinations for a reviewed primary Gem:
intrinsic assignments, reference-action selections, and deferred usage fields.
Its injected row identifies the physical Gem and intrinsic corruption slots,
binds the existing action correspondence, and supplies exactly five typed
recipes: Gem count, both global switches, group count and group full-DPS flag.
Decoding those fields proves their syntax only. It does not create gameplay
preferences or infer their effective values.

Completion requires both reference contexts to resolve and every Gem child to
be accounted for by those selections. Unknown fields, unaccounted child maps,
malformed values, stale bindings and ambiguous preset IDs retain Pending inputs.
The shared container check requires unique canonical numeric SkillSet IDs and a
valid saved active set; the older strict census still requires flat Gem rows.

Normalization first materializes ordinary Gem and Direct usage inputs and
reserves each V3 physical list's original Pending issue. V3 attachment runs in a
bounded pass after those consumers, so an earlier newly admitted disposition
cannot take over allocation of an existing preset usage issue. V1/V2 allocation
and proof paths retain their existing order.

The deferred proof binds the actual materialized SkillUse, its physical Gem and
its containing preset to that preset's real Pending usage inventory. It retains
existing usage records and their issue, adding exact source links. Only then can
the intrinsic list's own issue be retired; its original allocation stays
reserved. A completed, detached or foreign usage obligation cannot supply this
proof, and private proof tokens cannot be serialized or supplied by callers.

If no ordinary consumer created the usage inventory, the deferred pass creates
a real Pending obligation before attachment. Fresh V3-only inputs can therefore
allocate that new issue later than the previous eager path; historical local
allocation order is not promised for that case. Existing persisted drafts and
wire meanings are unchanged. No fallback creates complete usage or replaces a
failed destination proof with an assumed value.

The [Ice Nova disposition packet](../data/owned/poe2/3887ae68/ice-nova-inventory/README.md)
uses this path without a fabricated global-effect consumer. V3 preserves inherited
support and primary rows and binds an absent usage policy explicitly when no
older primary row requires one. No additional Core, Engine or interpreter path
is introduced; usage and numerical coverage remain separate gates.

## Ice Nova correspondence

The [reviewed packet](../data/owned/poe2/3887ae68/ice-nova-actions/README.md)
supplies one player action with two constructed stat sets, Ice Nova and
Cold-Infused. Metadata aliases are neither additional Skills nor extra stat
sets. All four physical occurrences in Original05 resolve independently; the
originals' 110 requested metrics remain unchanged. Ice Nova is exercised through
separate diagnostic queries.

The action packet adds topology only; the later disposition packet addresses
intrinsic physical inputs separately. Final level/quality, damage tables,
conditional modifiers, usage inventories and execution readiness remain
incomplete. Source assembly adds repeated stat names, so future damage data must
be validated after the source aggregation rather than copied by last-field
replacement. The existing native routing contract can distinguish these exact
owned stat sets; it needs no source ordinal or PoB reference context.

The native binding tests distinguish an invalid alternative on an offered output
from an unavailable output on a particular provider. Neither selects a fallback.
Finite topology tests exercise duplicate copies, disabled providers, query-free
binding, scratch reuse and independent Rayon workers. They do not assert complete
numerical evaluation of the real packet.

## Ice Nova checkpoint reproduction

Default Rust targets are `poe-optimizer-import --test owned_source_actions`,
`poe-optimizer-engine --test owned_ice_nova_actions`, and
`poe-optimizer-cli --test owned_ice_nova_actions_cli`. The real CLI publication
case is ignored by default and requires the exact predecessor plus the two
source reports authenticated by `authoring.json`.

```powershell
$env:POE_OPTIMIZER_TEST_ICE_NOVA_PRIOR = 'C:\code\poe-optimizer\runs\owned-sniper-reservation-01\package'
$env:POE_OPTIMIZER_TEST_ICE_NOVA_OUTPUT = 'C:\code\poe-optimizer\runs\owned-ice-nova-actions-repeat'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_OPT_LEVEL = '2'
$env:CARGO_PROFILE_TEST_DEBUG_ASSERTIONS = 'true'
$env:CARGO_PROFILE_TEST_OVERFLOW_CHECKS = 'true'
$env:CARGO_INCREMENTAL = '0'
cargo test -p poe-optimizer-cli --test owned_ice_nova_actions_cli --locked -- --include-ignored
```

Choose an output directory that does not exist. The test writes the exact bound
adapter, package, rebuilt package, original/diagnostic normalized drafts and
`validation.json`. It verifies predecessor preservation, eighteen identical
rebuilt artifacts, all five original selections and eight mutation controls.
It does not change source builds or assert numerical parity.
