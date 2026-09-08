//! Independent source/selection evidence checks for fresh document finalists.
use super::*;
use poe_optimizer_core::{
    coverage::{SkillActor, SkillOrigin, SkillResolution},
    evaluation::{BackendIdentity, BuildFormat, EvaluationResult},
    options::EvaluationOptions,
};
use poe_optimizer_data::class_tree::ClassTreeSelection;
use serde_json::{Value, json};
fn mismatch(message: &str) -> BuildCatalogError {
    BuildCatalogError::Structural(format!("fresh native realization mismatch: {message}"))
}
fn attachment(result: &EvaluationResult, media: &str) -> Result<Value> {
    let mut matching = result.attachments.iter().filter(|a| a.media_type == media);
    let value = matching
        .next()
        .ok_or_else(|| mismatch("required resolved evidence is missing"))?;
    if matching.next().is_some() || value.content.len() > 16 * 1024 * 1024 {
        return Err(mismatch("resolved evidence must be unique and bounded"));
    }
    serde_json::from_str(&value.content).map_err(|_| mismatch("resolved evidence is not JSON"))
}
impl ControlledBuildCatalog {
    /// The expected identity must come from the live selected backend. This checks
    /// its fresh document result against this private admitted selection without
    /// performing an extra calculation or trusting caller-authored attributes.
    pub fn validate_native_realization(
        &self,
        handle: &AdmittedBuildSelection,
        result: &EvaluationResult,
        expected_backend: &BackendIdentity,
    ) -> Result<()> {
        if !self.binding().accepts(handle) {
            return Err(BuildCatalogError::Ownership);
        }
        result
            .validate_recorded()
            .map_err(|_| mismatch("recorded result contract is invalid"))?;
        if &result.backend != expected_backend
            || expected_backend.id != "native-poe2"
            || expected_backend.data.as_ref() != Some(self.compiled.identity())
            || expected_backend.rules_revision
                != self.compiled.snapshot().tree().source.upstream_revision
            || !result.diagnostic_only
        {
            return Err(mismatch(
                "backend, selected data or diagnostic identity changed",
            ));
        }
        if handle.character().is_err() {
            return Err(mismatch(
                "candidate retained an unresolved scalar calculation failure",
            ));
        }
        let expected_xml = self.materialize(handle)?;
        if result.exports.len() != 1
            || result.exports[0].format != BuildFormat::PathOfBuilding2Xml
            || result.exports[0].content != expected_xml.content
        {
            return Err(mismatch(
                "export differs from exact lazy source materialization",
            ));
        }
        let data = self.compiled.snapshot().package();
        let tree = handle.tree();
        let source = &self.source;
        let expected_enemy_level = match source.config()["enemyLevel"] {
            Scalar::Number(value) => value as u32,
            _ => unreachable!("validated source scalar"),
        };
        let mut placeholders: BTreeMap<_, _> = data
            .quests
            .config_keys
            .iter()
            .zip(data.quests.default_enabled)
            .filter(|(name, _)| !source.config().contains_key(*name))
            .map(|(name, value)| (name.clone(), Scalar::Boolean(value)))
            .collect();
        for quest in &data.actor.spirit_quests {
            if !source.config().contains_key(&quest.config_key) {
                placeholders.insert(
                    quest.config_key.clone(),
                    Scalar::Boolean(quest.default_enabled),
                );
            }
        }
        if !source.config().contains_key("resistancePenalty") {
            placeholders.insert(
                "resistancePenalty".into(),
                Scalar::Number(data.encounters.default_resistance_penalty),
            );
        }
        let actor = handle.actor().values();
        let conditions: BTreeMap<_, _> = actor
            .conditions()
            .map(|(key, value)| (key.to_owned(), value))
            .collect();
        if result.context.requested != EvaluationOptions::default()
            || result.context.calculation_mode != "MAIN"
            || result.context.enemy_level != expected_enemy_level
            || result.context.config_inputs != *source.config()
            || result.context.config_placeholders != placeholders
            || result.context.player_conditions != conditions
            || !result.context.enemy_conditions.is_empty()
        {
            return Err(mismatch(
                "fixed encounter/defaults or resolved attribute conditions changed",
            ));
        }
        if result.build.level != source.level()
            || result.build.class_name != tree.class.name
            || result.build.ascendancy_name
                != tree.ascendancy.as_ref().map_or("None", |a| a.name.as_str())
            || result.build.tree_version != data.tree.source.tree_version
            || result.build.main_socket_group != 1
            || result.build.skill_groups != 1
            || result
                .build
                .allocated_nodes
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                != tree.allocated_nodes
            || result.build.allocated_nodes.len() != tree.allocated_nodes.len()
        {
            return Err(mismatch("build class, allocation, level or group changed"));
        }
        self.check_skill_evidence(handle, result)?;
        let media = match source.profile() {
            TemplateProfile::Spark => "application/vnd.poe-optimizer.native-profile+json;version=5",
            TemplateProfile::Mace => "application/vnd.poe-optimizer.native-profile+json;version=7",
        };
        let evidence = attachment(result, media)?;
        let expected_profile = match source.profile() {
            TemplateProfile::Spark => poe_optimizer_engine::spark::PROFILE_ID,
            TemplateProfile::Mace => poe_optimizer_engine::mace::PROFILE_ID,
        };
        if evidence["profile"] != expected_profile {
            return Err(mismatch("selected native numerical profile changed"));
        }
        let equipment: BTreeMap<_, _> = handle
            .selection()
            .candidate
            .equipment
            .iter()
            .map(|(slot, id)| (slot, self.items[id].item.diagnostic()))
            .collect();
        if evidence["equipment"] != json!(equipment)
            || evidence["actor_modifiers"] != source.actor_modifiers().diagnostic()
            || evidence["strength"] != json!(actor.attributes.strength)
            || evidence["dexterity"] != json!(actor.attributes.dexterity)
            || evidence["intelligence"] != json!(actor.attributes.intelligence)
            || evidence["supports_prepared_inputs"] != true
            || evidence["calculation_result_cached"] != false
        {
            return Err(mismatch(
                "equipment source, actor source or resolved attributes changed",
            ));
        }
        let resources = json!({"strength":actor.attributes.strength,"dexterity":actor.attributes.dexterity,"intelligence":actor.attributes.intelligence,
            "life":actor.life,"mana":actor.mana,"spirit":actor.spirit,"accuracy":actor.accuracy,
            "lowest_of_maximum_life_and_maximum_mana":actor.lowest_of_maximum_life_and_maximum_mana,
            "lowest_attribute":actor.lowest_attribute,"total_attributes":actor.total_attributes,
            "low_life_percentage":actor.low_life_percentage,"full_life_percentage":actor.full_life_percentage,
            "life_has_override":actor.life_has_override,"mana_has_override":actor.mana_has_override,"spirit_has_override":actor.spirit_has_override,
            "chaos_inoculation":actor.chaos_inoculation,"full_life_from_chaos_inoculation":actor.full_life_from_chaos_inoculation});
        let receiving = handle
            .actor()
            .receiving()
            .ok_or_else(|| mismatch("candidate receiving preparation is incomplete"))?;
        if evidence["receiving_defence"]
            != crate::actor_assembly::receiving_defence_evidence(receiving)
        {
            return Err(mismatch(
                "fresh receiving defences differ from admitted preparation",
            ));
        }
        if evidence["local_armour"] != self.local_armour_evidence(handle.selection()) {
            return Err(mismatch(
                "fresh local armour differs from selected components",
            ));
        }
        if evidence["actor_resources"] != resources {
            return Err(mismatch(
                "fresh shared actor resources differ from admitted preparation",
            ));
        }
        if source.profile() == TemplateProfile::Mace {
            let weapon = self.items[&handle.selection().candidate.equipment["Weapon 1"]]
                .item
                .weapon()
                .expect("admitted main hand");
            if evidence["weapon_base"] != weapon.base_name()
                || evidence["weapon_quality"] != weapon.quality()
                || evidence["weapon_item_level"] != weapon.item_level()
                || evidence["weapon_item"] != weapon.diagnostic()
                || evidence["support_loadout"] != json!(handle.support_keys())
                || evidence["configured_supports"]
                    != json!(
                        handle
                            .support_keys()
                            .iter()
                            .map(|key| data.support(key).expect("admitted support"))
                            .collect::<Vec<_>>()
                    )
            {
                return Err(mismatch(
                    "weapon source or selected support evidence changed",
                ));
            }
        }
        if attachment(
            result,
            "application/vnd.poe-optimizer.native-tree+json;version=3",
        )? != self.expected_tree_evidence(handle)
        {
            return Err(mismatch(
                "resolved tree view, attribute override or data evidence changed",
            ));
        }
        Ok(())
    }
    fn check_skill_evidence(
        &self,
        handle: &AdmittedBuildSelection,
        result: &EvaluationResult,
    ) -> Result<()> {
        let data = self.compiled.snapshot().package();
        let coverage = &result.coverage;
        if coverage.active_skill_set_id != Some(1)
            || coverage.groups.len() != 1
            || coverage.unresolved_entry_count != 0
            || coverage.selected_minion.is_some()
            || !coverage.tree_connections.is_empty()
        {
            return Err(mismatch(
                "unresolved, disconnected or extra actor/group coverage",
            ));
        }
        let (skill, game, variant) = match self.source.profile() {
            TemplateProfile::Spark => (
                &data.spark.skill_id,
                &data.spark.game_id,
                &data.spark.variant_id,
            ),
            TemplateProfile::Mace => (
                &data.mace.skill_id,
                &data.mace.game_id,
                &data.mace.variant_id,
            ),
        };
        let selected = coverage
            .selected_player
            .as_ref()
            .ok_or_else(|| mismatch("selected player action is missing"))?;
        if selected.skill_id.as_deref() != Some(skill)
            || selected.actor != SkillActor::Player
            || selected.group_index != Some(1)
            || selected.gem_index != Some(1)
            || selected.actor_skill_index != Some(1)
            || selected.synthesized_default_attack
            || selected.minion_id.is_some()
            || selected.part_index.is_some()
            || selected.stat_set_index != Some(1)
            || selected.show_average
        {
            return Err(mismatch("selected action changed or fell back"));
        }
        let group = &coverage.groups[0];
        if group.index != 1
            || !group.enabled
            || !group.include_in_full_dps
            || group.group_count != Some(1.0)
            || group.main_active_skill != Some(1)
            || group.slot.is_some()
            || group.provenance.kind != SkillOrigin::Manual
            || group.provenance.source.is_some()
            || group.provenance.item_id.is_some()
            || group.provenance.item_name.is_some()
            || group.provenance.node_id.is_some()
            || group.gems.len() != 1 + handle.support_keys().len()
        {
            return Err(mismatch("manual skill group changed"));
        }
        let source_supports = if handle.support_keys() == self.source.support_order() {
            self.source.support_order()
        } else {
            handle.support_keys()
        };
        for (index, gem) in group.gems.iter().enumerate() {
            let (skill, game, variant) = if index == 0 {
                (skill, game, variant)
            } else {
                let support = data
                    .support(&source_supports[index - 1])
                    .expect("admitted support");
                (&support.skill_id, &support.game_id, &support.variant_id)
            };
            if gem.index != index + 1
                || !gem.enabled
                || gem.resolution != SkillResolution::ResolvedGem
                || gem.skill_id.as_deref() != Some(skill)
                || gem.gem_game_id.as_deref() != Some(game)
                || gem.variant_id.as_deref() != Some(variant)
                || gem.level != Some(1.0)
                || gem.quality != Some(0.0)
                || gem.count != Some(1.0)
                || gem.is_support != Some(index > 0)
            {
                return Err(mismatch("exact gem identity/configuration changed"));
            }
        }
        Ok(())
    }
    fn expected_tree_evidence(&self, handle: &AdmittedBuildSelection) -> Value {
        let tree = handle.tree();
        let data = self.compiled.snapshot();
        let selection = &tree.selection;
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
            .resolve(data.tree())
            .ok()
        } else {
            None
        };
        let paid_nodes = if let Some(legacy) = legacy {
            legacy.paid_node.iter().map(|node|("ordinary",node)).chain(legacy.ascendancy_node.iter().map(|node|("ascendancy",node)))
                .map(|(kind,node)|json!({"allocation_kind":kind,"physical_node_id":node.physical_node_id,"effective_node_id":node.effective_source_id,
                    "name":node.name,"stats":node.stats,"override_provenance":node.provenance})).collect::<Vec<_>>()
        } else {
            tree.views.iter().map(|view|json!({"allocation_kind":if selection.ordinary_nodes.contains(&view.source.key.physical_node_id){"ordinary"}else{"ascendancy"},
            "physical_node_id":view.source.key.physical_node_id,"effective_node_id":view.source.effective_node_id,"name":view.source.name,"stats":view.source.stats,
            "source_view":view.source.key,"source_sha256":view.source.source_sha256})).collect()
        };
        json!({"schema_version":3,"class":{"index":tree.class.integer_id,"internal_id":tree.class.integer_id,"source_index":tree.class.source_index,"name":tree.class.name,"start_node_id":tree.class.start_node_id},
            "ascendancy":tree.ascendancy.as_ref().map(|asc|json!({"index":asc.class_index,"internal_id":asc.internal_id,"catalog_id":asc.catalog_id,"name":asc.name,"start_node_id":asc.start_node_id})),
            "allocated_nodes":tree.allocated_nodes,"ordinary_allocated_count":selection.ordinary_nodes.len(),"ascendancy_allocated_count":selection.ascendancy_nodes.len(),
            "paid_nodes":paid_nodes,"attribute_options":selection.attribute_options,
            "source":{"upstream_revision":data.tree().source.upstream_revision,"tree_version":data.tree().source.tree_version,"bundled_content_sha256":poe_optimizer_data::bundled::content_sha256()},
            "data_identity":self.compiled.identity(),"configured_effects":tree.views.iter().filter_map(|view|data.package().passive_view_effects(&view.source.key)).collect::<Vec<_>>(),
            "point_budget_verified":false,"evidence_kind":"native_source_resolution","scope":"connected_capability_admitted_passives_and_explicit_attribute_options"})
    }
}
