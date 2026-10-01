# Legacy physical Gem corruption flags

This reviewed data-only extension appends the missing physical `corrupted`
Boolean inputs for Twister (`30b0`) and Skeletal Sniper (`30b1`). The existing
Count corruption-delta inputs (`30a9`, `30aa`) remain separate and unchanged.
It starts from the exact physical-quality-kind successor input
`e4254877a62e2f62c24bb4ee03ca5ca69040890f19c89b0b96ab74153251c23b`.

`extension.json` uses the existing `OwnedRecipeExtension` contract. It appends two
required Boolean Gem-parameter slots and adds them to the two existing Partial
parameter inventories. It does not close either inventory or change programs,
tables, effect receivers, queries, or any other schema facet. The 603 preceding
quality-kind corrections remain present, so all 604 Known physical Gem definitions
retain their Complete ordinary-quality kind inventory.

`authoring.json` records the exact predecessor descriptors and normalization rows,
the two explicit replacement rows, and pinned source evidence. The replacements
remove only the old neutral `corrupted` guards and reuse the existing Cleric
Boolean recipe: exact `true` maps to true; exact `false` and `nil` map to false.
Missing, differently cased, malformed, or whitespace-padded tokens remain Pending.
The Count-delta recipe is preserved exactly and can succeed independently when the
Boolean is unresolved; conversely the Boolean can succeed when the delta is
unresolved. No missing token is implicitly zero or false.

The authenticated Rust source targets `owned_physical_gem_inputs`,
`owned_active_gem_inputs`, and `owned_effective_gem_inputs` establish the source
loading and preparation distinction. `LoadSkill` reads the Boolean separately
from `corruptLevel`; effective-level arithmetic uses the latter. Source predicates
and bookkeeping may read the Boolean. This extension records the raw input only:
it does not implement those predicates, post-support bookkeeping, or final
numerical Gem inputs. Its conservative missing/unknown-token behavior remains
stricter than the source loader's fallback and does not invent evidence.

Run the default compact contract tests:

```powershell
cargo test --locked -p poe-optimizer-cli --test owned_legacy_gem_flags_cli
```

The full publication test is explicitly ignored unless selected. Supply the
checked predecessor and a new output directory:

```powershell
$env:POE_OPTIMIZER_TEST_LEGACY_GEM_FLAGS_PRIOR = 'runs/owned-physical-quality-kinds-01/package'
$env:POE_OPTIMIZER_TEST_LEGACY_GEM_FLAGS_OUTPUT = 'runs/owned-legacy-gem-flags-01'
cargo test --locked -p poe-optimizer-cli --test owned_legacy_gem_flags_cli real_boolean_successor_preserves_release_and_normalizes_originals_and_probes -- --ignored --exact --test-threads=1
```

The test uses the actual `extend-owned-recipe`, `publish-owned-normalization`,
`assemble-owned-release`, and `normalize-owned` CLIs. After both compact
transitions, it constructs a complete endpoint preserving all predecessor
provenance and appending one explicit authoring commitment. It verifies the
allowed changes against the entire original typed input, immutable prior files,
exact library/CLI artifacts, idempotent extension replay, stale-policy rejection,
byte-identical rebuild, five original builds, and twenty controlled XML probes.
All five originals remain Pending, and the five query sets / 110 queries are
preserved. This is raw-input progress, not whole-build numerical parity.

Validated on 2026-10-01 UTC: the default target passed two tests and explicitly
ignored the real case; the separately selected real case then passed. The final
package and rebuild contain 18 identical files totaling 58,239,509 bytes, with
registry allocations ending at `30b1`. The five original normalizations retain
12 selected Twister/Sniper physical occurrences in total and remain Pending.

- Release input: `e1128e349928a1a6ee57fc07189143c1c30da65535e84b00cef4b5f3cd91efb1`.
- Definitions: `9ab94bed59a182387a226785c2b5b4a58ff543707cc467a9473a96bcb91f7b8f`.
- Registry: `35757dc079e2b661963e7e791f640676d588b6fd9515fb009f39d4b7f36dd879`.
- Six provenance entries preserve the full five-entry predecessor history.
- Complete original numerical coverage remains **0/5**.

The checked endpoint, final receipt, and validation report are under
`runs/owned-legacy-gem-flags-01`. Test logs are
`runs/legacy-gem-flags-tests.log` and `runs/legacy-gem-flags-real-tests.log`.
The release remains format V1 with schema V4 / operations V13 and no evaluation
group. This checkpoint introduces no native evaluator or compiler behavior.
