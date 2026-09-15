//! Directly authored registry/mapping laws; no source checkout, parser or VM.
use poe_optimizer_core::{
    data::DataIdentity, owned_build::DeclaredSlot, owned_definitions::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::{
    OwnedDefinitionSchemaPackage, OwnedSchemaLimits, SchemaPackageInput,
};
use poe_optimizer_import::owned_mapping::*;
use serde_json::{Value, json};

fn limits() -> OwnedMappingLimits {
    OwnedMappingLimits::default()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("authored-game", "v1").unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn number(value: i64) -> BoundedInteger {
    BoundedInteger::new(value).unwrap()
}
fn text(value: &str) -> SourceComponent {
    SourceComponent::Text(value.into())
}
fn subject<I: SchemaDefinitionId>(id: &I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
fn slot_subject<I: SchemaSlotId>(id: &DeclaredSlot<I>) -> SchemaSubject {
    SchemaSubject::Slot(I::address(id))
}
fn unmapped<I, D>(id: I, subject: SchemaSubject) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Unmapped {
            gaps: vec![SchemaGap {
                subject,
                facet: SchemaFacet::InputSchema,
                code: key("schema.not-converted"),
            }],
        },
    }
}
fn expect_kind<T: std::fmt::Debug>(
    result: Result<T, OwnedMappingError>,
    expected: OwnedMappingErrorKind,
) {
    match result.unwrap_err() {
        OwnedMappingError::Invalid { kind, .. } => assert_eq!(kind, expected),
        error => panic!("expected {expected:?}, got {error:?}"),
    }
}
fn pin() -> SourcePin {
    SourcePin {
        system: ExternalSourceSystem::PathOfBuilding2,
        revision: "commit-a".into(),
        files: vec![
            SourceFilePin {
                path: "Data/Gems.lua".into(),
                sha256: "a".repeat(64),
            },
            SourceFilePin {
                path: "Data/Skills.lua".into(),
                sha256: "b".repeat(64),
            },
        ],
    }
}
fn gem_selector(variant: SourceComponent) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::Gem {
        game_id: text("SourceGem"),
        variant_id: variant,
    })
}
fn mapped(source: ExternalSelector, target: SchemaSubject) -> MappingEntry {
    MappingEntry {
        source,
        outcome: MappingOutcome::Mapped {
            target,
            basis: MappingBasis::Exact,
        },
    }
}

struct Fixture {
    registry: OwnedIdRegistry,
    schema: OwnedDefinitionSchemaPackage,
    class: ClassDefId,
    gem_a: GemDefId,
    gem_b: GemDefId,
    parameter: DeclaredSlot<ParameterSlotDefId>,
    output: DeclaredSlot<ActionOutputDefId>,
}
impl Fixture {
    fn new() -> Self {
        let mut registry = OwnedIdRegistry::empty(ns(), limits()).unwrap();
        let class: ClassDefId = registry.allocate_definition().unwrap();
        let gem_a: GemDefId = registry.allocate_definition().unwrap();
        let gem_b: GemDefId = registry.allocate_definition().unwrap();
        let skill: SkillDefId = registry.allocate_definition().unwrap();
        let parameter: DeclaredSlot<ParameterSlotDefId> = registry
            .allocate_slot(SlotOwnerDefId::Gem(gem_a.clone()))
            .unwrap();
        let output: DeclaredSlot<ActionOutputDefId> = registry
            .allocate_slot(SlotOwnerDefId::Skill(skill.clone()))
            .unwrap();
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: 1,
                namespace: ns(),
                release: key("release-a"),
                semantics_version: key("semantics-a"),
                definitions: vec![
                    DefinitionDescriptor::Class(unmapped(class.clone(), subject(&class))),
                    DefinitionDescriptor::Gem(unmapped(gem_a.clone(), subject(&gem_a))),
                    DefinitionDescriptor::Gem(unmapped(gem_b.clone(), subject(&gem_b))),
                    DefinitionDescriptor::Skill(unmapped(skill.clone(), subject(&skill))),
                ],
                slots: vec![
                    SlotDescriptor::Parameter(unmapped(
                        parameter.clone(),
                        slot_subject(&parameter),
                    )),
                    SlotDescriptor::ActionOutput(unmapped(output.clone(), slot_subject(&output))),
                ],
            },
            OwnedSchemaLimits::default(),
        )
        .unwrap();
        Self {
            registry,
            schema,
            class,
            gem_a,
            gem_b,
            parameter,
            output,
        }
    }
    fn input(&self, entries: Vec<MappingEntry>) -> MappingPackageInput {
        MappingPackageInput {
            schema_version: 1,
            namespace: ns(),
            registry: self.registry.identity().unwrap(),
            definitions: self.schema.identity().clone(),
            source: pin(),
            policy_version: key("policy-v1"),
            entries,
        }
    }
    fn index(&self, input: MappingPackageInput) -> Result<OwnedMappingIndex, OwnedMappingError> {
        OwnedMappingIndex::new(input, &self.registry, &self.schema, limits())
    }
    fn one(&self) -> MappingPackageInput {
        self.input(vec![mapped(
            gem_selector(SourceComponent::Missing),
            subject(&self.gem_a),
        )])
    }
}

