//! Explicit finite component world over actual Djinn identities. Preparation and
//! inherited delivery use the production engine. Only the fixture supplies the
//! isolated activation/type facts and complete contributor inventory; none of
//! those declarations is published as whole-build coverage.
//!
//! The authenticated corrected release separates the imported support primary
//! association from potential Skill supplies. This fixture preserves its actual
//! Gem membership and Unmapped catalogue descriptors. Finite closures and test
//! activation inputs still do not establish production support-topology closure.
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_readiness::*, owned_routing::*,
    owned_rules::*, owned_schema::*, owned_source_properties::SourcePropertyPreparationInput,
    owned_stages::*, owned_support_inputs::*, owned_support_receiving::*, owned_supports::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting,
    owned_rules::OwnedRulePackage,
    owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageInput},
    owned_stages::OwnedEvaluationStages,
    owned_support_inputs::OwnedSupportInputBindings,
    owned_support_receiving::OwnedSupportReceiving,
    owned_supports::OwnedSupportPreparation,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry;
use poe_optimizer_import::owned_release_migration::OwnedReleaseMigrationInput;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

pub type Plan = OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>;
/// Explicit opt-in components for joined finite fixtures. Historical worlds use
/// the empty defaults and retain their existing stage and input configuration.
pub struct PlanComponents {
    pub tables: Vec<IntegerRuleTable>,
    pub receivers: DeclaredSet<StatReceiver>,
    pub source_properties: Option<SourcePropertyPreparationInput>,
}
impl Default for PlanComponents {
    fn default() -> Self {
        Self {
            tables: vec![],
            receivers: DeclaredSet::complete(vec![]),
            source_properties: None,
        }
    }
}
pub fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
pub fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
pub fn def<T: DefinitionDomain>(s: &str) -> DefId<T> {
    DefId::parse(ns(), s).unwrap()
}
pub fn id<T: BuildInstanceId>(n: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([119; 16]), n).unwrap())
}
pub fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68")
}
pub fn packet<T: DeserializeOwned>(name: &str) -> T {
    read(root().join("bidding-support-delivery").join(name))
}
pub fn decode<T: DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn ports() -> DeclaredSlots {
    DeclaredSlots {
        parameters: empty(),
        choices: empty(),
        grants: empty(),
        actors: empty(),
        skill_grants: empty(),
        outputs: empty(),
        sockets: empty(),
    }
}
fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
pub fn subject<I: SchemaDefinitionId>(id: I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
fn literal(id: &str, value: ParameterValue) -> RuleNode {
    RuleNode {
        id: key(id),
        expression: RuleExpression::Literal { value },
    }
}
fn effect(id: &str, effect: RuleEffectKind) -> RuleEffect {
    RuleEffect {
        id: key(id),
        when: None,
        effect,
    }
}
fn derive(id: &str, stat: StatDefId) -> RuleEffect {
    effect(
        id,
        RuleEffectKind::Derive {
            entity: RuleEntity::Current,
            stat,
            value: key(id),
        },
    )
}
pub fn quantity(value: f64, unit: &UnitDefId) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap())
}
/// Deliberate isolation, applied only after authenticating exact prior records.
fn finite<T: Serialize + DeserializeOwned>(source: &T) -> T {
    fn visit(v: &mut Value) {
        match v {
            Value::Object(o) => {
                if o.contains_key("members") && o.contains_key("closure") {
                    o.insert("closure".into(), serde_json::json!({"kind":"complete"}));
                }
                for v in o.values_mut() {
                    visit(v)
                }
            }
            Value::Array(a) => {
                for v in a {
                    visit(v)
                }
            }
            _ => {}
        }
    }
    let mut value = serde_json::to_value(source).unwrap();
    visit(&mut value);
    decode(&value)
}
#[derive(Clone, Deserialize)]
pub struct Receiving {
    pub roles: Vec<SupportReceivingRole>,
    pub targets: Vec<SupportTargetReceivingRoles>,
    pub supports: Vec<SupportReceivingEntry>,
}
#[derive(Clone)]
pub struct World {
    pub schema: SchemaPackageInput,
    pub operations: OwnedDefinitionKey,
    pub owners: Vec<DefinitionRules>,
    pub build: BuildInput,
    pub families: Vec<Value>,
    pub preparation: SupportPreparationInput,
    pub receiving: Receiving,
    pub original_receiving: Receiving,
    pub original_owners: Vec<DefinitionRules>,
    pub catalogue_primary_skills: Vec<(GemDefId, SkillDefId)>,
    pub channels: Value,
    pub bindings: Value,
    pub skill_types: Vec<OwnedDefinitionKey>,
    pub quality_unit: UnitDefId,
    pub factor_unit: UnitDefId,
    pub sources: Vec<usize>,
}
impl World {
    pub fn load() -> Self {
        static WORLD: OnceLock<World> = OnceLock::new();
        WORLD.get_or_init(Self::load_once).clone()
    }
    fn load_once() -> Self {
        let prior = PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_BIDDING_RELEASE").expect(
            "set exact checked Gem membership correction release for optional Bidding native component",
        ));
        Self::load_release(&prior, true)
    }
    pub fn load_release(prior: &Path, historical_endpoint: bool) -> Self {
        let before = super::super::release::inventory(prior);
        let endpoint = super::super::release::load(prior);
        let prior_schema = endpoint.input().recipe.schema.clone();
        let bindings: Value = packet("bindings.json");
        let migration: OwnedReleaseMigrationInput = packet("migration.json");
        let correction: OwnedReleaseMigrationInput =
            read(root().join("gem-executable-memberships-v1/migration.json"));
        let correction_authoring: Value =
            read(root().join("gem-executable-memberships-v1/authoring.json"));
        let correction_bindings: Value =
            read(root().join("gem-executable-memberships-v1/bindings.json"));
        let correction_dependencies: Value =
            read(root().join("gem-executable-memberships-v1/dependencies.json"));
        let provenance: Vec<_> = endpoint
            .input()
            .provenance
            .iter()
            .filter(|p| p.kind == key("gem-executable-memberships-v1"))
            .collect();
        assert_eq!(provenance.len(), 1);
        let provenance = provenance[0];
        if historical_endpoint {
            assert_eq!(endpoint.input().provenance.last(), Some(provenance));
            assert_eq!(endpoint.input().recipe.schema.release, correction.release);
        }
        assert_eq!(
            provenance.authoring_input,
            poe_optimizer_core::owned_content::digest_owned(
                "owned-gem-executable-memberships-v1",
                &(
                    correction_authoring,
                    correction_bindings,
                    correction_dependencies,
                    &correction
                ),
                8 * 1024 * 1024,
            )
            .unwrap()
        );
        assert_eq!(correction.schema.len(), 568);
        assert_eq!(provenance.prior_input, correction.before);
        // The loader authenticates the complete manifest and typed dependencies;
        // all correction rows and published Bidding programs are checked exactly.
        for entry in correction.schema.iter().chain(&migration.schema) {
            let SchemaExtensionEntry::Definition(definition) = entry else {
                panic!("Gem correction or Bidding Stat only")
            };
            assert_eq!(
                endpoint
                    .input()
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .filter(|d| *d == definition)
                    .count(),
                1
            );
        }
        for owner in &migration.owners {
            let rows: Vec<_> = endpoint
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|r| r.owner == owner.owner)
                .collect();
            assert_eq!(rows.len(), 1);
            if historical_endpoint {
                assert_eq!(rows[0], owner);
            } else {
                // A later checked release can append independent programs. The
                // finite Bidding world retains only its exact historical body;
                // the later family's loader authenticates its added programs.
                assert_eq!(rows[0].programs.closure, owner.programs.closure);
                for program in &owner.programs.members {
                    assert_eq!(
                        rows[0]
                            .programs
                            .members
                            .iter()
                            .filter(|p| *p == program)
                            .count(),
                        1
                    );
                }
            }
        }
        assert_eq!(before, super::super::release::inventory(prior));
        let original_receiving: Receiving = packet("receiving.json");
        let families = read::<Value>(root().join("djinn-actions/bindings.json"))["families"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(families.len(), 2);
        let mut preparation: SupportPreparationInput =
            read(root().join("djinn-support-preparation/preparation.json"));
        let gems: Vec<GemDefId> = bindings["supports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| decode(&s["gem"]))
            .collect();
        assert_eq!(gems.len(), 2);
        assert_eq!(
            gems.iter().cloned().collect::<BTreeSet<_>>(),
            BTreeSet::from([def("def.0000000000000674"), def("def.0000000000000673")])
        );
        preparation.supports.retain(|s| gems.contains(&s.gem));
        assert_eq!(preparation.supports.len(), 2);
        let quality_unit = preparation.quality_unit.clone();
        let factor_unit = decode(&bindings["factor_unit"]);
        let channels = bindings["channels"].clone();
        assert_eq!(
            decode::<StatDefId>(&channels["cooldown"]),
            def("def.00000000000032ea")
        );
        assert_eq!(
            decode::<StatDefId>(&channels["commandable"]),
            def("def.00000000000032eb")
        );
        assert_eq!(
            decode::<StatDefId>(&channels["damage_factor"]),
            def("def.00000000000032f8")
        );
        let mut wanted = BTreeSet::new();
        let mut catalogue_primary_skills = Vec::new();
        for family in &families {
            wanted.insert(decode::<SkillDefId>(&family["skill"]).address());
            wanted.insert(decode::<ActorDefId>(&family["minion"]["actor"]).address());
            for action in
                std::iter::once(&family["command"]).chain(family["actions"].as_array().unwrap())
            {
                wanted.insert(decode::<SkillDefId>(&action["skill"]).address());
                wanted.insert(decode::<ActionPartDefId>(&action["part"]).address());
                wanted.insert(decode::<ActionModeDefId>(&action["mode"]).address());
                for set in action["stat_sets"].as_array().unwrap() {
                    wanted.insert(decode::<ActionStatSetDefId>(&set["stat_set"]).address());
                }
            }
        }
        for gem in &gems {
            wanted.insert(gem.address());
        }
        for p in &preparation.supports {
            let DefinitionDescriptor::Gem(DefinitionEntry {
                schema: SchemaState::Known(g),
                ..
            }) = prior_schema
                .definitions
                .iter()
                .find(|d| d.address() == p.gem.address())
                .unwrap()
            else {
                panic!("actual Gem")
            };
            assert!(!g.declarations.parameters.is_complete());
            assert!(
                g.skills.members.is_empty(),
                "published support-only Gem has no standalone Skill supply"
            );
            assert!(
                !g.skills.is_complete(),
                "production coverage remains incomplete"
            );
            let association = endpoint
                .input()
                .roles
                .roles
                .iter()
                .find(|r| r.gem == p.gem)
                .unwrap();
            let poe_optimizer_import::owned_skill_catalog::OwnedPrimarySkill::Known(primary) =
                &association.primary
            else {
                panic!("exact imported primary association")
            };
            let expected = bindings["supports"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| decode::<GemDefId>(&s["gem"]) == p.gem)
                .unwrap();
            assert_eq!(*primary, decode::<SkillDefId>(&expected["skill"]));
            catalogue_primary_skills.push((p.gem.clone(), primary.clone()));
            wanted.insert(primary.address());
        }
        wanted.insert(decode::<StatDefId>(&channels["cooldown"]).address());
        wanted.insert(decode::<StatDefId>(&channels["commandable"]).address());
        wanted.insert(decode::<StatDefId>(&channels["damage_factor"]).address());
        let definitions: Vec<_> = prior_schema
            .definitions
            .iter()
            .filter(|d| {
                wanted.contains(&d.address())
                    || matches!(
                        d,
                        DefinitionDescriptor::Unit(_) | DefinitionDescriptor::Quality(_)
                    )
            })
            .cloned()
            .collect();
        let owner_addresses: BTreeSet<_> = definitions
            .iter()
            .filter_map(|d| match d {
                DefinitionDescriptor::Skill(v) => Some(SlotOwnerDefId::Skill(v.id.clone())),
                DefinitionDescriptor::Actor(v) => Some(SlotOwnerDefId::Actor(v.id.clone())),
                DefinitionDescriptor::Gem(v) => Some(SlotOwnerDefId::Gem(v.id.clone())),
                _ => None,
            })
            .collect();
        let slots: Vec<_> = prior_schema
            .slots
            .iter()
            .filter(|s| {
                let raw = serde_json::to_value(s).unwrap();
                let owner: SlotOwnerDefId = decode(&raw["value"]["id"]["declaration"]);
                owner_addresses.contains(&owner)
            })
            .cloned()
            .collect();
        assert!(!slots.is_empty());
        for d in &definitions {
            if let DefinitionDescriptor::Skill(DefinitionEntry {
                schema: SchemaState::Known(s),
                id,
            }) = d
                && families
                    .iter()
                    .any(|f| decode::<SkillDefId>(&f["skill"]) == *id)
            {
                assert!(!s.declarations.parameters.is_complete());
            }
        }
        let mut schema = prior_schema.clone();
        schema.release = key("fixture-bidding-delivery");
        schema.definitions = finite(&definitions);
        schema.slots = finite(&slots);
        let mut original_owners: Vec<_> = endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .filter(|owner| migration.owners.iter().any(|m| m.owner == owner.owner))
            .cloned()
            .collect();
        if !historical_endpoint {
            original_owners = migration.owners.clone();
        }
        assert_eq!(original_owners.len(), migration.owners.len());
        assert!(
            original_owners
                .iter()
                .filter(|o| gems.iter().any(|g| o.owner == subject(g.clone())))
                .all(|o| !o.programs.is_complete()),
            "no live Gem rule closure promoted by this component"
        );
        let skill_types = preparation.types.clone();
        let mut world = Self {
            operations: endpoint.input().recipe.rules.operations_version.clone(),
            schema,
            owners: finite(&original_owners),
            build: BuildInput {
                allocator: InstanceAllocatorState::from_parts(
                    BuildLineage::from_bytes([119; 16]),
                    10000,
                ),
                revision: BuildRevision::from_u64(1),
                game_version: ns(),
                character: CharacterSpec {
                    class: def("fixture.class"),
                    ascendancy: None,
                    level: 20,
                    rewards: vec![],
                },
                weapon_loadouts: vec![id(1), id(2)],
                active_weapon_loadout: id(1),
                items: vec![],
                gems: vec![],
                equipment: vec![],
                allocations: vec![],
                skills: vec![],
                supports: vec![],
                support_origins: Some(vec![]),
                generated_inputs: None,
                payload_links: vec![],
                choices: vec![],
            },
            families,
            preparation,
            receiving: Receiving {
                roles: original_receiving.roles.clone(),
                targets: finite(&original_receiving.targets),
                supports: finite(&original_receiving.supports),
            },
            original_receiving,
            original_owners,
            catalogue_primary_skills,
            channels,
            bindings,
            skill_types,
            quality_unit,
            factor_unit,
            sources: vec![],
        };
        world.add_fixture_definitions();
        world.add_fixture_programs();
        let ii = world.support_index("SupportBiddingPlayerTwo");
        world.add_source(0, ii);
        world.add_source(1, ii);
        world
    }
    pub fn owner_mut(&mut self, owner: SchemaSubject) -> &mut DefinitionRules {
        if !self.owners.iter().any(|o| o.owner == owner) {
            self.owners.push(DefinitionRules {
                owner: owner.clone(),
                programs: empty(),
            });
        }
        self.owners.iter_mut().find(|o| o.owner == owner).unwrap()
    }
    pub fn support_index(&self, effect: &str) -> usize {
        self.bindings["supports"]
            .as_array()
            .unwrap()
            .iter()
            .position(|s| s["source_effect"] == effect)
            .unwrap()
    }
    pub fn support_gem(&self, effect: &str) -> GemDefId {
        decode(&self.bindings["supports"][self.support_index(effect)]["gem"])
    }
    pub fn change_support(&mut self, source: usize, effect: &str) {
        let target = SkillTarget::Authored(id(20 + source as u64));
        let assignment = self
            .build
            .supports
            .iter()
            .find(|s| s.target == target)
            .unwrap();
        let gem_id = assignment.support;
        let definition = self.support_gem(effect);
        let parameters = self
            .schema
            .slots
            .iter()
            .filter_map(|s| match s {
                SlotDescriptor::Parameter(DefinitionEntry {
                    id,
                    schema: SchemaState::Known(s),
                }) if id.declaration == SlotOwnerDefId::Gem(definition.clone()) => {
                    Some(ParameterAssignment {
                        slot: id.clone(),
                        value: match &s.value {
                            ValueSchema::Boolean => ParameterValue::Boolean(true),
                            ValueSchema::Quantity(r) => quantity(1.0, r.minimum.unit()),
                            _ => panic!("actual saved support input"),
                        },
                    })
                }
                _ => None,
            })
            .collect();
        let gem = self.build.gems.iter_mut().find(|g| g.id == gem_id).unwrap();
        gem.definition = definition;
        gem.parameters = parameters;
    }
    fn stat(&mut self, name: &str, value: ComputedValueType, scope: RuleEntityKind) {
        self.schema
            .definitions
            .push(DefinitionDescriptor::Stat(entry(
                def::<StatDefinition>(name),
                StatSchema {
                    value,
                    targets: vec![scope],
                },
            )));
    }
    fn add_fixture_definitions(&mut self) {
        let range = IntegerRange {
            minimum: BoundedInteger::new(1).unwrap(),
            maximum: BoundedInteger::new(100).unwrap(),
        };
        self.schema.definitions.extend([
            DefinitionDescriptor::Class(entry(
                def("fixture.class"),
                ClassSchema {
                    level: range.clone(),
                    implicit_passives: empty(),
                    ascendancies: empty(),
                    declarations: ports(),
                },
            )),
            DefinitionDescriptor::Encounter(entry(
                def("fixture.encounter"),
                EncounterSchema {
                    enemy_level: range,
                    external_inputs: empty(),
                },
            )),
            DefinitionDescriptor::Metric(entry(
                def("fixture.observe"),
                MetricSchema {
                    targets: vec![MetricTargetKind::Action],
                    unit: self.factor_unit.clone(),
                    actor_roles: vec![MetricActorRole::Player, MetricActorRole::Owned],
                    provider_roles: vec![ProviderRole::SkillUse],
                },
            )),
        ]);
        for flag in [
            "minion-present",
            "summoner-present",
            "summoner-minion-present",
            "cannot-support",
            "has-gem",
            "from-item",
            "is-player",
        ] {
            self.stat(
                &format!("fixture.{flag}"),
                ComputedValueType::Boolean,
                RuleEntityKind::Skill,
            );
        }
        for t in self.skill_types.clone() {
            for prefix in [
                "type",
                "minion-type",
                "summoner-type",
                "summoner-minion-type",
            ] {
                self.stat(
                    &format!("fixture.{prefix}.{}", t.as_str()),
                    ComputedValueType::Boolean,
                    RuleEntityKind::Skill,
                );
            }
        }
        self.stat(
            "fixture.effective-level",
            ComputedValueType::Integer,
            RuleEntityKind::SupportOrigin,
        );
        self.stat(
            "fixture.effective-quality",
            ComputedValueType::Quantity {
                unit: self.quality_unit.clone(),
            },
            RuleEntityKind::SupportOrigin,
        );
    }
    fn add_fixture_programs(&mut self) {
        let source: Value = read(root().join("djinn-support-preparation/source-vectors.json"));
        let type_bindings: Value = read(root().join("djinn-support-preparation/bindings.json"));
        let contexts = source["sample"]["contexts"].as_array().unwrap();
        for family in self.families.clone() {
            let root_skill: SkillDefId = decode(&family["skill"]);
            let root_grants = vec![
                decode(&family["minion"]["entering_grant"]),
                decode(&family["command"]["entering_grant"]),
            ];
            self.activation(
                subject(root_skill.clone()),
                RuleEntityKind::Skill,
                root_grants,
            );
            let actor: ActorDefId = decode(&family["minion"]["actor"]);
            self.activation(
                subject(actor),
                RuleEntityKind::Actor,
                family["actions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|a| decode(&a["entering_grant"]))
                    .collect(),
            );
            let effect_skill =
                std::iter::once((family["skill_id"].as_str().unwrap().to_owned(), root_skill))
                    .chain(
                        std::iter::once(&family["command"])
                            .chain(family["actions"].as_array().unwrap())
                            .map(|a| {
                                (
                                    a["skill_id"].as_str().unwrap().to_owned(),
                                    decode::<SkillDefId>(&a["skill"]),
                                )
                            }),
                    )
                    .collect::<Vec<_>>();
            for (source_effect, skill) in effect_skill {
                let observed = contexts
                    .iter()
                    .find(|c| {
                        c["effect"] == source_effect
                            && c["mode"] == "MAIN"
                            && c["group"]["source_present"] == false
                    })
                    .expect("actual manual Direct context");
                let initial = &observed["initial_inputs"];
                let mut values = vec![
                    (
                        "minion-present".to_owned(),
                        initial["definition"]["minion_types"]["present"]
                            .as_bool()
                            .unwrap(),
                    ),
                    ("summoner-present".to_owned(), false),
                    ("summoner-minion-present".to_owned(), false),
                    (
                        "cannot-support".to_owned(),
                        initial["flags"]["cannot_be_supported"]
                            .as_bool()
                            .unwrap_or(false),
                    ),
                    (
                        "has-gem".to_owned(),
                        initial["flags"]["gem_data_present"]
                            .as_bool()
                            .unwrap_or(false),
                    ),
                    (
                        "from-item".to_owned(),
                        initial["flags"]["source_from_item"]
                            .as_bool()
                            .unwrap_or(false),
                    ),
                    (
                        "is-player".to_owned(),
                        initial["flags"]["actor_is_enemy_player"]
                            .as_bool()
                            .unwrap_or(false),
                    ),
                ];
                for t in &self.skill_types {
                    let binding = type_bindings["types"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|b| decode::<OwnedDefinitionKey>(&b["support_type"]) == *t)
                        .unwrap();
                    for (prefix, field) in
                        [("type", "skill_types"), ("minion-type", "minion_types")]
                    {
                        let present = initial["definition"][field]["values"]
                            .as_array()
                            .is_some_and(|a| a.iter().any(|v| v["id"] == binding["source_id"]));
                        values.push((format!("{prefix}.{}", t.as_str()), present));
                    }
                    values.push((format!("summoner-type.{}", t.as_str()), false));
                    values.push((format!("summoner-minion-type.{}", t.as_str()), false));
                }
                self.owner_mut(subject(skill))
                    .programs
                    .members
                    .push(RuleProgram {
                        id: key("fixture.initial-facts"),
                        context: RuleEntityKind::Skill,
                        reads: vec![],
                        nodes: values
                            .iter()
                            .map(|(n, v)| literal(n, ParameterValue::Boolean(*v)))
                            .collect(),
                        effects: values
                            .iter()
                            .map(|(n, _)| derive(n, def(&format!("fixture.{n}"))))
                            .collect(),
                    });
            }
        }
        for support in self.preparation.supports.clone() {
            let quality_unit = self.quality_unit.clone();
            self.owner_mut(subject(support.gem))
                .programs
                .members
                .push(RuleProgram {
                    id: key("fixture.origin-facts"),
                    context: RuleEntityKind::SupportOrigin,
                    reads: vec![
                        RuleRead {
                            id: key("level"),
                            value_type: ComputedValueType::Integer,
                            source: RuleReadSource::GemLevel,
                        },
                        RuleRead {
                            id: key("quality"),
                            value_type: ComputedValueType::Quantity { unit: quality_unit },
                            source: RuleReadSource::GemQualityAmount {
                                quality: def("def.0000000000000006"),
                            },
                        },
                    ],
                    nodes: ["level", "quality"]
                        .into_iter()
                        .map(|n| RuleNode {
                            id: key(n),
                            expression: RuleExpression::Read { input: key(n) },
                        })
                        .collect(),
                    effects: vec![
                        derive("level", def("fixture.effective-level")),
                        derive("quality", def("fixture.effective-quality")),
                    ],
                });
        }
        let owners: Vec<_> = self
            .schema
            .definitions
            .iter()
            .filter_map(|d| match d {
                DefinitionDescriptor::Skill(e) if matches!(e.schema, SchemaState::Known(_)) => {
                    Some(subject(e.id.clone()))
                }
                DefinitionDescriptor::Actor(e) => Some(subject(e.id.clone())),
                DefinitionDescriptor::Class(e) => Some(subject(e.id.clone())),
                DefinitionDescriptor::Encounter(e) => Some(subject(e.id.clone())),
                _ => None,
            })
            .chain(self.schema.slots.iter().filter_map(|s| match s {
                SlotDescriptor::Actor(e) => {
                    Some(SchemaSubject::Slot(SlotAddress::Actor(e.id.clone())))
                }
                SlotDescriptor::ActionOutput(e) => {
                    Some(SchemaSubject::Slot(SlotAddress::ActionOutput(e.id.clone())))
                }
                _ => None,
            }))
            .collect();
        for owner in owners {
            self.owner_mut(owner);
        }
        // The import role retains its actual primary Skill catalogue association.
        // That identity record remains Unmapped; it is
        // not a selected Skill occurrence or authority for an empty rule owner.
        for support in self.bindings["supports"].as_array().unwrap() {
            let skill: SkillDefId = decode(&support["skill"]);
            assert!(matches!(
                self.schema
                    .definitions
                    .iter()
                    .find(|d| d.address() == skill.address()),
                Some(DefinitionDescriptor::Skill(DefinitionEntry {
                    schema: SchemaState::Unmapped { .. },
                    ..
                }))
            ));
            assert!(
                !self
                    .owners
                    .iter()
                    .any(|o| o.owner == subject(skill.clone()))
            );
        }
    }
    fn activation(
        &mut self,
        owner: SchemaSubject,
        context: RuleEntityKind,
        grants: Vec<DeclaredSlot<GrantSlotDefId>>,
    ) {
        self.owner_mut(owner).programs.members.push(RuleProgram {
            id: key("fixture.activation"),
            context,
            reads: vec![],
            nodes: vec![literal("enabled", ParameterValue::Boolean(true))],
            effects: grants
                .into_iter()
                .enumerate()
                .map(|(i, slot)| {
                    effect(
                        &format!("grant-{i}"),
                        RuleEffectKind::ActivateGrant {
                            slot,
                            enabled: key("enabled"),
                        },
                    )
                })
                .collect(),
        });
    }
    pub fn add_source(&mut self, family_index: usize, support_index: usize) {
        let index = self.sources.len();
        self.sources.push(family_index);
        let family = &self.families[family_index];
        let skill: SkillDefId = decode(&family["skill"]);
        let params = self
            .schema
            .slots
            .iter()
            .filter_map(|s| match s {
                SlotDescriptor::Parameter(DefinitionEntry {
                    id,
                    schema: SchemaState::Known(s),
                }) if id.declaration == SlotOwnerDefId::Skill(skill.clone()) => {
                    Some(ParameterAssignment {
                        slot: id.clone(),
                        value: match &s.value {
                            ValueSchema::Quantity(r) => quantity(
                                if r.minimum.unit() == &self.quality_unit {
                                    0.0
                                } else {
                                    20.0
                                },
                                r.minimum.unit(),
                            ),
                            _ => panic!("actual raw Djinn quantity"),
                        },
                    })
                }
                _ => None,
            })
            .collect();
        self.build.skills.push(SkillUse {
            id: id(20 + index as u64),
            source: AuthoredSkillSource::Direct(skill),
            parameters: Some(params),
            enabled: true,
            scope: LoadoutScope::Shared,
        });
        self.add_support(index, support_index);
    }
    pub fn add_support(&mut self, source: usize, support_index: usize) {
        let serial = self.build.gems.len() as u64;
        let gem: GemDefId = decode(&self.bindings["supports"][support_index]["gem"]);
        let parameters = self
            .schema
            .slots
            .iter()
            .filter_map(|s| match s {
                SlotDescriptor::Parameter(DefinitionEntry {
                    id,
                    schema: SchemaState::Known(s),
                }) if id.declaration == SlotOwnerDefId::Gem(gem.clone()) => {
                    Some(ParameterAssignment {
                        slot: id.clone(),
                        value: match &s.value {
                            ValueSchema::Boolean => ParameterValue::Boolean(true),
                            ValueSchema::Quantity(r) => quantity(1.0, r.minimum.unit()),
                            _ => panic!("actual saved support input"),
                        },
                    })
                }
                _ => None,
            })
            .collect();
        self.build.gems.push(GemInstance {
            id: id(100 + serial),
            definition: gem,
            parameters,
            level: 1,
            quality: Some(QualitySelection {
                kind: def("def.0000000000000006"),
                amount: FiniteQuantity::new(0.0, self.quality_unit.clone()).unwrap(),
            }),
        });
        let target = SkillTarget::Authored(id(20 + source as u64));
        let assignment = id(200 + serial);
        self.build.supports.push(SupportAssignment {
            id: assignment,
            support: id(100 + serial),
            target: target.clone(),
            enabled: true,
        });
        let origins = self.build.support_origins.as_mut().unwrap();
        if let Some(row) = origins.iter_mut().find(|r| r.target == target) {
            row.origins.push(SupportOrigin::Assignment(assignment));
        } else {
            origins.push(SupportOriginSequence {
                target,
                origins: vec![SupportOrigin::Assignment(assignment)],
            });
        }
    }
    pub fn actions(&self, source: usize) -> Vec<ActionSelection> {
        let family = &self.families[self.sources[source]];
        std::iter::once((&family["command"], true))
            .chain(
                family["actions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|a| (a, false)),
            )
            .flat_map(|(a, player)| {
                let root = ProviderRoot::SkillUse(id(20 + source as u64));
                let provider = ProviderKey {
                    root: root.clone(),
                    grant_path: if player {
                        vec![decode(&a["entering_grant"])]
                    } else {
                        vec![
                            decode(&family["minion"]["entering_grant"]),
                            decode(&a["entering_grant"]),
                        ]
                    },
                };
                let actor = if player {
                    ActorKey::Player
                } else {
                    ActorKey::Owned(Box::new(OwnedActorKey {
                        provider: ProviderKey {
                            root,
                            grant_path: vec![],
                        },
                        slot: decode(&family["minion"]["population"]),
                    }))
                };
                a["stat_sets"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(move |set| ActionSelection {
                        action: ActionKey {
                            actor: actor.clone(),
                            provider: provider.clone(),
                            output: decode(&a["output"]),
                        },
                        part: decode(&a["part"]),
                        mode: decode(&a["mode"]),
                        stat_set: decode(&set["stat_set"]),
                    })
            })
            .collect()
    }
    pub fn commandable(&self, action: &ActionSelection) -> bool {
        action.action.output.slot != def::<ActionOutputDefinition>("def.00000000000032d0")
    }
    pub fn request(&self) -> OwnedEvaluationRequest {
        let requests = (0..self.sources.len())
            .flat_map(|i| self.actions(i))
            .enumerate()
            .map(|(n, a)| MetricRequest {
                id: QueryId::new(format!("fixture-{n}")).unwrap(),
                metric: def("fixture.observe"),
                target: MetricTarget::Action(Box::new(a)),
            })
            .collect();
        OwnedEvaluationRequest::new(
            BuildSpec::new(self.build.clone(), Default::default()).unwrap(),
            ScenarioSpec::new(
                ScenarioInput {
                    game_version: ns(),
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
                    game_version: ns(),
                    requests,
                },
                Default::default(),
            )
            .unwrap(),
            Default::default(),
        )
        .unwrap()
    }
    pub fn plan(&self) -> Plan {
        self.checked_plan().unwrap()
    }
    pub fn checked_plan(&self) -> std::result::Result<Plan, String> {
        self.checked_plan_with_request(self.request(), None)
    }
    pub fn checked_plan_with_request(
        &self,
        request: OwnedEvaluationRequest,
        readiness_override: Option<ReadinessInput>,
    ) -> std::result::Result<Plan, String> {
        self.checked_plan_with_components(
            request,
            readiness_override,
            PlanComponents::default(),
            |_| {},
            |_| {},
        )
    }
    pub fn checked_plan_with_components(
        &self,
        request: OwnedEvaluationRequest,
        readiness_override: Option<ReadinessInput>,
        components: PlanComponents,
        configure_stages: impl FnOnce(&mut EvaluationStagesInput),
        configure_inputs: impl FnOnce(&mut SupportInputBindingsInput),
    ) -> std::result::Result<Plan, String> {
        let definitions = Arc::new(
            OwnedDefinitionSchemaPackage::new(self.schema.clone(), Default::default())
                .map_err(|e| e.to_string())?,
        );
        let stored = OwnedRulePackage::new(
            RulePackageInput {
                existing_actor_rules: None,
                // This finite fixture includes every owner below and has no
                // ordered reads. V21 requires an explicit empty registry even
                // when no such consumer exists; this is not build coverage.
                ordered_contributions: if self.operations.as_str() == OWNED_RULE_OPERATIONS_V21 {
                    assert!(
                        self.owners
                            .iter()
                            .flat_map(|o| &o.programs.members)
                            .flat_map(|p| &p.reads)
                            .all(|r| !matches!(
                                r.source,
                                RuleReadSource::OrderedContributions { .. }
                            ))
                    );
                    Some(DeclaredSet::complete(vec![]))
                } else {
                    None
                },
                schema_version: OWNED_RULE_PACKAGE_VERSION,
                namespace: ns(),
                release: key("fixture.rules"),
                semantics_version: self.schema.semantics_version.clone(),
                operations_version: self.operations.clone(),
                definitions: definitions.identity().clone(),
                tables: components.tables,
                owners: self.owners.clone(),
                receivers: components.receivers,
                effect_applications: Some(empty()),
            },
            definitions.as_ref(),
            Default::default(),
        )
        .map_err(|e| e.to_string())?;
        let rules = Arc::new(
            CompiledRulePackage::compile_stored(&stored, definitions.as_ref(), Default::default())
                .map_err(|e| e.to_string())?,
        );
        let routing = Arc::new(
            OwnedActionRouting::new(
                ActionRoutingInput {
                    schema_version: OWNED_ACTION_ROUTING_VERSION,
                    namespace: ns(),
                    release: key("fixture.routing"),
                    definitions: definitions.identity().clone(),
                    outputs: self
                        .schema
                        .slots
                        .iter()
                        .filter_map(|s| match s {
                            SlotDescriptor::ActionOutput(e) => Some(ActionOutputRoutes {
                                output: e.id.clone(),
                                routes: empty(),
                                source_selectors: Some(empty()),
                            }),
                            _ => None,
                        })
                        .collect(),
                },
                definitions.as_ref(),
                Default::default(),
            )
            .map_err(|e| e.to_string())?,
        );
        // A physical source may keep its real grant/projection program while
        // the caller supplies an explicit finite preparation boundary for it.
        let preparation_programs: Vec<_> = readiness_override
            .as_ref()
            .into_iter()
            .flat_map(|r| &r.programs.members)
            .filter(|r| r.phase == ReadinessPhase::Preparation)
            .map(|r| (r.owner.clone(), r.program.clone()))
            .collect();
        let stage_for = |owner: &DefinitionRules, p: &RuleProgram| {
            if p.id.as_str().starts_with("fixture.")
                || preparation_programs
                    .iter()
                    .any(|(o, program)| o == &owner.owner && program == &p.id)
            {
                "prepare"
            } else if p
                .effects
                .iter()
                .any(|e| matches!(e.effect, RuleEffectKind::SupportApplicability { .. }))
            {
                "apply"
            } else if p
                .effects
                .iter()
                .any(|e| matches!(e.effect, RuleEffectKind::Contribute { .. }))
            {
                "deliver"
            } else {
                "facts"
            }
        };
        let programs: Vec<_> = stored
            .input()
            .owners
            .iter()
            .flat_map(|o| o.programs.members.iter().map(move |p| (o, p)))
            .collect();
        let mut frozen = vec![
            FrozenStageChannel {
                channel: StageChannel::Stat {
                    scope: RuleEntityKind::SupportOrigin,
                    stat: def("fixture.effective-level"),
                },
                stage: key("prepare"),
            },
            FrozenStageChannel {
                channel: StageChannel::Stat {
                    scope: RuleEntityKind::SupportOrigin,
                    stat: def("fixture.effective-quality"),
                },
                stage: key("prepare"),
            },
        ];
        for d in &self.schema.definitions {
            if let DefinitionDescriptor::Stat(DefinitionEntry {
                id,
                schema: SchemaState::Known(s),
            }) = d
                && id.key().as_str().starts_with("fixture.")
                && s.targets == [RuleEntityKind::Skill]
            {
                frozen.push(FrozenStageChannel {
                    channel: StageChannel::Stat {
                        scope: RuleEntityKind::Skill,
                        stat: id.clone(),
                    },
                    stage: key("prepare"),
                });
            }
        }
        // A caller with a final-input projection supplies its exact readiness
        // contract; do not eagerly build the narrower historical fallback.
        let readiness = readiness_override.unwrap_or_else(|| ReadinessInput {
            skills: self
                .schema
                .definitions
                .iter()
                .filter_map(|d| match d {
                    DefinitionDescriptor::Skill(DefinitionEntry {
                        id,
                        schema: SchemaState::Known(s),
                    }) => Some(SkillReadiness {
                        participation: None,
                        skill: id.clone(),
                        parameters: DeclaredSet::complete(
                            s.declarations
                                .parameters
                                .members
                                .iter()
                                .map(|parameter| ParameterReadiness {
                                    parameter: parameter.clone(),
                                    phase: ReadinessPhase::Preparation,
                                })
                                .collect(),
                        ),
                    }),
                    _ => None,
                })
                .collect(),
            programs: DeclaredSet::complete(
                programs
                    .iter()
                    .map(|(o, p)| {
                        let early = p.id.as_str().starts_with("fixture.");
                        ReadinessProgram {
                            owner: o.owner.clone(),
                            program: p.id.clone(),
                            phase: if early {
                                ReadinessPhase::Preparation
                            } else {
                                ReadinessPhase::Execution
                            },
                            role: if early {
                                ReadinessProgramRole::PreparationFacts
                            } else {
                                ReadinessProgramRole::Execution
                            },
                            outputs: if early {
                                p.effects
                                    .iter()
                                    .map(|e| match &e.effect {
                                        RuleEffectKind::Derive { stat, .. } => StageChannel::Stat {
                                            scope: p.context,
                                            stat: stat.clone(),
                                        },
                                        RuleEffectKind::ActivateGrant { slot, .. } => {
                                            StageChannel::Grant { slot: slot.clone() }
                                        }
                                        _ => panic!("bounded fixture writes"),
                                    })
                                    .collect()
                            } else {
                                vec![]
                            },
                        }
                    })
                    .collect(),
            ),
        });
        let mut stage_input = EvaluationStagesInput {
            schema_version: OWNED_EVALUATION_STAGES_V3,
            namespace: ns(),
            release: key("fixture.stages"),
            definitions: definitions.identity().clone(),
            rules: *stored.identity(),
            routing: *routing.identity(),
            stages: [
                ("prepare", None),
                ("facts", Some("prepare")),
                ("apply", Some("facts")),
                ("deliver", Some("apply")),
            ]
            .into_iter()
            .map(|(id, prior)| EvaluationStage {
                id: key(id),
                predecessors: prior.into_iter().map(key).collect(),
            })
            .collect(),
            programs: DeclaredSet::complete(
                programs
                    .iter()
                    .map(|(o, p)| StagedRuleProgram {
                        owner: o.owner.clone(),
                        program: p.id.clone(),
                        stage: key(stage_for(o, p)),
                    })
                    .collect(),
            ),
            effect_applications: Some(empty()),
            routing_stage: key("deliver"),
            frozen_channels: frozen,
            readiness: Some(readiness),
        };
        configure_stages(&mut stage_input);
        let stages = Arc::new(
            OwnedEvaluationStages::new(
                stage_input,
                definitions.as_ref(),
                &stored,
                &routing,
                Default::default(),
            )
            .map_err(|e| e.to_string())?,
        );
        let mut prep = self.preparation.clone();
        prep.definitions = definitions.identity().clone();
        prep.rules = *stored.identity();
        let preparation = Arc::new(
            OwnedSupportPreparation::new(prep, definitions.as_ref(), &stored, Default::default())
                .map_err(|e| e.to_string())?,
        );
        let types = |prefix: &str| {
            self.skill_types
                .iter()
                .map(|t| SupportTypeStat {
                    support_type: t.clone(),
                    stat: def(&format!("fixture.{prefix}.{}", t.as_str())),
                })
                .collect()
        };
        let mut input_bindings = SupportInputBindingsInput {
            schema_version: OWNED_SUPPORT_INPUT_BINDINGS_VERSION,
            namespace: ns(),
            release: key("fixture.inputs"),
            definitions: definitions.identity().clone(),
            rules: *stored.identity(),
            preparation: *preparation.identity(),
            stages: *stages.identity(),
            preparation_stage: key("prepare"),
            effective_level: def("fixture.effective-level"),
            effective_quality: def("fixture.effective-quality"),
            target: SupportTargetInputBindings {
                skill_types: types("type"),
                minion_types: OptionalTypeInputs {
                    present: def("fixture.minion-present"),
                    members: types("minion-type"),
                },
                summoner: OptionalTypeContextInputs {
                    present: def("fixture.summoner-present"),
                    skill_types: types("summoner-type"),
                    minion_types: OptionalTypeInputs {
                        present: def("fixture.summoner-minion-present"),
                        members: types("summoner-minion-type"),
                    },
                },
                cannot_be_supported: def("fixture.cannot-support"),
                has_gem: def("fixture.has-gem"),
                from_item: def("fixture.from-item"),
                is_player_actor: def("fixture.is-player"),
            },
        };
        configure_inputs(&mut input_bindings);
        let inputs = Arc::new(
            OwnedSupportInputBindings::new(
                input_bindings,
                definitions.as_ref(),
                &stored,
                &preparation,
                &stages,
                Default::default(),
            )
            .map_err(|e| e.to_string())?,
        );
        let receiving = Arc::new(
            OwnedSupportReceiving::new(
                SupportReceivingInput {
                    schema_version: if components.source_properties.is_some() {
                        OWNED_SUPPORT_RECEIVING_V3
                    } else {
                        OWNED_SUPPORT_RECEIVING_VERSION
                    },
                    namespace: ns(),
                    release: key("fixture.receiving"),
                    definitions: definitions.identity().clone(),
                    rules: *stored.identity(),
                    preparation: *preparation.identity(),
                    inputs: *inputs.identity(),
                    stages: *stages.identity(),
                    roles: self.receiving.roles.clone(),
                    targets: self.receiving.targets.clone(),
                    supports: self.receiving.supports.clone(),
                    source_properties: components.source_properties,
                },
                definitions.as_ref(),
                &stored,
                &preparation,
                &inputs,
                &stages,
                Default::default(),
            )
            .map_err(|e| e.to_string())?,
        );
        Plan::compile(
            SupportEffectPlanInputs {
                request: Arc::new(request),
                definitions,
                rules,
                routing,
                stages,
                preparation,
                inputs,
                receiving,
            },
            Default::default(),
            Default::default(),
        )
        .map_err(|e| e.to_string())
    }
}
