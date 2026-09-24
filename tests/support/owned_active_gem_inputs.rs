//! Reviewed active-Gem input membership; no action activation or gameplay legality.
use super::{
    scalar_families::recipe,
    skill_scopes::canonical_instances,
    support::{bundle, data, json, normalize, root, success},
};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_content::digest_owned,
    owned_definitions::*,
    owned_draft::{DraftLimits, DraftListCompletion, decode_draft},
    owned_schema::*,
};
use poe_optimizer_data::skill_identities::{GemIdentity, SkillIdentityCatalog};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_gem_catalog::{GemEffectMembershipPolicy, PhysicalGemSchemaPolicy},
    owned_gem_schema::GemSchemaMigrationInput,
    owned_mapping::{
        ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, OwnedMappingIndex,
        SourceComponent,
    },
    owned_normalize::{
        GemQualityPolicy, ImportQueryTemplate, NormalizationLimits, NormalizationPolicy,
    },
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
    process::Command,
};

fn command(cwd: &Path, name: &str, prior: &Path, args: &[(&str, PathBuf)], output: &Path) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command.current_dir(cwd).arg(name).arg(prior);
    for (flag, path) in args {
        command.arg(flag).arg(path);
    }
    success(command.arg("--output").arg(output).output().unwrap())
}
fn publication(path: &Path) -> Value {
    json(path.join(if path.join("release.json").exists() {
        "release.json"
    } else {
        "transition.json"
    }))
}
fn endpoint(publication: &Value) -> &Value {
    publication.get("after").unwrap_or(publication)
}
fn gem_id(mapping: &OwnedMappingIndex, source: &GemIdentity) -> GemDefId {
    let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
        game_id: SourceComponent::Text(source.game_id.clone()),
        variant_id: SourceComponent::Text(source.variant_id.clone()),
    });
    let Some(MappingOutcome::Mapped {
        target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)),
        basis: MappingBasis::Exact,
    }) = mapping.lookup(&selector)
    else {
        panic!("exact physical identity required")
    };
    gem.clone()
}
fn skill_id(mapping: &OwnedMappingIndex, effect: &str) -> SkillDefId {
    let selector = ExternalSelector::Definition(ExternalOwnerSelector::Skill {
        effect_id: SourceComponent::Text(effect.into()),
    });
    let Some(MappingOutcome::Mapped {
        target: SchemaSubject::Definition(DefinitionAddress::Skill(skill)),
        basis: MappingBasis::Exact,
    }) = mapping.lookup(&selector)
    else {
        panic!("exact potential effect identity required")
    };
    skill.clone()
}
struct Expected {
    source: String,
    parameters: Vec<
        DefinitionEntry<
            poe_optimizer_core::owned_build::DeclaredSlot<ParameterSlotDefId>,
            ParameterSlotSchema,
        >,
    >,
    family: usize,
}

