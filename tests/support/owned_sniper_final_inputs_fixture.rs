//! Actual item -> prepared physical Gem -> source assembly -> population graph.
//! The two-occurrence inventory is a finite test fixture. It does not complete
//! real support-origin discovery, Actor mechanics or incoming contributors.
#[allow(dead_code)]
#[path = "owned_offering_final_inputs_fixture.rs"]
pub mod offering;
pub use offering::{decode, def, id, key, quantity, shared, subject};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_rules::*, owned_schema::*,
    owned_source_properties::*, owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::OnceLock};

pub const ASSEMBLY: &str = "sniper-final-inputs";
pub const SNAPSHOT: &str = "pre-amulet-bonus-snapshot";
pub const RETENTION: &str = "ordinary-amulet-retention-factor";
pub const APPLICABILITY: &str = "ordinary-item-direct-applicability";
pub fn stat(n: u64) -> StatDefId {
    def(&format!("def.{n:016x}"))
}
pub fn gem() -> GemDefId {
    def("def.0000000000000011")
}
pub fn skill() -> SkillDefId {
    def("def.0000000000000012")
}

#[derive(Clone)]
pub struct World {
    pub base: offering::World,
    pub bindings: Value,
    pub readiness: crate::sniper_family::ReadinessFragment,
    pub receivers: DeclaredSet<StatReceiver>,
    pub early: Vec<(SchemaSubject, OwnedDefinitionKey)>,
    pub actual_gem: DefinitionRules,
}
impl World {
    pub fn load() -> Self {
        static WORLD: OnceLock<World> = OnceLock::new();
        WORLD.get_or_init(Self::load_once).clone()
    }
    fn load_once() -> Self {
        let path = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_SNIPER_INPUTS_RELEASE")
                .expect("published Sniper final inputs"),
        );
        Self::load_release(&path)
    }
    /// The calling target authenticates its own published endpoint. Stored
    /// Sniper/Offering programs remain exact; finite coverage is test-only.
    pub fn load_release(path: &std::path::Path) -> Self {
        let before = crate::release::inventory(path);
        let endpoint = crate::release::load(path);
        crate::sniper_family::assert_component(&endpoint);
        let modifier = endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == subject(def::<ModifierDefinition>("def.00000000000030ca")))
            .unwrap();
        let mut base = offering::World::load_release_with_item_owner(path, Some(modifier));
        let (receivers, early) = install_routing(&mut base, &endpoint);
        let bindings: Value = crate::sniper_family::read("bindings.json");
        let relation: SourcePropertyPreparationInput =
            crate::sniper_family::read("source-properties.json");
        base.relation
            .relations
            .members
            .extend(relation.relations.members);
        let readiness = crate::sniper_family::read("readiness.json");
        let (actual_gem, _) = install_sniper(&mut base, &endpoint, &bindings);
        base.source.base.inner.build.character.level = 100;
        assert_eq!(before, crate::release::inventory(path));
        Self {
            base,
            bindings,
            readiness,
            receivers,
            early,
            actual_gem,
        }
    }
    pub fn raw(&mut self, index: usize, level: u16, quality: f64, corruption: f64) {
        let gem = self
            .base
            .source
            .base
            .inner
            .build
            .gems
            .iter_mut()
            .find(|g| g.id == id(7100 + index as u64))
            .unwrap();
        gem.level = level;
        gem.quality.as_mut().unwrap().amount =
            FiniteQuantity::new(quality, def("def.0000000000000002")).unwrap();
        gem.parameters
            .iter_mut()
            .find(|p| p.slot.slot == def("def.00000000000030aa"))
            .unwrap()
            .value = quantity(corruption, &def("def.000000000000295a"));
    }
    pub fn checked_plan(&self) -> std::result::Result<shared::Plan, String> {
        self.checked_plan_configured(|_| {})
    }
    /// Append only test-component stages after retaining the exact preparation,
    /// census, assembly and population configuration above the consumer.
    pub fn checked_plan_configured(
        &self,
        configure: impl FnOnce(&mut EvaluationStagesInput),
    ) -> std::result::Result<shared::Plan, String> {
        self.checked_plan_with_request(self.base.source.request(), configure)
    }
    /// The caller composes usage through Core while this retains real input assembly.
    pub fn checked_plan_with_request(
        &self,
        request: OwnedEvaluationRequest,
        configure: impl FnOnce(&mut EvaluationStagesInput),
    ) -> std::result::Result<shared::Plan, String> {
        self.base.checked_plan_with_request(request, OWNED_EVALUATION_STAGES_V4, false, self.receivers.clone(), |stages| {
            let names = ["prepare", "routing-contributors", "routing-resolve", "routing-applicability", "item-delivery",
                "source-prepare", "source-census", "source-assembly", "population", "facts", "apply", "deliver"];
            stages.stages = names.iter().enumerate().map(|(i,name)| EvaluationStage {
                id:key(name), predecessors:i.checked_sub(1).map(|j|vec![key(names[j])]).unwrap_or_default(),
            }).collect();
            for row in &mut stages.programs.members {
                row.stage = match row.program.as_str() {
                    SNAPSHOT | RETENTION => key("routing-resolve"),
                    APPLICABILITY => key("routing-applicability"),
                    "contribute-player-minion-gem-level" | "amulet-copy-minion-gem-level" => key("item-delivery"),
                    "ordinary-population-inputs" => key("population"),
                    _ => row.stage.clone(),
                };
                if let Some(p) = self.readiness.programs.iter().find(|p| p.owner == row.owner && p.program == row.program) {
                    row.stage = p.stage.clone();
                }
                if row.program == key("fixture.initial-facts")
                    && (row.owner == subject(gem()) || row.owner == subject(skill())) {
                    row.stage = key("source-prepare");
                }
            }
            let ready = stages.readiness.as_mut().unwrap();
            for authored in &self.readiness.readiness.skills {
                *ready.skills.iter_mut().find(|s| s.skill == authored.skill).unwrap() = authored.clone();
            }
            for row in &mut ready.programs.members {
                if self.early.contains(&(row.owner.clone(),row.program.clone())) {
                    let program = self.base.source.base.inner.owners.iter().find(|o| o.owner==row.owner).unwrap()
                        .programs.members.iter().find(|p| p.id==row.program).unwrap();
                    row.phase=ReadinessPhase::Structural;
                    row.role=ReadinessProgramRole::PreparationFacts;
                    row.outputs=program.effects.iter().map(|e|offering::channel(program.context,&e.effect)).collect();
                }
                if let Some(authored)=self.readiness.readiness.programs.members.iter()
                    .find(|p|p.owner==row.owner && p.program==row.program) { *row=authored.clone(); }
                // These facts consume final parameters classified Execution.
                // They run after the actual source assembly, without claiming
                // early minion support admission or changing parameter gates.
            }
            for row in &mut stages.frozen_channels {
                if matches!(&row.channel,StageChannel::Contributions{stat:s,..} if *s==stat(0x30ab)) {
                    row.stage=key("item-delivery");
                }
            }
            for (s,kind) in [(stat(0x3305),ContributionKind::Multiply),(stat(0x32e4),ContributionKind::Add)] {
                stages.frozen_channels.extend([
                    FrozenStageChannel{channel:StageChannel::Contributions{scope:RuleEntityKind::Actor,stat:s.clone(),contribution:kind},stage:key("routing-contributors")},
                    FrozenStageChannel{channel:StageChannel::Stat{scope:RuleEntityKind::Actor,stat:s},stage:key("routing-resolve")},
                ]);
            }
            stages.frozen_channels.push(FrozenStageChannel{channel:StageChannel::Stat{
                scope:RuleEntityKind::EquipmentUse,stat:stat(0x3306)},stage:key("routing-applicability")});
            // Shared pre-support channels were already frozen by the Offering
            // relation. The actual Sniper writer uses that same early contract.
            configure(stages);
        })
    }
    pub fn plan(&self) -> shared::Plan {
        self.checked_plan().unwrap()
    }
    pub fn evaluate(&self) -> SupportEffectsReport {
        let p = self.plan();
        p.evaluate(&mut p.new_scratch()).unwrap()
    }
    pub fn generated(&self, index: usize) -> GeneratedSkillKey {
        GeneratedSkillKey {
            provider: ProviderKey {
                root: ProviderRoot::SkillUse(id(7200 + index as u64)),
                grant_path: vec![],
            },
            slot: decode(&self.bindings["target"]["primary_supply"]),
        }
    }
    pub fn actor(&self, index: usize) -> ActorKey {
        ActorKey::Owned(Box::new(OwnedActorKey {
            provider: ProviderKey {
                root: ProviderRoot::SkillUse(id(7200 + index as u64)),
                grant_path: vec![decode(&self.bindings["target"]["entering_grant"])],
            },
            slot: decode(&self.bindings["target"]["actor"]),
        }))
    }
    pub fn check(&self, report: &SupportEffectsReport, levels: [i64; 2], qualities: [f64; 2]) {
        let effects = offering::effects(report);
        for index in 0..2 {
            for (field, value) in [
                (
                    "final_level_parameter",
                    ParameterValue::Integer(BoundedInteger::new(levels[index]).unwrap()),
                ),
                (
                    "final_quality_parameter",
                    quantity(qualities[index], &def("def.0000000000000002")),
                ),
            ] {
                let wanted = PlanValueKey::SkillParameter {
                    skill: Box::new(self.generated(index)),
                    parameter: decode(&self.bindings["target"][field]),
                };
                let rows: Vec<_> = effects
                    .effects
                    .iter()
                    .filter(|e| matches!(&e.target,BoundEffectTarget::Value{key}if *key==wanted))
                    .collect();
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].key.invocation.program, key(ASSEMBLY));
                assert_eq!(rows[0].value, EffectValue::Known { value });
                assert!(
                    matches!(&rows[0].key.invocation.origin,RuleOrigin::SourceProperty{relation,owner,producer,position}
                    if *relation==key("sniper-source-inputs") && **owner==SkillTarget::Authored(id(7200+index as u64))
                    && producer.root==ProviderRoot::SkillUse(id(7200+index as u64)) && producer.grant_path.is_empty() && position.is_none())
                );
            }
            let table = self
                .base
                .tables
                .iter()
                .find(|t| t.id == key("sniper.actor-level"))
                .unwrap();
            let wanted = PlanValueKey::Stat {
                entity: ConcreteEntity::Actor(self.actor(index)),
                stat: stat(0x1c),
            };
            let row = effects
                .values
                .iter()
                .find(|v| v.key == wanted)
                .expect("actual population projection");
            assert_eq!(
                row.value,
                EffectValue::Known {
                    value: table.rows[(levels[index] - 1) as usize].clone()
                }
            );
            let census:Vec<_>=effects.effects.iter().filter(|e|matches!(&e.key.invocation.origin,
                RuleOrigin::SourcePropertyCensus{relation,owner} if *relation==key("sniper-source-inputs") && **owner==SkillTarget::Authored(id(7200+index as u64)))).collect();
            assert_eq!(census.len(), 1);
            assert_eq!(
                census[0].value,
                EffectValue::Known {
                    value: ParameterValue::Integer(BoundedInteger::new(0).unwrap())
                }
            );
        }
        assert!(
            effects
                .effects
                .iter()
                .all(
                    |e| e.key.invocation.program.as_str() != "fixture-explicit-final-level"
                        && e.key.invocation.program.as_str() != offering::SNAPSHOT
                )
        );
        assert!(
            effects
                .effects
                .iter()
                .any(|e| e.key.invocation.program == key(SNAPSHOT))
        );
    }
}

