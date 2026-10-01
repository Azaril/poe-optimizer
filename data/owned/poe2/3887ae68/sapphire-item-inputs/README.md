# Sapphire physical item inputs

This checked successor starts from the exact Solar release in `authoring.json`.
It first installs the separately authored cold-category prerequisite, then
corrects the Sapphire Ring (`09dc`) parameter declaration through the existing
checked release-revision contract. The historical Complete collection contained
only catalyst kind and amount; `correction.json` preserves those two members and
records the omitted physical-input inventory as Partial. The ordinary append
contract is unchanged.

The subsequent extension allocates four template-owned slots: rarity `316e`,
fresh corruption state `316f`, raw `LevelReq` `3170`, and socket capacity `3171`.
Raw `LevelReq` is OptionalOnce in the schema, while this source profile requires
an explicit valid header. Existing catalyst slots `09f9`/`09fa`, their None/20
defaults and transport program remain exactly unchanged. Existing item-level
and raw-quality absence defaults also remain unchanged. No owner programs,
numerical operations, final resistance contributions or metric bindings are
added. Static template input and owner coverage remain Partial.

The existing V2 physical-member proof gains one reviewed paired template and
the existing `ranged-plus-cold` rule. It requires exactly one complete implicit
and one complete explicit modifier, with full source-layout proof. The original
Sapphire item has the implicit cold-resistance range followed by explicit Life;
its cold amount remains the unrounded interpolated 25 percentage points and
its Life amount remains 10. Source property tags remain source inputs. The
English text does not supply additional tags.

`crates/poe-optimizer-pob/tests/owned_sapphire_item_inputs.rs` is the authenticated
complete-source witness. Its pinned files are recorded in `authoring.json`.
The final source run passed six complete loads and 44 fresh controls per JIT
mode, with identical JIT-on and JIT-off evidence.
Physical input proof distinguishes absent headers from explicit zero, and
retains the existing exclusions for malformed or duplicate headers, occupied
augments, unsupported flags, variants and incomplete layout/category evidence.
An explicit valid empty Sockets/Rune sequence has line-backed capacity evidence;
an absent header uses the reviewed base-construction proof instead.

`tests/owned_sapphire_item_inputs.rs` checks typed assets, complete predecessor
restoration, stale commitment rejection, byte-identical publication/rebuild,
all five original source imports and selections, and all 110 queries byte-for-byte.
The comparison follows the one physical ring through eight equipment uses,
including both selected ring slots. It verifies four new raw assignments and
the exact cold category while retaining the existing 47 modifier rolls.

Cold correction has explicit collateral: the predecessor contains 18 canonical
cold occurrences. Three have Proven layouts and receive a category; 15 have
Pending layouts and must lose canonical emission while retaining their raw
source and candidate diagnostics. Eight withdrawals are selected in originals
01–04. Those removed roll issues are reported separately from the four expected
Sapphire input/order/membership/roll completions in original05. A lower issue
count caused by withdrawal is not a completed input or numerical result.

The final publication run passed both tests and all 34 probes. Selected issue
counts became 127/128/120/158/34; only the last build's four retirements are the
positive Sapphire closures. The three retained cold occurrences became
Complete24, and the 15 withdrawals match the reviewed full-source census.
The immutable package rebuilt byte-for-byte, all 110 queries and saved
selections remained unchanged, and the prior endpoint was fully restored by
the preservation checks. The corrected ring retains its new static
`schema_partial` sidecar diagnostic independently of the proven physical input
inventory. All five original builds remain Pending and native calculation
remains `not_run`.

The real publication lane uses a fresh output directory:

```powershell
$env:POE_OPTIMIZER_TEST_SAPPHIRE_INPUTS_PRIOR = 'runs/owned-solar-item-inputs-01/package'
$env:POE_OPTIMIZER_TEST_SAPPHIRE_INPUTS_OUTPUT = 'runs/owned-sapphire-item-inputs-repro'
cargo test -p poe-optimizer-cli --locked --test owned_sapphire_item_inputs -- --include-ignored --nocapture
$env:POE_OPTIMIZER_TEST_SAPPHIRE_NATIVE_OUTPUT = $env:POE_OPTIMIZER_TEST_SAPPHIRE_INPUTS_OUTPUT
cargo test -p poe-optimizer-cli --locked --test owned_sapphire_native -- --include-ignored --nocapture
```

The output includes the full explicit package, rebuilt package, prior/new drafts
and sidecars, selected reports and a compact validation receipt. Neither the
authoring sequence nor source interpreter runs in native evaluation. The
separate `owned_sapphire_native` target checks actual endpoint numeric programs
and occurrence identity; it does not claim a final cold-resistance metric or a
complete original build.

The verified checkpoint is `runs/owned-sapphire-item-inputs-03`: publication
2/2 with 34 probes, native components 3/3, and 20 shared native regressions pass.
The native fixture retains both actual ring programs, including the existing
false base-attack-profile capability, and all four cold programs. Its complete
topology is a separate finite test domain; actual static owner gaps stay open.