fn family(
    cwd: &Path,
    prior: &Path,
    label: &str,
    count: usize,
    family_index: usize,
    catalog: &SkillIdentityCatalog,
    expected: &mut BTreeMap<GemDefId, Expected>,
) -> PathBuf {
    let authored: PhysicalGemSchemaPolicy =
        serde_json::from_value(json(data().join(format!("active-gem-inputs/{label}.json"))))
            .unwrap();
    assert_eq!(authored.source_gems.len(), count);
    assert_eq!(
        (authored.level.minimum.get(), authored.level.maximum.get()),
        (1, 40)
    );
    assert_eq!(
        authored.effect_membership,
        if family_index == 0 {
            GemEffectMembershipPolicy::SinglePrimary
        } else {
            GemEffectMembershipPolicy::ResolvedPotentialSkillsV1
        }
    );
    assert_eq!(authored.parameters.len(), 2);
    assert!(matches!(
        authored.parameters[0].schema.value,
        ValueSchema::Boolean
    ));
    let ValueSchema::Quantity(range) = &authored.parameters[1].schema.value else {
        panic!("quantity input")
    };
    let before = recipe(prior);
    let base = assemble_owned_recipe(before.clone(), Default::default()).unwrap();
    assert!(
        matches!(base.schema().definition(range.minimum.unit()), SchemaLookup::Known(unit) if unit.dimension == UnitDimension::Count)
    );
    let mapping = OwnedMappingIndex::new(
        serde_json::from_value(json(prior.join("mapping.json"))).unwrap(),
        base.registry(),
        base.schema(),
        Default::default(),
    )
    .unwrap();
    let roles = OwnedSkillRoleIndex::new(
        serde_json::from_value(json(prior.join("roles.json"))).unwrap(),
        &mapping,
        base.schema(),
        Default::default(),
    )
    .unwrap();
    let mut selected = BTreeMap::new();
    for key in &authored.source_gems {
        let source = catalog.gem_by_key(key).unwrap();
        let primary = catalog.skill_by_id(&source.primary_effect_id).unwrap();
        assert_ne!(primary.support, Some(true));
        assert_ne!(primary.from_tree, Some(true));
        assert!(source.declared_additional_stat_sets.is_empty());
        assert!(
            !catalog
                .data()
                .missing_references
                .iter()
                .any(|reference| reference.gem_key == *key)
        );
        let potential: BTreeSet<_> = std::iter::once(&source.primary_effect_id)
            .chain(source.additional_effects.iter())
            .collect();
        assert_eq!(potential, source.effect_list.iter().collect());
        assert_eq!(potential.len(), family_index + 1);
        assert_eq!(source.effect_list.len(), family_index + 1);
        let gem = gem_id(&mapping, source);
        let role = roles.role(&gem).unwrap();
        assert_eq!(role.materialization, OwnedGemMaterialization::Physical);
        assert_eq!(role.role, OwnedGemRole::Known(AuthoredGemRole::SkillUse));
        assert_eq!(
            role.primary,
            OwnedPrimarySkill::Known(skill_id(&mapping, &source.primary_effect_id))
        );
        assert!(matches!(
            base.schema().definition(&gem),
            SchemaLookup::Unmapped(_)
        ));
        let skills: Vec<_> = potential
            .iter()
            .map(|effect| skill_id(&mapping, effect))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        assert!(selected.insert(gem, (key.clone(), skills)).is_none());
    }
    assert_eq!(
        selected
            .values()
            .map(|(_, skills)| skills.len())
            .sum::<usize>(),
        count * (family_index + 1)
    );
    let mut registry = base.registry().clone();
    let mut parameters = vec![];
    for (gem, (source, _)) in &selected {
        let rows: Vec<_> = authored
            .parameters
            .iter()
            .map(|parameter| DefinitionEntry {
                id: registry
                    .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Gem(gem.clone()))
                    .unwrap(),
                schema: SchemaState::Known(parameter.schema.clone()),
            })
            .collect();
        parameters.extend(rows.clone());
        assert!(
            expected
                .insert(
                    gem.clone(),
                    Expected {
                        source: source.clone(),
                        parameters: rows,
                        family: family_index
                    }
                )
                .is_none()
        );
    }
    let compiled = cwd.join(format!("active-gem-{label}-compiled"));
    let report = command(
        cwd,
        "compile-owned-gem-inputs",
        prior,
        &[
            ("--catalog", data().join("import/skill-identities.json")),
            (
                "--policy",
                data().join(format!("active-gem-inputs/{label}.json")),
            ),
        ],
        &compiled,
    );
    assert_eq!(report["compilation"]["promoted_gems"], count);
    assert_eq!(report["compilation"]["allocated_parameters"], count * 2);
    let migration: GemSchemaMigrationInput =
        serde_json::from_value(json(compiled.join("migration.json"))).unwrap();
    assert_eq!(migration.parameters, parameters);
    assert_eq!(
        migration
            .gems
            .iter()
            .map(|gem| &gem.id)
            .collect::<BTreeSet<_>>(),
        selected.keys().collect()
    );
    let schema = cwd.join(format!("active-gem-{label}-schemas"));
    let schema_report = command(
        cwd,
        "migrate-owned-gem-schemas",
        prior,
        &[("--migration", compiled.join("migration.json"))],
        &schema,
    );
    assert_eq!(schema_report["publication"], report["schema_publication"]);
    let previous_publication = publication(prior);
    let previous = endpoint(&previous_publication);
    assert_eq!(
        schema_report["publication"]["before"]
            .as_object()
            .unwrap()
            .len(),
        8
    );
    for field in [
        "registry",
        "definitions",
        "mapping",
        "roles",
        "normalization",
        "rewards",
        "rules",
        "routing",
    ] {
        assert_eq!(
            schema_report["publication"]["before"][field],
            previous[field]
        );
    }
    let output = cwd.join(format!("active-gem-{label}-successor"));
    let normalized = command(
        cwd,
        "publish-owned-normalization",
        &schema,
        &[("--normalization", compiled.join("normalization.json"))],
        &output,
    );
    assert_eq!(normalized["publication"], report["input_publication"]);
    for transition in [&schema_report["publication"], &normalized["publication"]] {
        assert_eq!(transition["query_rows"], 110);
        assert_eq!(transition["whole_build_parity"], "not_established");
        assert_eq!(transition["calculation"], "not_run");
    }
    let after = recipe(&output);
    assert!(
        &after.registry == registry.input(),
        "unexpected registry change"
    );
    let mut restored = after.schema.clone();
    for row in &mut restored.definitions {
        let DefinitionDescriptor::Gem(gem) = row else {
            continue;
        };
        let Some((_, skills)) = selected.get(&gem.id) else {
            continue;
        };
        let SchemaState::Known(schema) = &gem.schema else {
            panic!("promoted input schema")
        };
        assert_eq!(schema.level, authored.level);
        assert_eq!(schema.roles, [AuthoredGemRole::SkillUse]);
        assert_eq!(&schema.skills.members, skills);
        assert_eq!(schema.quality.presence, authored.quality_presence);
        assert_eq!(schema.quality.allowed_kinds.members, authored.quality_kinds);
        assert_eq!(
            schema.declarations.parameters.members,
            expected[&gem.id]
                .parameters
                .iter()
                .map(|row| row.id.clone())
                .collect::<Vec<_>>()
        );
        for closure in [
            &schema.skills.closure,
            &schema.quality.allowed_kinds.closure,
            &schema.declarations.parameters.closure,
            &schema.declarations.choices.closure,
            &schema.declarations.grants.closure,
            &schema.declarations.actors.closure,
            &schema.declarations.skill_grants.closure,
            &schema.declarations.outputs.closure,
            &schema.declarations.sockets.closure,
        ] {
            assert!(matches!(closure, SchemaClosure::Partial { .. }));
        }
        assert!(
            schema.declarations.choices.members.is_empty()
                && schema.declarations.grants.members.is_empty()
                && schema.declarations.actors.members.is_empty()
                && schema.declarations.skill_grants.members.is_empty()
                && schema.declarations.outputs.members.is_empty()
                && schema.declarations.sockets.members.is_empty()
        );
        *row = before
            .schema
            .definitions
            .iter()
            .find(|old| old.address() == gem.id.address())
            .unwrap()
            .clone();
    }
    let new_slots: BTreeSet<_> = parameters
        .iter()
        .map(|row| SlotAddress::Parameter(row.id.clone()))
        .collect();
    for parameter in &parameters {
        assert!(
            after
                .schema
                .slots
                .contains(&SlotDescriptor::Parameter(parameter.clone()))
        );
    }
    assert_eq!(
        after.schema.slots.len(),
        before.schema.slots.len() + parameters.len()
    );
    restored
        .slots
        .retain(|row| !new_slots.contains(&row.address()));
    assert!(restored == before.schema, "unrelated schema change");
    let mut rules = after.rules.clone();
    rules.definitions = before.rules.definitions.clone();
    assert!(
        rules == before.rules,
        "intrinsic inputs acquired unreviewed numerical consumers"
    );
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
            assert_ne!(new[*field], old[*field]);
            new[*field] = old[*field].clone();
        }
        assert!(new == old, "unrelated {name} changed");
    }
    let old: NormalizationPolicy =
        serde_json::from_value(json(prior.join("normalization.json"))).unwrap();
    let mut new: NormalizationPolicy =
        serde_json::from_value(json(output.join("normalization.json"))).unwrap();
    let inputs = new.gem_inputs.as_mut().unwrap();
    assert_eq!(inputs.definitions, after.rules.definitions);
    assert_eq!(
        inputs.gems.len(),
        old.gem_inputs.as_ref().unwrap().gems.len() + count
    );
    for gem in selected.keys() {
        let rule = inputs.gems.iter().find(|rule| rule.gem == *gem).unwrap();
        assert_eq!(rule.guards, authored.guards);
        assert_eq!(rule.parameters.len(), 2);
        for ((actual, policy), slot) in rule
            .parameters
            .iter()
            .zip(&authored.parameters)
            .zip(&expected[gem].parameters)
        {
            assert_eq!(actual.value, policy.value);
            assert_eq!(actual.slot, slot.id);
        }
    }
    inputs.gems.retain(|rule| !selected.contains_key(&rule.gem));
    inputs.definitions = old.gem_inputs.as_ref().unwrap().definitions.clone();
    let (GemQualityPolicy::Attributes(a), GemQualityPolicy::Attributes(b)) =
        (&mut new.gem_quality, &old.gem_quality)
    else {
        panic!("quality binding")
    };
    a.definitions = b.definitions.clone();
    assert_eq!(new, old);
    output
}

