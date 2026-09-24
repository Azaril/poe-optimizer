# Reviewed owned data release corrections

`open-gem-inputs-v2.json` corrects the original Gem definitions `def.000000000000000a`
and `def.0000000000000011` (Twister and Skeletal Sniper). Their direct parameter and choice
collections were prematurely declared Complete-empty. The corrected data preserves every
member and marks those four collections Partial with an explicit input-schema gap. Level,
quality, physical role, potential Skills, grants and unresolved source-command references
are unchanged.

The policy binds the full canonical input of the validated multi-effect support package,
not just its schema. It names the new release `pob-3887ae68-open-gem-inputs-v2`. Registry
identities and watermark stay unchanged. Ordinary schema refinements cannot apply this
correction; their Complete/Known preservation rules remain correct and unchanged.

## Reproduce

Use the source-free Rust CLI with new output directories:

```text
poe-optimizer assemble-owned-release runs/owned-multieffect-support-inputs-01/package --output runs/owned-data-release-01/baseline
poe-optimizer assemble-owned-release runs/owned-data-release-01/baseline --revision data/owned/poe2/3887ae68/releases/open-gem-inputs-v2.json --output runs/owned-data-release-01/package
poe-optimizer assemble-owned-release runs/owned-data-release-01/package --output runs/owned-data-release-01/reproduced
```

The first command validates the previous full package and creates its canonical release
representation. The second applies the explicit data correction, validates all newly bound
artifacts, and records the prior/revision commitments as authoring provenance. The third
must reproduce every artifact byte. The existing CLI integration pipeline rebuilds the
predecessor from tracked inputs and exercises this correction with all five originals.
It is not necessary to execute PoB or any source code to apply this policy.

The canonical predecessor input is
`deb870ee12054739397a7c346741b769a444ffd6342417b2f074da716bac0c81`.
The corrected schema identity is
`a4657155d69fe6647bc2725b3b811ed732fe37e673a98b819ed81a7f92d713bc`.
The corrected full input identity is
`82cd2fd99a751460ec7d0aad5d3bdbd82aea1cbd1c47fd75d99191b49ffed8ac`.

This corrects coverage metadata; it does not implement the missing inputs or change
numerical mechanics. Twelve previously Complete original Gem parameter collections become
Pending, while all existing scalar values, items and 110 queries remain. Original build
parity is still incomplete. Active-Gem conversion and paired action preparation follow
through the [implementation plan](../../../../../docs/implementation.md).
See the [release contract](../../../../../docs/owned-releases.md) for the generic API.
