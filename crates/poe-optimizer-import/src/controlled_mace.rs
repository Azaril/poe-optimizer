//! Source-preserving, bounded weapon/support mutations for a structural Mace profile.
//! Pure Rust source projection shared by native and optional reference evaluators.
//! Experimental diagnostic domain, not a general build-legality claim.
use crate::actor_modifiers::{ValidatedActorModifiers, parse_actor_configuration};
pub use crate::mace_item::ValidatedMaceWeapon;
use crate::mace_item::{parse_mace_item, parse_mace_item_element};
use poe_optimizer_core::{
    candidate::*,
    coverage::{SkillActor, SkillOrigin, SkillResolution},
    data::DataIdentity,
    evaluation::{BackendIdentity, BuildDocument, BuildFormat, EvaluationResult},
    options::{EvaluationContext, EvaluationOptions, Scalar},
};
use poe_optimizer_data::class_tree::{self, ClassTreeSelection, ResolvedClassTree};
use poe_optimizer_data::game_data::{
    self, GameDataPackage, GameDataSnapshot, RequirementData, SupportColor, SupportData,
};
use poe_optimizer_engine::{
    CompiledGameData,
    character::{CharacterAttributes, CharacterInput},
    spark::SparkQuestRewards,
};
use roxmltree::{Document, Node, ParsingOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::sync::Arc;
use thiserror::Error;
/// Rules/data pin for this finite format projection; not a runtime dependency.
pub const PINNED_RULES_REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const SLOT: &str = "pob-group-1";
// Actor source evidence may contain 512 normalized records with eight bounded
// condition tags each. This guard covers their complete ordered diagnostics plus
// weapon/support evidence; source text/record counts remain independently bounded.
pub(crate) const MAX_NATIVE_MACE_PROFILE_BYTES: usize = 8 * 1024 * 1024;
const SOURCE: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaceWeaponAlternative {
    pub id: String,
    pub item_text: String,
}
/// Compatibility name for the supplied weapon payload API; generalized catalogs also
/// admit the explicitly supported rare/local-modifier item grammar.
pub type NormalMaceAlternative = MaceWeaponAlternative;
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaceSupportChoice {
    None,
    BrutalityI,
}
/// Canonical unordered support identities selected from the injected data package.
/// Construction checks shape; catalog construction also checks family and eligibility rules.
#[derive(Debug, Clone, Default, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct MaceSupportLoadout(Vec<String>);
impl MaceSupportLoadout {
    pub fn new(mut keys: Vec<String>) -> Result<Self> {
        if keys.len() > 2
            || keys.iter().any(|key| {
                key.is_empty()
                    || key.len() > 64
                    || !key.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
                    })
            })
        {
            return Err(unsupported(
                "support loadouts require zero to two bounded data keys",
            ));
        }
        keys.sort();
        if keys.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(unsupported("duplicate support in loadout"));
        }
        Ok(Self(keys))
    }
    pub fn keys(&self) -> &[String] {
        &self.0
    }
    pub fn support_ids(&self) -> &[String] {
        &self.0
    }
    pub fn id(&self) -> String {
        if self.0.is_empty() {
            "none".into()
        } else {
            self.0.join("+")
        }
    }
    pub fn legacy_choice(&self) -> Option<MaceSupportChoice> {
        match self.0.as_slice() {
            [] => Some(MaceSupportChoice::None),
            [key] if key == "brutality_i" => Some(MaceSupportChoice::BrutalityI),
            _ => None,
        }
    }
}
impl From<MaceSupportChoice> for MaceSupportLoadout {
    fn from(value: MaceSupportChoice) -> Self {
        Self(match value {
            MaceSupportChoice::None => Vec::new(),
            MaceSupportChoice::BrutalityI => vec!["brutality_i".into()],
        })
    }
}
impl<'de> Deserialize<'de> for MaceSupportLoadout {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        Self::new(Vec::<String>::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct ControlledMaceAlternative {
    pub id: String,
    pub weapon_id: String,
    pub support: MaceSupportLoadout,
    pub tree: ClassTreeSelection,
    pub xml_sha256: String,
    pub candidate: Candidate,
}
/// Closed-profile requirement evidence, independent of diagnostic numerical evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MaceRequirementValues {
    pub level: u32,
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
}
impl MaceRequirementValues {
    fn include(&mut self, other: &RequirementData) {
        self.level = self.level.max(other.level);
        self.strength = self.strength.max(other.attributes.strength);
        self.dexterity = self.dexterity.max(other.attributes.dexterity);
        self.intelligence = self.intelligence.max(other.attributes.intelligence);
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaceRequirementViolation {
    pub requirement: String,
    pub required: u32,
    pub available: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaceRequirementAssessment {
    pub available: MaceRequirementValues,
    pub required: MaceRequirementValues,
    pub violations: Vec<MaceRequirementViolation>,
}
impl MaceRequirementAssessment {
    pub fn is_legal(&self) -> bool {
        self.violations.is_empty()
    }
}
#[derive(Debug, Error)]
pub enum ControlledMutationError {
    #[error("unsupported controlled Mace input: {0}")]
    Unsupported(String),
    #[error("candidate is not in this immutable controlled Mace domain")]
    UnknownCandidate,
    #[error("controlled Mace realization mismatch: {0}")]
    Realization(String),
}
type Result<T> = std::result::Result<T, ControlledMutationError>;
#[derive(Debug)]
struct Profile {
    level: u32,
    config: BTreeMap<String, Scalar>,
    actor_modifiers: Arc<ValidatedActorModifiers>,
    item_range: Range<usize>,
    support_range: Range<usize>,
    extra_support_ranges: Vec<Range<usize>>,
    weapon: String,
    active_attributes: BTreeMap<String, String>,
    support: MaceSupportLoadout,
    support_order: Vec<String>,
    tree: Arc<ResolvedClassTree>,
    tree_attribute_ranges: BTreeMap<String, Range<usize>>,
    ascendancy_insert: usize,
}
/// Unforgeable process-local ownership proof for a single immutable catalog.
/// Clones interoperate; a separately constructed catalog gets a distinct binding,
/// even when its source/data identities are equal.
#[derive(Debug, Clone)]
pub struct NativeMaceBinding(Arc<()>);
#[derive(Debug, Clone, Copy)]
struct NativeMaceIndices {
    weapon: usize,
    tree: usize,
    loadout: usize,
}
#[derive(Debug)]
struct NativeMaceAxes {
    weapons: Vec<ValidatedMaceWeapon>,
    trees: Vec<Arc<ResolvedClassTree>>,
    loadouts: Vec<MaceSupportLoadout>,
    actor_modifiers: Arc<ValidatedActorModifiers>,
    available_attributes: Vec<MaceRequirementValues>,
}
/// Immutable source components admitted by this catalog and its verified native baseline.
/// This stores one template and the sum of axis sizes, never materialized candidate XML.
/// Native consumers still perform their own full template admission once at preparation.
#[derive(Debug, Clone)]
pub struct NativeMaceComponents {
    binding: NativeMaceBinding,
    axes: Arc<NativeMaceAxes>,
    snapshot: Arc<GameDataSnapshot>,
    identity: CatalogIdentity,
    backend: BackendIdentity,
    character_level: u32,
    context: EvaluationContext,
    template: Arc<str>,
}
impl NativeMaceComponents {
    pub fn binding(&self) -> &NativeMaceBinding {
        &self.binding
    }
    pub fn snapshot(&self) -> &Arc<GameDataSnapshot> {
        &self.snapshot
    }
    pub fn data_identity(&self) -> &DataIdentity {
        self.snapshot.identity()
    }
    pub fn catalog_identity(&self) -> &CatalogIdentity {
        &self.identity
    }
    pub fn catalog_fingerprint(&self) -> &str {
        &self.identity.content_fingerprint
    }
    pub fn backend_identity(&self) -> &BackendIdentity {
        &self.backend
    }
    pub fn character_level(&self) -> u32 {
        self.character_level
    }
    pub fn context(&self) -> &EvaluationContext {
        &self.context
    }
    pub fn config(&self) -> &BTreeMap<String, Scalar> {
        &self.context.config_inputs
    }
    pub fn template_build(&self) -> BuildDocument {
        BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: self.template.to_string(),
        }
    }
    pub fn weapons(&self) -> &[ValidatedMaceWeapon] {
        &self.axes.weapons
    }
    pub fn trees(&self) -> &[Arc<ResolvedClassTree>] {
        &self.axes.trees
    }
    pub fn loadouts(&self) -> &[MaceSupportLoadout] {
        &self.axes.loadouts
    }
    pub fn actor_modifiers(&self) -> &ValidatedActorModifiers {
        &self.axes.actor_modifiers
    }
    /// Counts in weapon/tree/loadout order. These count source components, not heap bytes.
    pub fn axis_counts(&self) -> [usize; 3] {
        [
            self.axes.weapons.len(),
            self.axes.trees.len(),
            self.axes.loadouts.len(),
        ]
    }
}
/// Exact registered candidate with requirements checked against its selected dataset.
/// Neither callers nor deserializers can manufacture or mutate this handle.
#[derive(Debug, Clone)]
pub struct NativeMaceCandidate {
    binding: NativeMaceBinding,
    indices: NativeMaceIndices,
}
impl NativeMaceCandidate {
    pub fn weapon_index(&self) -> usize {
        self.indices.weapon
    }
    pub fn tree_index(&self) -> usize {
        self.indices.tree
    }
    pub fn loadout_index(&self) -> usize {
        self.indices.loadout
    }
    pub fn belongs_to(&self, components: &NativeMaceComponents) -> bool {
        self.belongs_to_binding(components.binding())
    }
    pub fn belongs_to_binding(&self, binding: &NativeMaceBinding) -> bool {
        Arc::ptr_eq(&self.binding.0, &binding.0)
    }
}
#[derive(Debug, Clone)]
struct Choice {
    native_indices: Option<NativeMaceIndices>,
    weapon: String,
    support: MaceSupportLoadout,
    support_order: Vec<String>,
    tree: Arc<ResolvedClassTree>,
    patch_tree: bool,
}
/// Fresh validated template evidence; baseline-derived, not an independent golden.
#[derive(Debug)]
pub struct VerifiedMaceScenario {
    identity: CatalogIdentity,
    context: EvaluationContext,
    backend: BackendIdentity,
    exported_scenario: String,
}
/// Fresh native template evidence, bound to caller-authenticated backend identity.
/// This establishes this finite source projection, not complete game legality.
#[derive(Debug)]
pub struct VerifiedNativeMaceScenario {
    identity: CatalogIdentity,
    backend: BackendIdentity,
    context: EvaluationContext,
}
#[derive(Debug)]
pub struct ControlledMaceCatalog {
    data: Arc<GameDataSnapshot>,
    compiled: CompiledGameData,
    support_xml: BTreeMap<String, String>,
    template: String,
    profile: Profile,
    catalog: CandidateCatalog,
    alternatives: Vec<ControlledMaceAlternative>,
    choices: BTreeMap<Candidate, Choice>,
    native_binding: NativeMaceBinding,
    native_axes: Arc<NativeMaceAxes>,
    tree_choices: Vec<ClassTreeSelection>,
    candidate_index:
        BTreeMap<ClassTreeSelection, BTreeMap<String, BTreeMap<MaceSupportLoadout, usize>>>,
}
impl ControlledMaceCatalog {
    pub fn new(
        template_xml: String,
        weapons: Vec<NormalMaceAlternative>,
        supports: Vec<MaceSupportChoice>,
    ) -> Result<Self> {
        let data =
            game_data::bundled_snapshot().map_err(|error| unsupported(&error.to_string()))?;
        Self::with_data(Arc::new(data), template_xml, weapons, supports)
    }
    /// Retain one immutable selected dataset for materialization, identity and legality.
    /// The admitted structure remains Warrior without an ascendancy or paid passives.
    pub fn with_data(
        data: Arc<GameDataSnapshot>,
        template_xml: String,
        weapons: Vec<NormalMaceAlternative>,
        supports: Vec<MaceSupportChoice>,
    ) -> Result<Self> {
        Self::with_loadouts(
            data,
            template_xml,
            weapons,
            supports.into_iter().map(Into::into).collect(),
        )
    }
    /// Compose finite class/ascendancy/ordinary-entrance/ascendancy-passive choices with equipment and support.
    /// The snapshot is shared with requirements and evaluator admission; no Lua runs here.
    pub fn with_tree_choices(
        data: Arc<GameDataSnapshot>,
        template_xml: String,
        weapons: Vec<NormalMaceAlternative>,
        supports: Vec<MaceSupportChoice>,
        tree_choices: Vec<ClassTreeSelection>,
    ) -> Result<Self> {
        Self::with_tree_loadouts(
            data,
            template_xml,
            weapons,
            supports.into_iter().map(Into::into).collect(),
            tree_choices,
        )
    }
    pub fn with_loadouts(
        data: Arc<GameDataSnapshot>,
        template_xml: String,
        weapons: Vec<NormalMaceAlternative>,
        supports: Vec<MaceSupportLoadout>,
    ) -> Result<Self> {
        Self::build(data, template_xml, weapons, supports, None)
    }
    pub fn with_tree_loadouts(
        data: Arc<GameDataSnapshot>,
        template_xml: String,
        weapons: Vec<NormalMaceAlternative>,
        supports: Vec<MaceSupportLoadout>,
        tree_choices: Vec<ClassTreeSelection>,
    ) -> Result<Self> {
        Self::build(data, template_xml, weapons, supports, Some(tree_choices))
    }
    fn build(
        data: Arc<GameDataSnapshot>,
        template_xml: String,
        weapons: Vec<NormalMaceAlternative>,
        supports: Vec<MaceSupportLoadout>,
        requested_trees: Option<Vec<ClassTreeSelection>>,
    ) -> Result<Self> {
        let package = data.package();
        if package
            .quests
            .config_keys
            .iter()
            .map(String::as_str)
            .chain(
                package
                    .actor
                    .spirit_quests
                    .iter()
                    .map(|quest| quest.config_key.as_str()),
            )
            .any(|key| {
                INPUT_NAMES.contains(&key) || ["resistancePenalty", "customMods"].contains(&key)
            })
        {
            return Err(unsupported(
                "selected quest keys overlap controlled encounter configuration",
            ));
        }
        let profile = profile(&template_xml, package)?;
        let support_xml: BTreeMap<_, _> = package
            .supports
            .iter()
            .map(|gem| (gem.id.clone(), support_xml(gem)))
            .collect();
        let patch_tree = requested_trees.is_some();
        if !patch_tree && profile.tree.selection != fixed_warrior() {
            return Err(unsupported(
                "fixed catalog requires Warrior without ascendancy or paid passives",
            ));
        }
        let mut tree_choices = requested_trees.unwrap_or_else(|| vec![fixed_warrior()]);
        if tree_choices.is_empty() || tree_choices.len() > 105 {
            return Err(unsupported("provide 1..105 distinct tree choices"));
        }
        tree_choices.sort();
        if tree_choices.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(unsupported("duplicate tree alternatives"));
        }
        let resolved_trees = tree_choices
            .iter()
            .map(|selection| {
                selection
                    .resolve(&package.tree)
                    .map(Arc::new)
                    .map_err(|error| unsupported(&error.to_string()))
            })
            .collect::<Result<Vec<_>>>()?;
        if weapons.is_empty() || weapons.len() > 64 || supports.is_empty() || supports.len() > 7 {
            return Err(unsupported(
                "provide 1..64 weapons and 1..7 distinct support loadouts",
            ));
        }
        let mut ids = BTreeSet::new();
        let mut payloads = BTreeSet::new();
        let mut parsed = Vec::new();
        let mut typed_weapons = BTreeMap::new();
        for weapon in weapons {
            if weapon.id.trim().is_empty()
                || weapon.id.len() > 128
                || !ids.insert(weapon.id.clone())
            {
                return Err(unsupported(
                    "weapon IDs must be distinct nonblank strings of at most 128 bytes",
                ));
            }
            let typed = parse_mace_item(&weapon.item_text, package)
                .map_err(|error| unsupported(error.to_string().as_str()))?;
            let text = typed.source_text().to_owned();
            typed_weapons.insert(weapon.id.clone(), typed);
            if !payloads.insert(text.clone()) {
                return Err(unsupported("duplicate exact weapon alternatives"));
            }
            parsed.push((weapon.id, text));
        }
        let support_set: BTreeSet<_> = supports.iter().cloned().collect();
        if support_set.len() != supports.len() {
            return Err(unsupported("duplicate support alternatives"));
        }
        for support in &support_set {
            package
                .validate_mace_support_loadout(support.keys())
                .map_err(|error| unsupported(&error.to_string()))?;
        }
        parsed.sort();
        let preparations = parsed
            .len()
            .checked_mul(support_set.len())
            .and_then(|count| count.checked_mul(tree_choices.len()))
            .and_then(|count| count.checked_mul(template_xml.len().saturating_add(4096)));
        if preparations.is_none_or(|bytes| bytes > 256 * 1024 * 1024) {
            return Err(unsupported(
                "controlled materialization hashing exceeds 256 MiB preparation budget",
            ));
        }
        let graph = class_tree::candidate_catalog(&data)
            .map_err(|error| unsupported(&error.to_string()))?;
        let identity = CatalogIdentity {
            schema_version: 1,
            game: "path_of_exile_2".into(),
            rules_revision: PINNED_RULES_REVISION.into(),
            content_fingerprint: hash(
                &if patch_tree {
                    serde_json::to_string(&(
                        "pob-controlled-mace-v7",
                        data.identity(),
                        SOURCE,
                        hash(&template_xml),
                        &parsed,
                        &support_set,
                        &tree_choices,
                        &graph.identity,
                    ))
                } else {
                    serde_json::to_string(&(
                        "pob-controlled-mace-v5",
                        data.identity(),
                        SOURCE,
                        hash(&template_xml),
                        &parsed,
                        &support_set,
                    ))
                }
                .map_err(|error| unsupported(&error.to_string()))?,
            ),
        };
        let active_payload = payload(
            "pob2-gem-attributes-json-v1",
            serde_json::to_string(&profile.active_attributes).unwrap(),
        );
        let active_id = format!("pob-gem:{}", active_payload.sha256);
        let mut support_instances = BTreeMap::new();
        for gem in &package.supports {
            let document = Document::parse(&support_xml[&gem.id])
                .map_err(|error| unsupported(&error.to_string()))?;
            let gem_payload = payload(
                "pob2-gem-attributes-json-v1",
                serde_json::to_string(&attributes(document.root_element())).unwrap(),
            );
            let id = format!("pob-gem:{}", gem_payload.sha256);
            support_instances.insert(gem.id.clone(), (id, gem_payload));
        }
        let legacy_root = profile.tree.class.start_node_id;
        let mut catalog = CandidateCatalog {
            identity: identity.clone(),
            classes: if patch_tree {
                graph.classes
            } else {
                BTreeMap::from([(
                    "6".into(),
                    ClassDefinition {
                        start_node_id: legacy_root,
                    },
                )])
            },
            ascendancies: if patch_tree {
                graph.ascendancies
            } else {
                BTreeMap::new()
            },
            passive_nodes: if patch_tree {
                graph.passive_nodes
            } else {
                BTreeMap::from([(
                    legacy_root,
                    PassiveNode {
                        kind: PassiveKind::ClassStart {
                            class_ids: BTreeSet::from(["6".into()]),
                        },
                        ..Default::default()
                    },
                )])
            },
            equipment_slots: BTreeSet::from(["Weapon 1".into()]),
            skill_slots: BTreeSet::from([SLOT.into()]),
            items: BTreeMap::new(),
            active_skills: BTreeMap::from([(
                active_id.clone(),
                ActiveSkillInstance {
                    definition_id: package.mace.skill_id.clone(),
                    payload: active_payload,
                    ..Default::default()
                },
            )]),
            supports: BTreeMap::new(),
            support_definition_limits: BTreeMap::new(),
            unsupported_mechanics: BTreeSet::new(),
        };
        for key in support_set
            .iter()
            .flat_map(|loadout| loadout.keys())
            .collect::<BTreeSet<_>>()
        {
            let gem = package.support(key).expect("validated support");
            let (support_id, support_payload) = &support_instances[key];
            catalog.supports.insert(
                support_id.clone(),
                SupportInstance {
                    definition_id: gem.skill_id.clone(),
                    payload: support_payload.clone(),
                    compatible_active_skill_ids: BTreeSet::from([package.mace.skill_id.clone()]),
                    ..Default::default()
                },
            );
            catalog
                .support_definition_limits
                .insert(gem.skill_id.clone(), 1);
        }
        let mut choices = BTreeMap::new();
        let mut alternatives = Vec::new();
        let mut hashed_bytes = 0usize;
        let mut candidate_index: BTreeMap<
            ClassTreeSelection,
            BTreeMap<String, BTreeMap<MaceSupportLoadout, usize>>,
        > = BTreeMap::new();
        let compiled = CompiledGameData::compile(Arc::clone(&data))
            .map_err(|e| unsupported(&format!("actor requirement data: {e}")))?;
        let available_attributes = resolved_trees
            .iter()
            .map(|tree| actor_requirement_values(&compiled, &profile, tree))
            .collect::<Result<Vec<_>>>()?;
        let native_axes = Arc::new(NativeMaceAxes {
            weapons: parsed
                .iter()
                .map(|(id, _)| typed_weapons.remove(id).expect("parsed weapon"))
                .collect(),
            trees: resolved_trees.clone(),
            loadouts: support_set.iter().cloned().collect(),
            actor_modifiers: Arc::clone(&profile.actor_modifiers),
            available_attributes,
        });
        for (weapon_index, (id, weapon)) in parsed.into_iter().enumerate() {
            let item_payload = payload("pob2-item-text-v1", weapon.clone());
            let item_id = format!("pob-item-1:{}", item_payload.sha256);
            catalog.items.insert(
                item_id.clone(),
                ItemInstance {
                    definition_id: native_axes.weapons[weapon_index].base_name().into(),
                    payload: item_payload,
                    compatible_slots: BTreeSet::from(["Weapon 1".into()]),
                    ..Default::default()
                },
            );
            for (loadout_index, support) in support_set.iter().enumerate() {
                for (tree_index, tree) in resolved_trees.iter().enumerate() {
                    let candidate = Candidate {
                        catalog: identity.clone(),
                        class_id: tree.class.integer_id.to_string(),
                        ascendancy_id: tree.ascendancy.as_ref().map(|asc| asc.internal_id.clone()),
                        passives: tree
                            .selection
                            .entrance_node_id
                            .into_iter()
                            .chain(tree.selection.ascendancy_node_id)
                            .collect(),
                        equipment: BTreeMap::from([("Weapon 1".into(), item_id.clone())]),
                        skills: BTreeMap::from([(
                            SLOT.into(),
                            SkillAssignment {
                                active_instance_id: active_id.clone(),
                                support_instance_ids: support
                                    .keys()
                                    .iter()
                                    .map(|key| support_instances[key].0.clone())
                                    .collect(),
                            },
                        )]),
                    };
                    let choice = Choice {
                        native_indices: Some(NativeMaceIndices {
                            weapon: weapon_index,
                            tree: tree_index,
                            loadout: loadout_index,
                        }),
                        weapon: weapon.clone(),
                        support: support.clone(),
                        support_order: support.keys().to_vec(),
                        tree: tree.clone(),
                        patch_tree,
                    };
                    let xml = patch(&template_xml, &profile, &choice, &support_xml);
                    hashed_bytes = hashed_bytes.checked_add(xml.len()).ok_or_else(|| {
                        unsupported(
                            "controlled materialization hashing exceeds 256 MiB preparation budget",
                        )
                    })?;
                    if hashed_bytes > 256 * 1024 * 1024 {
                        return Err(unsupported(
                            "controlled materialization hashing exceeds 256 MiB preparation budget",
                        ));
                    }
                    candidate_index
                        .entry(tree.selection.clone())
                        .or_default()
                        .entry(id.clone())
                        .or_default()
                        .insert(support.clone(), alternatives.len());
                    alternatives.push(ControlledMaceAlternative {
                        id: format!(
                            "{}{id}/{}",
                            if patch_tree {
                                format!(
                                    "class/{}/asc/{}/entrance/{}/{ascendancy_passive}",
                                    tree.selection.class_id,
                                    tree.selection.ascendancy_id.as_deref().unwrap_or("none"),
                                    tree.selection
                                        .entrance_node_id
                                        .map_or_else(|| "none".into(), |id| id.to_string()),
                                    ascendancy_passive = tree
                                        .selection
                                        .ascendancy_node_id
                                        .map_or_else(String::new, |id| format!(
                                            "ascendancy-passive/{id}/"
                                        ))
                                )
                            } else {
                                String::new()
                            },
                            support.id()
                        ),
                        weapon_id: id.clone(),
                        support: support.clone(),
                        tree: tree.selection.clone(),
                        xml_sha256: hash(&xml),
                        candidate: candidate.clone(),
                    });
                    choices.insert(candidate, choice);
                }
            }
        }
        Ok(Self {
            data,
            compiled,
            support_xml,
            template: template_xml,
            profile,
            catalog,
            alternatives,
            choices,
            native_binding: NativeMaceBinding(Arc::new(())),
            native_axes,
            tree_choices,
            candidate_index,
        })
    }
    /// Authored actor configuration requires an explicit CLI problem scope even when disabled.
    /// This legacy weapon catalog admits local-only items; only enabled authored
    /// configuration records can require the new receiving source grammar.
    pub fn uses_action_speed_scope(&self) -> bool {
        self.profile.actor_modifiers.uses_action_speed()
    }
    pub fn uses_movement_scope(&self) -> bool {
        self.profile.actor_modifiers.uses_movement()
    }
    pub fn uses_receiving_defence_scope(&self) -> bool {
        self.profile.actor_modifiers.uses_receiving_defence()
    }
    pub fn uses_extended_actor_scope(&self) -> bool {
        self.profile.actor_modifiers.uses_extended_scope()
            || self
                .data
                .package()
                .actor
                .spirit_quests
                .iter()
                .any(|quest| self.profile.config.contains_key(&quest.config_key))
    }
    /// True when the template or any supplied weapon needs the expanded item grammar.
    pub fn uses_extended_weapon_scope(&self) -> bool {
        !parse_mace_item(&self.profile.weapon, self.data.package())
            .expect("admitted template weapon")
            .is_legacy_normal_payload()
            || self
                .native_axes
                .weapons
                .iter()
                .any(|weapon| !weapon.is_legacy_normal_payload())
    }
    pub fn snapshot(&self) -> &Arc<GameDataSnapshot> {
        &self.data
    }
    pub fn data_identity(&self) -> &DataIdentity {
        self.data.identity()
    }
    pub fn required_skill_id(&self) -> &str {
        &self.data.package().mace.skill_id
    }
    pub fn catalog(&self) -> &CandidateCatalog {
        &self.catalog
    }
    pub fn alternatives(&self) -> &[ControlledMaceAlternative] {
        &self.alternatives
    }
    pub fn tree_choices(&self) -> &[ClassTreeSelection] {
        &self.tree_choices
    }
    pub fn resolve_tree_candidate(
        &self,
        tree: &ClassTreeSelection,
        weapon_id: &str,
        support: MaceSupportChoice,
    ) -> Option<&Candidate> {
        self.resolve_tree_loadout_candidate(tree, weapon_id, &support.into())
    }
    pub fn resolve_tree_loadout_candidate(
        &self,
        tree: &ClassTreeSelection,
        weapon_id: &str,
        support: &MaceSupportLoadout,
    ) -> Option<&Candidate> {
        let index = self
            .candidate_index
            .get(tree)?
            .get(weapon_id)?
            .get(support)?;
        self.alternatives.get(*index).map(|value| &value.candidate)
    }
    /// Resolve a legacy support choice under the imported template's tree identity.
    pub fn resolve_candidate(
        &self,
        weapon_id: &str,
        support: MaceSupportChoice,
    ) -> Option<&Candidate> {
        self.resolve_loadout_candidate(weapon_id, &support.into())
    }
    pub fn resolve_loadout_candidate(
        &self,
        weapon_id: &str,
        support: &MaceSupportLoadout,
    ) -> Option<&Candidate> {
        self.resolve_tree_loadout_candidate(&self.profile.tree.selection, weapon_id, support)
    }
    /// Admitted passive records do not change attributes; equipment dependencies are excluded.
    /// Aggregate support-color costs compete with individual requirements by maximum;
    /// they are never added to weapon or active-gem requirements.
    pub fn requirements(&self, candidate: &Candidate) -> Result<MaceRequirementAssessment> {
        let choice = self
            .choices
            .get(candidate)
            .ok_or(ControlledMutationError::UnknownCandidate)?;
        let data = self.data.package();
        let available = self.native_axes.available_attributes
            [choice.native_indices.expect("registered choice").tree];
        let mut required = MaceRequirementValues {
            level: 0,
            strength: 0,
            dexterity: 0,
            intelligence: 0,
        };
        let selected_weapon =
            &self.native_axes.weapons[choice.native_indices.expect("registered choice").weapon];
        let weapon_key = selected_weapon.weapon_key();
        let weapon = data
            .weapons
            .iter()
            .find(|weapon| weapon.id == weapon_key)
            .expect("validated selected weapon");
        required.include(&weapon.requirements);
        // Authored equip level replaces the base level on PoB's non-unique item path;
        // item level never sets a character requirement.
        required.level = selected_weapon.effective_level_requirement();
        required.include(&data.mace.requirements);
        let costs = &data.mace.support_attribute_costs;
        let mut support_costs = MaceRequirementValues {
            level: 0,
            strength: 0,
            dexterity: 0,
            intelligence: 0,
        };
        for key in choice.support.keys() {
            let gem = data.support(key).expect("validated selected support");
            required.include(&gem.requirements);
            match gem.color {
                SupportColor::Red => support_costs.strength += costs.strength,
                SupportColor::Green => support_costs.dexterity += costs.dexterity,
                SupportColor::Blue => support_costs.intelligence += costs.intelligence,
            }
        }
        required.strength = required.strength.max(support_costs.strength);
        required.dexterity = required.dexterity.max(support_costs.dexterity);
        required.intelligence = required.intelligence.max(support_costs.intelligence);
        let mut violations = Vec::new();
        for (name, required, available) in [
            ("level", required.level, available.level),
            ("strength", required.strength, available.strength),
            ("dexterity", required.dexterity, available.dexterity),
            (
                "intelligence",
                required.intelligence,
                available.intelligence,
            ),
        ] {
            if required > available {
                violations.push(MaceRequirementViolation {
                    requirement: name.into(),
                    required,
                    available,
                });
            }
        }
        Ok(MaceRequirementAssessment {
            available,
            required,
            violations,
        })
    }
    pub fn validate_requirements(&self, candidate: &Candidate) -> Result<()> {
        let assessment = self.requirements(candidate)?;
        if assessment.is_legal() {
            return Ok(());
        }
        let reasons = assessment
            .violations
            .iter()
            .map(|violation| {
                format!(
                    "{} requires {}, available {}",
                    violation.requirement, violation.required, violation.available
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        Err(unsupported(&format!(
            "controlled candidate requirements not met: {reasons}"
        )))
    }
    pub fn template_build(&self) -> BuildDocument {
        BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: self.template.clone(),
        }
    }
    pub fn materialize(&self, candidate: &Candidate) -> Result<BuildDocument> {
        let choice = self
            .choices
            .get(candidate)
            .ok_or(ControlledMutationError::UnknownCandidate)?;
        Ok(BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: patch(&self.template, &self.profile, choice, &self.support_xml),
        })
    }
    /// Caller budgets this fresh template attempt; it does not replace final verification.
    pub fn bind_baseline(&self, result: &EvaluationResult) -> Result<VerifiedMaceScenario> {
        let choice = Choice {
            native_indices: None,
            weapon: self.profile.weapon.clone(),
            tree: self.profile.tree.clone(),
            patch_tree: false,
            support: self.profile.support.clone(),
            support_order: self.profile.support_order.clone(),
        };
        self.check_realization(&choice, result)?;
        Ok(VerifiedMaceScenario {
            identity: self.catalog.identity.clone(),
            context: result.context.clone(),
            backend: result.backend.clone(),
            exported_scenario: exported_scenario(&result.exports[0].content, self.data.package())?,
        })
    }
    pub fn validate_realization(
        &self,
        candidate: &Candidate,
        result: &EvaluationResult,
        scenario: &VerifiedMaceScenario,
    ) -> Result<()> {
        let choice = self
            .choices
            .get(candidate)
            .ok_or(ControlledMutationError::UnknownCandidate)?;
        if scenario.identity != self.catalog.identity {
            return Err(mismatch("baseline belongs to another catalog"));
        }
        self.check_realization(choice, result)?;
        let expected = &scenario.context;
        let actual = &result.context;
        if result.backend != scenario.backend
            || actual.requested != expected.requested
            || actual.calculation_mode != expected.calculation_mode
            || actual.enemy_level != expected.enemy_level
            || actual.config_inputs != expected.config_inputs
            || actual.config_placeholders != expected.config_placeholders
        {
            return Err(mismatch(
                "effective immutable scenario changed from the verified template",
            ));
        }
        let signature = exported_scenario(&result.exports[0].content, self.data.package())?;
        if signature != scenario.exported_scenario {
            let at = signature
                .chars()
                .zip(scenario.exported_scenario.chars())
                .position(|(a, b)| a != b)
                .unwrap_or(0);
            return Err(mismatch(&format!(
                "exported scenario changed at {at}: actual {:?}; expected {:?}",
                signature.chars().skip(at).take(160).collect::<String>(),
                scenario
                    .exported_scenario
                    .chars()
                    .skip(at)
                    .take(160)
                    .collect::<String>()
            )));
        }
        Ok(())
    }
    /// Validate a freshly evaluated native template without PoB-normalized export assumptions.
    /// `expected_backend` must come from the actual selected backend implementation, not the result.
    pub fn bind_native_baseline(
        &self,
        result: &EvaluationResult,
        expected_backend: &BackendIdentity,
    ) -> Result<VerifiedNativeMaceScenario> {
        if !same_backend(&result.backend, expected_backend) {
            return Err(mismatch(
                "native baseline backend identity differs from the selected implementation",
            ));
        }
        if expected_backend.data.as_ref() != Some(self.data.identity()) {
            return Err(mismatch(
                "native backend data differs from the selected controlled catalog",
            ));
        }
        let choice = Choice {
            native_indices: None,
            weapon: self.profile.weapon.clone(),
            tree: self.profile.tree.clone(),
            patch_tree: false,
            support: self.profile.support.clone(),
            support_order: self.profile.support_order.clone(),
        };
        self.check_native_realization(&choice, result, &self.template)?;
        Ok(VerifiedNativeMaceScenario {
            identity: self.catalog.identity.clone(),
            backend: expected_backend.clone(),
            context: result.context.clone(),
        })
    }
    /// Share the validated source axes after a fresh native baseline. Backend and data
    /// identities must come from the selected implementation, never from a caller-edited result.
    pub fn native_components(
        &self,
        scenario: &VerifiedNativeMaceScenario,
        expected_backend: &BackendIdentity,
    ) -> Result<NativeMaceComponents> {
        if scenario.identity != self.catalog.identity
            || !same_backend(&scenario.backend, expected_backend)
            || expected_backend.data.as_ref() != Some(self.data.identity())
            || scenario.context.config_inputs != self.profile.config
        {
            return Err(mismatch(
                "native components belong to another catalog, backend or scenario",
            ));
        }
        Ok(NativeMaceComponents {
            binding: self.native_binding.clone(),
            axes: self.native_axes.clone(),
            snapshot: self.data.clone(),
            identity: self.catalog.identity.clone(),
            backend: expected_backend.clone(),
            character_level: self.profile.level,
            context: scenario.context.clone(),
            template: Arc::from(self.template.as_str()),
        })
    }
    /// Resolve exact immutable membership and existing requirement rules once before
    /// the hot loop. Source parsing, class ownership and loadout eligibility were checked
    /// when building the catalog; no second numerical/legality model is introduced here.
    pub fn validated_native_candidate(
        &self,
        candidate: &Candidate,
        components: &NativeMaceComponents,
    ) -> Result<NativeMaceCandidate> {
        if !Arc::ptr_eq(&self.native_binding.0, &components.binding.0) {
            return Err(mismatch(
                "native components belong to another catalog instance",
            ));
        }
        let choice = self
            .choices
            .get(candidate)
            .ok_or(ControlledMutationError::UnknownCandidate)?;
        self.validate_requirements(candidate)?;
        Ok(NativeMaceCandidate {
            binding: self.native_binding.clone(),
            indices: choice.native_indices.expect("registered choice"),
        })
    }
    /// Check the exact requested materialization, resolved item/skill evidence and fixed encounter.
    pub fn validate_native_realization(
        &self,
        candidate: &Candidate,
        result: &EvaluationResult,
        scenario: &VerifiedNativeMaceScenario,
    ) -> Result<()> {
        let choice = self
            .choices
            .get(candidate)
            .ok_or(ControlledMutationError::UnknownCandidate)?;
        if scenario.identity != self.catalog.identity
            || !same_backend(&result.backend, &scenario.backend)
        {
            return Err(mismatch(
                "native scenario belongs to another catalog or backend",
            ));
        }
        self.check_native_realization(choice, result, &self.materialize(candidate)?.content)?;
        let expected = &scenario.context;
        let actual = &result.context;
        // Conditions may derive from the selected candidate; only external encounter inputs are frozen.
        if actual.requested != expected.requested
            || actual.calculation_mode != expected.calculation_mode
            || actual.enemy_level != expected.enemy_level
            || actual.config_inputs != expected.config_inputs
            || actual.config_placeholders != expected.config_placeholders
        {
            return Err(mismatch("native immutable configured scenario changed"));
        }
        Ok(())
    }
    fn check_native_realization(
        &self,
        choice: &Choice,
        result: &EvaluationResult,
        expected_xml: &str,
    ) -> Result<()> {
        result
            .validate_recorded()
            .map_err(|error| mismatch(&error.to_string()))?;
        if result.backend.id != "native-poe2"
            || result.backend.rules_revision != PINNED_RULES_REVISION
            || result.backend.data.as_ref() != Some(self.data.identity())
            || !result.diagnostic_only
        {
            return Err(mismatch(
                "wrong native backend identity or missing diagnostic marker",
            ));
        }
        self.check_common_realization(choice, result)?;
        if result.context.config_inputs != self.profile.config {
            return Err(mismatch(
                "native configured inputs differ from the exact source profile",
            ));
        }
        let mut expected_defaults: BTreeMap<String, Scalar> = self
            .data
            .package()
            .quests
            .config_keys
            .iter()
            .zip(self.data.package().quests.default_enabled)
            .filter(|(name, _)| !self.profile.config.contains_key(*name))
            .map(|(name, enabled)| (name.clone(), Scalar::Boolean(enabled)))
            .collect();
        for quest in &self.data.package().actor.spirit_quests {
            if !self.profile.config.contains_key(&quest.config_key) {
                expected_defaults.insert(
                    quest.config_key.clone(),
                    Scalar::Boolean(quest.default_enabled),
                );
            }
        }
        if !self.profile.config.contains_key("resistancePenalty") {
            expected_defaults.insert(
                "resistancePenalty".into(),
                Scalar::Number(self.data.package().encounters.default_resistance_penalty),
            );
        }
        if result.context.config_placeholders != expected_defaults {
            return Err(mismatch(
                "native quest or resistance defaults differ from selected data",
            ));
        }
        if result.exports.len() != 1
            || result.exports[0].format != BuildFormat::PathOfBuilding2Xml
            || result.exports[0].content != expected_xml
        {
            return Err(mismatch(
                "native export must match the exact materialized source XML",
            ));
        }
        let evidence: Vec<_> = result
            .attachments
            .iter()
            .filter(|attachment| {
                attachment.media_type
                    == "application/vnd.poe-optimizer.native-profile+json;version=9"
            })
            .collect();
        if evidence.len() != 1 || evidence[0].content.len() > MAX_NATIVE_MACE_PROFILE_BYTES {
            return Err(mismatch(
                "one bounded native resolved-profile attachment is required",
            ));
        }
        let evidence: serde_json::Value = serde_json::from_str(&evidence[0].content)
            .map_err(|error| mismatch(&error.to_string()))?;
        let weapon = parse_mace_item(&choice.weapon, self.data.package())
            .map_err(|error| mismatch(&error.to_string()))?;
        if evidence["weapon_base"].as_str() != Some(weapon.base_name())
            || evidence["weapon_quality"].as_u64() != Some(u64::from(weapon.quality()))
            || evidence["weapon_item_level"].as_u64() != Some(u64::from(weapon.item_level()))
            || evidence["weapon_item"] != weapon.diagnostic()
            || evidence["actor_modifiers"] != self.profile.actor_modifiers.diagnostic()
            || evidence["equipment"]
                != serde_json::json!({"Weapon 1":crate::equipment::parse_equipment_item(&choice.weapon,self.data.package(),1).map_err(|error|mismatch(&error.to_string()))?.diagnostic()})
            || evidence["support_loadout"] != serde_json::json!(choice.support.keys())
            || evidence["configured_supports"]
                != serde_json::json!(
                    choice
                        .support
                        .keys()
                        .iter()
                        .map(|key| self.data.package().support(key).expect("validated support"))
                        .collect::<Vec<_>>()
                )
        {
            return Err(mismatch(
                "native resolved weapon or support differs from candidate payload",
            ));
        }
        let prepared_actor = prepared_actor(&self.compiled, &self.profile, &choice.tree)?;
        if evidence["movement"]
            != crate::actor_assembly::movement_evidence(prepared_actor.movement())
        {
            return Err(mismatch("native movement differs from selected source"));
        }
        let action_speed = prepared_actor.action_speed();
        if evidence["action_speed"] != crate::actor_assembly::action_speed_evidence(action_speed) {
            return Err(mismatch("native action speed differs from selected source"));
        }
        let allocation = class_tree::PassiveAllocationSelection {
            class_id: choice.tree.selection.class_id,
            ascendancy_id: choice.tree.selection.ascendancy_id.clone(),
            ordinary_nodes: choice.tree.selection.entrance_node_id.into_iter().collect(),
            ascendancy_nodes: choice
                .tree
                .selection
                .ascendancy_node_id
                .into_iter()
                .collect(),
            attribute_options: Default::default(),
        }
        .resolve(&self.data)
        .map_err(|error| mismatch(&error.to_string()))?;
        let character = self
            .compiled
            .character_from_allocation(&allocation)
            .map_err(|error| mismatch(&error.to_string()))?;
        let timing = crate::actor_assembly::mace_action_timing(
            &self.compiled,
            &character,
            &weapon,
            choice.support.keys(),
            action_speed.action_speed_mod,
        )
        .map_err(|error| mismatch(&error))?;
        if evidence["action_timing"] != crate::actor_assembly::action_timing_evidence(timing) {
            return Err(mismatch(
                "native action timing differs from selected source components",
            ));
        }
        let receiving = prepared_actor
            .receiving()
            .ok_or_else(|| mismatch("native receiver preparation is incomplete"))?;
        if evidence["receiving_defence"]
            != crate::actor_assembly::receiving_defence_evidence(receiving)
        {
            return Err(mismatch(
                "native receiving defences differ from selected source",
            ));
        }
        if evidence["local_armour"] != serde_json::json!({"schema_version":2,"items":{}}) {
            return Err(mismatch(
                "legacy Mace candidate cannot contain local armour",
            ));
        }
        self.check_native_tree(choice, result)?;
        Ok(())
    }
    fn check_native_tree(&self, choice: &Choice, result: &EvaluationResult) -> Result<()> {
        let attachments: Vec<_> = result
            .attachments
            .iter()
            .filter(|attachment| {
                attachment.media_type == "application/vnd.poe-optimizer.native-tree+json;version=3"
            })
            .collect();
        if attachments.len() != 1 || attachments[0].content.len() > 64 * 1024 {
            return Err(mismatch(
                "one bounded native resolved-tree attachment is required",
            ));
        }
        let actual: serde_json::Value = serde_json::from_str(&attachments[0].content)
            .map_err(|error| mismatch(&error.to_string()))?;
        let tree = &choice.tree;
        let ascendancy = tree.ascendancy.as_ref().map(|asc| {
            serde_json::json!({
                "index":asc.class_index,"internal_id":asc.internal_id,"catalog_id":asc.catalog_id,
                "name":asc.name,"start_node_id":asc.start_node_id,
            })
        });
        let paid_nodes: Vec<_> = tree.paid_node.iter().map(|node| ("ordinary", node))
            .chain(tree.ascendancy_node.iter().map(|node| ("ascendancy", node)))
            .map(|(kind, node)| serde_json::json!({
            "allocation_kind":kind,
            "physical_node_id":node.physical_node_id,"effective_node_id":node.effective_source_id,
            "name":node.name,"stats":node.stats,"override_provenance":node.provenance,
        })).collect();
        let expected = serde_json::json!({
            "schema_version":3,
            "class":{"index":tree.class.integer_id,"internal_id":tree.class.integer_id,
                "source_index":tree.class.source_index,"name":tree.class.name,"start_node_id":tree.class.start_node_id},
            "ascendancy":ascendancy,"allocated_nodes":tree.allocated_nodes,
            "ordinary_allocated_count":usize::from(tree.paid_node.is_some()),
            "ascendancy_allocated_count":usize::from(tree.ascendancy_node.is_some()),"paid_nodes":paid_nodes,
            "source":{"upstream_revision":self.data.tree().source.upstream_revision,
                "tree_version":self.data.tree().source.tree_version,
                "bundled_content_sha256":poe_optimizer_data::bundled::content_sha256()},
            "data_identity":self.data.identity(),
            "attribute_options":{},
            "configured_effects":tree.paid_node.iter().chain(tree.ascendancy_node.iter())
                .filter_map(|node| {
                    let key=poe_optimizer_data::passive_allocation::key_for_effective(node);
                    self.data.package().passive_view_effects(&key).map(|effect|(key,effect))
                }).collect::<BTreeMap<_,_>>().into_values().collect::<Vec<_>>(),
            "point_budget_verified":false,
            "evidence_kind":"native_source_resolution",
            "scope":"connected_capability_admitted_passives_and_explicit_attribute_options",
        });
        if actual != expected {
            return Err(mismatch(
                "native tree resolution differs from selected class, ascendancy, allocation or configured effects",
            ));
        }
        Ok(())
    }
    fn check_common_realization(&self, choice: &Choice, result: &EvaluationResult) -> Result<()> {
        if result.context.requested != EvaluationOptions::default()
            || result.context.calculation_mode != "MAIN"
            || result.context.enemy_level
                != scalar_number(&self.profile.config["enemyLevel"]).unwrap() as u32
        {
            return Err(mismatch("encounter overrides or calculation mode changed"));
        }
        for (name, value) in &self.profile.config {
            if result.context.config_inputs.get(name) != Some(value) {
                return Err(mismatch(&format!("explicit configuration {name} changed")));
            }
        }
        let build = &result.build;
        if build.level != self.profile.level
            || build.class_name != choice.tree.class.name
            || build.ascendancy_name
                != choice
                    .tree
                    .ascendancy
                    .as_ref()
                    .map_or("None", |asc| asc.name.as_str())
            || build.tree_version != self.data.package().tree.source.tree_version
            || build.main_socket_group != 1
            || build.skill_groups != 1
            || build
                .allocated_nodes
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                != choice.tree.allocated_nodes
            || build.allocated_nodes.len() != choice.tree.allocated_nodes.len()
        {
            return Err(mismatch("class, ascendancy, level, tree or group changed"));
        }
        let coverage = &result.coverage;
        let count = 1 + choice.support.keys().len();
        if coverage.active_skill_set_id != Some(1)
            || coverage.groups.len() != 1
            || coverage.unresolved_entry_count != 0
            || coverage.selected_minion.is_some()
        {
            return Err(mismatch(
                "unexpected skill set, unresolved or minion entries",
            ));
        }
        let selected = coverage
            .selected_player
            .as_ref()
            .ok_or_else(|| mismatch("missing selected player"))?;
        if selected.skill_id.as_deref() != Some(self.required_skill_id())
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
            || group.provenance.kind != SkillOrigin::Manual
            || group.slot.is_some()
            || group.provenance.source.is_some()
            || group.provenance.item_id.is_some()
            || group.provenance.node_id.is_some()
            || group.gems.len() != count
        {
            return Err(mismatch("manual skill group changed"));
        }
        for (index, gem) in group.gems.iter().enumerate() {
            let data = self.data.package();
            let (skill, game, variant) = if index == 0 {
                (
                    &data.mace.skill_id,
                    &data.mace.game_id,
                    &data.mace.variant_id,
                )
            } else {
                let support = data
                    .support(&choice.support_order[index - 1])
                    .expect("validated support");
                (&support.skill_id, &support.game_id, &support.variant_id)
            };
            if gem.index != index + 1
                || !gem.enabled
                || gem.resolution != SkillResolution::ResolvedGem
                || gem.skill_id.as_deref() != Some(skill.as_str())
                || gem.gem_game_id.as_deref() != Some(game.as_str())
                || gem.variant_id.as_deref() != Some(variant.as_str())
                || gem.level != Some(1.0)
                || gem.quality != Some(0.0)
                || gem.count != Some(1.0)
                || gem.is_support != Some(index > 0)
            {
                return Err(mismatch("exact gem identity or configuration changed"));
            }
        }
        Ok(())
    }
    fn check_realization(&self, choice: &Choice, result: &EvaluationResult) -> Result<()> {
        result
            .validate_recorded()
            .map_err(|error| mismatch(&error.to_string()))?;
        if self.data.identity().content_sha256 != game_data::bundled_package_sha256() {
            return Err(mismatch(
                "PoB reference binding requires the reviewed default data content",
            ));
        }
        if result.backend.id != "pob-poe2-mlua"
            || result.backend.rules_revision != PINNED_RULES_REVISION
            || result.backend.source_fingerprint != SOURCE
            || !result.diagnostic_only
        {
            return Err(mismatch(
                "wrong backend identity or missing diagnostic marker",
            ));
        }
        self.check_common_realization(choice, result)?;
        if result.exports.len() != 1 || result.exports[0].format != BuildFormat::PathOfBuilding2Xml
        {
            return Err(mismatch("exactly one fresh PoB XML export is required"));
        }
        check_export(
            &result.exports[0].content,
            choice,
            self.profile.level,
            &result.context,
            self.data.package(),
            &self.profile.actor_modifiers,
        )?;
        check_template_frame(
            &patch(&self.template, &self.profile, choice, &self.support_xml),
            &result.exports[0].content,
        )?;
        Ok(())
    }
}
fn fixed_warrior() -> ClassTreeSelection {
    ClassTreeSelection {
        class_id: 6,
        ascendancy_id: None,
        entrance_node_id: None,
        ascendancy_node_id: None,
    }
}
fn same_backend(left: &BackendIdentity, right: &BackendIdentity) -> bool {
    left == right
}
fn profile(xml: &str, data: &GameDataPackage) -> Result<Profile> {
    let document = parse(xml)?;
    crate::xml_compat::validate_native_with_actor_inputs(&document)
        .map_err(|error| unsupported(&error.to_string()))?;
    let root = document.root_element();
    only(
        root,
        &[],
        &["Build", "Tree", "Skills", "Items", "Config", "Notes"],
    )?;
    let build = child(root, "Build")?;
    only(
        build,
        &[
            "level",
            "className",
            "ascendClassName",
            "targetVersion",
            "characterLevelAutoMode",
            "mainSocketGroup",
            "viewMode",
        ],
        &[],
    )?;
    fixed(
        build,
        &[
            ("targetVersion", "0_1"),
            ("characterLevelAutoMode", "false"),
            ("mainSocketGroup", "1"),
            ("viewMode", "CALCS"),
        ],
    )?;
    let level = integer(build.attribute("level"), 1, 100, "character level")?;
    let tree = child(root, "Tree")?;
    only(tree, &["activeSpec"], &["Spec"])?;
    fixed(tree, &[("activeSpec", "1")])?;
    let spec = child(tree, "Spec")?;
    only(
        spec,
        &[
            "title",
            "classId",
            "classInternalId",
            "ascendClassId",
            "ascendancyInternalId",
            "treeVersion",
            "nodes",
            "masteryEffects",
        ],
        &[],
    )?;
    fixed(
        spec,
        &[
            ("treeVersion", data.tree.source.tree_version.as_str()),
            ("masteryEffects", ""),
        ],
    )?;
    let class_id = integer(
        spec.attribute("classInternalId"),
        0,
        u32::MAX,
        "classInternalId",
    )?;
    let class = data
        .tree
        .class(class_id)
        .map_err(|error| unsupported(&error.to_string()))?;
    let source_class = integer(spec.attribute("classId"), 0, u32::MAX, "classId")?;
    if (source_class != class_id && source_class != class.source_index)
        || build.attribute("className") != Some(class.name.as_str())
    {
        return Err(unsupported("class identity fields disagree"));
    }
    let ascendancy_index = integer(
        spec.attribute("ascendClassId"),
        0,
        u32::MAX,
        "ascendClassId",
    )?;
    let ascendancy_id = spec.attribute("ascendancyInternalId").unwrap_or("");
    let selection = ClassTreeSelection {
        class_id,
        ascendancy_id: if ascendancy_index == 0 {
            None
        } else {
            Some(ascendancy_id.into())
        },
        entrance_node_id: None,
        ascendancy_node_id: None,
    };
    let base = selection
        .resolve(&data.tree)
        .map_err(|error| unsupported(&error.to_string()))?;
    if base.ascendancy.as_ref().map_or(0, |asc| asc.class_index) != ascendancy_index
        || base
            .ascendancy
            .as_ref()
            .map_or("", |asc| asc.internal_id.as_str())
            != ascendancy_id
        || build.attribute("ascendClassName")
            != Some(
                base.ascendancy
                    .as_ref()
                    .map_or("None", |asc| asc.name.as_str()),
            )
    {
        return Err(unsupported("ascendancy identity fields disagree"));
    }
    let nodes = spec
        .attribute("nodes")
        .ok_or_else(|| unsupported("explicit nodes required"))?;
    let mut requested = BTreeSet::new();
    if !nodes.is_empty() {
        for node in nodes.split(',') {
            if !requested.insert(integer(Some(node), 0, u32::MAX, "allocated node")?) {
                return Err(unsupported("duplicate allocated node"));
            }
        }
    }
    let (ascendancy_ids, ordinary_ids): (Vec<_>, Vec<_>) = requested
        .difference(&base.implicit_roots)
        .copied()
        .partition(|id| data.tree.ascendancy_nodes.contains_key(id));
    if ordinary_ids.len() > 1 || ascendancy_ids.len() > 1 {
        return Err(unsupported(
            "at most one ordinary entrance and one reviewed ascendancy passive are supported",
        ));
    }
    let tree = ClassTreeSelection {
        entrance_node_id: ordinary_ids.first().copied(),
        ascendancy_node_id: ascendancy_ids.first().copied(),
        ..selection
    }
    .resolve(&data.tree)
    .map(Arc::new)
    .map_err(|error| unsupported(&error.to_string()))?;
    let mut tree_attribute_ranges = BTreeMap::new();
    for (node, fields) in [
        (build, &["className", "ascendClassName"][..]),
        (
            spec,
            &[
                "classId",
                "classInternalId",
                "ascendClassId",
                "ascendancyInternalId",
                "nodes",
            ][..],
        ),
    ] {
        for name in fields {
            if let Some(attribute) = node
                .attributes()
                .find(|attribute| attribute.name() == *name)
            {
                tree_attribute_ranges.insert((*name).into(), attribute.range_value());
            }
        }
    }
    // Insert a missing optional ascendancy identity before the first existing attribute.
    let ascendancy_insert = spec
        .attributes()
        .next()
        .expect("validated Spec attributes")
        .range()
        .start;
    let skills = child(root, "Skills")?;
    only(
        skills,
        &["activeSkillSet", "defaultGemLevel", "defaultGemQuality"],
        &["SkillSet"],
    )?;
    fixed(
        skills,
        &[
            ("activeSkillSet", "1"),
            ("defaultGemLevel", "normalMaximum"),
            ("defaultGemQuality", "0"),
        ],
    )?;
    let set = child(skills, "SkillSet")?;
    only(set, &["id", "title"], &["Skill"])?;
    fixed(set, &[("id", "1")])?;
    let group = child(set, "Skill")?;
    only(
        group,
        &[
            "enabled",
            "includeInFullDPS",
            "groupCount",
            "label",
            "mainActiveSkill",
            "mainActiveSkillCalcs",
        ],
        &["Gem"],
    )?;
    fixed(
        group,
        &[
            ("enabled", "true"),
            ("includeInFullDPS", "true"),
            ("groupCount", "1"),
            ("mainActiveSkill", "1"),
            ("mainActiveSkillCalcs", "1"),
        ],
    )?;
    let gems: Vec<_> = group.children().filter(Node::is_element).collect();
    if !(1..=3).contains(&gems.len()) {
        return Err(unsupported(
            "one Mace Strike and zero to two reviewed supports required",
        ));
    }
    check_gem(gems[0], None, false, data)?;
    let mut support_order = Vec::new();
    for node in &gems[1..] {
        let gem = data
            .supports
            .iter()
            .find(|gem| Some(gem.skill_id.as_str()) == node.attribute("skillId"))
            .ok_or_else(|| unsupported("unknown support gem"))?;
        check_gem(*node, Some(gem), false, data)?;
        support_order.push(gem.id.clone());
    }
    let support = MaceSupportLoadout::new(support_order.clone())?;
    data.validate_mace_support_loadout(support.keys())
        .map_err(|error| unsupported(&error.to_string()))?;
    let support_range = if gems.len() > 1 {
        gems[1].range()
    } else {
        gems[0].range().end..gems[0].range().end
    };
    let items = child(root, "Items")?;
    only(items, &["activeItemSet"], &["Item", "ItemSet"])?;
    fixed(items, &[("activeItemSet", "1")])?;
    let item = child(items, "Item")?;
    only(item, &["id"], &[])?;
    fixed(item, &[("id", "1")])?;
    if item.children().count() != 1 || !item.first_child().is_some_and(|node| node.is_text()) {
        return Err(unsupported("item must contain one unsplit text payload"));
    }
    let weapon = parse_mace_item_element(item, data)
        .map_err(|error| unsupported(&error.to_string()))?
        .source_text()
        .to_owned();
    let item_set = child(items, "ItemSet")?;
    only(item_set, &["id", "title", "useSecondWeaponSet"], &["Slot"])?;
    fixed(item_set, &[("id", "1"), ("useSecondWeaponSet", "false")])?;
    let slot = child(item_set, "Slot")?;
    only(slot, &["name", "itemId"], &[])?;
    fixed(slot, &[("name", "Weapon 1"), ("itemId", "1")])?;
    let config = child(root, "Config")?;
    only(config, &["activeConfigSet"], &["ConfigSet"])?;
    fixed(config, &[("activeConfigSet", "1")])?;
    let config_set = child(config, "ConfigSet")?;
    only(
        config_set,
        &["id", "title"],
        &["Input", "CustomModifierBlock"],
    )?;
    fixed(config_set, &[("id", "1")])?;
    let actor_modifiers = Arc::new(
        parse_actor_configuration(config_set, data).map_err(|e| unsupported(&e.to_string()))?,
    );
    let mut inputs = BTreeMap::new();
    for node in config_set
        .children()
        .filter(|node| node.has_tag_name("Input") && node.attribute("name") != Some("customMods"))
    {
        let name = node
            .attribute("name")
            .ok_or_else(|| unsupported("configuration name missing"))?;
        let value = scalar(node)?;
        validate_input(name, &value, data)?;
        if inputs.insert(name.into(), value).is_some() {
            return Err(unsupported("duplicate configuration input"));
        }
    }
    for required in INPUT_NAMES {
        if !inputs.contains_key(*required) {
            return Err(unsupported(&format!(
                "explicit configuration {required} required"
            )));
        }
    }
    if [
        "enemyPhysicalDamage",
        "enemyFireDamage",
        "enemyColdDamage",
        "enemyLightningDamage",
        "enemyChaosDamage",
    ]
    .iter()
    .all(|name| scalar_number(&inputs[*name]) == Some(0.0))
    {
        return Err(unsupported(
            "at least one incoming damage component must be positive",
        ));
    }
    for notes in root.children().filter(|node| node.has_tag_name("Notes")) {
        only(notes, &[], &[])?;
    }
    Ok(Profile {
        level,
        config: inputs,
        actor_modifiers,
        item_range: item.range(),
        support_range,
        extra_support_ranges: gems.iter().skip(2).map(Node::range).collect(),
        weapon,
        active_attributes: attributes(gems[0]),
        support,
        support_order,
        tree,
        tree_attribute_ranges,
        ascendancy_insert,
    })
}
/// Requirement availability is computed by the same pure actor stage as numerical evaluation.
/// Selected passive actor records follow configuration in the same ordered layer.
/// Scalar offence composition is independent of requirement availability.
fn prepared_actor(
    compiled: &CompiledGameData,
    profile: &Profile,
    tree: &ResolvedClassTree,
) -> Result<poe_optimizer_engine::actor::PreparedActorResources> {
    let package = compiled.snapshot().package();
    let enabled =
        std::array::from_fn(
            |index| match profile.config.get(&package.quests.config_keys[index]) {
                Some(Scalar::Boolean(value)) => *value,
                None => package.quests.default_enabled[index],
                _ => unreachable!("admitted quest scalar"),
            },
        );
    let mut quests = compiled.actor_quest_selection(SparkQuestRewards::from_enabled(enabled));
    for (index, quest) in package.actor.spirit_quests.iter().enumerate() {
        if let Some(Scalar::Boolean(value)) = profile.config.get(&quest.config_key) {
            quests.spirit[index] = *value;
        }
    }
    let character = CharacterInput {
        attributes: CharacterAttributes {
            strength: f64::from(tree.class.base_strength),
            dexterity: f64::from(tree.class.base_dexterity),
            intelligence: f64::from(tree.class.base_intelligence),
        },
        ..Default::default()
    };
    let mut records = profile.actor_modifiers.records().to_vec();
    records.extend(
        crate::actor_assembly::legacy_passive_actor_records(compiled.snapshot(), tree)
            .map_err(|error| unsupported(&error))?,
    );
    let penalty = match profile.config.get("resistancePenalty") {
        Some(Scalar::Number(value)) => *value,
        None => package.encounters.default_resistance_penalty,
        _ => unreachable!("admitted resistance penalty"),
    };
    compiled
        .prepare_actor(
            profile.level,
            quests,
            compiled.receiving_scenario(SparkQuestRewards::from_enabled(enabled), penalty),
            &character,
            std::slice::from_ref(&records),
        )
        .map_err(|e| unsupported(&format!("actor requirement preparation: {e}")))
}
fn actor_requirement_values(
    compiled: &CompiledGameData,
    profile: &Profile,
    tree: &ResolvedClassTree,
) -> Result<MaceRequirementValues> {
    let attributes = prepared_actor(compiled, profile, tree)?.values().attributes;
    let whole = |value: f64| -> Result<u32> {
        if !value.is_finite()
            || value.fract() != 0.0
            || !(0.0..=f64::from(u32::MAX)).contains(&value)
        {
            return Err(unsupported(
                "resolved requirement attributes must be finite whole u32 values",
            ));
        }
        Ok(value as u32)
    };
    Ok(MaceRequirementValues {
        level: profile.level,
        strength: whole(attributes.strength)?,
        dexterity: whole(attributes.dexterity)?,
        intelligence: whole(attributes.intelligence)?,
    })
}
const INPUT_NAMES: &[&str] = &[
    "enemyIsBoss",
    "enemyLevel",
    "enemyArmour",
    "enemyFireResist",
    "enemyColdResist",
    "enemyLightningResist",
    "enemyChaosResist",
    "enemyDamageType",
    "enemyPhysicalDamage",
    "enemyFireDamage",
    "enemyColdDamage",
    "enemyLightningDamage",
    "enemyChaosDamage",
    "enemyPhysicalOverwhelm",
    "enemyFirePen",
    "enemyColdPen",
    "enemyLightningPen",
    "enemyCritChance",
    "enemySpeed",
    "conditionEnemyShocked",
    "conditionEnemyChilled",
    "conditionEnemyIgnited",
    "conditionCritRecently",
    "conditionBeenHitRecently",
    "multiplierNearbyEnemies",
    "multiplierNearbyRareOrUniqueEnemies",
];
fn validate_input(name: &str, value: &Scalar, data: &GameDataPackage) -> Result<()> {
    let valid = match (name, value) {
        (name, Scalar::Boolean(_))
            if data.quests.config_keys.iter().any(|key| key == name)
                || data
                    .actor
                    .spirit_quests
                    .iter()
                    .any(|quest| quest.config_key == name) =>
        {
            true
        }
        ("resistancePenalty", Scalar::Number(value)) => (-60.0..=0.0).contains(value),
        ("enemyIsBoss", Scalar::Text(value)) => {
            ["None", "Boss", "Pinnacle"].contains(&value.as_str())
        }
        ("enemyDamageType", Scalar::Text(value)) => value == "Melee",
        (
            "conditionEnemyShocked"
            | "conditionEnemyChilled"
            | "conditionEnemyIgnited"
            | "conditionCritRecently"
            | "conditionBeenHitRecently",
            Scalar::Boolean(value),
        ) => !value,
        ("enemyLevel", Scalar::Number(value)) => {
            (1.0..=85.0).contains(value) && value.fract() == 0.0
        }
        (
            "enemyFireResist" | "enemyColdResist" | "enemyLightningResist" | "enemyChaosResist",
            Scalar::Number(value),
        ) => (-200.0..=100.0).contains(value),
        (
            "enemyArmour"
            | "enemyPhysicalDamage"
            | "enemyFireDamage"
            | "enemyColdDamage"
            | "enemyLightningDamage"
            | "enemyChaosDamage",
            Scalar::Number(value),
        ) => (0.0..=1_000_000.0).contains(value),
        (
            "enemyPhysicalOverwhelm" | "enemyFirePen" | "enemyColdPen" | "enemyLightningPen",
            Scalar::Number(value),
        ) => (0.0..=100.0).contains(value),
        ("enemyCritChance", Scalar::Number(value)) => *value == 0.0,
        ("enemySpeed", Scalar::Number(value)) => (1.0..=60_000.0).contains(value),
        ("multiplierNearbyEnemies", Scalar::Number(value)) => *value == 1.0,
        ("multiplierNearbyRareOrUniqueEnemies", Scalar::Number(value)) => *value == 0.0,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(unsupported(&format!(
            "unsupported configuration {name}={value:?}"
        )))
    }
}
fn check_reference_item(item: Node<'_, '_>, weapon: &str, data: &GameDataPackage) -> Result<()> {
    only(item, &["id"], &["ModRange"])?;
    fixed(item, &[("id", "1")])?;
    let expected = parse_mace_item(weapon, data).map_err(|error| mismatch(&error.to_string()))?;
    let ranges: Vec<_> = item.children().filter(Node::is_element).collect();
    if ranges.len() != expected.modifier_lines().len() {
        return Err(mismatch("reference item modifier range count differs"));
    }
    for (index, range) in ranges.iter().enumerate() {
        only(*range, &["id", "range"], &[])?;
        fixed(
            *range,
            &[("id", &(index + 1).to_string()), ("range", "0.5")],
        )?;
        if range
            .children()
            .any(|node| node.is_text() && !node.text().unwrap_or_default().trim().is_empty())
        {
            return Err(mismatch("reference modifier range has unexpected content"));
        }
    }
    let actual: Vec<_> = item
        .children()
        .filter(Node::is_text)
        .filter_map(|node| node.text())
        .flat_map(str::lines)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if actual != expected.pob_export_lines() {
        return Err(mismatch("exact equipped item payload changed"));
    }
    Ok(())
}
fn check_export(
    xml: &str,
    choice: &Choice,
    level: u32,
    context: &EvaluationContext,
    data: &GameDataPackage,
    actor_modifiers: &ValidatedActorModifiers,
) -> Result<()> {
    let document = parse(xml)?;
    let root = document.root_element();
    let build = child(root, "Build")?;
    fixed(
        build,
        &[
            ("className", choice.tree.class.name.as_str()),
            (
                "ascendClassName",
                choice
                    .tree
                    .ascendancy
                    .as_ref()
                    .map_or("None", |asc| asc.name.as_str()),
            ),
            ("mainSocketGroup", "1"),
        ],
    )?;
    if integer(build.attribute("level"), 1, 100, "exported character level")? != level {
        return Err(mismatch("exported level changed"));
    }
    let tree = child(root, "Tree")?;
    fixed(tree, &[("activeSpec", "1")])?;
    let spec = child(tree, "Spec")?;
    fixed(
        spec,
        &[
            ("classId", &choice.tree.class.integer_id.to_string()),
            ("classInternalId", &choice.tree.class.integer_id.to_string()),
            (
                "ascendancyInternalId",
                choice
                    .tree
                    .ascendancy
                    .as_ref()
                    .map_or("", |asc| asc.internal_id.as_str()),
            ),
            (
                "ascendClassId",
                &choice
                    .tree
                    .ascendancy
                    .as_ref()
                    .map_or(0, |asc| asc.class_index)
                    .to_string(),
            ),
            ("treeVersion", &data.tree.source.tree_version),
            ("masteryEffects", ""),
        ],
    )?;
    let nodes = spec
        .attribute("nodes")
        .ok_or_else(|| mismatch("exported nodes missing"))?;
    let mut allocated = BTreeSet::new();
    for node in nodes.split(',') {
        if !allocated.insert(integer(Some(node), 0, u32::MAX, "exported node")?) {
            return Err(mismatch("duplicate exported allocation"));
        }
    }
    if allocated != choice.tree.allocated_nodes {
        return Err(mismatch("exported allocations differ from selected tree"));
    }
    if spec
        .attribute("secondaryAscendClassId")
        .is_some_and(|value| value != "nil")
    {
        return Err(mismatch("secondary ascendancy changed"));
    }
    for container in spec.children().filter(Node::is_element) {
        match container.tag_name().name() {
            "URL" => {}
            "Sockets" if !container.children().any(|node| node.is_element()) => {}
            "Overrides" => {
                for entry in container.children().filter(Node::is_element) {
                    if !entry.has_tag_name("AttributeOverride")
                        || entry.attributes().any(|attribute| {
                            !["strNodes", "dexNodes", "intNodes"].contains(&attribute.name())
                                || !attribute.value().is_empty()
                        })
                    {
                        return Err(mismatch("passive override added"));
                    }
                }
            }
            _ => return Err(mismatch("unsupported exported passive state")),
        }
    }
    let items = child(root, "Items")?;
    let item_set = child(items, "ItemSet")?;
    fixed(
        items,
        &[("activeItemSet", "1"), ("useSecondWeaponSet", "false")],
    )?;
    fixed(item_set, &[("id", "1"), ("useSecondWeaponSet", "false")])?;
    let equipped: Vec<_> = item_set
        .children()
        .filter(|node| node.has_tag_name("Slot") && node.attribute("itemId") != Some("0"))
        .collect();
    if equipped.len() != 1 {
        return Err(mismatch("equipped item count changed"));
    }
    fixed(equipped[0], &[("name", "Weapon 1"), ("itemId", "1")])?;
    if item_set
        .children()
        .filter(|node| node.has_tag_name("RuneSlot"))
        .any(|node| node.attribute("runeName") != Some("None"))
    {
        return Err(mismatch("equipment rune added"));
    }
    check_reference_item(child(items, "Item")?, &choice.weapon, data)?;
    let skills = child(root, "Skills")?;
    fixed(skills, &[("activeSkillSet", "1")])?;
    let set = child(skills, "SkillSet")?;
    fixed(set, &[("id", "1")])?;
    let group = child(set, "Skill")?;
    only(
        group,
        &[
            "enabled",
            "includeInFullDPS",
            "groupCount",
            "label",
            "mainActiveSkill",
            "mainActiveSkillCalcs",
        ],
        &["Gem"],
    )?;
    fixed(
        group,
        &[
            ("enabled", "true"),
            ("includeInFullDPS", "true"),
            ("groupCount", "1"),
            ("mainActiveSkill", "1"),
            ("mainActiveSkillCalcs", "1"),
        ],
    )?;
    let gems: Vec<_> = group.children().filter(Node::is_element).collect();
    if gems.len() != 1 + choice.support.keys().len() {
        return Err(mismatch("exported support count changed"));
    }
    for (index, gem) in gems.iter().enumerate() {
        let support = index.checked_sub(1).map(|index| {
            data.support(&choice.support_order[index])
                .expect("validated support")
        });
        check_gem(*gem, support, true, data)?;
    }
    let config = child(root, "Config")?;
    fixed(config, &[("activeConfigSet", "1")])?;
    let set = child(config, "ConfigSet")?;
    fixed(set, &[("id", "1")])?;
    actor_modifiers
        .validate_reference_blocks(set)
        .map_err(|e| mismatch(&e.to_string()))?;
    let mut names = BTreeSet::new();
    for entry in set.children().filter(Node::is_element) {
        let kind = entry.tag_name().name();
        if kind != "CustomModifierBlock" && !names.insert((kind, entry.attribute("name"))) {
            return Err(mismatch("duplicate exported configuration"));
        }
        match kind {
            "Input" | "Placeholder" => {
                let values = if kind == "Input" {
                    &context.config_inputs
                } else {
                    &context.config_placeholders
                };
                let actual = scalar(entry)?;
                if entry.attribute("name").and_then(|name| values.get(name)) != Some(&actual) {
                    return Err(mismatch(
                        "exported configuration differs from effective context",
                    ));
                }
            }
            "CustomModifierBlock" => {} // Complete ordered block comparison is performed above.
            _ => return Err(mismatch("unsupported exported configuration")),
        }
    }
    Ok(())
}
fn check_gem(
    node: Node<'_, '_>,
    support: Option<&SupportData>,
    exported: bool,
    data: &GameDataPackage,
) -> Result<()> {
    let mut allowed = vec![
        "nameSpec",
        "skillId",
        "gemId",
        "variantId",
        "level",
        "quality",
        "enabled",
        "enableGlobal1",
        "enableGlobal2",
        "count",
    ];
    if exported {
        allowed.extend([
            "corrupted",
            "corruptLevel",
            "statSetIndex",
            "statSetIndexCalcs",
        ]);
    }
    only(node, &allowed, &[])?;
    let (name, skill, game, variant) = if let Some(support) = support {
        (
            &support.name,
            &support.skill_id,
            &support.game_id,
            &support.variant_id,
        )
    } else {
        (
            &data.mace.name,
            &data.mace.skill_id,
            &data.mace.game_id,
            &data.mace.variant_id,
        )
    };
    fixed(
        node,
        &[
            ("nameSpec", name),
            ("skillId", skill),
            ("gemId", game),
            ("variantId", variant),
            ("level", "1"),
            ("quality", "0"),
            ("enabled", "true"),
            ("enableGlobal1", "true"),
            ("enableGlobal2", "true"),
            ("count", "1"),
        ],
    )?;
    for (name, expected) in [
        ("corrupted", "false"),
        ("corruptLevel", "0"),
        ("statSetIndex", "nil"),
        ("statSetIndexCalcs", "nil"),
    ] {
        if node.attribute(name).is_some_and(|value| value != expected) {
            return Err(mismatch("unsupported exported gem setting"));
        }
    }
    Ok(())
}
/// Canonical normalized XML except numeric Build outputs and validated mutable build state.
fn exported_scenario(xml: &str, data: &GameDataPackage) -> Result<String> {
    fn visit(node: Node<'_, '_>, data: &GameDataPackage) -> String {
        if !node.is_element() {
            return String::new();
        }
        let tag = node.tag_name().name();
        if (tag == "Gem"
            && data
                .supports
                .iter()
                .any(|support| node.attribute("skillId") == Some(support.skill_id.as_str())))
            || (["PlayerStat", "MinionStat", "FullDPSSkill"].contains(&tag)
                && node
                    .parent_element()
                    .is_some_and(|parent| parent.has_tag_name("Build")))
        {
            return String::new();
        }
        if tag == "URL"
            && node
                .parent_element()
                .is_some_and(|parent| parent.has_tag_name("Spec"))
        {
            return String::new();
        }
        let mut fields = attributes(node);
        if tag == "Build" {
            for name in ["className", "ascendClassName"] {
                fields.remove(name);
            }
        }
        if tag == "Spec" {
            for name in [
                "classId",
                "classInternalId",
                "ascendClassId",
                "ascendancyInternalId",
                "nodes",
            ] {
                fields.remove(name);
            }
        }
        let mut output = format!("<{tag}:{fields:?}>");
        if !(tag == "Item"
            && node
                .parent_element()
                .is_some_and(|parent| parent.has_tag_name("Items")))
        {
            // PoB serializes several source tables with Lua pairs(). Child order
            // is validated explicitly where it has semantics (active/support gems).
            let mut children: Vec<_> = node
                .children()
                .filter(Node::is_element)
                .map(|node| visit(node, data))
                .filter(|value| !value.is_empty())
                .collect();
            children.sort();
            for child in children {
                output.push_str(&child);
            }
            for text in node
                .children()
                .filter(Node::is_text)
                .filter_map(|child| child.text())
                .filter(|text| !text.trim().is_empty())
            {
                output.push_str(&format!("{:?}", text.trim()));
            }
        }
        output.push_str(&format!("</{tag}>"));
        output
    }
    let document = parse(xml)?;
    Ok(visit(document.root_element(), data))
}
fn patch(
    template: &str,
    profile: &Profile,
    choice: &Choice,
    support_xml: &BTreeMap<String, String>,
) -> String {
    let mut support = choice
        .support
        .keys()
        .iter()
        .map(|key| support_xml[key].as_str())
        .collect::<Vec<_>>()
        .join("\n        ");
    if !support.is_empty() && profile.support.keys().is_empty() {
        support.insert_str(0, "\n        ");
    }
    let mut patches = vec![
        (
            profile.item_range.clone(),
            format!("<Item id=\"1\">{}</Item>", escape(&choice.weapon)),
        ),
        (profile.support_range.clone(), support),
    ];
    patches.extend(
        profile
            .extra_support_ranges
            .iter()
            .cloned()
            .map(|range| (range, String::new())),
    );
    if choice.patch_tree {
        let tree = &choice.tree;
        let fields = [
            ("className", tree.class.name.clone()),
            (
                "ascendClassName",
                tree.ascendancy
                    .as_ref()
                    .map_or("None", |asc| asc.name.as_str())
                    .into(),
            ),
            ("classId", tree.class.integer_id.to_string()),
            ("classInternalId", tree.class.integer_id.to_string()),
            (
                "ascendClassId",
                tree.ascendancy
                    .as_ref()
                    .map_or(0, |asc| asc.class_index)
                    .to_string(),
            ),
            (
                "ascendancyInternalId",
                tree.ascendancy
                    .as_ref()
                    .map_or("", |asc| asc.internal_id.as_str())
                    .into(),
            ),
            // Roots are implicit; preserve the selected paid physical nodes in authored XML.
            (
                "nodes",
                tree.selection
                    .entrance_node_id
                    .into_iter()
                    .chain(tree.selection.ascendancy_node_id)
                    .collect::<BTreeSet<_>>()
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        ];
        for (name, value) in fields {
            if let Some(range) = profile.tree_attribute_ranges.get(name) {
                patches.push((range.clone(), escape_attribute(&value)));
            } else {
                debug_assert_eq!(name, "ascendancyInternalId");
                patches.push((
                    profile.ascendancy_insert..profile.ascendancy_insert,
                    format!("{name}=\"{}\" ", escape_attribute(&value)),
                ));
            }
        }
    }
    patches.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut output = template.to_owned();
    for (range, replacement) in patches {
        output.replace_range(range, &replacement);
    }
    output
}
fn support_xml(support: &SupportData) -> String {
    format!(
        r#"<Gem nameSpec="{}" skillId="{}" gemId="{}" variantId="{}" level="1" quality="0" enabled="true" enableGlobal1="true" enableGlobal2="true" count="1"/>"#,
        escape_attribute(&support.name),
        escape_attribute(&support.skill_id),
        escape_attribute(&support.game_id),
        escape_attribute(&support.variant_id)
    )
}
fn escape_attribute(text: &str) -> String {
    escape(text).replace('"', "&quot;").replace('\'', "&apos;")
}
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn hash(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
fn payload(format: &str, content: String) -> ExactPayload {
    ExactPayload {
        format: format.into(),
        sha256: hash(&content),
        content,
    }
}
fn attributes(node: Node<'_, '_>) -> BTreeMap<String, String> {
    node.attributes()
        .map(|attribute| (attribute.name().into(), attribute.value().into()))
        .collect()
}
fn unsupported(message: &str) -> ControlledMutationError {
    ControlledMutationError::Unsupported(message.into())
}
fn mismatch(message: &str) -> ControlledMutationError {
    ControlledMutationError::Realization(message.into())
}
fn integer(value: Option<&str>, min: u32, max: u32, name: &str) -> Result<u32> {
    value
        .and_then(|text| {
            text.parse::<u32>()
                .ok()
                .filter(|value| value.to_string() == text)
        })
        .filter(|value| (min..=max).contains(value))
        .ok_or_else(|| unsupported(&format!("{name} must be an integer in {min}..={max}")))
}
fn parse(xml: &str) -> Result<Document<'_>> {
    crate::preflight::validate(xml).map_err(|error| unsupported(&error.to_string()))?;
    Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: crate::MAX_XML_NODES,
            entity_resolver: None,
        },
    )
    .map_err(|error| unsupported(&error.to_string()))
}
fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Result<Node<'a, 'input>> {
    let mut children = node.children().filter(|node| node.has_tag_name(name));
    let found = children
        .next()
        .ok_or_else(|| unsupported(&format!("missing {name}")))?;
    if children.next().is_some() {
        return Err(unsupported(&format!("duplicate {name}")));
    }
    Ok(found)
}
fn only(node: Node<'_, '_>, allowed_attributes: &[&str], allowed_children: &[&str]) -> Result<()> {
    if node.tag_name().namespace().is_some()
        || node.attributes().any(|attribute| {
            attribute.namespace().is_some() || !allowed_attributes.contains(&attribute.name())
        })
        || node.children().filter(Node::is_element).any(|child| {
            child.tag_name().namespace().is_some()
                || !allowed_children.contains(&child.tag_name().name())
        })
    {
        Err(unsupported(&format!(
            "unmapped attributes or children in {}",
            node.tag_name().name()
        )))
    } else {
        Ok(())
    }
}
fn fixed(node: Node<'_, '_>, expected: &[(&str, &str)]) -> Result<()> {
    for (name, value) in expected {
        if node.attribute(*name) != Some(*value) {
            return Err(unsupported(&format!(
                "{}.{} must be {value:?}",
                node.tag_name().name(),
                name
            )));
        }
    }
    Ok(())
}
fn scalar(node: Node<'_, '_>) -> Result<Scalar> {
    only(node, &["name", "number", "string", "boolean"], &[])?;
    let values: Vec<_> = ["number", "string", "boolean"]
        .into_iter()
        .filter_map(|name| node.attribute(name).map(|value| (name, value)))
        .collect();
    match values.as_slice() {
        [("number", text)] => text
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(Scalar::Number)
            .ok_or_else(|| unsupported("nonfinite configuration number")),
        [("string", text)] => Ok(Scalar::Text((*text).into())),
        [("boolean", "true")] => Ok(Scalar::Boolean(true)),
        [("boolean", "false")] => Ok(Scalar::Boolean(false)),
        _ => Err(unsupported("configuration must contain exactly one scalar")),
    }
}
fn scalar_number(value: &Scalar) -> Option<f64> {
    if let Scalar::Number(value) = value {
        Some(*value)
    } else {
        None
    }
}
/// Compare each explicitly supported source attribute outside mutated state.
/// Generated export defaults are checked by their specific validators and baseline guard.
fn check_template_frame(source_xml: &str, exported_xml: &str) -> Result<()> {
    let source = parse(source_xml)?;
    let exported = parse(exported_xml)?;
    for path in [
        &["Build"][..],
        &["Tree"],
        &["Tree", "Spec"],
        &["Skills"],
        &["Skills", "SkillSet"],
        &["Skills", "SkillSet", "Skill"],
        &["Items"],
        &["Items", "ItemSet"],
        &["Config"],
        &["Config", "ConfigSet"],
    ] {
        let mut left = source.root_element();
        let mut right = exported.root_element();
        for name in path {
            left = child(left, name)?;
            right = child(right, name)?;
        }
        for attribute in left.attributes() {
            // PoB explicitly adds the implicit class root to an empty allocation.
            if left.has_tag_name("Spec") && ["nodes", "classId"].contains(&attribute.name()) {
                continue;
            }
            if right.attribute(attribute.name()) != Some(attribute.value()) {
                return Err(mismatch(&format!(
                    "source {}.{} was not preserved",
                    left.tag_name().name(),
                    attribute.name()
                )));
            }
        }
    }
    let notes = |document: &Document<'_>| {
        document
            .root_element()
            .children()
            .find(|node| node.has_tag_name("Notes"))
            .map(|node| {
                node.children()
                    .filter(Node::is_text)
                    .filter_map(|child| child.text())
                    .collect::<String>()
                    .trim()
                    .to_owned()
            })
            .unwrap_or_default()
    };
    if notes(&source) != notes(&exported) {
        return Err(mismatch("source Notes were not preserved"));
    }
    let build = child(exported.root_element(), "Build")?;
    let buffs = child(build, "Buffs")?;
    only(buffs, &["combatList", "buffList", "curseList"], &[])?;
    fixed(
        buffs,
        &[("combatList", ""), ("buffList", ""), ("curseList", "")],
    )?;
    let timeless = child(build, "TimelessData")?;
    only(
        timeless,
        &[
            "searchList",
            "searchListFallback",
            "devotionVariant1",
            "devotionVariant2",
        ],
        &[],
    )?;
    fixed(
        timeless,
        &[
            ("searchList", ""),
            ("searchListFallback", ""),
            ("devotionVariant1", "1"),
            ("devotionVariant2", "1"),
        ],
    )?;
    if build.children().filter(Node::is_element).any(|node| {
        ![
            "PlayerStat",
            "MinionStat",
            "FullDPSSkill",
            "Buffs",
            "TimelessData",
        ]
        .contains(&node.tag_name().name())
    }) {
        return Err(mismatch(
            "exported build acquired unsupported persistent state",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "controlled_mace_tests.rs"]
mod tests;
