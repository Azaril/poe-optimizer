//! Finite schema-binding requests using actual Original05 records. This does not
//! finalize the imported build, complete other schemas, or prepare an evaluator.
#[path = "owned_canonical_instances.rs"]
mod canonical;
use super::{family, release};
use poe_optimizer_core::{
    build_identity::*, owned_binding::*, owned_build::*, owned_definitions::*, owned_draft::*,
    owned_schema::*,
};
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn path(variable: &str) -> PathBuf {
    let path = PathBuf::from(std::env::var_os(variable).expect(variable));
    if path.is_absolute() {
        path
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
    }
}
fn original(package: &Path) -> (DraftSessionInput, EvaluationSelection) {
    let output = package.parent().unwrap();
    let document: Value = read(output.join("original-05/draft.json"));
    let draft = serde_json::from_value(document["draft"].clone()).unwrap();
    (draft, read(output.join("selected-05.json")))
}

fn assert_same_instances(
    mut before: Value,
    before_lineage: BuildLineage,
    mut after: Value,
    after_lineage: BuildLineage,
    context: &str,
) {
    // Fresh publication imports have independent lineages. Authenticate every
    // instance against its own allocator, then normalize comparison copies
    // only. No local IDs, allocation watermarks or live requests are changed.
    let before_count = canonical::canonical_instances(&mut before, before_lineage, &[]);
    let after_count = canonical::canonical_instances(&mut after, after_lineage, &[]);
    assert!(before_count > 0, "{context}: expected instance references");
    assert_eq!(
        before_count, after_count,
        "{context}: instance reference inventory changed"
    );
    assert!(
        before == after,
        "{context}: content differs after authenticating and normalizing lineages"
    );
}

fn request(
    draft: &DraftSessionInput,
    selected: EvaluationSelection,
    item: &ItemRecord,
    usage: Option<EquipmentUse>,
) -> OwnedEvaluationRequest {
    let character = draft
        .character_presets
        .members
        .iter()
        .find(|p| p.id == selected.build.character)
        .unwrap();
    let scenario = &draft
        .scenario_presets
        .members
        .iter()
        .find(|p| p.id == selected.scenario)
        .unwrap()
        .scenario;
    assert!(
        draft
            .weapon_loadouts
            .members
            .contains(&selected.build.active_weapon_loadout)
    );
    let limits = OwnedInputLimits::default();
    // Only this rolled item record, one real use, the selected character identity and
    // enemy identity are in this explicit test request. Pending collection
    // inventories in the imported session are never promoted to Complete.
    OwnedEvaluationRequest::new(
        BuildSpec::new(
            BuildInput {
                allocator: draft.allocator,
                revision: draft.revision,
                game_version: draft.game_version.clone(),
                character: CharacterSpec {
                    class: character.class.to_resolved().unwrap(),
                    ascendancy: character.ascendancy.to_resolved().unwrap(),
                    level: character.level.to_resolved().unwrap(),
                    rewards: vec![],
                },
                weapon_loadouts: vec![selected.build.active_weapon_loadout],
                active_weapon_loadout: selected.build.active_weapon_loadout,
                items: vec![item.clone()],
                gems: vec![],
                equipment: usage.into_iter().collect(),
                allocations: vec![],
                skills: vec![],
                supports: vec![],
                support_origins: None,
                generated_inputs: None,
                payload_links: vec![],
                choices: vec![],
            },
            limits,
        )
        .unwrap(),
        ScenarioSpec::new(
            ScenarioInput {
                game_version: draft.game_version.clone(),
                enemy: scenario.enemy.to_resolved().unwrap(),
                assumptions: vec![],
                usage: vec![],
            },
            limits,
        )
        .unwrap(),
        QuerySpec::new(
            QueryInput {
                game_version: draft.game_version.clone(),
                requests: vec![],
            },
            limits,
        )
        .unwrap(),
        limits,
    )
    .unwrap()
}

