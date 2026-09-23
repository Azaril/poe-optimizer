# Context-conditioned source membership

**Status:** Accepted within the existing owned-import architecture
**Date:** 2026-09-23
**Scope:** Import compatibility data; no native evaluator or Core model change

## Context

A lexical modifier match does not prove one physical source member. PoB may format a value,
return a partial parse and combine a following line. The same text may be skipped when it
duplicates a generated base effect. A known finite number, empty projected properties or a
later numerical cache is insufficient evidence of an independent initial source member.

Source attribution matters independently of the raw semantic value: saved ModRange indices
address source members, while owned modifiers and their parameters address semantic records.
Neither should inherit unproved source indexing or secretly require a live source evaluator.

## Decision

Source-policy v6 extends the existing flags/metadata dialect with data-authored conditional
member declarations. Each declaration targets one existing Unresolved line recipe and is an
all-of list of typed prerequisites. The first vocabulary is NoSourceTags,
NoGeneratedBuffMembers and UnsignedIntegerCapture with raw inclusive integer bounds. There
are no loops, callbacks, arithmetic recipes, implicit item classes or game-name dispatch.
The exact source-policy identity binds the line-policy identity, which binds the schema.
Existing v3–v5 representations and identity domains remain unchanged.

The constructor rejects unknown/duplicate targets, non-Unresolved roles, empty declarations,
non-modifier emissions, duplicate predicates/capture IDs, unknown/non-numeric captures and
inverted bounds. Numeric guards refer to the original captured spelling before codec scaling
or floating-point normalization. They reuse the checked line-pattern matcher through a
crate-private bounded method; a separate regex implementation cannot disagree about spans.

Conditional success also requires the ordinary probe to be valid. A guarded failure creates
UnprovedMemberConditions on the physical line and preserves PossibleCombinedLine for its
follower. V6 also blocks every other non-header line without a proven source-member role,
independently of whether its recipe requests property inputs; direct raw conversion remains
a separate API. Earlier policy versions retain their behavior. Raw and stripped probes share
the original physical context; a failed stripped
condition cannot be overridden by a different legacy raw-text rule. Missing or malformed
context withholds proof. The selected-template fact is available only when its header was
recognized at its structural position and exactly one template is present.

Policy condition work shares the constructor budget with metadata/default validation and is
cached for tighter encode checks. Rule/capture text, nested conditions, lookups, capture
rematching, whitespace scans and raw digit parsing are charged before work or allocation.
No new per-evaluation state, process, source AST or numerical operation enters the evaluator.

## Options considered

| Option | Consequence |
| --- | --- |
| Unconditional SingleModifier after lexical match | Smallest representation, contradicted by formatting, catalyst and generated-member cases. |
| Replay source formatting and parser/UI state | Large compatibility engine, couples native progress to source implementation details. |
| Explicit bounded source prerequisites | Reuses owned conversion and records a finite reviewed proof; unsupported contexts remain explicit and can be extended independently. |

The selected option adds a small import-only vocabulary. Future predicates require their own
bounded native semantics and independent source evidence. Do not turn this vocabulary into
another general source interpreter. Source declarations remain authored assertions whose
soundness is tested against pinned source; type validation alone does not authenticate them.

## First consumer and limits

The shipped fixed-cold proof requires no physical tags, no generated base prefix members and
an unsigned integer capture 0..1,000,000 in a grammar with literal '+'. The source's empty
modTags path establishes initial catalyst scalar 1 independently of item catalyst headers.
Existing advanced-control, rune, source-order and range-overlay blockers remain necessary.
Generated-prefix exclusion handles the supplied Sapphire Charm +25 line being skipped.

Initial complete parsing, source membership, final numerical eligibility, affix legality
and whole-build coverage are separate contracts. The reference harness executes complete
ParseRaw, including nested BuildModList, and distinguishes observations from those stages.
The guard makes no claim about subsequent magnitude transforms or final item contributions.
Caller-authored owned builds bypass source compatibility entirely.

## Follow-up

The source audit also falsifies unconditional roles for bare resistance/Life/critical text,
positive-sign ranges that resolve to zero, huge finite range endpoints and several negative
catalyst contexts. Migrate these current roles through separately reviewed successor data;
do not rewrite persisted declarations or declare the audit complete after the cold consumer.
Preserve every original query and report raw admissions separately from complete builds.
