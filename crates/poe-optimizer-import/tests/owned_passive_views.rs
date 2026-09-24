//! Full-list conversion laws with real physical nodes and independent coefficients.
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact};
use poe_optimizer_import::{
    owned_mapping::*, owned_passive_views::*, owned_recipe::*, owned_tree_catalog::*,
};
use serde::de::DeserializeOwned;
use std::{fs, path::PathBuf};
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn integer(n: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(n).unwrap())
}
fn load<T: DeserializeOwned>(path: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/owned/poe2/3887ae68")
                .join(path),
        )
        .unwrap(),
    )
    .unwrap()
}
fn rebind(recipe: &mut OwnedRecipeInput) {
    let schema =
        OwnedDefinitionSchemaPackage::new(recipe.schema.clone(), Default::default()).unwrap();
    recipe.rules.definitions = schema.identity().clone();
    recipe.routing.definitions = schema.identity().clone();
}
struct Fixture {
    recipe: OwnedRecipeInput,
    mapping: MappingPackageInput,
    catalog: TreeCatalogInput,
    policy: ViewRecipePolicy,
    stats: Vec<StatDefId>,
}
impl Fixture {
    fn new() -> Self {
        let mut recipe: OwnedRecipeInput = load("current/recipe.json");
        let mapping = load("current/mapping.json");
        let catalog: TreeCatalogInput = load("tree/tree-catalog.json");
        let mut registry =
            OwnedIdRegistry::new(recipe.registry.clone(), Default::default()).unwrap();
        let mut stats = vec![];
        for _ in 0..3 {
            let stat: StatDefId = registry.allocate_definition().unwrap();
            recipe
                .schema
                .definitions
                .push(DefinitionDescriptor::Stat(DefinitionEntry {
                    id: stat.clone(),
                    schema: SchemaState::Known(StatSchema {
                        value: ComputedValueType::Integer,
                        targets: vec![RuleEntityKind::Actor],
                    }),
                }));
            stats.push(stat);
        }
        recipe.registry = registry.input().clone();
        rebind(&mut recipe);
        let mut nodes = vec![];
        for id in ["1755", "22314", "41965", "60230", "4739", "18845", "51184"] {
            let n = catalog.nodes.iter().find(|n| n.key == id).unwrap();
            nodes.push(ViewNodePolicy {
                node: id.into(),
                pool: TreePoolKind::Ordinary,
                default: ViewEffectPolicy {
                    expected_stats: n.stats.clone(),
                    contributions: vec![ViewContribution {
                        stat: stats[0].clone(),
                        contribution: ContributionKind::Add,
                        value: integer(7),
                    }],
                },
                views: vec![ViewBranchPolicy {
                    selector: n.views[0].selector.clone(),
                    when: ViewSelector::Class { key: "1".into() },
                    effects: ViewEffectPolicy {
                        expected_stats: n.views[0].stats.clone(),
                        contributions: vec![ViewContribution {
                            stat: stats[1].clone(),
                            contribution: ContributionKind::Add,
                            value: integer(13),
                        }],
                    },
                }],
            });
        }
        Self {
            recipe,
            mapping,
            catalog,
            policy: ViewRecipePolicy {
                schema_version: 1,
                version: key("independent-view-policy"),
                nodes,
                receiver_rules: vec![],
                receivers: vec![],
            },
            stats,
        }
    }
    fn checked(&self) -> (StagedOwnedRecipe, OwnedMappingIndex) {
        let base = assemble_owned_recipe(self.recipe.clone(), Default::default()).unwrap();
        let mut input = self.mapping.clone();
        input.definitions = base.schema().identity().clone();
        input.registry = base.registry().identity().unwrap();
        let mapping =
            OwnedMappingIndex::new(input, base.registry(), base.schema(), Default::default())
                .unwrap();
        (base, mapping)
    }
    fn compile(&self) -> Result<StagedViewRecipe, ViewRecipeError> {
        let (base, mapping) = self.checked();
        compile_owned_passive_views(
            &base,
            &mapping,
            &self.catalog,
            &self.policy,
            Default::default(),
        )
    }
    fn raw_mut(&mut self) -> &mut TreeNodeInput {
        let id = &self.policy.nodes[0].node;
        self.catalog
            .nodes
            .iter_mut()
            .find(|n| &n.key == id)
            .unwrap()
    }
    fn with_ascendancy(&mut self) {
        self.raw_mut().views.push(TreeStatViewInput {
            selector: "Abyssal Lich".into(),
            stats: vec!["explicit synthetic ascendancy effect".into()],
        });
        self.policy.nodes[0].views.push(ViewBranchPolicy {
            selector: "Abyssal Lich".into(),
            when: ViewSelector::Ascendancy {
                class: "1".into(),
                key: "Witch3b".into(),
            },
            effects: ViewEffectPolicy {
                expected_stats: vec!["explicit synthetic ascendancy effect".into()],
                contributions: vec![ViewContribution {
                    stat: self.stats[2].clone(),
                    contribution: ContributionKind::Add,
                    value: integer(17),
                }],
            },
        });
    }
    fn with_receiver(&mut self) {
        let stat = self.stats[2].clone();
        self.policy.receiver_rules.push(DefinitionRules {
            owner: SchemaSubject::Definition(stat.address()),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key("sum-modifier-property"),
                context: RuleEntityKind::Actor,
                reads: vec![RuleRead {
                    id: key("contributions"),
                    value_type: ComputedValueType::Integer,
                    source: RuleReadSource::Contributions {
                        entity: RuleEntity::Current,
                        stat: stat.clone(),
                        contribution: ContributionKind::Add,
                        reduction: ContributionReduction::Sum,
                        empty: integer(0),
                    },
                }],
                nodes: vec![RuleNode {
                    id: key("sum"),
                    expression: RuleExpression::Read {
                        input: key("contributions"),
                    },
                }],
                effects: vec![RuleEffect {
                    id: key("final"),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: stat.clone(),
                        value: key("sum"),
                    },
                }],
            }]),
        });
        self.policy.receivers.push(ActorStatReceiver {
            id: key("owner-property"),
            stat,
            program: key("sum-modifier-property"),
            targets: vec![ActorReceiverTarget::Player],
        });
    }
}
fn bad(f: &Fixture, s: &str) {
    let error = f.compile().unwrap_err();
    assert!(error.to_string().contains(s), "{error}");
}
fn selected<'a>(out: &'a OwnedRecipeInput, address: &DefinitionAddress) -> &'a DefinitionRules {
    out.rules
        .owners
        .iter()
        .find(|r| r.owner == SchemaSubject::Definition(address.clone()))
        .unwrap()
}

