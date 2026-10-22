# Documentation

Start here. The October 22 architecture review is adopted; **the migration is
planned, not implemented**. No supplied build yet completes native evaluation.

| Document | Authority |
| --- | --- |
| [Product design](design.md) | User workflows, optimization scope and acceptance criteria |
| [Domain architecture](domain-architecture.md) | The single target architecture: DSL, data, model, evaluator and boundaries |
| [Implementation plan](implementation.md) | Current state, ordered phases, validation gates and the next session's resume point |
| [Execution overview](data-and-evaluation-overview.md) | A short explanation of what runs today and what changes |
| [Migration map](architecture-migration.md) | Where existing responsibilities move; no separate delivery schedule |
| [Retirement register](legacy-retirement.md) | Deletion requirements and retained semantic investigations |
| [Owner-provided review](architecture-review-2026-10-22.md) | Unmodified review evidence; the architecture and plan record accepted adjustments |

The architecture defines the end state, not a sequence of historical checkpoints.
The implementation plan is the only delivery schedule. Detailed `owned-*.md`
contracts retain useful mechanic/ownership evidence, but old wire versions,
receipt protocols and source-shaped implementations are not compatibility
requirements. Where they conflict, the October 22 architecture governs.

Useful domain details include [build composition](owned-build-contract.md),
[binding](owned-binding.md), [rules](owned-rules.md), [metrics](owned-metrics.md),
[generality review](build-generality-review.md) and
[breadth validation](breadth-validation.md). Read these for the task at hand,
not as additional current plans.

Historical checkpoint prose is available in [implementation history](implementation-history.md)
and the [pre-migration documentation at fa49fcea](https://github.com/Azaril/poe-optimizer/tree/fa49fcea32f0413cde5f5983e179c34e6c7acbdf/docs).
It is evidence, not active instruction. Do not copy it into a new plan or load it
by default when resuming work.
