//! Exact-release-bound admission component evidence, not complete-build coverage.
//! Receiver facts come from authenticated source observations. Native parent
//! preparation feeds its children; source accepted sets are expectations only.
#[allow(dead_code)]
#[path = "support/owned_djinn_support_preparation.rs"]
mod preparation_data;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;

#[path = "support/owned_bidding_delivery_native.rs"]
mod bidding_delivery;

use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_schema::*,
};
use poe_optimizer_data::owned_supports::OwnedSupportPreparation;
use poe_optimizer_engine::owned_supports::*;
use poe_optimizer_import::{
    owned_mapping::{
        ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, SourceComponent,
    },
    owned_skill_catalog::{OwnedGemMaterialization, OwnedGemRole, OwnedPrimarySkill},
};
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

const REPORT_HASH: &str = "2b43bd0ae0c52ac2cfa63978735f28df75a07153c62714cd40ef6e873d1a742d";
const STAGES: [&str; 3] = ["fresh", "rebuilt_once", "rebuilt_twice"];
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn decode<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(
            value.as_object().is_some_and(|v| v.is_empty()),
            "source empty array encoding"
        );
        &[]
    }
}
fn text(value: &Value) -> &str {
    value.as_str().unwrap()
}
fn number(value: &Value) -> u64 {
    value.as_u64().unwrap()
}
fn id<T: BuildInstanceId>(case: usize, ordinal: u64, lane: u64) -> T {
    // These are explicitly test-owned IDs; the source join, not the numeric ID,
    // authenticates correspondence. Lanes keep different instance kinds distinct.
    T::from_instance_id(
        InstanceId::from_parts(
            BuildLineage::from_bytes([107; 16]),
            1 + case as u64 * 1_000_000 + lane * 100_000 + ordinal,
        )
        .unwrap(),
    )
}
fn flag(flags: &Value, name: &str) -> bool {
    // This finite observer emits nil as an absent field and actual booleans.
    // Interpreting those facts is not a native default for missing build inputs.
    match flags.get(name) {
        None => false,
        Some(v) => v.as_bool().unwrap(),
    }
}
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