#[test]
fn allocation_uses_only_persisted_sequence_and_supports_all_typed_families() {
    let mut registry = OwnedIdRegistry::empty(ns(), limits()).unwrap();
    macro_rules! allocate { ($($ty:ty),+ $(,)?) => { $(let _: $ty = registry.allocate_definition().unwrap();)+ }; }
    allocate!(
        ClassDefId,
        AscendancyDefId,
        RewardDefId,
        ItemTemplateDefId,
        ModifierDefId,
        GemDefId,
        SkillDefId,
        PassiveNodeDefId,
        PointPoolDefId,
        EquipmentSlotDefId,
        EncounterDefId,
        MetricDefId,
        OptionDefId,
        ActionPartDefId,
        ActionModeDefId,
        ActionStatSetDefId,
        UsagePolicyDefId,
        SkillLinkRoleDefId,
        SocketSlotDefId,
        UnitDefId,
        QualityDefId,
        ExternalInputDefId,
        StatDefId,
        CapabilityDefId
    );
    let owner = SlotOwnerDefId::Class(ClassDefId::parse(ns(), "def.0000000000000001").unwrap());
    macro_rules! allocate_slots { ($($ty:ty),+ $(,)?) => { $(let _: DeclaredSlot<$ty> = registry.allocate_slot(owner.clone()).unwrap();)+ }; }
    allocate_slots!(
        ParameterSlotDefId,
        ChoiceSlotDefId,
        GrantSlotDefId,
        ActorSlotDefId,
        SkillGrantSlotDefId,
        ActionOutputDefId
    );
    assert_eq!(registry.input().last_issued.get(), 30);
    let encoded = encode_registry(&registry, limits()).unwrap();
    let mut shuffled = registry.input().clone();
    shuffled.entries.reverse();
    let restored = OwnedIdRegistry::new(shuffled, limits()).unwrap();
    assert_eq!(encode_registry(&restored, limits()).unwrap(), encoded);
    assert_eq!(restored.identity().unwrap(), registry.identity().unwrap());
    let mut restored = decode_registry(&encoded, limits()).unwrap();
    let next: GemDefId = restored.allocate_definition().unwrap();
    assert_eq!(next.key().as_str(), "def.000000000000001f");
    registry.validate_successor(&restored).unwrap();
}

#[test]
fn retirement_keeps_tombstones_and_requires_explicit_child_retirement() {
    let mut f = Fixture::new();
    let before = f.registry.identity().unwrap();
    expect_kind(
        f.registry.retire(&subject(&f.gem_a), key("removed")),
        OwnedMappingErrorKind::ActiveDependentSlots,
    );
    assert_eq!(before, f.registry.identity().unwrap());
    f.registry
        .retire(&slot_subject(&f.parameter), key("removed-slot"))
        .unwrap();
    f.registry
        .retire(&subject(&f.gem_a), key("removed-owner"))
        .unwrap();
    assert!(matches!(
        f.registry.entry(&subject(&f.gem_a)).unwrap().state,
        RegistryState::Retired { .. }
    ));
    let attempt: Result<DeclaredSlot<ParameterSlotDefId>, _> = f
        .registry
        .allocate_slot(SlotOwnerDefId::Gem(f.gem_a.clone()));
    expect_kind(attempt, OwnedMappingErrorKind::RetiredTarget);
    expect_kind(
        f.registry.retire(&subject(&f.gem_a), key("again")),
        OwnedMappingErrorKind::AlreadyRetired,
    );
    let replacement: GemDefId = f.registry.allocate_definition().unwrap();
    assert_ne!(replacement, f.gem_a);
    assert_eq!(f.registry.input().entries.len(), 7);
    let restored =
        decode_registry(&encode_registry(&f.registry, limits()).unwrap(), limits()).unwrap();
    assert_eq!(restored.input(), f.registry.input());
}

#[test]
fn registry_mutation_limits_are_atomic_across_wire_counter_boundaries() {
    let mut registry = OwnedIdRegistry::empty(ns(), limits()).unwrap();
    for _ in 0..32 {
        let _: GemDefId = registry.allocate_definition().unwrap();
        let bytes = encode_registry(&registry, limits()).unwrap();
        let exact = OwnedMappingLimits {
            max_wire_bytes: bytes.len(),
            ..limits()
        };
        let mut bounded = decode_registry(&bytes, exact).unwrap();
        assert_eq!(encode_registry(&bounded, exact).unwrap(), bytes);
        let before = bounded.input().clone();
        let attempt: Result<GemDefId, _> = bounded.allocate_definition();
        assert!(matches!(attempt, Err(OwnedMappingError::TooLarge { .. })));
        assert_eq!(bounded.input(), &before);
        let target = bounded.input().entries[0].target.clone();
        assert!(matches!(
            bounded.retire(&target, key("removed")),
            Err(OwnedMappingError::TooLarge { .. })
        ));
        assert_eq!(bounded.input(), &before);
    }
    let bounded = OwnedMappingLimits {
        max_entries: 1,
        ..limits()
    };
    let mut registry = OwnedIdRegistry::empty(ns(), bounded).unwrap();
    let _: ClassDefId = registry.allocate_definition().unwrap();
    let before = registry.input().clone();
    let attempt: Result<GemDefId, _> = registry.allocate_definition();
    expect_kind(attempt, OwnedMappingErrorKind::LimitExceeded);
    assert_eq!(&before, registry.input());
}

#[test]
fn malformed_history_and_registry_codec_are_rejected() {
    let f = Fixture::new();
    let mut raw = f.registry.input().clone();
    raw.revision = number(-1);
    expect_kind(
        OwnedIdRegistry::new(raw, limits()),
        OwnedMappingErrorKind::InvalidCounter,
    );
    let mut raw = f.registry.input().clone();
    raw.entries[1].sequence = number(1);
    expect_kind(
        OwnedIdRegistry::new(raw, limits()),
        OwnedMappingErrorKind::HistoryGap,
    );
    let mut raw = f.registry.input().clone();
    raw.entries[0].target = subject(&ClassDefId::parse(ns(), "external-display-name").unwrap());
    expect_kind(
        OwnedIdRegistry::new(raw, limits()),
        OwnedMappingErrorKind::WrongAllocatedKey,
    );
    let mut raw = f.registry.input().clone();
    if let SchemaSubject::Slot(SlotAddress::Parameter(slot)) = &mut raw.entries[4].target {
        slot.declaration =
            SlotOwnerDefId::Gem(GemDefId::parse(ns(), "def.0000000000000999").unwrap());
    }
    expect_kind(
        OwnedIdRegistry::new(raw, limits()),
        OwnedMappingErrorKind::MissingRegistryOwner,
    );
    let mut value = serde_json::to_value(f.registry.input()).unwrap();
    value["source_name"] = json!("forbidden");
    assert!(matches!(
        decode_registry(&serde_json::to_vec(&value).unwrap(), limits()),
        Err(OwnedMappingError::Json(_))
    ));
    let bytes = encode_registry(&f.registry, limits()).unwrap();
    assert!(matches!(
        decode_registry(
            &bytes,
            OwnedMappingLimits {
                max_wire_bytes: bytes.len() - 1,
                ..limits()
            }
        ),
        Err(OwnedMappingError::TooLarge { .. })
    ));
}