fn install_routing(
    base: &mut offering::World,
    endpoint: &StagedOwnedRelease,
) -> (
    DeclaredSet<StatReceiver>,
    Vec<(SchemaSubject, OwnedDefinitionKey)>,
) {
    for owner in &mut base.source.base.inner.owners {
        owner
            .programs
            .members
            .retain(|p| p.id != key(offering::SNAPSHOT));
    }
    base.item_programs
        .retain(|(_, p)| p != &key(offering::SNAPSHOT));
    for s in [stat(0x3305), stat(0x3306)] {
        offering::add_definition(
            &mut base.source,
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .find(|d| d.address() == s.address())
                .unwrap()
                .clone(),
        );
        base.source.base.inner.owner_mut(subject(s));
    }
    let mut receivers = vec![];
    let mut early = vec![];
    for s in [stat(0x3305), stat(0x32e4)] {
        let owner = endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == subject(s.clone()))
            .unwrap()
            .clone();
        assert!(owner.programs.is_complete());
        early.extend(
            owner
                .programs
                .members
                .iter()
                .map(|p| (owner.owner.clone(), p.id.clone())),
        );
        let target = base.source.base.inner.owner_mut(owner.owner.clone());
        *target = owner;
        let rows: Vec<_> = endpoint
            .input()
            .recipe
            .rules
            .receivers
            .members
            .iter()
            .filter(|r| r.stat == s)
            .cloned()
            .collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].targets, vec![StatReceiverTarget::Player]);
        receivers.extend(rows);
    }
    for template in base
        .source
        .base
        .inner
        .build
        .items
        .iter()
        .map(|i| i.template.clone())
        .collect::<Vec<_>>()
    {
        let owner = endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == subject(template.clone()))
            .unwrap();
        let p = owner
            .programs
            .members
            .iter()
            .find(|p| p.id == key(APPLICABILITY))
            .unwrap()
            .clone();
        base.source
            .base
            .inner
            .owner_mut(owner.owner.clone())
            .programs
            .members
            .push(p);
        base.item_programs
            .push((owner.owner.clone(), key(APPLICABILITY)));
    }
    (DeclaredSet::complete(receivers), early)
}

