//! Resolve the admitted native class/entrance subset from authenticated portable data.
//! This records native source resolution, not observations of PoB's Lua object graph.
use poe_optimizer_core::evaluation::{EvaluationError, EvaluationErrorKind};
use poe_optimizer_data::{
    bundled,
    tree_data::{EffectiveTreeNode, TreeAscendancy, TreeClass},
};
use poe_optimizer_engine::character::{CharacterAttributes, CharacterInput, CharacterModifiers};
use roxmltree::Node;
use std::collections::BTreeSet;

pub(crate) struct NativeTree {
    pub character: CharacterInput,
    pub class: &'static TreeClass,
    pub ascendancy: Option<&'static TreeAscendancy>,
    pub allocated_nodes: Vec<u32>,
    paid_node: Option<&'static EffectiveTreeNode>,
}
fn unsupported(message: impl Into<String>) -> EvaluationError {
    EvaluationError::new(EvaluationErrorKind::UnsupportedCapability, message)
}
fn integer(node: Node<'_, '_>, field: &str) -> Result<u32, EvaluationError> {
    let text = node
        .attribute(field)
        .ok_or_else(|| unsupported(format!("Native tree requires {field}")))?;
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(unsupported(format!(
            "Native tree {field} must be an unsigned integer"
        )));
    }
    text.parse()
        .map_err(|_| unsupported(format!("Native tree {field} exceeds its integer range")))
}
impl NativeTree {
    pub fn resolve(build: Node<'_, '_>, spec: Node<'_, '_>) -> Result<Self, EvaluationError> {
        let data = bundled::class_tree().map_err(|error| {
            EvaluationError::new(EvaluationErrorKind::BackendContract, error.to_string())
        })?;
        let internal_id = integer(spec, "classInternalId")?;
        let class = data
            .class(internal_id)
            .map_err(|error| unsupported(error.to_string()))?;
        let source_id = integer(spec, "classId")?;
        // PassiveSpec loads the canonical internal ID; older exports may retain
        // the raw data array position in classId. Both must identify this class.
        if (source_id != class.integer_id && source_id != class.source_index)
            || build.attribute("className") != Some(class.name.as_str())
        {
            return Err(unsupported(
                "Native class name/index disagrees with classInternalId",
            ));
        }
        let ascendancy_index = integer(spec, "ascendClassId")?;
        let ascendancy_id = spec.attribute("ascendancyInternalId").unwrap_or("");
        let ascendancy = if ascendancy_index == 0 {
            if !ascendancy_id.is_empty() || build.attribute("ascendClassName") != Some("None") {
                return Err(unsupported(
                    "Native no-ascendancy selection has conflicting identity fields",
                ));
            }
            None
        } else {
            let asc = data
                .ascendancy(internal_id, ascendancy_id)
                .map_err(|error| unsupported(error.to_string()))?;
            if ascendancy_index != asc.class_index
                || build.attribute("ascendClassName") != Some(asc.name.as_str())
            {
                return Err(unsupported(
                    "Native ascendancy name/index disagrees with its internal identity",
                ));
            }
            Some(asc)
        };
        let mut roots = BTreeSet::from([class.start_node_id]);
        if let Some(ascendancy) = ascendancy {
            roots.insert(ascendancy.start_node_id);
        }
        let node_text = spec
            .attribute("nodes")
            .ok_or_else(|| unsupported("Native tree requires explicit nodes"))?;
        let mut requested = BTreeSet::new();
        if !node_text.is_empty() {
            for node in node_text.split(',') {
                if node.is_empty() || !node.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err(unsupported(
                        "Native allocated node IDs must be comma-separated unsigned integers",
                    ));
                }
                let id = node.parse::<u32>().map_err(|_| {
                    unsupported("Native allocated node ID exceeds its integer range")
                })?;
                if !requested.insert(id) {
                    return Err(unsupported("Native allocated node IDs must be unique"));
                }
            }
        }
        let paid_ids: Vec<_> = requested.difference(&roots).copied().collect();
        if paid_ids.len() > 1 {
            return Err(unsupported(
                "Native class/tree profile currently admits zero or one ordinary entrance allocation",
            ));
        }
        let paid_node = paid_ids
            .first()
            .map(|id| {
                data.entrance(internal_id, *id)
                    .map_err(|error| unsupported(error.to_string()))
            })
            .transpose()?;
        let mut modifiers = CharacterModifiers::default();
        if let Some(node) = paid_node {
            for stat in &node.stats {
                apply_stat(&mut modifiers, stat)?;
            }
        }
        requested.extend(roots);
        Ok(Self {
            character: CharacterInput {
                attributes: CharacterAttributes {
                    strength: f64::from(class.base_strength),
                    dexterity: f64::from(class.base_dexterity),
                    intelligence: f64::from(class.base_intelligence),
                },
                modifiers,
            },
            class,
            ascendancy,
            allocated_nodes: requested.into_iter().collect(),
            paid_node,
        })
    }
    pub fn ascendancy_name(&self) -> &str {
        self.ascendancy.map_or("None", |asc| asc.name.as_str())
    }
    pub fn diagnostic(&self) -> serde_json::Value {
        let ascendancy = self.ascendancy.map(|asc| {
            serde_json::json!({
                "index":asc.class_index,"internal_id":asc.internal_id,"catalog_id":asc.catalog_id,
                "name":asc.name,"start_node_id":asc.start_node_id,
            })
        });
        let paid_nodes: Vec<_> = self.paid_node.iter().map(|node| serde_json::json!({
            "physical_node_id":node.physical_node_id,"effective_node_id":node.effective_source_id,
            "name":node.name,"stats":node.stats,"override_provenance":node.provenance,
        })).collect();
        serde_json::json!({
            "schema_version":1,
            "class":{"index":self.class.integer_id,"internal_id":self.class.integer_id,
                "source_index":self.class.source_index,"name":self.class.name,"start_node_id":self.class.start_node_id},
            "ascendancy":ascendancy,"allocated_nodes":self.allocated_nodes,
            "ordinary_allocated_count":paid_nodes.len(),"paid_nodes":paid_nodes,
            "source":{"upstream_revision":poe_optimizer_engine::UPSTREAM_REVISION,"tree_version":"0_5",
                "bundled_content_sha256":bundled::content_sha256()},
            "point_budget_verified":false,
            "evidence_kind":"native_source_resolution",
            "scope":"class_identity_and_zero_or_one_ordinary_entrance",
        })
    }
}
// Exact admitted source lines, resolved before numerical evaluation. New forms
// require parser/source parity and full-build tests before they can be admitted.
fn apply_stat(modifiers: &mut CharacterModifiers, stat: &str) -> Result<(), EvaluationError> {
    match stat {
        "4% increased Skill Speed" => modifiers.skill_speed_increased += 4.0,
        "+8 to Evasion Rating" => modifiers.evasion_flat += 8.0,
        "+16 to Evasion Rating" => modifiers.evasion_flat += 16.0,
        "+5 to maximum Energy Shield" => modifiers.energy_shield_flat += 5.0,
        "+10 to maximum Energy Shield" => modifiers.energy_shield_flat += 10.0,
        "+10 to Armour" => modifiers.armour_flat += 10.0,
        "+20 to Armour" => modifiers.armour_flat += 20.0,
        "10% increased Melee Damage" => modifiers.melee_damage_increased += 10.0,
        "10% increased Projectile Damage" => modifiers.projectile_damage_increased += 10.0,
        "10% increased Spell Damage" => modifiers.spell_damage_increased += 10.0,
        "10% increased Attack Damage" => modifiers.attack_damage_increased += 10.0,
        "Minions deal 10% increased Damage" => modifiers.minion_damage_increased += 10.0,
        _ => {
            return Err(unsupported(format!(
                "Unsupported native ordinary-passive stat: {stat}"
            )));
        }
    }
    Ok(())
}
