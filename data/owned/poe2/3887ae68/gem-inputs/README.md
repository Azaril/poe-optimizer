# Explicit neutral Gem import policy

`normalization.json` is a complete, explicitly supplied normalization policy for the
package produced by [defensive quality admission](../item-quality-inputs/README.md).
It adds only `gem_inputs`; all prior normalization fields are preserved.

The two existing Known Gem definitions have Complete empty intrinsic parameter declarations.
Their source admission now requires exact `corrupted` text `false` or `nil` and exact
`corruptLevel` text `0`. Every missing, empty, nonzero or unreviewed spelling stays Pending.
No spelling is compiled into the native evaluator. These guards do not define general
PoB parsing semantics or certify support activation, choices or complete build coverage.

The optional Rust source oracle executes complete authenticated LoadSkill/setup methods.
It establishes that the corruption flag and level delta are independent, that active and
support preparation treat the delta differently, and that quality UI limits are not loader
limits. The source can accept spellings deliberately outside this finite owned policy.

Publish against the exact predecessor package with an output directory that does not exist:

```text
poe-optimizer publish-owned-normalization runs/owned-item-quality-01/package --normalization data/owned/poe2/3887ae68/gem-inputs/normalization.json --output runs/owned-gem-inputs-01/package
```

The checked operation retains the true previous normalization identity, all game data,
item/source policies, tree content and 110 original query rows. Sidecar version 12 distinguishes
fresh normalization from the historical implicit empty-schema inference. Omitting the new
policy keeps old serialized policy identity but leaves intrinsic Gem parameters Pending.

The published original counts are 12 Complete / 466 Pending parameter collections, admitted
through explicit guards. The five originals still require other input/mechanical work;
this policy does not establish complete native evaluation.

Broader physical Gem conversion uses the separate V4 schema knowledge migration. Catalog
identity alone cannot prove complete potential effects or complete parameter declarations.
The next reviewed support conversion should retain Partial membership while carrying known
typed inputs. See the [contract](../../../../../docs/owned-gem-inputs.md).
