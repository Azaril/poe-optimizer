# Two template-scoped item-level absence facts

`defaults.json` adds `item_level: absent` for Rope Cuffs (`2007`) and Tattered
Robe (`238c`). These are reusable owned template identities, not character or
item-instance identifiers. Parameter defaults remain empty and quality remains
Pending in both rows. No schema, registry, calculation rule or completeness
declaration changes.

The existing source V8 policy applies these facts only after proving the whole
item layout and checking relevant headers. A numeric Item Level header retains
its explicit value. Unknown source lines and malformed or conflicting headers
do not acquire a fallback. Absence is `Known(None)`, not level zero or a value
derived from the character.

`authoring.json` binds the exact predecessor, pinned source manifest, five
source modules and the Rust reference test. That test executes complete
authenticated original `ItemsTab.Load`, `Item.ParseRaw` and `Item.BuildModList`
functions in both JIT modes. The untouched selected originals have nil
`itemLevel`; controlled full loads retain explicit levels 37 and 38. Reusing an
item object after removing the header preserves its stale level, whereas a
fresh object is nil. This is why the fact concerns fresh source loading.
Unknown-line controls also produce nil in PoB, but that does not authorize
closing an unresolved owned source layout.

The full endpoint constructor clones the checked predecessor and appends only
the two source facts, a source-policy version and one explicit provenance
record. It publishes with the existing `assemble-owned-release` command and
rebuilds the resulting package. No new production conversion API is needed.

```powershell
$env:POE_OPTIMIZER_TEST_ITEM_LEVEL_PRIOR = 'runs/owned-modifier-category-inputs-01/package'
$env:POE_OPTIMIZER_TEST_ITEM_LEVEL_OUTPUT = 'runs/owned-item-level-absence-release-01'
cargo test --locked -p poe-optimizer-cli --no-default-features --test owned_item_level_absence_cli real_two_template_absence_preserves_saved_requests_and_all_other_inputs -- --ignored --exact
```

The output directory must not already exist. The explicit real test is ignored
by default. `endpoint.json` is reproducible authored input for
`assemble-owned-release <endpoint.json> --output <new-directory>`.

The checked publication is `runs/owned-item-level-absence-release-01/package`:

- Input: `5c765a2528d844bbdef1c1671b26312412c664a354b6b300d15735f5a5727192`.
- Definitions unchanged: `de7ff449b2aecf225a1730a14a6901632daedd721372ef865d8ae17fd66da5ca`.
- Registry unchanged: `13a3d4a7c3c52f887530de2112717a44cf33160a2517fe42ef34778200e223c8`.
- 18 files, 58,433,554 bytes and 14 provenance records. Only `item-source.json`
  and `release.json` differ from the predecessor; the rebuilt package is
  byte-identical and the predecessor is unchanged.

One default test, the explicit real publication and eight controlled import
probes passed. Fresh normalization retains all five original saved selections
and all 110 queries byte-for-byte. Only original 05 source occurrences 572 and
574 change from Pending item level to `Known(None)`. All other canonical fields,
memberships, input coverage, source lines and origin links remain equal after
accounting for the exact two retired issue allocations. Remaining local IDs
shift only by those retired allocations; independently imported lineage IDs
are compared through an injective correspondence, never discarded.

The selected diagnostic counts are 316/322/313/379/149. This removes two missing
inputs, but all five requests remain Pending, calculations are not run, and
numerical coverage remains 0/5. The remaining four selected item-level gaps
belong to Cryptic Leggings, Solar Amulet, Fine Belt and Ashen Staff. Their
unresolved modifier lines still prevent a whole-layout absence proof.
`validation.json` records exact changes and selected reports;
`execution-receipt.json` records commands, executable hash and logs.
