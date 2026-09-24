//! Source-free catalogue compilation preserves unresolved support relationships.
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
    owned_gem_catalog::PhysicalGemSchemaPolicy,
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

pub(super) fn check_support_gem_inputs(cwd: &Path, prior: &Path) -> PathBuf {
    let policy_path = data().join("support-gem-inputs/policy.json");
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
    let mut expected_owners = BTreeMap::new();
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
        assert!(
            expected_owners
                .insert(gem.clone(), primary.clone())
                .is_none()
        );
    }
    assert_eq!(expected_owners.len(), authored.source_gems.len());
    assert_eq!(expected_owners.len(), 514);
    assert_eq!(authored.parameters.len(), 2);
    assert!(matches!(
        authored.parameters[0].schema.value,
        ValueSchema::Boolean
    ));
    let ValueSchema::Quantity(delta_range) = &authored.parameters[1].schema.value else {
        panic!("authored corruption delta must be a quantity")
    };
    let delta_unit = delta_range.minimum.unit().clone();
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
    let compiled = cwd.join("support-gem-inputs-compiled");
    let report = success(compile(cwd, prior, &policy_path, &compiled));
    assert_eq!(json(compiled.join("receipt.json")), report);
    assert_eq!(report["compilation"]["promoted_gems"], 514);
    assert_eq!(report["compilation"]["allocated_parameters"], 1_028);
    assert_eq!(report["calculation"], "not_run");
    assert_eq!(report["whole_build_parity"], "not_established");
    let migration: GemSchemaMigrationInput =
        serde_json::from_value(json(compiled.join("migration.json"))).unwrap();
    assert_eq!(migration.gems.len(), 514);
    assert_eq!(migration.parameters.len(), 1_028);
    let selected: BTreeSet<_> = migration.gems.iter().map(|gem| gem.id.clone()).collect();
    assert_eq!(
        selected,
        expected_owners.keys().cloned().collect::<BTreeSet<_>>()
    );
    assert_eq!(migration.parameters, expected_parameters);

    let schema_path = cwd.join("support-gem-inputs-schemas");
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
    let output = cwd.join("support-gem-inputs-successor");
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
    assert_eq!(&after.registry, expected_registry.input());
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
        assert_eq!(schema.skills.members, [expected_owners[&gem.id].clone()]);
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
    assert_eq!(promoted, 514);
    let new_slots: BTreeSet<_> = migration
        .parameters
        .iter()
        .map(|entry| SlotAddress::Parameter(entry.id.clone()))
        .collect();
    assert_eq!(after.schema.slots.len(), before.schema.slots.len() + 1_028);
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
    assert_eq!(restored, before.schema);
    let mut rules = after.rules.clone();
    rules.definitions = before.rules.definitions.clone();
    assert_eq!(rules, before.rules);
    let mut routing = after.routing.clone();
    routing.definitions = before.routing.definitions.clone();
    assert_eq!(routing, before.routing);

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
        assert_eq!(new, old, "unrelated {name} content changed");
    }
    let old_policy: NormalizationPolicy =
        serde_json::from_value(json(prior.join("normalization.json"))).unwrap();
    let current_policy: NormalizationPolicy =
        serde_json::from_value(json(output.join("normalization.json"))).unwrap();
    let current_inputs = current_policy.gem_inputs.as_ref().unwrap();
    assert_eq!(current_inputs.definitions, after.rules.definitions);
    assert_eq!(
        current_inputs.gems.len(),
        old_policy.gem_inputs.as_ref().unwrap().gems.len() + 514
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
    assert_eq!(restored_policy, old_policy);

    let mut totals = [0usize; 6]; // Gem rows, complete/pending lists, Boolean/Quantity members, queries.
    let mut summary = vec![];
    let mut nil_numeric = 0;
    for case in 1..=5 {
        let name = format!("queries-original-{case:02}.json");
        assert_eq!(fs::read(output.join(&name)).unwrap(), prior_bytes[&name]);
        let previous = cwd.join(format!("guarded-gem-inputs-original-{case}"));
        let destination = cwd.join(format!("support-gem-inputs-original-{case}"));
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
        let mut added = [0usize; 2];
        let mut complete = 0;
        for (old_gem, gem) in old
            .input()
            .gems
            .members
            .iter()
            .zip(&new.input().gems.members)
        {
            assert!(old_gem.parameters.members.is_empty());
            complete += usize::from(matches!(
                gem.parameters.completion,
                DraftListCompletion::Complete
            ));
            let definition = gem.definition.to_resolved().unwrap();
            if !selected.contains(&definition) {
                assert!(gem.parameters.members.is_empty());
                continue;
            }
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
            let flag = row.attribute("corrupted").unwrap().decoded().unwrap();
            let delta = row.attribute("corruptLevel").unwrap().decoded().unwrap();
            let rule = current_inputs
                .gems
                .iter()
                .find(|rule| rule.gem == definition)
                .unwrap();
            let mut expected = vec![];
            match flag {
                "true" => expected.push((
                    rule.parameters[0].slot.clone(),
                    ParameterValue::Boolean(true),
                )),
                "false" | "nil" => expected.push((
                    rule.parameters[0].slot.clone(),
                    ParameterValue::Boolean(false),
                )),
                _ => panic!("unreviewed original corruption flag {flag}"),
            }
            added[0] += 1;
            match delta {
                "0" => {
                    expected.push((
                        rule.parameters[1].slot.clone(),
                        ParameterValue::Quantity(
                            FiniteQuantity::new(0., delta_unit.clone()).unwrap(),
                        ),
                    ));
                    added[1] += 1;
                }
                "nil" => nil_numeric += 1,
                _ => panic!("unreviewed original corruption delta {delta}"),
            }
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
            assert_eq!(
                actual,
                expected,
                "original {case}, source {}",
                row.occurrence().id().ordinal()
            );
            assert!(gem.to_resolved().is_none());
        }
        let mut old_wire = serde_json::to_value(old.input()).unwrap();
        let mut new_wire = serde_json::to_value(new.input()).unwrap();
        // Only independently checked intrinsic scalar members may differ. All
        // closure issues, quality, level, identities and every other input stay.
        for gem in new_wire["gems"]["members"].as_array_mut().unwrap() {
            gem["parameters"]["members"] = serde_json::json!([]);
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
        for (total, count) in
            totals
                .iter_mut()
                .zip([rows, complete, rows - complete, added[0], added[1], queries])
        {
            *total += count;
        }
        summary.push(serde_json::json!({"original":case,"gems":rows,"complete_parameters":complete,"pending_parameters":rows-complete,"boolean_members":added[0],"quantity_members":added[1],"queries":queries,"calculation":"not_run"}));
    }
    assert_eq!(
        (totals[0], totals[1], totals[2], totals[5]),
        (478, 12, 466, 110)
    );
    assert_eq!((totals[3], totals[4], nil_numeric), (337, 178, 159));
    assert!(
        nil_numeric > 0,
        "original literal nil must remain unresolved, with Boolean retained"
    );
    assert_eq!(totals[3], totals[4] + nil_numeric);
    fs::write(
        cwd.join("support-gem-inputs-summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();

    let compiled_bytes = bundle(&compiled);
    assert!(
        !compile(cwd, prior, &policy_path, &compiled)
            .status
            .success()
    );
    assert_eq!(bundle(&compiled), compiled_bytes);
    let mut stale = json(&policy_path);
    stale["catalog_digest"] = Value::String("0".repeat(64));
    let stale_path = cwd.join("support-gem-inputs-stale-policy.json");
    fs::write(&stale_path, serde_json::to_vec(&stale).unwrap()).unwrap();
    let rejected = cwd.join("support-gem-inputs-stale-output");
    assert!(!compile(cwd, prior, &stale_path, &rejected).status.success());
    assert!(!rejected.exists());
    assert_eq!(bundle(prior), prior_bytes);
    assert_eq!(fs::read(&policy_path).unwrap(), policy_bytes);
    output
}
