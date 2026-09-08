//! Original Skills XML and identity observations, separate from native coverage.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Function, HookTriggers, Lua, Table, Value, VmState};
use poe_optimizer_data::{
    game_data::bundled_snapshot,
    skill_identities::{
        GemIdentityResolutionStatus, IdentitySourceSpan, IndexedIdentityReference,
        MissingIdentityReference, MissingIdentityReferenceKind,
    },
};
use poe_optimizer_import::{
    skill_definitions::{InstanceIdentityResolution, SkillIdentityRecord, lookup_definitions},
    skill_source::{self, SkillDiagnosticCode, SkillSourceKind, SkillSourceNode, SkillSourceUse},
};
use poe_optimizer_pob::source;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::OnceLock,
    time::{Duration, Instant},
};

fn repository() -> PathBuf {
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
struct Oracle {
    lua: Lua,
    helpers: Table,
    rounds: usize,
}
impl Oracle {
    fn new(warm: bool) -> Self {
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
            .load(include_str!("support/skill_source_oracle.lua"))
            .eval()
            .unwrap();
        Self {
            lua,
            helpers,
            rounds: if warm { 60 } else { 1 },
        }
    }
    fn load(&self, xml: &str, processed: bool) -> Table {
        let call: Function = self.helpers.get("load").unwrap();
        let mut result = None;
        for _ in 0..self.rounds {
            result = Some(call.call::<Table>((xml, processed)).unwrap());
        }
        result.unwrap()
    }
    fn data(&self) -> Table {
        self.lua.globals().get("data").unwrap()
    }
}
fn table(t: &Table, key: &str) -> Table {
    t.get(key).unwrap()
}
fn text(t: &Table, key: &str) -> String {
    t.get(key).unwrap()
}
fn number(t: &Table, key: &str) -> f64 {
    t.get(key).unwrap()
}
fn first_gem(tab: &Table) -> Table {
    let groups = table(tab, "socketGroupList");
    let group: Table = groups.get(1).unwrap();
    let gems = table(&group, "gemList");
    gems.get(1).unwrap()
}

#[test]
fn full_original_data_constructs_identity_graph_and_all_corpus_sets_load() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        let data = o.data();
        assert!(table(&data, "gems").pairs::<String, Table>().count() > 400);
        assert!(table(&data, "skills").pairs::<String, Table>().count() > 800);
        for index in 1..=5 {
            let xml = std::fs::read_to_string(repository().join(format!(
                "tests/fixtures/builds/breadth-20260908/build-{index:02}.xml"
            )))
            .unwrap();
            let raw = o.load(&xml, false);
            let processed = o.load(&xml, true);
            assert_eq!(
                number(&raw, "activeSkillSetId"),
                number(&processed, "activeSkillSetId")
            );
            assert_eq!(
                table(&raw, "skillSetOrderList").raw_len(),
                table(&processed, "skillSetOrderList").raw_len()
            );
        }
    }
}

