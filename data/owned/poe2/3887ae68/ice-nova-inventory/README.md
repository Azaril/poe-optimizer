# Ice Nova physical input disposition

This packet supplies one row to the reusable V3 physical Gem inventory policy. It
targets the checked Ice Nova action release, input
`3b05c2cadd9cd68184f5c602a75afaaeb938e4319333fe62fdf3cc1e29f1c184`.
Publication changes normalization and its dependent release/tree commitments;
definitions, rules, mappings, all 514 support rows, the two existing primary-skill
rows and all 110 query rows must remain unchanged.

`disposition.json` retains the already converted physical corruption flag and
corruption-level delta declarations. Its reference-action correspondence is an
exact copy of the adapter bound to that release's definitions, role compilation
and constructed catalog. This source adapter interprets saved action selectors;
it does not introduce PoB MAIN/CALCS concepts into native game mechanics.

Five deferred-usage recipes classify saved fields without emitting usage values:

| Field | Source attribute | Codec | Missing |
| --- | --- | --- | --- |
| Gem count | `count` | finite quantity, Count unit | Pending |
| Gem global 1 | `enableGlobal1` | explicit `true` / `false` | Pending |
| Gem global 2 | `enableGlobal2` | explicit `true` / `false` | Pending |
| Group count | `groupCount` | finite quantity, Count unit | Absent |
| Group full DPS | `includeInFullDPS` | `true` / `false` / `nil` (false) | Absent |

Every recipe uses exact whitespace and rejects duplicate attribute values.
Count recipes accept finite numeric syntax, with identity scale and no aliases;
they do not infer a default count, effective multiplicity, reservation or DPS.
The full-DPS `nil` token is explicitly represented in its Boolean codec.
Successful field disposition requires a real containing preset with Pending
usage inventory. No UsagePolicy definition, preference, native parameter,
numeric rule or fabricated completeness claim is added.

The V3 successor preserves V1/V2 behavior and retains known physical values when
disposition cannot be proven. Its whole-source and reference-action checks must
reject unreviewed fields or malformed maps rather than silently discard them.
The original five-build publication regression passed. It retires four Ice Nova
physical-list obligations across Original05's saved presets, including one
selected obligation (19 to 18). Local IDs, allocation watermarks, all old values
and usage obligations are preserved; only exact Gem/group provenance links are
added for existing Pending usage destinations. The other originals are unchanged.
All 110 query rows remain byte-identical, and the eighteen package files rebuild
exactly. Only normalization, its dependent tree commitment and the release receipt
change. No schema, rule, table or mapping is added.

The full-source witness passed in both JIT modes: 23 cases in each mode, with
fresh load and two requested original-frame rebuilds per case. The two complete
reports are byte-identical (5,226,330 bytes each); the authoring receipt pins
their hashes, all 12 observed source files and this packet's exact LF bytes.
It includes unchanged originals, independent globals, count and group-count
controls, FullDPS inclusion, disabled sources, separate MAIN/CALCS stat sets,
duplicate occurrences, archived-only edits and restoration. Count affects
reporting, and the source count lookup can share the first same-effect copy's
count; physical field disposition does not claim those semantics are computed.
Native Ice Nova mechanics remain Partial; complete native originals remain 0/5.

The successful local checkpoint is `runs/owned-ice-nova-inventory-02/`, input
`4b1823e1ac6fa26f9115764ebb1b77b2066aec0bf778e8e19919fb65713e07b0`.
Its eight mutation controls compare predecessor and successor on the same edited
XML. Unreviewed source shapes can withhold pre-existing numeric usage projections;
this packet retains that conservative behavior. The tests verify that V3 itself
does not manufacture or change usage values.

To reproduce, first run the `owned_ice_nova_occurrence_inputs` ignored PoB test
with `POE_ICE_NOVA_OCCURRENCE_SOURCE_CHILD` unset, then use the exact predecessor:

```powershell
$env:POE_OPTIMIZER_TEST_ICE_INVENTORY_PRIOR = 'C:\code\poe-optimizer\runs\owned-ice-nova-actions-04\package'
$env:POE_OPTIMIZER_TEST_ICE_INVENTORY_OUTPUT = 'C:\code\poe-optimizer\runs\owned-ice-nova-inventory-repeat'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_OPT_LEVEL = '2'
$env:CARGO_PROFILE_TEST_DEBUG_ASSERTIONS = 'true'
$env:CARGO_PROFILE_TEST_OVERFLOW_CHECKS = 'true'
$env:CARGO_INCREMENTAL = '0'
cargo test -p poe-optimizer-cli --test owned_ice_nova_inventory_cli --locked -- --include-ignored
```

Choose an output directory that does not exist. The default packet test does not
need local source reports; publication authenticates both complete reports and
all five original source hashes before staging anything.
