# Tracked rule source and remaining metadata bootstrap

`release.kdl` is the sole production source manifest for the retained release.
It lists every input explicitly, relative to this directory. First extract and verify the
allocation predecessor using the [canonical reference](../authoring-genesis.md),
then build into a fresh output directory:

```powershell
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release build data/owned/poe2/3887ae68/migration-source/release.kdl --predecessor canonical-checkpoint --expected-predecessor 71ff4429b7fcfb4f75e45d9c58df878453cb25428542d3531074fc08fccbba91 --output rebuilt-release
```

The canonical snapshot preserves the current 73-name allocation ledger. Source builds use that checked checkpoint with
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

All 117 contribution queries live in the five declared `contribution-queries/`
KDL files. Their names resolve after Stat allocation in the same build as rule
programs; no separate allocation step or predicted identifier is needed. Exact
source authority, member order, reducers, empty values and coverage remain in
these files. The original and revival fragments retain both outer Partial gaps;
all 48 partial groups retain their own gaps. The 123 groups contain 2,128 members.
Composition uses the existing runtime query model
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
must remain unchanged except for the explicitly admitted 73 Stat declarations,
nine program occurrences, three revival queries with one new outer gap, and 363
intrinsic support-domain declarations, plus four reviewed generated-singleton
usage destinations, the default-Timeless source-presentation flag and their
constructed dependency commitments. The nine programs comprise Sniper's initial
types and support prohibition, four exact skeletal Skill intrinsic-revival
instances and three passive revival sources. The original revival checkpoint
added Sniper and the three passives; the later extension adds Arsonist, Frost
Mage and Reaver with six query members, reusing the same Stats and source body.
The [singleton accounting evidence](../generated-firebolt-dispositions/singleton-accounting.md)
keeps the unchanged originals and independent generated sources blocked. The
independent [Timeless preference proof](../timeless-search-defaults/README.md)
reclassifies only exact default source-tool preferences in Originals 01/04/05;
extra saved distance fields in 02/03 stay unresolved. Native build/query values
and every selected input obligation remain unchanged. The
release test removes exactly the reviewed deltas,
restores historical program members and uses checked content constructors to bind
the original dependencies. Complete historical schema meaning and semantic receipt
fields must match. Every non-schema, non-manifest artifact remains byte-exact;
restoring only the old schema row must reproduce the original manifest size/hash.
New schema wire commitments are asserted separately, without an old encoder.
New semantic schema/registry identities change dependent artifact bindings;
the retained historical query payloads remain byte-exact.

Historical reference identity (not the current DSL input):

- Input: `a75ef471b57d8184b208ca5f2a26ce6a9a01d15340db8b15fc1f318be40c1905`.
- Rule bytes: 17,488,370; SHA-256 `6ebd514b3c9e57a41a025426d9bf3c80f1fc64749fa665c9e8f82ebcf41b1366`.
- Receipt SHA-256: `46f233e58f7449f789a93e0b9d589892b4883fe40dbff672954aa6af22faa538`.
- Upstream revision: `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`.

## Preserved scope and limitations

The release has 13,337 active allocations, 9,464 schema definitions and
3,873 slots. Rules have 6,550 owners, 8,221 program occurrences, 18 integer
tables, 59 receivers, 117 contribution queries, one effect application and one
existing-Actor binding. The five named query sets retain their original order
and 22 queries each. The 227 provenance rows are unchanged metadata, not
instructions to replay historical migrations.

There is no evaluation bundle. The derived manifest still reports 5,827 partial
rule owners and five partial routing outputs. The first declarations add Sniper's
initial own/minion type facts and explicit minion-context presence through generic
Skill Boolean Stats. See [evidence and limits](../initial-skill-types/README.md).
The next declaration adds the separately reviewed definition-level support
prohibition; see its [source proof and limits](../support-prohibition/README.md).
The [revival producers](../revival-producers/README.md) add three generic Skill/Actor
channels, intrinsic flags and twelve-second overrides for the exact Sniper,
Arsonist, Frost Mage and Reaver Skills, and three distinct Player passive
contributions. Their seven program instances share three bodies while retaining
eleven exact effects and separate physical supplies. Source mutation, crossed
supply and duplicate-member controls preserve this authority. Their three
queries retain incomplete membership and numeric ordering; the remaining source
census, paired/conditional contributors and post-stacking Sacrifice are still
required before a final revival duration can be claimed.
Eight existing Skill, Actor and declared-slot owners, three root/Ascendancy
owners, four UsagePolicies, 293 ordinary attribute-choice passives, eighteen
reviewed numeric minion passives, seven class-view passives, one intrinsic socket,
one Boolean-flag passive, six structurally reviewed passives, all eight Classes
the shared Player Actor, four reviewed item templates and nine additional
skeletal parent/Actor-slot/Actor owners now have intrinsic support-source domains,
bringing the total to 1,349 (1,320 Known, 29 Unmapped).
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
The [item-template proof](../item-support-domains/README.md) reuses complete base
records and authenticates constructor/output boundaries for Ashen Staff, Fine
Belt, Rope Cuffs and Tattered Robe. Numerical records, separate modifier suppliers
and Firebolt's exact Modifier grant remain intact.
The [skeletal proof](../skeletal-support-domains/README.md) covers the complete
Arsonist, Frost Mage and Reaver parent/profile bodies and all 22 resolved mapping
rows, retaining seven null mappings and all positive numerical outputs. The
maintained tests join all 28 selected SkillSet rows through existing source
correspondences, preserving duplicates and exact generated providers. Missing
Commands, children and numerical coverage remain unresolved. No runtime gap
removal is claimed before the unchanged build has genuine admitted inputs.
Inherited supports remain present, external origins remain independently checked,
and all numerical Partial coverage stays unchanged. The support-domain metadata
additions allocate no names or IDs; the separate Stat declarations do.
No supplied original is complete natively (0/5). This adds no whole-build coverage
or numerical parity and does not make an incomplete build executable. The first complete unchanged
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

Current input: `cc25c852ad3918a6fa610fe423890bb7d89e3c2fe03f1be264fb3a4ecd0004ea`.
The 104 packets reference 2,566 distinct complete rule sources, preserving all
8,221 occurrences. Their local names bind exact existing addresses; the shared
`declarations/mechanics.kdl` component declares the 73 Stats bound by the persisted
name ledger. The five
contribution-query fragments use those declarations without separate allocation.
The 2,802 declared inputs occupy 6,731,152 encoded and 46,030,245 decoded bytes;
the root manifest is 184,376 bytes. The eighteen runtime files occupy 53,665,506
bytes, including the 19,774,085-byte schema; the checkpoint receipt and name ledger
total 455,218 bytes. All fit the existing default limits. The maintained Authoring
tests check actual source graphs, historical artifacts and the new component producers.
