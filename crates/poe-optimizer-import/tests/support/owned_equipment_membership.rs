//! Exact inventory closure does not resolve the fixture's item mechanics.
use super::*;
use poe_optimizer_import::{owned_item_lines::*, owned_item_source::*};

struct Fixture {
    artifacts: Artifacts,
    policy: NormalizationPolicy,
    items: OwnedItemLinePolicy,
    item_source: ItemSourceLayoutPolicy,
}

fn fixture() -> Fixture {
    let (mut artifacts, mut policy) = loadout_artifacts();
    let mut schema = artifacts.schema.input().clone();
    let mut templates = Vec::new();
    let mut rules = vec![ItemLineRule {
        id: key("rarity"),
        pattern: vec![ItemPatternPart::Literal("Rarity: RARE".into())],
        captures: vec![],
        emissions: vec![ItemEmission::Metadata {
            role: key("rarity"),
        }],
    }];
    for (name, armour) in [("Ordinary Base", false), ("Socket Base", true)] {
        let template = artifacts
            .registry
            .allocate_definition::<ItemTemplateDefinition>()
            .unwrap();
        schema
            .definitions
            .push(DefinitionDescriptor::ItemTemplate(DefinitionEntry {
                id: template.clone(),
                schema: SchemaState::Known(ItemTemplateSchema {
                    item_level: IntegerRange {
                        minimum: BoundedInteger::new(1).unwrap(),
                        maximum: BoundedInteger::new(100).unwrap(),
                    },
                    equipment_slots: DeclaredSet::complete(vec![]),
                    socket_destinations: DeclaredSet::complete(vec![]),
                    modifiers: DeclaredSet::complete(vec![]),
                    quality: QualityUseSchema {
                        presence: QualityPresence::Forbidden,
                        allowed_kinds: DeclaredSet::complete(vec![]),
                    },
                    declarations: DeclaredSlots {
                        parameters: DeclaredSet::complete(vec![]),
                        choices: DeclaredSet::complete(vec![]),
                        grants: DeclaredSet::complete(vec![]),
                        actors: DeclaredSet::complete(vec![]),
                        skill_grants: DeclaredSet::complete(vec![]),
                        outputs: DeclaredSet::complete(vec![]),
                        sockets: DeclaredSet::complete(vec![]),
                    },
                }),
            }));
        rules.push(ItemLineRule {
            id: key(if armour {
                "socket-base"
            } else {
                "ordinary-base"
            }),
            pattern: vec![ItemPatternPart::Literal(name.into())],
            captures: vec![],
            emissions: vec![ItemEmission::Template {
                definition: template.clone(),
            }],
        });
        templates.push(EquipmentAugmentBase {
            template,
            base_name: name.into(),
            weapon: false,
            armour,
            wand: false,
            staff: false,
            sceptre: false,
        });
    }
    rebind_quality_schema(&mut artifacts, &mut policy, schema);
    policy.equipment_membership = Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 {
        definitions: artifacts.schema.identity().clone(),
        templates: templates.clone(),
        source_base_names: vec![
            "Ordinary Base".into(),
            "Socket Base".into(),
            "Unreviewed Base".into(),
        ],
        loader_jewel_fallback_titles: vec!["Legacy Jewel Title".into()],
    });
    let items = OwnedItemLinePolicy::new(
        ItemLinePolicyInput {
            schema_version: OWNED_ITEM_LINE_POLICY_VERSION,
            namespace: ns(),
            version: key("membership-items"),
            definitions: artifacts.schema.identity().clone(),
            whitespace: WhitespacePolicy::TrimAscii,
            rules,
        },
        &artifacts.schema,
        ItemLineLimits::default(),
    )
    .unwrap();
    let item_source = ItemSourceLayoutPolicy::new(
        ItemSourceLayoutPolicyInput {
            schema_version: OWNED_ITEM_SOURCE_PREAMBLE_POLICY_VERSION,
            namespace: ns(),
            version: key("membership-layout"),
            source: pin(),
            item_lines: *items.identity(),
            dialect: ItemSourceDialect::PobExportedSingleTextPreambleV1 {
                flag_bindings: vec![],
                metadata_rules: vec![key("rarity")],
            },
            property_bindings: vec![],
            template_defaults: vec![],
            rule_layouts: items
                .input()
                .rules
                .iter()
                .map(|rule| ItemRuleSourceLayout {
                    rule: rule.id.clone(),
                    role: ItemRuleSourceRole::Header,
                })
                .collect(),
            template_layouts: templates
                .iter()
                .map(|base| ItemTemplateSourceLayout {
                    template: base.template.clone(),
                    load_index_prefix: ItemLoadIndexPrefix::NoGeneratedBuffMembers,
                })
                .collect(),
        },
        &items,
        &artifacts.schema,
        ItemSourceLimits::default(),
    )
    .unwrap();
    Fixture {
        artifacts,
        policy,
        items,
        item_source,
    }
}

