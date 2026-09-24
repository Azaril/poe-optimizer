//! Multiple potential skills remain membership facts, never activation decisions.
use super::{
    scalar_families::recipe,
    skill_scopes::canonical_instances,
    support::{bundle, data, json, normalize, root, success},
};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::*,
    owned_draft::{DraftLimits, DraftListCompletion, decode_draft},
    owned_schema::*,
};
use poe_optimizer_data::skill_identities::SkillIdentityCatalog;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_gem_catalog::{GemEffectMembershipPolicy, PhysicalGemSchemaPolicy},
    owned_gem_schema::GemSchemaMigrationInput,
    owned_mapping::{
        ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, OwnedMappingIndex,
        SourceComponent,
    },
    owned_normalize::NormalizationPolicy,
    owned_recipe::assemble_owned_recipe,
    owned_skill_catalog::{
        OwnedGemMaterialization, OwnedGemRole, OwnedPrimarySkill, OwnedSkillRoleIndex,
    },
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn compile(cwd: &Path, prior: &Path, policy: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("compile-owned-gem-inputs")
        .arg(prior)
        .arg("--catalog")
        .arg(data().join("import/skill-identities.json"))
        .arg("--policy")
        .arg(policy)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
fn publish(
    cwd: &Path,
    command: &str,
    prior: &Path,
    flag: &str,
    input: &Path,
    output: &Path,
) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg(command)
        .arg(prior)
        .arg(flag)
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
fn pending(completion: &DraftListCompletion) {
    let DraftListCompletion::Pending { code, .. } = completion else {
        panic!("reviewed scalar members cannot close intrinsic input membership")
    };
    assert_eq!(code.as_str(), "gem-parameters-not-converted");
}

pub(super) fn check_multieffect_gem_inputs(cwd: &Path, prior: &Path) -> PathBuf {
    let policy_path = data().join("multieffect-support-gem-inputs/policy.json");
    let policy_bytes = fs::read(&policy_path).unwrap();
    let authored: PhysicalGemSchemaPolicy = serde_json::from_slice(&policy_bytes).unwrap();
    let catalog = SkillIdentityCatalog::new(
        serde_json::from_value(json(data().join("import/skill-identities.json"))).unwrap(),
    )
    .unwrap();
    let checked_prior = assemble_owned_recipe(recipe(prior), Default::default()).unwrap();
    let mapping = OwnedMappingIndex::new(
        serde_json::from_value(json(prior.join("mapping.json"))).unwrap(),
        checked_prior.registry(),
        checked_prior.schema(),
        Default::default(),
    )
    .unwrap();
    let roles = OwnedSkillRoleIndex::new(
        serde_json::from_value(json(prior.join("roles.json"))).unwrap(),
        &mapping,
        checked_prior.schema(),
        Default::default(),
    )
    .unwrap();
    // Expected owned identities come from authored source keys and the checked
    // predecessor mapping, independently of the compiler's emitted migration.
    assert_eq!(
        authored.effect_membership,
        GemEffectMembershipPolicy::ResolvedPotentialSkillsV1
    );
    let mut expected_owners = BTreeMap::new();
    let mut expected_by_source = BTreeMap::new();
    let mut generated_only = 0;
    let mut reordered_primary = 0;
    for key in &authored.source_gems {
        let source = catalog.gem_by_key(key).unwrap();
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text(source.game_id.clone()),
            variant_id: SourceComponent::Text(source.variant_id.clone()),
        });
        let Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)),
            basis: MappingBasis::Exact,
        }) = mapping.lookup(&selector)
        else {
            panic!("authored source Gem has no exact prior identity")
        };
        let skill = ExternalSelector::Definition(ExternalOwnerSelector::Skill {
            effect_id: SourceComponent::Text(source.primary_effect_id.clone()),
        });
        let Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Skill(primary)),
            basis: MappingBasis::Exact,
        }) = mapping.lookup(&skill)
        else {
            panic!("authored primary has no exact prior identity")
        };
        let role = roles.role(gem).unwrap();
        assert_eq!(role.materialization, OwnedGemMaterialization::Physical);
        assert_eq!(
            role.role,
            OwnedGemRole::Known(AuthoredGemRole::SupportAssignment)
        );
        assert_eq!(role.primary, OwnedPrimarySkill::Known(primary.clone()));
        assert!(matches!(
            checked_prior.schema().definition(gem),
            SchemaLookup::Unmapped(_)
        ));
        // The catalogue retains declaration, construction and display order
        // separately. Owned potential membership contains every resolved Skill,
        // sorted by owned identity; display order cannot select a primary role.
        let additional: BTreeSet<_> = source.additional_effects.iter().collect();
        let constructed: BTreeSet<_> = source
            .constructed_additional_effects
            .iter()
            .map(|reference| &reference.id)
            .collect();
        let declared: BTreeSet<_> = source
            .declared_additional_effects
            .iter()
            .map(|reference| &reference.id)
            .collect();
        assert_eq!(constructed, additional);
        assert!(declared.is_subset(&constructed));
        assert!(source.declared_additional_stat_sets.is_empty());
        let potential: BTreeSet<_> = std::iter::once(&source.primary_effect_id)
            .chain(additional.iter().copied())
            .collect();
        assert_eq!(
            potential,
            source.effect_list.iter().collect::<BTreeSet<_>>()
        );
        assert_eq!(potential.len(), source.effect_list.len());
        assert!(potential.len() >= 2);
        let mut skills = BTreeSet::new();
        for effect in potential {
            assert!(catalog.skill_by_id(effect).is_some());
            let selector = ExternalSelector::Definition(ExternalOwnerSelector::Skill {
                effect_id: SourceComponent::Text(effect.clone()),
            });
            let Some(MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Skill(skill)),
                basis: MappingBasis::Exact,
            }) = mapping.lookup(&selector)
            else {
                panic!("potential effect has no exact prior Skill mapping")
            };
            assert!(
                skills.insert(skill.clone()),
                "distinct potential effects collided"
            );
        }
        assert!(skills.contains(primary));
        if source.declared_additional_effects.is_empty() {
            generated_only += 1;
            assert!(key.starts_with("Metadata/Items/Gems/SkillGemBarbsSupport"));
            assert_eq!(source.constructed_additional_effects.len(), 1);
        }
        if key == "Metadata/Items/Gems/SkillGemEmpoweredSparksSupport" {
            reordered_primary += 1;
            assert_eq!(source.effect_list.last(), Some(&source.primary_effect_id));
            assert_ne!(source.effect_list.first(), Some(&source.primary_effect_id));
            assert_eq!(skills.len(), 3);
        }
        assert!(
            expected_by_source
                .insert(key.clone(), gem.clone())
                .is_none()
        );
        assert!(
            expected_owners
                .insert(gem.clone(), skills.into_iter().collect::<Vec<_>>())
                .is_none()
        );
    }
    assert_eq!(expected_owners.len(), authored.source_gems.len());
    assert_eq!(expected_owners.len(), 52);
    assert_eq!(expected_owners.values().map(Vec::len).sum::<usize>(), 105);
    assert_eq!((generated_only, reordered_primary), (3, 1));
    let excluded = catalog
        .gem_by_key("Metadata/Items/Gems/SkillGemConcussiveRunesSupport")
        .unwrap();
    assert!(!expected_by_source.contains_key(&excluded.key));
    assert_eq!(excluded.declared_additional_effects.len(), 1);
    assert!(
        catalog
            .skill_by_id(&excluded.declared_additional_effects[0].id)
            .is_none()
    );
    let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
        game_id: SourceComponent::Text(excluded.game_id.clone()),
        variant_id: SourceComponent::Text(excluded.variant_id.clone()),
    });
    let Some(MappingOutcome::Mapped {
        target: SchemaSubject::Definition(DefinitionAddress::Gem(excluded_gem)),
        basis: MappingBasis::Exact,
    }) = mapping.lookup(&selector)
    else {
        panic!("excluded source Gem has no retained identity")
    };
    assert!(matches!(
        checked_prior.schema().definition(excluded_gem),
        SchemaLookup::Unmapped(_)
    ));
    assert_eq!(authored.parameters.len(), 2);
    assert!(matches!(
        authored.parameters[0].schema.value,
        ValueSchema::Boolean
    ));
    let ValueSchema::Quantity(delta_range) = &authored.parameters[1].schema.value else {
        panic!("authored corruption delta must be a quantity")
    };
    let delta_unit = delta_range.minimum.unit().clone();
    let SchemaLookup::Known(unit) = checked_prior.schema().definition(&delta_unit) else {
        panic!("Count unit")
    };
    assert_eq!(unit.dimension, UnitDimension::Count);
    assert_eq!(
        authored.parameters[1].value.numeric_aliases,
        [
            poe_optimizer_import::owned_value_policy::NumericTokenAlias {
                token: "nil".into(),
                replacement: "0".into()
            }
        ]
    );
    let mut expected_registry = checked_prior.registry().clone();
    let mut expected_parameters = vec![];
    for gem in expected_owners.keys() {
        for parameter in &authored.parameters {
            let id = expected_registry
                .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Gem(gem.clone()))
                .unwrap();
            expected_parameters.push(DefinitionEntry {
                id,
                schema: SchemaState::Known(parameter.schema.clone()),
            });
        }
    }
    let prior_bytes = bundle(prior);
    let prior_transition = json(prior.join("transition.json"));
    let compiled = cwd.join("multieffect-support-gem-inputs-compiled");
    let report = success(compile(cwd, prior, &policy_path, &compiled));
    assert_eq!(json(compiled.join("receipt.json")), report);
    assert_eq!(report["compilation"]["promoted_gems"], 52);
    assert_eq!(report["compilation"]["allocated_parameters"], 104);
    assert_eq!(report["calculation"], "not_run");
    assert_eq!(report["whole_build_parity"], "not_established");
    let migration: GemSchemaMigrationInput =
        serde_json::from_value(json(compiled.join("migration.json"))).unwrap();
    assert_eq!(migration.gems.len(), 52);
    assert_eq!(migration.parameters.len(), 104);
    let selected: BTreeSet<_> = migration.gems.iter().map(|gem| gem.id.clone()).collect();
    assert_eq!(
        selected,
        expected_owners.keys().cloned().collect::<BTreeSet<_>>()
    );
    assert!(
        migration.parameters == expected_parameters,
        "migrated parameter IDs/order/schemas differ from authored policy"
    );

    let schema_path = cwd.join("multieffect-support-gem-inputs-schemas");
    let schema_report = success(publish(
        cwd,
        "migrate-owned-gem-schemas",
        prior,
        "--migration",
        &compiled.join("migration.json"),
        &schema_path,
    ));
    assert_eq!(schema_report["publication"], report["schema_publication"]);
    assert_eq!(
        schema_report["publication"]["before"],
        prior_transition["after"]
    );
    assert_eq!(
        schema_report["publication"]["schema_refinement"]["schema_version"],
        4
    );
    let output = cwd.join("multieffect-support-gem-inputs-successor");
    let final_report = success(publish(
        cwd,
        "publish-owned-normalization",
        &schema_path,
        "--normalization",
        &compiled.join("normalization.json"),
        &output,
    ));
    assert_eq!(final_report["publication"], report["input_publication"]);
    assert_eq!(
        final_report["publication"]["before"],
        schema_report["publication"]["after"]
    );
    for transition in [&schema_report["publication"], &final_report["publication"]] {
        assert_eq!(transition["query_rows"], 110);
        assert_eq!(transition["calculation"], "not_run");
        assert_eq!(transition["whole_build_parity"], "not_established");
    }

    // Independently account for every registry/schema change. Existing rules and
    // routing receive a new schema binding only; these parameters have no consumers.
    let (before, after) = (recipe(prior), recipe(&output));
    assert!(
        &after.registry == expected_registry.input(),
        "registry contains unrelated edits"
    );
    let mut restored = after.schema.clone();
    let mut promoted = 0;
    for row in &mut restored.definitions {
        let DefinitionDescriptor::Gem(gem) = row else {
            continue;
        };
        if !selected.contains(&gem.id) {
            continue;
        }
        let expected = migration
            .gems
            .iter()
            .find(|entry| entry.id == gem.id)
            .unwrap();
        assert_eq!(gem, expected);
        let SchemaState::Known(schema) = &gem.schema else {
            panic!("promoted Gem")
        };
        assert_eq!(schema.roles, [AuthoredGemRole::SupportAssignment]);
        assert_eq!(schema.level, authored.level);
        assert_eq!(schema.skills.members, expected_owners[&gem.id]);
        assert_eq!(schema.quality.presence, authored.quality_presence);
        assert_eq!(schema.quality.allowed_kinds.members, authored.quality_kinds);
        assert_eq!(
            schema.declarations.parameters.members,
            expected_parameters
                .iter()
                .filter(|entry| entry.id.declaration == SlotOwnerDefId::Gem(gem.id.clone()))
                .map(|entry| entry.id.clone())
                .collect::<Vec<_>>()
        );
        assert!(!schema.skills.is_complete());
        assert!(!schema.quality.allowed_kinds.is_complete());
        assert_eq!(schema.declarations.parameters.members.len(), 2);
        assert!(!schema.declarations.parameters.is_complete());
        assert!(!schema.declarations.choices.is_complete());
        assert!(!schema.declarations.grants.is_complete());
        assert!(!schema.declarations.actors.is_complete());
        assert!(!schema.declarations.skill_grants.is_complete());
        assert!(!schema.declarations.outputs.is_complete());
        assert!(!schema.declarations.sockets.is_complete());
        let old = before
            .schema
            .definitions
            .iter()
            .find(|entry| entry.address() == gem.id.address())
            .unwrap();
        assert!(
            matches!(old, DefinitionDescriptor::Gem(entry) if matches!(entry.schema, SchemaState::Unmapped { .. }))
        );
        *row = old.clone();
        promoted += 1;
    }
    assert_eq!(promoted, 52);
    let new_slots: BTreeSet<_> = migration
        .parameters
        .iter()
        .map(|entry| SlotAddress::Parameter(entry.id.clone()))
        .collect();
    assert_eq!(after.schema.slots.len(), before.schema.slots.len() + 104);
    for entry in &migration.parameters {
        assert!(
            after
                .schema
                .slots
                .contains(&SlotDescriptor::Parameter(entry.clone()))
        );
    }
    restored
        .slots
        .retain(|slot| !new_slots.contains(&slot.address()));
    assert!(restored == before.schema, "unrelated schema changed");
    let untouched = after
        .schema
        .definitions
        .iter()
        .find(|row| row.address() == excluded_gem.address())
        .unwrap();
    assert!(
        matches!(untouched, DefinitionDescriptor::Gem(entry) if matches!(entry.schema, SchemaState::Unmapped { .. }))
    );
    let mut rules = after.rules.clone();
    rules.definitions = before.rules.definitions.clone();
    assert!(rules == before.rules, "prior rule content changed");
    let mut routing = after.routing.clone();
    routing.definitions = before.routing.definitions.clone();
    assert!(routing == before.routing, "prior routing content changed");

    for (name, fields) in [
        ("mapping.json", &["definitions", "registry"][..]),
        ("roles.json", &["definitions", "mapping"][..]),
        ("rewards.json", &["definitions", "mapping"][..]),
        ("items.json", &["definitions"][..]),
        ("item-source.json", &["item_lines"][..]),
        (
            "tree-normalization.json",
            &["definitions", "mapping", "normalization", "registry"][..],
        ),
    ] {
        let old = json(prior.join(name));
        let mut new = json(output.join(name));
        for field in fields {
            assert_ne!(
                new[*field], old[*field],
                "{name}:{field} binding did not change"
            );
            if *field == "definitions" {
                assert_eq!(
                    new[*field],
                    serde_json::to_value(&after.rules.definitions).unwrap()
                );
            }
            new[*field] = old[*field].clone();
        }
        assert!(new == old, "unrelated {name} content changed");
    }
    let old_policy: NormalizationPolicy =
        serde_json::from_value(json(prior.join("normalization.json"))).unwrap();
    let current_policy: NormalizationPolicy =
        serde_json::from_value(json(output.join("normalization.json"))).unwrap();
    let current_inputs = current_policy.gem_inputs.as_ref().unwrap();
    assert_eq!(current_inputs.definitions, after.rules.definitions);
    assert_eq!(
        current_inputs.gems.len(),
        old_policy.gem_inputs.as_ref().unwrap().gems.len() + 52
    );
    for gem in &migration.gems {
        let rule = current_inputs
            .gems
            .iter()
            .find(|rule| rule.gem == gem.id)
            .unwrap();
        assert_eq!(rule.guards, authored.guards);
        assert_eq!(rule.parameters.len(), authored.parameters.len());
        let expected_slots: Vec<_> = expected_parameters
            .iter()
            .filter(|entry| entry.id.declaration == SlotOwnerDefId::Gem(gem.id.clone()))
            .map(|entry| &entry.id)
            .collect();
        for ((input, parameter), slot) in rule
            .parameters
            .iter()
            .zip(&authored.parameters)
            .zip(expected_slots)
        {
            assert_eq!(input.value, parameter.value);
            assert_eq!(&input.slot, slot);
        }
        assert_eq!(
            rule.parameters[0].value.tiers[0].selectors[0].name,
            "corrupted"
        );
        assert_eq!(
            rule.parameters[1].value.tiers[0].selectors[0].name,
            "corruptLevel"
        );
    }
    let mut restored_policy = current_policy.clone();
    let inputs = restored_policy.gem_inputs.as_mut().unwrap();
    inputs.gems.retain(|rule| !selected.contains(&rule.gem));
    inputs.definitions = old_policy.gem_inputs.as_ref().unwrap().definitions.clone();
    let (
        poe_optimizer_import::owned_normalize::GemQualityPolicy::Attributes(new),
        poe_optimizer_import::owned_normalize::GemQualityPolicy::Attributes(old),
    ) = (&mut restored_policy.gem_quality, &old_policy.gem_quality)
    else {
        panic!("quality policy")
    };
    new.definitions = old.definitions.clone();
    assert!(
        restored_policy == old_policy,
        "older input rules or numeric aliases changed"
    );

    let mut totals = [0usize; 6]; // Gem rows, complete/pending lists, all Boolean/Quantity members, queries.
    let mut gains = [0usize; 5];
    let mut summary = vec![];
    for case in 1..=5 {
        let name = format!("queries-original-{case:02}.json");
        assert_eq!(fs::read(output.join(&name)).unwrap(), prior_bytes[&name]);
        let previous = cwd.join(format!("gem-numeric-aliases-original-{case}"));
        let destination = cwd.join(format!("multieffect-support-gem-inputs-original-{case}"));
        let report = success(normalize(cwd, &output, case, &destination, true));
        assert_eq!(report["normalization_status"], "pending");
        assert_eq!(report["verification"]["calculation"], "not_run");
        let old = decode_draft(
            &fs::read(previous.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let new = decode_draft(
            &fs::read(destination.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let (old_sidecar, new_sidecar) = (
            json(previous.join("sidecar.json")),
            json(destination.join("sidecar.json")),
        );
        assert_eq!(new_sidecar["schema_version"], 12);
        let xml = fs::read(root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        )))
        .unwrap();
        let source = ImportedBuildInstance::from_decoded(
            decode_build(&xml).unwrap(),
            new.input().allocator.lineage(),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
        assert_eq!(
            old.input().gems.members.len(),
            new.input().gems.members.len()
        );
        let mut all_values = [0usize; 2];
        let mut complete = 0;
        let mut changed_indices = vec![];
        for (index, (old_gem, gem)) in old
            .input()
            .gems
            .members
            .iter()
            .zip(&new.input().gems.members)
            .enumerate()
        {
            complete += usize::from(matches!(
                gem.parameters.completion,
                DraftListCompletion::Complete
            ));
            for member in &gem.parameters.members {
                match member.value.to_resolved().unwrap() {
                    ParameterValue::Boolean(_) => all_values[0] += 1,
                    ParameterValue::Quantity(_) => all_values[1] += 1,
                    _ => panic!("unexpected intrinsic input type in pinned original"),
                }
            }
            let definition = gem.definition.to_resolved().unwrap();
            if !selected.contains(&definition) {
                continue;
            }
            assert!(old_gem.parameters.members.is_empty());
            pending(&old_gem.parameters.completion);
            pending(&gem.parameters.completion);
            let origins: Vec<_> = new_sidecar["origins"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|origin| {
                    origin["links"].as_array().unwrap().iter().any(|link| {
                        link["kind"] == "gem"
                            && link["value"] == serde_json::to_value(gem.id).unwrap()
                    })
                })
                .collect();
            assert_eq!(origins.len(), 1);
            let row = evidence
                .rows()
                .iter()
                .find(|row| {
                    serde_json::to_value(row.occurrence().id()).unwrap() == origins[0]["source"]
                })
                .unwrap();
            // The sole new original occurrence is derived through its source key;
            // it remains one physical Gem, with all potential Skill uses deferred.
            assert_eq!(case, 3);
            let witness = "Metadata/Items/Gems/SkillGemLivingLightningSupportTwo";
            assert_eq!(definition, expected_by_source[witness]);
            let identity = catalog.gem_by_key(witness).unwrap();
            assert_eq!(
                row.attribute("gemId").unwrap().decoded().unwrap(),
                identity.game_id
            );
            assert_eq!(
                row.attribute("variantId").unwrap().decoded().unwrap(),
                identity.variant_id
            );
            assert_eq!(row.attribute("corrupted").unwrap().raw(), "nil");
            assert_eq!(row.attribute("corruptLevel").unwrap().raw(), "nil");
            let expected_slots: Vec<_> = expected_parameters
                .iter()
                .filter(|entry| entry.id.declaration == SlotOwnerDefId::Gem(definition.clone()))
                .map(|entry| entry.id.clone())
                .collect();
            let expected = vec![
                (expected_slots[0].clone(), ParameterValue::Boolean(false)),
                (
                    expected_slots[1].clone(),
                    ParameterValue::Quantity(FiniteQuantity::new(0., delta_unit.clone()).unwrap()),
                ),
            ];
            let actual: Vec<_> = gem
                .parameters
                .members
                .iter()
                .map(|member| {
                    (
                        member.slot.to_resolved().unwrap(),
                        member.value.to_resolved().unwrap(),
                    )
                })
                .collect();
            assert_eq!(actual, expected, "original3 intrinsic source values");
            assert!(gem.to_resolved().is_none());
            gains[case - 1] += 1;
            changed_indices.push(index);
        }
        let mut old_wire = serde_json::to_value(old.input()).unwrap();
        let mut new_wire = serde_json::to_value(new.input()).unwrap();
        // Remove only the independently checked new intrinsic members. Every
        // existing parameter, issue, occurrence, relationship and choice remains.
        for index in changed_indices {
            new_wire["gems"]["members"][index]["parameters"]["members"] = serde_json::json!([]);
        }
        assert_eq!(
            canonical_instances(&mut old_wire, old.input().allocator.lineage(), &[]),
            canonical_instances(&mut new_wire, new.input().allocator.lineage(), &[])
        );
        assert!(
            old_wire == new_wire,
            "original {case}: unrelated draft changed"
        );
        for field in ["origins", "item_texts"] {
            let (mut old_field, mut new_field) =
                (old_sidecar[field].clone(), new_sidecar[field].clone());
            if field == "item_texts" {
                let (old_traces, new_traces) = (
                    old_field.as_array().unwrap(),
                    new_field.as_array_mut().unwrap(),
                );
                assert_eq!(old_traces.len(), new_traces.len());
                // Item rules themselves are unchanged above. Their content digests
                // change because their definition binding changes. Check each
                // attribution against its actual publication before rebinding it.
                for (trace_index, (previous, current)) in
                    old_traces.iter().zip(new_traces).enumerate()
                {
                    for (binding, manifest) in [("item_lines", "items"), ("policy", "item_source")]
                    {
                        assert!(prior_transition[manifest].as_str().is_some());
                        assert!(final_report["publication"][manifest].as_str().is_some());
                        assert_eq!(
                            previous["attribution"][binding], prior_transition[manifest],
                            "original {case}: item trace {trace_index} has stale prior {binding}"
                        );
                        assert_eq!(
                            current["attribution"][binding], final_report["publication"][manifest],
                            "original {case}: item trace {trace_index} has stale current {binding}"
                        );
                        current["attribution"][binding] = previous["attribution"][binding].clone();
                    }
                }
                for (sidecar, manifest) in [
                    ("item_policy", "items"),
                    ("item_source_policy", "item_source"),
                ] {
                    assert_eq!(old_sidecar[sidecar], prior_transition[manifest]);
                    assert_eq!(new_sidecar[sidecar], final_report["publication"][manifest]);
                }
            }
            assert_eq!(
                canonical_instances(&mut old_field, old.input().allocator.lineage(), &[]),
                canonical_instances(&mut new_field, new.input().allocator.lineage(), &[])
            );
            assert!(
                old_field == new_field,
                "original {case}: nonbinding {field} content changed"
            );
        }
        for field in [
            "source_sha256",
            "source_bytes",
            "source_schema",
            "revision",
            "mapping_source",
        ] {
            assert_eq!(
                old_sidecar[field], new_sidecar[field],
                "original {case}: {field}"
            );
        }
        assert!(
            !new.validate_limits(DraftLimits::default())
                .unwrap()
                .issues
                .is_empty()
        );
        let queries: usize = new
            .input()
            .query_presets
            .members
            .iter()
            .map(|preset| preset.queries.requests.members.len())
            .sum();
        assert_eq!(queries, 22);
        let rows = new.input().gems.members.len();
        for (total, count) in totals.iter_mut().zip([
            rows,
            complete,
            rows - complete,
            all_values[0],
            all_values[1],
            queries,
        ]) {
            *total += count;
        }
        summary.push(serde_json::json!({"original":case,"gems":rows,"complete_parameters":complete,"pending_parameters":rows-complete,"boolean_members":all_values[0],"quantity_members":all_values[1],"new_intrinsic_pairs":gains[case-1],"queries":queries,"calculation":"not_run"}));
    }
    assert_eq!(totals, [478, 12, 466, 338, 338, 110]);
    assert_eq!(gains, [0, 0, 1, 0, 0]);
    fs::write(
        cwd.join("multieffect-support-gem-inputs-summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();

    let compiled_bytes = bundle(&compiled);
    assert!(
        !compile(cwd, prior, &policy_path, &compiled)
            .status
            .success()
    );
    assert!(
        bundle(&compiled) == compiled_bytes,
        "no-clobber changed compiled output"
    );
    let mut stale = json(&policy_path);
    stale["catalog_digest"] = Value::String("0".repeat(64));
    let stale_path = cwd.join("multieffect-support-gem-inputs-stale-policy.json");
    fs::write(&stale_path, serde_json::to_vec(&stale).unwrap()).unwrap();
    let rejected = cwd.join("multieffect-support-gem-inputs-stale-output");
    assert!(!compile(cwd, prior, &stale_path, &rejected).status.success());
    assert!(!rejected.exists());
    assert!(bundle(prior) == prior_bytes, "predecessor bundle changed");
    assert!(
        fs::read(&policy_path).unwrap() == policy_bytes,
        "tracked policy changed"
    );
    output
}
