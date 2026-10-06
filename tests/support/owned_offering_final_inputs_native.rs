//! One native graph from real item programs to source-owned Offering inputs.
//! The occurrence/type inventories are deliberately finite. The pre-Amulet
//! snapshot remains an explicit test input; it is never a game default. No
//! final-level/quality literals, attached minion or complete-build claim exist.
#[allow(dead_code)]
#[path = "owned_amulet_level_copy_native.rs"]
mod item_fixture;
#[allow(dead_code)]
#[path = "owned_prolonged_duration_fixture.rs"]
mod prolonged;
// The reused nested fixture authenticates its endpoint through this same loader.
use crate::release;
use poe_optimizer_core::{
    build_identity::BuildLineage, owned_build::*, owned_definitions::*, owned_readiness::*,
    owned_rules::*, owned_schema::*, owned_source_properties::*, owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_release::StagedOwnedRelease,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use prolonged::bidding_fixture::{self as shared, decode, def, id, key, quantity, subject};
use rayon::prelude::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

const SNAPSHOT: &str = "fixture-pre-amulet-snapshot";
const TABLE_OBSERVATION: &str = "observe-offering-table";

#[derive(Clone)]
struct World {
    source: prolonged::World,
    bindings: Value,
    relation: SourcePropertyPreparationInput,
    reference: Value,
    item_programs: Vec<(SchemaSubject, OwnedDefinitionKey)>,
    tables: Vec<IntegerRuleTable>,
    actual_modifier: DefinitionRules,
}
impl World {
    fn load() -> Self {
        static WORLD: OnceLock<World> = OnceLock::new();
        WORLD.get_or_init(Self::load_once).clone()
    }
    fn load_once() -> Self {
        let path = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_OFFERING_INPUTS_RELEASE")
                .expect("checked Offering input publication"),
        );
        let before = release::inventory(&path);
        let endpoint = release::load(&path);
        crate::family::assert_endpoint(&endpoint);
        let bindings: Value = crate::family::read("bindings.json");
        let relation = crate::family::read("source-properties.json");
        let reference = crate::family::read("readiness.json");
        let mut source = prolonged::World::load_release(&path, false);
        let gem: GemDefId = decode(&bindings["target"]["physical_gem"]);
        source
            .base
            .inner
            .owner_mut(subject(gem))
            .programs
            .members
            .retain(|p| p.id != key(prolonged::FINAL_BOUNDARY));
        assert!(
            !source
                .base
                .inner
                .owners
                .iter()
                .flat_map(|o| &o.programs.members)
                .any(|p| p.id == key(prolonged::FINAL_BOUNDARY))
        );
        let stat: StatDefId = decode(&bindings["channels"]["non_hidden_count"]);
        add_definition(
            &mut source,
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .find(|d| d.address() == stat.address())
                .unwrap()
                .clone(),
        );
        source.base.inner.owner_mut(subject(stat));
        let (item_programs, actual_modifier) =
            install_items(&mut source, &endpoint, &path, &bindings);
        let table = endpoint
            .input()
            .recipe
            .rules
            .tables
            .iter()
            .find(|t| t.id == key("pain-offering.damage-increase"))
            .unwrap()
            .clone();
        assert_eq!((table.minimum.get(), table.maximum.get()), (1, 40));
        install_table_observation(&mut source, &bindings, &table);
        assert_eq!(before, release::inventory(&path));
        Self {
            source,
            bindings,
            relation,
            reference,
            item_programs,
            tables: vec![table],
            actual_modifier,
        }
    }
    fn raw(&mut self, source: usize, level: u16, quality: f64) {
        let gem = self
            .source
            .base
            .inner
            .build
            .gems
            .iter_mut()
            .find(|g| g.id == id(900 + (source - 2) as u64))
            .unwrap();
        gem.level = level;
        gem.quality.as_mut().unwrap().amount =
            FiniteQuantity::new(quality, self.source.base.inner.quality_unit.clone()).unwrap();
    }
    fn snapshot(&mut self, value: f64) {
        let owner = self
            .source
            .base
            .inner
            .owner_mut(subject(def::<ClassDefinition>("fixture.class")));
        let program = owner
            .programs
            .members
            .iter_mut()
            .find(|p| p.id == key(SNAPSHOT))
            .unwrap();
        let RuleExpression::Literal {
            value: ParameterValue::Quantity(q),
        } = &mut program.nodes[0].expression
        else {
            panic!()
        };
        *q = FiniteQuantity::new(value, q.unit().clone()).unwrap();
    }
    fn corruption(&mut self, source: usize, value: f64) {
        let slot = self
            .source
            .physical_owner
            .programs
            .members
            .iter()
            .find(|p| p.id == key("effective-input-preparation"))
            .unwrap()
            .reads
            .iter()
            .find_map(|r| match &r.source {
                RuleReadSource::Parameter { slot } if r.id == key("corruption") => {
                    Some(slot.clone())
                }
                _ => None,
            })
            .unwrap();
        let gem = self
            .source
            .base
            .inner
            .build
            .gems
            .iter_mut()
            .find(|g| g.id == id(900 + (source - 2) as u64))
            .unwrap();
        let assignment = gem.parameters.iter_mut().find(|p| p.slot == slot).unwrap();
        let ParameterValue::Quantity(q) = &assignment.value else {
            panic!()
        };
        assignment.value = quantity(value, q.unit());
    }
    fn item_amount(&mut self, template: &str, value: f64) {
        let template: ItemTemplateDefId = decode(&self.bindings["ordinary_items"][template]);
        let item = self
            .source
            .base
            .inner
            .build
            .items
            .iter_mut()
            .find(|i| i.template == template)
            .unwrap();
        let row = item.modifiers[0]
            .rolls
            .iter_mut()
            .find(|r| r.slot.slot == def("def.00000000000030cb"))
            .unwrap();
        let ParameterValue::Quantity(q) = &row.value else {
            panic!()
        };
        row.value = quantity(value, q.unit());
    }
    fn remove_item(&mut self, template: &str) {
        let template: ItemTemplateDefId = decode(&self.bindings["ordinary_items"][template]);
        let item = self
            .source
            .base
            .inner
            .build
            .items
            .iter()
            .find(|i| i.template == template)
            .unwrap()
            .id;
        self.source
            .base
            .inner
            .build
            .equipment
            .retain(|e| e.item != item);
    }
    fn checked_plan(&self) -> std::result::Result<shared::Plan, String> {
        self.checked_plan_variant(OWNED_EVALUATION_STAGES_V4, false)
    }
    fn checked_plan_variant(
        &self,
        version: u32,
        late_item: bool,
    ) -> std::result::Result<shared::Plan, String> {
        let target = &self.bindings["target"];
        let mut readiness = self.source.base.readiness_inputs(
            &[
                decode(&target["final_level_parameter"]),
                decode(&target["final_quality_parameter"]),
            ],
            &[],
        );
        let authored: ReadinessInput = decode(&self.reference["readiness"]);
        for row in authored.programs.members {
            if let Some(found) = readiness
                .programs
                .members
                .iter_mut()
                .find(|p| p.owner == row.owner && p.program == row.program)
            {
                *found = row;
            }
        }
        let source_inputs: BTreeSet<StatDefId> = [
            decode(&self.bindings["channels"]["pre_support_level"]),
            decode(&self.bindings["channels"]["pre_support_quality"]),
        ]
        .into_iter()
        .collect();
        // Other retained physical Skill definitions share these declared input
        // channels. Their potential writers must have the same early contract;
        // an unselected definition cannot conceal a late channel producer.
        let mut early_inputs = vec![];
        for owner in &self.source.base.inner.owners {
            for program in &owner.programs.members {
                if program.context==RuleEntityKind::Skill && program.effects.iter().any(|e|matches!(&e.effect,RuleEffectKind::Derive{stat,..} if source_inputs.contains(stat))) {
                    let row=readiness.programs.members.iter_mut().find(|r|r.owner==owner.owner && r.program==program.id).unwrap();
                    row.phase=ReadinessPhase::Structural;row.role=ReadinessProgramRole::PreparationFacts;
                    row.outputs=program.effects.iter().map(|e|channel(program.context,&e.effect)).collect();
                    early_inputs.push((owner.owner.clone(),program.id.clone()));
                }
            }
        }
        for (owner, program) in &self.item_programs {
            let p = self
                .source
                .base
                .inner
                .owners
                .iter()
                .find(|o| &o.owner == owner)
                .unwrap()
                .programs
                .members
                .iter()
                .find(|p| &p.id == program)
                .unwrap();
            let row = readiness
                .programs
                .members
                .iter_mut()
                .find(|r| &r.owner == owner && &r.program == program)
                .unwrap();
            row.phase = ReadinessPhase::Structural;
            row.role = ReadinessProgramRole::PreparationFacts;
            row.outputs = p
                .effects
                .iter()
                .map(|e| channel(p.context, &e.effect))
                .collect();
            if late_item && program == &key("effective-amount") {
                row.phase = ReadinessPhase::Execution;
                row.role = ReadinessProgramRole::Execution;
                row.outputs.clear();
            }
        }
        let generated_owner = subject(decode::<SkillDefId>(&target["primary_skill"]));
        let generated_facts = self
            .source
            .base
            .inner
            .owners
            .iter()
            .find(|o| o.owner == generated_owner)
            .unwrap()
            .programs
            .members
            .iter()
            .find(|p| p.id == key("fixture.initial-facts"))
            .unwrap();
        let generated_channels: Vec<_> = generated_facts
            .effects
            .iter()
            .map(|e| channel(generated_facts.context, &e.effect))
            .collect();
        let programs: Vec<StagedRuleProgram> = decode(&self.reference["programs"]);
        let frozen: Vec<FrozenStageChannel> = decode(&self.reference["frozen_channels"]);
        self.source.base.inner.checked_plan_with_components(
            self.source.request(),
            Some(readiness),
            shared::PlanComponents {
                tables: self.tables.clone(),
                receivers: vec![],
                source_properties: Some(self.relation.clone()),
            },
            |stages| {
                // Explicit V4 local preparation authority. The same declaration is
                // rejected by older stage versions in the focused negative test.
                stages.schema_version = version;
                stages.stages = [
                    "prepare",
                    "source-prepare",
                    "source-census",
                    "source-assembly",
                    "facts",
                    "apply",
                    "deliver",
                ]
                .into_iter()
                .enumerate()
                .map(|(i, name)| EvaluationStage {
                    id: key(name),
                    predecessors: if i == 0 {
                        vec![]
                    } else {
                        vec![key([
                            "prepare",
                            "source-prepare",
                            "source-census",
                            "source-assembly",
                            "facts",
                            "apply",
                        ][i - 1])]
                    },
                })
                .collect();
                let mut generated_facts = 0;
                for row in &mut stages.programs.members {
                    // Generated Skill facts depend on the real entering grant.
                    // Its activation is produced in source-prepare, so these
                    // finite predicate inputs cannot use the shared fixture's
                    // earlier prepare default.
                    if row.owner == generated_owner && row.program == key("fixture.initial-facts") {
                        row.stage = key("source-prepare");
                        generated_facts += 1;
                    }
                    if self
                        .item_programs
                        .contains(&(row.owner.clone(), row.program.clone()))
                    {
                        row.stage = key("prepare");
                    }
                    if early_inputs.contains(&(row.owner.clone(), row.program.clone())) {
                        row.stage = key("source-prepare");
                    }
                    if let Some(actual) = programs
                        .iter()
                        .find(|p| p.owner == row.owner && p.program == row.program)
                    {
                        row.stage = actual.stage.clone();
                    }
                    if late_item && row.program == key("effective-amount") {
                        row.stage = key("deliver");
                    }
                }
                assert_eq!(generated_facts, 1, "exact generated predicate-facts owner");
                stages.frozen_channels.retain(|r| {
                    r.channel
                        != StageChannel::Stat {
                            scope: RuleEntityKind::Skill,
                            stat: def("fixture.offering-table-value"),
                        }
                });
                for row in &mut stages.frozen_channels {
                    if generated_channels.contains(&row.channel) {
                        row.stage = key("source-prepare");
                    }
                }
                stages.frozen_channels.extend(frozen);
            },
            |inputs| {
                inputs.preparation_stage = key("source-prepare");
            },
        )
    }
    fn plan(&self) -> shared::Plan {
        self.checked_plan().unwrap()
    }
}