#[test]
fn successor_check_prevents_retargeting_deletion_and_retirement_reversal() {
    let mut original = OwnedIdRegistry::empty(ns(), limits()).unwrap();
    let class: ClassDefId = original.allocate_definition().unwrap();
    original.validate_successor(&original).unwrap();
    let mut raw = original.input().clone();
    raw.entries[0].target = subject(&GemDefId::new(ns(), class.key().clone()));
    let retargeted = OwnedIdRegistry::new(raw, limits()).unwrap();
    expect_kind(
        original.validate_successor(&retargeted),
        OwnedMappingErrorKind::SuccessorConflict,
    );
    let empty = OwnedIdRegistry::empty(ns(), limits()).unwrap();
    expect_kind(
        original.validate_successor(&empty),
        OwnedMappingErrorKind::SuccessorConflict,
    );
    let mut retired = original.clone();
    retired.retire(&subject(&class), key("removed")).unwrap();
    original.validate_successor(&retired).unwrap();
    let mut reversed = original.clone();
    let _: GemDefId = reversed.allocate_definition().unwrap();
    expect_kind(
        retired.validate_successor(&reversed),
        OwnedMappingErrorKind::SuccessorConflict,
    );
    let mut raw = retired.input().clone();
    raw.entries[0].state = RegistryState::Retired {
        reason: key("rewritten-history"),
    };
    let rewritten = OwnedIdRegistry::new(raw, limits()).unwrap();
    expect_kind(
        retired.validate_successor(&rewritten),
        OwnedMappingErrorKind::SuccessorConflict,
    );
}

#[test]
fn lookup_preserves_missing_empty_ambiguous_and_multiline_evidence() {
    let f = Fixture::new();
    let missing = gem_selector(SourceComponent::Missing);
    let empty = gem_selector(text(""));
    let ambiguous = gem_selector(text("unknown-variant"));
    let config = ExternalSelector::Configuration {
        key: text(" customMods "),
        source: ConfigSourceRole::Input,
        role: ConfigMappingRole::Parameter,
        value: text("line one\nLine TWO\n"),
    };
    let input = f.input(vec![
        mapped(missing.clone(), subject(&f.gem_a)),
        mapped(empty.clone(), subject(&f.gem_b)),
        MappingEntry {
            source: ambiguous.clone(),
            outcome: MappingOutcome::Ambiguous {
                candidates: vec![subject(&f.gem_b), subject(&f.gem_a)],
                issue: key("missing-variant"),
            },
        },
        MappingEntry {
            source: config.clone(),
            outcome: MappingOutcome::Unmapped {
                issue: key("unsupported-config"),
            },
        },
    ]);
    let index = f.index(input).unwrap();
    assert_ne!(index.lookup(&missing), index.lookup(&empty));
    assert!(matches!(
        index.lookup(&ambiguous),
        Some(MappingOutcome::Ambiguous { .. })
    ));
    assert!(matches!(
        index.lookup(&config),
        Some(MappingOutcome::Unmapped { .. })
    ));
    assert!(
        index
            .lookup(&gem_selector(text("UNKNOWN-VARIANT")))
            .is_none()
    );
    assert!(matches!(
        f.schema.definition(&f.gem_a),
        SchemaLookup::Unmapped(_)
    ));
    let bytes = encode_mapping_package(&index, limits()).unwrap();
    let restored = decode_mapping_package(&bytes, &f.registry, &f.schema, limits()).unwrap();
    assert_eq!(restored.lookup(&config), index.lookup(&config));
    assert_eq!(restored.identity(), index.identity());
}

#[test]
fn duplicate_selectors_and_unreviewed_aliases_never_pick_a_winner() {
    let f = Fixture::new();
    let row = f.one().entries.remove(0);
    for outcome in [
        row.outcome.clone(),
        MappingOutcome::Unmapped {
            issue: key("conflict"),
        },
    ] {
        expect_kind(
            f.index(f.input(vec![
                row.clone(),
                MappingEntry {
                    source: row.source.clone(),
                    outcome,
                },
            ])),
            OwnedMappingErrorKind::DuplicateSelector,
        );
    }
    let second = mapped(gem_selector(text("renamed")), subject(&f.gem_a));
    expect_kind(
        f.index(f.input(vec![row.clone(), second.clone()])),
        OwnedMappingErrorKind::UnreviewedAlias,
    );
    let mut reviewed = second;
    if let MappingOutcome::Mapped { basis, .. } = &mut reviewed.outcome {
        *basis = MappingBasis::ReviewedAlias {
            reason: key("reviewed-source-rename"),
        };
    }
    let before = f.registry.identity().unwrap();
    let index = f.index(f.input(vec![row, reviewed])).unwrap();
    assert_eq!(f.registry.identity().unwrap(), before);
    assert_eq!(index.input().entries.len(), 2);
}

