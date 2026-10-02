# Nonphysical saved skill rows and physical support inventory

A saved PoB Gem element can describe a generated effect without owning a physical
gem. The physical support census must distinguish those rows from physical active
gems, physical supports, and unresolved source rows. A source element name alone
cannot establish its role.

The V3 Import policy retains the V2 physical-Gem census and adds an injected list
of exact nonphysical skill IDs, bound to the source identity and reviewed role
catalog. Each admitted ID declares a reviewed source effect that exists, has no
physical gemForSkill mapping, and is not a support. The first finite declaration
is EnemyExplode. This is source-adapter evidence, not an owned Gem definition or
native effect implementation.

The adapter requires the gemId attribute to be absent and the decoded skillId to
match exactly. An authored empty or invalid gemId is still present and takes the
source loader's earlier branch; it must not be treated as absence. Source frame
validation, exact source/catalog bindings, bounded selectors and work budgets
remain required. Unknown IDs, unsupported frames or contradictory physical
Gem/Support links cannot supply the nonphysical proof.

The census visits every saved child independently. PoB may reuse an existing
Explode group and wipe/rebuild its contents from actual item providers. That does
not erase a separately saved physical support from the import inventory. A reused
group also does not acquire noSupports from the statement that creates a new
group. The inventory proof therefore never uses source=Explode or noSupports as a
blanket reason to ignore children.

Only physical support inventory can become complete. The original source row,
existing source links, instance IDs, allocator state and unresolved effect,
provider, input, activation and usage facts remain intact. No definitions are
allocated and no owned SkillUse, Gem or SupportAssignment is invented. Existing
V1/V2 behavior and wire formats remain unchanged; V3 is opt-in.

The checked publication requires fresh complete-source evidence in both JIT modes
and exact input hashes. Its five-original comparison permits only the separately
proved support-inventory obligation to retire, preserves every other draft and
sidecar fact, and retains all 110 requested queries. The source witness passes
23 cases / 25 complete load attempts per JIT mode, including all five originals.
Both 8,040,443-byte evidence files have SHA256
`57f4fd76bb6faab3510eb02f44a7f7b646a3eca9cb4454b10148803aa89b631e`.
The authored receipt records that proof separately from whole-build parity.

The source witness retains the different internal call histories of fresh and
warm loads. Reloading uses fewer preparation passes; those counts are not semantic
state. Every observed effect-processing call must leave actual support targets
unchanged, while loaded/final groups, providers, selections, items and output
values must match exactly across equivalent fresh and warm cases.

Checked publication in `runs/owned-nonphysical-skill-inventory-02` retires only
Original03's selected issue0085. Its 48 assignments and ten unresolved targets
remain unchanged. Selected counts are 116/116/108/121/19 and complete native
originals remain 0/5. Ten real CLI selector controls pass against the exact witness
XML. The default work limit is preserved by reusing already reviewed physical
selectors; the census only reads an additional skill ID for eligible rows.
