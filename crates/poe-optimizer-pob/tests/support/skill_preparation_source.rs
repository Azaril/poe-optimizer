//! Independent original-source skill preparation oracle. The executed source bodies
//! are verified against the pinned submodule; only UI controls and observations live here.
use mlua::{Function, HookTriggers, Lua, Table, Value, VmState};
use poe_optimizer_pob::source;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::OnceLock,
    time::{Duration, Instant},
};
pub fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn sources() -> &'static BTreeMap<String, String> {
    static SOURCES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    SOURCES.get_or_init(|| {
        let mut paths = vec![
            "runtime/lua/xml.lua".into(),
            "src/Modules/Common.lua".into(),
            "src/Modules/Data.lua".into(),
            "src/Modules/CalcTools.lua".into(),
            "src/Classes/SkillsTab.lua".into(),
            "src/Classes/DropDownControl.lua".into(),
            "src/Classes/EditControl.lua".into(),
            "src/Data/Global.lua".into(),
            "src/Data/Gems.lua".into(),
            "src/Data/SkillStatMap.lua".into(),
            "src/Data/Assets.lua".into(),
            "src/Data/Skills/SkillAssets.lua".into(),
        ];
        for name in [
            "act_str", "act_dex", "act_int", "other", "minion", "spectre", "sup_str", "sup_dex",
            "sup_int",
        ] {
            paths.push(format!("src/Data/Skills/{name}.lua"));
        }
        paths
            .into_iter()
            .map(|path: String| {
                let text = source::read_verified_text(
                    &repository().join("vendor/path-of-building-poe2"),
                    &path,
                )
                .unwrap();
                (path, text)
            })
            .collect()
    })
}
fn original(path: &str) -> &'static str {
    &sources()[path]
}
fn section<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    assert_eq!(text.matches(start).count(), 1, "source start {start}");
    let from = text.find(start).unwrap();
    let to = from + text[from..].find(end).unwrap();
    &text[from..to]
}
pub struct Oracle {
    lua: Lua,
    helpers: Table,
}
impl Oracle {
    pub fn new(warm: bool) -> Self {
        let lua = Lua::new();
        lua.set_memory_limit(512 * 1024 * 1024).unwrap();
        let start = Instant::now();
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(100_000),
            move |_, _| {
                if start.elapsed() > Duration::from_secs(90) {
                    Err(mlua::Error::RuntimeError(
                        "skill source oracle deadline exceeded".into(),
                    ))
                } else {
                    Ok(VmState::Continue)
                }
            },
        )
        .unwrap();
        lua.load(if warm { "jit.on()" } else { "jit.off()" })
            .exec()
            .unwrap();
        lua.load("t_insert=table.insert;t_remove=table.remove;m_min=math.min;m_max=math.max;m_floor=math.floor;SkillsTabClass={};DropDownClass={};EditClass={};main={defaultGemQuality=0};data={};calcLib={};observedSkillModules={}").exec().unwrap();
        for (path, start, end) in [
            (
                "src/Classes/SkillsTab.lua",
                "function SkillsTabClass:GetCorruptIndex(gemInstance)",
                "function SkillsTabClass:LoadSkill(",
            ),
            (
                "src/Classes/SkillsTab.lua",
                "function SkillsTabClass:SetDisplayGroup(socketGroup)",
                "function SkillsTabClass:AddSocketGroupTooltip(",
            ),
            (
                "src/Modules/Common.lua",
                "function sanitiseText(text)",
                "-- Convert int to 4 bytes string",
            ),
            (
                "src/Modules/Common.lua",
                "function copyTable(tbl, noRecurse)",
                "do\n\tlocal subTableMap",
            ),
            (
                "src/Modules/Common.lua",
                "function tableConcat(t1,t2)",
                "--- Simple table value equality",
            ),
            (
                "src/Modules/Common.lua",
                "function round(val, dec)",
                "--- Rounds down a number",
            ),
            (
                "src/Classes/SkillsTab.lua",
                "function SkillsTabClass:LoadSkill(node, skillSetId)",
                "function SkillsTabClass:Draw(",
            ),
            (
                "src/Classes/SkillsTab.lua",
                "function SkillsTabClass:FindSkillGem(nameSpec)",
                "-- Set the skill to be displayed/edited",
            ),
            (
                "src/Classes/SkillsTab.lua",
                "function SkillsTabClass:CreateSkillSet(skillSetId, title)",
                "-- Creates a new skill set with title",
            ),
            (
                "src/Classes/SkillsTab.lua",
                "function SkillsTabClass:SetActiveSkillSet(skillSetId, deferSync)",
                "-- Loop over all socket groups",
            ),
            (
                "src/Classes/DropDownControl.lua",
                "function DropDownClass:SelByValue(value, key)",
                "function DropDownClass:GetSelValue()",
            ),
            (
                "src/Classes/EditControl.lua",
                "function EditClass:SetText(text, notify)",
                "function EditClass:SetPlaceholder(",
            ),
            (
                "src/Modules/CalcTools.lua",
                "function calcLib.validateGemLevel(gemInstance)",
                "local typeExpressionStack",
            ),
            (
                "src/Modules/CalcTools.lua",
                "function calcLib.getGemStatRequirement(level, multi, isSupport)",
                "-- Build table of stats for the given skill instance statset",
            ),
        ] {
            lua.load(section(original(path), start, end))
                .set_name(format!("@{path}"))
                .exec()
                .unwrap();
        }
        lua.globals()
            .set(
                "LoadModule",
                lua.create_function(|lua, name: String| {
                    let path = format!("src/{name}.lua");
                    if name.starts_with("Data/Skills/") && name != "Data/Skills/SkillAssets" {
                        lua.globals()
                            .get::<Table>("observedSkillModules")?
                            .raw_push(path.clone())?;
                    }
                    let text = sources().get(&path).ok_or_else(|| {
                        mlua::Error::RuntimeError(format!("unapproved oracle dependency {path}"))
                    })?;
                    lua.load(text).set_name(format!("@{path}")).eval::<Value>()
                })
                .unwrap(),
            )
            .unwrap();
        let data_source = original("src/Modules/Data.lua");
        lua.load(original("src/Data/Global.lua"))
            .set_name("@src/Data/Global.lua")
            .exec()
            .unwrap();
        let assembly = format!(
            "{}\n{}\n{}",
            section(data_source, "local skillTypes = {", "local itemTypes = {"),
            section(
                data_source,
                "local function makeSkillMod(",
                "-----------------\n-- Common Data"
            ),
            section(data_source, "-- Load skills\n", "-- Load minions\n")
        );
        let assembly_helpers: Table = lua.load(format!("{assembly}\nreturn {{setupGem=setupGem,mod=makeSkillMod,flag=makeFlagMod,skill=makeSkillDataMod}}"))
            .set_name("@src/Modules/Data.lua")
            .eval()
            .unwrap();
        lua.globals()
            .set("originalAssemblyHelpers", assembly_helpers)
            .unwrap();
        let skills = original("src/Classes/SkillsTab.lua");
        lua.load(format!("function original_skill_defaults(self)\n{}\nend\nfunction original_control_lists()\n{}\nreturn {{groupSlotDropList=groupSlotDropList,defaultGemLevelList=defaultGemLevelList,showSupportGemTypeList=showSupportGemTypeList,sortGemTypeList=sortGemTypeList}} end",section(skills,"\tself.socketGroupList = { }","\t-- Set selector"),section(skills,"local groupSlotDropList = {","---@class SkillsTab:"))).exec().unwrap();
        let xml: Table = lua.load(original("runtime/lua/xml.lua")).eval().unwrap();
        lua.globals().set("originalXml", xml).unwrap();
        let helpers = lua
            .load(include_str!("skill_preparation_source.lua"))
            .eval()
            .unwrap();
        Self { lua, helpers }
    }
    pub fn observe(&self, xml: &str, selected: Option<f64>) -> Table {
        let call: Function = self.helpers.get("load").unwrap();
        call.call((xml, selected)).unwrap()
    }
    pub fn mutate(&self, mutation: &str) {
        self.lua.load(mutation).exec().unwrap();
    }
}