#[test]
fn mapped_slots_keep_exact_declaration_and_source_role() {
    let f = Fixture::new();
    let source = ExternalSelector::Slot {
        owner: ExternalOwnerSelector::Gem {
            game_id: text("SourceGem"),
            variant_id: SourceComponent::Missing,
        },
        kind: ExternalSlotKind::Parameter,
        key: text("variant"),
    };
    let output = ExternalSelector::Configuration {
        key: text("output"),
        source: ConfigSourceRole::Default,
        role: ConfigMappingRole::ActionOutput,
        value: SourceComponent::Missing,
    };
    f.index(f.input(vec![
        mapped(source.clone(), slot_subject(&f.parameter)),
        mapped(output, slot_subject(&f.output)),
    ]))
    .unwrap();
    let mut wrong = f.parameter.clone();
    wrong.declaration = SlotOwnerDefId::Gem(f.gem_b.clone());
    expect_kind(
        f.index(f.input(vec![mapped(source.clone(), slot_subject(&wrong))])),
        OwnedMappingErrorKind::UnknownRegistryTarget,
    );
    expect_kind(
        f.index(f.input(vec![mapped(source, subject(&f.gem_a))])),
        OwnedMappingErrorKind::WrongTargetKind,
    );
}

struct BrokenIndex<'a> {
    inner: &'a OwnedDefinitionSchemaPackage,
    wrong: &'a DefinitionDescriptor,
}
impl DefinitionSchemaIndex for BrokenIndex<'_> {
    fn identity(&self) -> &DataIdentity {
        self.inner.identity()
    }
    fn namespace(&self) -> &GameVersionNamespace {
        self.inner.namespace()
    }
    fn lookup_definition(&self, _: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
        Some(self.wrong)
    }
    fn lookup_slot(&self, address: &SlotAddress) -> Option<&SlotDescriptor> {
        self.inner.lookup_slot(address)
    }
}

#[test]
fn targets_require_active_registry_membership_and_exact_schema_address() {
    let mut f = Fixture::new();
    let source = gem_selector(SourceComponent::Missing);
    expect_kind(
        f.index(f.input(vec![mapped(source.clone(), subject(&f.class))])),
        OwnedMappingErrorKind::WrongTargetKind,
    );
    let unknown = GemDefId::parse(ns(), "def.0000000000000999").unwrap();
    expect_kind(
        f.index(f.input(vec![mapped(source.clone(), subject(&unknown))])),
        OwnedMappingErrorKind::UnknownRegistryTarget,
    );
    let foreign = GemDefId::new(
        GameVersionNamespace::new("other-game", "v1").unwrap(),
        f.gem_a.key().clone(),
    );
    expect_kind(
        f.index(f.input(vec![mapped(source.clone(), subject(&foreign))])),
        OwnedMappingErrorKind::ForeignNamespace,
    );
    let broken = BrokenIndex {
        inner: &f.schema,
        wrong: f.schema.lookup_definition(&f.class.address()).unwrap(),
    };
    expect_kind(
        OwnedMappingIndex::new(f.one(), &f.registry, &broken, limits()),
        OwnedMappingErrorKind::InconsistentSchemaIndex,
    );
    let mut schema = f.schema.input().clone();
    schema
        .definitions
        .retain(|entry| entry.address() != f.gem_b.address());
    let schema = OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
    let mut input = f.input(vec![mapped(source.clone(), subject(&f.gem_b))]);
    input.definitions = schema.identity().clone();
    expect_kind(
        OwnedMappingIndex::new(input, &f.registry, &schema, limits()),
        OwnedMappingErrorKind::MissingSchemaTarget,
    );
    f.registry
        .retire(&subject(&f.gem_b), key("retired"))
        .unwrap();
    expect_kind(
        f.index(f.input(vec![mapped(source, subject(&f.gem_b))])),
        OwnedMappingErrorKind::RetiredTarget,
    );
}

#[test]
fn binding_checks_registry_package_source_and_policy_snapshots() {
    let f = Fixture::new();
    let index = f.index(f.one()).unwrap();
    let mut reordered = pin();
    reordered.files.reverse();
    index
        .verify_bindings(
            &f.registry,
            &f.schema,
            &reordered,
            &key("policy-v1"),
            limits(),
        )
        .unwrap();
    let mut newer = f.registry.clone();
    let _: GemDefId = newer.allocate_definition().unwrap();
    expect_kind(
        index.verify_bindings(&newer, &f.schema, &pin(), &key("policy-v1"), limits()),
        OwnedMappingErrorKind::RegistryBindingMismatch,
    );
    let mut raw = f.schema.input().clone();
    raw.release = key("release-b");
    let newer_schema =
        OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    expect_kind(
        index.verify_bindings(
            &f.registry,
            &newer_schema,
            &pin(),
            &key("policy-v1"),
            limits(),
        ),
        OwnedMappingErrorKind::SchemaBindingMismatch,
    );
    for changed in [
        SourcePin {
            revision: "commit-b".into(),
            ..pin()
        },
        SourcePin {
            system: ExternalSourceSystem::PathOfBuilding1,
            ..pin()
        },
        SourcePin {
            files: vec![SourceFilePin {
                path: "Data/Gems.lua".into(),
                sha256: "c".repeat(64),
            }],
            ..pin()
        },
    ] {
        expect_kind(
            index.verify_bindings(
                &f.registry,
                &f.schema,
                &changed,
                &key("policy-v1"),
                limits(),
            ),
            OwnedMappingErrorKind::SourceBindingMismatch,
        );
    }
    expect_kind(
        index.verify_bindings(&f.registry, &f.schema, &pin(), &key("policy-v2"), limits()),
        OwnedMappingErrorKind::PolicyBindingMismatch,
    );
    let mut first_game_tag = f.one();
    first_game_tag.source.system = ExternalSourceSystem::PathOfBuilding1;
    f.index(first_game_tag).unwrap();
}

