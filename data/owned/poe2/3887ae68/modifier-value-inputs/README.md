# Canonical modifier value inputs

This extension prepares eleven modifier families for explicit effective-value recipes:
cold resistance, all elemental resistances, four local weapon percentage families
(physical increase, attack speed increase, critical chance increase and flat critical
chance), and five local weapon flat damage pairs. It is a schema and recipe binding
checkpoint. It does **not** admit source lines, establish complete modifier semantics,
or make an original build evaluable.

Apply `extension.json` to the package produced by the ordered modifier-transform
checkpoint, whose registry ends at 9533. The extension adds 310 entries, ending at
9843: four shared stats, eleven modifier definitions and 295 required parameter slots.
It also refines 338 existing item templates by adding canonical members beside the
predecessors already admitted there. Those memberships remain Partial with every
existing gap. Compatibility is not inferred for other item templates.

The predecessor modifier definitions have Complete parameter lists. Changing their
nominal amount contracts would reinterpret existing inputs. Every canonical owner
therefore has a fresh definition and fresh slots, including its copied semantic
property and qualifier facts. Existing owners, nominal slots and programs are unchanged.
`bindings.json` records the predecessor-to-canonical owner and slot migration, the
component roles and sixteen compiler bindings. Its migration entries describe a
relationship; they are not permission to copy nominal numeric values into raw inputs.

The sixteen numeric inputs are **unrounded canonical components**, after proven range
selection and source encoding reconciliation, before internal rounding, corruption and
ordered modifier magnitude. Increased/reduced families require a nonnegative component
and an explicit required `reduced` Boolean. The sign is applied after numeric stages.
Other component inputs retain their sign. The declared million-unit bounds are
computational envelopes, not evidence of legal game rolls. Every property Boolean is
required; absence is not false. Old source grammars continue to target old owners.

The compiler bindings use internal precision 1 and final decimal precision 0, except
flat critical chance, which uses 100 and 2. They target the reviewed numeric stages of
pinned `ItemTools.lua` `formatValue`: resolve to internal units, apply the distinct
corrupted-base factor, apply ordered magnitude, and return to display units. Textual
`applyValueScalar`, baked source strings, cached interpolation and sign normalization
are separate admission concerns. Similar visible numbers do not prove interchangeable
source encodings.

The shared Modifier-scoped output stats are `253e` (effective percent), `253f` (effective
damage minimum) and `2540` (effective damage maximum). Unit and owner occurrence remain
explicit, so sharing a channel does not merge unrelated modifiers. `2541` is the new
Modifier-scoped corrupted-base factor, using the existing factor unit. All bindings
read final ordered magnitude `253d`; catalyst-only intermediate `0a1b` is not a
substitute. Full keys use `def.000000000000` followed by the listed hexadecimal suffix.

No corrupted-base factor producer or default is supplied here. The nine local owners
also have no magnitude producer. Their effective values must remain unavailable until
those facts have a valid producer or explicit authorized binding. The two resistance
owners retain remapped copies of their reviewed catalyst and ordered-magnitude programs.
All eleven owners preserve their predecessor Partial rule gaps and add
`canonical-input-admission-unproved`. Compiling numeric programs does not close these
gaps, admit an effective contribution to a character, or satisfy full-build parity.

Publish and validate this extension before constructing the compiler policy. The policy
must bind the exact successor schema identity and use `compiler_bindings` from
`bindings.json`; its factor unit is also recorded there. It must not bind a predecessor
schema or guess a digest. The compiler then appends sixteen programs through the normal
validated extension path. A checked-in bound policy, when present, is specific to that
published schema, not a floating version alias.

Next, migrate reviewed source line recipes to canonical owners only when native Import
can prove the raw encoding, range, qualifier, source flags and parent occurrence. Add
explicit corrupted-base and local ordered-magnitude producers, then prove downstream
weapon/character channel assembly independently. Retire predecessor owners through an
explicit versioned migration after all consumers move; never repurpose their IDs or
remove them from an existing Complete declaration. Existing imports, the 110 query rows,
allocation Pending gates and the full original-build coverage gate remain unchanged.

## Reproduce the bound package

Starting from the ordered-transform package documented in the implementation checkpoint:

```powershell
poe-optimizer extend-owned-recipe runs/owned-modifier-transforms-01/package `
  --extension data/owned/poe2/3887ae68/modifier-value-inputs/extension.json `
  --output runs/owned-modifier-values-01/inputs-package
poe-optimizer compile-owned-modifier-values runs/owned-modifier-values-01/inputs-package `
  --policy data/owned/poe2/3887ae68/modifier-value-inputs/policy.json `
  --output runs/owned-modifier-values-01/package
```

Use fresh output directories; publication never overwrites an existing bundle. The tracked
policy binds schema `2eb895ad1a08225f48a1b09d9f60a27ce03bdc10461c28b1b18bec795ccebd63`.
The compiler adds 16 programs/540 nodes without a schema or operation-version change.
The published bundle has 9,843 registry entries and 39,041,109 artifact bytes, preserving
all 110 query rows. Neither publication nor this receipt establishes final numerical parity.
