# Final constructed skill-definition facts

The optional PoB adapter captures every entry in the final `data.skills` table
after the pinned static skill/gem constructor finishes. The catalog contains
1,436 exact source IDs: 1,430 absent `cannotBeSupported` fields and six explicit
true values. No current definition contains explicit false; that state is
preserved and tested independently.

`catalog.json` uses `cannot_be_supported: null` for an absent source field.
This observation alone does not authorize a native false producer. The reviewed
Boolean decoder, exporter and support consumer must establish that interpretation
before a KDL/DSL mechanic is admitted. Occurrence-specific `noSupports` remains a
separate source-use setting.

The catalog is 127,207 bytes, SHA-256
`98b3af560aa998fcc2f7cf4d2e9aeeff8fe909e67a0f8543ab96e6f77c41ad8b`.
`evidence.json` records the pinned revision, source-manifest commitment, all
sixteen admitted constructor inputs, actual module order, extractor fingerprint
and output counts. Capture first verifies the complete pinned checkout and then
rechecks the sources it reads. Invalid types, metatables, missing/mismatched row
identities and resource-limit violations fail acquisition.

Reproduce into a new directory:

```powershell
cargo run --locked --features pob --bin poe-optimizer -- export-owned-skill-definition-facts --source-root vendor/path-of-building-poe2 --output runs/skill-definition-facts-review
```

The command atomically publishes `catalog.json` and `evidence.json`, refuses an
existing destination and prints the evidence on stdout. Fresh captures must agree
exactly. Acquisition uses the existing bounded `mlua` static-data constructor in
process, with JIT disabled for its instruction budget. It loads no build or UI and
adds no dependency to the native candidate-evaluation path.

This artifact supplies offline review evidence. It adds no native Stat, producer,
coverage declaration or complete-build admission. See the
[support-prohibition proof](../support-prohibition/README.md) for the separate
reviewed native declaration and producer, and the
[implementation plan](../../../../../docs/implementation.md) for remaining work.
