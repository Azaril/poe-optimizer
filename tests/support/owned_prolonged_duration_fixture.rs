//! Shared finite physical Offering/Ice topology with exact published programs.
//! Historical callers explicitly retain the final-input fixture boundary; joined
//! input callers remove it and supply their checked preparation relation.
//! This module contains no tests or runtime fallback path.
#[allow(dead_code)]
#[path = "owned_bidding_delivery_fixture.rs"]
pub mod bidding_fixture;
#[allow(dead_code)]
#[path = "owned_magnified_area_fixture.rs"]
pub mod fixture;

use fixture::{decode, def, id, key, quantity, subject};
use poe_optimizer_core::{
    build_identity::SupportAssignmentId, owned_build::*, owned_definitions::*, owned_rules::*,
    owned_schema::*, owned_supports::*,
};
use poe_optimizer_import::{
    owned_release::StagedOwnedRelease, owned_release_migration::OwnedReleaseMigrationInput,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::PathBuf, sync::OnceLock};

pub(super) const SOURCES: [usize; 3] = [2, 3, 4];
pub(super) const OFFERINGS: [usize; 2] = [3, 4];
pub const FINAL_BOUNDARY: &str = "fixture.physical-final-inputs";

pub fn finite<T: Serialize + DeserializeOwned>(value: &T) -> T {
    fn close(value: &mut Value) {
        match value {
            Value::Object(fields) => {
                if fields.contains_key("members") && fields.contains_key("closure") {
                    fields.insert("closure".into(), json!({"kind":"complete"}));
                }
                for child in fields.values_mut() {
                    close(child);
                }
            }
            Value::Array(rows) => {
                for child in rows {
                    close(child);
                }
            }
            _ => {}
        }
    }
    let mut value = serde_json::to_value(value).unwrap();
    close(&mut value);
    decode(&value)
}

