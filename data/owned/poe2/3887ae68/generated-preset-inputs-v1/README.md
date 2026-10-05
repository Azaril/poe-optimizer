# Saved generated raw-quality inputs

This packet gives the two allocated Djinn supplies and the item Firebolt supply
explicit permission to accept a raw quality value from the selected skill preset.
It uses the existing generated Skill identity and shared parameter graph. It does
not create a physical Gem or a second SkillUse for a saved generated group.

The predecessor is `runs/owned-command-cooldown-01/package`, input
`e77ecaa55284d1dd888f3ad65145e735269fe589fc387076cbd711a8eb114244`.
The checked migration moves schema V5 / operations V18 to schema V6 / operations
V19 through migration V4. Publication and all three native component tests pass;
the implementation plan records their receipts and remaining coverage gates.

## Exact data changes

| Source | Supplying declaration | Supplied Skill | Raw quality |
| --- | --- | --- | --- |
| Tree Sand Djinn, node 13289 | Passive `0b34`, supply `32d2` | `0322` | existing `3262` |
| Tree Water Djinn, node 32705 | Passive `10d4`, supply `32d4` | `032c` | existing `3264` |
| Item Firebolt | Modifier `31c6`, supply `31c9` | `0134` | new `32ef` |

The migration changes only the three supplies' `preset_inputs` fields, appends
`32ef` to Firebolt's existing Partial parameter inventory, and declares that new
slot. Each permission is a Complete allowlist containing only its quality slot.
It does not close the supplying declaration's outputs or any owner's rules.

Firebolt quality is a RequiredOnce, projected-only Quantity in percentage-point
unit `0002`, with the finite f64 transport range. Existing Djinn quality slots
remain byte-for-byte unchanged, including their Direct authored input authority.
This raw transport domain is not an in-game edit domain or permission for an
optimizer to vary quality arbitrarily.

Existing raw level producers and slots `3261`, `3263`, and `31ca` are unchanged.
No program, table, receiver, query, reducer, damage formula or descendant input
projection is added. A root's quality does not implicitly become the quality of
its Command or minion children. Dependencies retain exact predecessor descriptors
and provider programs so publication and component tests can verify preservation.

## Import correspondence

`generated-inputs.json` is a template bound to the exact predecessor definition,
role, source and catalog identities. Publication must explicitly rebind its
definition and role identities to the migrated endpoint. The four rows describe
two Tree providers and two independently reviewed Firebolt item-name frames:
`Sol Pole, Ashen Staff` and `New Item, Ashen Staff`. Item IDs are resolved from the
actual source project; they are not hard-coded into the policy.

PoB preserves a saved generated source object only when its source string, slot,
first Gem's skill ID and level match the grant. The pinned source accepts either
the raw grant level or its normalized level. This first importer deliberately
admits the smaller exact-integer/raw-level-equality domain: Tree level 1 or the
already imported Firebolt modifier's level roll. Normalized-only matches remain
Pending. Level is an identity guard and is never copied into a new input binding.

Item source names are checked against the admitted physical Item, template and
three exact attributed header lines: rarity, title and base. This is a finite
reviewed name frame, not a new general Item parser. The source slot must match
the actual provider's slot, including a swap suffix. Tree sources require an
absent slot. A missing provider, ambiguous duplicate, stale item name, unsupported
frame, malformed value or unsupported preset correspondence cannot supply a
resolved quality record.

Quality uses the ordinary exact scientific Quantity recipe, scale 1/1, duplicate
rejection and MissingPending. The source loader calls `tonumber` without a quality
fallback. A newly reconstructed PoB source receives quality 0 only when no saved
source matched; that creation behavior is not an importer default. The first
normalization slice resolves the independently selected source axes. Archived
cross-preset provider correspondence remains Pending unless separately proved.
Existing gameplay usage obligations survive conversion to the preset envelope.

Original04 is a deliberate unresolved case in this publication: its Firebolt
grant line is still `source_meaning_unresolved`, and its Item layout has rune,
header and member obligations. The reviewed name row does not establish an owned
granting modifier. The importer must retain a generated-input obligation until
those independent provider facts are proved. Original01 and Original05 provide
the currently resolvable tree/item occurrences.