fn bind(
    endpoint: &StagedOwnedRelease,
    request: &OwnedEvaluationRequest,
) -> DefinitionBindingReport {
    bind_owned_request(endpoint.assembled().schema(), request, Default::default()).unwrap()
}
fn destination(report: &DefinitionBindingReport, usage: ItemSlotUseId) -> Vec<BindingIssue> {
    report
        .issues()
        .iter()
        .filter(|issue| {
            issue.site.location == BindingLocation::Equipment(usage)
                && issue.site.facet == BindingFacet::Destination
        })
        .cloned()
        .collect()
}
fn other_issues(report: &DefinitionBindingReport, usage: ItemSlotUseId) -> Vec<BindingIssue> {
    report
        .issues()
        .iter()
        .filter(|issue| {
            !(issue.site.location == BindingLocation::Equipment(usage)
                && issue.site.facet == BindingFacet::Destination)
        })
        .cloned()
        .collect()
}
fn expected(usage: ItemSlotUseId, slot: &EquipmentSlotDefId, complete: bool) -> BindingIssue {
    BindingIssue {
        site: BindingSite {
            location: BindingLocation::Equipment(usage),
            facet: BindingFacet::Destination,
        },
        class: if complete {
            IssueClass::Invalid
        } else {
            IssueClass::Unresolved
        },
        code: if complete {
            BindingIssueCode::NotDeclared
        } else {
            BindingIssueCode::PartialMembership
        },
        subject: Some(SchemaSubject::Definition(slot.address())),
    }
}

