//! Finite support-delivery world. Authored programs and receiving paths come
//! unchanged from a checked publication. Activation, admission facts, final
//! input assembly and contributor completeness here are test-owned isolation.
use super::bidding_fixture::{self as shared, *};
use poe_optimizer_core::{
    owned_build::*, owned_content::digest_owned, owned_definitions::*, owned_readiness::*,
    owned_rules::*, owned_schema::*, owned_stages::*, owned_support_receiving::*,
    owned_supports::*,
};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry, owned_release::StagedOwnedRelease,
    owned_release_migration::OwnedReleaseMigrationInput,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub const I: &str = "SupportMagnifiedAreaPlayer";
pub const II: &str = "SupportMagnifiedAreaPlayerTwo";
pub use shared::{decode, def, id, key, quantity, subject};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68")
}
fn packet<T: DeserializeOwned>(name: &str) -> T {
    read(root().join("magnified-area-support-delivery").join(name))
}
// Match the author's typed fragment serialization, including field order. A
// tracked observation is a fixture input only if this exact publication binds it.
#[derive(Serialize)]
struct ReceivingDigest<'a> {
    roles: &'a [SupportReceivingRole],
    targets: &'a [SupportTargetReceivingRoles],
    supports: &'a [SupportReceivingEntry],
}
fn finite<T: Serialize + DeserializeOwned>(value: &T) -> T {
    fn close(v: &mut Value) {
        match v {
            Value::Object(o) => {
                if o.contains_key("members") && o.contains_key("closure") {
                    o.insert("closure".into(), json!({"kind":"complete"}));
                }
                for v in o.values_mut() {
                    close(v);
                }
            }
            Value::Array(a) => {
                for v in a {
                    close(v);
                }
            }
            _ => {}
        }
    }
    let mut v = serde_json::to_value(value).unwrap();
    close(&mut v);
    decode(&v)
}
fn literal(name: &str, value: ParameterValue) -> RuleNode {
    RuleNode {
        id: key(name),
        expression: RuleExpression::Literal { value },
    }
}
fn derive(name: &str, stat: StatDefId) -> RuleEffect {
    RuleEffect {
        id: key(name),
        when: None,
        effect: RuleEffectKind::Derive {
            entity: RuleEntity::Current,
            stat,
            value: key(name),
        },
    }
}