fn channel(context: RuleEntityKind, effect: &RuleEffectKind) -> StageChannel {
    let scope = |entity| match entity {
        RuleEntity::Current => context,
        RuleEntity::Player | RuleEntity::Actor => RuleEntityKind::Actor,
        RuleEntity::Modifier => RuleEntityKind::Modifier,
        _ => panic!("unreviewed finite early destination"),
    };
    match effect {
        RuleEffectKind::Derive { entity, stat, .. } => StageChannel::Stat {
            scope: scope(*entity),
            stat: stat.clone(),
        },
        RuleEffectKind::Contribute {
            entity,
            stat,
            contribution,
            ..
        } => StageChannel::Contributions {
            scope: scope(*entity),
            stat: stat.clone(),
            contribution: *contribution,
        },
        _ => panic!("unreviewed item effect"),
    }
}
fn add_definition(w: &mut prolonged::World, value: DefinitionDescriptor) {
    if let Some(prior) = w
        .base
        .inner
        .schema
        .definitions
        .iter()
        .find(|d| d.address() == value.address())
    {
        assert_eq!(prior, &value, "shared definition identity");
    } else {
        w.base.inner.schema.definitions.push(value);
    }
}
fn known_assignments(value: &Value) -> Vec<ParameterAssignment> {
    assert_eq!(value["completion"]["kind"], "complete");
    value["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            assert_eq!(row["slot"]["kind"], "known");
            assert_eq!(row["value"]["kind"], "known");
            ParameterAssignment {
                slot: decode(&row["slot"]["value"]),
                value: decode(&row["value"]["value"]),
            }
        })
        .collect()
}
/// Re-import immutable corpus bytes with the checked current release. An
/// adjacent draft is not an authority: exact XML Item IDs join fresh sidecar
/// source ordinals to their canonical item instances before any value is used.
fn canonical_items(package: &std::path::Path, bindings: &Value) -> Vec<Value> {
    let corpus =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/builds/breadth-20260908");
    let index: Value = shared::read(corpus.join("index.json"));
    let manifest: Vec<_> = index["builds"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["xml"] == "build-05.xml")
        .collect();
    assert_eq!(manifest.len(), 1);
    let xml_path = corpus.join("build-05.xml");
    let bytes = std::fs::read(&xml_path).unwrap();
    assert_eq!(
        bytes.len() as u64,
        manifest[0]["xml_bytes"].as_u64().unwrap()
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        manifest[0]["xml_sha256"]
    );
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("canonical");
    let report = release::normalize(package, &xml_path, 5, &output);
    let draft: Value = shared::read(output.join("draft.json"));
    let sidecar: Value = shared::read(output.join("sidecar.json"));
    assert_eq!(sidecar["source_sha256"], manifest[0]["xml_sha256"]);
    assert_eq!(report["normalization_status"], "pending");
    let source = ImportedBuildInstance::from_decoded(
        decode_build(&bytes).unwrap(),
        BuildLineage::from_bytes([93; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let mut result = vec![];
    for field in ["helmet", "amulet"] {
        let source_id = bindings["ordinary_items"]["source_item_ids"][field]
            .as_u64()
            .unwrap()
            .to_string();
        let rows: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|row| {
                row.occurrence().name() == "Item"
                    && row
                        .attribute("id")
                        .is_some_and(|a| a.decoded().unwrap() == source_id)
            })
            .collect();
        assert_eq!(rows.len(), 1, "exact source Item {source_id}");
        let origin = serde_json::to_value(rows[0].occurrence().id()).unwrap();
        let links: Vec<_> = sidecar["origins"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["source"] == origin)
            .collect();
        assert_eq!(links.len(), 1);
        let item_links: Vec<_> = links[0]["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|link| link["kind"] == "item")
            .collect();
        assert_eq!(item_links.len(), 1);
        let items: Vec<_> = draft["draft"]["items"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["id"] == item_links[0]["value"])
            .collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["template"]["kind"], "known");
        assert_eq!(
            items[0]["template"]["value"],
            bindings["ordinary_items"][field]
        );
        result.push(items[0].clone());
    }
    assert_ne!(result[0]["id"], result[1]["id"]);
    assert_eq!(bytes, std::fs::read(&xml_path).unwrap());
    result
}

fn install_items(
    w: &mut prolonged::World,
    endpoint: &StagedOwnedRelease,
    path: &std::path::Path,
    b: &Value,
) -> (Vec<(SchemaSubject, OwnedDefinitionKey)>, DefinitionRules) {
    let mut items = item_fixture::Fixture::new();
    items.original_pair();
    items.boundary_factor(0.0);
    let modifier: ModifierDefId = decode(&b["ordinary_items"]["modifier"]);
    let actual = endpoint
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == subject(modifier.clone()))
        .unwrap()
        .clone();
    let component = items
        .native
        .recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == actual.owner)
        .unwrap();
    let mut component_programs = component.programs.members.clone();
    component_programs.sort_by(|a, b| a.id.cmp(&b.id));
    let mut actual_programs = actual.programs.members.clone();
    actual_programs.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(
        component_programs, actual_programs,
        "exact published numeric/copy bodies"
    );
    assert!(!actual.programs.is_complete());
    let templates: BTreeSet<_> = items
        .native
        .build
        .items
        .iter()
        .map(|i| i.template.clone())
        .collect();
    assert_eq!(templates.len(), 2);
    // Only real numeric dependencies and the two selected placements are used.
    // Fixture-only Class/Encounter/template allocations never enter this graph.
    for d in &items.native.recipe.schema.definitions {
        let wanted = match d {
            DefinitionDescriptor::Unit(_)
            | DefinitionDescriptor::Option(_)
            | DefinitionDescriptor::Stat(_) => true,
            DefinitionDescriptor::Modifier(e) => e.id == modifier,
            DefinitionDescriptor::ItemTemplate(e) => templates.contains(&e.id),
            DefinitionDescriptor::EquipmentSlot(e) => {
                e.id == items.bindings.placement_slots.helmet
                    || e.id == items.bindings.placement_slots.amulet
            }
            _ => false,
        };
        if wanted {
            add_definition(w, d.clone());
        }
    }
    // Canonical 24-slot modifier records, including category, come from a fresh
    // checked import and an exact source Item join. No parsed defaults.
    let canonical = canonical_items(path, b);
    let mut wanted_slots = BTreeSet::new();
    let mut programs = vec![];
    for (index, item) in items.native.build.items.iter_mut().enumerate() {
        let rows: Vec<_> = canonical
            .iter()
            .filter(|r| {
                r["template"]["kind"] == "known"
                    && decode::<ItemTemplateDefId>(&r["template"]["value"]) == item.template
            })
            .collect();
        assert_eq!(rows.len(), 1, "exact canonical item template");
        let source = rows[0];
        let mods: Vec<_> = source["modifiers"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| {
                m["definition"]["kind"] == "known"
                    && decode::<ModifierDefId>(&m["definition"]["value"]) == modifier
            })
            .collect();
        assert_eq!(mods.len(), 1);
        item.modifiers[0].rolls = known_assignments(&mods[0]["rolls"]);
        assert_eq!(item.modifiers[0].rolls.len(), 24);
        wanted_slots.extend(item.modifiers[0].rolls.iter().map(|p| p.slot.clone()));
        let owner = endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == subject(item.template.clone()))
            .unwrap();
        let selected: Vec<_> = owner
            .programs
            .members
            .iter()
            .filter(|p| p.id == key("catalyst-inputs") || p.id == key("amulet-copy-eligibility"))
            .cloned()
            .collect();
        assert_eq!(selected.len(), 2);
        assert!(!owner.programs.is_complete());
        let input_slots: BTreeSet<_> = selected
            .iter()
            .flat_map(|p| &p.reads)
            .filter_map(|r| match &r.source {
                RuleReadSource::Parameter { slot } => Some(slot.clone()),
                _ => None,
            })
            .collect();
        item.parameters = known_assignments(&source["parameters"])
            .into_iter()
            .filter(|p| input_slots.contains(&p.slot))
            .collect();
        assert_eq!(item.parameters.len(), input_slots.len());
        wanted_slots.extend(input_slots.clone());
        let d = w
            .base
            .inner
            .schema
            .definitions
            .iter_mut()
            .find(|d| d.address() == item.template.address())
            .unwrap();
        let DefinitionDescriptor::ItemTemplate(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) = d
        else {
            panic!()
        };
        schema.declarations.parameters = DeclaredSet::complete(input_slots.into_iter().collect());
        let destination = w.base.inner.owner_mut(owner.owner.clone());
        destination.programs = DeclaredSet::complete(selected);
        programs.extend(
            destination
                .programs
                .members
                .iter()
                .map(|p| (destination.owner.clone(), p.id.clone())),
        );
        let old = item.id;
        item.id = id(6000 + index as u64 * 10);
        item.modifiers[0].id = id(6001 + index as u64 * 10);
        item.modifier_order = vec![item.modifiers[0].id];
        let equipment = items
            .native
            .build
            .equipment
            .iter_mut()
            .find(|e| e.item == old)
            .unwrap();
        equipment.item = item.id;
        equipment.id = id(6002 + index as u64 * 10);
    }
    // The exact actual declaration includes the saved category slot absent from
    // the older numeric component's narrower isolated roll inventory.
    let definition = endpoint
        .input()
        .recipe
        .schema
        .definitions
        .iter()
        .find(|d| d.address() == modifier.address())
        .unwrap();
    *w.base
        .inner
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == modifier.address())
        .unwrap() = prolonged::finite(definition);
    for slot in wanted_slots {
        let value = endpoint
            .input()
            .recipe
            .schema
            .slots
            .iter()
            .find(|s| s.address() == SlotAddress::Parameter(slot.clone()))
            .unwrap();
        assert!(
            !w.base
                .inner
                .schema
                .slots
                .iter()
                .any(|s| s.address() == value.address())
        );
        if let SlotDescriptor::Parameter(DefinitionEntry {
            schema:
                SchemaState::Known(ParameterSlotSchema {
                    value: ValueSchema::Option { allowed },
                    ..
                }),
            ..
        }) = value
        {
            for option in &allowed.members {
                let definition = endpoint
                    .input()
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .find(|d| d.address() == option.address())
                    .unwrap();
                add_definition(w, definition.clone());
            }
        }
        w.base.inner.schema.slots.push(prolonged::finite(value));
    }
    let category = endpoint
        .input()
        .recipe
        .schema
        .definitions
        .iter()
        .find(|d| d.address() == def::<OptionDefinition>("def.00000000000030e2").address())
        .unwrap();
    add_definition(w, category.clone());
    *w.base.inner.owner_mut(actual.owner.clone()) = prolonged::finite(&actual);
    programs.extend(
        actual
            .programs
            .members
            .iter()
            .map(|p| (actual.owner.clone(), p.id.clone())),
    );
    let snapshot = items
        .native
        .recipe
        .rules
        .owners
        .iter()
        .flat_map(|o| &o.programs.members)
        .find(|p| p.id == key(SNAPSHOT))
        .unwrap()
        .clone();
    let class = subject(def::<ClassDefinition>("fixture.class"));
    w.base
        .inner
        .owner_mut(class.clone())
        .programs
        .members
        .push(snapshot);
    programs.push((class, key(SNAPSHOT)));
    w.base.inner.build.items = items.native.build.items;
    w.base.inner.build.equipment = items.native.build.equipment;
    (programs, actual)
}

