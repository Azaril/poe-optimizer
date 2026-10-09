# Constructed self-flat physical supplier census

This packet records a **nonempty supplier inventory**, not native contribution
membership or an empty-domain proof. It uses the original pinned PoB
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` constructors and parser. It adds no
owned definitions, runtime rules, coverage closures, or input dispositions.

## Captured scope

The original skill, shared stat-map, minion, and ModCache graphs contain 256
unique `PhysicalMin`/`PhysicalMax` modifier objects: 128 of each name. The raw
catalogue still has 112,987 tables and 344,790 rows; adding ModCache gives
145,363 tables, 436,112 rows, and 11,851 modifier objects. These are object
counts, not eligible contribution or occurrence counts.

The packet retains 23 physical mapping rows, 42 resolved stat-consumer rows,
and metadata for 19 owning definitions. It includes actual quality, alternate
quality, stat-set, and constant-stat consumers. Resolution calls the original
stat-map lookup, including its shared fallback and local overrides. The
`_grantedEffect` entry installed by `Data.lua` is treated only as metadata after
checking its exact owner table identity and id; no general underscore-field
exception is used.

Each modifier records its canonical raw graph path, whole body, and original
enclosing modifier bodies. Inventory traversal deduplicates shared table
objects. The three relevant alias groups retain their alternate paths and
the affected canonical flat paths, but do not prove delivery eligibility for
every alias. A mutation that reuses one inner flat table through a second
`MinionModifier` wrapper must remain visible as an alias route. This is a
unique-object catalogue with explicit alias limitations, not an exhaustive
enumeration of applied source occurrences.

The inner-filter classification identifies 114 records with `BASE`, zero
flags, zero keyword flags, and no tags; the other 142 require further review.
That classification does not admit their enclosing wrappers, values,
recipients, or delivery paths.

## Concrete supplier distinctions

Two of the resolved consumer rows are actual supports: Olroth's Hubris and
Runic Infusion consume `added_physical_damage_%_ward_cost`, with constant
values 15 and 25 respectively. Their physical endpoints carry a `PercentStat`
tag reading `WardCost`. These are Action-cost-dependent candidates, not an
unconditional receiving-Actor subtotal. The packet preserves the supports'
actual require/exclude skill-type metadata; it does not claim they can be
assigned to the selected Sniper. The other consumer rows belong to active or
granted-active definitions; a file name containing “support” is not support
authority. The selected Sniper summon and Basic Attack have no matching
stat-consumer rows in this census.

Four cached records are nested within actual `MinionModifier` wrappers:
“Minions deal 7 to 11 additional Attack Physical Damage” and “Minions deal 7
to 14 additional Attack Physical Damage”. Their inner records carry Attack
keyword flag 65536. They must not be conflated with the unqualified custom
3–7 control merely because their wrappers or numeric channels match.

The original parser is also executed on the three retained control strings:
unqualified 3–7, explicit 0–0, and 3–7 while on Full Life. Each produces two
actual `MinionModifier` payloads, with zero inner flags and keyword flags;
only the last has a `Condition: FullLife` tag. Parsing proves these controls
are accepted source syntax. It does not prove an obtainable game item or
passive supplies them.

## Original query observations

The exact retained preconversion report is authenticated through the existing
combined-added source proof. Its eight cases cover the original build,
repeat, warm restoration, effective CALCS selection, 3–7, explicit zero, and
Full Life disabled/enabled controls. Nine Physical calls were observed across
MAIN and CALCS; the seven unexecuted CALCS rows remain explicitly unobserved.

Every observed call retains the exact Sniper/Basic source occurrence,
original pass cfg and pointer checks, raw modifier inventory, filtered query
records, and the original `added_min`/`added_max` locals. The actual cfg has
flags 42949813253, keyword flags 330752, and skill conditions containing only
`MainHandAttack`. The supplied flat records reside one ancestor store above
the skill query. Explicit zero and disabled-condition records remain present
in the raw census while absent from nonzero `Tabulate` results. The enabled
condition produces 3 and 7 through the original query, with no copied formula
used as evidence. No fresh full-build VM acquisition is needed for this
packet; the retained report already includes independent uninstrumented
comparisons and its own acquisition authentication.

## Dynamic and input boundaries

The pinned `dynamic_domains` ledger distinguishes actual MinionModifier
delivery, global/minion buffs, Tactician and Companion parent-weapon damage,
Rallying Cry, Hollow Palm, spell-only dynamic flats, stat-map expansion,
condition resolution, Party transport, and ordered sums. These are reviewed
source paths, not a claim that the constructed modifier catalogue exhausts
every runtime factory.

In particular, Tactician and Companion use different outer filters;
Tactician is not excluded merely because the selected Sniper is not a
Companion. Rallying Cry uses parent weapon and buff state. Hollow Palm also
has an alternate enabling flag, so the lack of a Staff tag is not a universal
exclusion. Imported Party text can create arbitrary numeric modifier names;
this packet retains the accounted no-Party boundary and does not retire
configuration obligation `01f2`, external obligation `0207`, or any other
selected input obligation.

Full Life is an invariance candidate, not a completed native condition proof.
`ModStore.GetCondition` reads cfg overrides and Action-filtered condition
flags, and condition tags also read `cfg.skillCond`. The observed cfg alone
does not exclude future local Full Life assertions or Action-sensitive flag
suppliers. No native Full Life definition/import binding is supplied here.

## Next bounded native step

For a source family proven invariant for the receiving Actor, existing
checked Actor producers and contribution queries can feed an Action through
the existing Actor-to-Action stat route. The simplest evidence here is the
unqualified, zero-flags, zero-keywords MinionModifier shape. A real producer
and exact selected-source inventory still need authentication before any
membership can become Complete. A Full Life subset additionally needs its
condition binding and override exclusions proved. PoB parent-store topology
alone does not require a new public transport model. An actually required
Action-sensitive supplier, such as an eligible WardCost-dependent support,
would require a separate authority review.

Original local-record order followed by parent Sum remains a separate
constraint. This census does not justify arbitrary native source ranks or
floating-point reassociation. Live mutations of zero/nonzero records,
nested wrappers, shared aliases, and real stat consumers must change the
reviewed semantic census even when their source hashes are recomputed.
