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
    assert!(character.rewards.to_resolved().unwrap().is_empty());
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
    let prior_path = path("POE_OPTIMIZER_TEST_LEGGINGS_PLACEMENT_PRIOR");
    let next_path = path("POE_OPTIMIZER_TEST_LEGGINGS_PLACEMENT_RELEASE");
    let prior_inventory = release::inventory(&prior_path);
    let next_inventory = release::inventory(&next_path);
    let prior = release::load(&prior_path);
    let next = release::load(&next_path);
    family::assert_endpoint(&next);
    let bindings: Value = family::read("bindings.json");
    assert_eq!(json!(prior.receipt().input), bindings["before"]);
    let (old_draft, old_selection) = original(&prior_path);
    let (draft, selected) = original(&next_path);
    let old_lineage = old_draft.allocator.lineage();
    let lineage = draft.allocator.lineage();
    assert_same_instances(
        json!(&old_draft),
        old_lineage,
        json!(&draft),
        lineage,
        "publication preserves every retained original record",
    );
    assert_same_instances(
        json!(old_selection),
        old_lineage,
        json!(selected),
        lineage,
        "publication preserves the saved selection",
    );

    let candidates: Vec<_> = draft
        .items
        .members
        .iter()
        .filter(|i| i.id.local() == 0x028e)
        .collect();
    assert_eq!(candidates.len(), 1);
    let old_candidates: Vec<_> = old_draft
        .items
        .members
        .iter()
        .filter(|i| i.id.local() == 0x028e)
        .collect();
    assert_eq!(old_candidates.len(), 1);
    assert_eq!(
        json!(old_candidates[0].id),
        bindings["original_item"],
        "authored item identity joins the actual predecessor"
    );
    assert_same_instances(
        bindings["original_item"].clone(),
        old_lineage,
        json!(candidates[0].id),
        lineage,
        "authored item identity joins the actual successor",
    );
    let item = candidates[0]
        .to_resolved()
        .expect("actual Leggings record inputs are resolved");
    assert_eq!(item.template.key().as_str(), "def.0000000000001e0e");
    let boots: EquipmentSlotDefId =
        serde_json::from_value(bindings["equipment_slot"].clone()).unwrap();
    let uses: Vec<_> = draft
        .equipment
        .members
        .iter()
        .filter(|u| u.item.to_resolved() == Some(item.id))
        .collect();
    assert_eq!(
        uses.iter().map(|u| u.id.local()).collect::<Vec<_>>(),
        [0x02ca, 0x02d4, 0x02de, 0x02e8]
    );
    let old_uses: Vec<_> = old_draft
        .equipment
        .members
        .iter()
        .filter(|u| u.item.to_resolved() == Some(old_candidates[0].id))
        .collect();
    assert!(
        json!(old_uses) == bindings["original_uses"],
        "authored use rows join the actual predecessor exactly"
    );
    assert_same_instances(
        bindings["original_uses"].clone(),
        old_lineage,
        json!(uses),
        lineage,
        "authored use rows join the actual successor",
    );
    let presets: Vec<_> = draft
        .equipment_presets
        .members
        .iter()
        .filter(|p| {
            p.equipment
                .members
                .iter()
                .any(|id| uses.iter().any(|u| u.id == *id))
        })
        .collect();
    assert_eq!(
        presets.iter().map(|p| p.id.local()).collect::<Vec<_>>(),
        [0x02a8, 0x02aa, 0x02ac, 0x02ae]
    );
    assert_eq!(selected.build.equipment, presets[0].id);
    for (preset, usage) in presets.iter().zip(&uses) {
        assert_eq!(
            preset
                .equipment
                .members
                .iter()
                .filter(|id| uses.iter().any(|u| u.id == **id))
                .copied()
                .collect::<Vec<_>>(),
            [usage.id]
        );
    }

    let SchemaLookup::Known(before_schema) = prior.assembled().schema().definition(&item.template)
    else {
        panic!("known predecessor item schema")
    };
    let SchemaLookup::Known(after_schema) = next.assembled().schema().definition(&item.template)
    else {
        panic!("known published item schema")
    };
    assert!(before_schema.equipment_slots.members.is_empty());
    assert!(!before_schema.equipment_slots.is_complete());
    assert_eq!(
        after_schema.equipment_slots,
        DeclaredSet::complete(vec![boots.clone()])
    );
    let mut inverse = after_schema.clone();
    inverse.equipment_slots = before_schema.equipment_slots.clone();
    assert_eq!(
        &inverse, before_schema,
        "all non-placement declarations stay exact"
    );
    for remains_partial in [
        !after_schema.socket_destinations.is_complete(),
        !after_schema.modifiers.is_complete(),
        !after_schema.quality.allowed_kinds.is_complete(),
        !after_schema.declarations.sockets.is_complete(),
        !after_schema.declarations.choices.is_complete(),
    ] {
        assert!(remains_partial);
    }
    let owner = SchemaSubject::Definition(item.template.address());
    let before_owner = prior
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == owner)
        .unwrap();
    let after_owner = next
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == owner)
        .unwrap();
    assert_eq!(before_owner, after_owner);
    assert!(!after_owner.programs.is_complete());

    let slots: Vec<_> = next
        .input()
        .recipe
        .schema
        .definitions
        .iter()
        .filter_map(|d| {
            if let DefinitionDescriptor::EquipmentSlot(d) = d {
                let SchemaState::Known(schema) = &d.schema else {
                    panic!("declared character slot schema")
                };
                Some((d.id.clone(), schema.scope))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(slots.len(), 20);
    assert_eq!(slots.iter().filter(|(slot, _)| *slot == boots).count(), 1);
    for draft_use in uses {
        let usage = draft_use.to_resolved().unwrap();
        assert_eq!(
            usage.destination,
            EquipmentDestination::CharacterSlot(boots.clone())
        );
        assert_eq!(usage.scope, LoadoutScope::Shared);
        let actual = request(&draft, selected, &item, Some(usage.clone()));
        let before = bind(&prior, &actual);
        let after = bind(&next, &actual);
        assert_eq!(
            destination(&before, usage.id),
            [expected(usage.id, &boots, false)]
        );
        assert!(destination(&after, usage.id).is_empty());
        assert_eq!(
            other_issues(&before, usage.id),
            other_issues(&after, usage.id)
        );
        assert!(after.queries().is_empty());
        // Returning to the actual predecessor restores the same diagnostic;
        // no mutable evaluator or derived result state is involved.
        assert_eq!(before, bind(&prior, &actual));

        for (slot, scope) in &slots {
            if *slot == boots {
                continue;
            }
            let mut wrong = usage.clone();
            wrong.destination = EquipmentDestination::CharacterSlot(slot.clone());
            // Match the declared scope so a separate scope violation cannot
            // mask the placement membership outcome under test.
            wrong.scope = match scope {
                ScopePolicy::Selected => LoadoutScope::Selected {
                    loadouts: vec![selected.build.active_weapon_loadout],
                },
                ScopePolicy::Shared | ScopePolicy::Either => LoadoutScope::Shared,
            };
            let wrong = request(&draft, selected, &item, Some(wrong));
            let unknown = bind(&prior, &wrong);
            let rejected = bind(&next, &wrong);
            assert_eq!(
                destination(&unknown, usage.id),
                [expected(usage.id, slot, false)]
            );
            assert_eq!(
                destination(&rejected, usage.id),
                [expected(usage.id, slot, true)]
            );
            assert_eq!(
                other_issues(&unknown, usage.id),
                other_issues(&before, usage.id)
            );
            assert_eq!(
                other_issues(&rejected, usage.id),
                other_issues(&after, usage.id)
            );
            assert_eq!(rejected.schema(), SchemaBindingStatus::Invalid);
        }
        let removed = request(&draft, selected, &item, None);
        let remaining: Vec<_> = before
            .issues()
            .iter()
            .filter(|issue| issue.site.location != BindingLocation::Equipment(usage.id))
            .cloned()
            .collect();
        // Removing the use removes all of its obligations, including unchanged
        // Partial required-value declarations; character issues remain exact.
        assert_eq!(bind(&prior, &removed).issues(), remaining);
        assert_eq!(bind(&next, &removed).issues(), remaining);
    }
    assert_eq!(prior_inventory, release::inventory(&prior_path));
    assert_eq!(next_inventory, release::inventory(&next_path));
    println!(
        "4 actual retained uses: Boots placement resolved; 19 other declared slots rejected per use; other issues, partial facets, programs and package bytes preserved. Binding only; no evaluator completeness claim."
    );
}