fn install_table_observation(w: &mut prolonged::World, b: &Value, table: &IntegerRuleTable) {
    let stat: StatDefId = def("fixture.offering-table-value");
    w.base
        .inner
        .schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: stat.clone(),
            schema: SchemaState::Known(StatSchema {
                value: table.value_type.clone(),
                targets: vec![RuleEntityKind::Skill],
            }),
        }));
    w.base.inner.owner_mut(subject(stat.clone()));
    w.base
        .inner
        .owner_mut(subject(decode::<SkillDefId>(&b["target"]["primary_skill"])))
        .programs
        .members
        .push(RuleProgram {
            id: key(TABLE_OBSERVATION),
            context: RuleEntityKind::Skill,
            reads: vec![RuleRead {
                id: key("level"),
                value_type: ComputedValueType::Integer,
                source: RuleReadSource::Parameter {
                    slot: decode(&b["target"]["final_level_parameter"]),
                },
            }],
            nodes: vec![
                RuleNode {
                    id: key("level"),
                    expression: RuleExpression::Read {
                        input: key("level"),
                    },
                },
                RuleNode {
                    id: key("table"),
                    expression: RuleExpression::LookupIntegerTable {
                        table: table.id.clone(),
                        key: key("level"),
                    },
                },
            ],
            effects: vec![RuleEffect {
                id: key("observe"),
                when: None,
                effect: RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat,
                    value: key("table"),
                },
            }],
        });
}

