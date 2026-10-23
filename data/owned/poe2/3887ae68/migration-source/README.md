# Tracked rule source and remaining metadata bootstrap

`release.kdl` is the sole production source manifest for the retained release.
It lists every input explicitly, relative to this directory. First reproduce the
allocation predecessor using the [canonical reference](../authoring-genesis.md),
then build into a fresh output directory:

```powershell
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release build data/owned/poe2/3887ae68/migration-source/release.kdl --predecessor genesis-checkpoint --expected-predecessor e89ae92ac3895a3e9f38591452d340b8c640d4af737a544cc18759de4d98b109 --output rebuilt-release
```

Bootstrap reproduces the first 69-name allocation genesis only at its fixed
source revision. Later source builds use that checked checkpoint with
`--predecessor` and `--expected-predecessor`; do not bootstrap evolving sources.
The output contains `checkpoint.json`, `names.json` and `runtime/`. Runtime
consumers receive `rebuilt-release/runtime`. The [canonical reference](../authoring-genesis.md)
records the source revision, checkpoint digest and clean-clone commands outside
this compiled inventory; local `runs/` outputs are only verification evidence.

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
not the final compact runtime pack. Reproducing the fixed genesis needs no local
predecessor, historical staging helper, migration generator or PoB checkout.

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
must remain unchanged except for the explicitly admitted 69 Stat declarations,
one Sniper initial-type program and 349 intrinsic support-domain declarations.
The release test removes exactly that delta,
restores historical program members and uses checked content constructors to bind
the original dependencies. Every reconstructed historical artifact length/hash
and receipt byte must match. New schema/registry identities change dependent
artifact bindings; unchanged query payloads remain byte-exact.

Historical reference identity (not the current DSL input):

- Input: `a75ef471b57d8184b208ca5f2a26ce6a9a01d15340db8b15fc1f318be40c1905`.
- Rule bytes: 17,488,370; SHA-256 `6ebd514b3c9e57a41a025426d9bf3c80f1fc64749fa665c9e8f82ebcf41b1366`.
- Receipt SHA-256: `46f233e58f7449f789a93e0b9d589892b4883fe40dbff672954aa6af22faa538`.
- Upstream revision: `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`.

## Preserved scope and limitations

The release has 13,333 active allocations, 9,460 schema definitions and
3,873 slots. Rules have 6,550 owners, 8,213 program occurrences, 18 integer
tables, 59 receivers, 114 contribution queries, one effect application and one
existing-Actor binding. The five named query sets retain their original order
and 22 queries each. The 227 provenance rows are unchanged metadata, not
instructions to replay historical migrations.

There is no evaluation bundle. The derived manifest still reports 5,827 partial
rule owners and five partial routing outputs. The first declarations add Sniper's
initial own/minion type facts and explicit minion-context presence through generic
Skill Boolean Stats. See [evidence and limits](../initial-skill-types/README.md).
Eight existing Skill, Actor and declared-slot owners, three root/Ascendancy
owners, four UsagePolicies, 293 ordinary attribute-choice passives, eighteen
reviewed numeric minion passives, seven class-view passives, one intrinsic socket,
one Boolean-flag passive, six structurally reviewed passives and all eight Classes now have intrinsic
support-source domains,
bringing the total to 1,335 (1,306 Known, 29 Unmapped).
See [source evidence and boundaries](../sniper-support-domains/README.md).
The [attribute-choice proof](../attribute-support-domains/README.md) includes
every option and retains the current six Count contribution channels.
The [numeric minion proof](../minion-support-domains/README.md) preserves the
eighteen owners' 26 programs, including paired effects and conditional receivers.
The [class-view](../view-support-domains/README.md),
[socket](../socket-support-domains/README.md) and
[flag](../gigantic-support-domains/README.md) proofs keep alternate branches,
external socket providers and incomplete contribution-query membership distinct
from intrinsic source classification.
The [structural proof](../structural-support-domains/README.md) preserves both
positive generated-skill chains and the four scalar source families. All six
numerical owners remain Partial, and generated descendants require their own
source coverage.
The [Class proof](../class-support-domains/README.md) covers all eight mapped
classes, their base attributes and unarmed defaults. Start nodes, Ascendancies
and foreign weapon modifiers retain separate ownership; all sixteen numerical
programs remain Partial.
Inherited supports remain present, external origins remain independently checked,
and all numerical Partial coverage stays unchanged. No new names or IDs are
allocated by this metadata update.
This adds no whole-build coverage or numerical parity and
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

Current input: `615c66506ee4421ab03b1eef95486d8fa532755846168bb7738cee3c97e8d26e`.
The 103 packets reference 2,562 distinct complete rule sources, preserving all
8,213 occurrences. Their local name files use 5,011 distinct exact addresses;
the declaration ledger adds 69 names. The 2,787 declared inputs occupy 5,789,428
encoded and 58,665,128 decoded bytes; the root manifest is 183,544 bytes.
All fit the existing default limits. The maintained Authoring tests check actual
source graphs, historical artifacts and the new initial-type projection.
