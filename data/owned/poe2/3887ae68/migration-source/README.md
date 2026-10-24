# Tracked rule source and remaining metadata bootstrap

`release.kdl` is the sole production source manifest for the retained release.
It lists every input explicitly, relative to this directory. First extract and verify the
allocation predecessor using the [canonical reference](../authoring-genesis.md),
then build into a fresh output directory:

```powershell
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release build data/owned/poe2/3887ae68/migration-source/release.kdl --predecessor canonical-checkpoint --expected-predecessor 5f77a8f22888bd946d273a7376c92fe9838535b770dff5a58d6996aa04662a37 --output rebuilt-release
```

The canonical snapshot preserves the current 70-name allocation ledger. Source builds use that checked checkpoint with
`--predecessor` and `--expected-predecessor`; do not bootstrap evolving sources.
The output contains `checkpoint.json`, `names.json` and `runtime/`. Runtime
consumers receive `rebuilt-release/runtime`. The [canonical reference](../authoring-genesis.md)
records archive and checkpoint digests and clean-clone commands outside
this compiled inventory; local `runs/` outputs are only verification evidence.

Rule programs live in the declared `.rule` files. KDL packets bind each program
to its exact definition or declared slot and preserve membership order. The
packet-local KDL name files bind semantic aliases to checked typed addresses;
slot bindings include their declaration owner. Reusing a complete source file
does not merge its occurrences or their recipients. Identical program IDs alone
never justify sharing a body.

All 114 contribution queries live in the four declared `contribution-queries/`
KDL files. Their names resolve after Stat allocation in the same build as rule
programs; no separate allocation step or predicted identifier is needed. Exact
source authority, member order, reducers, empty values and coverage remain in
these files. The first fragment retains the outer Partial gap; all 45 partial
groups retain their own gaps. Composition uses the existing runtime query model
and final validators, not another evaluator.

The compressed `bootstrap/rules.json.gz` contains only rule metadata: owner and
coverage inventories, tables, receivers, applications and bindings.
It has **no program or contribution-query members**. The builder rejects embedded
members, including when no corresponding KDL sources are declared. A source error
cannot select another execution path.
The remaining compressed JSON constituents are temporary R1/R2 metadata inputs,
not the final compact runtime pack. Loading the canonical snapshot needs no local
predecessor, historical source rebuild, staging helper, migration generator or PoB checkout.
Schema artifacts use the sole compact typed codec with one root namespace; the
semantic schema version and identity are independent of that wire representation.

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
must remain unchanged except for the explicitly admitted 70 Stat declarations,
two Sniper initial-type/support-prohibition programs and 350 intrinsic support-domain declarations.
The release test removes exactly that delta,
restores historical program members and uses checked content constructors to bind
the original dependencies. Complete historical schema meaning and semantic receipt
fields must match. Every non-schema, non-manifest artifact remains byte-exact;
restoring only the old schema row must reproduce the original manifest size/hash.
New schema wire commitments are asserted separately, without an old encoder.
New semantic schema/registry identities change dependent artifact bindings;
unchanged query payloads remain byte-exact.

Historical reference identity (not the current DSL input):

- Input: `a75ef471b57d8184b208ca5f2a26ce6a9a01d15340db8b15fc1f318be40c1905`.
- Rule bytes: 17,488,370; SHA-256 `6ebd514b3c9e57a41a025426d9bf3c80f1fc64749fa665c9e8f82ebcf41b1366`.
- Receipt SHA-256: `46f233e58f7449f789a93e0b9d589892b4883fe40dbff672954aa6af22faa538`.
- Upstream revision: `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`.

## Preserved scope and limitations

The release has 13,334 active allocations, 9,461 schema definitions and
3,873 slots. Rules have 6,550 owners, 8,214 program occurrences, 18 integer
tables, 59 receivers, 114 contribution queries, one effect application and one
existing-Actor binding. The five named query sets retain their original order
and 22 queries each. The 227 provenance rows are unchanged metadata, not
instructions to replay historical migrations.

There is no evaluation bundle. The derived manifest still reports 5,827 partial
rule owners and five partial routing outputs. The first declarations add Sniper's
initial own/minion type facts and explicit minion-context presence through generic
Skill Boolean Stats. See [evidence and limits](../initial-skill-types/README.md).
The next declaration adds the separately reviewed definition-level support
prohibition; see its [source proof and limits](../support-prohibition/README.md).
Eight existing Skill, Actor and declared-slot owners, three root/Ascendancy
owners, four UsagePolicies, 293 ordinary attribute-choice passives, eighteen
reviewed numeric minion passives, seven class-view passives, one intrinsic socket,
one Boolean-flag passive, six structurally reviewed passives, all eight Classes
and the shared Player Actor now have intrinsic support-source domains,
bringing the total to 1,336 (1,307 Known, 29 Unmapped).
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
The [shared Player proof](../player-support-domains/README.md) covers its intrinsic
initializer and generic modifier constructors. Foreign sources and the nine
Partial numerical programs remain independently accountable; the UI fallback
action supplies no native coverage.
Inherited supports remain present, external origins remain independently checked,
and all numerical Partial coverage stays unchanged. The support-domain metadata
additions allocate no names or IDs; the separate Stat declarations do.
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

Current input: `9d9e3326f0a6fdf211f48df8d2813e61f8550012c5ed131ab7e3242d38fae838`.
The 103 packets reference 2,563 distinct complete rule sources, preserving all
8,214 occurrences. Their local name files use 5,011 distinct exact addresses;
the declaration ledger adds 70 names. The four contribution-query fragments and
their local names add no allocations. The 2,796 declared inputs occupy 6,720,948
encoded and 46,018,221 decoded bytes; the root manifest is 184,029 bytes.
All fit the existing default limits. The maintained Authoring tests check actual
source graphs, historical artifacts and both new definition-level producers.