struct Fixture {
    preparation: OwnedSupportPreparation,
    report: Value,
    types: BTreeMap<(u64, String), OwnedDefinitionKey>,
    supports: BTreeMap<String, GemDefId>,
    unreviewed_support: GemDefId,
    families: Vec<Value>,
    trees: Vec<Value>,
}
impl Fixture {
    fn load() -> &'static Self {
        static FIXTURE: OnceLock<Fixture> = OnceLock::new();
        FIXTURE.get_or_init(|| {
            let root = Path::new(env!("CARGO_MANIFEST_DIR"));
            let prior = PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_DJINN_SUPPORT_PREPARATION_PRIOR")
                .expect("set exact checked Ice intrinsic release directory for optional Djinn admission test"));
            let before = release::inventory(&prior);
            let release = release::load(&prior);
            assert!(release.evaluation().is_none());
            let preparation = preparation_data::load(&release);
            assert_eq!(release::inventory(&prior), before, "component loading never mutates release artifacts");
            assert_eq!(preparation.input().definitions, release.receipt().definitions);
            assert_eq!(preparation.input().rules, *release.assembled().rules().identity());
            let bindings: Value = preparation_data::bindings();
            let mut types = BTreeMap::new();
            for row in rows(&bindings["types"]) {
                assert!(types.insert((number(&row["source_id"]), text(&row["source_name"]).to_owned()), decode(&row["support_type"])).is_none());
            }
            let mut supports = BTreeMap::new();
            for row in rows(&bindings["supports"]) {
                assert!(supports.insert(text(&row["source_effect"]).to_owned(), decode(&row["gem"])).is_none());
            }
            assert_eq!(supports.len(), 10);
            let actions: Value = read(root.join("data/owned/poe2/3887ae68/djinn-actions/bindings.json"));
            let trees: Value = read(root.join("data/owned/poe2/3887ae68/djinn-tree-grants/bindings.json"));
            let families = rows(&actions["families"]).to_vec();
            let trees = rows(&trees["families"]).to_vec();
            assert_eq!(families.len(), 2); assert_eq!(trees.len(), 2);
            // Authenticate every reused path against real, still-Partial schema.
            let schema = &release.input().recipe.schema;
            let slot = |value: &Value, kind: &str| {
                let address: SlotAddress = decode(&serde_json::json!({"kind":kind,"value":value}));
                assert_eq!(schema.slots.iter().filter(|row| row.address() == address).count(), 1);
            };
            for family in &families {
                let skill: SkillDefId = decode(&family["skill"]);
                let actual = schema.definitions.iter().find(|row| row.address() == skill.address()).unwrap();
                let DefinitionDescriptor::Skill(DefinitionEntry { schema: SchemaState::Known(actual), .. }) = actual else { panic!("actual known Djinn Skill") };
                assert!(!actual.declarations.parameters.is_complete(), "no completeness promoted for component admission");
                slot(&family["command"]["supply"], "skill_grant");
                slot(&family["command"]["entering_grant"], "grant");
                slot(&family["minion"]["population"], "actor");
                slot(&family["minion"]["entering_grant"], "grant");
                for child in rows(&family["actions"]) {
                    slot(&child["supply"], "skill_grant"); slot(&child["entering_grant"], "grant");
                }
            }
            for tree in &trees { slot(&tree["supply"], "skill_grant"); slot(&tree["grant"], "grant"); }
            let off = fs::read(root.join("runs/owned-djinn-support-preparation-source-02/source-jit-off.json")).unwrap();
            let on = fs::read(root.join("runs/owned-djinn-support-preparation-source-02/source-jit-on.json")).unwrap();
            assert_eq!(off.len(), 16_410_774); assert!(off == on, "both JIT reports must match byte-for-byte");
            assert_eq!(format!("{:x}", Sha256::digest(&off)), REPORT_HASH);
            let report: Value = serde_json::from_slice(&off).unwrap();
            assert_eq!(report["source_revision"], "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4");
            assert_eq!(report["manifest_sha256"], "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675");
            assert_eq!(rows(&report["files"]).len(), 20);
            assert_eq!(report["lifecycle_stages"], serde_json::json!(STAGES));
            for name in ["business_wrappers", "source_tables_mutated", "native_build_parity", "native_inventory_authority", "numerical_delivery_authority", "canonical_parity_lifecycle_selected"] { assert_eq!(report[name], false); }
            for source in rows(&report["original_sources"]) {
                assert_eq!(format!("{:x}", Sha256::digest(fs::read(root.join(text(&source["path"]))).unwrap())), text(&source["sha256"]));
            }
            assert_eq!(rows(&report["original_sources"]).len(), 5);
            assert_eq!(rows(&report["cases"]).len(), 12);
            // Every actual candidate retains its exact saved Gem tuple and primary
            // effect role. Display names and the reviewed effect map alone cannot
            // authorize a different source occurrence.
            let mut observed_effects = BTreeSet::new();
            for case in rows(&report["cases"]) { for stage in STAGES {
                for context in rows(&case["states"][stage]["preparation"]["contexts"]) {
                    for candidate in rows(&context["candidates"]) {
                        let attributes=&candidate["source"]["saved_attributes"];
                        let effect=text(&candidate["effect"]);
                        assert_eq!(attributes["skillId"], effect);
                        let selector=ExternalSelector::Definition(ExternalOwnerSelector::Gem {
                            game_id:SourceComponent::Text(text(&attributes["gemId"]).to_owned()),
                            variant_id:SourceComponent::Text(text(&attributes["variantId"]).to_owned()),
                        });
                        let Some(MappingOutcome::Mapped {target:SchemaSubject::Definition(DefinitionAddress::Gem(gem)),basis:MappingBasis::Exact})=release.mapping().lookup(&selector) else {panic!("exact physical support mapping")};
                        assert_eq!(gem,&supports[effect]);
                        let role=release.roles().role(gem).unwrap();
                        assert_eq!(role.role,OwnedGemRole::Known(AuthoredGemRole::SupportAssignment));
                        assert_eq!(role.materialization,OwnedGemMaterialization::Physical);
                        let OwnedPrimarySkill::Known(primary)=&role.primary else {panic!("known support effect")};
                        let selector=ExternalSelector::Definition(ExternalOwnerSelector::Skill{effect_id:SourceComponent::Text(effect.to_owned())});
                        assert!(matches!(release.mapping().lookup(&selector),Some(MappingOutcome::Mapped{target:SchemaSubject::Definition(DefinitionAddress::Skill(id)),basis:MappingBasis::Exact}) if id==primary));
                        observed_effects.insert(effect.to_owned());
                    }
                }
            } }
            assert_eq!(observed_effects,supports.keys().cloned().collect());
            let unreviewed_support=schema.definitions.iter().find_map(|row| {
                let DefinitionDescriptor::Gem(DefinitionEntry{id,schema:SchemaState::Known(schema)})=row else{return None};
                (schema.roles.contains(&AuthoredGemRole::SupportAssignment) && preparation.preparation_for(id).is_none()).then(||id.clone())
            }).expect("actual known support outside this finite reviewed preparation artifact");
            Self { preparation, report, types, supports, unreviewed_support, families, trees }
        })
    }
    fn type_set(&self, source: &Value) -> DeclaredSet<OwnedDefinitionKey> {
        assert_eq!(source["present"], true);
        let values: Vec<_> = rows(&source["values"])
            .iter()
            .map(|row| {
                assert_eq!(row["value"], true);
                self.types[&(number(&row["id"]), text(&row["name"]).to_owned())].clone()
            })
            .collect();
        assert_eq!(values.iter().collect::<BTreeSet<_>>().len(), values.len());
        DeclaredSet::complete(values)
    }
    fn types(&self, source: &Value) -> SupportTypeContext {
        SupportTypeContext {
            skill_types: self.type_set(&source["skill_types"]),
            minion_types: match source["minion_types"]["present"].as_bool().unwrap() {
                true => Some(self.type_set(&source["minion_types"])),
                false => {
                    assert!(rows(&source["minion_types"]["values"]).is_empty());
                    None
                }
            },
        }
    }
    fn origins(&self, case: usize, context: &Value) -> Vec<ResolvedSupportOrigin> {
        rows(&context["candidates"])
            .iter()
            .enumerate()
            .map(|(position, row)| {
                assert_eq!(number(&row["index"]), position as u64 + 1);
                assert_eq!(row["exact_definition"], true);
                assert_eq!(row["source"]["enabled"], true);
                assert_eq!(row["source"]["effect"], row["effect"]);
                assert_eq!(row["source"]["group"], context["group"]["key"]);
                ResolvedSupportOrigin {
                    assignment: id(case, number(&row["source"]["source_ordinal"]), 2),
                    gem: self.supports[text(&row["effect"])].clone(),
                    enabled: Some(true),
                    // These are observed prepared support inputs; never raw-Gem fallbacks.
                    effective_level: Some(
                        BoundedInteger::new(row["level"].as_i64().unwrap()).unwrap(),
                    ),
                    effective_quality: Some(
                        FiniteQuantity::new(
                            row["quality"].as_f64().unwrap(),
                            self.preparation.input().quality_unit.clone(),
                        )
                        .unwrap(),
                    ),
                }
            })
            .collect()
    }
    fn target(&self, case: usize, context: &Value, contexts: &[Value]) -> SkillTarget {
        let effect = text(&context["effect"]);
        let family = self
            .families
            .iter()
            .find(|family| {
                family["skill_id"] == effect
                    || family["command"]["skill_id"] == effect
                    || rows(&family["actions"])
                        .iter()
                        .any(|child| child["skill_id"] == effect)
            })
            .unwrap();
        let source_context = if context["parent_present"] == true {
            let parent = &contexts[number(&context["parent_context"]) as usize - 1];
            assert_eq!(parent["effect"], family["skill_id"]);
            assert_eq!(parent["group"], context["group"]);
            assert_eq!(parent["mode"], context["mode"]);
            parent
        } else {
            context
        };
        let ordinal = number(&source_context["source"]["source_ordinal"]);
        let mut provider = ProviderKey {
            root: ProviderRoot::SkillUse(id(case, ordinal, 0)),
            grant_path: vec![],
        };
        let generated = context["group"]["source_present"] == true;
        let tree = self
            .trees
            .iter()
            .find(|tree| tree["key"] == family["key"])
            .unwrap();
        if generated {
            assert_eq!(
                text(&context["group"]["source"]),
                format!("Tree:{}", number(&tree["node_id"]))
            );
            provider.root = ProviderRoot::Allocation(id(case, ordinal, 1));
            if family["skill_id"] == effect {
                return SkillTarget::Generated(Box::new(GeneratedSkillKey {
                    provider,
                    slot: decode(&tree["supply"]),
                }));
            }
            provider.grant_path.push(decode(&tree["grant"]));
        } else {
            assert!(context["group"].get("source").is_none());
        }
        if family["skill_id"] == effect {
            return SkillTarget::Authored(id(case, ordinal, 0));
        }
        if family["command"]["skill_id"] == effect {
            return SkillTarget::Generated(Box::new(GeneratedSkillKey {
                provider,
                slot: decode(&family["command"]["supply"]),
            }));
        }
        assert_eq!(context["parent_present"], true);
        provider
            .grant_path
            .push(decode(&family["minion"]["entering_grant"]));
        let child = rows(&family["actions"])
            .iter()
            .find(|child| child["skill_id"] == effect)
            .unwrap();
        SkillTarget::Generated(Box::new(GeneratedSkillKey {
            provider,
            slot: decode(&child["supply"]),
        }))
    }
    fn receiving(
        &self,
        case: usize,
        context: &Value,
        contexts: &[Value],
        summoner: Option<SupportTypeContext>,
    ) -> SupportPreparationTarget {
        let flags = &context["constructor_flags"];
        let from_item = flag(flags, "effect_from_item")
            || flag(flags, "source_from_item")
            || text(&flags["mod_source"]).starts_with("Item");
        assert_eq!(
            context["parent_present"].as_bool().unwrap(),
            summoner.is_some()
        );
        SupportPreparationTarget {
            target: self.target(case, context, contexts),
            enabled: Some(true),
            types: self.types(&context["initial_definition"]),
            summoner,
            cannot_be_supported: Some(flag(flags, "cannot_be_supported")),
            has_gem: Some(flag(flags, "gem_data_present")),
            from_item: Some(from_item),
            is_player_actor: Some(flag(flags, "actor_is_enemy_player")),
        }
    }
    fn replay(&self, case_index: usize, stage: &str) -> Vec<PreparedSupports> {
        let observed = &self.report["cases"][case_index]["states"][stage]["preparation"];
        for flag in [
            "hook_removed",
            "jit_mode_preserved",
            "original_methods_preserved",
        ] {
            assert_eq!(observed[flag], true);
        }
        let contexts = rows(&observed["contexts"]);
        let mut prepared: Vec<PreparedSupports> = vec![];
        let mut selections: BTreeMap<(String, String), SelectedSupports> = BTreeMap::new();
        for (position, context) in contexts.iter().enumerate() {
            assert_eq!(number(&context["index"]), position as u64 + 1);
            for flag in [
                "exact_actor",
                "exact_constructor_object",
                "exact_group",
                "exact_parent",
            ] {
                assert_eq!(context[flag], true);
            }
            let origins = self.origins(case_index, context);
            let cache_key = (
                text(&context["mode"]).to_owned(),
                text(&context["group"]["key"]).to_owned(),
            );
            let selected = selections.entry(cache_key).or_insert_with(|| {
                let SupportSelectionOutcome::Known(selected) =
                    select_supports(&self.preparation, &origins, Default::default()).unwrap()
                else {
                    panic!("source candidate selection unresolved")
                };
                selected
            });
            assert_eq!(
                selected.origins(),
                origins,
                "one exact retained source selection shared with Command and children"
            );
            assert_eq!(
                selected.selected_origin_indices(),
                (0..origins.len()).collect::<Vec<_>>()
            );
            let summoner = if context["parent_present"] == true {
                let parent_index = number(&context["parent_context"]) as usize - 1;
                assert!(parent_index < prepared.len());
                let parent_source = &contexts[parent_index];
                let parent = &prepared[parent_index];
                let native = SupportTypeContext {
                    skill_types: DeclaredSet::complete(parent.final_types.clone()),
                    minion_types: self
                        .types(&parent_source["initial_definition"])
                        .minion_types,
                };
                assert_types_equal(&native, &self.types(&parent_source["constructor_types"]));
                for call in rows(&context["calls"]) {
                    assert_eq!(call["effective_type_owner"], "summoner");
                    assert_types_equal(&native, &self.types(&call["parent_prepared_types"]));
                }
                assert_eq!(context["source_instance_present"], false);
                assert_eq!(context["socket_group_present"], false);
                assert_eq!(context["source_owner_is_parent"], true);
                Some(native)
            } else {
                for call in rows(&context["calls"]) {
                    assert_eq!(call["effective_type_owner"], "self");
                }
                assert_eq!(context["source_instance_present"], true);
                None
            };
            let target = self.receiving(case_index, context, contexts, summoner);
            let SupportPreparationOutcome::Known(result) =
                prepare_selected_supports(&self.preparation, selected, &target, Default::default())
                    .unwrap()
            else {
                panic!("native admission unavailable for {}", context["effect"])
            };
            let expected_indices: Vec<_> = rows(&context["constructor_accepted"])
                .iter()
                .map(|v| number(v) as usize - 1)
                .collect();
            assert_eq!(context["constructor_accepted"], context["accepted_indices"]);
            let actual_indices: Vec<_> = result
                .selected
                .iter()
                .filter(|p| p.applicable)
                .map(|p| p.origin_index)
                .collect();
            assert_eq!(
                actual_indices,
                expected_indices,
                "case{case_index}/{stage}/context{} exact admission",
                position + 1
            );
            let actual_assignments: Vec<_> = result
                .selected
                .iter()
                .filter(|p| p.applicable)
                .map(|p| p.assignment)
                .collect();
            let expected_assignments: Vec<_> = expected_indices
                .iter()
                .map(|&i| origins[i].assignment)
                .collect();
            assert_eq!(actual_assignments, expected_assignments);
            for (position, candidate) in rows(&context["candidates"]).iter().enumerate() {
                assert_eq!(candidate["accepted"], expected_indices.contains(&position));
            }
            assert!(result.final_types_complete);
            assert_eq!(
                result.final_types.iter().collect::<BTreeSet<_>>(),
                self.type_set(&context["constructor_types"]["skill_types"])
                    .members
                    .iter()
                    .collect::<BTreeSet<_>>()
            );
            prepared.push(result);
        }
        prepared
    }
}
fn assert_types_equal(a: &SupportTypeContext, b: &SupportTypeContext) {
    assert_eq!(a.skill_types.closure, b.skill_types.closure);
    assert_eq!(
        a.skill_types.members.iter().collect::<BTreeSet<_>>(),
        b.skill_types.members.iter().collect::<BTreeSet<_>>()
    );
    match (&a.minion_types, &b.minion_types) {
        (None, None) => {}
        (Some(a), Some(b)) => {
            assert_eq!(a.closure, b.closure);
            assert_eq!(
                a.members.iter().collect::<BTreeSet<_>>(),
                b.members.iter().collect::<BTreeSet<_>>()
            );
        }
        _ => panic!("absent and present minion type inventories differ"),
    }
}

