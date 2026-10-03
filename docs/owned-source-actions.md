# Saved source settings and owned action selections

`resolve-owned-action` converts one caller-selected physical source occurrence
and one reference context into an existing `ImportQueryTarget`. It runs native
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

## CLI and existing normalization

```powershell
poe-optimizer resolve-owned-action BUILD.xml --release RELEASE_DIRECTORY `
  --correspondence ADAPTER.json --request REQUEST.json
```

The command prints a report with `source_execution`, `calculation` and
`whole_build_parity` explicitly false. An application can put `report.target`
into an explicit query template and use `normalize-owned`. That normalizer must
still find the actual materialized SkillUse and physical Gem, preserve their
source links, and retain every unresolved inventory. This command does not
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

The proof then binds the actual materialized SkillUse, its physical Gem and its
containing preset to that preset's real Pending usage inventory. It retains the
existing usage records and issue, adding exact source links. A missing inventory
gets a Pending obligation; a completed, detached or foreign obligation cannot be
used to claim this proof. Private proof tokens cannot be serialized or supplied
by callers. Only after attachment can the intrinsic list's own issue be retired.
Its former allocation remains reserved, preserving all local IDs and watermarks.

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
