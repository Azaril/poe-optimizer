//! Finite contribution fixture. Passive graph legality and all other modifiers
//! remain outside this boundary; production declarations are never closed here.
#![allow(dead_code)]
#[path = "minion_attack_source_fixture.rs"]
pub mod intrinsic;

use poe_optimizer_core::{
    build_identity::InstanceAllocator, owned_binding::*, owned_build::*, owned_definitions::*,
    owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_plan::*;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use std::{fs, path::PathBuf};

pub use intrinsic::{actor_value, def, known, value};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub node: PassiveNodeDefId,
    pub source_id: String,
    pub value: f64,
    pub pool: PointPoolDefId,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bindings {
    pub player_stat: StatDefId,
    pub actor_stat: StatDefId,
    pub unit: UnitDefId,
    pub nodes: Vec<Node>,
}
pub fn asset<T: DeserializeOwned>(name: &str) -> T {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/owned/poe2/3887ae68/plain-minion-damage-passives")
        .join(name);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
pub fn bindings() -> Bindings {
    asset("bindings.json")
}
pub fn selected() -> Vec<Node> {
    let records: Vec<Value> = asset("source-records.json");
    bindings()
        .nodes
        .into_iter()
        .filter(|n| {
            records.iter().any(|r| {
                r["id"].as_u64().unwrap().to_string() == n.source_id && r["allocated"] == true
            })
        })
        .collect()
}
pub fn owners() -> Vec<DefinitionRules> {
    let extension: Value = asset("extension.json");
    serde_json::from_value(extension["owners"].clone()).unwrap()
}

fn finite_node(descriptor: &DefinitionDescriptor) -> DefinitionDescriptor {
    // This unpublished fixture admits the literal contribution independently of
    // graph reachability, node choices and unconverted lines. The CLI publication
    // test retains and compares the complete original schema byte for byte.
    let mut value = serde_json::to_value(descriptor).unwrap();
    value["value"]["schema"]["value"]["adjacent"]["members"] = serde_json::json!([]);
    fn close(value: &mut Value) {
        match value {
            Value::Object(o) => {
                if o.contains_key("members") && o.contains_key("closure") {
                    o.insert("closure".into(), serde_json::json!({"kind":"complete"}));
                }
                o.values_mut().for_each(close);
            }
            Value::Array(a) => a.iter_mut().for_each(close),
            _ => {}
        }
    }
    close(&mut value);
    serde_json::from_value(value).unwrap()
}

pub struct World {
    pub intrinsic: intrinsic::World,
}
impl World {
    pub fn new(nodes: &[Node]) -> Self {
        let mut intrinsic = intrinsic::World::new([22, 1]);
        let dependencies: Vec<DefinitionDescriptor> = asset("dependencies.json");
        for descriptor in dependencies {
            let descriptor = if matches!(descriptor, DefinitionDescriptor::PassiveNode(_)) {
                finite_node(&descriptor)
            } else {
                descriptor
            };
            if let Some(existing) = intrinsic
                .f
                .schema
                .definitions
                .iter()
                .find(|r| r.address() == descriptor.address())
            {
                assert_eq!(existing, &descriptor, "shared dependency changed");
            } else {
                intrinsic.f.schema.definitions.push(descriptor);
            }
        }
        let mut programs = owners();
        for owner in &mut programs {
            assert!(!owner.programs.is_complete());
            owner.programs = DeclaredSet::complete(owner.programs.members.clone());
        }
        programs.extend(asset::<Vec<DefinitionRules>>("dependency-rules.json"));
        for owner in programs {
            assert!(!intrinsic.f.owners.iter().any(|r| r.owner == owner.owner));
            intrinsic.f.owners.push(owner);
        }
        intrinsic
            .f
            .receivers
            .members
            .extend(asset::<Vec<ActorStatReceiver>>("receivers.json"));
        let mut allocator = InstanceAllocator::from_state(intrinsic.f.build.allocator);
        for node in nodes {
            intrinsic.f.build.allocations.push(Allocation {
                id: allocator.allocate().unwrap(),
                node: node.node.clone(),
                pool: node.pool.clone(),
                scope: LoadoutScope::Shared,
                access: AllocationAccess::Ordinary,
                choices: vec![],
            });
        }
        intrinsic.f.build.allocator = allocator.state();
        Self { intrinsic }
    }
    pub fn compile(&self) -> OwnedEffectPlan<OwnedDefinitionSchemaPackage> {
        self.intrinsic.compile()
    }
    pub fn evaluate(&self) -> OwnedEffectsReport {
        self.intrinsic.evaluate()
    }
    pub fn binding(&self) -> DefinitionBindingReport {
        let schema =
            OwnedDefinitionSchemaPackage::new(self.intrinsic.f.schema.clone(), Default::default())
                .unwrap();
        bind_owned_request(&schema, &self.intrinsic.f.request(), Default::default()).unwrap()
    }
}

pub fn check(report: &OwnedEffectsReport, expected: f64) {
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert_eq!(
        known(value(
            report,
            ConcreteEntity::Actor(ActorKey::Player),
            0x1d33
        )),
        expected
    );
    for index in 0..2 {
        assert_eq!(known(actor_value(report, index, 0x1d34)), expected);
    }
    assert!(!report.values.iter().any(|r| {
        matches!(&r.key, PlanValueKey::Stat { entity: ConcreteEntity::Actor(ActorKey::Player), stat } if *stat == def(0x1d34))
    }));
}
