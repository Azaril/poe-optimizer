# Selected equipment placement

This packet closes five existing character-slot inventories from the flat-Life predecessor. It supersedes the former three-template `selected-life-item-placement` packet; historical evidence remains immutable, with no parallel current authoring path.

| Template | Complete destinations |
| --- | --- |
| Tattered Robe `238c` | Body Armour `0067` |
| Rope Cuffs `2007` | Gloves `0068` |
| Sapphire Ring `09dc` | Ring 1/2/3 `006b/006c/006d` |
| Fine Belt `1e84` | Belt `006e` |
| Ashen Staff `1d75` | Weapon 1 `0064`, scoped to a weapon loadout |

The Ring retains its three already-known members; the other four replace empty Partial inventories. Every other descriptor field, numerical owner/program, imported record, saved selection and contributor inventory is preserved. There are no new definitions or evaluator programs. Combined with prior template declarations, all nine actual selected uses (eight templates) now have complete character-slot placement. This does not complete item mechanics, sockets, requirements, stock, support discovery or any build evaluation.

The optional source witness calls the pinned original `ItemsTab:IsItemValidForSlot` on actual Original05 Items 19, 20, 26, 27 and 28. Each is the exact catalogue base and selected MAIN/CALCS object. It retains full base type/subtype/tags and covers 113 registered slots, the active default plus six saved equipment sets, and actual flags plus all eight combinations of the three accepted flag arguments. Calls repeat in place; independent fresh no-call controls and observed replays agree, as do both JIT modes. Eleven source-file pins include all five complete base modules, construction and the original placement method.

The branch audit excludes alternate accepting destinations: none has onehand, one_hand_weapon, axe, mace or sword tags, and none is Jewel, Flask, Transcendent Arm/Leg, Shield, Focus, Sceptre or Quiver. Staff has `twohand` but cannot enter the flagged offhand branches. Source `Weapon 1` and `Weapon 1 Swap` map to one typed slot through the existing loadout scope model; they do not become two receiving slots. This placement proof does not establish the Staff's attack profile or an Unarmed condition. Unregistered suffix strings and `Weapon` may be accepted by PoB, but are not owned destinations. Charm contents and sockets do not create character-slot eligibility.

`source-vectors.json` is an authenticated projection, not native input. Publication recomputes it from the full 35,595-call census per observed VM. Raw false, nil and zero-return outcomes remain distinct source evidence. Complete read sets and selected scalar outputs stay unchanged. Ring's two selected uses retain distinct use identities and the same rolled item record.

The existing descriptor migration compiler changes only these five inventories and rebases definition identities. An exact inverse, import rebindings, all 110 queries, all five original issue counts and the absent evaluation bundle are checked. Native public-binder controls cover 21 actual saved uses, 37 allowed bindings, 383 other-slot refusals, 21 wrong-scope refusals, both Staff loadouts, duplicate Ring uses, restored Partial predecessors and removed uses. These finite requests explicitly omit unrelated domains; they do not finalize the imported build. The joined Life fixture consumes published placement inventories without adding or closing destinations.

Immutable source evidence: `runs/owned-selected-equipment-placement-source-01/source-jit-{off,on}.json`; each 61,444,068 bytes, SHA-256 `17b24c4937313b1d35495210ca0f1d73af193246bef5a2c6ae524b4cb6a1619d`. The default optional-PoB test uses fresh temporary output; an explicit evidence path refuses reuse.
