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
    "src/Modules/Common.lua",
    "src/Data/Global.lua",
    "src/Data/Misc.lua",
    "src/Modules/Data.lua",
    "src/Modules/ModTools.lua",
    "src/Modules/ModParser.lua",
    "src/Classes/ModStore.lua",
    "src/Classes/ModDB.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/CalcDefence.lua",
    "src/Modules/CalcOffence.lua",
    "src/Modules/CalcTools.lua",
    "src/Modules/ConfigOptions.lua",
    "src/Classes/ConfigTab.lua",
    "src/Data/QuestRewards.lua",
    "src/Data/Gems.lua",
    "src/Data/Skills/act_int.lua",
    "src/Data/Skills/other.lua",
    "src/Data/Skills/sup_str.lua",
    "src/Data/Bases/mace.lua",
    "src/TreeData/0_5/tree.lua",
    "src/Classes/Item.lua",
    "src/Classes/SkillsTab.lua",
    "src/Data/SkillStatMap.lua",
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
    "src/Data/SkillStatMap.lua",
    "src/Classes/Item.lua",
    "src/Classes/SkillsTab.lua",
    "src/Modules/CalcTools.lua",
    "src/Data/Gems.lua",
];
const POLICY: &str = include_str!("game_data_policy.json");
const CONVERSION: &str = include_str!("game_data_extract.lua");
const MAX_SOURCE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
#[error("pinned game-data extraction failed: {0}")]
pub struct GameDataExtractionError(pub String);
type Result<T> = std::result::Result<T, GameDataExtractionError>;
fn error(e: impl std::fmt::Display) -> GameDataExtractionError {
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
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn normalized_hash(text: &str) -> String {
    hash(text.replace("\r\n", "\n").as_bytes())
}
fn extractor_sha256() -> String {
    let mut digest = Sha256::new();
    for text in [
        "poe-game-data-extractor-v2",
        include_str!("game_data.rs"),
        CONVERSION,
        include_str!("source.rs"),
        include_str!("../Cargo.toml"),
    ] {
        digest.update(text.replace("\r\n", "\n"));
    }
    format!("{:x}", digest.finalize())
}
fn expected_source_files() -> Result<BTreeMap<String, String>> {
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
struct Policy {
    schema_version: u32,
    id: String,
    release: String,
    source_role: String,
    spark_skill: String,
    mace_skill: String,
    support_skill: String,
    spark_default_class: String,
    mace_default_class: String,
    weapons: Vec<[String; 2]>,
    quests: Vec<String>,
    coverage: Vec<String>,
}
/// Generate all current package sections from authenticated source, never from
/// the embedded package. A separate policy owns selection and conversion rules.
pub fn extract_pinned_game_data(root: &Path) -> Result<ExtractedGameData> {
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
    if policy.schema_version != 1 || policy.quests.len() != 6 || policy.weapons.len() != 2 {
        return Err(error("unsupported extraction selection policy"));
    }
    let extractor = Extractor::new(sources)?;
    let record_function: Function = extractor.lua.globals().get("source_extract_records")?;
    let records: Table = record_function.call(extractor.lua.to_value(&policy)?)?;
    let mut entrance_effects = Vec::new();
    let convert: Function = extractor.lua.globals().get("source_extract_effect")?;
    for (class_id, nodes) in &tree.class_entrances {
        for (physical_node_id, node) in nodes {
            let mut effects = Vec::new();
            for line in &node.stats {
                let value: Value = convert.call(line.as_str())?;
                effects.push(extractor.lua.from_value(value)?);
            }
            entrance_effects.push(EntranceEffects {
                class_id: *class_id,
                physical_node_id: *physical_node_id,
                effective_node_id: node.effective_source_id,
                effects,
            });
        }
    }
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
        quests: extractor.record(&records, "quests")?,
        spark: extractor.record(&records, "spark")?,
        mace: extractor.record(&records, "mace")?,
        weapons: extractor.record(&records, "weapons")?,
        monsters: extractor.record(&records, "monsters")?,
        defence: extractor.defence()?,
        encounters: extractor.encounters()?,
        entrance_effects,
    };
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
    let output = ExtractedGameData { package, evidence };
    output.validate()?;
    Ok(output)
}
fn section<'a>(source: &'a str, begin: &str, end: &str) -> Result<&'a str> {
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
        lua.globals().set("data", data)?;
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
        lua.load(source("src/Classes/ModStore.lua")?).exec()?;
        lua.load(source("src/Classes/ModDB.lua")?).exec()?;
        let gems: Table = lua.load(source("src/Data/Gems.lua")?).eval()?;
        lua.globals().set("sourceGems", gems)?;
        lua.load("data.gems={};data.keystones={};data.skills={}")
            .exec()?;
        let parser: Function = lua.load(source("src/Modules/ModParser.lua")?).eval()?;
        lua.globals()
            .get::<Table>("modLib")?
            .set("parseMod", parser)?;
        lua.load("skills={};mod=modLib.createMod").exec()?;
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
                "{}\nlocal config={{}};addQuestModsRewardsConfigOptions(config);return config",
                section(
                    source("src/Modules/ConfigOptions.lua")?,
                    "local function addQuestModsRewardsConfigOptions(",
                    "\nlocal configSettings = {"
                )?
            ))
            .eval()?;
        lua.globals().set("sourceQuestConfig", config)?;
        let calcs: Table = lua.load(source("src/Modules/CalcDefence.lua")?).eval()?;
        lua.globals().set("sourceCalcs", calcs)?;
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
        ] {
            initialization.push_str(line(source("src/Modules/CalcSetup.lua")?, prefix)?);
            initialization.push('\n');
        }
        initialization.push_str("return modDB end");
        lua.globals().set(
            "sourceResourceInitialization",
            lua.load(initialization).eval::<Function>()?,
        )?;
        let bonuses=lua.load(format!("return function() local modDB=new('ModDB'):ModDB();local output={{Str=1,Dex=1,Int=1}};{}\nreturn modDB end",section(source("src/Modules/CalcPerform.lua")?,"\t-- Add attribute bonuses\n","\t-- Calculate Presence / Surrounded")?)).eval::<Function>()?;
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