fn normalize(
    xml: &str,
    fixture: &Fixture,
    policy: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> std::result::Result<NormalizedImport, NormalizationError> {
    normalize_queries(xml, fixture, policy, &[], limits)
}

fn normalize_queries(
    xml: &str,
    fixture: &Fixture,
    policy: &NormalizationPolicy,
    queries: &[ImportQueryTemplate],
    limits: NormalizationLimits,
) -> std::result::Result<NormalizedImport, NormalizationError> {
    let source = source(xml, 93);
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let result = normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            tree: None,
            items: &fixture.items,
            item_source: &fixture.item_source,
            mappings: &fixture.artifacts.mapping,
            registry: &fixture.artifacts.registry,
            definitions: &fixture.artifacts.schema,
            roles: &fixture.artifacts.roles,
            rewards: &fixture.artifacts.rewards,
        },
        policy,
        queries,
        limits,
    )?;
    let retired = result
        .draft()
        .input()
        .equipment_presets
        .members
        .iter()
        .filter(|preset| matches!(preset.equipment.completion, DraftListCompletion::Complete))
        .count();
    origin_integrity_with_retired(&source, &result, retired);
    Ok(result)
}
fn run(xml: &str, fixture: &Fixture) -> NormalizedImport {
    normalize(
        xml,
        fixture,
        &fixture.policy,
        NormalizationLimits::default(),
    )
    .unwrap()
}
fn xml(raw: &str, slots: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Items activeItemSet="1" useSecondWeaponSet="nil"><Item id="1">{raw}</Item><ItemSet id="1" useSecondWeaponSet="nil">{slots}</ItemSet></Items></PathOfBuilding2>"#
    )
}
const RAW: &str =
    "Rarity: RARE\nTest Title\nOrdinary Base\nImplicits: 0\n+12 to unconverted property";
const SOCKET_RAW: &str = "Rarity: RARE\nTest Title\nSocket Base\nQuality: 20\nSockets: S S\nRune: None\nRune: None\nImplicits: 0\n+12 to unconverted property";
const SLOT: &str = r#"<Slot name="coat" itemId="1" active="nil"/>"#;
fn complete(result: &NormalizedImport, index: usize) -> bool {
    matches!(
        result.draft().input().equipment_presets.members[index]
            .equipment
            .completion,
        DraftListCompletion::Complete
    )
}

