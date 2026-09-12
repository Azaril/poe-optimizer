# Rule execution model investigation

Status: **A1 in progress; no replacement or migration selected.**

The loader, parser and source interpreter now carry substantial complexity. Investigate
whether a more focused execution model can reduce the total cost of achieving and maintaining
full PoB parity. This is an architecture decision to investigate, not authorization to replace
the current evaluator or reduce its correctness objective.

Continue A1 before generalizing further unrelated Lua features. The investigation does not
require completion of the current evaluator first. Preserve the current measured checkpoint
and bounded correctness work while choosing representative prototypes with the owner.
The [implementation record](implementation.md) tracks that checkpoint and active work.
The initial [baseline](execution-model-baseline.md),
[semantic inventory](execution-model-semantics-inventory.md) and
[performance inventory](execution-model-performance-inventory.md) are recorded. A1 remains
open for attribution, general candidate invalidation, source-state and update-effort measurements;
restricted-profile throughput is not full-build parity or an architecture comparison.
Bounded [original modifier observations](execution-model-baseline.md#original-modifier-observations-for-the-comparison)
now make one comparison input executable across all five originals. They preserve the
separation between source facts, isolated queries and final actor/build behavior. They
neither select a model nor complete A2.
The [loader phase measurements](execution-model-baseline.md#attribution-inside-dataset-loading)
also separate bulk representation/proof costs from catalog construction, without adding
instrumentation to the shipping loader. A subsequent
[trusted-key simplification](execution-model-baseline.md#trusted-passive-key-loader-simplification)
reduces measured first-load time while preserving loaded data and validation. This is a bounded
optimization of the current model. A [supported balance-data replay](execution-model-baseline.md#supported-data-update-replay)
also preserves search/export behavior with a fixed native executable.
[Identity reconciliation](execution-model-baseline.md#identity-reconciliation-replay) and
[pinned-source acquisition](execution-model-baseline.md#pinned-source-acquisition-and-rejection)
now record coordinated reference edits, reproducible output, allowed traversal variation and
source-edit rejection. [Dataset allocation measurements](execution-model-baseline.md#dataset-allocation-and-ownership)
now distinguish owned snapshot copies from shared handles and catalog retention.
[Source-session lifecycle measurements](execution-model-baseline.md#source-session-lifecycle-measurements)
separate compilation, private import, clone/restart/drop and finite invocation histories
after source-host destruction. [Native allocation provenance](execution-model-baseline.md#native-allocation-origin-checkpoint)
also identifies the actual expression behind the remaining copy failure without admitting
its layout. [Requested private-session layouts](execution-model-baseline.md#requested-allocation-layouts-for-private-sessions)
now quantify copy/import/teardown storage separately from shared handles. The
[original constructor/producer join](execution-model-baseline.md#original-constructor-and-producer-checkpoint)
also closes the missing source identity; reserved-key mutation and tail behavior remain open
compatibility costs. The [R2w tail-call evidence](execution-model-baseline.md#original-parser-tail-call-checkpoint)
now establishes actual empty packs for those records, without admitting traversal. A bounded
[staged-import optimization](execution-model-baseline.md#transferring-staged-private-import-storage)
reduces measured allocation traffic while preserving the same source histories and logical
import budgets; retained session storage is unchanged. Per-stage loader/source allocation,
general invalidation, new effect-family work and actual upstream update effort remain open.
No model is selected.

## Separate the decisions

Rust is an implementation language, not a semantic model. A DSL interpreter, generated rule
graph and directly coded algorithms can all run in Rust; each exposes different extension,
validation and update contracts. Likewise, data-driven execution still requires an explicit
model for ordering, conditions, state, dependencies and interactions.

Separate acquisition from execution. PoB may supply authenticated source, extracted data and
parity evidence during offline preparation or upstream updates without being a production
runtime dependency. Keeping a switchable PoB reference backend does not require invoking it
inside native candidate evaluation. Account for both paths when comparing complexity.

The existing [shared programs](shared-source-programs.md) and
[parser session design](parser-sessions.md) describe the current approach. Their known state
and identity requirements are evidence for this investigation, not a predetermined winner.

## Candidate models

| Model | What executes | Questions the prototype must answer |
| --- | --- | --- |
| Retained interpreter | Validated source programs and injected definitions on the current native runtime. | Can a narrower supported language, clearer boundaries or simpler acquisition remove enough complexity? What ongoing VM compatibility remains necessary? |
| Domain DSL or rule graph | Explicit domain operations, conditions, dependencies and effects represented as validated rules. | Can the model express actual interaction families without hidden callbacks or an expanding escape language? How are evaluation order and explanations represented? |
| Native Rust algorithm families | Generic compiled algorithms consume injected game definitions and build instances. | Which changes remain data-only, which require new code, and how are new effect families added without per-skill, per-item or per-build handlers? |
| Hybrid or offline lowering | Source or domain rules become a smaller runtime representation, possibly combined with native kernels. | Is complexity removed or merely moved into generators, adapters and duplicated representations? Can generated behavior retain provenance and useful failures? |

Compare coherent combinations where useful. No model is preferred merely because it has
fewer runtime abstractions or more generated code.

## Behavior and integration boundaries

Full parity covers observable game and build behavior: imported meaning, selected skills,
configuration, conditional effects, modifier composition, numerical outputs and supported
failures. It also includes state history, returned structure, aliases or ordering wherever
these affect later consumers. An internal detail cannot be discarded just by calling it an
implementation detail. Propose a boundary, identify every relevant consumer and demonstrate
that the alternative preserves their behavior; document uncertainty as unsupported.

Conversely, reproducing an interpreter mechanism is not itself the product objective.
Allocator layout, instruction choices and internal cache machinery need equivalent behavior
only where they influence the agreed observable contract. Any narrower contract needs an
explicit rationale and parity evidence, including adversarial interaction and history cases.
The existing parser's exposed cache and mutable dictionaries make this distinction material.

Every candidate must preserve these constraints:

- Game values and extensible definitions remain injected; no hardcoded builds or special-case
  numeric answers. Document the boundary between new data and a genuinely new algorithm family.
- Shared immutable definitions and compiled work remain separate from private build/session
  state, including candidate isolation and reusable preparation.
- Native execution stays in process, supports Rayon candidate parallelism and portable WASM
  deployment; document any host-specific acquisition tooling separately.
- A switchable reference backend and common comparison boundary remain available for parity,
  upstream updates and regression diagnosis, without making PoB a native runtime dependency.
- Dataset formats, provenance, ownership and validation have explicit compatibility and
  migration paths. Unknown or unsupported behavior cannot silently become a default result.

## Evidence gates

### A1 — Inventory and cost baseline

Map the current acquisition, parser, preparation, resolution and evaluation paths, including
adapters, generators, schemas, diagnostics, tests and manual work. Identify complexity caused
by domain behavior versus fidelity to the source runtime. Record the current reached failures
and missing behavior without counting partial gates as completed evaluations.

Use all five supplied originals as the breadth baseline, then inventory interaction families
that they do not cover. Include import variants, configuration changes, conditions, skill and
support relationships, equipment and unique effects, triggers, minions, damage conversions,
resource behavior and combinations across these families. Choose meaningful vertical slices;
five successful builds alone would not establish the full interaction contract.

Measure cold acquisition/preparation separately from hot candidate evaluation. Record reuse,
throughput and scaling with candidate count and worker count, memory and state-copy costs.
Also record upstream update effort, data-only update capability, failure localization,
explainability and dataset migration work. Establish measurements before proposing targets.

### A2 — Comparable prototypes

Prototype selected models against the same injected definitions, input states, lifecycle and
observable outputs. Include parser/preparation state and a downstream interaction slice, not
only a numeric kernel. Exercise positive, no-match, failure and repeated/history-sensitive
cases against the original reference, with cold and reused preparation measured separately.

Keep prototypes bounded but expose missing mechanisms and all fallback costs. Include source
acquisition, generated artifacts, validation, test maintenance and debugging in the comparison.
Do not require full evaluator completion as a prerequisite, or treat a prototype's limited
coverage as permission to shrink the full parity goal.

### A3 — Evidence and architecture discussion

Present results, unresolved parity risks and total maintenance costs in an ADR discussion
with the owner. Explain the proposed semantic boundary, data/code extension model and how the
remaining interaction families would fit. Set acceptance criteria from the evidence; do not
preselect numerical targets or a winning architecture. Record the decision and its reasons.

### A4 — Approved migration, or retain and simplify

If a change is approved, migrate in reviewable stages behind the common backend boundary,
with dataset migration/replay, differential validation and a rollback path. Keep existing
numerical and interaction coverage throughout. If the evidence favors the current model,
record that decision and implement the justified simplifications instead. Either outcome
must reduce understood costs while preserving the full parity and breadth objective.
