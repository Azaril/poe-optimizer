//! Reviewed lexical aliases add intrinsic values without closing support semantics.
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
    owned_mapping::{
        ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, OwnedMappingIndex,
        SourceComponent,
    },
    owned_normalize::NormalizationPolicy,
    owned_recipe::assemble_owned_recipe,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_value_policy::{NumericTokenAlias, ValueLane},
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn publish(cwd: &Path, prior: &Path, policy: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("publish-owned-normalization")
        .arg(prior)
        .arg("--normalization")
        .arg(policy)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
fn save(path: &Path, policy: &NormalizationPolicy) {
    fs::write(path, serde_json::to_vec(policy).unwrap()).unwrap();
}
fn rejected(output: Output, destination: &Path, label: &str) {
    assert!(
        !output.status.success(),
        "invalid alias policy succeeded: {label}"
    );
    assert!(
        !output.stderr.is_empty(),
        "rejection lacks diagnostics: {label}"
    );
    assert!(
        !destination.exists(),
        "rejected alias policy created output: {label}"
    );
}

pub(super) fn check_gem_numeric_aliases(cwd: &Path, prior: &Path) -> PathBuf {
    let prior_bytes = bundle(prior);
    let prior_transition = json(prior.join("transition.json"));
    let old_policy: NormalizationPolicy =
        serde_json::from_value(json(prior.join("normalization.json"))).unwrap();
    let authored: PhysicalGemSchemaPolicy =
        serde_json::from_value(json(data().join("support-gem-inputs/policy.json"))).unwrap();
    let catalog = SkillIdentityCatalog::new(
        serde_json::from_value(json(data().join("import/skill-identities.json"))).unwrap(),
    )
    .unwrap();
    let base = assemble_owned_recipe(recipe(prior), Default::default()).unwrap();
    let mapping = OwnedMappingIndex::new(
        serde_json::from_value(json(prior.join("mapping.json"))).unwrap(),
        base.registry(),
        base.schema(),
        Default::default(),
    )
    .unwrap();
    // Derive the 514 subjects from authored source keys and the checked prior
    // mapping, independently of the normalization rules about to be modified.
    let mut selected = BTreeMap::new();
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
            panic!("reviewed source Gem lacks exact prior mapping")
        };
        let SchemaLookup::Known(schema) = base.schema().definition(gem) else {
            panic!("reviewed Gem schema")
        };
        assert!(!schema.declarations.parameters.is_complete());
        assert_eq!(
            schema.declarations.parameters.members.len(),
            authored.parameters.len()
        );
        assert!(
            selected
                .insert(gem.clone(), schema.declarations.parameters.members.clone())
                .is_none()
        );
    }
    assert_eq!(selected.len(), 514);
    let delta_indexes: Vec<_> = authored
        .parameters
        .iter()
        .enumerate()
        .filter(|(_, parameter)| {
            matches!(parameter.schema.value, ValueSchema::Quantity(_))
                && parameter
                    .value
                    .tiers
                    .iter()
                    .flat_map(|tier| &tier.selectors)
                    .any(|selector| {
                        selector.lane == ValueLane::Attribute && selector.name == "corruptLevel"
                    })
        })
        .map(|(index, _)| index)
        .collect();
    assert_eq!(delta_indexes.len(), 1);
    let delta_index = delta_indexes[0];
    assert_eq!(authored.parameters.len(), 2);
    assert_eq!(delta_index, 1);
    let ValueSchema::Quantity(range) = &authored.parameters[delta_index].schema.value else {
        unreachable!()
    };
    let unit = range.minimum.unit().clone();
    let zero = ParameterValue::Quantity(FiniteQuantity::new(0., unit).unwrap());
    let mut policy = old_policy.clone();
    policy.version = OwnedDefinitionKey::new("physical-support-numeric-aliases-v1").unwrap();
    let mut changed = 0;
    for rule in &mut policy.gem_inputs.as_mut().unwrap().gems {
        let Some(slots) = selected.get(&rule.gem) else {
            continue;
        };
        assert_eq!(rule.parameters.len(), authored.parameters.len());
        for ((input, source), slot) in rule.parameters.iter().zip(&authored.parameters).zip(slots) {
            assert_eq!(&input.slot, slot);
            assert_eq!(input.value, source.value);
            let SchemaLookup::Known(schema) = base.schema().slot(slot) else {
                panic!("reviewed slot schema")
            };
            assert_eq!(schema, &source.schema);
        }
        let parameter = &mut rule.parameters[delta_index];
        assert!(parameter.value.numeric_aliases.is_empty());
        parameter.value.numeric_aliases = vec![NumericTokenAlias {
            token: "nil".into(),
            replacement: "0".into(),
        }];
        changed += 1;
    }
    assert_eq!(changed, 514);
    let policy_path = data().join("support-gem-inputs/numeric-alias-normalization.json");
    let authored_bytes = fs::read(&policy_path).unwrap();
    let published_policy: NormalizationPolicy = serde_json::from_slice(&authored_bytes).unwrap();
    assert!(
        published_policy == policy,
        "tracked alias policy differs from independently derived policy"
    );
    let output = cwd.join("gem-numeric-aliases-successor");
    let report = success(publish(cwd, prior, &policy_path, &output));
    let transition = &report["publication"];
    assert_eq!(transition["before"], prior_transition["after"]);
    assert_eq!(transition["query_rows"], 110);
    assert_eq!(transition["calculation"], "not_run");
    assert_eq!(transition["whole_build_parity"], "not_established");
    assert_eq!(transition["items"], prior_transition["items"]);
    assert_eq!(transition["item_source"], prior_transition["item_source"]);
    let mut restored_bindings = transition["after"].clone();
    assert_ne!(
        restored_bindings["normalization"],
        transition["before"]["normalization"]
    );
    restored_bindings["normalization"] = transition["before"]["normalization"].clone();
    assert_eq!(restored_bindings, transition["before"]);
    let published = bundle(&output);
    for (name, bytes) in &prior_bytes {
        if ![
            "transition.json",
            "normalization.json",
            "tree-normalization.json",
            "catalog-append.json",
        ]
        .contains(&name.as_str())
        {
            assert!(
                published[name] == *bytes,
                "alias-only publication changed {name}"
            );
        }
    }
    let prior_tree = json(prior.join("tree-normalization.json"));
    let mut current_tree = json(output.join("tree-normalization.json"));
    assert_ne!(current_tree["normalization"], prior_tree["normalization"]);
    current_tree["normalization"] = prior_tree["normalization"].clone();
    assert!(
        current_tree == prior_tree,
        "alias-only publication changed nonbinding tree content"
    );
    assert!(
        serde_json::from_value::<NormalizationPolicy>(json(output.join("normalization.json")))
            .unwrap()
            == policy,
        "published normalization differs from the reviewed alias policy"
    );
    let mut restored = policy.clone();
    restored.version = old_policy.version.clone();
    for rule in &mut restored.gem_inputs.as_mut().unwrap().gems {
        if selected.contains_key(&rule.gem) {
            rule.parameters[delta_index].value.numeric_aliases.clear();
        }
    }
    assert!(
        restored == old_policy,
        "alias policy changed unrelated predecessor settings"
    );

    let mut totals = [0usize; 7]; // Rows, complete/pending, Boolean/Quantity, added quantities, queries.
    let mut summary = vec![];
    for case in 1..=5 {
        let previous = cwd.join(format!("support-gem-inputs-original-{case}"));
        let destination = cwd.join(format!("gem-numeric-aliases-original-{case}"));
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
        let mut new_wire = serde_json::to_value(new.input()).unwrap();
        let mut old_wire = serde_json::to_value(old.input()).unwrap();
        let mut counts = [0usize; 3]; // Existing/new Boolean, final Quantity, newly resolved nil Quantity.
        let mut complete = 0;
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
            complete += usize::from(matches!(
                gem.parameters.completion,
                DraftListCompletion::Complete
            ));
            let definition = gem.definition.to_resolved().unwrap();
            let Some(slots) = selected.get(&definition) else {
                continue;
            };
            assert_eq!(slots.len(), 2);
            for completion in [&old_gem.parameters.completion, &gem.parameters.completion] {
                let DraftListCompletion::Pending { code, .. } = completion else {
                    panic!("alias closed unreviewed membership")
                };
                assert_eq!(code.as_str(), "gem-parameters-not-converted");
            }
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
            let boolean = match flag {
                "false" | "nil" => false,
                "true" => true,
                _ => panic!("unreviewed saved flag {flag}"),
            };
            let expected = vec![
                (slots[0].clone(), ParameterValue::Boolean(boolean)),
                (slots[delta_index].clone(), zero.clone()),
            ];
            let values = |gem: &poe_optimizer_core::owned_draft::GemDraft| {
                gem.parameters
                    .members
                    .iter()
                    .map(|member| {
                        (
                            member.slot.to_resolved().unwrap(),
                            member.value.to_resolved().unwrap(),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                values(gem),
                expected,
                "original {case}: independently joined source values"
            );
            counts[0] += 1;
            counts[1] += 1;
            match delta {
                "0" => assert_eq!(values(old_gem), expected),
                "nil" => {
                    assert_eq!(row.attribute("corruptLevel").unwrap().raw(), "nil");
                    assert_eq!(values(old_gem), expected[..1]);
                    counts[2] += 1;
                    // Remove only the just-verified new value before comparing all
                    // inputs. Old members and pending issue identity remain exact.
                    new_wire["gems"]["members"][index]["parameters"]["members"] =
                        serde_json::to_value(&old_gem.parameters.members).unwrap();
                }
                _ => panic!("unreviewed saved numeric delta {delta}"),
            }
            assert!(gem.to_resolved().is_none());
        }
        assert_eq!(
            canonical_instances(&mut old_wire, old.input().allocator.lineage(), &[]),
            canonical_instances(&mut new_wire, new.input().allocator.lineage(), &[])
        );
        assert!(
            old_wire == new_wire,
            "original {case}: alias changed unrelated draft input"
        );
        for field in ["origins", "item_texts"] {
            let (mut old_trace, mut new_trace) =
                (old_sidecar[field].clone(), new_sidecar[field].clone());
            assert_eq!(
                canonical_instances(&mut old_trace, old.input().allocator.lineage(), &[]),
                canonical_instances(&mut new_trace, new.input().allocator.lineage(), &[])
            );
            assert!(
                old_trace == new_trace,
                "original {case}: alias changed {field}"
            );
        }
        for field in [
            "source_sha256",
            "source_bytes",
            "source_schema",
            "revision",
            "schema_version",
            "definitions",
            "registry",
            "mapping",
            "mapping_source",
            "skill_roles",
            "reward_policy",
            "item_policy",
            "item_source_policy",
        ] {
            assert_eq!(
                old_sidecar[field], new_sidecar[field],
                "original {case}: {field}"
            );
        }
        let rows = new.input().gems.members.len();
        let queries: usize = new
            .input()
            .query_presets
            .members
            .iter()
            .map(|preset| preset.queries.requests.members.len())
            .sum();
        assert_eq!(queries, 22);
        assert!(
            !new.validate_limits(DraftLimits::default())
                .unwrap()
                .issues
                .is_empty()
        );
        assert_eq!(counts[2], [0, 0, 47, 46, 66][case - 1]);
        for (total, count) in totals.iter_mut().zip([
            rows,
            complete,
            rows - complete,
            counts[0],
            counts[1],
            counts[2],
            queries,
        ]) {
            *total += count;
        }
        summary.push(serde_json::json!({"original":case,"gems":rows,"complete_parameters":complete,"pending_parameters":rows-complete,"boolean_members":counts[0],"quantity_members":counts[1],"added_nil_alias_quantities":counts[2],"queries":queries,"calculation":"not_run"}));
    }
    assert_eq!(totals, [478, 12, 466, 337, 337, 159, 110]);
    fs::write(
        cwd.join("gem-numeric-aliases-summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
    assert!(!publish(cwd, prior, &policy_path, &output).status.success());
    assert!(
        bundle(&output) == published,
        "no-clobber changed the published alias bundle"
    );
    for (label, token, replacement) in [
        ("malformed", "nil", "invalid"),
        ("non-finite", "nil", "1e999"),
        ("numeric-shadow", "0", "1"),
    ] {
        let mut invalid = policy.clone();
        let inputs = invalid.gem_inputs.as_mut().unwrap();
        let rule = inputs
            .gems
            .iter_mut()
            .find(|rule| selected.contains_key(&rule.gem))
            .unwrap();
        rule.parameters[delta_index].value.numeric_aliases = vec![NumericTokenAlias {
            token: token.into(),
            replacement: replacement.into(),
        }];
        let path = cwd.join(format!("gem-numeric-aliases-{label}.json"));
        save(&path, &invalid);
        let destination = cwd.join(format!("gem-numeric-aliases-{label}-rejected"));
        rejected(
            publish(cwd, prior, &path, &destination),
            &destination,
            label,
        );
    }
    let mut stale = policy.clone();
    stale
        .gem_inputs
        .as_mut()
        .unwrap()
        .definitions
        .content_sha256 = "0".repeat(64);
    let path = cwd.join("gem-numeric-aliases-stale.json");
    save(&path, &stale);
    let destination = cwd.join("gem-numeric-aliases-stale-rejected");
    rejected(
        publish(cwd, prior, &path, &destination),
        &destination,
        "stale binding",
    );
    assert!(
        bundle(prior) == prior_bytes,
        "alias publication modified predecessor"
    );
    assert!(
        fs::read(&policy_path).unwrap() == authored_bytes,
        "alias publication modified tracked policy bytes"
    );
    output
}
