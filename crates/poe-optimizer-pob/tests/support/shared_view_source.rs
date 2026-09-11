//! Four independent authored selector consumers. This is not a full build loader.
//! Numerical item parsing, gem processing, passive allocation, and config modifiers
//! are explicit inert boundaries. No inventory winner or evaluator admission claim.
use mlua::{Function, HookTriggers, Lua, Table, VmState};
use poe_optimizer_pob::source;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::OnceLock,
    time::{Duration, Instant},
};

fn originals() -> &'static BTreeMap<&'static str, String> {
    static TEXT: OnceLock<BTreeMap<&str, String>> = OnceLock::new();
    TEXT.get_or_init(|| {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
        [
            "runtime/lua/xml.lua",
            "src/GameVersions.lua",
            "src/Modules/Common.lua",
            "src/Classes/SkillsTab.lua",
            "src/Classes/ItemsTab.lua",
            "src/Classes/ConfigTab.lua",
            "src/Classes/TreeTab.lua",
            "src/Classes/PassiveSpec.lua",
        ]
        .into_iter()
        .map(|path| (path, source::read_verified_text(&root, path).unwrap()))
        .collect()
    })
}
fn install(lua: &Lua, path: &str, start: &str, end: Option<&str>, prelude: &str) {
    let text = &originals()[path];
    assert_eq!(
        text.matches(start).count(),
        1,
        "unique source anchor {start}"
    );
    let from = text.find(start).unwrap();
    let to = end.map_or(text.len(), |end| from + text[from..].find(end).unwrap());
    let line = text[..from].bytes().filter(|&b| b == b'\n').count();
    assert!(!prelude.contains('\n'));
    lua.load(format!(
        "{prelude}\n{}{}",
        "\n".repeat(line - 1),
        &text[from..to]
    ))
    .set_name(format!("@{path}"))
    .exec()
    .unwrap();
}
pub struct Oracle {
    _lua: Lua,
    helper: Table,
}
impl Oracle {
    pub fn new() -> Self {
        let lua = Lua::new();
        lua.set_memory_limit(128 * 1024 * 1024).unwrap();
        let started = Instant::now();
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(10_000),
            move |_, _| {
                if started.elapsed() > Duration::from_secs(30) {
                    Err(mlua::Error::RuntimeError(
                        "shared view source deadline".into(),
                    ))
                } else {
                    Ok(VmState::Continue)
                }
            },
        )
        .unwrap();
        lua.load("jit.off();SkillsTabClass={};ItemsTabClass={};ConfigTabClass={};TreeTabClass={};PassiveSpecClass={};t_insert=table.insert;t_remove=table.remove;m_min=math.min;m_max=math.max;legacyClassIdMap={};s_upper=string.upper").exec().unwrap();
        lua.globals()
            .set(
                "originalXml",
                lua.load(&originals()["runtime/lua/xml.lua"])
                    .eval::<Table>()
                    .unwrap(),
            )
            .unwrap();
        lua.load(&originals()["src/GameVersions.lua"])
            .set_name("@src/GameVersions.lua")
            .exec()
            .unwrap();
        for (path, start, end, prelude) in [
            (
                "src/Modules/Common.lua",
                "function sanitiseText(text)",
                Some("-- Convert int to 4 bytes string"),
                "",
            ),
            (
                "src/Classes/SkillsTab.lua",
                "function SkillsTabClass:LoadSkill(node, skillSetId)",
                Some("function SkillsTabClass:Save(xml)"),
                "",
            ),
            (
                "src/Classes/SkillsTab.lua",
                "function SkillsTabClass:CreateSkillSet(skillSetId, title)",
                Some("-- Creates a new skill set with title"),
                "",
            ),
            (
                "src/Classes/SkillsTab.lua",
                "function SkillsTabClass:SetActiveSkillSet(skillSetId, deferSync)",
                Some("-- Loop over all socket groups"),
                "",
            ),
            (
                "src/Classes/ItemsTab.lua",
                "function ItemsTabClass:Load(xml, dbFileName)",
                Some("function ItemsTabClass:Save(xml)"),
                "",
            ),
            (
                "src/Classes/ItemsTab.lua",
                "function ItemsTabClass:CreateItemSet(itemSetId, name)",
                Some("function ItemsTabClass:CopyItemSet("),
                "",
            ),
            (
                "src/Classes/ItemsTab.lua",
                "function ItemsTabClass:SetActiveItemSet(itemSetId, deferSync)",
                Some("-- Equips the given item"),
                "",
            ),
            (
                "src/Classes/ConfigTab.lua",
                "function ConfigTabClass:Load(xml, fileName)",
                Some("function ConfigTabClass:GetDefaultState(var, varType)"),
                "",
            ),
            (
                "src/Classes/ConfigTab.lua",
                "function ConfigTabClass:CreateConfigSet(configSetId, title)",
                Some("-- Creates a new config set, adds it"),
                "local varList={}",
            ),
            (
                "src/Classes/ConfigTab.lua",
                "function ConfigTabClass:SetActiveConfigSet(configSetId, init, deferSync)",
                None,
                "",
            ),
            (
                "src/Classes/TreeTab.lua",
                "function TreeTabClass:Load(xml, dbFileName)",
                Some("function TreeTabClass:PostLoad()"),
                "",
            ),
            (
                "src/Classes/TreeTab.lua",
                "function TreeTabClass:SetActiveSpec(specId, deferSync)",
                Some("function TreeTabClass:SetCompareSpec(specId)"),
                "",
            ),
            (
                "src/Classes/PassiveSpec.lua",
                "function PassiveSpecClass:Load(xml, dbFileName)",
                Some("function PassiveSpecClass:Save(xml)"),
                "",
            ),
        ] {
            install(&lua, path, start, end, prelude);
        }
        let helper = lua
            .load(include_str!("shared_view_source.lua"))
            .set_name("@shared-view-source-host.lua")
            .eval()
            .unwrap();
        Self { _lua: lua, helper }
    }
    pub fn observe(&self, xml: &str) -> Table {
        self.helper
            .get::<Function>("observe")
            .unwrap()
            .call(xml)
            .unwrap()
    }
}
