//! Deterministic offline extraction of every current native package section.
//! Run only under the isolated extraction worker's startup-inclusive deadline.
use crate::{source, tree_data};
use mlua::{Function, HookTriggers, Lua, LuaOptions, LuaSerdeExt, StdLib, Table, Value, VmState};
use poe_optimizer_data::{
    bundled::BundledClassTree, game_data::*, tree_projection::AuthenticatedTreeSnapshot,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

const READ_PATHS: &[&str] = &[
    "src/Modules/CalcFormat.lua",
    "src/Data/ModCache.lua",
    "runtime/lua/xml.lua",
    "runtime/lua/base64.lua",
    "runtime/lua/sha1/init.lua",
    "runtime/lua/sha1/common.lua",
    "runtime/lua/sha1/bit_ops.lua",
    "src/Data/Costs.lua",
    "src/Data/Essence.lua",
    "src/Data/FlavourText.lua",
    "src/Data/InventorySlots.lua",
    "src/Data/LiquidEmotions.lua",
    "src/Data/Minions.lua",
    "src/Data/ModMap.lua",
    "src/Data/Rares.lua",
    "src/Data/Spectres.lua",
    "src/Data/WorldAreas.lua",
    "src/Modules/StatDescriber.lua",
    "src/Modules/Common.lua",
    "src/Data/Global.lua",
    "src/Data/Misc.lua",
    "src/Modules/Data.lua",
    "src/Modules/ModTools.lua",
    "src/Modules/ModParser.lua",
    "src/Classes/ModStore.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModList.lua",
    "src/Classes/PassiveTree.lua",
    "src/Data/Bases/amulet.lua",
    "src/Data/Bases/helmet.lua",
    "src/Modules/ItemTools.lua",
    "src/Data/ModScalability.lua",
    "src/Data/Bases/gloves.lua",
    "src/Data/Bases/boots.lua",
    "src/Data/Bases/body.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/CalcDefence.lua",
    "src/Modules/CalcOffence.lua",
    "src/Modules/CalcTools.lua",
    "src/Modules/ConfigOptions.lua",
    "src/Classes/ConfigTab.lua",
    "src/Data/QuestRewards.lua",
    "src/Data/Bosses.lua",
    "src/Data/BossSkills.lua",
    "src/Data/Gems.lua",
    "src/Data/Skills/act_int.lua",
    "src/Data/Skills/other.lua",
    "src/Data/Skills/sup_str.lua",
    "src/Data/Skills/sup_dex.lua",
    "src/Data/Bases/mace.lua",
    "src/TreeData/0_5/tree.lua",
    "src/Classes/Item.lua",
    "src/Classes/SkillsTab.lua",
    "src/Data/SkillStatMap.lua",
    "src/Data/Assets.lua",
    "src/Data/Skills/SkillAssets.lua",
    "src/Data/Skills/act_str.lua",
    "src/Data/Skills/act_dex.lua",
    "src/Data/Skills/minion.lua",
    "src/Data/Skills/spectre.lua",
    "src/Data/Skills/sup_int.lua",
    "src/Data/Bases/axe.lua",
    "src/Data/Bases/belt.lua",
    "src/Data/Bases/bow.lua",
    "src/Data/Bases/claw.lua",
    "src/Data/Bases/crossbow.lua",
    "src/Data/Bases/dagger.lua",
    "src/Data/Bases/fishing.lua",
    "src/Data/Bases/flail.lua",
    "src/Data/Bases/flask.lua",
    "src/Data/Bases/focus.lua",
    "src/Data/Bases/incursionlimb.lua",
    "src/Data/Bases/jewel.lua",
    "src/Data/Bases/quiver.lua",
    "src/Data/Bases/ring.lua",
    "src/Data/Bases/sceptre.lua",
    "src/Data/Bases/shield.lua",
    "src/Data/Bases/spear.lua",
    "src/Data/Bases/staff.lua",
    "src/Data/Bases/sword.lua",
    "src/Data/Bases/talisman.lua",
    "src/Data/Bases/traptool.lua",
    "src/Data/Bases/wand.lua",
    "src/Data/Uniques/Special/Generated.lua",
    "src/Data/Uniques/Special/New.lua",
    "src/Data/Uniques/Special/race.lua",
    "src/Data/Uniques/amulet.lua",
    "src/Data/Uniques/axe.lua",
    "src/Data/Uniques/belt.lua",
    "src/Data/Uniques/body.lua",
    "src/Data/Uniques/boots.lua",
    "src/Data/Uniques/bow.lua",
    "src/Data/Uniques/claw.lua",
    "src/Data/Uniques/crossbow.lua",
    "src/Data/Uniques/dagger.lua",
    "src/Data/Uniques/fishing.lua",
    "src/Data/Uniques/flail.lua",
    "src/Data/Uniques/flask.lua",
    "src/Data/Uniques/focus.lua",
    "src/Data/Uniques/gloves.lua",
    "src/Data/Uniques/helmet.lua",
    "src/Data/Uniques/incursionlimb.lua",
    "src/Data/Uniques/jewel.lua",
    "src/Data/Uniques/mace.lua",
    "src/Data/Uniques/quiver.lua",
    "src/Data/Uniques/ring.lua",
    "src/Data/Uniques/sceptre.lua",
    "src/Data/Uniques/shield.lua",
    "src/Data/Uniques/soulcore.lua",
    "src/Data/Uniques/spear.lua",
    "src/Data/Uniques/staff.lua",
    "src/Data/Uniques/sword.lua",
    "src/Data/Uniques/talisman.lua",
    "src/Data/Uniques/tincture.lua",
    "src/Data/Uniques/traptool.lua",
    "src/Data/Uniques/wand.lua",
    "src/GameVersions.lua",
    "src/Modules/Main.lua",
    "src/Classes/ItemsTab.lua",
    "src/Data/ModItem.lua",
    "src/Data/ModFlask.lua",
    "src/Data/ModCharm.lua",
    "src/Data/ModIncursionLimb.lua",
    "src/Data/ModJewel.lua",
    "src/Data/ModCorrupted.lua",
    "src/Data/ModRunes.lua",
    "src/Data/ModItemExclusive.lua",
    "src/Data/ModVeiled.lua",
];
const PROVENANCE_PATHS: &[&str] = &[
    "src/Modules/ModParser.lua",
    "src/Data/Misc.lua",
    "src/Data/QuestRewards.lua",
    "src/Data/Skills/act_int.lua",
    "src/Modules/CalcDefence.lua",
    "src/Modules/CalcOffence.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/Common.lua",
    "src/Modules/ConfigOptions.lua",
    "src/Modules/Data.lua",
    "src/TreeData/0_5/tree.lua",
    "src/Data/Bases/mace.lua",
    "src/Data/Skills/other.lua",
    "src/Data/Skills/sup_str.lua",
    "src/Data/Skills/sup_dex.lua",
    "src/Data/SkillStatMap.lua",
    "src/Classes/Item.lua",
    "src/Classes/SkillsTab.lua",
    "src/Modules/CalcTools.lua",
    "src/Data/Gems.lua",
    "src/Classes/ModList.lua",
    "src/Classes/PassiveTree.lua",
    "src/Data/Bases/amulet.lua",
    "src/Data/Bases/helmet.lua",
    "src/Modules/ItemTools.lua",
    "src/Data/ModScalability.lua",
    "src/Data/Bases/gloves.lua",
    "src/Data/Bases/boots.lua",
    "src/Data/Bases/body.lua",
];
const POLICY: &str = include_str!("game_data_policy.json");
const CONVERSION: &str = include_str!("game_data_extract.lua");
const MAX_SOURCE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
#[error("pinned game-data extraction failed: {0}")]
pub struct GameDataExtractionError(pub String);
type Result<T> = std::result::Result<T, GameDataExtractionError>;
pub(crate) fn error(e: impl std::fmt::Display) -> GameDataExtractionError {
    GameDataExtractionError(e.to_string())
}
impl From<mlua::Error> for GameDataExtractionError {
    fn from(e: mlua::Error) -> Self {
        error(e)
    }
}
impl From<serde_json::Error> for GameDataExtractionError {
    fn from(e: serde_json::Error) -> Self {
        error(e)
    }
}
impl From<source::SourceError> for GameDataExtractionError {
    fn from(e: source::SourceError) -> Self {
        error(e)
    }
}
pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn normalized_hash(text: &str) -> String {
    hash(text.replace("\r\n", "\n").as_bytes())
}
fn extractor_sha256() -> String {
    let mut digest = Sha256::new();
    for text in [
        "poe-game-data-extractor-v25",
        include_str!("item_loading_extract.rs"),
        include_str!("unique_requirements_extract.rs"),
        include_str!("../../poe-optimizer-lua-utf8/src/lib.rs"),
        include_str!("../../poe-optimizer-lua-utf8/build.rs"),
        include_str!("../../poe-optimizer-lua-utf8/vendor/luautf8/lutf8lib.c"),
        include_str!("../../poe-optimizer-lua-utf8/vendor/luautf8/unidata.h"),
        include_str!("../../poe-optimizer-lua-utf8/vendor/provenance.json"),
        include_str!("item_scalability_extract.rs"),
        include_str!("modifier_parser_extract.rs"),
        include_str!("modifier_parser_extract/factories.rs"),
        include_str!("modifier_parser_extract/flags.rs"),
        include_str!("modifier_parser_extract/numbers.rs"),
        include_str!("modifier_parser_extract/ordinary.rs"),
        include_str!("modifier_parser_extract/strings.rs"),
        include_str!("modifier_parser_inputs.lua"),
        include_str!("skill_identity_extract.rs"),
        include_str!("configuration_extract.rs"),
        include_str!("game_data.rs"),
        CONVERSION,
        include_str!("source.rs"),
        include_str!("../Cargo.toml"),
        include_str!("../../../Cargo.toml"),
    ] {
        digest.update(text.replace("\r\n", "\n"));
    }
    format!("{:x}", digest.finalize())
}
pub(crate) fn expected_source_files() -> Result<BTreeMap<String, String>> {
    READ_PATHS
        .iter()
        .map(|path| Ok(((*path).into(), source::expected_file_sha256(path)?)))
        .collect()
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameDataExtractionEvidence {
    pub schema_version: u32,
    pub upstream_revision: String,
    pub source_manifest_sha256: String,
    /// Exact normalized bytes consumed by extraction or retained provenance.
    /// Full-inventory verification is additionally bound by source_manifest_sha256.
    pub source_files_sha256: BTreeMap<String, String>,
    pub extractor_sha256: String,
    pub policy_sha256: String,
    pub package_schema_version: u32,
    pub semantics_version: String,
    pub package_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractedGameData {
    pub package: GameDataPackage,
    pub evidence: GameDataExtractionEvidence,
}
impl ExtractedGameData {
    /// Validate the worker's pinned artifact/evidence without Lua or source I/O.
    /// This checks content and compatibility; a label alone cannot prove execution.
    pub fn validate(&self) -> Result<()> {
        let bytes = self.package.canonical_bytes().map_err(error)?;
        GameDataLoader::from_bytes(
            &bytes,
            &TrustPolicy::Reviewed {
                expected_sha256: bundled_package_sha256().into(),
            },
            &LoadLimits::default(),
        )
        .map_err(error)?;
        let expected = GameDataExtractionEvidence {
            schema_version: 1,
            upstream_revision: source::UPSTREAM_REVISION.into(),
            source_manifest_sha256: source::manifest_sha256(),
            source_files_sha256: expected_source_files()?,
            extractor_sha256: extractor_sha256(),
            policy_sha256: normalized_hash(POLICY),
            package_schema_version: SCHEMA_VERSION,
            semantics_version: SEMANTICS_VERSION.into(),
            package_sha256: hash(&bytes),
        };
        if self.evidence != expected {
            return Err(error(
                "extraction evidence does not match current source, policy, implementation and package",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ItemRulePolicy {
    id: String,
    template: String,
    form_pattern: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ActorRulePolicy {
    id: String,
    template: String,
    source_kind: String,
    pattern: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    condition_pattern: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema_version: u32,
    id: String,
    release: String,
    source_role: String,
    spark_skill: String,
    mace_skill: String,
    supports: Vec<[String; 2]>,
    support_level: u32,
    support_quality: u32,
    spark_default_class: String,
    mace_default_class: String,
    weapons: Vec<[String; 2]>,
    item_rules: Vec<ItemRulePolicy>,
    actor_rules: Vec<ActorRulePolicy>,
    spirit_quests: Vec<String>,
    quests: Vec<String>,
    coverage: Vec<String>,
}
/// Generate all current package sections from authenticated source, never from
/// the embedded package. A separate policy owns selection and conversion rules.
pub fn extract_pinned_game_data(root: &Path) -> Result<ExtractedGameData> {
    let output = extract_pinned_game_data_for_review(root)?;
    output.validate()?;
    Ok(output)
}
/// Offline maintainer generation for an explicitly reviewed package/schema migration.
/// Verifies the pinned complete source, but does not assert that newly generated
/// bytes already match the compiled reviewed artifact. Runtime/CLI extraction uses
/// `extract_pinned_game_data`, which additionally enforces that reviewed digest.
pub fn extract_pinned_game_data_for_review(root: &Path) -> Result<ExtractedGameData> {
    let source_manifest_sha256 = source::verify(root)?;
    let mut sources = BTreeMap::new();
    let mut total = 0usize;
    for path in READ_PATHS {
        let text = source::read_verified_text(root, path)?;
        total = total
            .checked_add(text.len())
            .ok_or_else(|| error("source byte count overflow"))?;
        if total > MAX_SOURCE_BYTES {
            return Err(error("total source bytes exceed extraction limit"));
        }
        sources.insert((*path).to_owned(), text);
    }
    let source_files_sha256 = sources
        .iter()
        .map(|(path, text)| (path.clone(), hash(text.as_bytes())))
        .collect();
    let snapshot = tree_data::extract_pinned_tree(root, "0_5").map_err(error)?;
    let digest = snapshot.sha256().map_err(error)?;
    let authenticated =
        AuthenticatedTreeSnapshot::from_trusted_extraction(snapshot, &digest).map_err(error)?;
    let tree = BundledClassTree::from_authenticated_snapshot(&authenticated).map_err(error)?;
    let policy: Policy = serde_json::from_str(POLICY)?;
    if policy.schema_version != 9
        || policy.spirit_quests.len() != 3
        || policy.actor_rules.is_empty()
        || policy.quests.len() != 6
        || policy.weapons.len() != 2
        || policy.item_rules.len() != 5
        || policy.supports.len() != 3
        || policy.support_level != 1
        || policy.support_quality != 0
    {
        return Err(error("unsupported extraction selection policy"));
    }
    let extractor = Extractor::new(sources)?;
    let record_function: Function = extractor.lua.globals().get("source_extract_records")?;
    let records: Table = record_function.call(extractor.lua.to_value(&policy)?)?;
    let (passive_effects, passive_exclusions) =
        extract_passive_catalog(&tree, authenticated.snapshot(), &extractor)?;
    let jewellery: Function = extractor.lua.globals().get("source_extract_jewellery")?;
    let (jewellery, _excluded): (Table, Table) = jewellery.call(
        records
            .get::<Table>("actor")?
            .get::<Table>("modifier_rules")?,
    )?;
    let mut jewellery_bases: Vec<JewelleryBaseData> = Vec::new();
    let bases: Table = extractor.lua.globals().get("sourceJewelleryBases")?;
    for row in jewellery.sequence_values::<Table>() {
        let row = row?;
        let source_table =
            copy_primitive_source_table(bases.get::<Table>(row.get::<String>("name")?)?, 0)?;
        // Deserialize the simple row by its declared shape, then attach source
        // maps in Rust; Lua's generic content buffering cannot preserve every
        // empty/integer-keyed map versus empty sequence distinction.
        let record: ExtractedJewelleryBase = extractor.lua.from_value(Value::Table(row))?;
        jewellery_bases.push(JewelleryBaseData {
            id: record.id,
            name: record.name,
            slot: record.slot,
            requirements: record.requirements,
            implicit: record.implicit,
            source: source_table,
        });
    }
    let armour: Function = extractor.lua.globals().get("source_extract_armour")?;
    let (armour, _excluded): (Table, Table) = armour.call(())?;
    let bases: Table = extractor.lua.globals().get("sourceArmourBases")?;
    let mut armour_bases = Vec::new();
    for row in armour.sequence_values::<Table>() {
        let row = row?;
        let source_table =
            copy_primitive_source_table(bases.get::<Table>(row.get::<String>("name")?)?, 0)?;
        let record: ExtractedArmourBase = extractor.lua.from_value(Value::Table(row))?;
        armour_bases.push(ArmourBaseData {
            id: record.id,
            name: record.name,
            slot: record.slot,
            requirements: record.requirements,
            quality: record.quality,
            armour: record.armour,
            evasion: record.evasion,
            energy_shield: record.energy_shield,
            movement_penalty: record.movement_penalty,
            source: source_table,
        });
    }
    let item_formatting: Function = extractor
        .lua
        .globals()
        .get("source_extract_item_formatting")?;
    let item_formatting = extractor
        .lua
        .from_value(item_formatting.call::<Value>(records.clone())?)?;
    let mut provenance = BTreeMap::new();
    for path in PROVENANCE_PATHS {
        provenance.insert((*path).into(), hash(extractor.sources[*path].as_bytes()));
    }
    provenance.insert("upstream_revision".into(), source::UPSTREAM_REVISION.into());
    provenance.insert("tree_version".into(), tree.source.tree_version.clone());
    provenance.insert("tree_content_sha256".into(), tree.sha256().map_err(error)?);
    provenance.insert("source_role".into(), policy.source_role);
    let mut package = GameDataPackage {
        manifest: GameDataManifest {
            game: "poe2".into(),
            release: policy.release,
            schema_version: SCHEMA_VERSION,
            semantics_version: SEMANTICS_VERSION.into(),
            provenance,
            section_sha256: BTreeMap::new(),
            coverage: policy.coverage,
        },
        tree,
        character: extractor.record(&records, "character")?,
        actor: extractor.record(&records, "actor")?,
        receiving_defence: extractor.record(&records, "receiving_defence")?,
        quests: extractor.record(&records, "quests")?,
        spark: extractor.record(&records, "spark")?,
        mace: extractor.record(&records, "mace")?,
        supports: extractor.record(&records, "supports")?,
        weapons: extractor.record(&records, "weapons")?,
        item_modifier_rules: extractor.record(&records, "item_modifier_rules")?,
        monsters: extractor.record(&records, "monsters")?,
        defence: extractor.defence()?,
        encounters: extractor.encounters()?,
        passive_effects,
        passive_exclusions,
        jewellery_bases,
        armour_bases,
        item_formatting,
        movement: extractor.record(&records, "movement")?,
        action_speed: extractor.record(&records, "action_speed")?,
        direct_action_timing: extractor.record(&records, "direct_action_timing")?,
        configuration: crate::configuration_extract::extract(&extractor.sources)?,
        skill_identities: crate::skill_identity_extract::extract(&extractor.sources)?,
        item_loading: crate::item_loading_extract::extract(&extractor.sources)?,
        item_scalability: crate::item_scalability_extract::extract(&extractor.sources)?,
        modifier_parser: crate::modifier_parser_extract::extract(&extractor.sources)?,
        unique_requirements: UniqueRequirementData::unavailable(
            "complete unique construction not yet exported",
        ),
    };
    package.unique_requirements = crate::unique_requirements_extract::extract(
        &extractor.sources,
        &package.item_loading,
        &package.tree,
    )?;
    package.refresh_section_digests().map_err(error)?;
    let evidence = GameDataExtractionEvidence {
        schema_version: 1,
        upstream_revision: source::UPSTREAM_REVISION.into(),
        source_manifest_sha256,
        source_files_sha256,
        extractor_sha256: extractor_sha256(),
        policy_sha256: normalized_hash(POLICY),
        package_schema_version: SCHEMA_VERSION,
        semantics_version: SEMANTICS_VERSION.into(),
        package_sha256: hash(&package.canonical_bytes().map_err(error)?),
    };
    Ok(ExtractedGameData { package, evidence })
}
pub(crate) fn section<'a>(source: &'a str, begin: &str, end: &str) -> Result<&'a str> {
    let matches: Vec<_> = source.match_indices(begin).collect();
    if matches.len() != 1 {
        return Err(error(format!(
            "missing or ambiguous source anchor {begin:?}"
        )));
    }
    let start = matches[0].0;
    let finish = source[start + begin.len()..]
        .find(end)
        .ok_or_else(|| error(format!("missing source terminator {end:?}")))?
        + start
        + begin.len();
    Ok(&source[start..finish])
}
fn line<'a>(source: &'a str, prefix: &str) -> Result<&'a str> {
    let matches: Vec<_> = source
        .lines()
        .filter(|line| line.trim_start().starts_with(prefix))
        .collect();
    if matches.len() != 1 {
        return Err(error(format!("missing or ambiguous source line {prefix}")));
    }
    Ok(matches[0])
}
fn literal(source: &str, begin: &str, end: &str) -> Result<f64> {
    let matches: Vec<_> = source.match_indices(begin).collect();
    if matches.len() != 1 {
        return Err(error(format!(
            "missing or ambiguous numeric source anchor {begin}"
        )));
    }
    let tail = &source[matches[0].0 + begin.len()..];
    let value = tail[..tail
        .find(end)
        .ok_or_else(|| error("missing numeric source terminator"))?]
        .trim()
        .parse::<f64>()
        .map_err(error)?;
    if !value.is_finite() {
        return Err(error("nonfinite source coefficient"));
    }
    Ok(value)
}
struct Extractor {
    lua: Lua,
    sources: BTreeMap<String, String>,
}
impl Extractor {
    fn new(sources: BTreeMap<String, String>) -> Result<Self> {
        let lua = Lua::new_with(
            StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::BIT | StdLib::JIT,
            LuaOptions::default(),
        )?;
        lua.set_memory_limit(128 * 1024 * 1024)?;
        // Hooks must also bound loops whose bytecode LuaJIT would otherwise trace.
        lua.load("jit.off(); jit.flush(); jit=nil; load=nil; loadstring=nil; loadfile=nil; dofile=nil; collectgarbage=nil; print=nil; package=nil; io=nil; os=nil; debug=nil; ffi=nil").exec()?;
        let ticks = Arc::new(AtomicUsize::new(0));
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(10_000),
            move |_, _| {
                if ticks.fetch_add(1, Ordering::Relaxed) >= 20_000 {
                    return Err(mlua::Error::RuntimeError(
                        "game-data extraction instruction limit".into(),
                    ));
                }
                Ok(VmState::Continue)
            },
        )?;
        let calc_base = lua.create_table()?;
        lua.globals().set(
            "require",
            lua.create_function(move |_, name: String| {
                if name == "Modules.CalcBase" {
                    Ok(calc_base.clone())
                } else {
                    Err(mlua::Error::RuntimeError(
                        "module is outside extraction allowlist".into(),
                    ))
                }
            })?,
        )?;
        let source = |path: &str| {
            sources
                .get(path)
                .map(String::as_str)
                .ok_or_else(|| error(format!("missing verified source {path}")))
        };
        let common = source("src/Modules/Common.lua")?;
        lua.load(format!(
            "local s_format=string.format;local m_floor=math.floor;common={{}};{}\n{}\n{}",
            section(common, "-- Class library\n", "function codePointToUTF8")?,
            section(
                common,
                "function round(val, dec)\n",
                "\n--- Rounds down a number"
            )?,
            section(common, "function copyTable(tbl, noRecurse)\n", "\ndo\n")?
        ))
        .exec()?;
        lua.load(source("src/Data/Global.lua")?).exec()?;
        let data: Table = lua.load(source("src/Data/Misc.lua")?).eval()?;
        lua.globals().set("data", data.clone())?;
        data.set(
            "modScalability",
            lua.load(source("src/Data/ModScalability.lua")?)
                .eval::<Table>()?,
        )?;
        lua.load(format!(
            "local m_floor=math.floor;local m_ceil=math.ceil;{}\n{}",
            section(
                common,
                "function roundSymmetric(val, dec)",
                "-- Symmetric ceil with precision:"
            )?,
            section(
                common,
                "function wipeTable(tbl)",
                "-- Search a table for a value"
            )?
        ))
        .exec()?;
        lua.load(section(
            source("src/Modules/ItemTools.lua")?,
            "local t_insert = table.insert",
            "function itemLib.formatModLine(",
        )?)
        .exec()?;
        for (begin, end) in [
            ("data.misc = {", "\ndata.skillColorMap = "),
            ("data.ailmentTypeList =", "data.buildupTypes ="),
            ("data.highPrecisionMods = {", "data.weaponTypeInfo = {"),
        ] {
            lua.load(section(source("src/Modules/Data.lua")?, begin, end)?)
                .exec()?;
        }
        lua.load("modLib={}").exec()?;
        lua.load(section(
            source("src/Modules/ModTools.lua")?,
            "function modLib.createMod(",
            "\nmodLib.parseMod,",
        )?)
        .exec()?;
        lua.load(section(
            source("src/Modules/ModTools.lua")?,
            "function modLib.setSource(",
            "function modLib.hasTag(",
        )?)
        .exec()?;
        lua.load(source("src/Classes/ModStore.lua")?).exec()?;
        lua.load(source("src/Classes/ModDB.lua")?).exec()?;
        lua.load(source("src/Classes/ModList.lua")?).exec()?;
        lua.load(format!("local t_insert=table.insert;local t_sort=table.sort;local s_format=string.format;local band=AND64;{}",section(source("src/Modules/ModTools.lua")?,"function modLib.formatFlags(","-- Check if a mod contains a specific tag already")?)).exec()?;
        lua.load(format!(
            "PassiveTreeClass={{}};local t_insert=table.insert;local t_remove=table.remove;{}",
            section(
                source("src/Classes/PassiveTree.lua")?,
                "function PassiveTreeClass:ProcessStats(",
                "-- Common processing code for nodes"
            )?
        ))
        .exec()?;
        let jewellery = lua.create_table()?;
        lua.load(source("src/Data/Bases/amulet.lua")?)
            .eval::<Function>()?
            .call::<()>(jewellery.clone())?;
        lua.globals().set("sourceJewelleryBases", jewellery)?;
        let armour = lua.create_table()?;
        for path in [
            "src/Data/Bases/helmet.lua",
            "src/Data/Bases/gloves.lua",
            "src/Data/Bases/boots.lua",
            "src/Data/Bases/body.lua",
        ] {
            lua.load(source(path)?)
                .eval::<Function>()?
                .call::<()>(armour.clone())?;
        }
        lua.globals().set("sourceArmourBases", armour)?;

        let gems: Table = lua.load(source("src/Data/Gems.lua")?).eval()?;
        lua.globals().set("sourceGems", gems)?;
        lua.load("data.gems={};data.keystones={};data.skills={}")
            .exec()?;
        let item_forms: Table = lua
            .load(format!(
                "{}\nreturn formList",
                section(
                    source("src/Modules/ModParser.lua")?,
                    "local formList = {",
                    "\n-- Map of modifier names"
                )?
            ))
            .eval()?;
        lua.globals().set("sourceItemForms", item_forms)?;
        // Expose actual local parser tables for grammar evidence without changing
        // its parsing branches. The complete source parser still interprets probes.
        let parser_source = source("src/Modules/ModParser.lua")?;
        let anchor = "\nreturn function(line, isComb)";
        if parser_source.matches(anchor).count() != 1 {
            return Err(error("ambiguous actor parser table observation anchor"));
        }
        let parser: Function = lua.load(parser_source.replacen(anchor,
            "\nsourceActorForms=formList;sourceActorSpecials=oldList;sourceActorTags=modTagList;\nreturn function(line, isComb)",1)).eval()?;
        lua.globals()
            .get::<Table>("modLib")?
            .set("parseMod", parser)?;
        lua.load(format!(
            "{}\nmod=makeSkillMod;flag=makeFlagMod;skill=makeSkillDataMod;skills={{}}",
            section(
                source("src/Modules/Data.lua")?,
                "local function makeSkillMod(",
                "local function processMod("
            )?
        ))
        .exec()?;
        let stat_map = lua
            .load(source("src/Data/SkillStatMap.lua")?)
            .eval::<Function>()?
            .call::<Table>((
                lua.globals().get::<Function>("mod")?,
                lua.globals().get::<Function>("flag")?,
                lua.globals().get::<Function>("skill")?,
            ))?;
        lua.globals().set("sourceSupportStatMap", stat_map)?;
        for (path, begin, end) in [
            (
                "src/Data/Skills/act_int.lua",
                "skills[\"SparkPlayer\"] = {",
                "\nskills[\"SummonSpectrePlayer\"]",
            ),
            (
                "src/Data/Skills/other.lua",
                "skills[\"Melee1HMacePlayer\"] = {",
                "\nskills[\"Melee2HMacePlayer\"]",
            ),
            (
                "src/Data/Skills/sup_str.lua",
                "skills[\"SupportBrutalityPlayer\"] = {",
                "\nskills[\"SupportBrutalityPlayerTwo\"]",
            ),
            (
                "src/Data/Skills/sup_str.lua",
                "skills[\"SupportMeleePhysicalDamagePlayer\"] = {",
                "\nskills[\"SupportHeftPlayer\"]",
            ),
            (
                "src/Data/Skills/sup_dex.lua",
                "skills[\"SupportRapidAttacksPlayer\"] = {",
                "\nskills[\"SupportRapidAttacksPlayerTwo\"]",
            ),
        ] {
            lua.load(section(source(path)?, begin, end)?).exec()?;
        }
        let bases = lua.create_table()?;
        lua.load(source("src/Data/Bases/mace.lua")?)
            .eval::<Function>()?
            .call::<()>(bases.clone())?;
        lua.globals().set("sourceBases", bases)?;
        let quests: Table = lua.load(source("src/Data/QuestRewards.lua")?).eval()?;
        lua.globals()
            .get::<Table>("data")?
            .set("questRewards", quests)?;
        let config: Table = lua
            .load(format!(
                "local StripEscapes=function(text) assert(not text:find(\"^\",1,true),\"quest colour escape requires source support\");return text end;{}\nlocal config={{}};addQuestModsRewardsConfigOptions(config);return config",
                section(
                    source("src/Modules/ConfigOptions.lua")?,
                    "local function questModsRewards(",
                    "\nlocal configSettings = {"
                )?
            ))
            .eval()?;
        lua.globals().set("sourceQuestConfig", config)?;
        let calcs: Table = lua.load(source("src/Modules/CalcDefence.lua")?).eval()?;
        lua.globals().set("sourceCalcs", calcs)?;
        lua.load(format!(
            "local calcs=sourceCalcs;local m_min=math.min;local m_max=math.max;{}",
            section(
                source("src/Modules/CalcPerform.lua")?,
                "function calcs.actionSpeedMod(actor)",
                "-- Initialises a minion's modifier database"
            )?
        ))
        .exec()?;
        lua.globals().set(
            "sourceActionSpeedText",
            section(
                source("src/Modules/CalcPerform.lua")?,
                "function calcs.actionSpeedMod(actor)",
                "-- Initialises a minion's modifier database",
            )?,
        )?;
        lua.globals().set("sourceDirectActionTiming", lua.load(format!("local originalRound=round;local round=function(v,p)return (sourceTimingRound or originalRound)(v,p)end;local m_min=math.min;local m_max=math.max;return function(baseTime,skillModList,globalOutput,skillFlags,channel) local output={{}};local cfg=nil;local skillCfg=nil;local source={{}};local skillData={{}};local activeSkill={{skillTypes={{[SkillType.Channel]=channel}}}};local more=skillModList:More(cfg, 'Speed');{}\n{}\n{}\nreturn output end", line(source("src/Modules/CalcOffence.lua")?, "output.Repeats = globalOutput.Repeats or")?, section(source("src/Modules/CalcOffence.lua")?, "\n\t\t\tlocal inc = skillModList:Sum(\"INC\", cfg, \"Speed\")", "\t\t\t-- Crossbows: Adjust attack speed values")?, section(source("src/Modules/CalcOffence.lua")?, "\n\t\t\tif output.Speed == 0 then", "\n\t\t\tif breakdown then")?)).eval::<Function>()?)?;
        lua.globals().set("sourceMovement",lua.load(format!("local originalRound=round;local round=function(v,p)return (sourceMovementRound or originalRound)(v,p)end;return function(actor) local modDB=actor.modDB;local output=actor.output;local m_max=math.max;{};return output end",section(source("src/Modules/CalcDefence.lua")?,"\t-- Miscellaneous: move speed, avoidance, weapon swap speed","\n\tif breakdown then\n\t\tbreakdown.EffectiveMovementSpeedMod")?)).eval::<Function>()?)?;
        lua.globals().set("sourceArmourPenalty",lua.load(format!("local t_remove=table.remove;local m_floor=math.floor;{};return function(value) local self={{base={{armour={{MovementPenalty=value}}}},quality=0,armourData={{}},modSource='Item:1:Source probe'}};local modList=new('ModList'):ModList();{};return modList end",section(source("src/Classes/Item.lua")?,"local function calcLocal(","-- Build list of modifiers")?,section(source("src/Classes/Item.lua")?,"\t\tlocal armourData = self.armourData","\telseif self.base.flask then")?)).eval::<Function>()?)?;

        let receiver_resources: Table = lua
            .load(format!(
                "local modDB={{Flag=function() return false end}};{};return resourceList",
                section(
                    source("src/Modules/CalcDefence.lua")?,
                    "local resourceList = {",
                    "\n\t\tfor _, source in ipairs(resourceList) do"
                )?
            ))
            .eval()?;
        lua.globals()
            .set("sourceReceiverResources", receiver_resources)?;
        let (resist_types, elemental): (Table, Table) = lua
            .load(format!(
                "{};{};return resistTypeList,isElemental",
                line(
                    source("src/Modules/CalcDefence.lua")?,
                    "local resistTypeList ="
                )?,
                line(
                    source("src/Modules/CalcDefence.lua")?,
                    "local isElemental ="
                )?
            ))
            .eval()?;
        lua.globals()
            .set("sourceReceiverResistTypes", resist_types)?;
        lua.globals().set("sourceReceiverElemental", elemental)?;
        lua.load(section(
            source("src/Modules/CalcTools.lua")?,
            "calcLib = { }",
            "-- Validate the level of the given gem",
        )?)
        .exec()?;
        lua.load(section(
            source("src/Modules/CalcTools.lua")?,
            "function calcLib.getGemStatRequirement(",
            "-- Build table of stats for the given skill instance statset",
        )?)
        .exec()?;
        let mut gem_requirements = String::from(
            "return function(gemData,grantedEffect) local gemInstance={gemData=gemData,level=1};\n",
        );
        for prefix in [
            "gemInstance.reqLevel =",
            "gemInstance.reqStr =",
            "gemInstance.reqDex =",
            "gemInstance.reqInt =",
        ] {
            gem_requirements.push_str(line(source("src/Classes/SkillsTab.lua")?, prefix)?);
            gem_requirements.push('\n');
        }
        gem_requirements.push_str("return {level=gemInstance.reqLevel,attributes={strength=gemInstance.reqStr,dexterity=gemInstance.reqDex,intelligence=gemInstance.reqInt}} end");
        lua.globals().set(
            "sourceGemRequirements",
            lua.load(gem_requirements).eval::<Function>()?,
        )?;
        let mut item_requirements = String::from(
            "return function(base) local m_max=math.max;local self={base=base,requirements={}};\n",
        );
        for prefix in [
            "self.requirements.runeLevel = 0",
            "self.requirements.str = self.base.req.str",
            "self.requirements.dex = self.base.req.dex",
            "self.requirements.int = self.base.req.int",
            "self.requirements.level = m_max(self.base.req.level",
        ] {
            item_requirements.push_str(line(source("src/Classes/Item.lua")?, prefix)?);
            item_requirements.push('\n');
        }
        item_requirements.push_str("return {level=self.requirements.level,attributes={strength=self.requirements.str,dexterity=self.requirements.dex,intelligence=self.requirements.int}} end");
        lua.globals().set(
            "sourceItemRequirements",
            lua.load(item_requirements).eval::<Function>()?,
        )?;
        let support_requirements = lua.load(format!(
            "return function(colors) local t_insert=table.insert;local gems={{}};for _,color in ipairs(colors) do gems[#gems+1]={{supportEffect={{grantedEffect={{color=color}}}}}} end;local env={{build={{skillsTab={{socketGroupList={{{{enabled=true,gemList=gems}}}}}},calcsTab={{}}}},modDB={{multipliers={{}}}},requirementsTableGems={{}}}};{}\nlocal req=env.requirementsTableGems[1];return {{strength=req.Str,dexterity=req.Dex,intelligence=req.Int}} end",
            section(source("src/Modules/CalcSetup.lua")?, "\tlocal slotSupportGemSocketsCount = { R = 0, G = 0, B = 0 }", "\t-- Merge Requirements Tables")?
        )).eval::<Function>()?;
        lua.globals()
            .set("sourceSupportRequirements", support_requirements)?;
        let tree: Table = lua.load(source("src/TreeData/0_5/tree.lua")?).eval()?;
        lua.globals().set("sourceTree", tree)?;
        let mut initialization =
            String::from("return function() local modDB=new('ModDB'):ModDB();\n");
        for prefix in [
            "modDB:NewMod(\"Life\", \"BASE\", data.characterConstants",
            "modDB:NewMod(\"Mana\", \"BASE\", data.characterConstants",
            "modDB:NewMod(\"Accuracy\", \"BASE\", data.characterConstants",
            "modDB:NewMod(\"CritChanceCap\", \"BASE\",",
            "modDB:NewMod(\"Spirit\", \"BASE\", 0,",
        ] {
            initialization.push_str(line(source("src/Modules/CalcSetup.lua")?, prefix)?);
            initialization.push('\n');
        }
        initialization.push_str("return modDB end");
        lua.globals().set(
            "sourceResourceInitialization",
            lua.load(initialization).eval::<Function>()?,
        )?;
        let bonuses=lua.load(format!("return function(flags) local modDB=new('ModDB'):ModDB();for _,flag in ipairs(flags or {{}}) do modDB:NewMod(flag,'FLAG',true) end;local output={{Str=1,Dex=1,Int=1}};{}\nreturn modDB end",section(source("src/Modules/CalcPerform.lua")?,"\t-- Add attribute bonuses\n","\t-- Calculate Presence / Surrounded")?)).eval::<Function>()?;
        let normal_multiplier: f64 = lua
            .load(format!(
                "{};return inherentAttributeMultiplier",
                line(
                    source("src/Modules/CalcPerform.lua")?,
                    "local inherentAttributeMultiplier ="
                )?
            ))
            .eval()?;
        lua.globals()
            .set("sourceNormalAttributeMultiplier", normal_multiplier)?;
        lua.globals().set("sourceAttributeBonuses", bonuses)?;
        // The source omits level-one Mace's baseMultiplier. Evaluate the actual
        // CalcOffence fallback expression rather than filling a copied constant.
        let base_multiplier=lua.load(format!("return function(level,skillData) local activeSkill={{activeEffect={{grantedEffectLevel=level}}}};{}\nreturn baseMultiplier end",line(source("src/Modules/CalcOffence.lua")?,"local baseMultiplier = activeSkill.activeEffect.grantedEffectLevel.baseMultiplier")?)).eval::<Function>()?;
        lua.globals().set("sourceBaseMultiplier", base_multiplier)?;
        let update=lua.load(format!("local ConfigTabClass={{}};local m_min=math.min;{}\nreturn ConfigTabClass.UpdateLevel",section(source("src/Classes/ConfigTab.lua")?,"function ConfigTabClass:UpdateLevel()","\nfunction ConfigTabClass:BuildModList()")?)).eval::<Function>()?;
        lua.globals().set("sourceUpdateLevel", update)?;
        lua.load(CONVERSION)
            .set_name("game_data_extract.lua")
            .exec()?;
        Ok(Self { lua, sources })
    }
    fn record<T: DeserializeOwned>(&self, records: &Table, name: &str) -> Result<T> {
        Ok(self.lua.from_value(records.get::<Value>(name)?)?)
    }
    fn number(&self, expression: &str) -> Result<f64> {
        Ok(self.lua.load(format!("return {expression}")).eval()?)
    }
    fn defence(&self) -> Result<DefenceData> {
        let source = &self.sources["src/Modules/CalcDefence.lua"];
        let hit = section(
            source,
            "function calcs.hitChance(",
            "-- Calculate monster hit chance",
        )?;
        let monster = section(
            source,
            "function calcs.monsterHitChance(",
            "-- Calculate Deflect chance",
        )?;
        let deflect = section(
            source,
            "function calcs.deflectChance(",
            "-- Calculate damage reduction from armour",
        )?;
        let deflect_line = line(deflect, "local chanceToNotDeflect =")?;
        Ok(DefenceData {
            armour_ratio: self.number("data.misc.ArmourRatio")?,
            deflection_chance_cap: self.number("data.misc.DeflectionChanceCap")?,
            hit_accuracy_multiplier: literal(hit, "accuracy * ", " )")?,
            hit_evasion_multiplier: literal(hit, "evasion * ", " )")?,
            hit_chance_floor: literal(hit, "\t\treturn ", "\n")?,
            hit_chance_cap: literal(hit, "m_min(round(rawChance), ", ")")?,
            monster_evasion_multiplier: literal(monster, "1 - ( ", " * evasion")?,
            monster_accuracy_multiplier: literal(monster, "evasion + ", " * accuracy")?,
            deflection_rating_multiplier: literal(deflect_line, "deflection * ", " )")?,
            deflection_chance_multiplier: literal(deflect_line, " ) * ", " - ")?,
            deflection_chance_offset: deflect_line
                .rsplit_once(" - ")
                .ok_or_else(|| error("missing deflection offset"))?
                .1
                .trim()
                .parse::<f64>()
                .map_err(error)?,
            deflection_rating_floor: literal(deflect, "deflection < ", " then")?,
            resistance_floor: self.number("data.misc.ResistFloor")?,
            player_resistance_cap: self
                .number("data.characterConstants['base_maximum_all_resistances_%']")?,
            resistance_maximum_cap: self.number("data.misc.MaxResistCap")?,
            enemy_resistance_cap: self.number("data.misc.MaxResistCap")?,
            enemy_physical_reduction_cap: self
                .number("data.monsterConstants['maximum_physical_damage_reduction_%']")?,
        })
    }
    fn encounters(&self) -> Result<EncounterData> {
        let source = &self.sources["src/Modules/ConfigOptions.lua"];
        let penalty: Table = self
            .lua
            .load(format!(
                "return {}",
                line(source, "{ var = \"resistancePenalty\"")?
                    .trim()
                    .trim_end_matches(',')
            ))
            .eval()?;
        let default_resistance_penalty = penalty
            .get::<Table>("list")?
            .get::<Table>(penalty.get::<usize>("defaultIndex")?)?
            .get("val")?;
        let boss: Table = self
            .lua
            .load(format!(
                "return {{{}}}",
                section(
                    source,
                    "\t{ var = \"enemyIsBoss\"",
                    "\t{ var = \"deliriousPercentage\""
                )?
            ))
            .eval::<Table>()?
            .get(1)?;
        let default_boss = boss
            .get::<Table>("list")?
            .get::<Table>(boss.get::<usize>("defaultIndex")?)?
            .get("val")?;
        let mut resistances = Vec::new();
        let mut pinnacle_level = None;
        let mut normal_level_cap = None;
        for (branch, end, start) in [
            (
                "\t\tif val == \"None\" then",
                "\t\telseif val == \"Boss\" then",
                "\t\t\tlocal defaultResist =",
            ),
            (
                "\t\telseif val == \"Boss\" then",
                "\t\telseif val == \"Pinnacle\" then",
                "\t\t\tlocal defaultEleResist =",
            ),
            (
                "\t\telseif val == \"Pinnacle\" then",
                "\t\telseif val == \"Uber\" then",
                "\t\t\tlocal defaultEleResist =",
            ),
        ] {
            let block = section(
                section(
                    section(
                        source,
                        "\t{ var = \"enemyIsBoss\"",
                        "\t{ var = \"deliriousPercentage\"",
                    )?,
                    branch,
                    end,
                )?,
                start,
                "\n\t\t\tlocal defaultDamage",
            )?;
            let apply: Function = self
                .lua
                .load(format!(
                    "return function(build)local m_max=math.max;{block}\nreturn build.configTab end"
                ))
                .eval()?;
            let build: Function = self.lua.globals().get("source_encounter_build")?;
            let config: Table = apply.call(build.call::<Table>(100)?)?;
            let values: Table = config
                .get::<Table>("configSets")?
                .get::<Table>(1)?
                .get("placeholder")?;
            let resistance = match values.get::<Value>("enemyLightningResist")? {
                Value::String(value) if value.to_str()?.is_empty() => 0.0,
                Value::Number(value) => value,
                Value::Integer(value) => value as f64,
                _ => return Err(error("unsupported enemy resistance placeholder")),
            };
            if resistances.is_empty() {
                normal_level_cap = Some(config.get::<u32>("enemyLevel")?);
            }
            if resistances.len() == 2 {
                pinnacle_level = Some(values.get::<u32>("enemyLevel")?);
            }
            resistances.push(resistance);
        }
        Ok(EncounterData {
            default_resistance_penalty,
            default_boss,
            normal_elemental_resistance: resistances[0],
            standard_elemental_resistance: resistances[1],
            pinnacle_elemental_resistance: resistances[2],
            normal_level_cap: normal_level_cap
                .ok_or_else(|| error("missing normal enemy level"))?,
            pinnacle_level: pinnacle_level.ok_or_else(|| error("missing pinnacle enemy level"))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2")
    }
    fn conversion() -> Lua {
        let lua = Lua::new_with(
            StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::BIT,
            LuaOptions::default(),
        )
        .unwrap();
        let common = source::read_verified_text(&root(), "src/Modules/Common.lua").unwrap();
        lua.load(section(&common, "function copyTable(tbl, noRecurse)\n", "\ndo\n").unwrap())
            .exec()
            .unwrap();
        lua.load(source::read_verified_text(&root(), "src/Data/Global.lua").unwrap())
            .exec()
            .unwrap();
        lua.load("modLib={}").exec().unwrap();
        let text = source::read_verified_text(&root(), "src/Modules/ModTools.lua").unwrap();
        lua.load(section(&text, "function modLib.createMod(", "\nmodLib.parseMod,").unwrap())
            .exec()
            .unwrap();
        lua.load(CONVERSION).exec().unwrap();
        lua
    }
    #[test]
    fn pinned_complete_extraction_reproduces_reviewed_bytes_and_rejects_changed_evidence() {
        let mut extracted = extract_pinned_game_data(&root()).unwrap();
        assert_eq!(
            extracted.package.canonical_bytes().unwrap(),
            bundled_package_bytes()
        );
        assert_eq!(
            extracted.evidence.source_files_sha256.len(),
            READ_PATHS.len()
        );
        extracted.validate().unwrap();
        let evidence = extracted.evidence.clone();
        extracted.evidence.policy_sha256 = "0".repeat(64);
        assert!(extracted.validate().is_err());
        extracted.evidence = evidence;
        extracted.package.spark.lightning_maximum += 1.0;
        extracted.package.refresh_section_digests().unwrap();
        extracted.evidence.package_sha256 = hash(&extracted.package.canonical_bytes().unwrap());
        assert!(extracted.validate().is_err());
    }
    #[test]
    fn typed_conversion_rejects_unconsumed_flags_tags_targets_and_order() {
        let lua = conversion();
        for expression in [
            "{modLib.createMod('Damage','INC',10,nil,ModFlag.Spell)}",
            "{modLib.createMod('FireResist','BASE',-0.5)}",
            "{modLib.createMod('ColdResist','BASE',-1000000)}",
            "{modLib.createMod('LightningResist','BASE',1000000)}",
            "{modLib.createMod('ChaosResist','BASE',7)}",
            "{modLib.createMod('ElementalResist','BASE',-20)}",
            "{modLib.createMod('Speed','INC',4),modLib.createMod('WarcrySpeed','INC',4),modLib.createMod('TotemPlacementSpeed','INC',4)}",
            "{modLib.createMod('MinionModifier','LIST',{mod=modLib.createMod('Damage','INC',10)})}",
        ] {
            lua.load(format!("return source_convert_modifiers({expression})"))
                .eval::<Table>()
                .unwrap();
        }
        for expression in [
            "{}",
            "{modLib.createMod('Life','BASE',10)}",
            "{modLib.createMod('Damage','INC',10,nil,ModFlag.Spell+ModFlag.Attack)}",
            "{modLib.createMod('Damage','INC',10,nil,ModFlag.Spell,{type='Condition',var='Unknown'})}",
            "{modLib.createMod('Damage','INC',10,'UnmodeledSource',ModFlag.Spell)}",
            "{modLib.createMod('Speed','INC',4),modLib.createMod('TotemPlacementSpeed','INC',4),modLib.createMod('WarcrySpeed','INC',4)}",
            "{modLib.createMod('MinionModifier','LIST',{mod=modLib.createMod('Damage','INC',10,nil,ModFlag.Spell)})}",
            "{modLib.createMod('Armour','BASE',0/0)}",
            "{modLib.createMod('Armour','BASE',-1)}",
            "{modLib.createMod('FireResist','BASE',0/0)}",
            "{modLib.createMod('FireResist','BASE',1/0)}",
            "{modLib.createMod('FireResist','BASE',-1000001)}",
            "{modLib.createMod('FireResist','INC',8)}",
            "{modLib.createMod('FireResist','MORE',8)}",
            "{modLib.createMod('FireResist','OVERRIDE',8)}",
            "{modLib.createMod('FireResistMax','BASE',8)}",
            "{modLib.createMod('FireResist','BASE',8,nil,ModFlag.Spell)}",
            "{modLib.createMod('FireResist','BASE',8,nil,0,0,{type='Condition',var='Unknown'})}",
            "{modLib.createMod('FireResist','BASE',8),modLib.createMod('ChaosResist','BASE',8)}",
            "{modLib.createMod('MinionModifier','LIST',{mod=modLib.createMod('FireResist','BASE',8)})}",
        ] {
            assert!(
                lua.load(format!("return source_convert_modifiers({expression})"))
                    .eval::<Table>()
                    .is_err(),
                "{expression}"
            );
        }
    }
    #[test]
    fn source_selection_and_numeric_anchors_are_unambiguous() {
        let lua = conversion();
        for values in ["{}", "{{id=1},{id=1}}"] {
            assert!(
                lua.load(format!(
                    "return source_unique({values},function(v)return v.id==1 end,'fixture')"
                ))
                .eval::<Table>()
                .is_err()
            );
        }
        assert!(section("start A start B finish", "start", "finish").is_err());
        assert!(section("start only", "start", "finish").is_err());
        assert!(literal("operand=1;operand=2;", "operand=", ";").is_err());
        assert!(literal("operand=no;", "operand=", ";").is_err());
        assert_eq!(literal("operand=1.25;", "operand=", ";").unwrap(), 1.25);
    }
    #[test]
    fn critical_chance_cap_conversion_requires_complete_unscoped_base_initialization() {
        let lua = conversion();
        assert_eq!(lua.load("return source_critical_chance_cap(modLib.createMod('CritChanceCap','BASE',100,'Base'))").eval::<f64>().unwrap(), 100.0);
        for mutation in [
            "m.type='MORE'",
            "m.type='INC'",
            "m.type='OVERRIDE'",
            "m.name='CritChance'",
            "m.source=nil",
            "m.source='Other'",
            "m.flags=ModFlag.Attack",
            "m.keywordFlags=1",
            "m[1]={type='Condition',var='Unknown'}",
            "m[1]={type='InSlot',num=1}",
            "m.unconsumed=true",
            "m.value=0/0",
            "m.value=1/0",
            "m.value=-1",
        ] {
            assert!(lua.load(format!("local m=modLib.createMod('CritChanceCap','BASE',100,'Base');{mutation};return source_critical_chance_cap(m)")).eval::<f64>().is_err(), "{mutation}");
        }
    }
    #[test]
    fn actor_record_conversion_preserves_all_fields_and_rejects_unconsumed_shapes() {
        let lua = conversion();
        for expression in [
            "modLib.createMod('Str','BASE',-2.5,'Custom:One')",
            "modLib.createMod('Mana','OVERRIDE',0)",
            "modLib.createMod('LifeConvertToEnergyShield','BASE',100)",
            "modLib.createMod('NoAttributeBonuses','FLAG',false)",
            "modLib.createMod('Life','MORE',10,nil,0,0,{type='Condition',varList={'DexHigherThanInt','StrHigherThanInt'},neg=true})",
        ] {
            lua.load(format!(
                "return source_convert_actor_modifier({expression})"
            ))
            .eval::<Table>()
            .unwrap();
        }
        for mutation in [
            "m.flags=1",
            "m.keywordFlags=1",
            "m.type='LIST'",
            "m.name='All'",
            "m.source={}",
            "m.value=0/0",
            "m.value=1/0",
            "m.value=-1000001",
            "m.hidden=true",
            "m[99]={type='Condition',var='StrHigherThanInt'}",
            "m[1]={type='Condition',var='Unknown'}",
            "m[1]={type='Condition',var='StrHigherThanInt',actor='enemy'}",
            "m[1]={type='PerStat',stat='Str'}",
            "m[1]={type='Condition',var='StrHigherThanInt',neg=0}",
            "m.name='ExtraLife';m.type='MORE'",
            "m.name='NoAttributeBonuses';m.type='FLAG'",
            "m.type='FLAG'",
            "m.name='DexAccBonusOverride'",
        ] {
            assert!(lua.load(format!("local m=modLib.createMod('Life','BASE',10);{mutation};return source_convert_actor_modifier(m)")).eval::<Table>().is_err(),"{mutation}");
        }
    }
    #[test]
    fn receiving_modifier_conversion_preserves_exact_global_scope_and_rejects_partial_targets() {
        let lua = conversion();
        for expression in [
            "modLib.createMod('Armour','BASE',-2.5,'Item:1',0,0,{type='Global'})",
            "modLib.createMod('ArmourAndEvasion','INC',17,nil,0,0,{type='Condition',var='StrHigherThanInt'})",
            "modLib.createMod('EnergyShield','INC',17,nil,0,0,{type='Global'},{type='Condition',var='DexHigherThanInt'})",
            "modLib.createMod('ElementalResist','BASE',-17.5)",
            "modLib.createMod('ChaosResist','INC',-117)",
            "modLib.createMod('Defences','INC',17)",
        ] {
            let converted: Value = lua
                .load(format!(
                    "return source_convert_actor_modifier({expression})"
                ))
                .eval()
                .unwrap();
            let record: ActorModifierRecord = lua.from_value(converted).unwrap();
            record.validate().unwrap();
        }
        for mutation in [
            "m.type='MORE'",
            "m.type='OVERRIDE'",
            "m.name='Defences'",
            "m.name='ArmourAndEnergyShield'",
            "m.name='FireResistMax'",
            "m.name='Ward'",
            "m.flags=1",
            "m.keywordFlags=1",
            "m.hidden=true",
            "m[1].actor='enemy'",
            "m[1].value=1",
            "m[1].var='StrHigherThanInt'",
            "m[1].type='InSlot';m[1].slot=1",
            "m[1].type='Condition';m[1].var='Unknown'",
            "m.name='Life'",
            "m.name='Accuracy'",
        ] {
            assert!(lua.load(format!(
                "local m=modLib.createMod('Armour','BASE',10,nil,0,0,{{type='Global'}});{mutation};return source_convert_actor_modifier(m)"
            )).eval::<Value>().is_err(), "{mutation}");
        }
    }
    #[test]
    fn actor_rule_extraction_requires_actual_source_grammar_and_complete_parser_outputs() {
        let sources = READ_PATHS
            .iter()
            .map(|p| ((*p).into(), source::read_verified_text(&root(), p).unwrap()))
            .collect();
        let extractor = Extractor::new(sources).unwrap();
        let lua = &extractor.lua;
        let policy: Policy = serde_json::from_str(POLICY).unwrap();
        let convert: Function = lua.globals().get("source_extract_actor_rule").unwrap();
        let reviewed = bundled_snapshot().unwrap();
        for (selection, expected) in policy
            .actor_rules
            .iter()
            .zip(&reviewed.package().actor.modifier_rules)
        {
            let actual: Value = convert.call(lua.to_value(selection).unwrap()).unwrap();
            assert_eq!(
                &lua.from_value::<ActorModifierRule>(actual).unwrap(),
                expected
            );
        }
        for template in [
            "{0} to all Attributes",
            "{0} to Ward",
            "{0} to maximum Life while holding a Shield",
            "{0} to Strength trailing text",
            "{1} to Strength",
        ] {
            let selection = ActorRulePolicy {
                id: "invalid".into(),
                template: template.into(),
                source_kind: "form".into(),
                pattern: policy.actor_rules[0].pattern.clone(),
                condition_pattern: None,
            };
            assert!(
                convert
                    .call::<Value>(lua.to_value(&selection).unwrap())
                    .is_err(),
                "{template}"
            );
        }
        let check:Function=lua.load("return function(selection,mutate) local original=modLib.parseMod;modLib.parseMod=function(text) local mods,extra=original(text);mutate(mods);return mods,extra end;local ok=pcall(source_extract_actor_rule,selection);modLib.parseMod=original;return ok end").eval().unwrap();
        for mutation in [
            "mods[1].source='Hidden'",
            "mods[1].hidden=true",
            "mods[1].flags=1",
            "mods[1].keywordFlags=1",
            "mods[1][1]={type='Condition',var='Unknown'}",
            "mods[1].value=mods[1].value+1",
            "mods[99]=mods[1]",
            "mods.extra=true",
            "mods[1].type='FLAG'",
        ] {
            let mutate: Function = lua
                .load(format!("return function(mods) {mutation} end"))
                .eval()
                .unwrap();
            assert!(
                !check
                    .call::<bool>((lua.to_value(&policy.actor_rules[0]).unwrap(), mutate))
                    .unwrap(),
                "{mutation}"
            );
        }
    }
    #[test]
    fn source_item_conversion_rejects_partial_global_tagged_or_unconsumed_mechanics() {
        let sources = READ_PATHS
            .iter()
            .map(|p| ((*p).into(), source::read_verified_text(&root(), p).unwrap()))
            .collect();
        let extractor = Extractor::new(sources).unwrap();
        let lua = &extractor.lua;
        let policy: Policy = serde_json::from_str(POLICY).unwrap();
        let convert: Function = lua.globals().get("source_extract_item_rule").unwrap();
        let reviewed = bundled_snapshot().unwrap();
        for (selection, expected) in policy
            .item_rules
            .iter()
            .zip(&reviewed.package().item_modifier_rules)
        {
            let actual: Value = convert.call(lua.to_value(selection).unwrap()).unwrap();
            assert_eq!(
                &lua.from_value::<ItemModifierRule>(actual).unwrap(),
                expected
            );
        }
        for text in [
            "Adds {0} to {1} Lightning Damage",
            "Adds {0} to {1} Physical Damage to Attacks",
            "Adds {0} to {1} Physical Damage to Spells",
            "Adds {0} to {1} Physical Damage while holding a Shield",
            "Adds {0} to {1} Physical Damage unconsumed text",
            "Adds {0} to {0} Physical Damage",
        ] {
            let selection = ItemRulePolicy {
                id: "probe".into(),
                template: text.into(),
                form_pattern: policy.item_rules[0].form_pattern.clone(),
            };
            assert!(
                convert
                    .call::<Value>(lua.to_value(&selection).unwrap())
                    .is_err(),
                "{text}"
            );
        }
        let check: Function = lua.load("return function(selection,mutate) local original=modLib.parseMod;modLib.parseMod=function(text) local mods,extra=original(text);mutate(mods);return mods,extra end;local ok=pcall(source_extract_item_rule,selection);modLib.parseMod=original;return ok end").eval().unwrap();
        for mutation in [
            "mods[1].flags=ModFlag.Attack",
            "mods[1].keywordFlags=1",
            "mods[1][1]={type='Condition',var='Unknown'}",
            "mods[1][1]={type='InSlot',num=1}",
            "mods[1].source='Unconsumed'",
            "mods.extra=true",
            "mods[99]=mods[1]",
            "mods[1].hidden=true",
            "mods[3]=mods[1]",
            "mods[1].value=mods[1].value+1",
            "mods[1].type='MORE'",
            "mods[1].name='ColdMin'",
            "mods[1],mods[2]=mods[2],mods[1]",
        ] {
            let mutate: Function = lua
                .load(format!("return function(mods) {mutation} end"))
                .eval()
                .unwrap();
            assert!(
                !check
                    .call::<bool>((lua.to_value(&policy.item_rules[0]).unwrap(), mutate))
                    .unwrap(),
                "{mutation}"
            );
        }
    }
    #[test]
    fn source_support_conversion_rejects_unconsumed_mechanics_and_type_expressions() {
        let sources = READ_PATHS
            .iter()
            .map(|p| ((*p).into(), source::read_verified_text(&root(), p).unwrap()))
            .collect();
        let extractor = Extractor::new(sources).unwrap();
        let check: Function = extractor.lua.load("return function(mutate) local original=skills.SupportMeleePhysicalDamagePlayer;skills.SupportMeleePhysicalDamagePlayer=copyTable(original);local s=skills.SupportMeleePhysicalDamagePlayer;mutate(s);local ok,err=pcall(source_extract_support,{'heavy_swing','SupportMeleePhysicalDamagePlayer'},1,0);skills.SupportMeleePhysicalDamagePlayer=original;return ok end").eval().unwrap();
        for mutation in [
            "s.extraEffect=true",
            "s.gemFamily[2]='OtherFamily'",
            "s.gemFamily.extra='HiddenFamily'",
            "s.statSets.extra=s.statSets[1]",
            "s.requireSkillTypes={SkillType.Melee,SkillType.Attack,SkillType.AND}",
            "s.excludeSkillTypes={SkillType.NOT}",
            "s.addSkillTypes={SkillType.Attack}",
            "s.levels[1].cost={Mana=1}",
            "s.statSets[1].baseFlags.attack=true",
            "s.statSets[1].constantStats[1][2]=0/0",
            "s.statSets[1].constantStats[3]=s.statSets[1].constantStats[1]",
            "s.statSets[1].constantStats.hidden={'unknown',5}",
            "s.statSets[1].constantStats[99]={'unknown',5}",
            "s.statSets[1].constantStats[1].hidden=5",
            "s.statSets[1].constantStats[1][99]=5",
            "s.statSets[1].stats.hidden='unknown'",
            "s.statSets[1].stats[99]='unknown'",
            "s.statSets[1].statMap.support_melee_physical_damage_foo={mod('PhysicalDamage','MORE',nil)}",
            "s.statSets[1].statMap['support_melee_physical_damage_+%_final'][1][1]={type='Condition',var='Unimplemented'}",
            "s.statSets[1].statMap['support_melee_physical_damage_+%_final'][1].flags=ModFlag.Spell",
            "s.statSets[1].statMap['support_melee_physical_damage_+%_final'][1].keywordFlags=1",
        ] {
            let mutate: Function = extractor
                .lua
                .load(format!("return function(s) {mutation} end"))
                .eval()
                .unwrap();
            assert!(!check.call::<bool>(mutate).unwrap(), "{mutation}");
        }
    }
    #[test]
    fn extractor_state_has_no_file_loader_or_general_module_access() {
        let sources = READ_PATHS
            .iter()
            .map(|p| ((*p).into(), source::read_verified_text(&root(), p).unwrap()))
            .collect();
        let extractor = Extractor::new(sources).unwrap();
        assert!(extractor.lua.load("return load==nil and loadstring==nil and loadfile==nil and dofile==nil and jit==nil and io==nil and os==nil and package==nil and debug==nil and ffi==nil").eval::<bool>().unwrap());
        assert!(
            extractor
                .lua
                .load("require('unexpected.module')")
                .exec()
                .is_err()
        );
    }
}

fn passive_option_fields_consumed(
    snapshot: &poe_optimizer_data::tree_data::TreeDataSnapshot,
    key: &poe_optimizer_data::class_tree::PassiveViewKey,
) -> bool {
    use poe_optimizer_data::{class_tree::PassiveViewSelector, tree_data::SourceValue};
    let node = &snapshot.nodes[&key.physical_node_id];
    let table = match &key.selector {
        PassiveViewSelector::Base => return true,
        PassiveViewSelector::Class { class_id } => snapshot
            .classes
            .get(class_id)
            .and_then(|class| node.automatic_overrides.get(&class.name)),
        PassiveViewSelector::Ascendancy { ascendancy_id } => snapshot
            .ascendancies
            .get(ascendancy_id)
            .and_then(|asc| node.automatic_overrides.get(&asc.name)),
        PassiveViewSelector::Attribute { option } => match node.source.named.get("options") {
            Some(SourceValue::Table(options)) => {
                match options.indexed.get(&i64::from(option.source_index())) {
                    Some(SourceValue::Table(table)) => Some(table),
                    _ => None,
                }
            }
            _ => None,
        },
    };
    table.is_some_and(passive_option_table_consumed)
}
fn passive_option_table_consumed(table: &poe_optimizer_data::tree_data::SourceTable) -> bool {
    table.indexed.is_empty()
        && table
            .named
            .keys()
            .all(|key| ["id", "name", "stats", "icon"].contains(&key.as_str()))
}

fn extract_passive_catalog(
    tree: &BundledClassTree,
    snapshot: &poe_optimizer_data::tree_data::TreeDataSnapshot,
    extractor: &Extractor,
) -> Result<(
    Vec<PassiveEffects>,
    Vec<poe_optimizer_data::passive_allocation::ExcludedPassiveView>,
)> {
    use poe_optimizer_data::{passive_allocation::ExcludedPassiveView, tree_data::TreeNodeKind};
    let parse: Function = extractor.lua.globals().get("source_extract_passive")?;
    let mut effects = Vec::new();
    let mut exclusions = Vec::new();
    const FIELDS: &[&str] = &[
        "activeEffectImage",
        "connections",
        "group",
        "icon",
        "isAttribute",
        "isNotable",
        "isSwitchable",
        "name",
        "options",
        "orbit",
        "orbitIndex",
        "recipe",
        "skill",
        "stats",
        "stringId",
    ];
    for view in &tree.allocation_views {
        let reason = if let Some(node) = tree.allocation_nodes.get(&view.key.physical_node_id) {
            if !matches!(node.kind, TreeNodeKind::Normal | TreeNodeKind::Notable)
                || node.source_default_point_cost != Some(1)
            {
                Some("unsupported_structural_node_kind")
            } else if node
                .unsupported_mechanics
                .iter()
                .any(|m| m != "attribute_choice")
            {
                Some("unsupported_structural_node_mechanic")
            } else if !passive_option_fields_consumed(snapshot, &view.key) {
                Some("unconsumed_structural_option_field")
            } else if snapshot.nodes[&view.key.physical_node_id]
                .source
                .named
                .keys()
                .any(|k| !FIELDS.contains(&k.as_str()))
            {
                Some("unconsumed_structural_source_field")
            } else {
                None
            }
        } else {
            None
        };
        if let Some(reason) = reason {
            exclusions.push(ExcludedPassiveView {
                key: view.key.clone(),
                reason: reason.into(),
            });
            continue;
        }
        let (parsed, reason): (Option<Table>, Option<String>) = parse.call((
            extractor.lua.to_value(&view.stats)?,
            view.key.physical_node_id,
            view.name.as_str(),
        ))?;
        if let Some(parsed) = parsed {
            effects.push(PassiveEffects {
                key: view.key.clone(),
                effective_node_id: view.effective_node_id,
                effects: extractor.lua.from_value(parsed.get("effects")?)?,
                actor_modifiers: extractor.lua.from_value(parsed.get("actor_modifiers")?)?,
            });
        } else {
            exclusions.push(ExcludedPassiveView {
                key: view.key.clone(),
                reason: reason.ok_or_else(|| error("missing source exclusion reason"))?,
            });
        }
    }
    Ok((effects, exclusions))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtractedArmourBase {
    id: String,
    name: String,
    slot: EquipmentSlot,
    requirements: RequirementData,
    quality: u32,
    armour: f64,
    evasion: f64,
    energy_shield: f64,
    movement_penalty: Option<f64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtractedJewelleryBase {
    id: String,
    name: String,
    slot: EquipmentSlot,
    requirements: RequirementData,
    implicit: JewelleryImplicitData,
}
fn copy_primitive_source_table(
    table: Table,
    depth: usize,
) -> Result<poe_optimizer_data::tree_data::SourceTable> {
    use poe_optimizer_data::tree_data::{SourceTable, SourceValue};
    if depth > 16 || table.metatable().is_some() {
        return Err(error("unsupported primitive source depth/metatable"));
    }
    let mut result = SourceTable::default();
    let mut count = 0;
    for pair in table.pairs::<Value, Value>() {
        let (key, value) = pair?;
        count += 1;
        if count > 256 {
            return Err(error("primitive source table is too large"));
        }
        let value = match value {
            Value::Boolean(v) => SourceValue::Boolean(v),
            Value::Integer(v) => SourceValue::Integer(v),
            Value::Number(v) if v.is_finite() => SourceValue::Number(v),
            Value::String(v) => {
                let v = v.to_str()?.to_owned();
                if v.len() > 16384 {
                    return Err(error("primitive source text too long"));
                }
                SourceValue::String(v)
            }
            Value::Table(v) => SourceValue::Table(copy_primitive_source_table(v, depth + 1)?),
            _ => return Err(error("nonprimitive source base value")),
        };
        match key {
            Value::String(k) => {
                result.named.insert(k.to_str()?.to_owned(), value);
            }
            Value::Integer(k) => {
                result.indexed.insert(k, value);
            }
            Value::Number(k) if k.is_finite() && k.fract() == 0.0 && k.abs() <= 1e6 => {
                result.indexed.insert(k as i64, value);
            }
            _ => return Err(error("unsupported primitive source key")),
        }
    }
    Ok(result)
}

#[cfg(test)]
mod passive_assembly_source_tests {
    use super::*;
    fn extractor() -> Extractor {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
        Extractor::new(
            READ_PATHS
                .iter()
                .map(|p| ((*p).into(), source::read_verified_text(&root, p).unwrap()))
                .collect(),
        )
        .unwrap()
    }
    #[test]
    fn source_option_metadata_cannot_change_unimplemented_topology_or_mechanics() {
        use poe_optimizer_data::tree_data::{SourceTable, SourceValue};
        let mut option = SourceTable::default();
        option
            .named
            .insert("id".into(), SourceValue::Integer(26297));
        option
            .named
            .insert("name".into(), SourceValue::String("Strength".into()));
        assert!(passive_option_table_consumed(&option));
        for field in [
            "connections",
            "unlockConstraint",
            "nodeOverlay",
            "grantedPassive",
            "unknown",
        ] {
            let mut edited = option.clone();
            edited
                .named
                .insert(field.into(), SourceValue::Boolean(true));
            assert!(!passive_option_table_consumed(&edited));
        }
        option.indexed.insert(1, SourceValue::Integer(1));
        assert!(!passive_option_table_consumed(&option));
    }
    #[test]
    fn whole_passive_source_lines_never_drop_unsupported_parts_or_unmodeled_order() {
        let e = extractor();
        let parse: Function = e.lua.globals().get("source_extract_passive").unwrap();
        let accepted: (Option<Table>, Option<String>) = parse
            .call((
                e.lua
                    .to_value(&["10% increased Spell Damage", "+10 to Intelligence"])
                    .unwrap(),
                51184,
                "Raw Power",
            ))
            .unwrap();
        let accepted = accepted.0.unwrap();
        assert_eq!(accepted.get::<Table>("effects").unwrap().raw_len(), 1);
        assert_eq!(
            accepted.get::<Table>("actor_modifiers").unwrap().raw_len(),
            1
        );
        for text in [
            "+10 to Intelligence\n10% increased Cast Speed while Dual Wielding",
            "10% more maximum Life",
            "Your Maximum Life is 1",
            "+10 to all Attributes",
            "10% increased Spell Damage plus an unknown effect",
        ] {
            let (result, reason): (Option<Table>, Option<String>) = parse
                .call((e.lua.to_value(&[text]).unwrap(), 51184, "Rejected"))
                .unwrap();
            assert!(result.is_none() && reason.is_some(), "{text}");
        }
        let mutate:Function=e.lua.load("return function() local original=modLib.parseMod;modLib.parseMod=function(...)local list,extra=original(...);if list and list[1]then list[1].hidden=true end;return list,extra end;local result=source_extract_passive({'+10 to Intelligence'},51184,'Hidden');modLib.parseMod=original;return result==nil end").eval().unwrap();
        assert!(mutate.call::<bool>(()).unwrap());
    }
    #[test]
    fn receiving_query_extraction_rejects_unconsumed_source_initialization() {
        let e = extractor();
        let check: Function = e.lua.load("return function(mutate) local old=sourceReceiverResources;sourceReceiverResources=copyTable(old);mutate(sourceReceiverResources);local ok=pcall(source_extract_receiving_defence);sourceReceiverResources=old;return ok end").eval().unwrap();
        for edit in [
            "r[1].hidden=true",
            "r[1].globalBase=1",
            "r[1].basePerSlot.Helmet=1",
            "r[1].conversionRate.EnergyShield=1",
            "r[1].defence=false",
            "r[1].modsTotal={'Armour'}",
            "r[1].mods[1]='Ward'",
            "r[1].mods.extra='Armour'",
            "r[6]=nil",
            "r.extra={}",
        ] {
            let mutate: Function = e
                .lua
                .load(format!("return function(r){edit} end"))
                .eval()
                .unwrap();
            assert!(!check.call::<bool>(mutate).unwrap(), "{edit}");
        }
    }
    #[test]
    fn item_formatting_extraction_observes_original_formats_and_rejects_incomplete_shapes() {
        let e = extractor();
        let snapshot = bundled_snapshot().unwrap();
        let package = snapshot.package();
        // source_extract_item_formatting reads only these two rule collections.
        // Keep unrelated package catalogs outside the extractor's bounded Lua state.
        let actor = e.lua.create_table().unwrap();
        actor
            .set(
                "modifier_rules",
                e.lua.to_value(&package.actor.modifier_rules).unwrap(),
            )
            .unwrap();
        let records = e.lua.create_table().unwrap();
        records.set("actor", actor).unwrap();
        records
            .set(
                "item_modifier_rules",
                e.lua.to_value(&package.item_modifier_rules).unwrap(),
            )
            .unwrap();
        let check:Function=e.lua.load("return function(records,edit) local old=data.modScalability;local format=itemLib.formatValue;local row=copyTable(old['# to Evasion Rating']);data.modScalability={['# to Evasion Rating']=row};edit(row);local ok,result=pcall(source_extract_item_formatting,records);data.modScalability=old;itemLib.formatValue=format;return ok,result end").eval().unwrap();
        for edit in [
            "r.extra=true",
            "r[1].hidden=true",
            "r[1].isScalable=1",
            "r[1].formats={hidden=true}",
            "r[1].formats={1}",
            "r[2]=copyTable(r[1])",
        ] {
            let mutate: Function = e
                .lua
                .load(format!("return function(r){edit} end"))
                .eval()
                .unwrap();
            let (ok, _): (bool, Value) = check.call((records.clone(), mutate)).unwrap();
            assert!(!ok, "{edit}");
        }
        for (format, precision, display, trim) in [
            ("divide_by_ten_1dp", 10.0, Some(1), false),
            ("divide_by_two_0dp", 2.0, Some(0), true),
            ("negate", 1.0, None, false),
        ] {
            let mutate: Function = e
                .lua
                .load(format!("return function(r)r[1].formats={{'{format}'}} end"))
                .eval()
                .unwrap();
            let (ok, value): (bool, Value) = check.call((records.clone(), mutate)).unwrap();
            assert!(ok);
            let parsed: ItemFormattingData = e.lua.from_value(value).unwrap();
            assert_eq!(
                parsed.rules[0].captures[0],
                ItemNumberFormat {
                    precision,
                    display_precision: display,
                    trim_trailing_zeroes: trim
                }
            );
        }
    }
    #[test]
    fn movement_extraction_rejects_source_scope_flags_operations_and_dynamic_cycles() {
        let e = extractor();
        let check:Function=e.lua.load("return function(edit) local old=sourceArmourPenalty;sourceArmourPenalty=function(v)local mods=old(v);edit(mods);return mods end;local ok=pcall(source_extract_movement);sourceArmourPenalty=old;return ok end").eval().unwrap();
        for edit in [
            "m[1].type='INC'",
            "m[1].source='Other'",
            "m[1].flags=1",
            "m[1].keywordFlags=1",
            "m[1].hidden=true",
            "m[1][1].neg=false",
            "m[1][1].var='Unknown'",
            "m[2]=copyTable(m[1])",
        ] {
            let mutate: Function = e
                .lua
                .load(format!("return function(m){edit} end"))
                .eval()
                .unwrap();
            assert!(!check.call::<bool>(mutate).unwrap(), "{edit}");
        }
        for stat in [
            "Str",
            "Life",
            "Armour",
            "Condition:IgnoreMovementPenalties",
            "MovementSpeedCannotBeBelowBase",
        ] {
            let kind = if stat == "Condition:IgnoreMovementPenalties"
                || stat == "MovementSpeedCannotBeBelowBase"
            {
                "FLAG"
            } else {
                "BASE"
            };
            let value = if kind == "FLAG" { "true" } else { "1" };
            assert!(e.lua.load(format!("return source_convert_actor_modifier(modLib.createMod('{stat}','{kind}',{value},nil,0,0,{{type='Condition',var='IgnoreMovementPenalties'}}))")).eval::<Value>().is_err(),"{stat}");
        }
        let original: Function = e
            .lua
            .globals()
            .get::<Table>("sourceCalcs")
            .unwrap()
            .get("actionSpeedMod")
            .unwrap();
        e.lua
            .globals()
            .get::<Table>("sourceCalcs")
            .unwrap()
            .set(
                "actionSpeedMod",
                e.lua
                    .load("return function()return 1.01 end")
                    .eval::<Function>()
                    .unwrap(),
            )
            .unwrap();
        assert!(
            e.lua
                .globals()
                .get::<Function>("source_extract_movement")
                .unwrap()
                .call::<Value>(())
                .is_err()
        );
        e.lua
            .globals()
            .get::<Table>("sourceCalcs")
            .unwrap()
            .set("actionSpeedMod", original)
            .unwrap();
    }
    #[test]
    fn action_speed_source_conversion_rejects_discarded_metadata_and_query_timing_drift() {
        let e = extractor();
        for edit in [
            "m[1].unscalable=false",
            "m[1].effectType='Aura'",
            "m[1].extra=true",
            "m.name='ActionSpeed'",
            "m.type='BASE'",
            "m.flags=1",
        ] {
            let check:Function=e.lua.load(format!("return function()local m=modLib.createMod('MinimumActionSpeed','MAX',100,nil,0,0,{{type='GlobalEffect',effectType='Global',unscalable=true}});{};return pcall(source_convert_actor_modifier,m)end",edit)).eval().unwrap();
            assert!(!check.call::<bool>(()).unwrap(), "{edit}");
        }
        for (target, operation) in [
            ("Str", "MAX"),
            ("MovementSpeed", "MAX"),
            ("ActionSpeed", "BASE"),
            ("ActionSpeed", "MORE"),
            ("ActionSpeed", "OVERRIDE"),
            ("TemporalChainsActionSpeed", "BASE"),
            ("MinimumActionSpeed", "INC"),
        ] {
            assert!(e.lua.load(format!("return source_convert_actor_modifier(modLib.createMod('{target}','{operation}',17))")).eval::<Value>().is_err(),"{target}/{operation}");
        }
        let action: Function = e.lua.globals().get("source_extract_action_speed").unwrap();
        let old: Function = e
            .lua
            .globals()
            .get::<Table>("sourceCalcs")
            .unwrap()
            .get("actionSpeedMod")
            .unwrap();
        e.lua
            .globals()
            .set("originalActionForTest", old.clone())
            .unwrap();
        for body in [
            "return 1.01",
            "local result=originalActionForTest(actor);actor.modDB:Sum('INC',nil,'ExtraUnconsumedQuery');return result",
        ] {
            e.lua
                .globals()
                .get::<Table>("sourceCalcs")
                .unwrap()
                .set(
                    "actionSpeedMod",
                    e.lua
                        .load(format!("return function(actor){body} end"))
                        .eval::<Function>()
                        .unwrap(),
                )
                .unwrap();
            assert!(action.call::<Value>(()).is_err(), "{body}");
        }
        e.lua
            .globals()
            .get::<Table>("sourceCalcs")
            .unwrap()
            .set("actionSpeedMod", old)
            .unwrap();
        let timing: Function = e.lua.globals().get("sourceDirectActionTiming").unwrap();
        e.lua
            .globals()
            .set("originalTimingForTest", timing.clone())
            .unwrap();
        e.lua.globals().set("sourceDirectActionTiming",e.lua.load("return function(base,db,actor,flags,channel)local value=originalTimingForTest(base,db,actor,flags,channel);value.CastRate=value.Speed;return value end").eval::<Function>().unwrap()).unwrap();
        assert!(
            e.lua
                .globals()
                .get::<Function>("source_extract_direct_action_timing")
                .unwrap()
                .call::<Value>(())
                .is_err()
        );
        e.lua
            .globals()
            .set("sourceDirectActionTiming", timing)
            .unwrap();
    }
    #[test]
    fn movement_base_extraction_preserves_absent_versus_explicit_zero_for_any_slot() {
        let e = extractor();
        let check:Function=e.lua.load("return function()local old=sourceArmourBases;local b=copyTable(old['Rusted Greathelm']);sourceArmourBases={['Rusted Greathelm']=b};local none=source_extract_armour();assert(none[1].movement_penalty==nil);b.armour.MovementPenalty=0;local zero=source_extract_armour();assert(zero[1].movement_penalty==0);b.armour.MovementPenalty=.125;local other=source_extract_armour();assert(other[1].movement_penalty==.125);sourceArmourBases=old;return true end").eval().unwrap();
        assert!(check.call::<bool>(()).unwrap());
    }
    #[test]
    fn armour_base_extraction_requires_complete_fixed_source_shape() {
        let e = extractor();
        let f: Function = e.lua.globals().get("source_extract_armour").unwrap();
        let (accepted, excluded): (Table, Table) = f.call(()).unwrap();
        assert_eq!(accepted.raw_len(), 402);
        assert_eq!(excluded.raw_len(), 594);
        let check:Function=e.lua.load("return function(mutate) local old=sourceArmourBases;local base=copyTable(old['Rusted Greathelm']);sourceArmourBases={['Rusted Greathelm']=base};mutate(base);local result,excluded=source_extract_armour();sourceArmourBases=old;return #result==0 and #excluded==1 end").eval().unwrap();
        for edit in [
            "b.hidden=false",
            "b.implicit='+10 to maximum Life'",
            "b.implicitModTypes[1]={'life'}",
            "b.type='Body Armour'",
            "b.subType='Unknown'",
            "b.quality=30",
            "b.socketLimit=4",
            "b.tags.hidden=true",
            "b.tags.armour=false",
            "b.tags.boots=true",
            "b.tags.helmet=nil",
            "b.armour.Ward=0",
            "b.armour.BlockChance=0",
            "b.armour.MovementPenalty=-0.1",
            "b.armour.EvasionPerLevel=0",
            "b.armour.EnergyShieldPerLevel=0",
            "b.armour.Armour=-1",
            "b.armour.Armour=0/0",
            "b.armour={}",
            "b.req.extra=0",
            "b.req.str=-1",
            "b.req.int=1.5",
            "b.req.level=101",
        ] {
            let mutate: Function = e
                .lua
                .load(format!("return function(b){edit} end"))
                .eval()
                .unwrap();
            assert!(check.call::<bool>(mutate).unwrap(), "{edit}");
        }
    }
    #[test]
    fn armour_pairs_cannot_be_admitted_as_passive_or_global_actor_effects() {
        let e = extractor();
        for stat in ["ArmourAndEnergyShield", "EvasionAndEnergyShield"] {
            let row: Value = e
                .lua
                .load(format!(
                    "return source_convert_actor_modifier(modLib.createMod('{stat}','INC',25))"
                ))
                .eval()
                .unwrap();
            let row: ActorModifierRecord = e.lua.from_value(row).unwrap();
            assert!(row.stat.is_local_armour_only());
            row.validate().unwrap();
            for tail in [
                "m.type='MORE'",
                "m[1]={type='Global'}",
                "m[1]={type='Condition',var='Unknown'}",
            ] {
                assert!(e.lua.load(format!("local m=modLib.createMod('{stat}','BASE',25);{tail};return source_convert_actor_modifier(m)")).eval::<Value>().is_err(),"{stat} {tail}");
            }
        }
        let parse: Function = e.lua.globals().get("source_extract_passive").unwrap();
        for line in [
            "+10 to Armour and Energy Shield",
            "20% increased Evasion Rating and Energy Shield",
        ] {
            let (result, reason): (Option<Table>, Option<String>) = parse
                .call((
                    e.lua.to_value(&[line]).unwrap(),
                    123,
                    "Unsupported paired passive",
                ))
                .unwrap();
            assert!(result.is_none() && reason.is_some(), "{line}");
        }
    }
    #[test]
    fn jewellery_extraction_enumerates_whole_family_and_rejects_unconsumed_source_changes() {
        let e = extractor();
        let rules = e
            .lua
            .to_value(&bundled_snapshot().unwrap().package().actor.modifier_rules)
            .unwrap();
        let f: Function = e.lua.globals().get("source_extract_jewellery").unwrap();
        let (accepted, excluded): (Table, Table) = f.call(rules.clone()).unwrap();
        assert_eq!(accepted.raw_len(), 7);
        assert!(
            excluded
                .sequence_values::<String>()
                .map(|value| value.unwrap())
                .any(|name| name == "Stellar Amulet")
        );
        let check:Function=e.lua.load("return function(rules,mutate) local old=sourceJewelleryBases;local base=copyTable(old['Amber Amulet']);sourceJewelleryBases={['Amber Amulet']=base};mutate(base);local result,excluded=source_extract_jewellery(rules);sourceJewelleryBases=old;return #result==0 and #excluded==1 end").eval().unwrap();
        for edit in [
            "b.hidden=true",
            "b.tags.attack=true",
            "b.req.extra=1",
            "b.implicit=b.implicit..'\\n+10 to maximum Life'",
            "b.implicit='+(10-15) to all Attributes'",
            "b.implicitModTypes[2]={}",
            "b.implicit='+(15-10) to Strength'",
            "b.implicit='+(1.5-10) to Strength'",
        ] {
            let mutate: Function = e
                .lua
                .load(format!("return function(b){edit} end"))
                .eval()
                .unwrap();
            assert!(
                check.call::<bool>((rules.clone(), mutate)).unwrap(),
                "{edit}"
            );
        }
    }
}
