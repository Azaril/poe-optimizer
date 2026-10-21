# Generated skill field-accounting checkpoint

Historical checkpoint at commit `6de7a46f`. The current packet is maintained in
[the same authoring family](../../../data/owned/poe2/3887ae68/generated-firebolt-dispositions/README.md);
subsequent changes do not rewrite this checkpoint's validation receipt.

Published input `a8317fccf1cdf6bad38be2308e5ac69f7d32695d6439b7afb169d42b6c9fceca` connects thirteen existing intrinsic-input receipts for the reviewed Firebolt source and removes two redundant reporting Deferred destinations. It retains reporting policy 338e / parameter 338f and all other consumers. Six Deferred fields remain for the live source; nineteen remain for the unresolved alternative. No production Rust API, evaluator, numerical rule or complete-build claim changes.

Validation passed: nine Import tests, 9 ordinary CLI tests (7 explicit publication tests ignored), one all-five publication test, and strict Import/CLI Clippy. Synthetic controls exercise private receipt consumption. Real-build checks preserve the actual persisted provider, raw inputs, reporting false value and source links; they do not directly observe private receipts from those imports.

All five complete checked outputs, allocators, selected issues and 110 query rows are preserved. Independent fresh CLI replays agree; all eighteen package files rebuild byte-identically. Only normalization.json, tree-normalization.json and release.json change. Selected issue counts remain 123/137/127/144/5; Original05 remains closest, with 0/5 builds complete.

[validation.json](validation.json) retains the exact files, logs, complete release receipt and publication result. The first publication attempt caught an expected-snapshot representation error (integer JSON zero instead of declared f64 zero). At this checkpoint the test read the expected records through their Rust types, with strict lineage and value checks; live comparisons and sealed authoring data were unchanged. The failed log is retained. The subsequent Rust authoring pass writes those typed values directly and removes that snapshot-repair code. No new PoB execution was needed for this historical data change.