#[derive(Clone)]
pub struct World {
    pub inner: shared::World,
    pub ice: Value,
    pub bindings: Value,
    pub originals: Vec<DefinitionRules>,
    pub original_receiving: Receiving,
    pub ice_source: usize,
    pub source_matrix: Value,
    pub area_usage: Vec<UsagePolicySelection>,
}
impl World {
    /// Install exact published support bodies in this explicitly finite world.
    /// The caller authenticates the family's publication/provenance; this join
    /// independently checks every copied descriptor and owner against that same
    /// endpoint. Existing receiving roles are retained, never silently replaced.
    #[allow(dead_code)] // Base-only fixture targets install no additional family.
    pub fn add_support_fragment(
        &mut self,
        endpoint: &StagedOwnedRelease,
        bindings: &Value,
        owners: &[DefinitionRules],
        preparation: &SupportPreparationInput,
        receiving: &Receiving,
    ) {
        let origin = self
            .inner
            .owners
            .iter()
            .flat_map(|o| &o.programs.members)
            .find(|p| p.id == key("fixture.origin-facts"))
            .unwrap()
            .clone();
        let rows = bindings["supports"].as_array().unwrap();
        assert_eq!(rows.len(), owners.len());
        assert_eq!(rows.len(), preparation.supports.len());
        assert_eq!(rows.len(), receiving.supports.len());
        for row in rows {
            let gem: GemDefId = decode(&row["gem"]);
            let skill: SkillDefId = decode(&row["skill"]);
            let association = endpoint
                .input()
                .roles
                .roles
                .iter()
                .find(|r| r.gem == gem)
                .unwrap();
            assert_eq!(
                association.primary,
                poe_optimizer_import::owned_skill_catalog::OwnedPrimarySkill::Known(skill.clone())
            );
            for address in [gem.address(), skill.address()] {
                let actual = endpoint
                    .input()
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .find(|d| d.address() == address)
                    .unwrap();
                if let DefinitionDescriptor::Gem(DefinitionEntry {
                    schema: SchemaState::Known(s),
                    ..
                }) = actual
                {
                    assert!(s.skills.members.is_empty());
                    assert!(!s.skills.is_complete());
                }
                assert!(
                    !self
                        .inner
                        .schema
                        .definitions
                        .iter()
                        .any(|d| d.address() == address)
                );
                self.inner.schema.definitions.push(finite(actual));
            }
            for slot in &endpoint.input().recipe.schema.slots {
                let value = serde_json::to_value(slot).unwrap();
                let declaration: SlotOwnerDefId = decode(&value["value"]["id"]["declaration"]);
                if declaration == SlotOwnerDefId::Gem(gem.clone()) {
                    assert!(!self.inner.schema.slots.iter().any(|s| s == slot));
                    self.inner.schema.slots.push(finite(slot));
                }
            }
            let owner = owners
                .iter()
                .find(|o| o.owner == subject(gem.clone()))
                .unwrap();
            assert_eq!(
                endpoint
                    .input()
                    .recipe
                    .rules
                    .owners
                    .iter()
                    .filter(|o| *o == owner)
                    .count(),
                1
            );
            assert!(!owner.programs.is_complete());
            assert!(!self.inner.owners.iter().any(|o| o.owner == owner.owner));
            let mut component: DefinitionRules = finite(owner);
            component.programs.members.push(origin.clone());
            self.inner.owners.push(component);
            assert!(!self.inner.preparation.supports.iter().any(|s| s.gem == gem));
            self.inner.preparation.supports.push(
                preparation
                    .supports
                    .iter()
                    .find(|s| s.gem == gem)
                    .unwrap()
                    .clone(),
            );
            assert!(!self.inner.receiving.supports.iter().any(|s| s.gem == gem));
            self.inner.receiving.supports.push(finite(
                receiving.supports.iter().find(|s| s.gem == gem).unwrap(),
            ));
            let current = self.inner.bindings["supports"].as_array_mut().unwrap();
            assert!(!current.iter().any(|s| s["gem"] == row["gem"]));
            current.push(row.clone());
        }
        assert_eq!(preparation.policy, self.inner.preparation.policy);
        assert_eq!(preparation.quality_unit, self.inner.quality_unit);
        for ty in &preparation.types {
            assert!(
                self.inner.skill_types.contains(ty),
                "finite vocabulary must declare {ty:?}"
            );
        }
        for effect in &preparation.effects {
            // A vocabulary symbol can precede any concrete support occurrence.
            // Duplicate Gem owners/entries are rejected independently above.
            if !self.inner.preparation.effects.contains(effect) {
                self.inner.preparation.effects.push(effect.clone());
            }
        }
        for family in &preparation.families {
            if !self.inner.preparation.families.contains(family) {
                self.inner.preparation.families.push(family.clone());
            }
        }
        for role in &receiving.roles {
            assert!(!self.inner.receiving.roles.iter().any(|r| r.id == role.id));
            self.inner.receiving.roles.push(role.clone());
        }
        for target in &receiving.targets {
            let target: SupportTargetReceivingRoles = finite(target);
            if let Some(current) = self
                .inner
                .receiving
                .targets
                .iter_mut()
                .find(|t| t.owner == target.owner)
            {
                assert!(current.roles.is_complete());
                for role in target.roles.members {
                    assert!(!current.roles.members.iter().any(|r| r.role == role.role));
                    current.roles.members.push(role);
                }
            } else {
                self.inner.receiving.targets.push(target);
            }
        }
    }
    pub fn load() -> Self {
        static WORLD: OnceLock<World> = OnceLock::new();
        WORLD.get_or_init(Self::load_once).clone()
    }
    fn load_once() -> Self {
        let path = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_MAGNIFIED_RELEASE")
                .expect("exact checked Magnified publication"),
        );
        Self::load_release(&path, true)
    }
    pub fn load_release(path: &Path, historical_endpoint: bool) -> Self {
        let before = crate::release::inventory(path);
        let release = crate::release::load(path);
        let migration: OwnedReleaseMigrationInput = packet("migration.json");
        let bindings: Value = packet("bindings.json");
        let receiving: Receiving = packet("receiving.json");
        let vectors: Value = packet("source-vectors.json");
        let authoring: Value = packet("authoring.json");
        let dependencies: Value = packet("dependencies.json");
        let provenance: Vec<_> = release
            .input()
            .provenance
            .iter()
            .filter(|p| p.kind == key("magnified-area-support-delivery-v1"))
            .collect();
        assert_eq!(provenance.len(), 1);
        let provenance = provenance[0];
        if historical_endpoint {
            assert_eq!(release.input().recipe.schema.release, migration.release);
            assert_eq!(release.input().provenance.last(), Some(provenance));
        }
        assert_eq!(provenance.prior_input, decode(&authoring["before"]));
        assert_eq!(
            provenance.authoring_input,
            digest_owned(
                "owned-magnified-area-support-delivery-v1",
                &(
                    &authoring,
                    &bindings,
                    &dependencies,
                    &vectors,
                    &migration,
                    ReceivingDigest {
                        roles: &receiving.roles,
                        targets: &receiving.targets,
                        supports: &receiving.supports,
                    }
                ),
                8 * 1024 * 1024,
            )
            .unwrap()
        );
        for entry in &migration.schema {
            let SchemaExtensionEntry::Definition(d) = entry else {
                panic!("Magnified stat descriptors only")
            };
            assert_eq!(
                release
                    .input()
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .filter(|a| *a == d)
                    .count(),
                1
            );
        }
        for owner in &migration.owners {
            assert_eq!(
                release
                    .input()
                    .recipe
                    .rules
                    .owners
                    .iter()
                    .filter(|a| *a == owner)
                    .count(),
                1
            );
            assert!(!owner.programs.is_complete());
        }
        // This publication has authenticated Magnified provenance after the
        // inherited Gem correction, rather than ending at that historical stage.
        let mut inner = shared::World::load_release(path, false);
        let origin = inner
            .owners
            .iter()
            .flat_map(|o| &o.programs.members)
            .find(|p| p.id == key("fixture.origin-facts"))
            .unwrap()
            .clone();
        let ice: Value = read(root().join("ice-nova-source-inputs/bindings.json"));
        let gems: Vec<GemDefId> = bindings["supports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| decode(&v["gem"]))
            .collect();
        let ice_gem: GemDefId = decode(&ice["physical_gem"]);
        let ice_skill: SkillDefId = decode(&ice["primary_skill"]);
        let mut wanted = BTreeSet::from([ice_gem.address(), ice_skill.address()]);
        for g in &gems {
            wanted.insert(g.address());
        }
        for s in bindings["supports"].as_array().unwrap() {
            wanted.insert(decode::<SkillDefId>(&s["skill"]).address());
        }
        wanted.insert(decode::<ActionPartDefId>(&ice["part"]).address());
        wanted.insert(decode::<ActionModeDefId>(&ice["mode"]).address());
        for s in ice["stat_sets"].as_array().unwrap() {
            wanted.insert(decode::<ActionStatSetDefId>(&s["stat_set"]).address());
        }
        // Stat declarations are inert schema, not rule owners or assumed values.
        for d in &release.input().recipe.schema.definitions {
            if (wanted.contains(&d.address()) || matches!(d, DefinitionDescriptor::Stat(_)))
                && !inner
                    .schema
                    .definitions
                    .iter()
                    .any(|p| p.address() == d.address())
            {
                inner.schema.definitions.push(finite(d));
            }
        }
        let slot_owners = [
            SlotOwnerDefId::Gem(ice_gem.clone()),
            SlotOwnerDefId::Skill(ice_skill.clone()),
        ]
        .into_iter()
        .chain(gems.iter().cloned().map(SlotOwnerDefId::Gem))
        .collect::<Vec<_>>();
        for slot in &release.input().recipe.schema.slots {
            let v = serde_json::to_value(slot).unwrap();
            let owner: SlotOwnerDefId = decode(&v["value"]["id"]["declaration"]);
            if slot_owners.contains(&owner) && !inner.schema.slots.iter().any(|s| s == slot) {
                inner.schema.slots.push(finite(slot));
            }
        }
        inner.schema.release = key("fixture.magnified-delivery");
        inner.build.gems.clear();
        inner.build.supports.clear();
        inner.build.skills.clear();
        inner.build.support_origins = Some(vec![]);
        inner.sources.clear();
        // Keep shared explicit Djinn activation/admission facts, replacing only
        // support-owner selection with this publication's complete real bodies.
        inner.owners.retain(|o| {
            !matches!(
                &o.owner,
                SchemaSubject::Definition(DefinitionAddress::Gem(_))
            )
        });
        for owner in &migration.owners {
            let mut owner: DefinitionRules = finite(owner);
            owner.programs.members.push(origin.clone());
            inner.owners.push(owner);
        }
        let mut prep: SupportPreparationInput =
            read(root().join("djinn-support-preparation/preparation.json"));
        prep.supports.retain(|s| gems.contains(&s.gem));
        assert_eq!(prep.supports.len(), 2);
        inner.preparation = prep;
        inner.bindings = bindings.clone();
        inner.channels = bindings["channels"].clone();
        inner.receiving = Receiving {
            roles: receiving.roles.clone(),
            targets: finite(&receiving.targets),
            supports: finite(&receiving.supports),
        };
        inner.add_source(0, 0);
        inner.add_source(1, 0);
        let mut world = Self {
            inner,
            ice,
            bindings,
            originals: migration.owners,
            original_receiving: receiving,
            ice_source: 2,
            source_matrix: vectors["action_matrix"].clone(),
            area_usage: vec![],
        };
        world.add_ice(0);
        world.set_area_fact(Some(true));
        assert_eq!(before, crate::release::inventory(path));
        world
    }
    fn add_ice(&mut self, support: usize) {
        let gem: GemDefId = decode(&self.ice["physical_gem"]);
        let skill: SkillDefId = decode(&self.ice["primary_skill"]);
        let grants: DeclaredSlot<GrantSlotDefId> = decode(&self.ice["entering_grant"]);
        let projection = RuleProgram {
            id: key("fixture.ice-final-input"),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![literal(
                "level",
                ParameterValue::Integer(BoundedInteger::new(20).unwrap()),
            )],
            effects: vec![RuleEffect {
                id: key("project"),
                when: None,
                effect: RuleEffectKind::ProjectSkillParameter {
                    skill: decode(&self.ice["primary_supply"]),
                    parameter: decode(&self.ice["final_level"]),
                    value: key("level"),
                },
            }],
        };
        let activation = RuleProgram {
            id: key("fixture.ice-activation"),
            context: RuleEntityKind::Skill,
            reads: vec![],
            nodes: vec![literal("active", ParameterValue::Boolean(true))],
            effects: vec![RuleEffect {
                id: key("activate"),
                when: None,
                effect: RuleEffectKind::ActivateGrant {
                    slot: grants.clone(),
                    enabled: key("active"),
                },
            }],
        };
        self.inner
            .owner_mut(subject(gem.clone()))
            .programs
            .members
            .extend([activation, projection]);
        // This isolated Area support predicate receives a finite known input
        // context. It does not certify the complete game's Ice skill-type set.
        let mut values = vec![
            ("minion-present".to_owned(), false),
            ("summoner-present".to_owned(), false),
            ("summoner-minion-present".to_owned(), false),
            ("cannot-support".to_owned(), false),
            ("has-gem".to_owned(), true),
            ("from-item".to_owned(), false),
            ("is-player".to_owned(), false),
        ];
        for t in &self.inner.skill_types {
            for prefix in [
                "type",
                "minion-type",
                "summoner-type",
                "summoner-minion-type",
            ] {
                values.push((
                    format!("{prefix}.{}", t.as_str()),
                    prefix == "type" && t.as_str() == "type.area",
                ));
            }
        }
        let initial_facts = RuleProgram {
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
        };
        // The assigned physical Gem and its generated receiving Skill are
        // distinct targets. Supply the finite predicate inputs on each exact
        // owner; a root input never implicitly becomes a child Skill input.
        for owner in [subject(gem.clone()), subject(skill)] {
            self.inner
                .owner_mut(owner)
                .programs
                .members
                .push(initial_facts.clone());
        }
        for slot in [
            SchemaSubject::Slot(SlotAddress::Grant(grants)),
            SchemaSubject::Slot(SlotAddress::SkillGrant(decode(&self.ice["primary_supply"]))),
            SchemaSubject::Slot(SlotAddress::ActionOutput(decode(&self.ice["output"]))),
        ] {
            self.inner.owner_mut(slot);
        }
        let parameters = self
            .inner
            .schema
            .slots
            .iter()
            .filter_map(|s| match s {
                SlotDescriptor::Parameter(DefinitionEntry {
                    id,
                    schema: SchemaState::Known(p),
                }) if id.declaration == SlotOwnerDefId::Gem(gem.clone()) => {
                    Some(ParameterAssignment {
                        slot: id.clone(),
                        value: match &p.value {
                            ValueSchema::Boolean => ParameterValue::Boolean(false),
                            ValueSchema::Quantity(q) => quantity(0.0, q.minimum.unit()),
                            _ => panic!("actual Ice raw input"),
                        },
                    })
                }
                _ => None,
            })
            .collect();
        self.inner.build.gems.push(GemInstance {
            id: id(900),
            definition: gem,
            parameters,
            level: 20,
            quality: Some(QualitySelection {
                kind: def("def.0000000000000006"),
                amount: FiniteQuantity::new(0.0, self.inner.quality_unit.clone()).unwrap(),
            }),
        });
        self.inner.build.skills.push(SkillUse {
            id: id(20 + self.ice_source as u64),
            source: AuthoredSkillSource::Gem(id(900)),
            parameters: None,
            enabled: true,
            scope: LoadoutScope::Shared,
        });
        self.inner.add_support(self.ice_source, support);
    }
    pub fn actions(&self, source: usize) -> Vec<ActionSelection> {
        if source != self.ice_source {
            return self.inner.actions(source);
        }
        self.ice["stat_sets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| ActionSelection {
                action: ActionKey {
                    actor: ActorKey::Player,
                    provider: ProviderKey {
                        root: ProviderRoot::SkillUse(id(20 + source as u64)),
                        grant_path: vec![decode(&self.ice["entering_grant"])],
                    },
                    output: decode(&self.ice["output"]),
                },
                part: decode(&self.ice["part"]),
                mode: decode(&self.ice["mode"]),
                stat_set: decode(&s["stat_set"]),
            })
            .collect()
    }
    pub fn set_area_fact(&mut self, value: Option<bool>) {
        self.area_usage.clear();
        let channel: StatDefId = decode(&self.bindings["channels"]["area_eligible"]);
        let outputs: BTreeSet<_> = (0..=self.ice_source)
            .flat_map(|s| self.actions(s))
            .map(|a| a.action.output)
            .collect();
        for output in outputs {
            let owner = self
                .inner
                .owner_mut(SchemaSubject::Slot(SlotAddress::ActionOutput(output)));
            owner
                .programs
                .members
                .retain(|p| p.id != key("finite-area-eligibility"));
            if let Some(value) = value {
                // Explicit finite caller input; the packet intentionally has no
                // production Area classifier. Both truth values are exercised.
                owner.programs.members.push(RuleProgram {
                    id: key("finite-area-eligibility"),
                    context: RuleEntityKind::Action,
                    reads: vec![],
                    nodes: vec![literal("area", ParameterValue::Boolean(value))],
                    effects: vec![derive("area", channel.clone())],
                });
            }
        }
    }
    pub fn source_fact(&self, action: &ActionSelection) -> &Value {
        let endpoints: Vec<_> = self.bindings["targets"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|t| t["endpoints"].as_array().unwrap())
            .filter(|e| {
                decode::<DeclaredSlot<ActionOutputDefId>>(&e["output"]) == action.action.output
                    && decode::<Vec<DeclaredSlot<GrantSlotDefId>>>(&e["endpoint"]["path"])
                        == action.action.provider.grant_path
            })
            .collect();
        assert_eq!(endpoints.len(), 1, "exact published receiving endpoint");
        let endpoint = endpoints[0];
        let sets: Vec<_> = endpoint["stat_sets"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| decode::<ActionStatSetDefId>(&s["stat_set"]) == action.stat_set)
            .collect();
        assert_eq!(sets.len(), 1);
        let matrix = self
            .source_matrix
            .as_array()
            .expect("authenticated complete source action matrix");
        assert_eq!(matrix.len(), 14);
        let facts: Vec<_> = matrix
            .iter()
            .filter(|r| {
                r["source_effect"] == endpoint["source_effect"]
                    && r["source_stat_set_index"] == sets[0]["source_index"]
            })
            .collect();
        assert_eq!(facts.len(), 1, "one observed exact endpoint/stat-set");
        facts[0]
    }
    pub fn admitted(&self, action: &ActionSelection, tier: &str) -> bool {
        self.source_fact(action)["admitted"][tier]
            .as_bool()
            .unwrap()
    }
    pub fn set_source_area_matrix(&mut self) {
        self.set_area_fact(None);
        let policy: UsagePolicyDefId = def("fixture.observed-area-input");
        let parameter = DeclaredSlot {
            declaration: SlotOwnerDefId::UsagePolicy(policy.clone()),
            slot: def("fixture.observed-area-value"),
        };
        self.inner
            .schema
            .definitions
            .push(DefinitionDescriptor::UsagePolicy(DefinitionEntry {
                id: policy.clone(),
                schema: SchemaState::Known(UsagePolicySchema {
                    targets: vec![UsageTargetKind::Action],
                    declarations: DeclaredSlots {
                        parameters: DeclaredSet::complete(vec![parameter.clone()]),
                        choices: DeclaredSet::complete(vec![]),
                        grants: DeclaredSet::complete(vec![]),
                        actors: DeclaredSet::complete(vec![]),
                        skill_grants: DeclaredSet::complete(vec![]),
                        outputs: DeclaredSet::complete(vec![]),
                        sockets: DeclaredSet::complete(vec![]),
                    },
                }),
            }));
        self.inner
            .schema
            .slots
            .push(SlotDescriptor::Parameter(DefinitionEntry {
                id: parameter.clone(),
                schema: SchemaState::Known(ParameterSlotSchema {
                    value: ValueSchema::Boolean,
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::UsagePolicyParameter],
                    skill_input: None,
                }),
            }));
        let stat: StatDefId = decode(&self.bindings["channels"]["area_eligible"]);
        self.inner
            .owner_mut(subject(policy.clone()))
            .programs
            .members
            .push(RuleProgram {
                id: key("finite-observed-area"),
                context: RuleEntityKind::Action,
                reads: vec![RuleRead {
                    id: key("observed-area"),
                    value_type: ComputedValueType::Boolean,
                    source: RuleReadSource::Parameter {
                        slot: parameter.clone(),
                    },
                }],
                nodes: vec![RuleNode {
                    id: key("area"),
                    expression: RuleExpression::Read {
                        input: key("observed-area"),
                    },
                }],
                effects: vec![derive("area", stat)],
            });
        self.area_usage = (0..=self.ice_source)
            .flat_map(|s| self.actions(s))
            .map(|action| {
                let flag = self.source_fact(&action)["area_eligible"]
                    .as_bool()
                    .unwrap();
                UsagePolicySelection {
                    policy: policy.clone(),
                    target: UsageTarget::Action(Box::new(action)),
                    parameters: vec![ParameterAssignment {
                        slot: parameter.clone(),
                        value: ParameterValue::Boolean(flag),
                    }],
                }
            })
            .collect();
    }
    pub fn request(&self) -> OwnedEvaluationRequest {
        OwnedEvaluationRequest::new(
            BuildSpec::new(self.inner.build.clone(), Default::default()).unwrap(),
            ScenarioSpec::new(
                ScenarioInput {
                    game_version: ns(),
                    enemy: EnemySpec {
                        encounter: def("fixture.encounter"),
                        level: 20,
                    },
                    assumptions: vec![],
                    usage: self.area_usage.clone(),
                },
                Default::default(),
            )
            .unwrap(),
            QuerySpec::new(
                QueryInput {
                    game_version: ns(),
                    requests: (0..=self.ice_source)
                        .flat_map(|i| self.actions(i))
                        .enumerate()
                        .map(|(i, a)| MetricRequest {
                            id: QueryId::new(format!("magnified-{i}")).unwrap(),
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
    pub fn plan(&self) -> Plan {
        self.checked_plan().unwrap()
    }
    pub fn checked_plan(&self) -> std::result::Result<Plan, String> {
        self.checked_plan_with_request(self.request())
    }
    pub fn checked_plan_with_request(
        &self,
        request: OwnedEvaluationRequest,
    ) -> std::result::Result<Plan, String> {
        let final_level: DeclaredSlot<ParameterSlotDefId> = decode(&self.ice["final_level"]);
        let readiness = ReadinessInput {
            skills: self
                .inner
                .schema
                .definitions
                .iter()
                .filter_map(|d| match d {
                    DefinitionDescriptor::Skill(DefinitionEntry {
                        id,
                        schema: SchemaState::Known(s),
                    }) => Some(GeneratedSkillReadiness {
                        skill: id.clone(),
                        parameters: DeclaredSet::complete(
                            s.declarations
                                .parameters
                                .members
                                .iter()
                                .map(|p| ParameterReadiness {
                                    parameter: p.clone(),
                                    phase: if *p == final_level {
                                        ReadinessPhase::Execution
                                    } else {
                                        ReadinessPhase::Preparation
                                    },
                                })
                                .collect(),
                        ),
                    }),
                    _ => None,
                })
                .collect(),
            programs: DeclaredSet::complete(
                self.inner
                    .owners
                    .iter()
                    .flat_map(|o| {
                        o.programs.members.iter().map(|p| {
                            let early = p.id.as_str().starts_with("fixture.");
                            let projection = p.id == key("fixture.ice-final-input");
                            ReadinessProgram {
                                owner: o.owner.clone(),
                                program: p.id.clone(),
                                phase: if early {
                                    ReadinessPhase::Preparation
                                } else {
                                    ReadinessPhase::Execution
                                },
                                role: if projection {
                                    ReadinessProgramRole::FinalInputAssembly
                                } else if early {
                                    ReadinessProgramRole::PreparationFacts
                                } else {
                                    ReadinessProgramRole::Execution
                                },
                                outputs: if early {
                                    p.effects
                                        .iter()
                                        .map(|e| match &e.effect {
                                            RuleEffectKind::Derive { stat, .. } => {
                                                StageChannel::Stat {
                                                    scope: p.context,
                                                    stat: stat.clone(),
                                                }
                                            }
                                            RuleEffectKind::ActivateGrant { slot, .. } => {
                                                StageChannel::Grant { slot: slot.clone() }
                                            }
                                            RuleEffectKind::ProjectSkillParameter {
                                                parameter,
                                                ..
                                            } => StageChannel::SkillParameter {
                                                parameter: parameter.clone(),
                                            },
                                            _ => panic!("finite preparation"),
                                        })
                                        .collect()
                                } else {
                                    vec![]
                                },
                            }
                        })
                    })
                    .collect(),
            ),
        };
        self.inner
            .checked_plan_with_request(request, Some(readiness))
    }
}
