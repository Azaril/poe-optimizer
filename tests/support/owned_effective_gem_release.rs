//! Reproducible full endpoint authoring for one exact predecessor. This is a
//! preservation test, not an extension of append-only migration semantics.
use super::{compile_cli, key, read, record, success, write};
use poe_optimizer_core::{
    owned_build::*,
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{
    owned_effective_gem_recipe::*,
    owned_item_lines::OwnedItemLinePolicy,
    owned_mapping::{OwnedIdRegistry, OwnedMappingIndex},
    owned_normalize::GemQualityPolicy,
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_release::{
        OwnedReleaseInput, OwnedReleaseProvenance, OwnedReleaseReceipt, StagedOwnedRelease,
        assemble_owned_release,
    },
    owned_successor::NamedQuerySet,
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Authoring {
    schema_version: u32,
    before: OwnedContentDigest,
    release: OwnedDefinitionKey,
    quality: OwnedDefinitionKey,
    count_unit: OwnedDefinitionKey,
    percentage_unit: OwnedDefinitionKey,
    allocations: Vec<OwnedDefinitionKey>,
    bindings: Vec<Binding>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    gem: OwnedDefinitionKey,
    phase: EffectiveGemInputPhase,
    corruption: Option<OwnedDefinitionKey>,
    append_corruption: bool,
    maximum: u16,
    natural_maximum: u16,
    global_minion_level: bool,
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn inventory(root: &Path) -> BTreeMap<String, (u64, String)> {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(entry.file_type().unwrap().is_file());
            let bytes = fs::read(entry.path()).unwrap();
            (
                entry.file_name().into_string().unwrap(),
                (bytes.len() as u64, sha(&bytes)),
            )
        })
        .collect()
}
fn load(prior: &Path) -> StagedOwnedRelease {
    let receipt: OwnedReleaseReceipt = read(prior.join("release.json"));
    assert_eq!(receipt.schema_version, 1);
    assert!(receipt.evaluation.is_none());
    let input = OwnedReleaseInput {
        schema_version: receipt.schema_version,
        recipe: OwnedRecipeInput {
            schema_version: 1,
            registry: read(prior.join("registry.json")),
            schema: read(prior.join("schema.json")),
            rules: read(prior.join("rules.json")),
            routing: read(prior.join("routing.json")),
        },
        mapping: read(prior.join("mapping.json")),
        roles: read(prior.join("roles.json")),
        normalization: read(prior.join("normalization.json")),
        rewards: read(prior.join("rewards.json")),
        items: read(prior.join("items.json")),
        item_source: read(prior.join("item-source.json")),
        tree: receipt
            .tree
            .map(|_| read(prior.join("tree-normalization.json"))),
        evaluation: None,
        query_sets: receipt
            .artifacts
            .iter()
            .filter_map(|artifact| {
                artifact
                    .file
                    .strip_prefix("queries-")
                    .and_then(|v| v.strip_suffix(".json"))
                    .map(|name| NamedQuerySet {
                        name: key(name),
                        queries: read(prior.join(&artifact.file)),
                    })
            })
            .collect(),
        provenance: receipt.provenance.clone(),
    };
    let checked = assemble_owned_release(input, Default::default()).unwrap();
    assert_eq!(*checked.receipt(), receipt);
    assert_eq!(checked.artifacts().count(), inventory(prior).len());
    for (name, bytes) in checked.artifacts() {
        assert_eq!(
            bytes,
            fs::read(prior.join(name)).unwrap(),
            "predecessor artifact {name}"
        );
    }
    checked
}
fn gem_schema<'a>(input: &'a mut OwnedReleaseInput, gem: &GemDefId) -> &'a mut GemSchema {
    let row = input
        .recipe
        .schema
        .definitions
        .iter_mut()
        .find(|v| v.address() == gem.address())
        .unwrap();
    let DefinitionDescriptor::Gem(row) = row else {
        panic!("Gem descriptor");
    };
    let SchemaState::Known(schema) = &mut row.schema else {
        panic!("known Gem");
    };
    schema
}
fn stat(
    input: &mut OwnedReleaseInput,
    registry: &mut OwnedIdRegistry,
    value: ComputedValueType,
    scope: RuleEntityKind,
) -> StatDefId {
    let id = registry.allocate_definition().unwrap();
    input
        .recipe
        .schema
        .definitions
        .push(DefinitionDescriptor::Stat(record(
            id.clone(),
            StatSchema {
                value,
                targets: vec![scope],
            },
        )));
    id
}
fn dependency_bindings(input: &mut OwnedReleaseInput) {
    // Author exact bindings through public constructors. Full assembly below
    // independently checks them; no migration preservation guard is bypassed.
    let schema =
        OwnedDefinitionSchemaPackage::new(input.recipe.schema.clone(), Default::default()).unwrap();
    input.recipe.rules.definitions = schema.identity().clone();
    input.recipe.routing.definitions = schema.identity().clone();
    let recipe = assemble_owned_recipe(input.recipe.clone(), Default::default()).unwrap();
    input.mapping.definitions = recipe.schema().identity().clone();
    input.mapping.registry = recipe.registry().identity().unwrap();
    let mapping = OwnedMappingIndex::new(
        input.mapping.clone(),
        recipe.registry(),
        recipe.schema(),
        Default::default(),
    )
    .unwrap();
    input.roles.mapping = *mapping.identity();
    input.roles.definitions = recipe.schema().identity().clone();
    if let GemQualityPolicy::Attributes(v) = &mut input.normalization.gem_quality {
        v.definitions = recipe.schema().identity().clone();
    }
    if let Some(v) = &mut input.normalization.gem_inputs {
        v.definitions = recipe.schema().identity().clone();
    }
    input.rewards.mapping = *mapping.identity();
    input.rewards.definitions = recipe.schema().identity().clone();
    input.items.definitions = recipe.schema().identity().clone();
    let items =
        OwnedItemLinePolicy::new(input.items.clone(), recipe.schema(), Default::default()).unwrap();
    input.item_source.item_lines = *items.identity();
    input.tree = input.tree.take().map(|v| {
        OwnedTreeNormalizationPolicy::bind_new(
            v.content,
            recipe.registry(),
            recipe.schema(),
            &mapping,
            &input.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone()
    });
}
fn retire_raw_projections(program: &mut RuleProgram) {
    assert_eq!(program.id, key("primary-supply"));
    assert_eq!(program.context, RuleEntityKind::Actor);
    assert_eq!(program.effects.len(), 3);
    assert!(matches!(
        program.effects[0].effect,
        RuleEffectKind::ProjectSkillParameter { .. }
    ));
    assert!(matches!(
        program.effects[1].effect,
        RuleEffectKind::ProjectSkillParameter { .. }
    ));
    let activation = program.effects[2].clone();
    let RuleEffectKind::ActivateGrant { enabled, .. } = &activation.effect else {
        panic!("preserve activation");
    };
    let node = program
        .nodes
        .iter()
        .find(|v| &v.id == enabled)
        .unwrap()
        .clone();
    assert!(matches!(
        node.expression,
        RuleExpression::Literal {
            value: ParameterValue::Boolean(true)
        }
    ));
    program.reads.clear();
    program.nodes = vec![node];
    program.effects = vec![activation];
}

fn author(
    prior: &StagedOwnedRelease,
    policy: &Authoring,
    evidence: OwnedContentDigest,
) -> (
    OwnedReleaseInput,
    EffectiveGemRecipeInput,
    EffectiveGemRecipeOutput,
) {
    assert_eq!(policy.schema_version, 1);
    assert_eq!(policy.before, prior.receipt().input);
    let mut input = prior.input().clone();
    let ns = input.recipe.schema.namespace.clone();
    let count = UnitDefId::new(ns.clone(), policy.count_unit.clone());
    let percent = UnitDefId::new(ns.clone(), policy.percentage_unit.clone());
    let quality = QualityDefId::new(ns.clone(), policy.quality.clone());
    let mut registry =
        OwnedIdRegistry::new(input.recipe.registry.clone(), Default::default()).unwrap();
    let first = registry.input().entries.len();
    for binding in policy.bindings.iter().filter(|v| v.append_corruption) {
        let gem = GemDefId::new(ns.clone(), binding.gem.clone());
        let slot = registry
            .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Gem(gem.clone()))
            .unwrap();
        assert_eq!(Some(slot.slot.key()), binding.corruption.as_ref());
        gem_schema(&mut input, &gem)
            .declarations
            .parameters
            .members
            .push(slot.clone());
        input
            .recipe
            .schema
            .slots
            .push(SlotDescriptor::Parameter(record(
                slot,
                ParameterSlotSchema {
                    value: ValueSchema::Quantity(QuantityRange {
                        minimum: FiniteQuantity::new(-f64::MAX, count.clone()).unwrap(),
                        maximum: FiniteQuantity::new(f64::MAX, count.clone()).unwrap(),
                    }),
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::GemParameter],
                },
            )));
    }
    let external = stat(
        &mut input,
        &mut registry,
        ComputedValueType::Quantity {
            unit: count.clone(),
        },
        RuleEntityKind::Actor,
    );
    let active_level = stat(
        &mut input,
        &mut registry,
        ComputedValueType::Quantity {
            unit: count.clone(),
        },
        RuleEntityKind::Skill,
    );
    let active_quality = stat(
        &mut input,
        &mut registry,
        ComputedValueType::Quantity {
            unit: percent.clone(),
        },
        RuleEntityKind::Skill,
    );
    let support_level = stat(
        &mut input,
        &mut registry,
        ComputedValueType::Integer,
        RuleEntityKind::SupportOrigin,
    );
    let support_quality = stat(
        &mut input,
        &mut registry,
        ComputedValueType::Quantity { unit: percent },
        RuleEntityKind::SupportOrigin,
    );
    assert_eq!(
        registry.input().entries[first..]
            .iter()
            .map(|v| match &v.target {
                SchemaSubject::Definition(v) => v.key().clone(),
                SchemaSubject::Slot(v) => v.key().clone(),
            })
            .collect::<Vec<_>>(),
        policy.allocations
    );
    prior
        .assembled()
        .registry()
        .validate_successor(&registry)
        .unwrap();
    input.recipe.registry = registry.input().clone();
    input.recipe.schema.schema_version = 4;
    input.recipe.schema.release = policy.release.clone();
    input.recipe.schema.semantics_version = key("owned-mechanics-effective-gem-inputs-v1");
    input.recipe.rules.semantics_version = key("owned-mechanics-effective-gem-inputs-v1");
    input.recipe.rules.operations_version = key("owned-domain-operations-v13");
    let schema =
        OwnedDefinitionSchemaPackage::new(input.recipe.schema.clone(), Default::default()).unwrap();
    let recipe = EffectiveGemRecipeInput {
        schema_version: 1,
        version: key("reviewed-global-minion-preparation-v1"),
        definitions: schema.identity().clone(),
        bindings: policy
            .bindings
            .iter()
            .map(|v| {
                let gem = GemDefId::new(ns.clone(), v.gem.clone());
                let active = v.phase == EffectiveGemInputPhase::ActivePreSupport;
                EffectiveGemRecipeBinding {
                    gem: gem.clone(),
                    program: key("effective-input-preparation"),
                    role: if active {
                        EffectiveGemRecipeRole::ActivePreSupport {
                            corruption: DeclaredSlot {
                                declaration: SlotOwnerDefId::Gem(gem),
                                slot: ParameterSlotDefId::new(
                                    ns.clone(),
                                    v.corruption.clone().unwrap(),
                                ),
                            },
                        }
                    } else {
                        EffectiveGemRecipeRole::SupportPreparation {
                            levels: DenseGemLevelPolicy {
                                maximum: v.maximum,
                                natural_maximum: v.natural_maximum,
                            },
                        }
                    },
                    quality: quality.clone(),
                    quality_absence: QualityAbsencePolicy::RequireSelectedQuality,
                    level_unit: count.clone(),
                    external_level: if v.global_minion_level {
                        vec![external.clone()]
                    } else {
                        vec![]
                    },
                    external_quality: vec![],
                    level_output: if active {
                        active_level.clone()
                    } else {
                        support_level.clone()
                    },
                    quality_output: if active {
                        active_quality.clone()
                    } else {
                        support_quality.clone()
                    },
                }
            })
            .collect(),
    };
    let compiled = compile_effective_gem_recipe(&recipe, &schema, Default::default()).unwrap();
    for key_value in ["def.000000000000000a", "def.0000000000000011"] {
        let gem = GemDefId::new(ns.clone(), key(key_value));
        let owner = input
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|v| v.owner == SchemaSubject::Definition(gem.address()))
            .unwrap();
        retire_raw_projections(
            owner
                .programs
                .members
                .iter_mut()
                .find(|v| v.id == key("primary-supply"))
                .unwrap(),
        );
    }
    for row in &compiled.programs {
        let subject = SchemaSubject::Definition(row.gem.address());
        if let Some(owner) = input
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|v| v.owner == subject)
        {
            assert!(!owner.programs.is_complete());
            assert!(
                !owner
                    .programs
                    .members
                    .iter()
                    .any(|v| v.id == row.program.id)
            );
            owner.programs.members.push(row.program.clone());
        } else {
            input.recipe.rules.owners.push(DefinitionRules {
                owner: subject.clone(),
                programs: DeclaredSet {
                    members: vec![row.program.clone()],
                    closure: SchemaClosure::Partial {
                        gaps: vec![SchemaGap {
                            subject,
                            facet: SchemaFacet::GameRules,
                            code: key("other-support-effects-unconverted"),
                        }],
                    },
                },
            });
        }
    }
    dependency_bindings(&mut input);
    input.provenance.push(OwnedReleaseProvenance {
        kind: key("explicit-effective-input-authoring"),
        prior_input: policy.before,
        authoring_input: evidence,
    });
    (input, recipe, compiled)
}