fn install_sniper(
    base: &mut offering::World,
    endpoint: &StagedOwnedRelease,
    b: &Value,
) -> (DefinitionRules, DefinitionRules) {
    let actual_gem = endpoint
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == subject(gem()))
        .unwrap()
        .clone();
    let actual_skill = endpoint
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == subject(skill()))
        .unwrap()
        .clone();
    assert!(!actual_gem.programs.is_complete() && !actual_skill.programs.is_complete());
    let physical = [
        gem().address(),
        skill().address(),
        def::<ActorDefinition>("def.0000000000003091").address(),
    ];
    let wanted = physical
        .iter()
        .cloned()
        .chain([stat(0x1c).address(), stat(0x1d).address()])
        .collect::<Vec<_>>();
    for address in wanted {
        let actual = endpoint
            .input()
            .recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == address)
            .unwrap();
        let mut value = json!(offering::prolonged::finite(actual));
        // This component stops at the actual population. Ability Action demand,
        // reservation, damage and the full Actor owner remain separate obligations.
        if address == skill().address() {
            value["value"]["schema"]["value"]["declarations"]["outputs"]["members"] = json!([]);
        }
        if address == physical[2] {
            for field in ["grants", "skill_grants"] {
                value["value"]["schema"]["value"]["declarations"][field]["members"] = json!([]);
            }
        }
        let descriptor: DefinitionDescriptor = decode(&value);
        if let Some(old) = base
            .source
            .base
            .inner
            .schema
            .definitions
            .iter_mut()
            .find(|d| d.address() == address)
        {
            *old = descriptor;
        } else {
            base.source.base.inner.schema.definitions.push(descriptor);
        }
        base.source
            .base
            .inner
            .owner_mut(SchemaSubject::Definition(address));
    }
    let slot_ids = [0x13, 0x14, 0x16, 0x17, 0x1f, 0x20, 0x30aa, 0x30b1];
    for original in &endpoint.input().recipe.schema.slots {
        let mut value = json!(offering::prolonged::finite(original));
        let name = value["value"]["id"]["slot"]["key"]
            .as_str()
            .unwrap()
            .to_owned();
        if !slot_ids.iter().any(|n| name == format!("def.{n:016x}")) {
            continue;
        }
        if name == "def.0000000000000016" {
            value["value"]["schema"]["value"]["outputs"]["members"] = json!([]);
        }
        if name == "def.000000000000001f" {
            value["value"]["schema"]["value"]["outputs"]["members"] = json!([]);
            value["value"]["schema"]["value"]["skills"]["members"] = json!([]);
        }
        let slot: SlotDescriptor = decode(&value);
        let address = slot.address();
        if let Some(old) = base
            .source
            .base
            .inner
            .schema
            .slots
            .iter_mut()
            .find(|s| s.address() == address)
        {
            *old = slot;
        } else {
            base.source.base.inner.schema.slots.push(slot);
        }
        base.source
            .base
            .inner
            .owner_mut(SchemaSubject::Slot(address));
    }
    *base.source.base.inner.owner_mut(actual_gem.owner.clone()) =
        offering::prolonged::finite(&actual_gem);
    let mut population: DefinitionRules = offering::prolonged::finite(&actual_skill);
    population.programs.members.retain(|p| {
        matches!(
            p.id.as_str(),
            "ordinary-population-inputs" | "ordinary-population-requirements"
        )
    });
    *base.source.base.inner.owner_mut(actual_skill.owner.clone()) = population;
    // Even an empty selected sequence requires typed receiving inputs. Reuse
    // the existing finite fixture vocabulary solely for this empty-support
    // component. No support predicate is exercised, and these literals are not
    // source facts, final Gem inputs, or production admission authority.
    let predicate_facts = base
        .source
        .base
        .inner
        .owners
        .iter()
        .find(|o| o.owner == subject(decode::<GemDefId>(&base.bindings["target"]["physical_gem"])))
        .unwrap()
        .programs
        .members
        .iter()
        .find(|p| p.id == key("fixture.initial-facts"))
        .unwrap()
        .clone();
    for owner in [subject(gem()), subject(skill())] {
        base.source
            .base
            .inner
            .owner_mut(owner)
            .programs
            .members
            .push(predicate_facts.clone());
    }
    for name in ["sniper.actor-level", "sniper.required-character-level"] {
        base.tables.push(
            endpoint
                .input()
                .recipe
                .rules
                .tables
                .iter()
                .find(|t| t.id == key(name))
                .unwrap()
                .clone(),
        );
    }
    let parameters: Vec<_> = base
        .source
        .base
        .inner
        .schema
        .slots
        .iter()
        .filter_map(|s| match s {
            SlotDescriptor::Parameter(DefinitionEntry {
                id,
                schema: SchemaState::Known(s),
            }) if id.declaration == SlotOwnerDefId::Gem(gem()) => Some(ParameterAssignment {
                slot: id.clone(),
                value: match &s.value {
                    ValueSchema::Boolean => ParameterValue::Boolean(false),
                    ValueSchema::Quantity(q) => quantity(0., q.minimum.unit()),
                    _ => panic!("actual raw Gem input"),
                },
            }),
            _ => None,
        })
        .collect();
    for index in 0..2 {
        base.source.base.inner.build.gems.push(GemInstance {
            id: id(7100 + index),
            definition: gem(),
            parameters: parameters.clone(),
            level: 20,
            quality: Some(QualitySelection {
                kind: def("def.0000000000000006"),
                amount: FiniteQuantity::new(0., def("def.0000000000000002")).unwrap(),
            }),
        });
        base.source.base.inner.build.skills.push(SkillUse {
            id: id(7200 + index),
            source: AuthoredSkillSource::Gem(id(7100 + index)),
            parameters: None,
            enabled: true,
            scope: LoadoutScope::Shared,
        });
        // Explicit test inventory, not a discovered real Original05 empty list.
        base.source
            .base
            .inner
            .build
            .support_origins
            .as_mut()
            .unwrap()
            .push(SupportOriginSequence {
                target: SkillTarget::Authored(id(7200 + index)),
                origins: vec![],
            });
    }
    assert_eq!(b["target"]["physical_gem"], json!(gem()));
    (actual_gem, actual_skill)
}
