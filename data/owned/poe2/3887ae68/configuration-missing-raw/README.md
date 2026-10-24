# Missing raw configuration: three reviewed source families

This packet records source evidence for the missing-name branches of `ignoreJewelLimits`, `ignoreItemDisablers` and `multiplierManaBurnStacks`. It supports a future checked `MissingRawNoAdditionalInput` assignment for each family. It does not create a complete configuration census, emit a native default, or complete ordinary item, passive, resource or Scenario rules. Authored false, zero and empty values are outside this missing-only claim.

The reviewed source revision is `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`. The [source manifest](../../../../../crates/poe-optimizer-pob/data/pob-source-manifest.json) has SHA256 `8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675` and specifies `utf8_crlf_to_lf`. All complete files below already exist in tracked evidence; this packet adds no duplicate archive.

## Complete source authority

The source byte counts and hashes are for the complete LF files, after gzip decompression where applicable. The gzip identities separately authenticate the stored transport. These are complete files, not extracted fragments or a selected-build search result.

| Pinned source | Existing tracked file | Source bytes | Source SHA256 |
| --- | --- | ---: | --- |
| `src/Classes/ConfigTab.lua` | [ConfigTab.lua](../configuration-accounting/evidence/pob/src/Classes/ConfigTab.lua) | 50,481 | `0107cdedf34094d67542e8a9012fe1040cf616ab0ad8379bc88d9167df28af88` |
| `src/Modules/ConfigOptions.lua` | [ConfigOptions.lua](../configuration-accounting/evidence/pob/src/Modules/ConfigOptions.lua) | 235,127 | `0ddf50157dc98a63c11327fc41b989dd7aca713ef7e402037eb79409bffb0467` |
| `src/Modules/CalcSetup.lua` | [CalcSetup.lua.gz](../configuration-distance-inputs/CalcSetup.lua.gz) | 100,609 | `5c51f3a8dd93ab99ce67763a015ad4644b4a99b1a74c31359b831e3965404312` |
| `src/Modules/CalcPerform.lua` | [CalcPerform.lua.gz](../configuration-distance-inputs/CalcPerform.lua.gz) | 181,097 | `d7aee3caffd3e4225066716075e54437466ccf0b8e53a816fa7083e210839cae` |
| `src/Classes/PassiveSpec.lua` | [src_Classes_PassiveSpec.lua.gz](../passive-structural-inventories/sources/src_Classes_PassiveSpec.lua.gz) | 95,117 | `fb1f9cb7ad9c303808c741b70059981afdcb2876cc5a8a0a67da940c9401049c` |

| Gzip file | Stored bytes | Stored SHA256 |
| --- | ---: | --- |
| `configuration-distance-inputs/CalcSetup.lua.gz` | 21,399 | `cdefa2fbbb4a15befdc728fd549902f4032c1cdf0b52c3070dc356f0a45e718f` |
| `configuration-distance-inputs/CalcPerform.lua.gz` | 34,408 | `7a5fd43ebed36bbc17bbc46da449f8871940e3476b1bd11450e7050f31dec575` |
| `passive-structural-inventories/sources/src_Classes_PassiveSpec.lua.gz` | 22,792 | `8b797c17e938efbf73b95f0a57ea197e81be37510b54a04ea25d8a400612c55e` |

The following inclusive, one-based spans identify the reviewed branches. Each span hash includes the terminating LF of every selected line. Full-file authentication remains required: these spans do not independently prove the absence of other consumers or arbitrary dynamic aliases.

| File and lines | Reviewed boundary | Span SHA256 |
| --- | --- | --- |
| `ConfigOptions.lua:216-217` | Complete two check-control declarations; neither has an apply callback or installed default | `a1def4e86f7b8cc5f567772cdef25cff61beeac1385410e4067e16d21a08bec7` |
| `ConfigTab.lua:713-731` | Input defaults versus widget display defaults | `0455b59b41080822c50336a165f248200b22344d3e6f6181670a2d4410bf39f1` |
| `ConfigTab.lua:878-978` | Complete saved Config loader, including every ConfigSet and all Input/Placeholder lanes | `029d5a2a7b8d5ca5cdccd5f1b862429d292e50cce962a230dde31771fab94578` |
| `ConfigTab.lua:1169-1199` | Ordinary BuildModList callback dispatch before custom modifiers | `a3600410e99875df19dcb301113842db017960ffcc5ee55eb66508bcdcbd4018` |
| `ConfigTab.lua:1317-1334` | Complete CreateConfigSet constructor | `60ff84266cd7f571d1b199bf0dae52509d516d61c1bf8108131029fef2724dd3` |
| `CalcSetup.lua:1039-1048` | Jewel-limit flag forwarded into granted passive discovery | `10db6d1fbc2d7b7da158e8f68dc237d79e695926992548118a19c2c436c0844f` |
| `CalcSetup.lua:1119-1151` | Allocated/valid jewel and limit checks before radius handling | `24ca0ee4c66c3b04a9255b08f7a0d966981a401472f089c8ff3e8e4f783bc400` |
| `CalcSetup.lua:1216-1268` | Complete item-disabler branch | `f056ee627e111745ca350b573581036ca4bddd39711e7559c6a9e30a89d9a331` |
| `PassiveSpec.lua:1222-1274` | Complete CollectGrantedPassiveNodesFromItems function | `4d346db04386042310655aaef4204eaf6e6eb91ac9ead3f31adf340a07d64953` |
| `CalcPerform.lua:805-818` | Complete configuration Mana Burn stack branch | `b1bbcc85ee95093b3434c30f1137093ac03f1d960b127e22a2e3d5a33503458a` |