pub(super) fn check(prior_path: &Path) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let policy_path = root.join("data/owned/poe2/3887ae68/effective-gem-inputs/authoring.json");
    let policy: Authoring = read(&policy_path);
    let policy_before = fs::read(&policy_path).unwrap();
    let evidence_path = policy_path.with_file_name("evidence.json");
    let source_evidence_before = fs::read(&evidence_path).unwrap();
    let evidence = digest_owned(
        "owned-effective-gem-authoring-v1",
        &(
            read::<serde_json::Value>(&policy_path),
            read::<serde_json::Value>(&evidence_path),
        ),
        2 * 1024 * 1024,
    )
    .unwrap();
    let prior_hashes = inventory(prior_path);
    let prior = load(prior_path);
    let (input, recipe, compiled) = author(&prior, &policy, evidence);
    let temp = tempfile::tempdir().unwrap();
    let output = std::env::var_os("POE_OPTIMIZER_TEST_EFFECTIVE_GEM_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("authored"));
    assert!(!output.exists(), "authoring output must be new");
    fs::create_dir_all(&output).unwrap();
    let endpoint = output.join("endpoint.json");
    write(&endpoint, &input);
    let staged = assemble_owned_release(input, Default::default()).unwrap();
    preservation(&prior, &staged, &policy, &compiled);
    let schema = output.join("schema.json");
    let recipe_path = output.join("recipe.json");
    write(&schema, staged.assembled().schema().input());
    write(&recipe_path, &recipe);
    let fragment_dir = output.join("fragments");
    success(compile_cli(&recipe_path, &schema, &fragment_dir));
    assert_eq!(
        read::<EffectiveGemRecipeOutput>(fragment_dir.join("programs.json")),
        compiled
    );
    for (source, destination) in [
        (&endpoint, output.join("package")),
        (&output.join("package"), output.join("rebuilt")),
    ] {
        let report = success(
            Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
                .arg("assemble-owned-release")
                .arg(source)
                .arg("--output")
                .arg(&destination)
                .output()
                .unwrap(),
        );
        assert_eq!(report, serde_json::to_value(staged.receipt()).unwrap());
    }
    assert_eq!(
        inventory(&output.join("package")),
        inventory(&output.join("rebuilt"))
    );
    for case in 1..=5 {
        let package = output.join("package");
        let destination = output.join(format!("original-{case:02}"));
        let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
        command.arg("normalize-owned").arg(root.join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        )));
        for (flag, file) in [
            ("--policy", "normalization.json"),
            ("--registry", "registry.json"),
            ("--definitions", "schema.json"),
            ("--mapping", "mapping.json"),
            ("--roles", "roles.json"),
            ("--rewards", "rewards.json"),
            ("--items", "items.json"),
            ("--item-source", "item-source.json"),
            ("--tree-policy", "tree-normalization.json"),
        ] {
            command.arg(flag).arg(package.join(file));
        }
        let report = success(
            command
                .arg("--queries")
                .arg(package.join(format!("queries-original-{case:02}.json")))
                .arg("--output")
                .arg(&destination)
                .output()
                .unwrap(),
        );
        assert_eq!(report["normalization_status"], "pending");
        assert_eq!(report["verification"]["calculation"], "not_run");
        write(
            output.join(format!("normalization-{case:02}.json")),
            &report,
        );
    }
    assert_eq!(prior_hashes, inventory(prior_path));
    assert_eq!(policy_before, fs::read(policy_path).unwrap());
    assert_eq!(source_evidence_before, fs::read(evidence_path).unwrap());
    for (name, bytes) in staged
        .artifacts()
        .filter(|(n, _)| n.starts_with("queries-"))
    {
        assert_eq!(bytes, fs::read(prior_path.join(name)).unwrap());
    }
    write(
        output.join("validation.json"),
        &serde_json::json!({"before":prior.receipt().input,"after":staged.receipt().input,"schema_version":4,"operations_version":"owned-domain-operations-v13","query_sets":5,"query_rows":110,"normalizations_pending":5,"coverage":"unchanged","complete_original_builds":0,"final_active_inputs":"not_produced","prior_unchanged":true,"rebuild_byte_identical":true,"new_allocations":policy.allocations}),
    );
}