#[derive(Clone)]
pub struct World {
    pub base: fixture::World,
    pub bindings: Value,
    pub originals: Vec<DefinitionRules>,
    pub physical_owner: DefinitionRules,
    pub receiving: bidding_fixture::Receiving,
}
impl World {
    pub(super) fn load() -> Self {
        static WORLD: OnceLock<World> = OnceLock::new();
        WORLD.get_or_init(Self::load_once).clone()
    }
    fn load_once() -> Self {
        let path = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_PROLONGED_RELEASE")
                .expect("checked Prolonged Duration publication"),
        );
        Self::load_release(&path, true)
    }
    /// A successor is independently authenticated by its own test family. The
    /// existing Prolonged support/topology data remain this fixture's seed.
    pub fn load_release(path: &std::path::Path, historical_endpoint: bool) -> Self {
        let before = crate::release::inventory(path);
        let endpoint = crate::release::load(path);
        if historical_endpoint {
            crate::family::assert_endpoint(&endpoint);
        }
        let packet = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data/owned/poe2/3887ae68/prolonged-duration-support-delivery");
        let bindings: Value = bidding_fixture::read(packet.join("bindings.json"));
        let migration: OwnedReleaseMigrationInput =
            bidding_fixture::read(packet.join("migration.json"));
        let preparation: SupportPreparationInput =
            bidding_fixture::read(packet.join("preparation.json"));
        let receiving: bidding_fixture::Receiving =
            bidding_fixture::read(packet.join("receiving.json"));
        assert_eq!(bindings["supports"].as_array().unwrap().len(), 2);
        assert_eq!(migration.owners.len(), 2);
        let mut base = fixture::World::load_release(path, false);
        base.set_area_fact(None);
        for field in [
            "physical_gem",
            "primary_skill",
            "primary_supply",
            "entering_grant",
            "output",
            "part",
            "mode",
            "stat_sets",
        ] {
            assert_eq!(
                bindings["contrast"][field], base.ice[field],
                "exact Ice contrast {field}"
            );
        }
        base.add_support_fragment(
            &endpoint,
            &bindings,
            &migration.owners,
            &preparation,
            &receiving,
        );
        let prior_supports: Vec<GemDefId> = base.bindings["supports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| decode(&row["gem"]))
            .collect();
        base.inner
            .owners
            .retain(|o| !prior_supports.iter().any(|g| o.owner == subject(g.clone())));
        base.inner.preparation = preparation;
        base.inner.bindings = bindings.clone();
        base.inner.channels = bindings["channels"].clone();
        base.inner.receiving = bidding_fixture::Receiving {
            roles: receiving.roles.clone(),
            targets: finite(&receiving.targets),
            supports: finite(&receiving.supports),
        };
        // The existing fixture intentionally supplies finite effective inputs for
        // preparation. Actual published 30ae/30af outputs are asserted separately;
        // they are not misrepresented as a complete preparation assembly here.
        for owner in [
            subject(decode::<GemDefId>(&base.ice["physical_gem"])),
            subject(decode::<SkillDefId>(&base.ice["primary_skill"])),
        ] {
            let facts = base
                .inner
                .owner_mut(owner)
                .programs
                .members
                .iter_mut()
                .find(|p| p.id == key("fixture.initial-facts"))
                .unwrap();
            set_fact(facts, "type.type.duration", true);
            set_fact(facts, "type.type.spell", true);
        }
        let physical_owner = install_physical_source(&mut base, &endpoint, &bindings["target"]);
        base.inner.build.supports.clear();
        base.inner.build.authored_support_order = Some(vec![]);
        base.inner.build.gems.retain(|g| g.id == id(900));
        base.inner.build.skills.retain(|s| s.id == id(22));
        for source in OFFERINGS {
            add_physical_occurrence(&mut base, &bindings["target"], source);
        }
        for source in SOURCES {
            base.inner.add_support(source, 0);
        }
        assert_eq!(before, crate::release::inventory(path));
        Self {
            base,
            bindings,
            originals: migration.owners,
            physical_owner,
            receiving,
        }
    }
    pub(super) fn target(&self, source: usize) -> &Value {
        if source == self.base.ice_source {
            &self.bindings["contrast"]
        } else {
            assert!(OFFERINGS.contains(&source));
            &self.bindings["target"]
        }
    }
    pub fn actions(&self, source: usize) -> Vec<ActionSelection> {
        let target = self.target(source);
        target["stat_sets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|set| ActionSelection {
                action: ActionKey {
                    actor: ActorKey::Player,
                    provider: ProviderKey {
                        root: ProviderRoot::SkillUse(id(20 + source as u64)),
                        grant_path: vec![decode(&target["entering_grant"])],
                    },
                    output: decode(&target["output"]),
                },
                part: decode(&target["part"]),
                mode: decode(&target["mode"]),
                stat_set: decode(&set["stat_set"]),
            })
            .collect()
    }
    pub fn request(&self) -> OwnedEvaluationRequest {
        OwnedEvaluationRequest::new(
            BuildSpec::new(self.base.inner.build.clone(), Default::default()).unwrap(),
            ScenarioSpec::new(
                ScenarioInput {
                    game_version: bidding_fixture::ns(),
                    enemy: EnemySpec {
                        encounter: def("fixture.encounter"),
                        level: 20,
                    },
                    assumptions: vec![],
                    usage: vec![],
                },
                Default::default(),
            )
            .unwrap(),
            QuerySpec::new(
                QueryInput {
                    game_version: bidding_fixture::ns(),
                    requests: SOURCES
                        .into_iter()
                        .flat_map(|s| self.actions(s))
                        .enumerate()
                        .map(|(i, a)| MetricRequest {
                            id: QueryId::new(format!("prolonged-{i}")).unwrap(),
                            metric: def("fixture.observe"),
                            target: MetricTarget::Action(Box::new(a)),
                        })
                        .collect(),
                },
                Default::default(),
            )
            .unwrap(),
            Default::default(),
        )
        .unwrap()
    }
    pub(super) fn checked_plan(&self) -> std::result::Result<bidding_fixture::Plan, String> {
        let target = &self.bindings["target"];
        self.base.checked_plan_with_readiness_inputs(
            self.request(),
            &[
                decode(&target["final_level_parameter"]),
                decode(&target["final_quality_parameter"]),
            ],
            &[(
                self.physical_owner.owner.clone(),
                key(target["primary_supply_program"].as_str().unwrap()),
            )],
        )
    }
    pub(super) fn plan(&self) -> bidding_fixture::Plan {
        self.checked_plan().unwrap()
    }
    pub fn selected(&self, source: usize) -> SupportAssignmentId {
        let rows: Vec<_> = self
            .base
            .inner
            .build
            .supports
            .iter()
            .filter(|s| s.target == SkillTarget::Authored(id(20 + source as u64)))
            .collect();
        assert_eq!(rows.len(), 1);
        rows[0].id
    }
}

pub(super) fn set_fact(program: &mut RuleProgram, name: &str, value: bool) {
    program
        .nodes
        .iter_mut()
        .find(|n| n.id == key(name))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(value),
    };
}

