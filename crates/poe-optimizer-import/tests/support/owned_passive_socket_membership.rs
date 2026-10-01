//! Empty Spec inventory proof is independent of node allocation interpretation.
use super::*;

fn reviewed_policy() -> NormalizationPolicy {
    let mut result = policy();
    result.passive_socket_membership =
        Some(PassiveSocketMembershipPolicy::PobExplicitEmptySpecSocketsV1 {});
    result
}

fn normalize(
    source: &ImportedBuildInstance,
    artifacts: &Artifacts,
    policy: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> Result<NormalizedImport, NormalizationError> {
    let evidence = SourceProjectEvidence::collect(source, SourceEvidenceLimits::default()).unwrap();
    let result = normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            tree: None,
            items: &empty_items(&artifacts.schema),
            item_source: &empty_item_source(&artifacts.schema),
            mappings: &artifacts.mapping,
            registry: &artifacts.registry,
            definitions: &artifacts.schema,
            roles: &artifacts.roles,
            rewards: &artifacts.rewards,
        },
        policy,
        &queries(),
        limits,
    )?;
    let retired = result
        .draft()
        .input()
        .allocation_presets
        .members
        .iter()
        .filter(|preset| matches!(preset.equipment.completion, DraftListCompletion::Complete))
        .count();
    origin_integrity_with_retired(source, &result, retired);
    Ok(result)
}

fn run(xml: &str) -> NormalizedImport {
    normalize(
        &source(xml, 0x72),
        &artifacts(false),
        &reviewed_policy(),
        NormalizationLimits::default(),
    )
    .unwrap()
}

fn complete(result: &NormalizedImport, index: usize) -> bool {
    matches!(
        result.draft().input().allocation_presets.members[index]
            .equipment
            .completion,
        DraftListCompletion::Complete
    )
}

fn xml(specs: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Tree activeSpec="2">{specs}</Tree><Items><Item id="1">Unconverted item</Item><ItemSet id="1"/></Items></PathOfBuilding2>"#
    )
}

const EMPTY: &str = r#"<Spec title="same title" nodes="1,2" treeVersion="injected" classId="7" classInternalId="7" ascendClassId="3" ascendancyInternalId="injected" secondaryAscendClassId="nil" masteryEffects=""><URL>opaque source URL</URL><Sockets/><Overrides><AttributeOverride strNodes="1" dexNodes="" intNodes="2"/></Overrides></Spec>"#;
const OCCUPIED: &str = r#"<Spec title="same title" nodes="1"><Sockets><Socket nodeId="1" itemId="1"/></Sockets></Spec>"#;