fn preservation(
    prior: &StagedOwnedRelease,
    after: &StagedOwnedRelease,
    policy: &Authoring,
    compiled: &EffectiveGemRecipeOutput,
) {
    let before = prior.input();
    let mut restored = after.input().clone();
    assert_eq!(after.receipt().source, prior.receipt().source);
    assert_eq!(
        (after.receipt().query_sets, after.receipt().query_rows),
        (5, 110)
    );
    assert_eq!(restored.query_sets, before.query_sets);
    assert!(restored.evaluation.is_none());
    let new: BTreeSet<_> = policy.allocations.iter().collect();
    restored
        .recipe
        .schema
        .definitions
        .retain(|v| !new.contains(v.address().key()));
    restored
        .recipe
        .schema
        .slots
        .retain(|v| !new.contains(v.address().key()));
    for v in &mut restored.recipe.schema.definitions {
        if let DefinitionDescriptor::Gem(v) = v
            && let SchemaState::Known(s) = &mut v.schema
        {
            s.declarations
                .parameters
                .members
                .retain(|v| !new.contains(v.slot.key()));
        }
    }
    restored.recipe.schema.schema_version = before.recipe.schema.schema_version;
    restored.recipe.schema.release = before.recipe.schema.release.clone();
    restored.recipe.schema.semantics_version = before.recipe.schema.semantics_version.clone();
    restored.recipe.registry = before.recipe.registry.clone();
    for row in &compiled.programs {
        let subject = SchemaSubject::Definition(row.gem.address());
        let owner = restored
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|v| v.owner == subject)
            .unwrap();
        owner.programs.members.retain(|v| v.id != row.program.id);
        if !before
            .recipe
            .rules
            .owners
            .iter()
            .any(|v| v.owner == subject)
        {
            assert!(owner.programs.members.is_empty());
            restored.recipe.rules.owners.retain(|v| v.owner != subject);
        }
    }
    for old in &before.recipe.rules.owners {
        if !matches!(&old.owner,SchemaSubject::Definition(DefinitionAddress::Gem(g)) if matches!(g.key().as_str(),"def.000000000000000a"|"def.0000000000000011"))
        {
            continue;
        }
        let row = restored
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|v| v.owner == old.owner)
            .unwrap();
        assert_eq!(row.programs.closure, old.programs.closure);
        for p in &mut row.programs.members {
            if p.id == key("primary-supply") {
                *p = old
                    .programs
                    .members
                    .iter()
                    .find(|v| v.id == p.id)
                    .unwrap()
                    .clone();
            }
        }
    }
    restored.recipe.rules.operations_version = before.recipe.rules.operations_version.clone();
    restored.recipe.rules.semantics_version = before.recipe.rules.semantics_version.clone();
    restored.recipe.rules.definitions = before.recipe.rules.definitions.clone();
    restored.recipe.routing.definitions = before.recipe.routing.definitions.clone();
    restored.mapping.registry = before.mapping.registry;
    restored.mapping.definitions = before.mapping.definitions.clone();
    restored.roles.mapping = before.roles.mapping;
    restored.roles.definitions = before.roles.definitions.clone();
    if let (GemQualityPolicy::Attributes(a), GemQualityPolicy::Attributes(b)) = (
        &mut restored.normalization.gem_quality,
        &before.normalization.gem_quality,
    ) {
        a.definitions = b.definitions.clone();
    }
    if let (Some(a), Some(b)) = (
        &mut restored.normalization.gem_inputs,
        &before.normalization.gem_inputs,
    ) {
        a.definitions = b.definitions.clone();
    }
    restored.rewards.mapping = before.rewards.mapping;
    restored.rewards.definitions = before.rewards.definitions.clone();
    restored.items.definitions = before.items.definitions.clone();
    restored.item_source.item_lines = before.item_source.item_lines;
    assert_eq!(
        restored.tree.as_ref().map(|v| &v.content),
        before.tree.as_ref().map(|v| &v.content)
    );
    restored.tree = before.tree.clone();
    assert_eq!(restored.provenance.len(), before.provenance.len() + 1);
    restored.provenance.pop();
    assert_eq!(
        restored, *before,
        "only explicit additions, two raw projection retirements and exact dependency bindings may change"
    );
}