pub(super) fn check_active_gem_inputs(cwd: &Path, prior: &Path) -> PathBuf {
    let prior_bytes = bundle(prior);
    let prior_publication = publication(prior);
    let catalog = SkillIdentityCatalog::new(
        serde_json::from_value(json(data().join("import/skill-identities.json"))).unwrap(),
    )
    .unwrap();
    let mut expected = BTreeMap::new();
    let single = family(cwd, prior, "singleton", 26, 0, &catalog, &mut expected);
    let output = family(cwd, &single, "multieffect", 10, 1, &catalog, &mut expected);
    assert_eq!(expected.len(), 36);
    assert_eq!(
        json(output.join("registry.json"))["last_issued"]
            .as_u64()
            .unwrap()
            - json(prior.join("registry.json"))["last_issued"]
                .as_u64()
                .unwrap(),
        72
    );
    let final_publication = publication(&output);
    let base = assemble_owned_recipe(recipe(&output), Default::default()).unwrap();
    let mapping = OwnedMappingIndex::new(
        serde_json::from_value(json(output.join("mapping.json"))).unwrap(),
        base.registry(),
        base.schema(),
        Default::default(),
    )
    .unwrap();
    let roles = OwnedSkillRoleIndex::new(
        serde_json::from_value(json(output.join("roles.json"))).unwrap(),
        &mapping,
        base.schema(),
        Default::default(),
    )
    .unwrap();
    let mut excluded = BTreeMap::new();
    for source in &catalog.data().gems {
        let unresolved = catalog
            .data()
            .missing_references
            .iter()
            .any(|row| row.gem_key == source.key);
        if source.declared_additional_stat_sets.is_empty() && !unresolved {
            continue;
        }
        let gem = gem_id(&mapping, source);
        let Some(role) = roles.role(&gem) else {
            continue;
        };
        if role.materialization != OwnedGemMaterialization::Physical
            || role.role != OwnedGemRole::Known(AuthoredGemRole::SkillUse)
        {
            continue;
        }
        assert!(!expected.contains_key(&gem));
        // The two previously reviewed seed descriptors are preserved by the
        // whole-schema comparisons above; this census concerns unconverted Gems.
        if matches!(base.schema().definition(&gem), SchemaLookup::Unmapped(_)) {
            excluded.insert(gem, unresolved);
        }
    }
    let old_policy: NormalizationPolicy =
        serde_json::from_value(json(prior.join("normalization.json"))).unwrap();
    let new_policy: NormalizationPolicy =
        serde_json::from_value(json(output.join("normalization.json"))).unwrap();
    let mut totals = [0usize; 5]; // physical Gems, Boolean, Quantity, modifiers, queries
    let mut gains = [[0usize; 5]; 2];
    let mut skipped = [0usize; 5];
    let mut skipped_definitions = BTreeSet::new();
    for case in 1..=5 {
        let query = format!("queries-original-{case:02}.json");
        assert_eq!(fs::read(output.join(&query)).unwrap(), prior_bytes[&query]);
        let queries: Vec<ImportQueryTemplate> =
            serde_json::from_slice(&prior_bytes[&query]).unwrap();
        let pair_digest = |policy: &NormalizationPolicy| {
            serde_json::to_value(
                digest_owned(
                    "owned-normalization-policy-v3",
                    &(policy, &queries),
                    NormalizationLimits::default().max_policy_bytes,
                )
                .unwrap(),
            )
            .unwrap()
        };
        let previous = cwd.join(format!("open-gem-inputs-original-{case}"));
        let destination = cwd.join(format!("active-gem-inputs-original-{case}"));
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
        let mut a = json(previous.join("sidecar.json"));
        let mut b = json(destination.join("sidecar.json"));
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
        let mut old_wire = serde_json::to_value(old.input()).unwrap();
        let mut new_wire = serde_json::to_value(new.input()).unwrap();
        assert_eq!(
            old.input().gems.members.len(),
            new.input().gems.members.len()
        );
        for (index, (old_gem, gem)) in old
            .input()
            .gems
            .members
            .iter()
            .zip(&new.input().gems.members)
            .enumerate()
        {
            totals[0] += 1;
            assert!(matches!(
                old_gem.parameters.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert!(matches!(
                gem.parameters.completion,
                DraftListCompletion::Pending { .. }
            ));
            for parameter in &gem.parameters.members {
                match parameter.value.to_resolved().unwrap() {
                    ParameterValue::Boolean(_) => totals[1] += 1,
                    ParameterValue::Quantity(_) => totals[2] += 1,
                    _ => panic!("intrinsic scalar kind"),
                }
            }
            let definition = gem.definition.to_resolved().unwrap();
            if excluded.contains_key(&definition) {
                skipped[case - 1] += 1;
                skipped_definitions.insert(definition.clone());
                assert!(gem.parameters.members.is_empty());
                assert!(matches!(
                    base.schema().definition(&definition),
                    SchemaLookup::Unmapped(_)
                ));
            }
            let Some(expected) = expected.get(&definition) else {
                continue;
            };
            assert!(old_gem.parameters.members.is_empty());
            assert_eq!(gem.parameters.members.len(), 2);
            let origins: Vec<_> = b["origins"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| {
                    row["links"].as_array().unwrap().iter().any(|link| {
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
            let identity = catalog.gem_by_key(&expected.source).unwrap();
            assert_eq!(
                row.attribute("gemId").unwrap().decoded().unwrap(),
                identity.game_id
            );
            assert_eq!(
                row.attribute("variantId").unwrap().decoded().unwrap(),
                identity.variant_id
            );
            assert!(matches!(
                row.attribute("corrupted").unwrap().raw(),
                "false" | "nil"
            ));
            assert!(matches!(
                row.attribute("corruptLevel").unwrap().raw(),
                "0" | "nil"
            ));
            for (member, slot) in gem.parameters.members.iter().zip(&expected.parameters) {
                assert_eq!(member.slot.to_resolved().unwrap(), slot.id);
            }
            assert_eq!(
                gem.parameters.members[0].value.to_resolved(),
                Some(ParameterValue::Boolean(false))
            );
            let SchemaState::Known(ParameterSlotSchema {
                value: ValueSchema::Quantity(range),
                ..
            }) = &expected.parameters[1].schema
            else {
                panic!("count input")
            };
            assert_eq!(
                gem.parameters.members[1].value.to_resolved(),
                Some(ParameterValue::Quantity(
                    FiniteQuantity::new(0., range.minimum.unit().clone()).unwrap()
                ))
            );
            gains[expected.family][case - 1] += 1;
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
        let old_pair = pair_digest(&old_policy);
        let new_pair = pair_digest(&new_policy);
        for (field, old_binding, new_binding) in [
            ("policy", &old_pair, &new_pair),
            (
                "mapping",
                &endpoint(&prior_publication)["mapping"],
                &endpoint(&final_publication)["mapping"],
            ),
            (
                "registry",
                &endpoint(&prior_publication)["registry"],
                &endpoint(&final_publication)["registry"],
            ),
            (
                "definitions",
                &endpoint(&prior_publication)["definitions"],
                &endpoint(&final_publication)["definitions"],
            ),
            (
                "skill_roles",
                &endpoint(&prior_publication)["roles"],
                &endpoint(&final_publication)["roles"],
            ),
            (
                "reward_policy",
                &endpoint(&prior_publication)["rewards"],
                &endpoint(&final_publication)["rewards"],
            ),
            (
                "item_policy",
                &prior_publication["items"],
                &final_publication["items"],
            ),
            (
                "item_source_policy",
                &prior_publication["item_source"],
                &final_publication["item_source"],
            ),
            (
                "tree_policy",
                &prior_publication["tree"],
                &final_publication["tree"],
            ),
        ] {
            assert_eq!(&a[field], old_binding);
            assert_eq!(&b[field], new_binding);
            b[field] = a[field].clone();
        }
        let old_traces = a["item_texts"].as_array().unwrap();
        let new_traces = b["item_texts"].as_array_mut().unwrap();
        assert_eq!(old_traces.len(), new_traces.len());
        for (old, new) in old_traces.iter().zip(new_traces) {
            for (field, binding) in [("item_lines", "items"), ("policy", "item_source")] {
                assert_eq!(old["attribution"][field], prior_publication[binding]);
                assert_eq!(new["attribution"][field], final_publication[binding]);
                new["attribution"][field] = old["attribution"][field].clone();
            }
        }
        for (sidecar, draft) in [(&mut a, &old), (&mut b, &new)] {
            let recorded = sidecar.as_object_mut().unwrap().remove("draft").unwrap();
            assert_eq!(
                recorded,
                serde_json::to_value(
                    draft
                        .digest(DraftLimits::default().input.max_wire_bytes)
                        .unwrap()
                )
                .unwrap()
            );
        }
        assert_eq!(
            canonical_instances(&mut a, old.input().allocator.lineage(), &[]),
            canonical_instances(&mut b, new.input().allocator.lineage(), &[])
        );
        assert!(a == b, "original {case}: unrelated sidecar changed");
        totals[3] += new
            .input()
            .items
            .members
            .iter()
            .map(|item| item.modifiers.members.len())
            .sum::<usize>();
        let queries = new
            .input()
            .query_presets
            .members
            .iter()
            .map(|preset| preset.queries.requests.members.len())
            .sum::<usize>();
        assert_eq!(queries, 22);
        totals[4] += queries;
    }
    assert_eq!(gains, [[3, 22, 4, 8, 18], [0, 12, 2, 4, 4]]);
    assert_eq!(totals, [478, 415, 415, 96, 110]);
    assert_eq!(skipped, [5, 23, 3, 1, 19]);
    assert_eq!(skipped_definitions.len(), 17);
    assert_eq!(
        skipped_definitions
            .iter()
            .filter(|gem| excluded[*gem])
            .count(),
        5
    );
    assert!(
        bundle(prior) == prior_bytes,
        "corrected release predecessor changed"
    );
    output
}
