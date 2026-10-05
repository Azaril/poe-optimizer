# Potential Skill supplies and support associations

This offline V4 migration corrects the full Bidding endpoint at
`runs/owned-bidding-delivery-01/package`. It removes an association from a Known
Gem's `skills.members` only when the exact mapped source Skill has
`support == true` in the pinned constructed catalogue. The migration allocates
no IDs and changes no numerical programs, mappings, source primary identities,
roles, parameter declarations, or coverage closures.

The catalogue contains 966 Gems and 1,436 resolved Skill identities. The existing
endpoint has 619 Known Gem schemas and 347 Unmapped schemas. Exactly 568 Known
descriptors lose one support member each. The complete census also records all
16 missing-reference rows, the 567 support-primary Gems, and the 81 Gems whose
resolved effect lists contain both support and non-support associations. No
Unmapped descriptor is promoted. The 618 Partial and one Complete existing
Known membership closures are retained verbatim.

The source's `grantedEffect` is a primary association for both active and support
Gems. It is not, by itself, an executable Skill supply. `Data.lua` constructs and
orders additional effects separately; `CalcSetup` processes support effects in
the support lane. The correction retains every non-support candidate rather
than using UI visibility or display order as a capability classifier:

- Bidding II and III retain their source primary association, role evidence,
  and numerical owner programs, while their potential Skill-supply lists become
  empty with the prior Partial closure.
- Barbs retains its constructed triggered Skill even though it was absent from
  the Gem's raw additional-effect declaration.
- Empowered Sparks retains both non-support additional effects, including
  `TriggeredSparkEmpowerPlayer`. Its hidden/helper execution semantics remain
  unresolved; this packet does not declare its membership Complete or reproduce
  `hideFromSideBar` as a native gameplay rule.
- Mirage Archer and Pounce demonstrate that active Gems can also have support
  associations and that the primary effect need not be first in display order.
  Their Unmapped schemas remain unchanged.
- Concussive Runes retains its missing additional-effect evidence and Unmapped
  schema. Removing a support association never repairs an unresolved reference.

`bindings.json` contains the full 966-row reference and prior-membership census.
`dependencies.json` contains all 568 exact predecessor descriptors.
`migration.json` contains only their filtered replacements. Those three files
use compact JSON because they intentionally preserve full descriptor facts.
`authoring.json` pins these artifacts, the checked-in catalogue and exact mapping,
the existing classification policy, and the complete relevant manifest file
set. No source evaluator rerun or new maintained extraction program is required.

The normal Rust authoring checks use only tracked files. They independently join
the source catalogue, declaration role flags, exact mapping, complete reference
census, and migration delta. Negative checks reject loss of a non-support effect
or promotion of a closure. The ignored publication gate authenticates the exact
predecessor, compiles the existing V4 migration, then restores only the reviewed
membership replacements and definition identity rebindings to compare the whole
prior input. It replays all five unchanged originals and all 110 queries, with
selected issue counts `114 / 117 / 109 / 122 / 11`, and checks all 18 output files
against an independently rebuilt package. Full builds remain Pending; no
evaluation bundle is added.

Checked publication: `runs/owned-gem-executable-memberships-02/package`, input
`2a9935bf63de91702021c10b43bd692d97bcae787cf709c07d827ab00fb40074`.
All six catalog CLI tests pass in `runs/owned-gem-membership-publication-02.log`.
The eighteen files total 60,798,258 bytes and reproduce byte-for-byte; the release
has 95 provenance rows. The first publication attempt found an old unconditional
primary-membership guard in physical input-inventory validation. The guard now
distinguishes support association from active supply, with regressions preserving
the active-skill requirement and historical immutable-package replay.

Run the ordinary checks:

```powershell
cargo test --test owned_catalog_cli executable_membership_correction
```

Run publication using a new, nonexistent output directory:

```powershell
$env:POE_OPTIMIZER_TEST_GEM_MEMBERSHIP_PRIOR = 'runs/owned-bidding-delivery-01/package'
$env:POE_OPTIMIZER_TEST_GEM_MEMBERSHIP_OUTPUT = 'runs/owned-gem-executable-memberships-01'
cargo test --test owned_catalog_cli publish_executable_membership_correction_preserving_all_five_originals -- --include-ignored --exact --test-threads=1
```

The corrected endpoint is published at `<output>/package`. The Bidding component
test can consume it through `POE_OPTIMIZER_TEST_BIDDING_RELEASE`; that fixture
checks the exact published correction and Bidding programs without locally
clearing any Gem membership list. Runtime activation, additional-effect
conditions, and numerical coverage remain separate obligations.
