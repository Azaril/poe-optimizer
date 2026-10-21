# Import preference policy checkpoint

The existing source-presentation policy now opts into Import preference accounting. Actual publication input is `6f58eba00e71984264123419f51114f375bdce23440eaa4ff3ffa4999e2beb11` (prior `720d65ca89ec10a9ec57d26324abc52d963b6246cbfae618105c86e9e129eacf`). Full release receipt SHA-256 is `73393bbd6b8629e564ad31073c90eeb69133932d9bad77510fc9874b36dcc856`; the complete receipt and compact per-original evidence are retained in validation.json (SHA-256 `388ee6cddc6db740cbfb2c37c74b1cbb8e5c85990b6d61406af084fe1f3d1b34`).

Validation passed: five existing Import adapter tests, 15 ordinary presentation tests (four explicit publication tests ignored), one all-five publication/preservation test, and scoped Clippy with warnings denied. The package rebuild is byte-identical. Each new original was normalized again in a fresh CLI process; its complete semantic draft and sidecar match after authenticated lineage/draft-digest normalization.

Exactly one saved Import origin per build becomes source-only. Configuration-linked origins change [31,66,22,18,13] to [30,65,21,17,12]. Original05 changes 13 to 12 while both Config-subtree origins remain; Import797 loses its sole configuration link. Whole drafts, allocators, selections, all selected finalization issues and all110 ordered queries are preserved. Selected issue counts remain [123,137,127,144,5]. No issue, native inventory or complete build is closed.

Only normalization.json, tree-normalization.json and release.json change in the package. All numerical rules remain identical. These are import/publication checks; no new source execution, numerical parity or complete-build claim is made.

This record was generated only after the exact logs, installed seven-file candidate, maintained adapter seams, full receipt/artifacts and all-five replay/preservation evidence passed fail-closed checks. It introduces no evaluator or publisher.