## Meaning of the missing branches

`CreateConfigSet` installs declared defaults before saved fields are loaded. The two check controls have no installed input default. `multiplierManaBurnStacks` is a direct calculation reader with no `ConfigOptions` declaration; a catalogue-only census would omit it. `Load` accepts multiple value lanes, and a string `Placeholder` writes to the input table. Consequently the admission condition must be whole-name raw absence across Input and Placeholder lanes in the exact checked ConfigSet. It cannot mean “no numeric adapter succeeded.” Every saved ConfigSet requires its own accounting, including dormant alternatives.

| Family | Missing-branch meaning | Remaining responsibility |
| --- | --- | --- |
| `ignoreJewelLimits` | Ordinary jewel limits remain active both during fixed-point granted-socket discovery and during jewel/radius contribution collection. The missing branch requests no additional override input. | Item limits, socket allocation/validity, granted passive discovery and resulting contributions still need their ordinary native rules and coverage. A true value is a topology bypass and is not admitted here. |
| `ignoreItemDisablers` | The item-disabler branch remains active. It examines `CanNotUseHelm`, actual `DisablesItem` tags and exclusions, then resolves chains/cycles and removes disabled equipment. The missing branch requests no additional override input. | Equipment legality and participation remain authoritative. This proof neither marks items active nor completes their requirements. A true bypass remains outside the admitted branch. |
| `multiplierManaBurnStacks` | `(value or 0) > 0` does not execute, so this block adds no Config-origin Mana Burn/Effective Mana Burn/Weeping Wounds stack contribution. | Other stack sources, maximum handling, Actor resources and the rest of the mechanic remain unproved here. No universal zero-stack claim follows. A saved zero, negative, false or malformed value is still a raw occurrence, not absence. |

The pinned direct-reader branches are reviewed source semantics. This packet adds no new executed PoB observation and does not use an empty selected modifier list to prove source absence. Positive values and changed constructor/default/direct-reader definitions must invalidate a missing-only assignment unless separately reviewed.

## Unchanged Original05 and current commitments

[Original05](../../../../../tests/fixtures/builds/breadth-20260908/build-05.xml) has SHA256 `442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089`. Its sole ConfigSet is `1`, containing 8 Inputs and 34 Placeholders. None of the three reviewed names occurs in either family. This is a raw source fact, not an absent native value or a fabricated source occurrence. Current normalization and saved-selection completion must retain their unresolved responsibilities until their entire inventories are accounted for.

At this checkpoint the maintained configuration disposition policy has 38 rules and no inventory. Its mapping source is `4d61f5c99db4ed1a53ba34ab12d21688edabe486a63b790a0b7fec9dad05b177`. Its catalogue commitment is `dc2f92843c43922c4cc0d949c5d203557185f132e0fb35d068582c790fa09102`; its constructor commitment is `fe7cd80255e0ff4ec5c7a6ba01640d1b230727737a78f9b1b4ad2819c3c7b088`.

Those last two commitments identify the existing [catalogue evidence](../configuration-accounting/catalogue-evidence.json) and [constructor evidence](../configuration-accounting/constructor-evidence.json) under their typed commitment domains. They are not raw file hashes or permission to widen the evidence claim. The catalogue explicitly covers one numeric `enemyDamageRollRange` Placeholder and excludes default-inventory authority; the constructor evidence concerns the no-boss callback for that same control. The presence of complete source files in that older packet does not broaden those semantic commitments. Keep those documents unchanged. A future combined evidence document must explicitly retain their existing claim and add these reviewed branches before binding a real enlarged source census.

## Smallest useful integration and publication boundary

Use the existing complete, finite Import test fixtures to exercise the missing-raw receipt/matcher and atomic retirement paths. The fixture's complete declared domain is its own authored contract; it must not be described as the complete PoB configuration catalogue. Check same-name occurrences in every value lane, malformed siblings, dormant ConfigSets, wrong source/owner/commitments, missing or surplus assignments, exact and one-short budgets, and deterministic replay. Missing-raw admission must not consume another ConfigSet's proof or manufacture an output/default.

A maintained Authoring source law can reuse `tracked_release()` and the existing Original05 normalization/selection helper. Authenticate the five complete files and reviewed spans, join all three missing names to every actual ConfigSet, and preserve the current draft, queries, allocation history and selected Pending inventories. It can check that the current policy still has no inventory and that changed source/default/consumer evidence refuses admission. This provides current-release regression evidence without publishing a false completion claim. No new importer or production name dispatch is needed.

Do not publish a three-row Partial census solely to exercise compilation: the existing inventory prepare/matcher returns before completion matching when the census is Partial. Such an artifact would prove binding only, not execute these admissions or retire Original05's configuration obligation. The first production publication should join these assignments to the reviewed catalogue plus direct-consumer census, bind its complete accounting to actual adapter dependencies and explicit remaining semantic owners, and pass final maintained assembly. Any unreviewed consumer keeps that census incomplete. This packet alone changes no production policy, Scenario inventory, support-source domain or complete-build status.