#[test]
#[ignore = "requires exact prior release env and both authenticated complete Djinn support source reports"]
fn actual_preparation_matches_all_source_contexts_and_native_parent_chaining() {
    let f = Fixture::load();
    let mut total = 0;
    for case in 0..12 {
        for stage in STAGES {
            let results = f.replay(case, stage);
            assert_eq!(
                results.len(),
                if [1, 2, 3].contains(&case) { 0 } else { 48 }
            );
            total += results.len();
        }
    }
    assert_eq!(total, 1296);
}

#[test]
#[ignore = "requires exact prior release env and both authenticated complete Djinn support source reports"]
fn exact_disable_controls_preserve_other_candidates_and_frost_type_addition() {
    let f = Fixture::load();
    for case in 5..11 {
        for stage in STAGES {
            let original = rows(&f.report["cases"][4]["states"][stage]["preparation"]["contexts"]);
            let changed =
                rows(&f.report["cases"][case]["states"][stage]["preparation"]["contexts"]);
            let control = &f.report["cases"][case]["control"];
            let ordinal = number(&control["source_ordinal"]);
            let before = f.replay(4, stage);
            let after = f.replay(case, stage);
            let candidate_ids = |context: &Value| {
                rows(&context["candidates"])
                    .iter()
                    .map(|v| number(&v["source"]["source_ordinal"]))
                    .collect::<Vec<_>>()
            };
            let accepted_ids = |context: &Value| {
                rows(&context["candidates"])
                    .iter()
                    .filter(|v| v["accepted"] == true)
                    .map(|v| number(&v["source"]["source_ordinal"]))
                    .collect::<Vec<_>>()
            };
            let mut affected = 0;
            for (i, (a, b)) in original.iter().zip(changed).enumerate() {
                for field in [
                    "effect",
                    "mode",
                    "group",
                    "parent_context",
                    "initial_definition",
                ] {
                    assert_eq!(a.get(field), b.get(field));
                }
                let expected: Vec<_> = candidate_ids(a)
                    .into_iter()
                    .filter(|&id| id != ordinal)
                    .collect();
                assert_eq!(candidate_ids(b), expected);
                assert_eq!(
                    accepted_ids(b),
                    accepted_ids(a)
                        .into_iter()
                        .filter(|&id| id != ordinal)
                        .collect::<Vec<_>>()
                );
                let target_affected = candidate_ids(a).contains(&ordinal);
                affected += usize::from(target_affected);
                if control["support"] == "SupportChillingIcePlayer"
                    && target_affected
                    && a["effect"] != "CommandWaterDjinnBubblePlayer"
                {
                    let frost = f.types[&(167, "CreatesGroundEffect".to_owned())].clone();
                    assert!(
                        !f.types(&a["initial_definition"])
                            .skill_types
                            .members
                            .contains(&frost)
                    );
                    assert!(before[i].final_types.contains(&frost));
                    assert!(!after[i].final_types.contains(&frost));
                } else {
                    assert_eq!(before[i].final_types, after[i].final_types);
                }
            }
            assert!(
                affected >= 10,
                "disable control retains exact source consumers"
            );
        }
    }
}