#[test]
fn seven_real_nodes_preserve_physical_identity_topology_and_unrelated_rules() {
    let f = Fixture::new();
    let (base, _) = f.checked();
    let out = f.compile().unwrap();
    assert_eq!(out.receipt.converted_nodes, 7);
    assert_eq!(out.refined.len(), 7);
    assert_eq!(out.receipt.changed_program_owners, 7);
    assert_eq!(out.successor.registry, f.recipe.registry);
    assert_eq!(out.successor.schema.slots, base.schema().input().slots);
    assert_eq!(
        out.successor.rules.operations_version,
        key(OWNED_RULE_OPERATIONS_V7)
    );
    for old in &base.schema().input().definitions {
        let next = out
            .successor
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == old.address())
            .unwrap();
        if out.refined.contains(&old.address()) {
            let DefinitionDescriptor::PassiveNode(old) = old else {
                unreachable!()
            };
            let DefinitionDescriptor::PassiveNode(next) = next else {
                unreachable!()
            };
            let (SchemaState::Known(old), SchemaState::Known(next)) = (&old.schema, &next.schema)
            else {
                unreachable!()
            };
            assert_eq!(old.pools, next.pools);
            assert_eq!(old.adjacent, next.adjacent);
            for closure in [
                &next.declarations.parameters.closure,
                &next.declarations.choices.closure,
                &next.declarations.grants.closure,
                &next.declarations.actors.closure,
                &next.declarations.skill_grants.closure,
                &next.declarations.outputs.closure,
                &next.declarations.sockets.closure,
            ] {
                assert_eq!(closure, &SchemaClosure::Complete);
            }
        } else {
            assert_eq!(old, next);
        }
    }
    for old in &base.rules().input().owners {
        if !out
            .refined
            .iter()
            .any(|a| old.owner == SchemaSubject::Definition(a.clone()))
        {
            assert_eq!(
                out.successor
                    .rules
                    .owners
                    .iter()
                    .find(|r| r.owner == old.owner),
                Some(old)
            );
        }
    }
    assert_eq!(
        out.successor.rules.receivers,
        base.rules().input().receivers
    );
}
#[test]
fn class_match_replaces_entire_default_list_and_nonmatching_selection_uses_default() {
    let f = Fixture::new();
    let out = f.compile().unwrap();
    let base = assemble_owned_recipe(out.successor.clone(), Default::default()).unwrap();
    let compiled =
        CompiledRulePackage::compile(base.rules().input(), base.schema(), Default::default())
            .unwrap();
    let mut scratch = compiled.new_scratch();
    for address in &out.refined {
        let row = selected(&out.successor, address);
        let p = &row.programs.members[0];
        for (is_class, expected_stat, value) in [(false, &f.stats[0], 7), (true, &f.stats[1], 13)] {
            let facts = vec![RuleFact {
                read: p.reads[0].id.clone(),
                value: ParameterValue::Boolean(is_class),
            }];
            let result = compiled
                .evaluate(&row.owner, &p.id, &facts, base.schema(), &mut scratch)
                .unwrap();
            let applied: Vec<_> = result
                .effects
                .iter()
                .filter(|e| matches!(e.disposition, EffectDisposition::Applied { .. }))
                .collect();
            assert_eq!(applied.len(), 1);
            assert_eq!(
                applied[0].disposition,
                EffectDisposition::Applied {
                    value: integer(value)
                }
            );
            let RuleEffectKind::Contribute { stat, .. } = &applied[0].effect else {
                unreachable!()
            };
            assert_eq!(stat, expected_stat);
            assert_eq!(
                result
                    .effects
                    .iter()
                    .filter(|e| matches!(e.disposition, EffectDisposition::Inactive))
                    .count(),
                1
            );
        }
        assert!(
            compiled
                .evaluate(&row.owner, &p.id, &[], base.schema(), &mut scratch)
                .unwrap()
                .effects
                .iter()
                .all(|e| matches!(e.disposition, EffectDisposition::Unresolved { .. }))
        );
    }
}
#[test]
fn class_precedes_primary_ascendancy_and_inactive_branch_does_not_demand_its_input() {
    let mut f = Fixture::new();
    f.with_ascendancy();
    let out = f.compile().unwrap();
    let base = assemble_owned_recipe(out.successor.clone(), Default::default()).unwrap();
    let compiled =
        CompiledRulePackage::compile(base.rules().input(), base.schema(), Default::default())
            .unwrap();
    let mut scratch = compiled.new_scratch();
    let row = out
        .successor
        .rules
        .owners
        .iter()
        .find(|r| {
            r.programs
                .members
                .iter()
                .any(|p| p.id == key("passive-view") && p.reads.len() == 2)
        })
        .unwrap();
    let p = &row.programs.members[0];
    for (class, asc, value) in [
        (true, Some(true), 13),
        (true, Some(false), 13),
        (true, None, 13),
        (false, Some(true), 17),
        (false, Some(false), 7),
    ] {
        let facts: Vec<_> = p
            .reads
            .iter()
            .filter_map(|r| match r.source {
                RuleReadSource::CharacterClassIs { .. } => Some(RuleFact {
                    read: r.id.clone(),
                    value: ParameterValue::Boolean(class),
                }),
                RuleReadSource::CharacterAscendancyIs { .. } => asc.map(|v| RuleFact {
                    read: r.id.clone(),
                    value: ParameterValue::Boolean(v),
                }),
                _ => panic!("identity read"),
            })
            .collect();
        let result = compiled
            .evaluate(&row.owner, &p.id, &facts, base.schema(), &mut scratch)
            .unwrap();
        let applied: Vec<_> = result
            .effects
            .iter()
            .filter(|e| matches!(e.disposition, EffectDisposition::Applied { .. }))
            .collect();
        assert_eq!(applied.len(), 1);
        assert_eq!(
            applied[0].disposition,
            EffectDisposition::Applied {
                value: integer(value)
            }
        );
        assert_eq!(
            result
                .effects
                .iter()
                .filter(|e| matches!(e.disposition, EffectDisposition::Inactive))
                .count(),
            2
        );
    }
}
#[test]
fn unchanged_recipe_and_policy_order_are_idempotent() {
    let mut f = Fixture::new();
    f.with_ascendancy();
    let first = f.compile().unwrap();
    f.recipe = first.successor.clone();
    f.policy.nodes.reverse();
    for n in &mut f.policy.nodes {
        n.views.reverse();
    }
    f.catalog.nodes.reverse();
    let second = f.compile().unwrap();
    assert!(second.refined.is_empty());
    assert_eq!(second.receipt.changed_program_owners, 0);
    assert_eq!(first.successor, second.successor);
}
#[test]
fn whole_list_view_membership_shape_and_pool_must_be_explicitly_reviewed() {
    let mut f = Fixture::new();
    f.raw_mut().stats.push("unconverted".into());
    bad(&f, "full stat list");
    let mut f = Fixture::new();
    f.raw_mut().views[0].stats.push("unconverted".into());
    bad(&f, "full stat list");
    let mut f = Fixture::new();
    f.raw_mut().views.push(TreeStatViewInput {
        selector: "unreviewed".into(),
        stats: vec![],
    });
    bad(&f, "view membership");
    let mut f = Fixture::new();
    f.policy.nodes[0].views.clear();
    bad(&f, "view membership");
    let mut f = Fixture::new();
    f.raw_mut().unlock.push("unconverted".into());
    bad(&f, "shape or unlock");
    let mut f = Fixture::new();
    f.raw_mut().kind = TreeNodeKind::ImplicitRoot;
    bad(&f, "shape or unlock");
    let mut f = Fixture::new();
    f.policy.nodes[0].pool = TreePoolKind::Ascendancy;
    bad(&f, "shape or unlock");
    let mut f = Fixture::new();
    f.policy.nodes[0].default.contributions.clear();
    bad(&f, "full stat list");
    let mut f = Fixture::new();
    f.with_ascendancy();
    f.policy.nodes[0].views[1].when = f.policy.nodes[0].views[0].when.clone();
    bad(&f, "aliased view predicate");
}
#[test]
fn conflicting_prior_effects_and_unrelated_declaration_gaps_reject() {
    let mut f = Fixture::new();
    let out = f.compile().unwrap();
    f.recipe = out.successor;
    f.policy.nodes[0].views[0].effects.contributions[0].value = integer(99);
    bad(&f, "cannot replace");
    let mut f = Fixture::new();
    let out = f.compile().unwrap();
    let address = out.refined[0].clone();
    let DefinitionDescriptor::PassiveNode(entry) = f
        .recipe
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == address)
        .unwrap()
    else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    let SchemaClosure::Partial { gaps } = &mut schema.declarations.parameters.closure else {
        unreachable!()
    };
    gaps[0].code = key("unrelated-input-obligation");
    rebind(&mut f.recipe);
    bad(&f, "cannot replace");
}
#[test]
fn new_receiver_rules_merge_idempotently_without_closing_existing_receiver_membership() {
    let mut f = Fixture::new();
    f.with_receiver();
    f.recipe.rules.receivers.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: SchemaSubject::Definition(f.stats[0].address()),
            facet: SchemaFacet::GameRules,
            code: key("other-receivers-unconverted"),
        }],
    };
    let first = f.compile().unwrap();
    assert_eq!(first.receipt.added_receiver_owners, 1);
    assert_eq!(first.receipt.added_receivers, 1);
    assert_eq!(
        first.successor.rules.receivers.closure,
        f.recipe.rules.receivers.closure
    );
    f.recipe = first.successor.clone();
    let second = f.compile().unwrap();
    assert_eq!(second.successor, first.successor);
    assert_eq!(second.receipt.added_receiver_owners, 0);
    assert_eq!(second.receipt.added_receivers, 0);
    f.policy.receivers[0].program = key("different-program");
    bad(&f, "cannot replace");
    let mut f = Fixture::new();
    f.with_receiver();
    f.policy.receiver_rules[0].owner = SchemaSubject::Definition(
        f.recipe
            .schema
            .definitions
            .iter()
            .find_map(|d| {
                if let DefinitionDescriptor::Class(e) = d {
                    Some(e.id.address())
                } else {
                    None
                }
            })
            .unwrap(),
    );
    bad(&f, "must be Stat");
}
#[test]
fn invalid_numeric_types_units_bindings_and_limits_reject_without_mutation() {
    let mut f = Fixture::new();
    f.policy.nodes[0].default.contributions[0].value = ParameterValue::Boolean(false);
    bad(&f, "must be numeric");
    let mut f = Fixture::new();
    f.policy.nodes[0].default.contributions[0].contribution = ContributionKind::Increase;
    bad(&f, "quantity required");
    let mut f = Fixture::new();
    f.catalog.source.files[0].sha256 = "0".repeat(64);
    bad(&f, "binding differs");
    let mut f = Fixture::new();
    f.policy.nodes[0].views[0].when = ViewSelector::Class {
        key: "missing-class".into(),
    };
    bad(&f, "unresolved exact selector");
    let f = Fixture::new();
    let (base, mapping) = f.checked();
    for limits in [
        ViewRecipeLimits {
            max_nodes: 1,
            ..Default::default()
        },
        ViewRecipeLimits {
            max_work: 1,
            ..Default::default()
        },
        ViewRecipeLimits {
            max_wire_bytes: 1,
            ..Default::default()
        },
        ViewRecipeLimits {
            max_contributions: 1,
            ..Default::default()
        },
    ] {
        assert!(
            compile_owned_passive_views(&base, &mapping, &f.catalog, &f.policy, limits).is_err()
        );
    }
    assert_eq!(base.registry().input(), &f.recipe.registry);
}

