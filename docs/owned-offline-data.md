# Offline owned definition assembly

Status: implemented component data path, 2026-09-15. The controlling boundary is the
[domain architecture](domain-architecture.md); current validation and outstanding work
are recorded in [implementation](implementation.md). Full original-build parity remains
0/5. This pipeline delivers inspected definitions and executable components, not an
implicit claim that all game rules or incoming effects have been converted.

## Data build and runtime are separate

The release data build has three steps:

1. Acquire source definitions at an explicit pin, or author game facts directly. Optional
   PoB source acquisition and independent reference calculations belong here.
2. Convert reviewed facts into the project's persisted schemas, typed finite data tables,
   effect recipes and routing declarations. Keep source selectors, hashes and unconverted
   facts in offline provenance artifacts. Do not use reference output metrics as input data.
3. Assemble and validate all owned artifacts, then publish a new directory atomically.
   Normal native evaluation loads those artifacts with an owned build request; it neither
   reparses source definitions nor runs PoB, a Lua interpreter or UI lifecycle callbacks.

This is an explicit data-build operation, not an implicit source download or PoB execution
on every `cargo build`. Prebuilt owned artifacts can be shipped with native or web clients.
Their IDs and contracts belong to this project. Source-format changes require converter
review; they do not automatically change the evaluator's schema or operations.

## Production assembly

The [Import assembler](../crates/poe-optimizer-import/src/owned_recipe.rs) accepts strict
`OwnedRecipeInput { schema_version, registry, schema, rules, routing }`. It reuses the
existing registry, schema, rule, routing and Engine compilation constructors. It performs
no ID allocation and no evaluation. All schema definitions and slots must be active in
the supplied persisted registry; unused future registry allocations remain allowed.

Schema construction establishes its canonical DataIdentity before rule/routing binding.
Bindings must already be exact. The assembler rejects stale, retired, duplicate or foreign
declarations; it never repairs IDs or silently rebinds supplied rules. Package release
fields retain their existing independent contracts. Table-cell changes can preserve the
registry/schema identities while changing the rule and recipe identities.

Input/output bytes, registry checks and all constituent resource limits are bounded.
Compilation precedes artifact publication. The staged result contains `registry.json`,
`schema.json`, `rules.json`, `routing.json` and `manifest.json`. The manifest identifies
source-free runtime content by hashes and stored/compiled identities, records known
partial owners/routes, and explicitly says calculation was not run and whole-build parity
is not established. A digest proves content identity, not semantic correctness or trust.

The [thin CLI](../src/owned_recipe_cli.rs) writes only successfully assembled artifacts:

```sh
poe-optimizer assemble-owned-recipe data/owned/poe2/3887ae68/recipe.json --output owned-release
poe-optimizer check-owned-rules owned-release/rules.json --definitions owned-release/schema.json
```

The destination's parent must exist and the destination must be new. Same-directory
staging plus atomic no-replace rename publishes the complete directory on Windows/Linux.
Existing empty directories, files and competing publications are not overwritten. Other
hosts can use the I/O-free library; this CLI's directory publication explicitly rejects
unsupported platforms. This guarantees namespace publication, not power-loss durability
of every filesystem or distributed storage backend.

## Initial real data and its limits

The persisted [PoE2 component data](../data/owned/poe2/3887ae68/recipe.json) uses reviewed
Twister and Skeletal Sniper definitions and complete 1–40 finite level domains. The
[allocation ledger](../data/owned/poe2/3887ae68/ids.json) gives human-readable names to
stable registry addresses; no production Rust path dispatches on those names.

The optional [literal exporter](../scripts/export-owned-mechanics.py) accepts an injected
source-selection manifest, identity catalog, recipe and source checkout. It recognizes
only explicitly declared numeric record/array shapes, verifies file/span hashes and
winning declarations, and rejects unsupported expressions, shape changes, duplicate or
missing levels and nonfinite values. Aggregate row/cell expansion is bounded before
materialization. It does not interpret Lua. The current identity
catalog comes from the existing offline snapshot, not the native runtime dependency graph.

