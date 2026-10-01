# Active physical-Gem input policies

These reviewed policies use the existing offline Rust Gem compiler. `singleton.json`
selects 26 active Gem definitions with one primary effect; `multieffect.json` selects ten
with two fully resolved potential effects. The five supplied originals contain 55 and 22
occurrences respectively. Source keys and exact owned joins are configuration data, never
runtime skill dispatch.

`statset-primary.json` is now authored and source-validated for twelve additional
physical families / 34 original occurrences, but is **not yet published** in the
current release. Their declared additional-stat-set aliases are absent as standalone
Skill identities and are not appended to the constructed granted-effect list. The
existing `SinglePrimary` compiler policy can therefore describe their physical
inputs without allocating extra Skills or interpreting action stat sets. The
stricter `ResolvedPotentialSkillsV1` behavior is unchanged.

All three policies supply an explicit level envelope of 1..40, the existing optional quality
kind, and independent Boolean corruption / numeric Count level-delta inputs. Each primary
effect has source level rows 1..40 and natural/default maximum 20. The envelope means
unchanged, table-supported physical input; it is not a claim that every such Gem level is
legal in the game. Zero/negative levels clamp in the reference, values above 40 cap, and
fractional levels can fall back to the natural level; those transformations are not silently
performed by this schema compiler. Missing/malformed values remain unresolved.

The complete-source Rust test `owned_active_gem_inputs` executes unchanged `LoadSkill`,
`ProcessSocketGroup` and `validateGemLevel` in both LuaJIT modes. Each lane now checks
48 Gems, 58 potential effects and 1,392 loading cases, including the twelve prepared
families, repeated processing, level bounds,
quality and corruption inputs. Saved quality is not clamped to the UI default-settings
range. The owned quality envelope is an explicit accepted data range, not a source clamp.

All generated memberships and intrinsic input collections remain Partial. Potential effects
are not authored skills, activated abilities, support applications or generated providers.
The 12 additional-stat-set families (34 original occurrences) and five unresolved-command
families (17 occurrences) remain Unmapped. Already Known seed definitions are preserved.

## Reproduce

Starting from the checked corrected release, run these commands for `singleton` and then
`multieffect`, using the preceding family output as the next input:

```text
poe-optimizer compile-owned-gem-inputs PRIOR --catalog data/owned/poe2/3887ae68/import/skill-identities.json --policy data/owned/poe2/3887ae68/active-gem-inputs/FAMILY.json --output COMPILED
poe-optimizer migrate-owned-gem-schemas PRIOR --migration COMPILED/migration.json --output SCHEMAS
poe-optimizer publish-owned-normalization SCHEMAS --normalization COMPILED/normalization.json --output OUTPUT
poe-optimizer assemble-owned-release OUTPUT --output RELEASE
```

The two published policies allocate 72 new parameter slots. All 77 selected original occurrences gain
one reviewed Boolean and one Count value. No inputs or queries are removed. Source tests
establish loading behavior; they do not establish native activation or whole-build parity.
See the [implementation resume](../../../../../docs/implementation.md) for publication and
validation evidence and the [Gem input contract](../../../../../docs/owned-gem-inputs.md).

The prepared stat-set family needs its own publication from the latest checked
release, followed by preservation and fresh-original normalization tests. It would
allocate 24 physical parameter slots; none is allocated by the source test. Preserve
all current quality refinements, provenance and original query rows. Exact action
stat-set membership needs a separate authenticated constructed-inventory/selection
proof; metadata aliases alone do not provide it. The expanded source observations
are in `runs/owned-active-gem-inputs-02`, with parent log
`runs/active-statset-gem-source-tests.log`.

A compiler-only probe against the quality-kind release successfully stages all
twelve definitions and 24 parameters using the unchanged compiler. Its receipt
is `runs/active-statset-compiler-probe-01/validation.json`. This is not a published
release or registry baseline: recompile from the later corruption-flag successor
so its newly allocated IDs and full provenance are preserved.