#[test]
fn source_manifest_and_candidate_sets_have_strict_bounded_shapes() {
    let f = Fixture::new();
    for path in [
        "",
        "/absolute",
        "../outside",
        "Data//Gem",
        "./Gem",
        "C:/Gem",
        "Data\\Gem",
    ] {
        let mut input = f.one();
        input.source.files[0].path = path.into();
        expect_kind(f.index(input), OwnedMappingErrorKind::InvalidSourcePin);
    }
    let mut input = f.one();
    input.source.files[0].sha256 = "A".repeat(64);
    expect_kind(f.index(input), OwnedMappingErrorKind::InvalidDigest);
    let mut input = f.one();
    input.source.files.push(input.source.files[0].clone());
    expect_kind(f.index(input), OwnedMappingErrorKind::InvalidSourcePin);
    let mut input = f.one();
    input.source.files.clear();
    expect_kind(f.index(input), OwnedMappingErrorKind::InvalidSourcePin);
    for (candidates, expected) in [
        (
            vec![subject(&f.gem_a)],
            OwnedMappingErrorKind::TooFewCandidates,
        ),
        (
            vec![subject(&f.gem_a), subject(&f.gem_a)],
            OwnedMappingErrorKind::DuplicateCandidate,
        ),
    ] {
        let input = f.input(vec![MappingEntry {
            source: gem_selector(SourceComponent::Missing),
            outcome: MappingOutcome::Ambiguous {
                candidates,
                issue: key("ambiguous"),
            },
        }]);
        expect_kind(f.index(input), expected);
    }
    let candidates = f.input(vec![MappingEntry {
        source: gem_selector(SourceComponent::Missing),
        outcome: MappingOutcome::Ambiguous {
            candidates: vec![subject(&f.gem_a), subject(&f.gem_b)],
            issue: key("ambiguous"),
        },
    }]);
    expect_kind(
        OwnedMappingIndex::new(
            candidates,
            &f.registry,
            &f.schema,
            OwnedMappingLimits {
                max_candidates: 1,
                ..limits()
            },
        ),
        OwnedMappingErrorKind::LimitExceeded,
    );
    for bounded in [
        OwnedMappingLimits {
            max_entries: 2,
            ..limits()
        },
        OwnedMappingLimits {
            max_collection_entries: 1,
            ..limits()
        },
        OwnedMappingLimits {
            max_string_bytes: 63,
            ..limits()
        },
        OwnedMappingLimits {
            max_total_string_bytes: 64,
            ..limits()
        },
    ] {
        expect_kind(
            OwnedMappingIndex::new(f.one(), &f.registry, &f.schema, bounded),
            OwnedMappingErrorKind::LimitExceeded,
        );
    }
}

#[test]
fn mapping_codec_is_canonical_strict_and_rechecks_tighter_limits() {
    let f = Fixture::new();
    let mut input = f.input(vec![
        mapped(gem_selector(SourceComponent::Missing), subject(&f.gem_a)),
        mapped(gem_selector(text("")), subject(&f.gem_b)),
    ]);
    let a = f.index(input.clone()).unwrap();
    input.entries.reverse();
    input.source.files.reverse();
    let b = f.index(input).unwrap();
    assert_eq!(a.identity(), b.identity());
    let bytes = encode_mapping_package(&a, limits()).unwrap();
    assert_eq!(bytes, encode_mapping_package(&b, limits()).unwrap());
    for location in ["root", "source", "outcome"] {
        let mut value = serde_json::to_value(a.input()).unwrap();
        let object = match location {
            "root" => &mut value,
            "source" => &mut value["source"],
            _ => &mut value["entries"][0]["outcome"],
        };
        object["source_program"] = json!("unsupported");
        assert!(matches!(
            decode_mapping_package(
                &serde_json::to_vec(&value).unwrap(),
                &f.registry,
                &f.schema,
                limits()
            ),
            Err(OwnedMappingError::Json(_))
        ));
    }
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    value["schema_version"] = json!(99);
    assert!(matches!(
        decode_mapping_package(
            &serde_json::to_vec(&value).unwrap(),
            &f.registry,
            &f.schema,
            limits()
        ),
        Err(OwnedMappingError::UnsupportedVersion { .. })
    ));
    let exact = OwnedMappingLimits {
        max_wire_bytes: bytes.len(),
        ..limits()
    };
    assert_eq!(encode_mapping_package(&a, exact).unwrap(), bytes);
    let short = OwnedMappingLimits {
        max_wire_bytes: bytes.len() - 1,
        ..limits()
    };
    assert!(matches!(
        encode_mapping_package(&a, short),
        Err(OwnedMappingError::TooLarge { .. })
    ));
    assert!(matches!(
        decode_mapping_package(&bytes, &f.registry, &f.schema, short),
        Err(OwnedMappingError::TooLarge { .. })
    ));
    expect_kind(
        encode_mapping_package(
            &a,
            OwnedMappingLimits {
                max_collection_entries: 1,
                ..limits()
            },
        ),
        OwnedMappingErrorKind::LimitExceeded,
    );
}