Source facts retain unused stat-set actor scaling separately from the ordinary minion
level table. Sniper's unresolved command references remain unresolved. Recipes preserve
physical Gem inputs, generated Skill inputs and actor/action identity. Reservation coefficients
belong to each summoning action. Quality effects multiply injected coefficients and truncate
per stat while retaining the original fractional Gem input. Additive alternate-quality effects
and missing commands keep their explicit coverage gaps: nine rule owners and four routes
remain partial. Incomplete incoming contributions prevent component results from becoming
complete effective actor values. The remaining local equipment, support, effective-level,
actor scaling, scenario, cost and metric semantics must be completed before these data
make either protected original fully executable.

The exporter supports review/refresh of these finite facts. The Rust assembler is still
the validation/publication gate. Source-free tests reload emitted artifacts and exercise
real component formulas; source checks separately verify acquisition fidelity. Neither
replaces independent whole-build oracle comparisons. The exporter exclusively creates a new
output directory; an I/O failure can leave an incomplete directory. Its output is an offline
intermediate, and it does not provide the assembler's atomic publication guarantee.

## Production import identity integration

The [persisted import bundle](../data/owned/poe2/3887ae68/import/README.md) connects source
normalization to these same owned definitions. Its reviewed seed preserves the original
38 allocations and adds reward/option/equipment metadata. Six exact source mappings reuse
the two physical Gems and four Skills already referenced by the mechanics graph. The
standalone source identity catalog is an offline compiler input; native normalization
loads only the emitted registry, schema, mappings, roles and policies.

`compile_owned_skill_catalog_extension` extends under an exact source pin and policy
version. Positive mappings reuse active addresses and full descriptors; genuinely missing
unique selectors allocate new Unmapped definitions. Affected unresolved/ambiguous selectors,
retired targets and known-role contradictions reject. Other schema/mapping entries and
tombstones survive. Repeating with the successor allocates nothing. The mapping proves its
version binding, not the contents of an unrecorded historical policy; the new receipt records
the full current injected policy.

`extend_owned_catalog_recipe` validates the prior recipe and mapping, calls this compiler,
independently checks that every old descriptor is unchanged, and explicitly constructs new
schema/rule/routing/mapping/role bindings. The thin `extend-owned-skill-catalog` CLI publishes
nine artifacts with the shared atomic no-replace directory publisher. Its transition receipt
records old/new identities, reuse/allocation counts and hashes for the complete emitted
family. Runtime load never repairs stale bindings. Whole-input and whole-output byte limits
apply in addition to constituent catalog/schema/compiler budgets.

The optional offline import exporter authors reviewed reward, equipment, quality and syntax
policies as data. A separate explicit policy-binding operation checks seed bindings and the
successor before producing final policy files; it cannot be used to silently repair a bad
old artifact. Source-file pins are canonical paths and LF hashes. Explicit empty item policies
preserve pending semantics and honest provenance. Ordered example query templates contain
only requested identities/targets, never cached metric values. Actual build input is always
a separate caller-supplied XML/share code or owned document.

The production family contains 2,515 stable allocations, including 2,396 newly appended
identity-only Gem/Skill definitions. These new entries deliberately do not assert mechanics
coverage. Existing partial rules/routes remain partial. Both imported examples and authored
requests use the same six reviewed Gem/Skill IDs. The all-five breadth and reference-routing
tests load these shipped policies rather than privately allocating unrelated test catalogs.

## Remaining integration

Selected inputs, support relationships and all applicable contributions must still resolve
before complete effective skill/actor values are available. Raw Gem level is not generally
effective level. Weapon and actor timing must consume shared native kernels after this
resolution. Class/ascendancy, passive access and separate point pools, local item semantics,
scenario effects and metric bindings retain their declared gaps. Completing one import field
must not close unrelated membership obligations. Migrate independent realization checks
before removing remaining profile-specific preparation callers; full-original completion is
still a separate gate.