fn injected_data(o: &Oracle) -> Table {
    o.lua.load(r#"
 local effects={}
 for _,id in ipairs({'One','Two'})do effects[id]={id=id,name='Effect '..id,modSource='Skill:'..id,color=3,levels={{levelRequirement=1}},statSets={}}end
 local gems={}
 for _,id in ipairs({'One','Two'})do gems['internal-'..id]={id='internal-'..id,name='Gem '..id,gameId='external-'..id,variantId='variant-'..id,grantedEffectId=id,grantedEffect=effects[id],naturalMaxLevel=1,reqStr=0,reqDex=0,reqInt=0}end
 return {skills=effects,gems=gems,gemForSkill={[effects.One]='internal-One',[effects.Two]='internal-Two'},gemsByGameId={['external-One']={['variant-One']=gems['internal-One']},['external-Two']={['variant-Two']=gems['internal-Two']},['external-many']={['variant-One']=gems['internal-One'],['variant-Two']=gems['internal-Two']}}}
 "#).eval().unwrap()
}
fn injected_load(o: &Oracle, attributes: &str, processed: bool) -> Table {
    let load: Function = o.helpers.get("load").unwrap();
    let xml = format!(
        "<Skills><SkillSet id=\"1\"><Skill enabled=\"true\"><Gem level=\"1\" {attributes}/></Skill></SkillSet></Skills>"
    );
    load.call((xml, processed, injected_data(o))).unwrap()
}
#[test]
fn original_external_identity_precedence_and_fallback_do_not_invent_a_sorted_variant() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        for (attributes, expected) in [
            (
                r#"gemId="external-One" variantId="variant-One" skillId="Two""#,
                Some("One"),
            ),
            (
                r#"gemId="external-One" variantId="missing" skillId="Two""#,
                Some("One"),
            ),
            (r#"gemId="external-One" skillId="Two""#, Some("One")),
            (r#"gemId="missing" skillId="Two""#, None),
            (r#"gemId="" skillId="Two""#, None),
            (r#"skillId="Two""#, Some("Two")),
            (r#"skillId="missing""#, None),
        ] {
            let tab = injected_load(&o, attributes, false);
            let gem = first_gem(&tab);
            assert_eq!(
                gem.get::<Option<String>>("skillId").unwrap().as_deref(),
                expected,
                "{attributes}"
            );
            if let Some(id) = expected {
                assert_eq!(text(&gem, "gemId"), format!("internal-{id}"));
            }
        }
        for variant in ["", " variantId=\"unknown\""] {
            let gem = first_gem(&injected_load(
                &o,
                &format!("gemId=\"external-many\"{variant}"),
                false,
            ));
            assert!(["One", "Two"].contains(&text(&gem, "skillId").as_str()));
        }
        let raw = first_gem(&injected_load(
            &o,
            r#"gemId="missing" skillId="Two" nameSpec="Gem One""#,
            false,
        ));
        assert!(matches!(raw.get::<Value>("skillId").unwrap(), Value::Nil));
        let processed = first_gem(&injected_load(
            &o,
            r#"gemId="missing" skillId="Two" nameSpec="Gem One""#,
            true,
        ));
        assert_eq!(
            text(&processed, "skillId"),
            "One",
            "name matching belongs to later ProcessSocketGroup"
        );
        let raw = first_gem(&injected_load(&o, r#"skillId="Two""#, false));
        assert_eq!(text(&raw, "nameSpec"), "Effect Two");
        let processed = first_gem(&injected_load(&o, r#"skillId="Two""#, true));
        assert_eq!(text(&processed, "nameSpec"), "Gem Two");
    }
}

#[test]
fn original_loadskill_resets_scalar_statsets_and_consumes_any_child_tag() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        let xml = r#"<Skills><Skill enabled="false" active="true" skillPart="8" mainActiveSkill="2" mainActiveSkillCalcs="3"><FutureGem nameSpec="&lt;i&gt;A&lt;/i&gt;—B ö" note="raw &lt;i&gt;note&lt;/i&gt;—ö" level="0" quality="-3" count="0" statSetIndex="8" statSetIndexCalcs="9" skillPart="2" skillPartCalcs="5" enableGlobal1="false" enableGlobal2="true">
<StatSetIndex grantedEffect="One" index="2"/><StatSetIndex grantedEffect="One" index="7"/><StatSetCalcsIndex grantedEffect="One" index="3"/>
<MinionSkillIndexLookup grantedEffect="Minion"><Anything skillIndex="2" statSetIndex="4"/></MinionSkillIndexLookup>
<MinionSkillIndexLookup grantedEffect="Minion"><Different skillIndex="3" statSetIndex="6"/></MinionSkillIndexLookup>
<MinionSkillIndexLookupCalcs grantedEffect="Minion"><Unknown skillIndex="2" statSetIndex="8"/></MinionSkillIndexLookupCalcs>
</FutureGem></Skill></Skills>"#;
        let tab = o.load(xml, false);
        let gem = first_gem(&tab);
        let group: Table = table(&tab, "socketGroupList").get(1).unwrap();
        assert!(group.get::<bool>("enabled").unwrap());
        assert_eq!(number(&group, "mainActiveSkill"), 2.0);
        assert_eq!(number(&group, "mainActiveSkillCalcs"), 3.0);
        assert_eq!(text(&gem, "nameSpec"), "A-B o");
        assert_eq!(text(&gem, "note"), "raw <i>note</i>—ö");
        assert_eq!(number(&gem, "skillPart"), 8.0);
        assert_eq!(number(&gem, "skillPartCalcs"), 5.0);
        assert_eq!(number(&gem, "level"), 0.0);
        assert_eq!(number(&gem, "quality"), -3.0);
        assert_eq!(number(&gem, "count"), 0.0);
        assert!(!gem.get::<bool>("enableGlobal1").unwrap());
        assert!(gem.get::<bool>("enableGlobal2").unwrap());
        let main = table(&gem, "statSet");
        let calcs = table(&gem, "statSetCalcs");
        assert!(matches!(main.get::<Value>("index").unwrap(), Value::Nil));
        assert!(matches!(calcs.get::<Value>("index").unwrap(), Value::Nil));
        assert_eq!(number(&main, "One"), 7.0);
        assert_eq!(number(&calcs, "One"), 3.0);
        let lookup = table(&table(&gem, "skillMinionSkillStatSetIndexLookup"), "Minion");
        assert!(matches!(lookup.get::<Value>(2).unwrap(), Value::Nil));
        assert_eq!(lookup.get::<f64>(3).unwrap(), 6.0);
        let calcs_lookup = table(
            &table(&gem, "skillMinionSkillStatSetIndexLookupCalcs"),
            "Minion",
        );
        assert_eq!(calcs_lookup.get::<f64>(2).unwrap(), 8.0);
        let save: Function = o.helpers.get("save").unwrap();
        let (_, saved): (Table, String) = save.call(tab).unwrap();
        let document = roxmltree::Document::parse(&saved).unwrap();
        let saved_gem = document
            .descendants()
            .find(|n| n.has_tag_name("Gem"))
            .unwrap();
        assert_eq!(saved_gem.attribute("statSetIndex"), Some("nil"));
        assert_eq!(saved_gem.attribute("statSetIndexCalcs"), Some("nil"));
        assert_eq!(saved_gem.attribute("note"), Some("raw <i>note</i>—ö"));
        let load: Function = o.helpers.get("load").unwrap();
        for body in [
            "<Skill>text<Gem/></Skill>",
            "<Skill><Gem><MinionSkillIndexLookup grantedEffect=\"x\"><Anything skillIndex=\"bad\" statSetIndex=\"1\"/></MinionSkillIndexLookup></Gem></Skill>",
        ] {
            assert!(
                load.call::<Table>((format!("<Skills>{body}</Skills>"), false))
                    .is_err()
            );
        }
    }
}

#[test]
fn original_ordered_sets_duplicate_winners_and_legacy_mixing_are_not_source_projection() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        for (body, active, order) in [
            ("", 1.0, vec![1.0]),
            ("<SkillSet id=\"not-numeric\"/>", 1.0, vec![1.0]),
            (
                "<SkillSet id=\"2\"/><SkillSet id=\"1\"/>",
                1.0,
                vec![2.0, 1.0],
            ),
            (
                "<SkillSet id=\"2\"/><SkillSet id=\"3\"/>",
                2.0,
                vec![2.0, 3.0],
            ),
            (
                "<SkillSet id=\"1\" title=\"discarded\"><Skill label=\"discarded\"/></SkillSet><SkillSet id=\"01\" title=\"winner\"><Skill label=\"winner\"/></SkillSet>",
                1.0,
                vec![1.0, 1.0],
            ),
            (
                "<Skill label=\"legacy\"/><SkillSet id=\"1\" title=\"winner\"><Skill label=\"winner\"/></SkillSet>",
                1.0,
                vec![1.0, 1.0],
            ),
            ("<SkillSet id=\"-2.5\"/>", -2.5, vec![-2.5]),
        ] {
            let tab = o.load(&format!("<Skills>{body}</Skills>"), false);
            assert_eq!(number(&tab, "activeSkillSetId"), active, "{body}");
            let got: Vec<f64> = table(&tab, "skillSetOrderList")
                .sequence_values()
                .map(Result::unwrap)
                .collect();
            assert_eq!(got, order, "{body}");
            if body.contains("winner") {
                let set: Table = table(&tab, "skillSets").get(1).unwrap();
                assert_eq!(text(&set, "title"), "winner");
                let save: Function = o.helpers.get("save").unwrap();
                let (saved, _): (Table, String) = save.call(tab).unwrap();
                assert_eq!(saved.raw_len(), 2);
                for child in saved.sequence_values::<Table>() {
                    assert_eq!(text(&table(&child.unwrap(), "attrib"), "title"), "winner");
                }
            }
        }
        let tab = o.load(
            "<Skills activeSkillSet=\"2\"><SkillSet id=\"1\"/><SkillSet id=\"2\"/></Skills>",
            false,
        );
        assert_eq!(number(&tab, "activeSkillSetId"), 2.0);
        let load: Function = o.helpers.get("load").unwrap();
        assert!(
            load.call::<Table>(("<Skills><SkillSet id=\"2\"/><Skill/></Skills>", false))
                .is_err()
        );
    }
}

fn exact_projection(node: &SkillSourceNode<'_>, actual: &Table, xml: &str) -> usize {
    let element = node.element();
    assert_eq!(element.source_xml(), &xml[element.source_range()]);
    assert_eq!(element.name(), text(actual, "elem"));
    let attributes = table(actual, "attrib");
    assert_eq!(
        element.attributes().len(),
        attributes.clone().pairs::<String, String>().count()
    );
    for attribute in element.attributes() {
        assert_eq!(attribute.value().raw(), &xml[attribute.value().range()]);
        assert_eq!(
            attribute.value().decoded(),
            text(&attributes, attribute.name())
        );
    }
    let children: Vec<Table> = actual
        .clone()
        .sequence_values::<Value>()
        .filter_map(|v| match v.unwrap() {
            Value::Table(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(children.len(), node.children().len());
    1 + node
        .children()
        .iter()
        .zip(children)
        .map(|(projected, source)| exact_projection(projected, &source, xml))
        .sum::<usize>()
}
fn has_diagnostic(node: &SkillSourceNode<'_>, code: SkillDiagnosticCode) -> bool {
    node.diagnostics().iter().any(|d| d.code() == code)
        || node.children().iter().any(|n| has_diagnostic(n, code))
}
#[test]
fn portable_projection_matches_original_xml_for_every_corpus_skill_node() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        let parse: Function = o.helpers.get("node").unwrap();
        for index in 1..=5 {
            let xml = std::fs::read_to_string(repository().join(format!(
                "tests/fixtures/builds/breadth-20260908/build-{index:02}.xml"
            )))
            .unwrap();
            let projection = skill_source::project_xml(&xml).unwrap();
            assert_eq!(projection.source_xml(), xml);
            assert_eq!(
                projection.source_sha256(),
                format!("{:x}", Sha256::digest(xml.as_bytes()))
            );
            assert_eq!(projection.containers().len(), 1);
            let parsed: Table = parse.call(xml.as_str()).unwrap();
            assert!(exact_projection(&projection.containers()[0], &parsed, &xml) > 20);
            let raw = o.load(&xml, false);
            let ordered = table(&raw, "skillSetOrderList");
            let container = &projection.containers()[0];
            let saved: Vec<_> = container
                .children()
                .iter()
                .filter(|n| n.source_use() == SkillSourceUse::SavedSet)
                .collect();
            assert_eq!(saved.len(), ordered.raw_len());
            for (index, node) in saved.iter().enumerate() {
                let id: f64 = ordered.get(index + 1).unwrap();
                assert_eq!(
                    node.element()
                        .attribute("id")
                        .unwrap()
                        .decoded()
                        .parse::<f64>()
                        .unwrap(),
                    id
                );
                let set: Table = table(&raw, "skillSets").get(id).unwrap();
                assert_eq!(
                    node.children()
                        .iter()
                        .filter(|n| n.source_use() == SkillSourceUse::Group)
                        .count(),
                    table(&set, "socketGroupList").raw_len()
                );
            }
        }
    }
}
#[test]
fn projection_keeps_adversarial_source_without_applying_lossy_loader_rules() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        let parse: Function = o.helpers.get("node").unwrap();
        for newline in ["\n", "\r\n"] {
            let xml = format!(
                "<PathOfBuilding2><Skills activeSkillSet=\"missing\" defaultGemQuality=\"99\"><Skill label=\"legacy\"><FutureGem nameSpec=\"A{newline}\tB &amp; &lt; &gt; &quot; &apos;\" note=\"raw{newline}\tnote\" count=\"invalid\" enabled=\"TRUE\" statSetIndex=\"9\"><StatSetIndex grantedEffect=\"One\" index=\"1\"/><StatSetIndex grantedEffect=\"One\" index=\"2\"/><MinionSkillIndexLookup grantedEffect=\"M\"><Anything skillIndex=\"2\" statSetIndex=\"3\"/></MinionSkillIndexLookup></FutureGem></Skill><SkillSet id=\"1\" title=\"first\"/><SkillSet id=\"01\" title=\"last\"/></Skills></PathOfBuilding2>"
            );
            let projection = skill_source::project_xml(&xml).unwrap();
            let node = &projection.containers()[0];
            let parsed: Table = parse.call(xml.as_str()).unwrap();
            exact_projection(node, &parsed, &xml);
            for code in [
                SkillDiagnosticCode::MixedLegacyAndSavedSets,
                SkillDiagnosticCode::LegacyDirectGroup,
                SkillDiagnosticCode::DuplicateNumericSetId,
                SkillDiagnosticCode::PositionalGemChild,
                SkillDiagnosticCode::PositionalMinionMapChild,
                SkillDiagnosticCode::LegacyStatSetAttributeReset,
                SkillDiagnosticCode::DuplicateLookupKey,
                SkillDiagnosticCode::NonCanonicalBoolean,
                SkillDiagnosticCode::NumericOutsideFiniteDecimal,
            ] {
                assert!(has_diagnostic(node, code), "{code:?}");
            }
            let gem = &node.children()[0].children()[0];
            assert_eq!(gem.kind(), SkillSourceKind::Unknown);
            assert_eq!(gem.source_use(), SkillSourceUse::GemInstance);
            assert_eq!(
                gem.element().attribute("nameSpec").unwrap().decoded(),
                format!("A{newline}\tB & < > \" '")
            );
            assert_eq!(
                gem.children()[2].children()[0].source_use(),
                SkillSourceUse::MinionIndexMap
            );
            let raw = o.load(&xml, false);
            assert_eq!(number(&raw, "defaultGemQuality"), 23.0);
            assert_eq!(number(&raw, "activeSkillSetId"), 1.0);
            assert_eq!(
                node.element()
                    .attribute("defaultGemQuality")
                    .unwrap()
                    .decoded(),
                "99"
            );
            let before: Table = table(&raw, "observed_before_process").get(1).unwrap();
            let original_gem: Table = table(&before, "gemList").get(1).unwrap();
            assert_eq!(number(&original_gem, "count"), 1.0);
            assert!(!original_gem.get::<bool>("enabled").unwrap());
            assert_eq!(
                gem.element().attribute("count").unwrap().decoded(),
                "invalid"
            );
        }
    }
    for xml in [
        "<PathOfBuilding2><Skills><Skill>text<Gem/></Skill></Skills></PathOfBuilding2>",
        "<PathOfBuilding2><Skills><SkillSet id=\"nonnumeric\"/></Skills><Skills/></PathOfBuilding2>",
    ] {
        let p = skill_source::project_xml(xml).unwrap();
        assert_eq!(p.source_xml(), xml);
        assert!(p.containers().iter().any(|n| !n.diagnostics().is_empty()
            || n.children().iter().any(|c| !c.diagnostics().is_empty())));
    }
    let xml = "<PathOfBuilding2><Skills xmlns=\"foreign\"><SkillSet id=\"1\"><Skill><Gem/></Skill></SkillSet></Skills></PathOfBuilding2>";
    let p = skill_source::project_xml(xml).unwrap();
    assert_eq!(
        p.containers()[0].source_use(),
        SkillSourceUse::NamespaceUnknown
    );
    for body in [
        "<Skills activeSkillSet=\"&#49;\"/>",
        "<Skills activeSkillSet =\"1\"/>",
        "<Skills value=\"x>y\"/>",
        "<Skills activeSkillSet=\"1\" activeSkillSet=\"2\"/>",
    ] {
        assert!(
            skill_source::project_xml(&format!("<PathOfBuilding2>{body}</PathOfBuilding2>"))
                .is_err()
        );
    }
}

#[test]
fn original_setupgem_preserves_generated_effects_display_order_and_missing_reference_behavior() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        let execute:Function=o.lua.load(r#"
return function(name,order,missing)
 local originalData=data
 data={skills={},gemsByGameId={},gemForSkill={},gemNameForModSource={},gemForBaseName={}}
 for _,id in ipairs({'Primary','Second','Third','TriggeredBarbsPlayer','TriggeredBarbsPlayerTwo','TriggeredBarbsPlayerThree'})do data.skills[id]={id=id,name=id=='Second'and 'Command: {0} 'or id,modSource='Skill:'..id,levels={{}},support=false}end
 local gem={name=name,gameId='authored-game',variantId='authored-variant',grantedEffectId='Primary',naturalMaxLevel=1,additionalGrantedEffectId1=missing and 'Missing'or'Second',additionalGrantedEffectId2='Third',grantedEffectDisplayOrder=order}
 originalAssemblyHelpers.setupGem(gem,'authored-internal')
 local result={gem=gem,data=data};data=originalData;return result
end
"#).eval().unwrap();
        for (name, generated) in [
            ("Arbitrary caller gem", None),
            ("Barbs I", Some("TriggeredBarbsPlayer")),
            ("Barbs II", Some("TriggeredBarbsPlayerTwo")),
            ("Barbs III", Some("TriggeredBarbsPlayerThree")),
        ] {
            let result: Table = execute.call((name, Value::Nil, false)).unwrap();
            let gem = table(&result, "gem");
            let data = table(&result, "data");
            assert_eq!(text(&gem, "id"), "authored-internal");
            assert_eq!(
                text(&table(&table(&data, "skills"), "Second"), "name"),
                "Command"
            );
            let effects: Vec<String> = table(&gem, "grantedEffectList")
                .sequence_values::<Table>()
                .map(|v| text(&v.unwrap(), "id"))
                .collect();
            let mut expected = vec!["Primary", "Second", "Third"];
            if let Some(id) = generated {
                expected.push(id);
                assert_eq!(text(&gem, "additionalGrantedEffectId3"), id);
            }
            assert_eq!(effects, expected);
            let primary = table(&table(&data, "skills"), "Primary");
            assert_eq!(
                table(&data, "gemForSkill").get::<String>(primary).unwrap(),
                "authored-internal"
            );
            assert!(matches!(
                table(&data, "gemForSkill").get::<Value>("Primary").unwrap(),
                Value::Nil
            ));
        }
        for (order, expected) in [
            (vec![2, 0, 2], vec!["Third", "Primary", "Third", "Second"]),
            (vec![99], vec!["Primary", "Second", "Third"]),
            (vec![1], vec!["Second", "Primary", "Third"]),
        ] {
            let order = o.lua.create_sequence_from(order).unwrap();
            let result: Table = execute
                .call(("Arbitrary caller gem", order, false))
                .unwrap();
            let effects: Vec<String> = table(&table(&result, "gem"), "grantedEffectList")
                .sequence_values::<Table>()
                .map(|v| text(&v.unwrap(), "id"))
                .collect();
            assert_eq!(effects, expected);
        }
        let result: Table = execute
            .call(("Arbitrary caller gem", Value::Nil, true))
            .unwrap();
        let gem = table(&result, "gem");
        assert_eq!(text(&gem, "additionalGrantedEffectId1"), "Missing");
        let effects: Vec<String> = table(&gem, "grantedEffectList")
            .sequence_values::<Table>()
            .map(|v| text(&v.unwrap(), "id"))
            .collect();
        assert_eq!(effects, vec!["Primary", "Third"]);
    }
}

fn span_text(span: &IdentitySourceSpan) -> String {
    let text = &sources()[&span.path];
    let excerpt: String = text
        .split_inclusive('\n')
        .skip(span.line as usize - 1)
        .take((span.end_line - span.line + 1) as usize)
        .collect();
    assert_eq!(
        format!("{:x}", Sha256::digest(excerpt.as_bytes())),
        span.sha256,
        "{}:{}",
        span.path,
        span.line
    );
    excerpt
}
fn indexed(t: &Table, prefix: &str) -> Vec<IndexedIdentityReference> {
    let mut refs: Vec<_> = t
        .clone()
        .pairs::<Value, Value>()
        .filter_map(|entry| {
            let (key, value) = entry.unwrap();
            let Value::String(key) = key else { return None };
            let name = key.to_str().unwrap();
            let index = name.strip_prefix(prefix)?.parse::<u32>().ok()?;
            let Value::String(value) = value else {
                panic!("source indexed reference is not a string")
            };
            Some(IndexedIdentityReference {
                index,
                id: value.to_str().unwrap().to_owned(),
            })
        })
        .collect();
    refs.sort_by_key(|r| r.index);
    refs
}
fn effect_ids(t: Table) -> Vec<String> {
    t.sequence_values::<Table>()
        .map(|row| text(&row.unwrap(), "id"))
        .collect()
}
fn optional_order(t: &Table) -> Option<Vec<i64>> {
    t.get::<Option<Table>>("grantedEffectDisplayOrder")
        .unwrap()
        .map(|v| v.sequence_values().map(Result::unwrap).collect())
}
#[test]
fn every_catalog_declaration_and_constructed_identity_matches_independent_original_source() {
    let snapshot = bundled_snapshot().unwrap();
    let catalog = snapshot.skill_identities();
    let expected = catalog.data();
    assert_eq!(expected.source.upstream_revision, source::UPSTREAM_REVISION);
    for (path, hash) in &expected.source.files {
        assert_eq!(*hash, source::expected_file_sha256(path).unwrap());
    }
    span_text(&expected.source.skill_assembly);
    span_text(&expected.source.gem_assembly);
    for warm in [false, true] {
        let o = Oracle::new(warm);
        let module_order: Vec<String> = table(&o.lua.globals(), "observedSkillModules")
            .sequence_values()
            .map(Result::unwrap)
            .collect();
        assert_eq!(module_order, expected.source.skill_module_order);
        let declared_skills: Vec<_> = module_order
            .iter()
            .flat_map(|path| {
                original(path)
                    .lines()
                    .enumerate()
                    .filter_map(move |(line, text)| {
                        text.trim_start()
                            .strip_prefix("skills[\"")
                            .and_then(|rest| rest.split_once("\"] ="))
                            .map(|(key, _)| (path.as_str(), key, line as u32 + 1))
                    })
            })
            .collect();
        assert_eq!(declared_skills.len(), expected.skill_declarations.len());
        for ((path, key, line), row) in declared_skills.iter().zip(&expected.skill_declarations) {
            assert_eq!(
                (*path, *key, *line),
                (row.source.path.as_str(), row.id.as_str(), row.source.line)
            );
        }
        let declared_gems: Vec<_> = original("src/Data/Gems.lua")
            .lines()
            .enumerate()
            .filter_map(|(line, text)| {
                text.strip_prefix("\t[\"")
                    .and_then(|rest| rest.split_once("\"] ="))
                    .map(|(key, _)| (key, line as u32 + 1))
            })
            .collect();
        assert_eq!(declared_gems.len(), expected.gem_declarations.len());
        for ((key, line), row) in declared_gems.iter().zip(&expected.gem_declarations) {
            assert_eq!((*key, *line), (row.key.as_str(), row.source.line));
        }
        // Run each unchanged module with a table proxy to observe actual ordered
        // writes, including overwrites. The complete Data construction above uses
        // ordinary tables independently; this trace supplies provenance only.
        let trace: Table = o
            .lua
            .load(
                r#"
            local writes,winners,values,records={},{},{},{}
            local proxy=setmetatable({},{__index=values,__newindex=function(_,key,value)
                values[key]=value;writes[#writes+1]=key;winners[key]=#writes;records[#records+1]=value
            end})
            return {writes=writes,winners=winners,records=records,proxy=proxy}
        "#,
            )
            .eval()
            .unwrap();
        let helpers = table(&o.lua.globals(), "originalAssemblyHelpers");
        for path in &module_order {
            let module: Function = o.lua.load(original(path)).eval().unwrap();
            module
                .call::<()>((
                    table(&trace, "proxy"),
                    helpers.get::<Function>("mod").unwrap(),
                    helpers.get::<Function>("flag").unwrap(),
                    helpers.get::<Function>("skill").unwrap(),
                ))
                .unwrap();
        }
        let traced: Vec<String> = table(&trace, "writes")
            .sequence_values()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            traced,
            expected
                .skill_declarations
                .iter()
                .map(|row| row.id.clone())
                .collect::<Vec<_>>()
        );
        let raw_gems: Table = o.lua.load(original("src/Data/Gems.lua")).eval().unwrap();
        let actual = o.data();
        let gems = table(&actual, "gems");
        let skills = table(&actual, "skills");
        let mut missing = Vec::new();
        assert_eq!(
            gems.clone().pairs::<String, Table>().count(),
            expected.gems.len()
        );
        assert_eq!(
            skills.clone().pairs::<String, Table>().count(),
            expected.skills.len()
        );
        let load_span = &expected.source.load_skill;
        o.lua
            .load(format!(
                "{}{}",
                "\n".repeat(load_span.line as usize - 1),
                span_text(load_span)
            ))
            .set_name(format!("@{}", load_span.path))
            .exec()
            .unwrap();
        let info = table(&o.lua.globals(), "SkillsTabClass")
            .get::<Function>("LoadSkill")
            .unwrap()
            .info();
        assert_eq!(info.line_defined, Some(load_span.line as usize));
        assert_eq!(info.last_line_defined, Some(load_span.end_line as usize));
        for row in &expected.gem_declarations {
            let raw: Table = o
                .lua
                .load(format!("return {{ {} }}", span_text(&row.source)))
                .eval()
                .unwrap();
            assert_eq!(raw.clone().pairs::<String, Table>().count(), 1);
            let raw: Table = raw.get(row.key.as_str()).unwrap();
            let value = &row.identity;
            for (field, expected) in [
                ("gameId", &value.game_id),
                ("variantId", &value.variant_id),
                ("name", &value.name),
                ("grantedEffectId", &value.primary_effect_id),
            ] {
                assert_eq!(text(&raw, field), *expected, "{}:{field}", row.key);
            }
            assert_eq!(
                raw.get::<Option<String>>("nameSpec").unwrap(),
                value.name_spec
            );
            assert_eq!(
                raw.get::<Option<String>>("baseTypeName").unwrap(),
                value.base_type_name
            );
            assert_eq!(
                indexed(&raw, "additionalGrantedEffectId"),
                value.additional_effects
            );
            assert_eq!(
                indexed(&raw, "additionalStatSet"),
                value.additional_stat_sets
            );
            assert_eq!(optional_order(&raw), value.display_order);
        }
        for row in &expected.skill_declarations {
            span_text(&row.source);
            let raw: Table = table(&trace, "records").get(row.index).unwrap();
            let value = &row.identity;
            assert_eq!(text(&raw, "name"), value.name, "{}", row.id);
            assert_eq!(
                raw.get::<Option<String>>("baseTypeName").unwrap(),
                value.base_type_name
            );
            assert_eq!(raw.get::<Option<bool>>("support").unwrap(), value.support);
            assert_eq!(
                raw.get::<Option<bool>>("fromTree").unwrap(),
                value.from_tree
            );
        }
        for row in &expected.skills {
            let raw: Table = skills.get(row.id.as_str()).unwrap();
            assert_eq!(text(&raw, "id"), row.id);
            assert_eq!(text(&raw, "name"), row.name);
            assert_eq!(
                raw.get::<Option<String>>("baseTypeName").unwrap(),
                row.base_type_name
            );
            assert_eq!(raw.get::<Option<bool>>("support").unwrap(), row.support);
            assert_eq!(raw.get::<Option<bool>>("fromTree").unwrap(), row.from_tree);
            let winner = &expected.skill_declarations[row.winning_declaration as usize - 1];
            assert_eq!(winner.id, row.id);
            assert_eq!(
                table(&trace, "winners")
                    .get::<u32>(row.id.as_str())
                    .unwrap(),
                row.winning_declaration
            );
        }
        for row in &expected.gems {
            let raw: Table = gems.get(row.key.as_str()).unwrap();
            for (field, expected) in [
                ("id", &row.key),
                ("gameId", &row.game_id),
                ("variantId", &row.variant_id),
                ("name", &row.name),
                ("grantedEffectId", &row.primary_effect_id),
            ] {
                assert_eq!(text(&raw, field), *expected, "{}:{field}", row.key);
            }
            assert_eq!(
                raw.get::<Option<String>>("nameSpec").unwrap(),
                row.name_spec
            );
            assert_eq!(
                raw.get::<Option<String>>("baseTypeName").unwrap(),
                row.base_type_name
            );
            assert_eq!(optional_order(&raw), row.display_order);
            assert_eq!(
                indexed(&raw, "additionalGrantedEffectId"),
                row.constructed_additional_effects
            );
            assert_eq!(
                effect_ids(table(&raw, "additionalGrantedEffects")),
                row.additional_effects
            );
            assert_eq!(
                effect_ids(table(&raw, "grantedEffectList")),
                row.effect_list
            );
            let winner = &expected.gem_declarations[row.winning_declaration as usize - 1];
            assert_eq!(winner.key, row.key);
            // Lua table-constructor duplicate writes are not assumed to follow a
            // portable last-row rule. Compare the actual unsanitized winner.
            let raw_winner: Table = raw_gems.get(row.key.as_str()).unwrap();
            assert_eq!(text(&raw_winner, "name"), winner.identity.name);
            assert_eq!(text(&raw_winner, "gameId"), winner.identity.game_id);
            assert_eq!(text(&raw_winner, "variantId"), winner.identity.variant_id);
            assert_eq!(
                text(&raw_winner, "grantedEffectId"),
                winner.identity.primary_effect_id
            );
            assert_eq!(
                raw_winner.get::<Option<String>>("nameSpec").unwrap(),
                winner.identity.name_spec
            );
            assert_eq!(
                raw_winner.get::<Option<String>>("baseTypeName").unwrap(),
                winner.identity.base_type_name
            );
            assert_eq!(
                indexed(&raw_winner, "additionalGrantedEffectId"),
                winner.identity.additional_effects
            );
            assert_eq!(
                indexed(&raw_winner, "additionalStatSet"),
                winner.identity.additional_stat_sets
            );
            assert_eq!(optional_order(&raw_winner), winner.identity.display_order);
            let primary = text(&raw, "grantedEffectId");
            if skills
                .get::<Option<Table>>(primary.as_str())
                .unwrap()
                .is_none()
            {
                missing.push(MissingIdentityReference {
                    gem_key: row.key.clone(),
                    kind: MissingIdentityReferenceKind::PrimaryEffect,
                    index: None,
                    effect_id: primary,
                });
            }
            for (kind, raw_refs) in [
                (
                    MissingIdentityReferenceKind::DeclaredAdditionalEffect,
                    indexed(&raw_winner, "additionalGrantedEffectId"),
                ),
                (
                    MissingIdentityReferenceKind::ConstructedAdditionalEffect,
                    indexed(&raw, "additionalGrantedEffectId"),
                ),
            ] {
                for reference in raw_refs {
                    if skills
                        .get::<Option<Table>>(reference.id.as_str())
                        .unwrap()
                        .is_none()
                    {
                        missing.push(MissingIdentityReference {
                            gem_key: row.key.clone(),
                            kind,
                            index: Some(reference.index),
                            effect_id: reference.id,
                        });
                    }
                }
            }
            assert_eq!(
                row.declared_additional_effects,
                winner.identity.additional_effects
            );
            assert_eq!(
                row.declared_additional_stat_sets,
                winner.identity.additional_stat_sets
            );
            let variants: Table = table(&actual, "gemsByGameId")
                .get(row.game_id.as_str())
                .unwrap();
            let selected: Table = variants.get(row.variant_id.as_str()).unwrap();
            let resolution = catalog.resolve_external(&row.game_id, Some(&row.variant_id));
            assert!(
                resolution
                    .candidates
                    .iter()
                    .any(|candidate| candidate.key == text(&selected, "id"))
            );
            assert_eq!(
                resolution.status,
                if resolution.candidates.len() == 1 {
                    GemIdentityResolutionStatus::Exact
                } else {
                    GemIdentityResolutionStatus::Ambiguous
                }
            );
            let effect: Table = skills.get(row.primary_effect_id.as_str()).unwrap();
            let owner: String = table(&actual, "gemForSkill").get(effect).unwrap();
            assert!(
                catalog
                    .gems_for_effect(&row.primary_effect_id)
                    .any(|candidate| candidate.key == owner)
            );
        }
        missing.sort();
        assert_eq!(missing, expected.missing_references);
    }
}

#[test]
fn source_bound_lookup_matches_original_loadskill_before_name_and_socket_processing() {
    let snapshot = bundled_snapshot().unwrap();
    let catalog = snapshot.skill_identities();
    let mut documents = Vec::new();
    for index in 1..=5 {
        documents.push(
            std::fs::read_to_string(repository().join(format!(
                "tests/fixtures/builds/breadth-20260908/build-{index:02}.xml"
            )))
            .unwrap(),
        );
    }
    let single = catalog
        .data()
        .gems
        .iter()
        .find(|gem| catalog.gems_for_external_id(&gem.game_id).count() == 1)
        .unwrap();
    let other = catalog
        .data()
        .skills
        .iter()
        .find(|effect| effect.id != single.primary_effect_id)
        .unwrap();
    let controls = format!(
        r#"<PathOfBuilding2><Skills><SkillSet id="17"><Skill>
<Gem gemId="{}" variantId="not-an-authored-variant" skillId="{}"/>
<Gem gemId="" skillId="{}"/>
<Gem gemId="not-an-external-id" skillId="{}"/>
<Gem skillId="{effect}"><StatSetIndex grantedEffect="{effect}" index="2"/><StatSetCalcsIndex grantedEffect="{effect}" index="3"/></Gem>
<Gem nameSpec="{}"/>
<FutureGem/>
</Skill></SkillSet></Skills></PathOfBuilding2>"#,
        single.game_id,
        other.id,
        other.id,
        other.id,
        single.name,
        effect = other.id
    );
    documents.push(controls);
    for warm in [false, true] {
        let o = Oracle::new(warm);
        let load: Function = o.helpers.get("load").unwrap();
        let skills = table(&o.data(), "skills");
        let mut instances = 0;
        let mut selections = 0;
        let mut unresolved = 0;
        for xml in &documents {
            let projection = skill_source::project_xml(xml).unwrap();
            let lookup = lookup_definitions(&projection, &snapshot).unwrap();
            assert_eq!(
                lookup.source_xml_sha256(),
                format!("{:x}", Sha256::digest(xml.as_bytes()))
            );
            assert_eq!(lookup.data(), snapshot.identity());
            for located in lookup.records() {
                match &located.record {
                    SkillIdentityRecord::Instance(instance) => {
                        instances += 1;
                        // Reuse each exact authored fragment. No candidate adapter or
                        // native profile supplies the expected resolution.
                        let fragment = &xml[instance.source_range.clone()];
                        let input = format!(
                            "<Skills><SkillSet id=\"1\"><Skill>{fragment}</Skill></SkillSet></Skills>"
                        );
                        let loaded: Table = load.call((input.as_str(), false)).unwrap();
                        let gem = first_gem(&loaded);
                        let actual_gem: Option<String> = gem.get("gemId").unwrap();
                        let actual_effect: Option<String> = gem.get("skillId").unwrap();
                        match &instance.resolution {
                            InstanceIdentityResolution::ExternalGem {
                                status, candidates, ..
                            } => {
                                if *status == GemIdentityResolutionStatus::Missing {
                                    assert!(candidates.is_empty());
                                    assert_eq!(actual_gem, None);
                                    assert_eq!(actual_effect, None);
                                    unresolved += 1;
                                } else {
                                    // Lua pairs may choose an ambiguous winner; portable
                                    // evidence retains every candidate and does not choose.
                                    let selected = candidates
                                        .iter()
                                        .find(|candidate| {
                                            Some(&candidate.key) == actual_gem.as_ref()
                                        })
                                        .unwrap();
                                    assert_eq!(
                                        actual_effect.as_ref(),
                                        Some(&selected.primary_effect_id)
                                    );
                                    if *status != GemIdentityResolutionStatus::Ambiguous {
                                        assert_eq!(candidates.len(), 1);
                                    }
                                }
                            }
                            InstanceIdentityResolution::ExplicitEffect {
                                matched,
                                possible_primary_gem_keys,
                            } => {
                                assert_eq!(
                                    actual_effect.as_deref(),
                                    matched.as_ref().map(|effect| effect.id.as_str())
                                );
                                if let Some(owner) = actual_gem {
                                    assert!(possible_primary_gem_keys.contains(&owner));
                                } else {
                                    assert!(possible_primary_gem_keys.is_empty());
                                }
                            }
                            InstanceIdentityResolution::NameOnlyNotResolved
                            | InstanceIdentityResolution::MissingIdentity => {
                                assert_eq!(actual_gem, None);
                                assert_eq!(actual_effect, None);
                                unresolved += 1;
                            }
                        }
                    }
                    SkillIdentityRecord::EffectSelection(selection) => {
                        selections += 1;
                        let actual: Option<Table> = match &selection.granted_effect {
                            Some(key) => skills.get(key.decoded()).unwrap(),
                            None => None,
                        };
                        assert_eq!(
                            actual.as_ref().map(|effect| text(effect, "id")),
                            selection.matched.as_ref().map(|effect| effect.id.clone())
                        );
                    }
                }
            }
        }
        assert!(instances > 100);
        assert!(selections >= 2);
        assert!(unresolved >= 4);
    }
}
