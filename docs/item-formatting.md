# General item formatting

Item formatting is a reusable native preparation component. It converts an item's authored
numeric ranges and scaling metadata into the exact text consumed by modifier parsing.
It does not select equipment, prove a modifier is implemented, or calculate a build.
The [implementation log](implementation.md) records delivery and validation status.

## Data and execution boundary

The evaluator receives an immutable `GameDataSnapshot`. Complete case-sensitive scalability
keys, ordered per-capture records, raw format labels, partial formatting assignments,
antonyms and fallback/catalyst defaults belong in its versioned data package. Existing
actor precision definitions and item catalyst definitions are shared rather than copied
into formatter code. The extractor must bind every definition and policy to the pinned
original sources and include this section in package compatibility and identity checks.
Caller input never selects a fixture-specific implementation.

The engine owns text scanning, ordered matching, arithmetic and rounding. Its API borrows
the selected catalog and actor data, without filesystem, Lua, subprocess or global mutable
state. This supports concurrent preparation and a native-only or WASM deployment. Imported
items should be prepared once and reused across candidates where their effective inputs
are unchanged. Detailed diagnostic tracing belongs in the import layer, outside the
prepared evaluator's numerical hot loop.

The restricted formatting grammar used by currently admitted numerical profiles remains
separate until a migration has complete differential evidence. Adding a complete identity
catalog does not automatically widen numerical admission.

## Exact source behavior

The reference is the original
[ItemTools.lua](../vendor/path-of-building-poe2/src/Modules/ItemTools.lua) and catalyst logic in
[Item.lua](../vendor/path-of-building-poe2/src/Classes/Item.lua). Retain these distinctions:

- Lookup is exact-case and tries more literal specializations before general numeric keys.
  Normalizing `+#` for lookup must not erase the authored sign from output.
- An absent key, an existing key with no captures, absent format labels and an empty label
  array are distinct. Unknown labels remain no-ops when the source dispatcher ignores them.
- Ordered labels partially assign precision, display precision and optional-decimal state.
  Unspecified fields retain earlier assignments. A label's name does not define behavior.
- Range selection, base scaling, magnitude scaling and display rounding follow the source's
  order. Per-capture scalability gates both scaling operations. Signed values, antonym
  rewrites, negative zero and Lua's numeric text representation are observable output.
- A scalar range, a per-number range array and a missing range retain separate behavior,
  including source errors. Missing array entries use injected policy.
- Catalyst scaling tests the original nonempty tag list before adding prefix/suffix flags.
  A matching catalyst applies once; explicit zero quality differs from missing quality.

The original item loader initially formats at range 1 before parsing. Later assembly
reapplies selected ranges, including authored ModRange changes. Combining these calls or
using the final selection during initial parsing can change modifier classification and
which subsequent source lines are consumed.

## Explicit parser dependency

The legacy fallback can require a modifier parse to discover precision. A pure formatter
must return the exact parser text and a bounded continuation when this happens. Resumption
requires explicit feedback and is bound to the original injected data. There is no hidden
PoB call, guessed empty parse or default successful continuation.

Feedback retains absent versus empty modifier lists, presence of an extra remainder,
original record iteration order and the nested `value.mod` shape. Precision lookup follows
the source's single nested unwrap and last matching record; taking a maximum or sorting
records would alter the result. Source arithmetic errors and resource-limit rejections
remain different from an unavailable parser.

The typed API accepts numeric or missing range/scalar inputs and ordinary metadata tables.
It does not emulate arbitrary Lua metatables or nonnumeric range entries. Those shapes need
an explicit adapter or unsupported result, not an inferred coercion.

The import provider combines this component with explicitly supplied parsing and assembly
implementations. Diagnostic evidence must retain parser requests made inside formatting
separately from the loader's subsequent parse of formatted text, while preserving their
shared execution order. Requests, feedback, continuations and generated strings all consume
bounded resources. Work limits must stop combinatorial literal matching before it can
exhaust memory or CPU, without making the current corpus's largest capture count a rule.

## Identity and validation

Reports bind source XML, selected data and the built-in loading/formatting implementation.
A custom provider's implementation and results require separate host-owned provenance.
Source-only inspection remains available without data or a reference runtime; inspection
with definitions may stop at a specific pending dependency without discarding earlier
source state. Successful formatting does not certify native item or complete-build support.

Independent tests execute the original functions and actual parser, rather than computing
expected outputs through Rust. Compare the entire extracted catalog and policy with the
source runtime, then check exact output strings, errors and parser order across real corpus
items and synthetic combinations. Warmed claims require surviving JIT traces belonging to
the original formatter functions. Item-loading comparisons must cover initial range 1 and
later assembly selections. Full-build numerical goldens, fresh reference evaluations and
native admission checks remain separate gates.

Package upgrades preserve unchanged sections and their digests. Test custom injected keys
and policies through the caller CLI; this demonstrates that production behavior follows
the chosen data rather than the small calibration fixture set. Preserve original caller
files, source pins and independent goldens throughout validation.