#[test]
fn complete_membership_preserves_uses_scopes_allocator_and_every_other_pending_field() {
    let fixture = fixture();
    let input = xml(
        SOCKET_RAW,
        r#"<Slot name="blade" itemId="1"/><Slot name="blade-alt" itemId="1"/><Slot name="coat" itemId="0"/><SocketIdURL nodeId="7960" itemPbURL="" name="Jewel 7960"/>"#,
    );
    let mut old_policy = fixture.policy.clone();
    old_policy.equipment_membership = None;
    let old = normalize(
        &input,
        &fixture,
        &old_policy,
        NormalizationLimits::default(),
    )
    .unwrap();
    let result = run(&input, &fixture);
    assert!(complete(&result, 0));
    assert_eq!(result.allocator_after(), old.allocator_after());
    let new_uses = &result.draft().input().equipment.members;
    assert_eq!(new_uses.len(), 2);
    assert_ne!(new_uses[0].id, new_uses[1].id);
    assert_eq!(new_uses[0].item, new_uses[1].item);
    assert_ne!(new_uses[0].scope, new_uses[1].scope);
    let mut old_value = serde_json::to_value(old.draft().input()).unwrap();
    let new_value = serde_json::to_value(result.draft().input()).unwrap();
    old_value["equipment_presets"]["members"][0]["equipment"]["completion"] =
        new_value["equipment_presets"]["members"][0]["equipment"]["completion"].clone();
    assert_eq!(old_value, new_value);
    assert!(matches!(
        result.draft().input().equipment.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(matches!(
        result.draft().input().items.members[0].modifiers.completion,
        DraftListCompletion::Pending { .. }
    ));
    let old_links: usize = old.sidecar().origins.iter().map(|o| o.links.len()).sum();
    let new_links: usize = result.sidecar().origins.iter().map(|o| o.links.len()).sum();
    assert_eq!(old_links, new_links + 1);
}

#[test]
fn socketless_proof_is_independent_of_unconverted_modifier_layout() {
    let fixture = fixture();
    let result = run(&xml(RAW, SLOT), &fixture);
    assert!(complete(&result, 0));
    assert!(matches!(
        result.sidecar().item_texts[0].attribution.layout,
        ItemLayoutStatus::Pending(_)
    ));
    assert!(matches!(
        result.draft().input().items.members[0].modifier_order,
        DraftField::Pending(_)
    ));
}

#[test]
fn harmless_tags_mod_ranges_and_loader_trade_metadata_do_not_change_membership() {
    let fixture = fixture();
    for raw in [
        format!(
            "{RAW}\n{{range:0.5}}+(10-15) to Spirit\n{{tags:cold_resistance,elemental,cold}}{{range:0.5}}+(20-30)% to Cold Resistance"
        ),
        format!(
            "{SOCKET_RAW}\n{{range:0.5}}Grants Skill: Level (1-20) Firebolt\nPrefix: {{range:0.732}}IncreasedLife1\nCrafted: true"
        ),
    ] {
        let input = xml(&raw, SLOT)
            .replace("</Item>", "<ModRange id=\"1\" range=\"0.5\"/></Item>")
            .replace("</Items>", "<TradeSearchWeights><Stat label=\"Example\" stat=\"example\" weightMult=\"0.5\"/></TradeSearchWeights></Items>");
        assert!(complete(&run(&input, &fixture), 0), "{raw}");
    }
}

#[test]
fn occupied_unknown_missing_and_unreviewed_socket_lifecycles_remain_pending() {
    let fixture = fixture();
    let cases = [
        SOCKET_RAW.replacen("Rune: None", "Rune: Occupied", 1),
        SOCKET_RAW.replace("Rune: None\n", ""),
        SOCKET_RAW.replace("Sockets: S S", "Sockets: S J"),
        SOCKET_RAW.replace("Sockets: S S", "Sockets: S S\nSockets: S S"),
        SOCKET_RAW.replace("Sockets: S S", "Sockets: S"),
        SOCKET_RAW.replace("Socket Base", "Unknown Base"),
        SOCKET_RAW.replace("Socket Base", "{variant:1}Socket Base"),
        SOCKET_RAW.replace("Implicits: 0", "Variant: 1\nImplicits: 0"),
        SOCKET_RAW.replace(
            "+12 to unconverted property",
            "{rune}+12 to unconverted property",
        ),
        SOCKET_RAW.replace(
            "+12 to unconverted property",
            "+12 to unconverted property (rune)",
        ),
        RAW.replace("Test Title", "Legacy Jewel Title"),
        format!("{RAW}\nUnreviewed Base"),
        format!("{RAW}\nSuperior Unreviewed Base"),
        format!("{RAW}\n{{range:0.5}}Unreviewed Base"),
        format!("{RAW}\n[Unreviewed Base]"),
        RAW.replace("Ordinary Base", "\u{a0}Ordinary Base"),
        SOCKET_RAW.replace("Sockets: S S", "(Reminder\nSockets: S S"),
    ];
    for raw in cases {
        assert!(!complete(&run(&xml(&raw, SLOT), &fixture), 0), "{raw}");
    }
}

#[test]
fn whole_set_census_refuses_unknown_nested_or_ambiguous_receiving_rows() {
    let fixture = fixture();
    for slots in [
        format!("{SLOT}{SLOT}"),
        format!("{SLOT}<Slot name=\"coat\" itemId=\"0\"/>"),
        format!("<Slot name=\"coat\" itemId=\"0\"/>{SLOT}"),
        format!("{SLOT}<RuneSlot slotName=\"coat\" runeName=\"None\"/>"),
        format!("{SLOT}<Unknown/>"),
        r#"<Slot name="coat"/>"#.into(),
        r#"<Slot name="unknown" itemId="0"/>"#.into(),
        r#"<Slot name="coat" itemId="0" unexpected="yes"/>"#.into(),
        r#"<Slot name="coat" itemId="1"><Unknown/></Slot>"#.into(),
        r#"<Wrap><Slot name="coat" itemId="1"/></Wrap>"#.into(),
        r#"<Slot xmlns="urn:unknown" name="coat" itemId="1"/>"#.into(),
        r#"<Slot name="coat" itemId="01"/>"#.into(),
        r#"<Slot name="coat" itemId="2"/>"#.into(),
        format!("{SLOT}<SocketIdURL nodeId=\"01\" itemPbURL=\"\"/>"),
        format!(
            "{SLOT}<SocketIdURL nodeId=\"7960\" itemPbURL=\"\" name=\"Display only\" itemId=\"2\"/>"
        ),
    ] {
        assert!(!complete(&run(&xml(RAW, &slots), &fixture), 0), "{slots}");
    }
}

#[test]
fn numeric_aliases_multiple_item_containers_and_source_overlays_cannot_close() {
    let fixture = fixture();
    let original = xml(RAW, SLOT);
    for changed in [
        original.replace("</Items>", &format!("<Item id=\"01\">{RAW}</Item></Items>")),
        original.replace("</Items>", &format!("<Item id=\"1\">{RAW}</Item></Items>")),
        original.replace("</Items>", "<ItemSet id=\"01\"/></Items>"),
        original.replace(
            "</PathOfBuilding2>",
            "<Items><ItemSet id=\"2\"/></Items></PathOfBuilding2>",
        ),
        original.replace(
            "</PathOfBuilding2>",
            "<Items xmlns=\"urn:unreviewed\"><ItemSet id=\"2\"/></Items></PathOfBuilding2>",
        ),
        original.replace("<Item id=\"1\">", "<Item id=\"1\" variant=\"1\">"),
        original.replace("</Item>", "<Unknown/></Item>"),
        original.replace("</Item>", "<ModRange id=\"1\" range=\"NaN\"/></Item>"),
        original.replace("</Items>", "<Slot name=\"coat\" itemId=\"1\"/></Items>"),
        original.replace("<Items ", "<Items unknown=\"1\" "),
    ] {
        assert!(!complete(&run(&changed, &fixture), 0), "{changed}");
    }
}

#[test]
fn archived_occupied_item_and_independent_incomplete_set_do_not_poison_proven_set() {
    let fixture = fixture();
    let other = SOCKET_RAW.replacen("Rune: None", "Rune: Occupied", 1);
    let input = xml(RAW, SLOT).replace("</Items>", &format!("<Item id=\"2\">{other}</Item><ItemSet id=\"2\"><Slot name=\"coat\" itemId=\"2\"/></ItemSet></Items>"));
    let result = run(&input, &fixture);
    assert!(complete(&result, 0));
    assert!(!complete(&result, 1));
    assert_eq!(result.draft().input().equipment.members.len(), 2);
}

#[test]
fn missing_mapping_or_scope_never_supplies_inventory_authority() {
    let fixture = fixture();
    let mut policy = fixture.policy.clone();
    policy.equipment_loadouts.clear();
    assert!(!complete(
        &normalize(
            &xml(RAW, SLOT),
            &fixture,
            &policy,
            NormalizationLimits::default()
        )
        .unwrap(),
        0
    ));
}

#[test]
fn omitted_policy_roundtrips_original_bytes_digest_and_allocations() {
    let fixture = fixture();
    let mut old = fixture.policy.clone();
    old.equipment_membership = None;
    let encoded = serde_json::to_vec(&old).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert!(value.get("equipment_membership").is_none());
    let decoded: NormalizationPolicy = serde_json::from_value(value).unwrap();
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), encoded);
    let original = normalize(
        &xml(RAW, SLOT),
        &fixture,
        &old,
        NormalizationLimits::default(),
    )
    .unwrap();
    let restored = normalize(
        &xml(RAW, SLOT),
        &fixture,
        &decoded,
        NormalizationLimits::default(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_vec(original.draft().input()).unwrap(),
        serde_json::to_vec(restored.draft().input()).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(original.sidecar()).unwrap(),
        serde_json::to_vec(restored.sidecar()).unwrap()
    );
    assert!(!complete(&original, 0));
}

#[test]
fn injected_base_facts_are_exact_unique_schema_bound_and_bounded() {
    let fixture = fixture();
    let original = xml(RAW, SLOT);
    for case in 0..6 {
        let mut policy = fixture.policy.clone();
        let Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 {
            definitions,
            templates,
            source_base_names,
            loader_jewel_fallback_titles,
        }) = &mut policy.equipment_membership
        else {
            unreachable!()
        };
        match case {
            0 => definitions.content_sha256 = "a".repeat(64),
            1 => templates.push(templates[0].clone()),
            2 => {
                source_base_names.pop();
                source_base_names.remove(0);
            }
            3 => loader_jewel_fallback_titles.push(loader_jewel_fallback_titles[0].clone()),
            4 => {
                templates[0].base_name =
                    "x".repeat(OwnedMappingLimits::default().max_string_bytes + 1)
            }
            5 => {
                templates[0].base_name = "[Wrapped Base]".into();
                source_base_names.push("[Wrapped Base]".into());
            }
            _ => unreachable!(),
        }
        assert!(normalize(&original, &fixture, &policy, NormalizationLimits::default()).is_err());
    }
    assert!(matches!(
        normalize(
            &original,
            &fixture,
            &fixture.policy,
            NormalizationLimits {
                max_work: 1,
                ..NormalizationLimits::default()
            }
        ),
        Err(NormalizationError::Limit(_))
    ));
}

