//! Resolve connected, capability-admitted passive selections and physical attribute choices.
//! This records native source resolution, not observations of PoB's Lua object graph.
use poe_optimizer_core::evaluation::{EvaluationError, EvaluationErrorKind};
use poe_optimizer_data::class_tree::{
    ClassTreeSelection, ResolvedClassTree, ResolvedPassiveAllocation,
};
use poe_optimizer_data::tree_data::{
    EffectiveTreeNode, TREE_PATH, TreeAscendancy, TreeClass, TreeSourceIdentity,
};
use poe_optimizer_engine::character::{CharacterAttributes, CharacterInput, CharacterModifiers};
use roxmltree::Node;

pub(crate) struct NativeTree {
    pub character: CharacterInput,
    pub class: TreeClass,
    pub ascendancy: Option<TreeAscendancy>,
    pub allocated_nodes: Vec<u32>,
    paid_node: Option<EffectiveTreeNode>,
    ascendancy_node: Option<EffectiveTreeNode>,
    pub allocation: ResolvedPassiveAllocation,
}
fn unsupported(message: impl Into<String>) -> EvaluationError {
    EvaluationError::new(EvaluationErrorKind::UnsupportedCapability, message)
}
// Data artifacts and numerical source pins evolve independently. Refuse mixed
// revisions or tree data before any document can reach the calculation kernel.
pub(crate) fn validate_calculation_source(
    source: &TreeSourceIdentity,
) -> Result<(), EvaluationError> {
    use poe_optimizer_engine::{UPSTREAM_REVISION, mace, spark};
    let tree_hash = source.source_files_sha256.get(TREE_PATH);
    let matching_tree = [spark::SOURCE_FILES, mace::SOURCE_FILES]
        .into_iter()
        .all(|files| {
            files.iter().any(|file| {
                file.path == TREE_PATH && tree_hash.map(String::as_str) == Some(file.sha256)
            })
        });
    if source.upstream_revision != UPSTREAM_REVISION
        || source.tree_version != spark::TREE_VERSION
        || source.tree_version != mace::TREE_VERSION
        || !matching_tree
    {
        return Err(EvaluationError::new(
            EvaluationErrorKind::BackendContract,
            "Native data and calculation source revisions, tree versions and tree hashes must agree",
        ));
    }
    Ok(())
}
impl NativeTree {
    pub fn resolve(
        build: Node<'_, '_>,
        spec: Node<'_, '_>,
        compiled: &crate::CompiledGameData,
    ) -> Result<Self, EvaluationError> {
        let data = compiled.snapshot().tree();
        validate_calculation_source(&data.source)?;
        let selection = poe_optimizer_import::controlled_build::parse_passive_allocation(
            build,
            spec,
            compiled.snapshot().package(),
        )
        .map_err(|error| unsupported(error.to_string()))?;
        let allocation = selection
            .resolve(compiled.snapshot())
            .map_err(|error| unsupported(error.to_string()))?;
        let character = compiled
            .character_from_allocation(&allocation)
            .map_err(|error| unsupported(error.to_string()))?;
        let legacy = if selection.ordinary_nodes.len() <= 1
            && selection.ascendancy_nodes.len() <= 1
            && selection.attribute_options.is_empty()
        {
            ClassTreeSelection {
                class_id: selection.class_id,
                ascendancy_id: selection.ascendancy_id.clone(),
                entrance_node_id: selection.ordinary_nodes.first().copied(),
                ascendancy_node_id: selection.ascendancy_nodes.first().copied(),
            }
            .resolve(data)
            .ok()
        } else {
            None
        };
        Ok(Self {
            character,
            class: allocation.class.clone(),
            ascendancy: allocation.ascendancy.clone(),
            allocated_nodes: allocation.allocated_nodes.iter().copied().collect(),
            paid_node: legacy.as_ref().and_then(|tree| tree.paid_node.clone()),
            ascendancy_node: legacy
                .as_ref()
                .and_then(|tree| tree.ascendancy_node.clone()),
            allocation,
        })
    }
    pub fn actor_modifiers(
        &self,
    ) -> impl Iterator<Item = &poe_optimizer_data::game_data::ActorModifierRecord> {
        self.allocation
            .views
            .iter()
            .flat_map(|view| &view.actor_modifiers)
    }
    pub fn ascendancy_name(&self) -> &str {
        self.ascendancy
            .as_ref()
            .map_or("None", |asc| asc.name.as_str())
    }
    pub fn diagnostic(&self, compiled: &crate::CompiledGameData) -> serde_json::Value {
        let ascendancy = self.ascendancy.as_ref().map(|asc| {
            serde_json::json!({
                "index":asc.class_index,"internal_id":asc.internal_id,"catalog_id":asc.catalog_id,
                "name":asc.name,"start_node_id":asc.start_node_id,
            })
        });
        let mut paid_nodes: Vec<_> = self.paid_node.iter().map(|node| ("ordinary", node))
            .chain(self.ascendancy_node.iter().map(|node| ("ascendancy", node)))
            .map(|(kind, node)| serde_json::json!({
            "allocation_kind":kind,
            "physical_node_id":node.physical_node_id,"effective_node_id":node.effective_source_id,
            "name":node.name,"stats":node.stats,"override_provenance":node.provenance,
        })).collect();
        if paid_nodes.len() != self.allocation.views.len() {
            paid_nodes = self.allocation.views.iter().map(|view| serde_json::json!({
                "allocation_kind":if self.allocation.selection.ordinary_nodes.contains(&view.source.key.physical_node_id) {"ordinary"} else {"ascendancy"},
                "physical_node_id":view.source.key.physical_node_id,"effective_node_id":view.source.effective_node_id,
                "name":view.source.name,"stats":view.source.stats,"source_view":view.source.key,
                "source_sha256":view.source.source_sha256,
            })).collect();
        }
        serde_json::json!({
            "schema_version":3,
            "class":{"index":self.class.integer_id,"internal_id":self.class.integer_id,
                "source_index":self.class.source_index,"name":self.class.name,"start_node_id":self.class.start_node_id},
            "ascendancy":ascendancy,"allocated_nodes":self.allocated_nodes,
            "ordinary_allocated_count":self.allocation.selection.ordinary_nodes.len(),
            "ascendancy_allocated_count":self.allocation.selection.ascendancy_nodes.len(),"paid_nodes":paid_nodes,
            "attribute_options":self.allocation.selection.attribute_options,
            "source":{"upstream_revision":compiled.snapshot().tree().source.upstream_revision,"tree_version":compiled.snapshot().tree().source.tree_version,
                "bundled_content_sha256":poe_optimizer_data::bundled::content_sha256()},
            "data_identity":compiled.identity(),
            "configured_effects":self.allocation.views.iter().filter_map(|view|
                compiled.snapshot().package().passive_view_effects(&view.source.key)).collect::<Vec<_>>(),
            "point_budget_verified":false,
            "evidence_kind":"native_source_resolution",
            "scope":"connected_capability_admitted_passives_and_explicit_attribute_options",
        })
    }
}
/// Shared native composition used after either strict XML resolution or private
/// catalog admission. The caller supplies a resolved record from the same data
/// snapshot; ownership and physical/effective node resolution are data operations.
pub(crate) fn character_from_resolved(
    resolved: &ResolvedClassTree,
    compiled: &crate::CompiledGameData,
) -> Result<CharacterInput, EvaluationError> {
    validate_calculation_source(&compiled.snapshot().tree().source)?;
    let mut modifiers = CharacterModifiers::default();
    for (owner, node) in resolved.paid_node.iter().map(|node| (None, node)).chain(
        resolved
            .ascendancy_node
            .iter()
            .map(|node| (resolved.selection.ascendancy_id.as_deref(), node)),
    ) {
        let effect = compiled
            .passive_modifiers(resolved.selection.class_id, owner, node.physical_node_id)
            .ok_or_else(|| unsupported("Missing compiled selected passive effects"))?;
        modifiers = modifiers.checked_add(*effect).map_err(|error| {
            unsupported(poe_optimizer_engine::data::GameDataError(error.to_string()).to_string())
        })?;
    }
    Ok(CharacterInput {
        attributes: CharacterAttributes {
            strength: f64::from(resolved.base_attributes.strength),
            dexterity: f64::from(resolved.base_attributes.dexterity),
            intelligence: f64::from(resolved.base_attributes.intelligence),
        },
        modifiers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use poe_optimizer_data::bundled;

    #[test]
    fn bundled_data_cannot_mix_with_another_calculation_pin_or_tree() {
        let source = &bundled::class_tree().unwrap().source;
        validate_calculation_source(source).unwrap();
        let mut wrong_revision = source.clone();
        wrong_revision.upstream_revision = "0".repeat(40);
        let mut wrong_version = source.clone();
        wrong_version.tree_version = "0_4".into();
        let mut wrong_hash = source.clone();
        wrong_hash
            .source_files_sha256
            .insert(TREE_PATH.into(), "0".repeat(64));
        let mut absent_hash = source.clone();
        absent_hash.source_files_sha256.remove(TREE_PATH);
        for altered in [wrong_revision, wrong_version, wrong_hash, absent_hash] {
            assert_eq!(
                validate_calculation_source(&altered).unwrap_err().kind,
                EvaluationErrorKind::BackendContract
            );
        }
    }
}