#[test]
fn default_only_passive_uses_the_same_full_list_conversion_without_character_facts() {
    let mut f = Fixture::new();
    let raw = f.catalog.nodes.iter().find(|n| n.key == "34202").unwrap();
    assert_eq!(raw.stats, ["+8 to Strength"]);
    assert!(raw.views.is_empty() && raw.unlock.is_empty());
    let unrequested_view = f.policy.nodes[0].views[0].clone();
    f.policy.nodes = vec![ViewNodePolicy {
        node: raw.key.clone(),
        pool: TreePoolKind::Ordinary,
        default: ViewEffectPolicy {
            expected_stats: raw.stats.clone(),
            contributions: vec![ViewContribution {
                stat: f.stats[0].clone(),
                contribution: ContributionKind::Add,
                value: integer(8),
            }],
        },
        views: vec![],
    }];
    let out = f.compile().unwrap();
    assert_eq!(out.refined.len(), 1);
    let row = selected(&out.successor, &out.refined[0]);
    assert!(row.programs.is_complete());
    assert_eq!(row.programs.members.len(), 1);
    let program = &row.programs.members[0];
    assert!(
        program.reads.is_empty(),
        "no class/ascendancy input is implied"
    );
    let staged = assemble_owned_recipe(out.successor.clone(), Default::default()).unwrap();
    let compiled =
        CompiledRulePackage::compile(staged.rules().input(), staged.schema(), Default::default())
            .unwrap();
    let result = compiled
        .evaluate(
            &row.owner,
            &program.id,
            &[],
            staged.schema(),
            &mut compiled.new_scratch(),
        )
        .unwrap();
    assert_eq!(result.effects.len(), 1);
    assert_eq!(
        result.effects[0].disposition,
        EffectDisposition::Applied { value: integer(8) }
    );
    assert!(matches!(&result.effects[0].effect,
        RuleEffectKind::Contribute { entity: RuleEntity::Player, stat, contribution: ContributionKind::Add, .. }
        if stat == &f.stats[0]));

    f.policy.nodes[0].views.push(unrequested_view);
    bad(&f, "view membership");
    f.policy.nodes[0].views.clear();
    f.policy.nodes[0]
        .default
        .expected_stats
        .push("unreviewed".into());
    bad(&f, "full stat list");
    f.policy.nodes[0].default.expected_stats.pop();
    let reviewed_stats = f.raw_mut().stats.clone();
    f.raw_mut().stats.clear();
    f.policy.nodes[0].default.expected_stats.clear();
    f.policy.nodes[0].default.contributions.clear();
    bad(&f, "shape or unlock");
    f.raw_mut().stats = reviewed_stats.clone();
    f.policy.nodes[0].default.expected_stats = reviewed_stats;
    f.policy.nodes[0].default.contributions = vec![ViewContribution {
        stat: f.stats[0].clone(),
        contribution: ContributionKind::Add,
        value: integer(8),
    }];
    f.recipe = out.successor.clone();
    let replay = f.compile().unwrap();
    assert!(replay.refined.is_empty());
    assert_eq!(replay.successor, out.successor);
}
