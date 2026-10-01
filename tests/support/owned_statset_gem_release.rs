use super::{
    base, catalog, command, evidence, fixture, key, policy_path, read, root, success, write,
};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_content::digest_owned,
    owned_definitions::*,
    owned_draft::{DraftLimits, DraftListCompletion, decode_draft},
    owned_schema::*,
};
use poe_optimizer_import::{
    owned_gem_catalog::{
        CompiledOwnedGemCatalog, PhysicalGemSchemaPolicy, compile_owned_gem_catalog,
    },
    owned_gem_schema::GemSchemaMigrationInput,
    owned_mapping::{
        ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, SourceComponent,
    },
    owned_normalize::GemQualityPolicy,
    owned_recipe::OwnedRecipeInput,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

fn gem_id(prior: &StagedOwnedRelease, game: &str, variant: &str) -> GemDefId {
    let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
        game_id: SourceComponent::Text(game.into()),
        variant_id: SourceComponent::Text(variant.into()),
    });
    let Some(MappingOutcome::Mapped {
        target: SchemaSubject::Definition(DefinitionAddress::Gem(id)),
        basis: MappingBasis::Exact,
    }) = prior.mapping().lookup(&selector)
    else {
        panic!("exact physical mapping")
    };
    id.clone()
}
fn run(name: &str, prior: &Path, args: &[(&str, PathBuf)], output: &Path) -> Value {
    let mut c = command(name);
    c.arg(prior);
    for (flag, path) in args {
        c.arg(flag).arg(path);
    }
    success(c.arg("--output").arg(output).output().unwrap())
}
fn preservation(
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    compiled: &CompiledOwnedGemCatalog,
) {
    let b = prior.input();
    let mut r = next.input().clone();
    assert_eq!(next.receipt().source, prior.receipt().source);
    assert_eq!(
        (next.receipt().query_sets, next.receipt().query_rows),
        (5, 110)
    );
    assert_eq!(r.recipe, compiled.staged.successor);
    assert_eq!(
        &r.recipe.registry.entries[..b.recipe.registry.entries.len()],
        b.recipe.registry.entries.as_slice()
    );
    assert_eq!(
        r.recipe.registry.entries.len(),
        b.recipe.registry.entries.len() + 24
    );
    assert_eq!(r.recipe.registry.last_issued.get(), 0x30c9);
    assert_eq!(b.recipe.registry.last_issued.get(), 0x30b1);
    r.recipe.registry = b.recipe.registry.clone();
    for row in &compiled.migration.gems {
        let entry = r
            .recipe
            .schema
            .definitions
            .iter_mut()
            .find(|v| v.address() == row.id.address())
            .unwrap();
        assert_eq!(*entry, DefinitionDescriptor::Gem(row.clone()));
        let old = b
            .recipe
            .schema
            .definitions
            .iter()
            .find(|v| v.address() == row.id.address())
            .unwrap();
        assert!(
            matches!(old,DefinitionDescriptor::Gem(g) if matches!(g.schema,SchemaState::Unmapped{..}))
        );
        let SchemaState::Known(s) = &row.schema else {
            panic!("Known physical schema")
        };
        assert_eq!(s.roles, [AuthoredGemRole::SkillUse]);
        assert_eq!(s.skills.members.len(), 1);
        for closure in [
            &s.skills.closure,
            &s.quality.allowed_kinds.closure,
            &s.declarations.parameters.closure,
            &s.declarations.choices.closure,
            &s.declarations.grants.closure,
            &s.declarations.actors.closure,
            &s.declarations.skill_grants.closure,
            &s.declarations.outputs.closure,
            &s.declarations.sockets.closure,
        ] {
            assert!(matches!(closure, SchemaClosure::Partial { .. }));
        }
        assert!(
            s.declarations.choices.members.is_empty()
                && s.declarations.grants.members.is_empty()
                && s.declarations.actors.members.is_empty()
                && s.declarations.skill_grants.members.is_empty()
                && s.declarations.outputs.members.is_empty()
                && s.declarations.sockets.members.is_empty()
        );
        *entry = old.clone();
    }
    assert_eq!(
        r.recipe.schema.slots.len(),
        b.recipe.schema.slots.len() + 24
    );
    for row in &compiled.migration.parameters {
        let i = r
            .recipe
            .schema
            .slots
            .iter()
            .position(|v| v.address() == SlotAddress::Parameter(row.id.clone()))
            .unwrap();
        assert_eq!(
            r.recipe.schema.slots.remove(i),
            SlotDescriptor::Parameter(row.clone())
        );
    }
    assert_eq!(r.recipe.schema, b.recipe.schema);
    r.recipe.rules.definitions = b.recipe.rules.definitions.clone();
    r.recipe.routing.definitions = b.recipe.routing.definitions.clone();
    r.mapping.definitions = b.mapping.definitions.clone();
    r.mapping.registry = b.mapping.registry;
    r.roles.definitions = b.roles.definitions.clone();
    r.roles.mapping = b.roles.mapping;
    let (GemQualityPolicy::Attributes(x), GemQualityPolicy::Attributes(y)) = (
        &mut r.normalization.gem_quality,
        &b.normalization.gem_quality,
    ) else {
        panic!("quality policy")
    };
    x.definitions = y.definitions.clone();
    let rules = r.normalization.gem_inputs.as_mut().unwrap();
    assert_eq!(
        rules.gems.len(),
        b.normalization.gem_inputs.as_ref().unwrap().gems.len() + 12
    );
    for row in &compiled.inputs {
        let i = rules.gems.iter().position(|v| v.gem == row.gem).unwrap();
        assert_eq!(rules.gems.remove(i), *row);
    }
    rules.definitions = b
        .normalization
        .gem_inputs
        .as_ref()
        .unwrap()
        .definitions
        .clone();
    r.rewards.definitions = b.rewards.definitions.clone();
    r.rewards.mapping = b.rewards.mapping;
    r.items.definitions = b.items.definitions.clone();
    r.item_source.item_lines = b.item_source.item_lines;
    assert_eq!(
        r.tree.as_ref().map(|v| &v.content),
        b.tree.as_ref().map(|v| &v.content)
    );
    r.tree = b.tree.clone();
    assert_eq!(r.provenance.len(), b.provenance.len() + 1);
    r.provenance.pop();
    assert_eq!(
        r, *b,
        "only explicit physical inputs and dependent bindings changed"
    );
}
fn canonical_lineages(v: &mut Value, expected: &Value) {
    match v {
        Value::Object(o) => {
            if let Some(lineage) = o.get_mut("lineage") {
                assert_eq!(lineage, expected);
                *lineage = Value::String("canonical-test-lineage".into());
            }
            for child in o.values_mut() {
                canonical_lineages(child, expected);
            }
        }
        Value::Array(a) => {
            for child in a {
                canonical_lineages(child, expected);
            }
        }
        _ => {}
    }
}
fn original_inputs(
    prior: &Path,
    package: &Path,
    out: &Path,
    compiled: &CompiledOwnedGemCatalog,
) -> Vec<usize> {
    let mut counts = vec![];
    let mut seen = BTreeSet::new();
    for case in 1..=5 {
        let query = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(prior.join(&query)).unwrap(),
            fs::read(package.join(&query)).unwrap()
        );
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let a = out.join(format!("prior-original-{case:02}"));
        let z = out.join(format!("original-{case:02}"));
        fixture::normalize(prior, &xml, case, &a);
        let report = fixture::normalize(package, &xml, case, &z);
        write(out.join(format!("original-{case:02}-report.json")), &report);
        let old = decode_draft(
            &fs::read(a.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let new = decode_draft(
            &fs::read(z.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let mut aw = serde_json::to_value(old.input()).unwrap();
        let mut zw = serde_json::to_value(new.input()).unwrap();
        assert_eq!(
            old.input().gems.members.len(),
            new.input().gems.members.len()
        );
        let mut count = 0;
        for (i, g) in new.input().gems.members.iter().enumerate() {
            let Some(id) = g.definition.to_resolved() else {
                continue;
            };
            let Some(rule) = compiled.inputs.iter().find(|v| v.gem == id) else {
                continue;
            };
            seen.insert(id);
            count += 1;
            assert!(old.input().gems.members[i].parameters.members.is_empty());
            assert!(matches!(
                g.parameters.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert_eq!(g.parameters.members.len(), 2);
            for (parameter, recipe) in g.parameters.members.iter().zip(&rule.parameters) {
                assert_eq!(parameter.slot.to_resolved().unwrap(), recipe.slot);
            }
            assert_eq!(
                g.parameters.members[0].value.to_resolved(),
                Some(ParameterValue::Boolean(false))
            );
            let Some(ParameterValue::Quantity(delta)) = g.parameters.members[1].value.to_resolved()
            else {
                panic!("raw Count delta")
            };
            assert_eq!(delta.value(), 0.);
            assert_eq!(delta.unit().key().as_str(), "def.000000000000295a");
            zw["gems"]["members"][i]["parameters"]["members"] = serde_json::json!([]);
        }
        canonical_lineages(
            &mut aw,
            &serde_json::to_value(old.input().allocator.lineage()).unwrap(),
        );
        canonical_lineages(
            &mut zw,
            &serde_json::to_value(new.input().allocator.lineage()).unwrap(),
        );
        assert_eq!(
            aw, zw,
            "unchanged draft identities, choices, skills, queries and issues; case{case}"
        );
        counts.push(count);
    }
    assert_eq!(seen.len(), 12);
    assert_eq!(counts.iter().sum::<usize>(), 34);
    counts
}
fn mutate(xml: &str, variant: &str, flag: Option<&str>, delta: Option<&str>) -> String {
    let index = xml.find(&format!("variantId=\"{variant}\"")).unwrap();
    let start = xml[..index].rfind("<Gem ").unwrap();
    let end = index + xml[index..].find("/>").unwrap() + 2;
    let mut gem = xml[start..end].to_string();
    for (name, value) in [("corrupted", flag), ("corruptLevel", delta)] {
        let token = format!("{name}=\"");
        let s = gem.find(&token).unwrap();
        let e = s + token.len() + gem[s + token.len()..].find('"').unwrap() + 1;
        gem.replace_range(
            s..e,
            &value.map(|v| format!("{name}=\"{v}\"")).unwrap_or_default(),
        );
    }
    format!("{}{}{}", &xml[..start], gem, &xml[end..])
}
fn independent_probes(package: &Path, out: &Path, compiled: &CompiledOwnedGemCatalog) {
    let c = catalog();
    let source = c
        .gem_by_key("Metadata/Items/Gems/SkillGemBonestorm")
        .unwrap();
    let policy: PhysicalGemSchemaPolicy = read(policy_path());
    assert!(policy.source_gems.contains(&source.key));
    let prior = fixture::load(package);
    let id = gem_id(&prior, &source.game_id, &source.variant_id);
    let rule = compiled.inputs.iter().find(|v| v.gem == id).unwrap();
    let (case, xml) = (1..=5)
        .find_map(|case| {
            let s = fs::read_to_string(root().join(format!(
                "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
            )))
            .unwrap();
            s.contains(&format!("variantId=\"{}\"", source.variant_id))
                .then_some((case, s))
        })
        .unwrap();
    for (label, flag, delta, expected_flag, expected_delta) in [
        ("missing-flag", None, Some("0.25"), None, Some(0.25)),
        ("bad-flag", Some("TRUE"), Some("-3"), None, Some(-3.)),
        ("missing-delta", Some("true"), None, Some(true), None),
        ("bad-delta", Some("false"), Some("1x"), Some(false), None),
        ("nil", Some("nil"), Some("nil"), Some(false), Some(0.)),
    ] {
        let xml = mutate(&xml, &source.variant_id, flag, delta);
        let path = out.join(format!("probe-{label}.xml"));
        fs::write(&path, xml).unwrap();
        let dest = out.join(format!("probe-{label}"));
        fixture::normalize(package, &path, case, &dest);
        let draft = decode_draft(
            &fs::read(dest.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let g = draft
            .input()
            .gems
            .members
            .iter()
            .find(|g| g.definition.to_resolved() == Some(id.clone()))
            .unwrap();
        assert!(matches!(
            g.parameters.completion,
            DraftListCompletion::Pending { .. }
        ));
        for (i, expected) in [
            expected_flag.map(ParameterValue::Boolean),
            expected_delta.map(|v| {
                ParameterValue::Quantity(
                    FiniteQuantity::new(
                        v,
                        UnitDefId::new(id.namespace().clone(), key("def.000000000000295a")),
                    )
                    .unwrap(),
                )
            }),
        ]
        .into_iter()
        .enumerate()
        {
            let selected: Vec<_> = g
                .parameters
                .members
                .iter()
                .filter(|v| v.slot.to_resolved() == Some(rule.parameters[i].slot.clone()))
                .collect();
            match expected {
                Some(v) => {
                    assert_eq!(selected.len(), 1);
                    assert_eq!(selected[0].value.to_resolved(), Some(v));
                }
                None => assert!(selected.is_empty()),
            }
        }
    }
}
pub(super) fn check(prior_path: &Path) {
    let hashes = fixture::inventory(prior_path);
    let prior = fixture::load(prior_path);
    let proof = evidence();
    assert_eq!(
        serde_json::to_value(prior.receipt().input).unwrap(),
        proof["before"]
    );
    let policy: PhysicalGemSchemaPolicy = read(policy_path());
    let c = catalog();
    let compiled = compile_owned_gem_catalog(
        prior.assembled(),
        prior.mapping(),
        prior.roles(),
        &c,
        &policy,
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        (
            compiled.receipt.promoted_gems,
            compiled.receipt.allocated_parameters
        ),
        (12, 24)
    );
    let expected: BTreeSet<_> = policy
        .source_gems
        .iter()
        .map(|k| {
            let g = c.gem_by_key(k).unwrap();
            gem_id(&prior, &g.game_id, &g.variant_id)
        })
        .collect();
    assert_eq!(
        expected,
        compiled
            .migration
            .gems
            .iter()
            .map(|v| v.id.clone())
            .collect()
    );
    for row in &compiled.migration.gems {
        let source = policy
            .source_gems
            .iter()
            .map(|k| c.gem_by_key(k).unwrap())
            .find(|g| gem_id(&prior, &g.game_id, &g.variant_id) == row.id)
            .unwrap();
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Skill {
            effect_id: SourceComponent::Text(source.primary_effect_id.clone()),
        });
        let Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Skill(skill)),
            basis: MappingBasis::Exact,
        }) = prior.mapping().lookup(&selector)
        else {
            panic!("exact primary")
        };
        let SchemaState::Known(g) = &row.schema else {
            panic!("Known")
        };
        assert_eq!(g.skills.members.as_slice(), std::slice::from_ref(skill));
    }
    assert_eq!(prior.input().recipe.schema.definitions.iter().filter(|v|matches!(v,DefinitionDescriptor::Gem(g) if matches!(&g.schema,SchemaState::Known(s) if s.quality.allowed_kinds.is_complete()))).count(),604);
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_STATSET_GEM_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("publication"));
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    let prepared = out.join("compiled");
    let report = run(
        "compile-owned-gem-inputs",
        prior_path,
        &[
            ("--catalog", base().join("import/skill-identities.json")),
            ("--policy", policy_path()),
        ],
        &prepared,
    );
    assert_eq!(
        report["compilation"],
        serde_json::to_value(&compiled.receipt).unwrap()
    );
    assert_eq!(
        read::<GemSchemaMigrationInput>(prepared.join("migration.json")),
        compiled.migration
    );
    // Reconstruct the frozen earlier probe's stale schema-binding failure;
    // never read its ignored output or authorize it as a registry baseline.
    let mut stale = serde_json::to_value(&compiled.migration).unwrap();
    stale["before"]["content_sha256"] = proof["pre_boolean_schema"].clone();
    write(out.join("stale-migration.json"), &stale);
    let bad = command("migrate-owned-gem-schemas")
        .arg(prior_path)
        .arg("--migration")
        .arg(out.join("stale-migration.json"))
        .arg("--output")
        .arg(out.join("stale-rejected"))
        .output()
        .unwrap();
    assert!(!bad.status.success());
    assert!(!out.join("stale-rejected").exists());
    let schemas = out.join("schemas");
    let s = run(
        "migrate-owned-gem-schemas",
        prior_path,
        &[("--migration", prepared.join("migration.json"))],
        &schemas,
    );
    assert_eq!(s["publication"], report["schema_publication"]);
    let normalized = out.join("normalized");
    let n = run(
        "publish-owned-normalization",
        &schemas,
        &[("--normalization", prepared.join("normalization.json"))],
        &normalized,
    );
    assert_eq!(n["publication"], report["input_publication"]);
    let mut input = prior.input().clone();
    input.recipe = OwnedRecipeInput {
        schema_version: 1,
        registry: read(normalized.join("registry.json")),
        schema: read(normalized.join("schema.json")),
        rules: read(normalized.join("rules.json")),
        routing: read(normalized.join("routing.json")),
    };
    input.mapping = read(normalized.join("mapping.json"));
    input.roles = read(normalized.join("roles.json"));
    input.normalization = read(normalized.join("normalization.json"));
    input.rewards = read(normalized.join("rewards.json"));
    input.items = read(normalized.join("items.json"));
    input.item_source = read(normalized.join("item-source.json"));
    input.tree = Some(read(normalized.join("tree-normalization.json")));
    input.provenance.push(OwnedReleaseProvenance {
        kind: key("explicit-stat-set-primary-physical-inputs"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-stat-set-primary-physical-inputs-v1",
            &(&proof, &policy, &compiled.migration),
            16 * 1024 * 1024,
        )
        .unwrap(),
    });
    let staged = assemble_owned_release(input.clone(), Default::default()).unwrap();
    preservation(&prior, &staged, &compiled);
    let endpoint = out.join("endpoint.json");
    write(&endpoint, &input);
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    for (src, dst) in [(&endpoint, &package), (&package, &rebuilt)] {
        assert_eq!(
            run("assemble-owned-release", src, &[], dst),
            serde_json::to_value(staged.receipt()).unwrap()
        );
    }
    assert_eq!(fixture::inventory(&package), fixture::inventory(&rebuilt));
    let counts = original_inputs(prior_path, &package, &out, &compiled);
    independent_probes(&package, &out, &compiled);
    assert_eq!(hashes, fixture::inventory(prior_path));
    write(
        out.join("validation.json"),
        &serde_json::json!({"before":prior.receipt().input,"after":staged.receipt().input,"definitions":staged.receipt().definitions,"registry":staged.receipt().registry,"promoted_gems":12,"allocated_parameters":24,"first_parameter":"def.00000000000030b2","last_parameter":"def.00000000000030c9","existing_complete_quality_owners":604,"new_partial_quality_owners":12,"remaining_unmapped_gems":350,"original_occurrences":counts,"query_sets":5,"query_rows":110,"originals_pending":5,"independence_probes":5,"prior_provenance_entries":prior.input().provenance.len(),"final_provenance_entries":staged.input().provenance.len(),"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
