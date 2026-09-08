# Build fixtures

`pobarchives-Dfz36mCq.xml` is the exact decoded byte stream from the user-supplied
original `example.import.txt` (now preserved as `pobarchives-Dfz36mCq.import.txt`). Its adjacent metadata JSON records source hashes, structural
observations, source assumptions, and compatibility limits. The original
`example.build` was left unchanged. On 2026-09-08 the user replaced `example.import.txt`
with five imports; the exact decoded corpus and intake index are in `breadth-20260908/`.
See [breadth validation](../../../docs/breadth-validation.md).

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


`spark-receiving-defence.xml` and `mace-receiving-defence.xml` are separate synthetic
receiving-stage fixtures derived from the passive/equipment study fixtures. They combine
connected source passives, Lunar/Pearlescent amulets, reviewed global item modifiers and
authored configuration. Their full outputs are compared with fresh native/PoB evaluations
and source-preserving export reimports; they are not independent saved numerical goldens
or assertions that the supplied rare rolls are obtainable. See
[receiving defences](../../../docs/receiving-defences.md).
