# Owned metric evaluation boundary

The native metric path consumes a complete owned request and injected definitions,
rules, action routes and metric mappings. It calls no PoB parser, UI, source interpreter
or subprocess. CLI, future desktop/web applications and search can use this same library
boundary. It does not yet replace the legacy numerical/search backend or complete a supplied
original build. The [implementation record](implementation.md) owns current validation.

## Contracts

Core's `MetricMappingInput` declares `(MetricDefId, PlayerActor | OwnedActor | Action)`
to final `StatDefId` relationships. Data's immutable `OwnedMetricMapping` validates and
canonicalizes that artifact against an exact definition-schema identity. It rejects stale
bindings, duplicate keys, incompatible target/actor roles and any result type except a
Quantity with the metric's identical unit ID. A shared dimension is not a unit conversion.
Concrete action/provider roles remain checked by the existing owned request binder.

The mapping expresses game semantics, not a PoB output field, fixture name or reference
row index. Missing entries remain unknown. A new metric can use a data binding without a
new named Rust result field; a genuinely new calculation still needs supported rule
semantics. Reference projection and expected values remain separate comparison artifacts.

Engine's `OwnedMetricPlan::compile` accepts an immutable `OwnedEffectPlan` and the mapping.
It binds each requested final stat and target readiness once. Evaluation uses the same
effect executor and per-worker `OwnedPlanScratch`, then reads prebound final indices.
It does not allocate a diagnostic report containing every effect on the metric path.
The ordered result rows preserve query IDs and exact actor/action/provider instances,
including repeated requests for the same metric. A deserialized report cannot be fed
back as private-plan execution authority.

| Result | Meaning |
| --- | --- |
| Known Quantity | An active requested target has a final value in the exact declared unit and the current plan's contributor closure is complete. |
| Inactive | The selected provider/actor/action is disabled by established activation. |
| Unresolved | Missing mapping, producer, required input, topology or contributor closure; the reason remains explicit. |
| UnsupportedDomain / UnsupportedValue | An implemented component cannot accept the demanded value; preserve its typed cause. |
| NumericalError | The demanded calculation failed; preserve the original operation and failure. |

A parent may project a known diagnostic stat into an actor whose grant is false. Query
readiness separately checks that actor's grant, provider ancestors and required generated
skill inputs. It must not publish the projection as an available actor metric. An unused
RequiredOnce generated input still gates a constant action result. Siblings and multiple
uses of the same gem/item retain distinct occurrence identities.

The current complete-request and **whole-plan contributor closure** gates remain in force.
This does not implement partial-draft or per-metric dependency coverage. Legality, requested
measurement availability, numerical agreement and whole-build parity remain separate.
No missing quantity becomes zero, and no metric result is proof of gameplay legality.

## Host API

```text
evaluate-owned --input REQUEST --schema SCHEMA --rules RULES --routing ROUTES --metrics METRICS [--output NEW_REPORT]
```

The CLI shares owned file loading with `resolve-owned-effects`. It validates before
publishing output, preserves an existing output file, and labels legality `not_checked`
and whole-build parity `not_established`. It works with directly authored owned artifacts
from a directory containing no PoB files. The existing `evaluate --backend native` command
still uses the legacy profile backend; these are explicit different experimental commands.

Mapping format is version 1; metric plan identity uses `owned-metric-plan-v1` and binds
both the effect plan and mapping. Effect-plan identity is now `owned-effect-plan-v4`, extending the v3 prebound query
activation contract with explicit stat receiver instantiation. Worker scratch can be reused after success or error and
across plans; immutable plans can be shared across native workers. This is not evidence of
throughput for admitted whole builds or reusable bindings after candidate mutation.

## Remaining integration

Real data needs complete selected item/passive/reward/scenario contributions and shared
receiver rules before it can declare a final resistance stat and map the fixed original
query. The [stat-owned actor receivers](owned-stat-receivers.md) give common
formulas explicit semantic ownership and player/owned-actor applicability. They are
implemented and retain the existing whole-plan coverage requirement; no fabricated usage
policy or per-class receiver is a substitute.
The [resistance data slice](../data/owned/poe2/3887ae68/resistance/README.md) intentionally
contains contributions only. Full original import finalization, complete mechanics,
backend-neutral search adoption and legacy retirement remain separate milestones.
