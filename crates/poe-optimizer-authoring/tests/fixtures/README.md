# Retained rule migration reference

`retained-rules.json.gz` is the immutable rule IR captured before the tracked DSL
cutover. It is regression evidence, never an input to `poe-author release build`
or to runtime evaluation. Its decoded bytes are authenticated against the
historical receipt in
`data/owned/poe2/3887ae68/migration-source/reference-release.json`.

- Decoded length: 17,488,370 bytes.
- Decoded SHA-256: `6ebd514b3c9e57a41a025426d9bf3c80f1fc64749fa665c9e8f82ebcf41b1366`.
- Original input identity: `a75ef471b57d8184b208ca5f2a26ce6a9a01d15340db8b15fc1f318be40c1905`.

The corpus tests independently compare each owner-qualified program graph with
the tracked DSL, allowing only node renaming. The release test restores only
those historical program members and requires the historical artifact receipt
to match exactly. This also authenticates the unchanged non-program metadata.
Do not regenerate this reference merely to make a migration test pass.
