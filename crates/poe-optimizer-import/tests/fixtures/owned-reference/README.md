# Exact recorded reference fixtures

These five gzip files contain the unchanged full CLI reports originally retained under `runs/number-factories-corpus/line-00001` through `line-00005`. They exercise the actual recorded-report validator and projection producer. They do not establish owned numerical, availability or whole-build parity.

`manifest.json` records each exact raw report SHA-256/byte count and source XML SHA-256, plus the gzip SHA-256/byte count. Compression uses level 9, an empty filename, and `mtime = 0`. The raw reports total 2,306,993 bytes; gzip files plus manifest total 226,423 bytes. They retain all 110 measurement rows. Six rows have no selected-minion target; the other unavailable measurements must not be treated as absent targets.

Only `expectations_sha256` uses **LF-normalized bytes** (`CRLF` replaced by `LF`), as explicitly recorded by `expectations_digest_format`. This permits the existing text expectation manifest to keep its repository checkout behavior. Raw reports and source XML hashes are never normalized. `tests/support/owned_reference_fixture.rs` bounds compressed reads and decompression, then verifies all corresponding hashes and sizes before passing full report bytes to the producer.

From the repository root, reproduce into a new directory, or verify the committed files with:

```text
python scripts/export-owned-reference-fixtures.py --expectations tests/fixtures/breadth-expectations/originals-v1.json --corpus runs/number-factories-corpus --output crates/poe-optimizer-import/tests/fixtures/owned-reference --check
```

The exporter reads existing evidence only. It verifies the original source/report bindings and fixed comparison before packaging; it does not run PoB or any evaluator. Existing reports were inspected before inclusion: no unrelated credential, account, user-home-path or machine-specific secret was identified. Runtime platform/version fingerprints and user build/source information remain as recorded evidence.
