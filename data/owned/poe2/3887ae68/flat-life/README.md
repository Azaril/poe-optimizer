# Canonical flat Life inputs

This authoring replaces the `fixed-life` line rule's legacy integer emission with an owned modifier containing explicit numeric and eligibility inputs. It uses existing checked schema extension, template membership refinement, item-line/source policies, numeric compilation and full release assembly. No production parser or evaluator dispatch is added.

The exact predecessor is `runs/owned-movement-speed-release-02/package`, input `db7cb3798f8aeb87fed0d780e4fa658a0bb35d7568ee79a25f1a671eaa66da08`. `prior-rule.json` and `prior-condition.json` freeze the replaced definitions. Publication asserts both original rows before replacement, preserves their positions and source role, and retains the old `09da`/`09db` definitions, owner, memberships and programs.

## Authored data

The registry appends 27 definitions, `3100` through `311a`:

- Modifier `3100` has 24 required inputs: raw amount `3101`, 20 source property flags `3102`–`3115`, unscalable `3116`, initial corruption factor `3117`, and category `3118`.
- Category uses the existing finite Explicit `30e2`, Implicit `30e3` and Enchant `30e4` options. Only a proven complete source layout supplies this input.
- Raw and effective amounts use Count unit `295a` and modifier stat `295b`. The existing numeric compiler uses precision 1, display precision 0 and direct sign.
- The authored contribution divides the effective Count by one Count, then scales one Life point by that dimensionless ratio. Life-points unit `3119` is distinct from Count. Player/Add stat `311a` receives the result once per receiving equipment use.

The input inventory is Complete for this bounded family. Owner programs, template modifier membership, ordered magnitude and contributor/routing coverage retain their Partial declarations. The new channel does not provide a final Life or effective-hit-points result.

`bindings.json` lists 22 templates from actual original source occurrences. Their existing memberships and closures remain intact. The fixed grammar requires a literal `+`, then an unsigned integer from 0 through 1,000,000, then ` to maximum Life`. Its source predicates require no source scaling tags, initial scaling equal to one and no generated buff members. It does not infer a Life property flag from English text or the affix catalogue.

## Source evidence and real admission

`authoring.json` binds the pinned source revision, authenticated module hashes and Rust source witness `crates/poe-optimizer-pob/tests/owned_flat_life.rs`. That witness runs complete source loading and calculation functions with JIT off and on: all 24 plain original Life lines, 14 complete build loads and 23 controls per JIT mode. Results agree across modes.

All 24 actual lines have empty parsed tags and zero flags; 23 are Explicit and one is Implicit. The saved catalogue's `resource`/`life` tags are separate evidence and are not substituted for parsed modifier tags. Identity-scaled `+0` produces an active zero-valued Life BASE record. Bare `17 to maximum Life` leaves unparsed text and is excluded. Ranges, fractional values, source scaling tags and corrupted ranges remain outside this publication's admission domain.

The real successor admits exactly five canonical physical occurrences:

| Original | Source / line | Raw Life | Change |
| --- | --- | ---: | --- |
| 02 | 501 / 19 | 69 | New canonical modifier on an inactive saved item |
| 02 | 506 / 21 | 25 | New canonical modifier on an inactive saved item |
| 05 | 572 / 20 | 17 | Selected Tattered Robe |
| 05 | 574 / 20 | 16 | Selected Rope Cuffs |
| 05 | 587 / 14 | 10 | Replaces the legacy Sapphire Ring modifier |

The Sapphire Ring remains one physical item used by both saved ring slots; source delivery contains two records of 10. Exact canonical correspondence preserves equipment-use multiplicity. The other 19 plain Life candidates remain Pending because their source context is not proven. Existing item modifier inventory and order gaps remain visible even for the admitted occurrences.

## Reproduction and preservation

The default test validates the finite authored contract. The real test is explicitly ignored unless its checked predecessor is supplied:

```powershell
$env:CARGO_PROFILE_TEST_OPT_LEVEL = '2'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$env:CARGO_PROFILE_TEST_DEBUG_ASSERTIONS = 'true'
$env:CARGO_PROFILE_TEST_OVERFLOW_CHECKS = 'true'
cargo test --locked -p poe-optimizer-cli --no-default-features --test owned_flat_life_cli
$env:POE_OPTIMIZER_TEST_FLAT_LIFE_PRIOR = 'runs/owned-movement-speed-release-02/package'
$env:POE_OPTIMIZER_TEST_FLAT_LIFE_OUTPUT = 'runs/owned-flat-life-release-01' # must not exist
cargo test --locked -p poe-optimizer-cli --no-default-features --test owned_flat_life_cli -- --ignored --exact real_life_family_preserves_originals_and_replaces_only_reviewed_occurrences
```

The test assembles a full endpoint, publishes and rebuilds through the existing CLI, and verifies byte-identical package inventories. All five originals are normalized afresh; all 110 queries remain byte-for-byte identical. Exact source joins establish an injective identity correspondence for retained canonical occurrences and source links, including issue links. Added modifiers can shift allocator values, so unrelated identities are verified through that correspondence rather than discarded. Saved request selections, item defaults, remaining issue content and relationships are unchanged.

Eleven probes cover zero, bare/fractional/negative text, parsed tags, desecration, corruption, ranges, unknown source prefixes, and explicit Implicit/Enchant category context. The shared preservation helper's existing minion and movement publications also pass their default and real regression tests.

The checked result at `runs/owned-flat-life-release-01/package` has input `fc9fe72366ebf991c0eca8fd40d3c3e857e6e412f0414a674d334e752d24c2c4`, schema content `e694c68892c775d6e49994eedfa2568a11f4534d625b74ed06945058b5a8feb5`, and registry `8c824cfabd134398f5ede961bc2738eb9ae00418872dad95601e9330f575b4a0`. It contains 18 files, 58,568,868 bytes and 16 provenance entries. `validation.json` and `execution-receipt.json` record commands, binary hash, census and selected reports. Selected issue counts remain 317/322/314/380/148; complete numerical coverage remains 0/5.
