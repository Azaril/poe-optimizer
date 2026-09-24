# Multi-effect physical support Gem input conversion

`policy.json` explicitly opts into `resolved_potential_skills_v1`. It selects all 52
physical multi-effect support Gems with resolved effect references and no additional
stat-set declarations in the pinned identity catalog. The family has 105 potential
Skill memberships: 51 Gems have two effects and Empowered Sparks has three. Selection
uses source identity and topology, independent of the five supplied builds.

The compiler joins every effect through exact external-to-owned mappings. Declared
additional references must be included in constructed references; constructed and
resolved additional sets must agree, and their union with the primary must equal the
final effect list. Missing/ambiguous references, repeated effects, incompatible role
metadata and stat-set declarations reject. Constructed-only Barbs effects are retained.
Display order does not determine primary identity: Empowered Sparks places its primary
last. Concussive Runes has an unresolved declared effect and remains excluded.

The physical Gem's primary determines its authored role. Its potential Skills are
canonical Partial membership, not extra physical Gems, activated SkillUses or providers.
The shared physical level domain is 1; additional active effects can have larger level
tables and inherit a supported skill's level during reference setup. Those derived
levels, support applicability, activation and provider choices are separate obligations.

The two injected parameters retain independent Boolean corruption and numeric Count
corruption-delta inputs. The numeric recipe explicitly aliases selected `nil` to `0`;
missing/unavailable and unlisted malformed text remain Pending. Every generated input,
quality and potential-effect membership remains Partial. This policy establishes input
knowledge, not complete support behavior or whole-build evaluation parity.

## Reproduce the owned package

The source-free Rust compiler prepares a V4 schema migration and successor-bound full
normalization policy. Keep the actual predecessor and intermediate schema publications;
all output directories must be new. Old definitions, programs, queries and input rules
are preserved apart from required schema identity bindings.

```text
poe-optimizer compile-owned-gem-inputs runs/owned-numeric-aliases-01/package --catalog data/owned/poe2/3887ae68/import/skill-identities.json --policy data/owned/poe2/3887ae68/multieffect-support-gem-inputs/policy.json --output runs/owned-multieffect-support-inputs-01/compiled
poe-optimizer migrate-owned-gem-schemas runs/owned-numeric-aliases-01/package --migration runs/owned-multieffect-support-inputs-01/compiled/migration.json --output runs/owned-multieffect-support-inputs-01/schema
poe-optimizer publish-owned-normalization runs/owned-multieffect-support-inputs-01/schema --normalization runs/owned-multieffect-support-inputs-01/compiled/normalization.json --output runs/owned-multieffect-support-inputs-01/package
```

See the [Gem input contract](../../../../../docs/owned-gem-inputs.md) and
[living implementation checkpoint](../../../../../docs/implementation.md) for measured
validation and remaining coverage.
