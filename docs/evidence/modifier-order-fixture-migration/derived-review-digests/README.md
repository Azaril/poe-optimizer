# Derived review digests after the item-order migration

The initial CLI pass found two final retained-proof mismatches in self-skill construction. Its inputs were already migrated, but digests of the composed input and selected membership still described the previous order format. This applied repair updates nine derived leaves in six reviews and their exact dependency closure: 25 JSON files and five Rust hash constants, 69 hash-string replacements total. Every changed file has a whole-byte inverse and unchanged length. No calculations, source observations, gameplay values, completeness claims or refusal conditions change.

The preparation reproduces all three retained old Rust digests before deriving the two new ones. A separate token-preserving implementation independently reproduces both sets, verifies the complete selected and full-input inverses, and confirms unchanged source, recipe and gate data. The only numeric input change is the authority receipt length for the already-migrated bindings file. The prepared dependency overlay scan covered 3,637 maintained files including 163 gzip files; only preserved historical receipts retain old hashes. This is bounded evidence, not a substitute for native tests.

The postcheck supersedes the earlier minimal-wire artifact inventory where changed: it checks 45 current artifacts and eight literal-pin files. The separately archived Mana raw-report repair remains additional evidence. Earlier reports stay immutable. At archival, the 21 affected targets plus item-evasion-property were running under the unchanged test profile; numerical/runtime validation is still pending. Consult the living implementation checkpoint for terminal results.

These archived scripts document executed one-off fixture authoring. They are not production importers, new runtime formats, or compatibility readers.

| File | SHA-256 |
| --- | --- |
| [prepare.cjs](prepare.cjs) | `9fa661adfde9d3ef49277ba375b93c0d552b8795942b5b327bf559079e7598d5` |
| [preparation-report.json](preparation-report.json) | `9efc4d936dfff1d7b1c4d825c2b1806cd36603d41fab3edeec2444384087d224` |
| [expected-inventory.json](expected-inventory.json) | `6498f091b496e82a3bf087b86673839b3bb072d609337b3108574265097ec88a` |
| [affected-targets.json](affected-targets.json) | `1773487a81b42765053e6ca9b58e564c8256fec17e9ad6458450032832a444e8` |
| [apply.cjs](apply.cjs) | `e105f60ddb466c29bf45a56b6733189ccf6c9a486b610942fcaa49034936e5cc` |
| [postcheck.json](postcheck.json) | `e3d88ecd88440deced0602b6a61924dec7e54156d3cf05fcc861802112000385` |
| [independent-review.json](independent-review.json) | `5fe3b7fe500cffb8aa89f3b71e8c7f4aa903be7b3275a5b2c745be538ec14d5d` |
| [stale-hash-audit.json](stale-hash-audit.json) | `fcb095097c6efaba06318880bc8b8e1791c094ce04a261bcad9c01d9bb684ccc` |