#[test]
fn registry_rejects_foreign_owners_and_impossible_edit_counters() {
    let mut f = Fixture::new();
    let before = f.registry.identity().unwrap();
    let foreign = SlotOwnerDefId::Gem(GemDefId::new(
        GameVersionNamespace::new("other-game", "v1").unwrap(),
        f.gem_a.key().clone(),
    ));
    let attempt: Result<DeclaredSlot<ParameterSlotDefId>, _> = f.registry.allocate_slot(foreign);
    expect_kind(attempt, OwnedMappingErrorKind::ForeignNamespace);
    let absent = SlotOwnerDefId::Gem(GemDefId::parse(ns(), "def.0000000000000999").unwrap());
    let attempt: Result<DeclaredSlot<ParameterSlotDefId>, _> = f.registry.allocate_slot(absent);
    expect_kind(attempt, OwnedMappingErrorKind::UnknownRegistryTarget);
    assert_eq!(before, f.registry.identity().unwrap());
    let mut raw = f.registry.input().clone();
    raw.entries[1].state = RegistryState::Retired {
        reason: key("retired-before-slot"),
    };
    raw.revision = number(raw.revision.get() + 1);
    expect_kind(
        OwnedIdRegistry::new(raw, limits()),
        OwnedMappingErrorKind::RetiredTarget,
    );
    let mut raw = f.registry.input().clone();
    raw.revision = number(raw.revision.get() + 1);
    expect_kind(
        OwnedIdRegistry::new(raw, limits()),
        OwnedMappingErrorKind::InvalidCounter,
    );
    let mut value = serde_json::to_value(f.registry.input()).unwrap();
    value["last_issued"] = json!(9_007_199_254_740_992_u64);
    assert!(matches!(
        decode_registry(&serde_json::to_vec(&value).unwrap(), limits()),
        Err(OwnedMappingError::Json(_))
    ));
    let mut value = serde_json::to_value(f.registry.input()).unwrap();
    value["entries"][0]["target"]["value"]["value"]["kind"] = json!("gem");
    assert!(matches!(
        decode_registry(&serde_json::to_vec(&value).unwrap(), limits()),
        Err(OwnedMappingError::Json(_))
    ));
}

#[test]
fn registered_slot_families_and_positive_owner_mappings_must_agree() {
    let mut f = Fixture::new();
    let item: ItemTemplateDefId = f.registry.allocate_definition().unwrap();
    let item_slot: DeclaredSlot<ParameterSlotDefId> = f
        .registry
        .allocate_slot(SlotOwnerDefId::ItemTemplate(item.clone()))
        .unwrap();
    let other_gem_slot: DeclaredSlot<ParameterSlotDefId> = f
        .registry
        .allocate_slot(SlotOwnerDefId::Gem(f.gem_b.clone()))
        .unwrap();
    let mut package = f.schema.input().clone();
    package
        .definitions
        .push(DefinitionDescriptor::ItemTemplate(unmapped(
            item.clone(),
            subject(&item),
        )));
    package.slots.push(SlotDescriptor::Parameter(unmapped(
        item_slot.clone(),
        slot_subject(&item_slot),
    )));
    package.slots.push(SlotDescriptor::Parameter(unmapped(
        other_gem_slot.clone(),
        slot_subject(&other_gem_slot),
    )));
    f.schema = OwnedDefinitionSchemaPackage::new(package, OwnedSchemaLimits::default()).unwrap();
    let owner = ExternalOwnerSelector::Gem {
        game_id: text("SourceGem"),
        variant_id: SourceComponent::Missing,
    };
    let source = ExternalSelector::Slot {
        owner: owner.clone(),
        kind: ExternalSlotKind::Parameter,
        key: text("level-state"),
    };
    expect_kind(
        f.index(f.input(vec![mapped(source.clone(), slot_subject(&item_slot))])),
        OwnedMappingErrorKind::WrongTargetOwner,
    );
    let child = mapped(source, slot_subject(&other_gem_slot));
    f.index(f.input(vec![child.clone()])).unwrap();
    let parent = ExternalSelector::Definition(owner);
    expect_kind(
        f.index(f.input(vec![
            mapped(parent.clone(), subject(&f.gem_a)),
            child.clone(),
        ])),
        OwnedMappingErrorKind::WrongTargetOwner,
    );
    f.index(f.input(vec![
        mapped(parent.clone(), subject(&f.gem_b)),
        child.clone(),
    ]))
    .unwrap();
    for outcome in [
        MappingOutcome::Unmapped {
            issue: key("owner-not-mapped"),
        },
        MappingOutcome::Ambiguous {
            candidates: vec![subject(&f.gem_a), subject(&f.gem_b)],
            issue: key("owner-ambiguous"),
        },
    ] {
        f.index(f.input(vec![
            MappingEntry {
                source: parent.clone(),
                outcome,
            },
            child.clone(),
        ]))
        .unwrap();
    }
}