#[test]
fn aggregate_policy_and_queries_fit_two_mib_while_lower_and_hard_caps_still_hold() {
    use poe_optimizer_core::owned_content::ContentDigestError;
    let fixture = fixture();
    let input = xml(RAW, SLOT);
    let limits = NormalizationLimits::default();
    assert_eq!(limits.max_policy_bytes, 2 * 1024 * 1024);
    assert_eq!(
        poe_optimizer_import::owned_tree_policy::TreePolicyLimits::default().max_base_policy_bytes,
        limits.max_policy_bytes
    );
    let queries: Vec<_> = (0..300)
        .map(|index| ImportQueryTemplate {
            id: QueryId::new(format!("budget-{index}")).unwrap(),
            metric: metric_selector(&"x".repeat(4000)),
            target: ImportQueryTarget::Player,
        })
        .collect();
    let bytes = serde_json::to_vec(&(&fixture.policy, &queries))
        .unwrap()
        .len();
    assert!(bytes > 1024 * 1024 && bytes < 2 * 1024 * 1024);
    let result = normalize_queries(&input, &fixture, &fixture.policy, &queries, limits).unwrap();
    assert_eq!(
        result.draft().input().query_presets.members[0]
            .queries
            .requests
            .members
            .len(),
        queries.len()
    );
    assert!(matches!(
        normalize_queries(&input, &fixture, &fixture.policy, &queries, NormalizationLimits { max_policy_bytes: 1024 * 1024, ..limits }),
        Err(NormalizationError::Digest(ContentDigestError::TooLarge { maximum })) if maximum == 1024 * 1024
    ));
    assert!(matches!(
        normalize(
            &input,
            &fixture,
            &fixture.policy,
            NormalizationLimits {
                max_policy_bytes: 2 * 1024 * 1024 + 1,
                ..limits
            }
        ),
        Err(NormalizationError::InvalidLimit("policy bytes"))
    ));
    let oversized: Vec<_> = queries
        .iter()
        .cloned()
        .chain(
            queries
                .iter()
                .enumerate()
                .map(|(index, query)| ImportQueryTemplate {
                    id: QueryId::new(format!("second-{index}")).unwrap(),
                    ..query.clone()
                }),
        )
        .collect();
    assert!(matches!(
        normalize_queries(&input, &fixture, &fixture.policy, &oversized, limits),
        Err(NormalizationError::Digest(ContentDigestError::TooLarge { maximum })) if maximum == 2 * 1024 * 1024
    ));
}
