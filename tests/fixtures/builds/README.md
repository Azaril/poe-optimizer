# Build fixtures

`pobarchives-Dfz36mCq.xml` is the exact decoded byte stream from the user-supplied
`example.import.txt`. Its adjacent metadata JSON records source hashes, structural
observations, source assumptions, and compatibility limits. The original
`example.build` and `example.import.txt` were read only and left unchanged.

This is a Path of Exile 2 level 96 Sorceress / Disciple of Varashta minion army
build, with tree version `0_5`; its planner title claims patch 0.5.5. The selected
main group is Kelari, the Tainted Sands. A successful decode is not evaluator
parity. The embedded mlua evaluator now produces fresh outputs and passes fresh-worker
A/B/A and export/reimport consistency tests; an independent reference is still needed.
See the [implementation record](../../../docs/implementation.md). In particular, all Full DPS
group flags are the literal string `nil`, and three named skill entries have no
stable IDs. Keep these source ambiguities visible instead of repairing the raw
fixture or accepting cached metrics as an oracle.

The XML is untrusted imported data. The importer bounds decoding and XML
size, rejects DTD/entity expansion, and preserves unknown fields. Never execute
content in a build export. Derived mutation/calibration fixtures must use separate
files and provenance; do not edit this raw reference in place.
