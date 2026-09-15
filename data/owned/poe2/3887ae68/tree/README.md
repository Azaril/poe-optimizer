# Offline passive tree input

`source-manifest.json` pins the finite tree JSON and the two source files reviewed for tree/spec semantics. `scripts/export-owned-tree.py` reads the JSON and checks LF-normalized source hashes. It executes no Lua, reads no character build and allocates no owned IDs. Source formats and field names stop at this tool and the Import crate.

`tree-catalog.json` is a bounded conversion input, not a runtime rules package. It retains 4914 classified source rows, 5,187 nonvisual undirected edges, 14 unresolved endpoint links, class/ascendancy identities, shared physical roots, attached options and explicit attribute lanes. One source self-connection at 35653 is excluded from graph adjacency and recorded in `source-facts.json`. Class-dependent stat text and unlock lists are evidence for future semantic conversion; no source stat text is executable authority.

`catalog-policy.json` supplies the supported level range and exact saved-build syntax, including independent weapon overlays and attribute override names. The Rust compiler consumes these injected inputs over an existing owned ledger, emits owned definition schemas and explicit partial effect owners, and produces a separately bound tree normalization policy. The shared successor finalizer installs it alongside all other import policies in the [current package](../current/README.md). No arbitrary extra artifact or parallel identity ledger is permitted.

Regenerate only the finite exported input with:

```powershell
python scripts/export-owned-tree.py `
  --manifest data/owned/poe2/3887ae68/tree/source-manifest.json `
  --source-root vendor/path-of-building-poe2 `
  --output-dir runs/tree-export
```

Use `--check-dir data/owned/poe2/3887ae68/tree` instead of `--output-dir` for exact verification. Input/acquisition/output, row, expanded-membership and string budgets apply; output encoding stops at its limit. Unknown source fields or ambiguous roles fail conversion. Review source changes and revise the converter/policy explicitly rather than executing source code as a fallback.

This checkpoint proves structural conversion. Costs/capacities, reachability, radius grants, conditional views and numerical contributions remain incomplete; the five original builds retain their full query manifests and pending status.
