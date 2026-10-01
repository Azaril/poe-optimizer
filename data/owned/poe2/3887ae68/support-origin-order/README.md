# Saved manual-group support order

This data-only publication enables the existing `SavedManualGroupOrder`
normalization policy. It addresses an actual missing input in original02's
selected Twister setup: five physical support assignments were retained, but the
normalization policy omitted their saved source order.

The selected source skill set is 6, group 8. Its local order is Retreat II,
Elemental Armament II, Projectile Acceleration III, Salvo, then Prolonged Duration
II. These names describe the test witness, not runtime dispatch. The shared
normalizer records exact authored targets and physical assignment identities in
source encounter order. Duplicate definitions and disabled assignments remain
separate; their IDs never select or sort the winners.

`policy.json` contains the existing typed policy. `authoring.json` binds the exact
twelve-family predecessor and its normalization digest. Publication changes only
the optional policy field, dependent tree commitment, and one provenance entry.
No schema, rules, registry allocation, original query, or evaluator API changes.

Local order is not full support discovery. Every outer origin inventory retains
`support-origin-discovery-not-converted` and remains Pending. Ambiguous, generated,
item-supplied, or otherwise unproven targets gain no guessed authored sequence.
Original05's selected Sniper group contains no physical supports; the policy does
not infer a complete empty support inventory. The next blocker remains proving
complete per-target discovery and other actual selected-input dependencies.

Run the compact policy regression:

```powershell
cargo test --locked -p poe-optimizer-cli --test owned_support_origin_order_cli
```

The full real-release test is explicitly ignored unless selected and needs a new
output directory:

```powershell
$env:POE_OPTIMIZER_TEST_SUPPORT_ORDER_PRIOR = 'runs/owned-statset-gem-inputs-01/package'
$env:POE_OPTIMIZER_TEST_SUPPORT_ORDER_OUTPUT = 'runs/owned-support-origin-order-02'
cargo test --locked -p poe-optimizer-cli --test owned_support_origin_order_cli real_publication_retains_exact_twister_order_and_pending_unknown_origins -- --ignored --exact --test-threads=1
```

It uses shared checked release fixtures and the actual publication, assembly and
normalization CLIs. It preserves the full predecessor provenance, verifies an
exact typed-input change allowlist, byte-identical rebuild and immutable prior
artifacts, then rejoins every support assignment to XML source evidence across
all five originals. Controlled copies of original02 cover disabled duplicate
supports, an ambiguous two-active-skill group, and an item-generated group. The
user's original files are unchanged. No calculation or numerical parity is claimed.

Validated on 2026-10-01 UTC: default target 1 passed and 1 explicitly ignored;
explicit real case 1 passed. All 338 physical support assignments remain present,
273 are retained in proven local source sequences, and all 110 query rows remain
unchanged. The three controlled probes also passed. The actual policy's empty
generated-support-prefix inventory is preserved: an item-generated group retains
its source evidence without inventing physical assignments or authored targets.

- Final input: `86dd5ac81ac894d6674121cbf22ebc726d11bbe23376da7e6904103828648f5a`.
- Definitions unchanged: `cc4ebdffaead1b2aa58802b3a5ade812ceb58d39d8134aaf5939e4a651faa054`.
- Registry unchanged: `f98c0b22c2d6f1bcf43a790937ac8398c9df1e822e10ce1ee0dc302d937f4437`.
- Eight provenance entries preserve all seven predecessor entries.
- Package/rebuild: 18 identical files, 58,323,208 bytes.

The final checked release and per-original order receipts are under
`runs/owned-support-origin-order-02`. Logs are
`runs/support-origin-order-cli-tests.log` and
`runs/support-origin-order-real-tests.log`. The first failed test-only generated
probe is retained separately in run01 and its failure log. All original builds
still remain Pending and complete numerical coverage remains **0/5**. This
checkpoint fills known local ordering; it does not close full support discovery.