#[test]
#[ignore = "requires exact prior release env and both authenticated complete Djinn support source reports"]
fn duplicate_values_unknown_facts_and_disabled_origins_keep_native_contract_gates() {
    // These are native contract controls, not additional source observations.
    let f = Fixture::load();
    let contexts = rows(&f.report["cases"][4]["states"]["fresh"]["preparation"]["contexts"]);
    let context = contexts
        .iter()
        .find(|v| v["effect"] == "SummonSandDjinnPlayer" && !rows(&v["candidates"]).is_empty())
        .unwrap();
    let origins = f.origins(4, context);
    let mut unreviewed = origins[0].clone();
    unreviewed.gem = f.unreviewed_support.clone();
    assert!(matches!(
        select_supports(&f.preparation, &[unreviewed], Default::default()).unwrap(),
        SupportSelectionOutcome::Unresolved {
            reason: SupportPreparationGap::MissingDefinition,
            origin_index: Some(0)
        }
    ));
    let mut pair = vec![origins[0].clone(), origins[0].clone()];
    pair[1].assignment = id(99, 1, 2);
    pair[1].effective_level = None;
    assert!(matches!(
        select_supports(&f.preparation, &pair, Default::default()).unwrap(),
        SupportSelectionOutcome::Unresolved {
            reason: SupportPreparationGap::EffectiveLevel,
            origin_index: Some(1)
        }
    ));
    pair[1].effective_level = pair[0].effective_level;
    pair[1].effective_quality = None;
    assert!(matches!(
        select_supports(&f.preparation, &pair, Default::default()).unwrap(),
        SupportSelectionOutcome::Unresolved {
            reason: SupportPreparationGap::EffectiveQuality,
            origin_index: Some(1)
        }
    ));
    pair[1].effective_quality = pair[0].effective_quality.clone();
    let SupportSelectionOutcome::Known(tied) =
        select_supports(&f.preparation, &pair, Default::default()).unwrap()
    else {
        panic!()
    };
    assert_eq!(tied.selected_origin_indices(), [0]);
    pair.reverse();
    let SupportSelectionOutcome::Known(reversed) =
        select_supports(&f.preparation, &pair, Default::default()).unwrap()
    else {
        panic!()
    };
    assert_eq!(reversed.selected_origin_indices(), [0]);
    assert_ne!(
        tied.origins()[0].assignment,
        reversed.origins()[0].assignment
    );
    pair[0].enabled = Some(false);
    pair[0].effective_level = None;
    pair[0].effective_quality = None;
    let SupportSelectionOutcome::Known(disabled) =
        select_supports(&f.preparation, &pair, Default::default()).unwrap()
    else {
        panic!()
    };
    assert_eq!(disabled.disabled_origins(), [0]);
    assert_eq!(disabled.selected_origin_indices(), [1]);
    pair[0].enabled = None;
    assert!(matches!(
        select_supports(&f.preparation, &pair, Default::default()).unwrap(),
        SupportSelectionOutcome::Unresolved {
            reason: SupportPreparationGap::OriginEnabled,
            origin_index: Some(0)
        }
    ));
    let SupportSelectionOutcome::Known(selected) =
        select_supports(&f.preparation, &origins, Default::default()).unwrap()
    else {
        panic!()
    };
    let mut target = f.receiving(4, context, contexts, None);
    target.enabled = None;
    assert!(matches!(
        prepare_selected_supports(&f.preparation, &selected, &target, Default::default()).unwrap(),
        SupportPreparationOutcome::Unresolved {
            reason: SupportPreparationGap::TargetEnabled,
            origin_index: None
        }
    ));
    target.enabled = Some(false);
    assert!(matches!(
        prepare_selected_supports(&f.preparation, &selected, &target, Default::default()).unwrap(),
        SupportPreparationOutcome::Inactive { .. }
    ));
    target.enabled = Some(true);
    target.cannot_be_supported = None;
    assert!(matches!(
        prepare_selected_supports(&f.preparation, &selected, &target, Default::default()).unwrap(),
        SupportPreparationOutcome::Unresolved {
            reason: SupportPreparationGap::Applicability,
            origin_index: Some(0)
        }
    ));
    target.cannot_be_supported = Some(false);
    let subject = SchemaSubject::Definition(f.supports["SupportBiddingPlayerTwo"].address());
    target.types.skill_types = DeclaredSet::partial(
        vec![],
        vec![SchemaGap {
            subject,
            facet: SchemaFacet::InputSchema,
            code: key("test-unresolved-types"),
        }],
    );
    target.types.minion_types = None;
    assert!(matches!(
        prepare_selected_supports(&f.preparation, &selected, &target, Default::default()).unwrap(),
        SupportPreparationOutcome::Unresolved {
            reason: SupportPreparationGap::Applicability,
            origin_index: Some(0)
        }
    ));
}

