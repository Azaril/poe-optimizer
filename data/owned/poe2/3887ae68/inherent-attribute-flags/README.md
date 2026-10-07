# Inherent attribute flag contributions

`rules.json` contains ordinary owned rule data: two passive contribution programs,
five Boolean query definitions, five final flag reducers and their Player
receivers. The existing Strength-Life program consumes the same five Stats;
no final flag values are injected into production evaluation.

The authored sources are Giant's Blood (allocation 32349, owned `10ac`) and
Enhanced Effectiveness (58591, owned `18d9`). `source.json` records the exact
mapping, parsed modifier meaning and pinned source files. The new programs are
appended to their existing Partial program inventories. Wielding and attribute
requirements, and Enhanced Effectiveness's attribute penalty, remain separate
unimplemented mechanics.

Every query group remains Partial. In particular, the three groups without a
published producer are not proved empty. Item, transformed-passive and other
flag sources are still required; a true known source cannot conceal that gap.
Irongrasp's duplicate suppression sources are a planned acquisition contrast,
not part of this publication.

The native Rust test `owned_inherent_attribute_flags` uses these exact programs,
queries and the existing Strength-Life receiver in an explicitly finite test
domain. It supplies final Strength as a fixture input and closes only that test
domain. It exercises both passive selections, their combination and absence,
plus refusal with the actual Partial inventories. It does not calculate final
Strength, final Life, or a complete build.

The development-format cutover uses rule schema 3, operations 22 and new rule
storage/compiler/effect-plan hash domains. It has one current loader. The
publication test reads historical bytes only to authenticate and check the
explicit inverse; no historical DTO parser is added to application code.