#[test]
fn known_socket_topology_rejects_contradictions_without_promoting_unmapped_schema() {
    let mut f = Fixture::new();
    let item: ItemTemplateDefId = f.registry.allocate_definition().unwrap();
    let other_item: ItemTemplateDefId = f.registry.allocate_definition().unwrap();
    let node: PassiveNodeDefId = f.registry.allocate_definition().unwrap();
    let item_socket: SocketSlotDefId = f.registry.allocate_definition().unwrap();
    let passive_socket: SocketSlotDefId = f.registry.allocate_definition().unwrap();
    let unknown_socket: SocketSlotDefId = f.registry.allocate_definition().unwrap();
    let mut package = f.schema.input().clone();
    package.definitions.extend([
        DefinitionDescriptor::ItemTemplate(unmapped(item.clone(), subject(&item))),
        DefinitionDescriptor::ItemTemplate(unmapped(other_item.clone(), subject(&other_item))),
        DefinitionDescriptor::PassiveNode(unmapped(node.clone(), subject(&node))),
        DefinitionDescriptor::SocketSlot(DefinitionEntry {
            id: item_socket.clone(),
            schema: SchemaState::Known(SocketSlotSchema {
                owner: SlotOwnerDefId::ItemTemplate(item),
                kind: SocketKind::Item,
                scope: ScopePolicy::Either,
            }),
        }),
        DefinitionDescriptor::SocketSlot(DefinitionEntry {
            id: passive_socket.clone(),
            schema: SchemaState::Known(SocketSlotSchema {
                owner: SlotOwnerDefId::PassiveNode(node),
                kind: SocketKind::Passive,
                scope: ScopePolicy::Either,
            }),
        }),
        DefinitionDescriptor::SocketSlot(unmapped(
            unknown_socket.clone(),
            subject(&unknown_socket),
        )),
    ]);
    f.schema = OwnedDefinitionSchemaPackage::new(package, OwnedSchemaLimits::default()).unwrap();
    let owner = ExternalOwnerSelector::ItemTemplate {
        base: text("source-item"),
        prototype: SourceComponent::Missing,
        variant: SourceComponent::Missing,
    };
    let source = ExternalSelector::Socket {
        owner: owner.clone(),
        kind: SocketKind::Item,
        key: text("socket"),
    };
    f.index(f.input(vec![mapped(source.clone(), subject(&item_socket))]))
        .unwrap();
    let wrong_kind = ExternalSelector::Socket {
        owner: owner.clone(),
        kind: SocketKind::Passive,
        key: text("socket"),
    };
    expect_kind(
        f.index(f.input(vec![mapped(wrong_kind, subject(&item_socket))])),
        OwnedMappingErrorKind::WrongSocketKind,
    );
    let wrong_owner = ExternalSelector::Socket {
        owner: ExternalOwnerSelector::Gem {
            game_id: text("source-gem"),
            variant_id: SourceComponent::Missing,
        },
        kind: SocketKind::Item,
        key: text("socket"),
    };
    expect_kind(
        f.index(f.input(vec![mapped(wrong_owner.clone(), subject(&item_socket))])),
        OwnedMappingErrorKind::WrongTargetOwner,
    );
    expect_kind(
        f.index(f.input(vec![mapped(source.clone(), subject(&passive_socket))])),
        OwnedMappingErrorKind::WrongTargetOwner,
    );
    expect_kind(
        f.index(f.input(vec![
            mapped(ExternalSelector::Definition(owner), subject(&other_item)),
            mapped(source, subject(&item_socket)),
        ])),
        OwnedMappingErrorKind::WrongTargetOwner,
    );
    f.index(f.input(vec![mapped(wrong_owner, subject(&unknown_socket))]))
        .unwrap();
    assert!(matches!(
        f.schema.definition(&unknown_socket),
        SchemaLookup::Unmapped(_)
    ));
}

#[test]
fn positively_mapped_output_checks_only_known_complete_alternative_membership() {
    let mut f = Fixture::new();
    let part_a: ActionPartDefId = f.registry.allocate_definition().unwrap();
    let part_b: ActionPartDefId = f.registry.allocate_definition().unwrap();
    let mode_a: ActionModeDefId = f.registry.allocate_definition().unwrap();
    let mode_b: ActionModeDefId = f.registry.allocate_definition().unwrap();
    let set_a: ActionStatSetDefId = f.registry.allocate_definition().unwrap();
    let set_b: ActionStatSetDefId = f.registry.allocate_definition().unwrap();
    let other_output: DeclaredSlot<ActionOutputDefId> = f
        .registry
        .allocate_slot(f.output.declaration.clone())
        .unwrap();
    let mut package = f.schema.input().clone();
    package.definitions.extend([
        DefinitionDescriptor::ActionPart(unmapped(part_a.clone(), subject(&part_a))),
        DefinitionDescriptor::ActionPart(unmapped(part_b.clone(), subject(&part_b))),
        DefinitionDescriptor::ActionMode(unmapped(mode_a.clone(), subject(&mode_a))),
        DefinitionDescriptor::ActionMode(unmapped(mode_b.clone(), subject(&mode_b))),
        DefinitionDescriptor::ActionStatSet(unmapped(set_a.clone(), subject(&set_a))),
        DefinitionDescriptor::ActionStatSet(unmapped(set_b.clone(), subject(&set_b))),
    ]);
    package
        .slots
        .retain(|slot| slot.address() != ActionOutputDefId::address(&f.output));
    package.slots.extend([
        SlotDescriptor::ActionOutput(DefinitionEntry {
            id: f.output.clone(),
            schema: SchemaState::Known(ActionOutputSchema {
                actor_role: DeclaredActorRole::Player,
                parts: DeclaredSet::complete(vec![part_a.clone()]),
                modes: DeclaredSet::complete(vec![mode_a.clone()]),
                stat_sets: DeclaredSet::complete(vec![set_a.clone()]),
                choices: DeclaredSet::complete(vec![]),
            }),
        }),
        SlotDescriptor::ActionOutput(unmapped(other_output.clone(), slot_subject(&other_output))),
    ]);
    f.schema = OwnedDefinitionSchemaPackage::new(package, OwnedSchemaLimits::default()).unwrap();
    let owner = ExternalOwnerSelector::Skill {
        effect_id: text("source-effect"),
    };
    let parent_source = ExternalSelector::Slot {
        owner: owner.clone(),
        kind: ExternalSlotKind::ActionOutput,
        key: text("output"),
    };
    let parent = mapped(parent_source.clone(), slot_subject(&f.output));
    for (kind, allowed, forbidden) in [
        (
            ActionAlternativeKind::Part,
            subject(&part_a),
            subject(&part_b),
        ),
        (
            ActionAlternativeKind::Mode,
            subject(&mode_a),
            subject(&mode_b),
        ),
        (
            ActionAlternativeKind::StatSet,
            subject(&set_a),
            subject(&set_b),
        ),
    ] {
        let source = ExternalSelector::ActionAlternative {
            owner: owner.clone(),
            output: text("output"),
            key: text("alternative"),
            kind,
        };
        f.index(f.input(vec![parent.clone(), mapped(source.clone(), allowed)]))
            .unwrap();
        let child = mapped(source, forbidden);
        expect_kind(
            f.index(f.input(vec![parent.clone(), child.clone()])),
            OwnedMappingErrorKind::WrongOutputTopology,
        );
        f.index(f.input(vec![child.clone()])).unwrap();
        for outcome in [
            MappingOutcome::Unmapped {
                issue: key("output-unmapped"),
            },
            MappingOutcome::Ambiguous {
                candidates: vec![slot_subject(&f.output), slot_subject(&other_output)],
                issue: key("output-ambiguous"),
            },
        ] {
            f.index(f.input(vec![
                MappingEntry {
                    source: parent_source.clone(),
                    outcome,
                },
                child.clone(),
            ]))
            .unwrap();
        }
    }
    let source = ExternalSelector::ActionAlternative {
        owner,
        output: text("output"),
        key: text("part"),
        kind: ActionAlternativeKind::Part,
    };
    let child = mapped(source.clone(), subject(&part_b));
    let original = f.schema.clone();
    let mut package = original.input().clone();
    for slot in &mut package.slots {
        if let SlotDescriptor::ActionOutput(DefinitionEntry {
            id,
            schema: SchemaState::Known(schema),
        }) = slot
            && id == &f.output
        {
            schema.parts = DeclaredSet::partial(
                vec![],
                vec![SchemaGap {
                    subject: slot_subject(&f.output),
                    facet: SchemaFacet::StaticLinks,
                    code: key("incomplete-parts"),
                }],
            );
        }
    }
    f.schema = OwnedDefinitionSchemaPackage::new(package, OwnedSchemaLimits::default()).unwrap();
    f.index(f.input(vec![parent.clone(), child.clone()]))
        .unwrap();
    let mut package = original.input().clone();
    for slot in &mut package.slots {
        if slot.address() == ActionOutputDefId::address(&f.output) {
            *slot =
                SlotDescriptor::ActionOutput(unmapped(f.output.clone(), slot_subject(&f.output)));
        }
    }
    f.schema = OwnedDefinitionSchemaPackage::new(package, OwnedSchemaLimits::default()).unwrap();
    f.index(f.input(vec![parent.clone(), child])).unwrap();
    f.schema = original;
    let allowed = mapped(source.clone(), subject(&part_a));
    let mut alias_source = source;
    if let ExternalSelector::ActionAlternative { key, .. } = &mut alias_source {
        *key = text("part-alias");
    }
    let mut alias = mapped(alias_source, subject(&part_a));
    if let MappingOutcome::Mapped { basis, .. } = &mut alias.outcome {
        *basis = MappingBasis::ReviewedAlias {
            reason: key("reviewed-part-alias"),
        };
    }
    let input = f.input(vec![parent, allowed, alias]);
    let bounded = OwnedMappingLimits {
        max_entries: 6,
        ..limits()
    };
    let index = OwnedMappingIndex::new(input.clone(), &f.registry, &f.schema, bounded).unwrap();
    encode_mapping_package(&index, bounded).unwrap();
    expect_kind(
        OwnedMappingIndex::new(
            input,
            &f.registry,
            &f.schema,
            OwnedMappingLimits {
                max_entries: 5,
                ..limits()
            },
        ),
        OwnedMappingErrorKind::LimitExceeded,
    );
}