/// Copy exact checked physical topology, then explicitly close only its finite
/// component copy. No Actor is manufactured from SkillType.Minion metadata.
fn install_physical_source(
    base: &mut fixture::World,
    endpoint: &StagedOwnedRelease,
    target: &Value,
) -> DefinitionRules {
    let gem: GemDefId = decode(&target["physical_gem"]);
    let skill: SkillDefId = decode(&target["primary_skill"]);
    let mut wanted = BTreeSet::from([
        gem.address(),
        skill.address(),
        decode::<ActionPartDefId>(&target["part"]).address(),
        decode::<ActionModeDefId>(&target["mode"]).address(),
    ]);
    for set in target["stat_sets"].as_array().unwrap() {
        wanted.insert(decode::<ActionStatSetDefId>(&set["stat_set"]).address());
    }
    assert_eq!(target["stat_sets"].as_array().unwrap().len(), 1);
    let schema = &endpoint.input().recipe.schema;
    for address in wanted {
        let actual = schema
            .definitions
            .iter()
            .find(|d| d.address() == address)
            .unwrap();
        assert!(
            !base
                .inner
                .schema
                .definitions
                .iter()
                .any(|d| d.address() == address)
        );
        base.inner.schema.definitions.push(finite(actual));
    }
    let declarations = [
        SlotOwnerDefId::Gem(gem.clone()),
        SlotOwnerDefId::Skill(skill.clone()),
    ];
    for slot in &schema.slots {
        let value = serde_json::to_value(slot).unwrap();
        let declaration: SlotOwnerDefId = decode(&value["value"]["id"]["declaration"]);
        if declarations.contains(&declaration) {
            assert!(
                !matches!(slot, SlotDescriptor::Actor(_)),
                "source has no attached minion"
            );
            assert!(!base.inner.schema.slots.iter().any(|s| s == slot));
            base.inner.schema.slots.push(finite(slot));
        }
    }
    let physical_owner = endpoint
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == subject(gem.clone()))
        .unwrap()
        .clone();
    assert!(!physical_owner.programs.is_complete());
    assert_eq!(
        physical_owner
            .programs
            .members
            .iter()
            .filter(|p| p.id.as_str() == target["primary_supply_program"].as_str().unwrap())
            .count(),
        1
    );
    for owner in [subject(gem.clone()), subject(skill.clone())] {
        let actual = endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == owner)
            .unwrap();
        assert!(!base.inner.owners.iter().any(|o| o.owner == owner));
        base.inner.owners.push(finite(actual));
    }
    for slot in [
        SchemaSubject::Slot(SlotAddress::Grant(decode(&target["entering_grant"]))),
        SchemaSubject::Slot(SlotAddress::SkillGrant(decode(&target["primary_supply"]))),
        SchemaSubject::Slot(SlotAddress::ActionOutput(decode(&target["output"]))),
    ] {
        base.inner.owner_mut(slot);
    }
    let mut facts = base
        .inner
        .owner_mut(subject(decode::<GemDefId>(&base.ice["physical_gem"])))
        .programs
        .members
        .iter()
        .find(|p| p.id == key("fixture.initial-facts"))
        .unwrap()
        .clone();
    // Finite relevant source vocabulary. The optional minion context stays absent:
    // source types Minion/CreatesMinion do not create an owned recipient actor.
    set_fact(&mut facts, "type.type.spell", false);
    for name in [
        "type.type.duration",
        "type.type.minion",
        "type.type.buff",
        "type.type.creates-minion",
        "type.type.usable-while-moving",
        "type.type.triggerable",
    ] {
        set_fact(&mut facts, name, true);
    }
    for owner in [subject(gem.clone()), subject(skill)] {
        base.inner
            .owner_mut(owner)
            .programs
            .members
            .push(facts.clone());
    }
    let quality = base.inner.quality_unit.clone();
    base.inner
        .owner_mut(subject(gem))
        .programs
        .members
        .push(RuleProgram {
            id: key(FINAL_BOUNDARY),
            context: RuleEntityKind::Skill,
            reads: vec![],
            nodes: vec![
                RuleNode {
                    id: key("level"),
                    expression: RuleExpression::Literal {
                        value: ParameterValue::Integer(BoundedInteger::new(22).unwrap()),
                    },
                },
                RuleNode {
                    id: key("quality"),
                    expression: RuleExpression::Literal {
                        value: quantity(0.0, &quality),
                    },
                },
            ],
            effects: [
                ("level", "physical_final_level"),
                ("quality", "physical_final_quality"),
            ]
            .into_iter()
            .map(|(name, field)| RuleEffect {
                id: key(name),
                when: None,
                effect: RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat: decode(&target[field]),
                    value: key(name),
                },
            })
            .collect(),
        });
    physical_owner
}

fn add_physical_occurrence(base: &mut fixture::World, target: &Value, source: usize) {
    let gem: GemDefId = decode(&target["physical_gem"]);
    let parameters = base
        .inner
        .schema
        .slots
        .iter()
        .filter_map(|slot| match slot {
            SlotDescriptor::Parameter(DefinitionEntry {
                id,
                schema: SchemaState::Known(p),
            }) if id.declaration == SlotOwnerDefId::Gem(gem.clone()) => Some(ParameterAssignment {
                slot: id.clone(),
                value: match &p.value {
                    ValueSchema::Boolean => ParameterValue::Boolean(false),
                    ValueSchema::Quantity(q) => quantity(0.0, q.minimum.unit()),
                    _ => panic!("actual physical corruption input"),
                },
            }),
            _ => None,
        })
        .collect();
    let instance = id(900 + (source - 2) as u64);
    base.inner.build.gems.push(GemInstance {
        id: instance,
        definition: gem,
        parameters,
        level: 20,
        quality: Some(QualitySelection {
            kind: def("def.0000000000000006"),
            amount: FiniteQuantity::new(0.0, base.inner.quality_unit.clone()).unwrap(),
        }),
    });
    base.inner.build.skills.push(SkillUse {
        id: id(20 + source as u64),
        source: AuthoredSkillSource::Gem(instance),
        parameters: None,
        enabled: true,
        scope: LoadoutScope::Shared,
    });
}
