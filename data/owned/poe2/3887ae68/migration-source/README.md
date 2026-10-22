# Tracked rule source and remaining metadata bootstrap

`release.kdl` is the sole production source manifest for the retained release.
It lists every input explicitly, relative to this directory. Build into a fresh
output directory with:

```powershell
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release build data/owned/poe2/3887ae68/migration-source/release.kdl --output rebuilt-release
```

Rule programs live in the declared `.rule` files. KDL packets bind each program
to its exact definition or declared slot and preserve membership order. The
packet-local KDL name files bind semantic aliases to checked typed addresses;
slot bindings include their declaration owner. Reusing a complete source file
does not merge its occurrences or their recipients. Identical program IDs alone
never justify sharing a body.

The compressed `bootstrap/rules.json.gz` contains only rule metadata: owner and
coverage inventories, tables, receivers, queries, applications and bindings.
It has **no program members**. The builder rejects embedded programs, including
when no packets are declared. A DSL error cannot select another execution path.
The remaining compressed JSON constituents are temporary R1/R2 metadata inputs,
not the final compact runtime pack. No local predecessor, historical staging
helper, migration generator, PoB checkout or network operation is needed to
build this tracked source.

## Independent migration evidence

`reference-release.json` remains the immutable historical receipt. The original
rule IR has moved to
`crates/poe-optimizer-authoring/tests/fixtures/retained-rules.json.gz`; it is used
only by regression tests and is not in the production source manifest.

The corpus gate checks every original owner/program occurrence against the
actual tracked aliases, packets and DSL. Its independent comparator permits
node renaming, retains read/operand/effect semantics and order, checks exact
owner membership, and compiles the complete reconstructed rule package with the
existing Engine. Non-program metadata and all incomplete-coverage declarations
must remain unchanged. The release test restores only historical program members
and requires every historical artifact length/hash and receipt byte to match.
The actual DSL release has a new rule/input identity because its node names differ.
All unrelated emitted constituents retain their historical bytes.

Historical reference identity (not the current DSL input):

- Input: `a75ef471b57d8184b208ca5f2a26ce6a9a01d15340db8b15fc1f318be40c1905`.
- Rule bytes: 17,488,370; SHA-256 `6ebd514b3c9e57a41a025426d9bf3c80f1fc64749fa665c9e8f82ebcf41b1366`.
- Receipt SHA-256: `46f233e58f7449f789a93e0b9d589892b4883fe40dbff672954aa6af22faa538`.
- Upstream revision: `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`.

## Preserved scope and limitations

The release retains 13,264 active allocations, 9,391 schema definitions and
3,873 slots. Rules retain 6,550 owners, 8,212 program occurrences, 18 integer
tables, 59 receivers, 114 contribution queries, one effect application and one
existing-Actor binding. The five named query sets retain their original order
and 22 queries each. The 227 provenance rows are unchanged metadata, not
instructions to replay historical migrations.

There is no evaluation bundle. The derived manifest still reports 5,827 partial
rule owners and five partial routing outputs. This representation migration
adds no whole-build coverage or numerical parity, retires no mechanic gap and
does not make an incomplete build executable. The first complete unchanged
build remains gated on the remaining data/runtime migration and actual blockers.

The two `contribute-inherent-strength-life` programs remain distinct: shared
Player Actor `332a` uses the seven-node checked-domain form, while supplied
Actor slot `001f` declared by Skill `0012` uses the six-node form. The Player
range is inclusive [-137438953472, 137438953472] in Life units, behind its lazy
enabled guard. Both owners remain Partial; Life Stat `311a` retains
`unproved-life-conversion-order-domain`.

The names and source files are maintained authoring inputs. Their initial
conversion used the final mapping, current programs and reviewed source-field
evidence; historical family files can reuse addresses and are not authoritative
by themselves. No test-generated sequential aliases or arbitrary array offsets
serve as production names.

## Checked source inventory

Current input: `d5f7086228a1fe2bfbc7fcdde033fd62741e719aa3af188fc5f3686b33b02406`.
The 103 packets reference 2,561 distinct complete rule sources, preserving all
8,212 occurrences. Their local name files use 5,011 distinct exact addresses.
The 2,784 declared inputs occupy 5,772,456 encoded and 58,564,599 decoded bytes;
expanded rule text is 4,572,462 bytes and the root manifest is 183,380 bytes.
All fit the existing default limits. Authoring's 179 tests pass, including the
actual-source graph and historical artifact gates; none are ignored.
