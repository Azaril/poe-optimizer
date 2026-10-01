# Physical Gem quality kinds

This prior-bound schema revision closes only the ordinary-quality kind inventory
for 603 already-Known physical Gem definitions. All 604 Known physical Gems then
have exactly ordinary quality `def.0000000000000006`; the already-Complete Sniper
row and all 362 Unmapped Gem definitions remain unchanged. No identities are
allocated. Quality amount and presence, other declarations, rule coverage,
support activation, and alternate-quality calculations remain unchanged.

The source evidence is the optional Rust target
`crates/poe-optimizer-pob/tests/owned_physical_quality_inputs.rs`. It authenticates
the pinned source and observes all 966 physical Gems in nine loading and
reprocessing cases each, in both JIT modes. Physical quality is a numeric scalar;
the computed Gemling alternate-quality flag adds effects using the same scalar,
without introducing a second physical quality kind. Missing or malformed amounts
are not proven zero. The normalizer's explicit kind-token policy is preserved:
an unknown `qualityId` still remains Pending even though the source loader ignores
that attribute.

`revision.json` is an existing `OwnedReleaseRevisionInput`, bound to predecessor
input `90208367c88c36cf2e20fad88573430bc7bfc13c56f9002c2570ab9eaf133ced`.
It contains full replacements for exactly 603 existing Gem descriptors, with only
`quality.allowed_kinds.closure` changed from Partial to Complete. `scope.json`
records the exact source catalog join, unchanged sets, former closures, and former
descriptor SHA-256 values. The descriptor hashes use the compact typed Rust
`serde_json::to_vec(DefinitionDescriptor)` representation. The default Rust test
restores each former closure and verifies that hash, including the exact finite
quantity bounds and every unrelated facet. The real test independently rejoins
the tracked catalog and actual predecessor mapping/schema before publication.

Reproduce the default data validation:

```powershell
cargo test --locked -p poe-optimizer-cli --test owned_physical_gem_quality_cli
```

The real publication case is explicitly ignored unless selected. It requires the
checked predecessor produced by the raw-Gem-input checkpoint and a new output
directory:

```powershell
$env:POE_OPTIMIZER_TEST_PHYSICAL_GEM_QUALITY_PRIOR = 'runs/owned-effective-gem-raw-inputs-02/package'
$env:POE_OPTIMIZER_TEST_PHYSICAL_GEM_QUALITY_OUTPUT = 'runs/owned-physical-quality-kinds-01'
cargo test --locked -p poe-optimizer-cli --test owned_physical_gem_quality_cli real_quality_revision_preserves_release_and_pending_input_semantics -- --ignored --exact --test-threads=1
```

This uses the existing checked `assemble-owned-release --revision` CLI and
unchanged resource limits, then rebuilds the output. It asserts facet-only
changes and exact dependent rebinding through a restored full-input comparison,
preserves the entire predecessor inventory and all five query files byte for
byte, freshly normalizes all five original builds, and probes missing, literal
nil, malformed, unknown-kind, fractional, and zero quality inputs. Every original
remains Pending. Revision authoring adds one provenance entry without discarding
the prior four.

Validated on 2026-10-01 UTC: default target 1 passed and 1 explicitly ignored;
explicit real case 1 passed. The new package and its rebuild contain 18 identical
files totaling 58,236,027 bytes. Receipt and logs are under
`runs/owned-physical-quality-kinds-01`,
`runs/physical-gem-quality-cli-tests.log`, and
`runs/physical-gem-quality-real-publication-tests.log`.

- Release input: `e4254877a62e2f62c24bb4ee03ca5ca69040890f19c89b0b96ab74153251c23b`.
- Schema: `8c21f416d6be52b65ae762c8f29824aa3f6bf5e01c306725196d2485c82fe2e5`.
- Registry unchanged: `17c99a5f4d5d4fb84c71b7f069929021c50ee1a21ba0324e1fdee3140a9bf666`.
- Five query sets / 110 rows remain unchanged; complete original numerical coverage remains **0/5**.

This is input-schema completeness for one finite facet. It does not establish
complete game rules, valid support delivery, final effective Gem inputs, or
whole-build numerical parity.
