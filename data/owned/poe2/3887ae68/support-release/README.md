# Real support release evidence

[`evidence.json`](evidence.json) records the pinned source facts and current native
availability needed to author Twister / Elemental Armament II and Skeletal Cleric /
Meat Shield II support artifacts. It is an audit, not an executable package or a
whole-build parity claim. The package and source file hashes make the observation
repeatable; detailed selected schema, rule, import and source excerpts live in
`runs/support-release-data-audit-01`.

The baseline is the checked `runs/owned-actor-abilities-01/package` release. It uses
operations v11 and contains no evaluation or support-origin-order artifact. The
current native support path requires an explicit migration to v13, exact package
bindings and computed frozen inputs. No raw Gem value or source fixture result may
stand in for effective support level or quality.

| Component | Existing identity / topology | Current limit |
| --- | --- | --- |
| Twister | Gem `000a`, Skill `000b`, Gem grant `0010`, skill slot `000f`, output `000e` | Gem and Skill rule owners remain Partial. Required Skill level `000c` and quality `000d` already have projections. |
| Elemental Armament II | Gem `0733`, source effect Skill `03f0` | Gem input declarations remain Partial; effect schema is Unmapped; no owner programs. |
| Skeletal Cleric | Gem `0916`, summon Skill `0325` | Summon Skill schema is Unmapped; no native Cleric actor, grant or output chain exists. |
| Heal Buff | Skill `01c6` | Schema is Unmapped; requires explicit Cleric actor-owned ability supply and output. |
| Meat Shield II | Gem `083e`, source effect Skill `048d` | Gem input declarations remain Partial; effect schema is Unmapped; no owner programs. |

All abbreviated keys above have the `def.000000000000` prefix and belong to the
namespace recorded in the evidence. The baseline's Actor `3091` is Skeletal Sniper,
so it cannot receive Cleric effects. First materialize the Cleric population and
Heal topology, using the existing generic actor-supply contract. The source Cleric
list also names Resurrect and DoLiterallyNothing, but neither has a pinned skill
definition; the original constructor filters them. Preserve this distinction from
unconverted or newly added abilities.

Armament II contributes 25% MORE ElementalDamage with an Attack keyword filter and
also has a 20% mana multiplier. Meat Shield II contributes -40% MORE Damage and
DamageTaken through MinionModifier transfer to the exact summoned Actor. Those
modifiers must be delivered once per retained application to that Actor; children
inherit its channels. Child preparation reuses the parent's selected support list,
preserving source scalars, and uses the explicit summoner context without replacing
the child's own initial types. The four-type controlled parity fixture is not a
complete real skill vocabulary; the evidence includes the full raw membership of
Twister, Cleric and Heal.

The original Twister group is original02, active skill set 6, group 8, with five
supports. The Cleric group is original01, active skill set 1, group 5, with four.
The evidence preserves their authored order and source hashes. Local group order
does not establish the completeness of merged support origins. Other supports and
all existing build contributors must retain their Pending/Partial gates.

The next useful result is a real migrated data release containing reviewed support
definitions, exact receiver roles, stage/input/output bindings and numeric channel
recipes while preserving these gaps. It should reproduce the existing authenticated
component witnesses before any broader evaluator coverage is claimed. The optional
source tests demonstrate factors 1.25 and 0.6 in controlled surroundings; they do
not establish imported-build effective values, damage, cost, defences or legality.
