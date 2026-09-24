# Active physical-Gem input policies

These reviewed policies use the existing offline Rust Gem compiler. `singleton.json`
selects 26 active Gem definitions with one primary effect; `multieffect.json` selects ten
with two fully resolved potential effects. The five supplied originals contain 55 and 22
occurrences respectively. Source keys and exact owned joins are configuration data, never
runtime skill dispatch.

Both policies supply an explicit level envelope of 1..40, the existing optional quality
kind, and independent Boolean corruption / numeric Count level-delta inputs. Each primary
effect has source level rows 1..40 and natural/default maximum 20. The envelope means
unchanged, table-supported physical input; it is not a claim that every such Gem level is
legal in the game. Zero/negative levels clamp in the reference, values above 40 cap, and
fractional levels can fall back to the natural level; those transformations are not silently
performed by this schema compiler. Missing/malformed values remain unresolved.

The complete-source Rust test `owned_active_gem_inputs` executes unchanged `LoadSkill`,
`ProcessSocketGroup` and `validateGemLevel` in both LuaJIT modes. Each lane checks 36 Gems,
46 potential effects and 1,044 loading cases, including repeated processing, level bounds,
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

The policies allocate 72 new parameter slots. All 77 selected original occurrences gain
one reviewed Boolean and one Count value. No inputs or queries are removed. Source tests
establish loading behavior; they do not establish native activation or whole-build parity.
See the [implementation resume](../../../../../docs/implementation.md) for publication and
validation evidence and the [Gem input contract](../../../../../docs/owned-gem-inputs.md).