fn evaluate(w: &World) -> SupportEffectsReport {
    let p = w.plan();
    p.evaluate(&mut p.new_scratch()).unwrap()
}
fn effects(r: &SupportEffectsReport) -> &OwnedEffectsReport {
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    let SupportEffectsOutcome::Evaluated { effects } = &r.outcome else {
        panic!("{r:?}")
    };
    assert!(effects.gaps.is_empty(), "{:?}", effects.gaps);
    effects
}
fn check(w: &World, r: &SupportEffectsReport, levels: [i64; 2], qualities: [f64; 2]) {
    let report = effects(r);
    for (i, source) in [3usize, 4].into_iter().enumerate() {
        let generated = GeneratedSkillKey {
            provider: ProviderKey {
                root: ProviderRoot::SkillUse(id(20 + source as u64)),
                grant_path: vec![],
            },
            slot: decode(&w.bindings["target"]["primary_supply"]),
        };
        for (field, value) in [
            (
                "final_level_parameter",
                ParameterValue::Integer(BoundedInteger::new(levels[i]).unwrap()),
            ),
            (
                "final_quality_parameter",
                quantity(qualities[i], &w.source.base.inner.quality_unit),
            ),
        ] {
            let wanted = PlanValueKey::SkillParameter {
                skill: Box::new(generated.clone()),
                parameter: decode(&w.bindings["target"][field]),
            };
            let rows: Vec<_> = report
                .effects
                .iter()
                .filter(|e| matches!(&e.target,BoundEffectTarget::Value{key}if *key==wanted))
                .collect();
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0].key.invocation.program,
                key("pain-offering-final-inputs")
            );
            assert!(
                matches!(&rows[0].key.invocation.origin,RuleOrigin::SourceProperty {relation,owner,producer,position}
                if *relation==key("pain-offering-source-inputs")
                    && **owner==SkillTarget::Authored(id(20+source as u64))
                    && producer.root==ProviderRoot::SkillUse(id(20+source as u64))
                    && producer.grant_path.is_empty() && position.is_none())
            );
            assert_eq!(rows[0].value, EffectValue::Known { value });
        }
        let wanted = PlanValueKey::Stat {
            entity: ConcreteEntity::Skill(Box::new(SkillTarget::Generated(Box::new(generated)))),
            stat: def("fixture.offering-table-value"),
        };
        let row = report.values.iter().find(|r| r.key == wanted).unwrap();
        assert_eq!(
            row.value,
            EffectValue::Known {
                value: w.tables[0].rows[(levels[i] - 1) as usize].clone()
            }
        );
    }
    assert!(
        report
            .effects
            .iter()
            .all(|e| e.key.invocation.program != key(prolonged::FINAL_BOUNDARY))
    );
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_OFFERING_INPUTS_RELEASE; finite native component"]
fn offering_inputs_use_actual_items_and_independent_raw_sources() {
    let mut w = World::load();
    let r = evaluate(&w);
    check(&w, &r, [22, 22], [0.0, 0.0]);
    let direct: Vec<_> = effects(&r)
        .effects
        .iter()
        .filter(|e| {
            e.key.invocation.program == key("contribute-player-minion-gem-level")
                && e.value != EffectValue::Inactive
        })
        .collect();
    assert_eq!(direct.len(), 2);
    assert_ne!(direct[0].key, direct[1].key);
    for (equipment, modifier) in [(6002, 6001), (6012, 6011)] {
        let rows:Vec<_>=direct.iter().filter(|e|matches!(&e.key.invocation.origin,RuleOrigin::Provider{provider}
            if provider.root==ProviderRoot::ItemModifier{equipment_use:id(equipment),modifier:id(modifier)} && provider.grant_path.is_empty())).collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].value,
            EffectValue::Known {
                value: quantity(1.0, &def("def.000000000000295a"))
            }
        );
        assert!(
            matches!(&rows[0].target,BoundEffectTarget::Contribution{key}
            if key.entity==ConcreteEntity::Actor(ActorKey::Player) && key.stat==decode::<StatDefId>(&w.bindings["channels"]["global_minion_level"]) && key.kind==ContributionKind::Add)
        );
    }
    let copy: Vec<_> = effects(&r)
        .effects
        .iter()
        .filter(|e| {
            e.key.invocation.program == key("amulet-copy-minion-gem-level")
                && e.value != EffectValue::Inactive
        })
        .collect();
    assert_eq!(copy.len(), 1);
    assert_eq!(
        copy[0].value,
        EffectValue::Known {
            value: quantity(0.0, &def("def.000000000000295a"))
        }
    );
    assert!(
        matches!(&copy[0].key.invocation.origin,RuleOrigin::Provider{provider}
        if provider.root==ProviderRoot::ItemModifier{equipment_use:id(6002),modifier:id(6001)} && provider.grant_path.is_empty())
    );
    assert_eq!(
        w.tables[0].rows[21],
        quantity(62.0, &def("def.0000000000000002"))
    );
    w.raw(3, 17, 9.0);
    w.raw(4, 23, 15.0);
    check(&w, &evaluate(&w), [19, 25], [9.0, 15.0]);
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_OFFERING_INPUTS_RELEASE; finite native component"]
fn offering_inputs_respond_to_item_rolls_removal_and_explicit_snapshot() {
    let mut w = World::load();
    w.item_amount("helmet", 2.0);
    check(&w, &evaluate(&w), [23, 23], [0.0, 0.0]);
    w.snapshot(100.0);
    check(&w, &evaluate(&w), [24, 24], [0.0, 0.0]);
    w.remove_item("amulet");
    check(&w, &evaluate(&w), [22, 22], [0.0, 0.0]);
    w.remove_item("helmet");
    check(&w, &evaluate(&w), [20, 20], [0.0, 0.0]);
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_OFFERING_INPUTS_RELEASE; finite native component"]
fn offering_inputs_retain_both_support_tiers_removal_and_disabled_origins() {
    let mut w = World::load();
    for tier in [0usize, 1] {
        w.source.base.inner.build.supports.clear();
        w.source.base.inner.build.support_origins = Some(vec![]);
        for source in [2, 3, 4] {
            w.source.base.inner.add_support(source, tier);
        }
        let report = evaluate(&w);
        check(&w, &report, [22, 22], [0.0, 0.0]);
        check_census_and_delivery(&w, &report, Some(tier));
    }
    for s in &mut w.source.base.inner.build.supports {
        s.enabled = false;
    }
    let report = evaluate(&w);
    check(&w, &report, [22, 22], [0.0, 0.0]);
    check_census_and_delivery(&w, &report, None);
    w.source.base.inner.build.supports.clear();
    w.source.base.inner.build.support_origins = Some(vec![]);
    let report = evaluate(&w);
    check(&w, &report, [22, 22], [0.0, 0.0]);
    check_census_and_delivery(&w, &report, None);
}

fn check_census_and_delivery(w: &World, report: &SupportEffectsReport, tier: Option<usize>) {
    let report = effects(report);
    let counts: Vec<_> = report
        .effects
        .iter()
        .filter(|e| {
            matches!(
                e.key.invocation.origin,
                RuleOrigin::SourcePropertyCensus { .. }
            )
        })
        .collect();
    assert_eq!(
        counts.len(),
        2,
        "one actual census per independent Offering source"
    );
    for source in [3usize, 4] {
        let target = SkillTarget::Authored(id(20 + source as u64));
        let rows:Vec<_>=counts.iter().filter(|e|matches!(&e.key.invocation.origin,
            RuleOrigin::SourcePropertyCensus{relation,owner} if *relation==key("pain-offering-source-inputs") && **owner==target)).collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].target,
            BoundEffectTarget::Value {
                key: PlanValueKey::Stat {
                    entity: ConcreteEntity::Skill(Box::new(target)),
                    stat: decode(&w.bindings["channels"]["non_hidden_count"]),
                }
            }
        );
        assert_eq!(
            rows[0].value,
            EffectValue::Known {
                value: ParameterValue::Integer(
                    BoundedInteger::new(i64::from(tier.is_some())).unwrap()
                )
            }
        );
    }
    let factor: UnitDefId = decode(&w.source.bindings["factor_unit"]);
    for source in [2usize, 3, 4] {
        for action in w.source.actions(source) {
            let rows:Vec<_>=report.effects.iter().filter(|e|e.key.invocation.program==key("prolonged-duration-factors")
                && matches!(&e.target,BoundEffectTarget::Contribution{key} if key.entity==ConcreteEntity::Action(Box::new(action.clone())))).collect();
            let Some(tier) = tier else {
                assert!(
                    rows.is_empty(),
                    "disabled/removed origins cannot contribute"
                );
                continue;
            };
            assert_eq!(rows.len(), 2);
            for (name, value) in [("duration_factor", [1.3, 1.35][tier]), ("cost_factor", 1.2)] {
                let channel: StatDefId = decode(&w.source.bindings["channels"][name]);
                let matching:Vec<_>=rows.iter().filter(|e|matches!(&e.target,BoundEffectTarget::Contribution{key} if key.stat==channel && key.kind==ContributionKind::Multiply)).collect();
                assert_eq!(matching.len(), 1);
                let row = matching[0];
                let RuleOrigin::SupportApplication { application } = &row.key.invocation.origin
                else {
                    panic!("exact receiving application")
                };
                assert_eq!(
                    application.prepared.origin,
                    SupportOrigin::Assignment(w.source.selected(source))
                );
                assert_eq!(
                    application.prepared.target,
                    SkillTarget::Authored(id(20 + source as u64))
                );
                assert_eq!(
                    application.receiver,
                    SupportReceiverKey::Action(Box::new(action.clone()))
                );
                assert_eq!(
                    row.value,
                    EffectValue::Known {
                        value: quantity(value, &factor)
                    }
                );
            }
        }
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_OFFERING_INPUTS_RELEASE; finite native component"]
fn offering_inputs_refuse_fractional_out_of_domain_and_missing_producers() {
    for fractional in [true, false] {
        let mut w = World::load();
        if fractional {
            // Count is a quantity before assembly. The actual item numeric
            // recipe rounds ordinary item values, so use the declared raw
            // corruption input to reach the fractional assembly boundary.
            for source in [3, 4] {
                w.corruption(source, 0.5);
            }
        } else {
            w.item_amount("helmet", 40.0);
        }
        let r = evaluate(&w);
        match &r.outcome {
            SupportEffectsOutcome::Evaluated { effects } => {
                let levels: Vec<_> = effects
                    .effects
                    .iter()
                    .filter(|e| {
                        e.key.invocation.program == key("pain-offering-final-inputs")
                            && e.key.effect == key("project-final-level")
                    })
                    .collect();
                assert_eq!(levels.len(), 2);
                assert!(
                    levels.iter().all(|e| e.value == EffectValue::Inactive),
                    "{levels:?}"
                );
                assert!(
                    !effects
                        .effects
                        .iter()
                        .any(|e| e.key.invocation.program == key(TABLE_OBSERVATION)
                            && matches!(e.value, EffectValue::Known { .. }))
                );
            }
            SupportEffectsOutcome::Unavailable { cause, input } => {
                assert!(
                    matches!(
                        cause,
                        EffectValue::Inactive
                            | EffectValue::Unresolved {
                                reason: PlanGapReason::MissingInput,
                                ..
                            }
                    ),
                    "{cause:?}"
                );
                assert!(
                    matches!(input.as_deref(),Some(PlanValueKey::SkillParameter{parameter,..}) if *parameter==decode::<DeclaredSlot<ParameterSlotDefId>>(&w.bindings["target"]["final_level_parameter"])),
                    "{input:?}"
                );
            }
            other => panic!("final-domain refusal must identify actual final input: {other:?}"),
        }
    }
    for missing in [SNAPSHOT, "effective-amount", "pain-offering-final-inputs"] {
        let mut w = World::load();
        for o in &mut w.source.base.inner.owners {
            o.programs.members.retain(|p| p.id != key(missing));
        }
        w.item_programs.retain(|(_, p)| *p != key(missing));
        if missing == "pain-offering-final-inputs" {
            let error = w
                .checked_plan()
                .err()
                .expect("explicit assembly reference cannot disappear");
            assert!(error.contains("unknown source property program"), "{error}");
        } else {
            let plan = w
                .checked_plan()
                .expect("missing producer is a numerical obligation, not a malformed fixture");
            let r = plan.evaluate(&mut plan.new_scratch()).unwrap();
            match &r.outcome {
                SupportEffectsOutcome::Unavailable { cause, .. } => assert!(
                    matches!(
                        cause,
                        EffectValue::Unresolved {
                            reason: PlanGapReason::MissingProducer,
                            ..
                        }
                    ),
                    "{missing}: {cause:?}"
                ),
                SupportEffectsOutcome::Evaluated { effects } => {
                    let expected_read = key(if missing == SNAPSHOT {
                        "pre-amulet-percent"
                    } else {
                        "effective"
                    });
                    assert!(
                        effects.effects.iter().any(|e| e.key.invocation.program
                            == key("amulet-copy-minion-gem-level")
                            && e.value
                                == EffectValue::Unresolved {
                                    reason: PlanGapReason::MissingProducer,
                                    read: Some(expected_read.clone())
                                }),
                        "{missing}: actual copy dependency must remain unresolved"
                    );
                    assert!(!effects.effects.iter().any(|e| e.key.invocation.program
                        == key("pain-offering-final-inputs")
                        && e.key.effect == key("project-final-level")
                        && matches!(e.value, EffectValue::Known { .. })));
                }
                other => panic!("{missing}: unrelated preparation failure {other:?}"),
            }
        }
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_OFFERING_INPUTS_RELEASE; finite native component"]
fn offering_inputs_preserve_actual_partial_owner_refusal() {
    let mut w = World::load();
    w.source
        .base
        .inner
        .owner_mut(w.actual_modifier.owner.clone())
        .programs
        .closure = w.actual_modifier.programs.closure.clone();
    let error = w
        .checked_plan()
        .err()
        .expect("actual Partial item family cannot be early-complete");
    assert!(error.contains("complete owner programs"), "{error}");
    let mut w = World::load();
    let owner = w.source.physical_owner.clone();
    w.source.base.inner.owner_mut(owner.owner).programs.closure = owner.programs.closure;
    let error = w
        .checked_plan()
        .err()
        .expect("actual Partial Offering owner remains unresolved");
    assert!(error.contains("complete owner programs"), "{error}");
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_OFFERING_INPUTS_RELEASE; finite native component"]
fn offering_inputs_require_explicit_local_readiness_and_early_item_dependencies() {
    let w = World::load();
    let old = w
        .checked_plan_variant(OWNED_EVALUATION_STAGES_V3, false)
        .err()
        .expect("historical V3 cannot authorize local item preparation writes");
    assert!(old.contains("preparation output role"), "{old}");
    let late = w
        .checked_plan_variant(OWNED_EVALUATION_STAGES_V4, true)
        .err()
        .expect("late item value cannot feed early source inputs");
    assert!(
        late.contains("stage") || late.contains("readiness") || late.contains("phase"),
        "{late}"
    );
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_OFFERING_INPUTS_RELEASE; finite native component"]
fn offering_inputs_reused_scratch_and_rayon_keep_exact_source_values() {
    let a = World::load();
    let mut b = a.clone();
    b.raw(3, 7, 13.0);
    b.snapshot(100.0);
    let ap = a.plan();
    let bp = b.plan();
    let mut scratch = ap.new_scratch();
    let first = ap.evaluate(&mut scratch).unwrap();
    check(&a, &first, [22, 22], [0.0, 0.0]);
    let middle = bp.evaluate(&mut scratch).unwrap();
    check(&b, &middle, [10, 23], [13.0, 0.0]);
    assert_eq!(first, ap.evaluate(&mut scratch).unwrap());
    let plan = Arc::new(ap);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let rows: Vec<_> = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map(|_| plan.evaluate(&mut plan.new_scratch()).unwrap())
            .collect()
    });
    assert!(rows.iter().all(|r| r == &first));
}