#[test]
fn stat_and_capability_catalog_mappings_keep_distinct_owned_identity_and_coverage() {
    let mut registry = OwnedIdRegistry::empty(ns(), limits()).unwrap();
    let stat: StatDefId = registry.allocate_definition().unwrap();
    let capability: CapabilityDefId = registry.allocate_definition().unwrap();
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: 1,
            namespace: ns(),
            release: key("computed"),
            semantics_version: key("schema-only"),
            definitions: vec![
                DefinitionDescriptor::Stat(unmapped(stat.clone(), subject(&stat))),
                DefinitionDescriptor::Capability(unmapped(
                    capability.clone(),
                    subject(&capability),
                )),
            ],
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let external = |kind| ExternalSelector::Catalog {
        kind,
        key: text("exact-source-key"),
        version: SourceComponent::Missing,
        variant: SourceComponent::Missing,
    };
    let input = MappingPackageInput {
        schema_version: OWNED_MAPPING_PACKAGE_VERSION,
        namespace: ns(),
        registry: registry.identity().unwrap(),
        definitions: schema.identity().clone(),
        source: pin(),
        policy_version: key("computed-identities"),
        entries: vec![
            mapped(external(ExternalCatalogKind::Stat), subject(&stat)),
            mapped(
                external(ExternalCatalogKind::Capability),
                subject(&capability),
            ),
        ],
    };
    let index = OwnedMappingIndex::new(input.clone(), &registry, &schema, limits()).unwrap();
    assert!(
        matches!(index.lookup(&external(ExternalCatalogKind::Stat)), Some(MappingOutcome::Mapped { target, .. }) if target == &subject(&stat))
    );
    assert!(matches!(
        schema.definition(&stat),
        SchemaLookup::Unmapped(_)
    ));
    assert!(matches!(
        schema.definition(&capability),
        SchemaLookup::Unmapped(_)
    ));
    let bytes = encode_mapping_package(&index, limits()).unwrap();
    let restored = decode_mapping_package(&bytes, &registry, &schema, limits()).unwrap();
    assert_eq!(index.identity(), restored.identity());
    for (source, target) in [
        (external(ExternalCatalogKind::Stat), subject(&capability)),
        (external(ExternalCatalogKind::Capability), subject(&stat)),
    ] {
        let mut wrong = input.clone();
        wrong.entries = vec![mapped(source, target)];
        expect_kind(
            OwnedMappingIndex::new(wrong, &registry, &schema, limits()),
            OwnedMappingErrorKind::WrongTargetKind,
        );
    }
    registry
        .retire(&subject(&stat), key("retired-stat"))
        .unwrap();
    let restored_registry =
        decode_registry(&encode_registry(&registry, limits()).unwrap(), limits()).unwrap();
    let mut retired = input;
    retired.registry = restored_registry.identity().unwrap();
    expect_kind(
        OwnedMappingIndex::new(retired, &restored_registry, &schema, limits()),
        OwnedMappingErrorKind::RetiredTarget,
    );
}
