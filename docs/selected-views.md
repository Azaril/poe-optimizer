# Selected build views

Status: R1b source-selection component implemented. This does not admit a complete
native build or execute the full PoB load/preparation lifecycle. The
[R1c native integration](native-preparation.md) now consumes this boundary; complete
effective producers and numerical breadth remain R2/R3.

## Caller and ownership boundary

`poe_optimizer_core::build_view::ViewRequest` carries a bounded caller label and four
independent typed requests: saved selection or a stable skill-set, item-set, passive-spec
or configuration-set instance. Weapon state is separately saved, primary or secondary.
These portable values contain no XML, Lua, database connection or native plan handle.

`poe_optimizer_import::selected_view::resolve_view` consumes an immutable
`ImportedBuildInstance`, an injected `GameDataSnapshot`, the request and resource limits.
The returned `SelectedView` retains the exact imported owner and borrows the definition
owner. `validate_binding` accepts a clone sharing that imported storage and rejects an
independently imported identical document or another definition snapshot, even when
lineage labels, bytes and digests match. Its report is serialization-only, not an
admission token. Source/data/revision identity and resolver semantics are recorded.

An explicit instance request must belong to the source owner and survive the final
collection's lookup rules. A duplicate-key occurrence that has been overwritten cannot
silently retarget the winning occurrence. A set from an earlier replaced root container
is unavailable. A valid explicit passive choice may replace an invalid saved position;
the saved request and its failure remain evidence. An override cannot bypass a reached
source-structural loader failure.

## Domain rules and preserved evidence

| Domain | Lookup and generated-ID behavior |
| --- | --- |
| Skills | Numeric keys; fallback to the first ordered key. Missing IDs use the original insert-only LuaJIT table length plus one. Legacy groups target key1 and can fail if it is missing after another saved set. |
| Items | Numeric keys; fallback to the first ordered key. Missing IDs use the first unused positive integer. Source slot uses remain separate instances. |
| Configuration | Numeric keys; fallback to the first ordered key. XML array positions, holes, duplicate keys and legacy-created key1 are preserved. A missing authored ConfigSet ID creates an ID, then the original loader dereferences the original nil key and fails. |
| Passives | One-based source positions, independent of authored Spec IDs. Only the upper bound is clamped. Zero, negative and fractional positions can fail; non-finite inputs retain their bits. |

Reports preserve raw attributes, parsed/requested IEEE numbers, ordering keys, every
created set occurrence, the selected origin, requested override and the applicable rule.
Numbers serialize as sixteen hexadecimal IEEE-754 bits, preserving negative zero and
non-finite values. Set lookup normalizes numeric equality without rewriting the source.
Default sets have an explicit derived origin; they do not manufacture authored instance IDs.
Unknown namespace contexts produce diagnostics rather than disappearing into an empty set.

Repeated ordinary containers are projected in source order and tree/spec containers after
ordinary sections. Earlier selection problems remain recorded. This is independent domain
selection evidence, not a claim that a full original loader continued past an earlier error.
Full root lifecycle effects and partial-error state remain preparation prerequisites.

Selected skill-entry identity evidence reuses the existing injected definition lookup.
It associates each entry with its authored instance/source and retains unknown, ambiguous,
name-only and external-gem outcomes. A failed external gem lookup does not fall through to
an authored effect ID. No new name matching or skill-specific dispatch is introduced.

## Preparation and performance

The report names outstanding producers: skill processing/grants/supports/actors,
item parsing/base validation/assembly, passive allocation/version rules/jewel switching,
configuration effects and root lifecycle. An item with a matching raw numeric ID is not
assumed to be registered: original registration depends on `ParseRaw`, a valid base and
`BuildModList`. Multiple slot uses therefore remain source references until that producer
resolves them. Effective actions and native metrics are not synthesized from this report.

Set winners use an indexed numeric-key map. Item auto-ID allocation uses membership plus
a monotonic free-key cursor; passive membership traverses each spec subtree. This avoids
cubic missing-ID scans and rescanning the entire XML tree per passive spec. Native
`NumericSetKeys` models fresh, insert-only numeric LuaJIT tables with bounded storage;
it explicitly excludes deletion, literal/preallocated tables, nonnumeric keys and JIT
length hints. It does not allocate arrays proportional to attacker-supplied numeric keys.

Default resolution caps are 32,768 records, 131,072 lexical fragments and 8 MiB of consumed
source/definition text. Copied weapon-state and selected identity text share that budget.
The existing XML/import and definition-projection limits also apply. Identity projection
uses its own bounded temporary lookup before selected records are copied into the report.
Labels and data identity metadata retain their own validated bounds. Limit failures return
errors rather than truncating evidence. Resolution is preparation work: future compiled
plans must keep XML, diagnostic serialization and this source index outside the hot loop.

## CLI

```powershell
cargo run --no-default-features -- inspect-build tests/fixtures/builds/breadth-20260908/build-01.xml --with-view
```

The command expects one build per invocation; use an individual XML/share-code fixture or
extract one line from a multi-build input. `--with-view` includes instance mapping and the
saved view/selected identities. It loads definitions but does not run item inspection unless
`--with-definitions` or an explicit data input also requests that existing route. Existing
inspection output remains unchanged when the new flag is absent.

## Validation and remaining gate

- Native numeric-key tests and 333,614 original LuaJIT observations exercise sparse insertion
  histories, duplicates, signed zero, non-finite/fractional keys and source/resource errors.
- Four-domain source pairing covers all five immutable originals plus malformed selectors,
  duplicate/sparse keys, Config holes, legacy forms and structural skill failures. These
  harnesses execute original selection bodies while declaring their inert producer boundaries.
- Five fresh complete PoB evaluations corroborate the selected keys in actual exported state.
  They use the preserved reference executable whose hash is recorded in the run ledger;
  they are not claims about a newly compiled CLI or complete native calculations.
- Ownership, explicit overrides, namespace/limit errors, selected identity provenance,
  larger generated-ID inputs and CLI behavior have focused tests.

The [implementation record](implementation.md) records exact terminal results and the resume
point. Complete supplied native builds remain **0/5**. R1 requires existing numerical controls
through the shared boundary in R1c; general item reference registration, loader side effects,
effective graph producers and numerical parity must still pass their reached integration gates.