#[test]
fn exact_empty_inventory_retires_only_its_issue_and_preserves_all_ids() {
    let xml = xml(&format!("{OCCUPIED}{EMPTY}{OCCUPIED}{EMPTY}"));
    let source = source(&xml, 0x72);
    let artifacts = artifacts(false);
    let old = normalize(
        &source,
        &artifacts,
        &policy(),
        NormalizationLimits::default(),
    )
    .unwrap();
    let result = normalize(
        &source,
        &artifacts,
        &reviewed_policy(),
        NormalizationLimits::default(),
    )
    .unwrap();
    assert_eq!(result.allocator_after(), old.allocator_after());
    assert!(!complete(&result, 0));
    assert!(complete(&result, 1));
    assert!(!complete(&result, 2));
    assert!(complete(&result, 3));
    let mut expected = old.draft().input().clone();
    let mut expected_origins = old.sidecar().origins.clone();
    for index in [1, 3] {
        let DraftListCompletion::Pending { id, code } = &expected.allocation_presets.members[index]
            .equipment
            .completion
        else {
            panic!("old equipment membership must be unresolved")
        };
        assert_eq!(
            code.as_str(),
            "allocation-equipment-membership-not-converted"
        );
        for origin in &mut expected_origins {
            origin
                .links
                .retain(|link| !matches!(link, OwnedOriginTarget::Issue(found) if found == id));
        }
        expected.allocation_presets.members[index]
            .equipment
            .completion = DraftListCompletion::Complete;
    }
    assert_eq!(&expected, result.draft().input());
    assert_eq!(expected_origins, result.sidecar().origins);
    assert!(matches!(
        expected.equipment_presets.members[0].equipment.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(matches!(
        expected.allocation_presets.members[1]
            .allocations
            .completion,
        DraftListCompletion::Pending { .. }
    ));
}

#[test]
fn empty_is_independent_of_selection_title_node_values_and_container_spelling() {
    for spec in [
        "<Spec><Sockets/></Spec>",
        "<Spec><Sockets> \r\n\t </Sockets></Spec>",
        "<Spec nodes=\"unmapped,opaque\"><Overrides/><Sockets/></Spec>",
        EMPTY,
    ] {
        let input = xml(spec).replace("activeSpec=\"2\"", "activeSpec=\"999\"");
        assert!(complete(&run(&input), 0), "{spec}");
    }
}

#[test]
fn missing_occupied_stale_duplicate_and_lexically_unknown_socket_shapes_stay_pending() {
    for body in [
        "",
        "<Sockets/><Sockets/>",
        "<Sockets unknown=\"x\"/>",
        "<Sockets>not empty</Sockets>",
        "<Sockets><!-- hidden source content --></Sockets>",
        "<Sockets><![CDATA[]]></Sockets>",
        "<Sockets><Socket nodeId=\"1\" itemId=\"1\"/></Sockets>",
        "<Sockets><Socket nodeId=\"1\" itemId=\"0\"/></Sockets>",
        "<Sockets><Socket nodeId=\"1\" itemId=\"999\"/></Sockets>",
        "<Sockets><Socket nodeId=\"1\"/></Sockets>",
        "<Sockets><Socket itemId=\"1\"/></Sockets>",
        "<Sockets><Unknown/></Sockets>",
        "<Sockets xmlns=\"urn:unknown\"/>",
        "<x:Sockets xmlns:x=\"urn:unknown\"/>",
        "<Sockets/><Unknown><Socket nodeId=\"1\" itemId=\"1\"/></Unknown>",
        "<Sockets/><Spec><Sockets/></Spec>",
        "<Sockets/><WeaponSet1 nodes=\"1\"/>",
    ] {
        let input = xml(&format!("<Spec>{body}</Spec>"));
        assert!(!complete(&run(&input), 0), "{body}");
    }
}

#[test]
fn source_scope_ambiguity_never_proves_a_fresh_spec() {
    for input in [
        format!("<PathOfBuilding2><Tree>{EMPTY}</Tree><Tree>{EMPTY}</Tree></PathOfBuilding2>"),
        format!(
            "<PathOfBuilding2><Tree>{EMPTY}</Tree><x:Tree xmlns:x=\"urn:unknown\"/></PathOfBuilding2>"
        ),
        format!("<PathOfBuilding2><Tree>{EMPTY}</Tree>{EMPTY}</PathOfBuilding2>"),
        format!("<PathOfBuilding2>{EMPTY}</PathOfBuilding2>"),
        format!("<PathOfBuilding2><Unknown><Tree>{EMPTY}</Tree></Unknown></PathOfBuilding2>"),
        format!("<PathOfBuilding2><Tree><Unknown/>{EMPTY}</Tree></PathOfBuilding2>"),
        format!(
            "<PathOfBuilding2><Tree><x:Spec xmlns:x=\"urn:unknown\"/>{EMPTY}</Tree></PathOfBuilding2>"
        ),
        format!("<PathOfBuilding2><Tree unknown=\"x\">{EMPTY}</Tree></PathOfBuilding2>"),
    ] {
        let result = run(&input);
        assert!(
            result
                .draft()
                .input()
                .allocation_presets
                .members
                .iter()
                .all(|row| matches!(
                    row.equipment.completion,
                    DraftListCompletion::Pending { .. }
                )),
            "{input}"
        );
    }
}

#[test]
fn unknown_spec_facets_or_nonflat_reviewed_siblings_stay_pending_without_poisoning_other_specs() {
    for spec in [
        "<Spec unknown=\"x\"><Sockets/></Spec>",
        "<Spec title=\"&#49;\"><Sockets/></Spec>",
        "<Spec xmlns:x=\"urn:unknown\"><Sockets/></Spec>",
        "<Spec><Sockets/><URL/></Spec>",
        "<Spec><Sockets/><URL><Unknown/></URL></Spec>",
        "<Spec><Sockets/><URL>a</URL><URL>b</URL></Spec>",
        "<Spec><Sockets/><Overrides/><Overrides/></Spec>",
        "<Spec><Sockets/><Overrides><Unknown/></Overrides></Spec>",
        "<Spec><Sockets/><Overrides><AttributeOverride strNodes=\"\" intNodes=\"\"/></Overrides></Spec>",
        "<Spec><Sockets/><Overrides><AttributeOverride strNodes=\"\" intNodes=\"\" dexNodes=\"\"><Sockets/></AttributeOverride></Overrides></Spec>",
    ] {
        let result = run(&xml(&format!("{spec}{EMPTY}")));
        if spec.contains("xmlns") {
            assert!(
                result
                    .draft()
                    .input()
                    .allocation_presets
                    .members
                    .iter()
                    .all(|row| matches!(
                        row.equipment.completion,
                        DraftListCompletion::Pending { .. }
                    ))
            );
            continue;
        }
        assert!(!complete(&result, 0), "{spec}");
        assert!(complete(&result, 1), "{spec}");
    }
}

#[test]
fn duplicate_attributes_and_namespaced_roots_are_rejected_before_normalization() {
    for input in [
        format!(
            "<PathOfBuilding2><Tree activeSpec=\"1\" activeSpec=\"2\">{EMPTY}</Tree></PathOfBuilding2>"
        ),
        xml("<Spec title=\"a\" title=\"b\"><Sockets/></Spec>"),
        format!("<PathOfBuilding2 xmlns=\"urn:unknown\"><Tree>{EMPTY}</Tree></PathOfBuilding2>"),
    ] {
        assert!(decode_build(input.as_bytes()).is_err(), "{input}");
    }
}

#[test]
fn omission_retains_policy_wire_bytes_draft_digest_and_allocator_behavior() {
    let input = policy();
    let bytes = serde_json::to_vec(&input).unwrap();
    let mut historical = serde_json::to_value(&input).unwrap();
    assert!(historical.get("passive_socket_membership").is_none());
    historical
        .as_object_mut()
        .unwrap()
        .remove("passive_socket_membership");
    let historical: NormalizationPolicy = serde_json::from_value(historical).unwrap();
    assert_eq!(bytes, serde_json::to_vec(&historical).unwrap());
    let source = source(&xml(EMPTY), 0x72);
    let artifacts = artifacts(false);
    let first = normalize(&source, &artifacts, &input, NormalizationLimits::default()).unwrap();
    let second = normalize(
        &source,
        &artifacts,
        &historical,
        NormalizationLimits::default(),
    )
    .unwrap();
    assert_eq!(first.draft(), second.draft());
    assert_eq!(
        serde_json::to_vec(first.sidecar()).unwrap(),
        serde_json::to_vec(second.sidecar()).unwrap()
    );
    assert!(!complete(&first, 0));
}

#[test]
fn additional_source_walk_is_bounded_without_advancing_the_callers_allocator() {
    let source = source(
        &xml(&EMPTY.replace("opaque source URL", &"x".repeat(4096))),
        0x72,
    );
    let artifacts = artifacts(false);
    let original_allocator = *source.allocator_state();
    let reviewed = reviewed_policy();
    // Find the old adapter's successful work boundary. The new source proof
    // must charge additional bytes rather than inherit that boundary.
    let mut low = 0;
    let mut high = NormalizationLimits::default().max_work;
    while low < high {
        let mid = low + (high - low) / 2;
        if normalize(
            &source,
            &artifacts,
            &policy(),
            NormalizationLimits {
                max_work: mid,
                ..NormalizationLimits::default()
            },
        )
        .is_ok()
        {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    assert!(matches!(
        normalize(
            &source,
            &artifacts,
            &reviewed,
            NormalizationLimits {
                max_work: low,
                ..NormalizationLimits::default()
            }
        ),
        Err(NormalizationError::Limit("work"))
    ));
    assert_eq!(*source.allocator_state(), original_allocator);
    assert!(complete(
        &normalize(
            &source,
            &artifacts,
            &reviewed,
            NormalizationLimits::default()
        )
        .unwrap(),
        0
    ));
}