#[test]
#[ignore = "requires exact prior release env and both authenticated complete Djinn support source reports"]
fn immutable_package_and_selection_support_parallel_reuse_and_bounded_failures() {
    let f = Fixture::load();
    let a = f.replay(4, "fresh");
    let b = f.replay(10, "fresh");
    assert_eq!(f.replay(4, "fresh"), a);
    assert_ne!(a, b);
    let parallel = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..48)
                .into_par_iter()
                .map(|i| (i % 2, f.replay(if i % 2 == 0 { 4 } else { 10 }, "fresh")))
                .collect::<Vec<_>>()
        });
    for (i, result) in parallel {
        assert_eq!(result, if i == 0 { a.clone() } else { b.clone() });
    }
    let contexts = rows(&f.report["cases"][4]["states"]["fresh"]["preparation"]["contexts"]);
    let context = contexts
        .iter()
        .find(|v| v["effect"] == "SummonSandDjinnPlayer" && !rows(&v["candidates"]).is_empty())
        .unwrap();
    let origins = f.origins(4, context);
    let limits = SupportPreparationLimits::default();
    let mut work = limits.max_work;
    let SupportSelectionOutcome::Known(selected) =
        select_supports_with_budget(&f.preparation, &origins, limits, &mut work).unwrap()
    else {
        panic!()
    };
    let selection_work = limits.max_work - work;
    assert!(selection_work > 0);
    let target = f.receiving(4, context, contexts, None);
    let before = work;
    let expected = prepare_selected_supports_with_budget(
        &f.preparation,
        &selected,
        &target,
        limits,
        &mut work,
    )
    .unwrap();
    let admission_work = before - work;
    assert!(admission_work > 0);
    let mut exact = selection_work + admission_work;
    let SupportSelectionOutcome::Known(repeated) =
        select_supports_with_budget(&f.preparation, &origins, limits, &mut exact).unwrap()
    else {
        panic!()
    };
    assert_eq!(
        prepare_selected_supports_with_budget(
            &f.preparation,
            &repeated,
            &target,
            limits,
            &mut exact
        )
        .unwrap(),
        expected
    );
    assert_eq!(exact, 0);
    assert_eq!(
        prepare_selected_supports_with_budget(
            &f.preparation,
            &selected,
            &target,
            limits,
            &mut exact
        ),
        Err(SupportPreparationError::Limit("work"))
    );
    let mut short = admission_work - 1;
    assert_eq!(
        prepare_selected_supports_with_budget(
            &f.preparation,
            &selected,
            &target,
            limits,
            &mut short
        ),
        Err(SupportPreparationError::Limit("work"))
    );
    assert_eq!(short, 0);
    assert_eq!(
        prepare_selected_supports(&f.preparation, &selected, &target, limits).unwrap(),
        expected,
        "failed attempt cannot contaminate shared immutable selection"
    );
}