Firebolt's catalog Gem is still a physical SkillUse. Its item-generated ownership
comes from the exact ItemModifier provider join, not from changing that catalog
role. Manual Djinn and physical Firebolt inputs remain independent.

## Source evidence and limits

The complete-source witness is
`generated_skill_usage::complete_generated_skill_usage_preserves_source_consumers`
in `crates/poe-optimizer-pob/tests/owned_authored_skill_membership_source.rs` and its
shared `support/generated_skill_usage.rs` module. It passes 46 cases in each JIT
mode with independent fresh, rebuilt-once and rebuilt-twice observations. Reports:

- `runs/owned-generated-skill-usage-source-01/source-jit-off.json`
- `runs/owned-generated-skill-usage-source-01/source-jit-on.json`

Each report is **64,295,778 bytes**, SHA256
`75ca2ba05fdecc465d4727f43b2a31358ffcd2fb134a80281534bc4d24a867d5`.
The reports are byte-identical. Source revision is
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`; manifest SHA256 is
`8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675`.
The witness preserves all five original XML files and activates archived presets
through the original complete loader. It does not choose a canonical lifecycle.

The five originals contain fourteen saved rows in this reviewed family: two
Tree rows in Original01, one Firebolt row in Original04, and eleven rows across
Original05's six presets. All save quality 0. Independent Original05 mutations
save quality 12.5 on each of the three selected generated sources. Exact source
object joins and prepared quality preserve that changed source's value in both
MAIN and CALCS at every observed stage, while other sources remain independent.
These controls are not evidence that arbitrary quality edits are legal in game.

Raw and prepared values differ in real builds. Original01's generated Djinn raw
level 1 prepares as level 12; Original05's prepares as level 3. Original04's
Firebolt raw level 17 / quality 0 prepares as level 26 / quality 21. This packet
imports raw values only and does not implement those downstream adjustments.

`source-vectors.json` contains bounded exact JSON-pointer observations from the
authenticated reports. `source-facts.json` records exact pinned loader, grant-join,
normalization and Item-name excerpts, plus the original saved census. Ordinary
authoring checks can use these checked-in facts and the tracked manifest; report
and source-checkout authentication belongs to the optional publication gate.
No large report is checked in or used as a native runtime input.

To reproduce the source witness, leave `POE_GENERATED_SKILL_USAGE_SOURCE_CHILD`
unset and run the ignored test explicitly:

```text
cargo test -p poe-optimizer-pob --test owned_authored_skill_membership_source generated_skill_usage::complete_generated_skill_usage_preserves_source_consumers -- --ignored --exact --nocapture --test-threads=1
```

This evidence does not authorize an empty usage inventory, supported/final quality
mechanics, descendant fanout, complete owner programs, an evaluation bundle or
whole-build native parity. Those boundaries remain independently checked.

## Publication and native replay

The ordinary Rust authoring test authenticates the checked-in packet and tracked
source manifest. Full publication additionally requires the exact predecessor,
the pinned PoB checkout and both large source reports above. Set
`POE_OPTIMIZER_TEST_GENERATED_INPUT_PRIOR` to that predecessor package and
`POE_OPTIMIZER_TEST_GENERATED_INPUT_OUTPUT` to a new output directory, then run:

```text
cargo test -p poe-optimizer-cli --no-default-features --test owned_generated_skill_inputs_cli generated_input_publication --locked -- --ignored --nocapture --test-threads=1
cargo test -p poe-optimizer-cli --no-default-features --test owned_generated_skill_inputs_native --locked -- --ignored --nocapture --test-threads=1
```

The second command reads the first command's package through the same output
variable. Its finite native component preserves the actual supply permissions,
raw input schemas and level-projection programs, while explicitly supplying
unrelated owner/topology closure. It checks raw quality transport and independent
provider identity, not final quality or whole-build mechanics. These optional
replays are not run by ordinary CI; the implementation plan records their latest
explicit result and exact publication identity.

Checked publication: `runs/owned-generated-preset-inputs-05/validation.json`;
native log: `runs/owned-generated-input-native-03.log`. The package has eighteen
files totaling 60,809,799 bytes, with 89 provenance rows and all 110 prior queries.
The immutable authoring record describes the publication request; these replay
receipts record its checked outcome.
