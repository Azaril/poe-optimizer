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
    equipment: Vec<EquipmentUse>,
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
    // Only this rolled item record, the explicit uses, the selected character identity and
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
                weapon_loadouts: draft.weapon_loadouts.members.clone(),
                active_weapon_loadout: selected.build.active_weapon_loadout,
                items: vec![item.clone()],
                gems: vec![],
                equipment,
                allocations: vec![],
                skills: vec![],
                supports: vec![],
                authored_support_order: None,
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
    let report =
        bind_owned_request(endpoint.assembled().schema(), request, Default::default()).unwrap();
    assert_eq!(report.data_identity(), &endpoint.receipt().definitions);
    report
}
fn same_outcomes(a: &DefinitionBindingReport, b: &DefinitionBindingReport) {
    // Request/data commitments intentionally change across releases or loadouts.
    // Compare the complete binding result, retaining independent identity checks.
    assert_eq!(a.schema(), b.schema());
    assert_eq!(a.issues(), b.issues());
    assert_eq!(a.queries(), b.queries());
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
    let prior_path = path("POE_OPTIMIZER_TEST_SELECTED_EQUIPMENT_PLACEMENT_PRIOR");
    let next_path = path("POE_OPTIMIZER_TEST_SELECTED_EQUIPMENT_PLACEMENT_RELEASE");
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
    let mut accepted_slots = 0;
    let mut rejected_slots = 0;
    let mut scope_refusals = 0;
    for binding in b["items"].as_array().unwrap() {
        let template: ItemTemplateDefId =
            serde_json::from_value(binding["template"].clone()).unwrap();
        let allowed: Vec<EquipmentSlotDefId> =
            serde_json::from_value(binding["equipment_slots"].clone()).unwrap();
        assert!(!allowed.is_empty());
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
        let source_item = binding["source_item_id"].as_u64().unwrap();
        let expected_uses = match source_item {
            19 | 20 | 27 => 4,
            26 => 8,
            28 => 1,
            _ => panic!("unexpected source item"),
        };
        assert_eq!(uses.len(), expected_uses);
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
            if source_item == 26 { 2 } else { 1 }
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
        assert!(
            before_schema
                .equipment_slots
                .members
                .iter()
                .all(|s| allowed.contains(s))
        );
        if source_item == 26 {
            assert_eq!(before_schema.equipment_slots.members, allowed);
        } else {
            assert!(before_schema.equipment_slots.members.is_empty());
        }
        assert_eq!(
            after_schema.equipment_slots,
            DeclaredSet::complete(allowed.clone())
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
        let uses: Vec<_> = uses.into_iter().map(|u| u.to_resolved().unwrap()).collect();
        if source_item == 26 {
            let pair: Vec<_> = uses
                .iter()
                .filter(|u| selected_preset.equipment.members.contains(&u.id))
                .cloned()
                .collect();
            assert_eq!(pair.len(), 2);
            assert_ne!(pair[0].id, pair[1].id);
            assert_eq!(pair[0].item, pair[1].item);
            assert_ne!(pair[0].destination, pair[1].destination);
            let both = request(&draft, selection, &item, pair.clone());
            assert_eq!(both.build().input().equipment, pair);
            let before = bind(&prior, &both);
            let after = bind(&next, &both);
            // These exact ring slots were already known members of the old
            // Partial set. Closing the set must not manufacture an old error.
            assert_eq!(before.request_digest(), after.request_digest());
            assert_ne!(before.data_identity(), after.data_identity());
            same_outcomes(&before, &after);
            for removed in &pair {
                let kept = pair
                    .iter()
                    .filter(|u| u.id != removed.id)
                    .cloned()
                    .collect();
                let remaining = request(&draft, selection, &item, kept);
                let expected: Vec<_> = before
                    .issues()
                    .iter()
                    .filter(|i| i.site.location != BindingLocation::Equipment(removed.id))
                    .cloned()
                    .collect();
                assert_eq!(bind(&prior, &remaining).issues(), expected);
                assert_eq!(bind(&next, &remaining).issues(), expected);
                assert_eq!(remaining.build().input().equipment.len(), 1);
                assert_ne!(remaining.build().input().equipment[0].id, removed.id);
            }
        }
        for usage in uses {
            checked_uses += 1;
            let EquipmentDestination::CharacterSlot(slot) = &usage.destination else {
                panic!("actual character-slot use")
            };
            assert!(allowed.contains(slot));
            if source_item == 28 {
                assert_eq!(
                    usage.scope,
                    LoadoutScope::Selected {
                        loadouts: vec![selection.build.active_weapon_loadout],
                    }
                );
                assert_eq!(draft.weapon_loadouts.members.len(), 2);
            } else {
                assert_eq!(usage.scope, LoadoutScope::Shared);
            }
            let actual = request(&draft, selection, &item, vec![usage.clone()]);
            let before = bind(&prior, &actual);
            let after = bind(&next, &actual);
            let expected_before = if before_schema.equipment_slots.members.contains(slot) {
                vec![]
            } else {
                vec![expected(usage.id, slot, false)]
            };
            assert_eq!(destination(&before, usage.id), expected_before);
            assert!(destination(&after, usage.id).is_empty());
            assert_eq!(
                other_issues(&before, usage.id),
                other_issues(&after, usage.id)
            );
            assert!(after.queries().is_empty());
            assert_eq!(
                bind(&prior, &actual),
                before,
                "restoring the real predecessor preserves its exact known/unknown membership"
            );
            for (other, scope) in &slots {
                let mut candidate = usage.clone();
                candidate.destination = EquipmentDestination::CharacterSlot(other.clone());
                candidate.scope = match scope {
                    ScopePolicy::Selected => LoadoutScope::Selected {
                        loadouts: vec![selection.build.active_weapon_loadout],
                    },
                    ScopePolicy::Shared | ScopePolicy::Either => LoadoutScope::Shared,
                };
                let candidate = request(&draft, selection, &item, vec![candidate]);
                let old = bind(&prior, &candidate);
                let current = bind(&next, &candidate);
                let expected_old = if before_schema.equipment_slots.members.contains(other) {
                    vec![]
                } else {
                    vec![expected(usage.id, other, false)]
                };
                assert_eq!(destination(&old, usage.id), expected_old);
                if allowed.contains(other) {
                    accepted_slots += 1;
                    assert!(destination(&current, usage.id).is_empty());
                } else {
                    rejected_slots += 1;
                    assert_eq!(
                        destination(&current, usage.id),
                        [expected(usage.id, other, true)]
                    );
                    assert_eq!(current.schema(), SchemaBindingStatus::Invalid);
                }
                assert_eq!(
                    other_issues(&old, usage.id),
                    other_issues(&before, usage.id)
                );
                assert_eq!(
                    other_issues(&current, usage.id),
                    other_issues(&after, usage.id)
                );
            }
            let policy = slots.iter().find(|(id, _)| id == slot).unwrap().1;
            let mut wrong_scope = usage.clone();
            wrong_scope.scope = match policy {
                ScopePolicy::Selected => LoadoutScope::Shared,
                ScopePolicy::Shared => LoadoutScope::Selected {
                    loadouts: vec![selection.build.active_weapon_loadout],
                },
                ScopePolicy::Either => panic!("selected original slots have explicit scope"),
            };
            let wrong_scope = bind(&next, &request(&draft, selection, &item, vec![wrong_scope]));
            let scope_issue = BindingIssue {
                site: BindingSite {
                    location: BindingLocation::Equipment(usage.id),
                    facet: BindingFacet::Scope,
                },
                class: IssueClass::Invalid,
                code: BindingIssueCode::IncompatibleScope,
                subject: Some(SchemaSubject::Definition(slot.address())),
            };
            assert_eq!(
                wrong_scope
                    .issues()
                    .iter()
                    .filter(|i| i.site == scope_issue.site)
                    .cloned()
                    .collect::<Vec<_>>(),
                std::slice::from_ref(&scope_issue)
            );
            assert_eq!(
                wrong_scope
                    .issues()
                    .iter()
                    .filter(|i| i.site != scope_issue.site)
                    .cloned()
                    .collect::<Vec<_>>(),
                after.issues()
            );
            assert_eq!(wrong_scope.schema(), SchemaBindingStatus::Invalid);
            scope_refusals += 1;
            if source_item == 28 {
                // Weapon 1 and Weapon 1 Swap are the same typed slot with
                // distinct real loadout scopes. These explicit candidates test
                // structural binding only, not active execution or saved usage.
                for loadout in &draft.weapon_loadouts.members {
                    let mut scoped = usage.clone();
                    scoped.scope = LoadoutScope::Selected {
                        loadouts: vec![*loadout],
                    };
                    for active in &draft.weapon_loadouts.members {
                        let mut selected = selection;
                        selected.build.active_weapon_loadout = *active;
                        let candidate = request(&draft, selected, &item, vec![scoped.clone()]);
                        same_outcomes(&bind(&prior, &candidate), &before);
                        same_outcomes(&bind(&next, &candidate), &after);
                        assert_eq!(candidate.build().input().active_weapon_loadout, *active);
                        assert_eq!(candidate.build().input().equipment[0].scope, scoped.scope);
                    }
                }
            }
            let removed = request(&draft, selection, &item, vec![]);
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
    assert_eq!(checked_uses, 21);
    assert_eq!(accepted_slots, 37);
    assert_eq!(rejected_slots, 383);
    assert_eq!(scope_refusals, 21);
    let selected_preset = draft
        .equipment_presets
        .members
        .iter()
        .find(|p| p.id == selection.build.equipment)
        .unwrap();
    let mut templates = std::collections::BTreeSet::new();
    assert_eq!(selected_preset.equipment.members.len(), 9);
    for id in &selected_preset.equipment.members {
        let usage = draft
            .equipment
            .members
            .iter()
            .find(|u| u.id == *id)
            .unwrap()
            .to_resolved()
            .unwrap();
        let item = draft
            .items
            .members
            .iter()
            .find(|i| i.id == usage.item)
            .unwrap();
        let template = item.template.to_resolved().unwrap();
        let SchemaLookup::Known(schema) = next.assembled().schema().definition(&template) else {
            panic!("selected template schema");
        };
        let EquipmentDestination::CharacterSlot(slot) = &usage.destination else {
            panic!("selected character slot");
        };
        assert!(schema.equipment_slots.is_complete());
        assert!(schema.equipment_slots.members.contains(slot));
        templates.insert(template);
    }
    assert_eq!(templates.len(), 8);
    assert_eq!(release::inventory(&prior_path), before_inventory);
    assert_eq!(release::inventory(&next_path), after_inventory);
    println!(
        "All 9 selected uses / 8 templates have complete character-slot placement. 21 exact retained equipment uses, 37 allowed-slot bindings, 383 alternative-slot refusals, 21 scope refusals, distinct ring uses and both declared staff loadouts; prior Partial and removed-use controls preserve unrelated diagnostics. Binding only, no complete-build numerical claim."
    );
}