pub fn check() {
    let prior_path = path("POE_OPTIMIZER_TEST_SELECTED_LIFE_ITEM_PLACEMENT_PRIOR");
    let next_path = path("POE_OPTIMIZER_TEST_SELECTED_LIFE_ITEM_PLACEMENT_RELEASE");
    let before_inventory = release::inventory(&prior_path);
    let after_inventory = release::inventory(&next_path);
    let prior = release::load(&prior_path);
    let next = release::load(&next_path);
    family::assert_endpoint(&next);
    let b: Value = family::read("bindings.json");
    assert_eq!(json!(prior.receipt().input), b["before"]);
    let (old_draft, old_selection) = original(&prior_path);
    let (draft, selection) = original(&next_path);
    let before_lineage = old_draft.allocator.lineage();
    let after_lineage = draft.allocator.lineage();
    assert_same_instances(
        json!(&old_draft),
        before_lineage,
        json!(&draft),
        after_lineage,
        "all imported records",
    );
    assert_same_instances(
        json!(old_selection),
        before_lineage,
        json!(selection),
        after_lineage,
        "saved selection",
    );
    let slots: Vec<_> = next
        .input()
        .recipe
        .schema
        .definitions
        .iter()
        .filter_map(|d| {
            if let DefinitionDescriptor::EquipmentSlot(d) = d {
                let SchemaState::Known(schema) = &d.schema else {
                    panic!("known canonical slot")
                };
                Some((d.id.clone(), schema.scope))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(slots.len(), 20);
    let mut checked_uses = 0;
    for binding in b["items"].as_array().unwrap() {
        let template: ItemTemplateDefId =
            serde_json::from_value(binding["template"].clone()).unwrap();
        let slot: EquipmentSlotDefId =
            serde_json::from_value(binding["equipment_slot"].clone()).unwrap();
        let old_item: Vec<_> = old_draft
            .items
            .members
            .iter()
            .filter(|i| i.template.to_resolved() == Some(template.clone()))
            .collect();
        let actual: Vec<_> = draft
            .items
            .members
            .iter()
            .filter(|i| i.template.to_resolved() == Some(template.clone()))
            .collect();
        assert_eq!(old_item.len(), 1);
        assert_eq!(actual.len(), 1);
        assert_eq!(json!(old_item[0].id), binding["original_item"]);
        assert_same_instances(
            binding["original_item"].clone(),
            before_lineage,
            json!(actual[0].id),
            after_lineage,
            "exact actual rolled item",
        );
        let item = actual[0]
            .to_resolved()
            .expect("actual item inputs resolve without filling any fields");
        let old_uses: Vec<_> = old_draft
            .equipment
            .members
            .iter()
            .filter(|u| u.item.to_resolved() == Some(old_item[0].id))
            .collect();
        let uses: Vec<_> = draft
            .equipment
            .members
            .iter()
            .filter(|u| u.item.to_resolved() == Some(item.id))
            .collect();
        assert_eq!(json!(old_uses), binding["original_uses"]);
        assert_eq!(uses.len(), 4);
        assert_same_instances(
            binding["original_uses"].clone(),
            before_lineage,
            json!(uses),
            after_lineage,
            "exact retained uses",
        );
        let selected_preset = draft
            .equipment_presets
            .members
            .iter()
            .find(|p| p.id == selection.build.equipment)
            .unwrap();
        assert_eq!(
            selected_preset
                .equipment
                .members
                .iter()
                .filter(|id| uses.iter().any(|u| u.id == **id))
                .count(),
            1
        );
        let SchemaLookup::Known(before_schema) = prior.assembled().schema().definition(&template)
        else {
            panic!("known prior")
        };
        let SchemaLookup::Known(after_schema) = next.assembled().schema().definition(&template)
        else {
            panic!("known successor")
        };
        assert!(!before_schema.equipment_slots.is_complete());
        assert!(before_schema.equipment_slots.members.is_empty());
        assert_eq!(
            after_schema.equipment_slots,
            DeclaredSet::complete(vec![slot.clone()])
        );
        let mut inverse = after_schema.clone();
        inverse.equipment_slots = before_schema.equipment_slots.clone();
        assert_eq!(&inverse, before_schema);
        assert!(!after_schema.modifiers.is_complete());
        assert!(!after_schema.socket_destinations.is_complete());
        let owner = SchemaSubject::Definition(template.address());
        let before_owner = prior
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|x| x.owner == owner)
            .unwrap();
        let after_owner = next
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|x| x.owner == owner)
            .unwrap();
        assert_eq!(before_owner, after_owner);
        assert!(!after_owner.programs.is_complete());
        for usage in uses {
            let usage = usage.to_resolved().unwrap();
            checked_uses += 1;
            assert_eq!(
                usage.destination,
                EquipmentDestination::CharacterSlot(slot.clone())
            );
            assert_eq!(usage.scope, LoadoutScope::Shared);
            let actual = request(&draft, selection, &item, Some(usage.clone()));
            let before = bind(&prior, &actual);
            let after = bind(&next, &actual);
            assert_eq!(
                destination(&before, usage.id),
                [expected(usage.id, &slot, false)]
            );
            assert!(destination(&after, usage.id).is_empty());
            assert_eq!(
                other_issues(&before, usage.id),
                other_issues(&after, usage.id)
            );
            assert!(after.queries().is_empty());
            assert_eq!(
                bind(&prior, &actual),
                before,
                "restoring the real Partial predecessor refuses membership again"
            );
            for (other, scope) in &slots {
                if *other == slot {
                    continue;
                }
                let mut wrong = usage.clone();
                wrong.destination = EquipmentDestination::CharacterSlot(other.clone());
                wrong.scope = match scope {
                    ScopePolicy::Selected => LoadoutScope::Selected {
                        loadouts: vec![selection.build.active_weapon_loadout],
                    },
                    ScopePolicy::Shared | ScopePolicy::Either => LoadoutScope::Shared,
                };
                let wrong = request(&draft, selection, &item, Some(wrong));
                let old = bind(&prior, &wrong);
                let rejected = bind(&next, &wrong);
                assert_eq!(
                    destination(&old, usage.id),
                    [expected(usage.id, other, false)]
                );
                assert_eq!(
                    destination(&rejected, usage.id),
                    [expected(usage.id, other, true)]
                );
                assert_eq!(
                    other_issues(&old, usage.id),
                    other_issues(&before, usage.id)
                );
                assert_eq!(
                    other_issues(&rejected, usage.id),
                    other_issues(&after, usage.id)
                );
                assert_eq!(rejected.schema(), SchemaBindingStatus::Invalid);
            }
            let removed = request(&draft, selection, &item, None);
            let remaining: Vec<_> = before
                .issues()
                .iter()
                .filter(|x| x.site.location != BindingLocation::Equipment(usage.id))
                .cloned()
                .collect();
            assert_eq!(bind(&prior, &removed).issues(), remaining);
            assert_eq!(bind(&next, &removed).issues(), remaining);
        }
    }
    assert_eq!(checked_uses, 12);
    assert_eq!(release::inventory(&prior_path), before_inventory);
    assert_eq!(release::inventory(&next_path), after_inventory);
    println!(
        "12 exact retained equipment uses, 228 alternative-slot refusals, Partial predecessor and removed-use controls; all unrelated diagnostics preserved. Binding only, no complete-build numerical claim."
    );
}
