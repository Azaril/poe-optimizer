//! Independent pinned XML/configuration source observations, not build legality.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Function, Lua, Table, Value as LuaValue};
use poe_optimizer_core::options::Scalar;
use poe_optimizer_import::configuration::{
    self, ActiveSetResolution, ConfigurationLayout, ConfigurationRecordIndex,
};
use poe_optimizer_pob::source;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf};

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn source_text(path: &str) -> String {
    source::read_verified_text(&repository().join("vendor/path-of-building-poe2"), path).unwrap()
}
fn section<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    assert_eq!(
        text.matches(start).count(),
        1,
        "unique source start {start}"
    );
    let begin = text.find(start).unwrap();
    let finish = begin + text[begin..].find(end).unwrap();
    &text[begin..finish]
}
fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
struct Oracle {
    _lua: Lua,
    parse: Function,
    compose: Function,
    load: Function,
    save: Function,
    quest_lines: Function,
    rounds: usize,
}
impl Oracle {
    fn new(warm: bool) -> Self {
        let lua = Lua::new();
        lua.load(if warm { "jit.on()" } else { "jit.off()" })
            .exec()
            .unwrap();
        let xml: Table = lua.load(source_text("runtime/lua/xml.lua")).eval().unwrap();
        lua.globals().set("originalXml", &xml).unwrap();
        let tab = source_text("src/Classes/ConfigTab.lua");
        // Load/Save/CreateConfigSet/SetActiveConfigSet are unmodified source.
        // Empty varList deliberately isolates authored state from game defaults;
        // only GUI rebuilding, undo, and build notifications are stubbed.
        let methods = format!(
            "local t_insert=table.insert;local s_upper=string.upper;local varList={{}};ConfigTabClass={{}};{}\n{}\n{}",
            section(
                &tab,
                "function ConfigTabClass:Load(xml, fileName)",
                "function ConfigTabClass:UpdateControls()"
            ),
            section(
                &tab,
                "function ConfigTabClass:CreateConfigSet(configSetId, title)",
                "-- Creates a new config set, adds it"
            ),
            &tab[tab
                .find("function ConfigTabClass:SetActiveConfigSet(configSetId, init, deferSync)")
                .unwrap()..]
        );
        lua.load(methods).exec().unwrap();
        let helpers: Table = lua.load(r#"
launch={ShowErrMsg=function(_,message)error(message)end}
local function config(text)
 local roots,err=originalXml.ParseXML(text);assert(roots,err)
 local root=roots[1]
 if root.elem=='Config' then return root end
 for _,child in ipairs(root)do if type(child)=='table' and child.elem=='Config'then return child end end
 return{elem='Config',empty=true,attrib={}}
end
local function load(text)
 local tab=setmetatable({defaultState={},build={SyncLoadouts=function()end},
   UpdateControls=function()end,BuildModList=function()end,ResetUndo=function()end},
   {__index=ConfigTabClass})
 tab:Load(config(text),'independent source observation');return tab
end
return{
 parse=function(text,rounds)local root;for i=1,rounds do root=config(text)end;return root end,
 load=function(text,rounds)local tab;for i=1,rounds do tab=load(text)end;return tab end,
 save=function(text)local node={elem='Config'};load(text):Save(node);local out,err=originalXml.ComposeXML(node);assert(out,err);return out end}
"#).eval().unwrap();
        let quests: Table = lua
            .load(source_text("src/Data/QuestRewards.lua"))
            .eval()
            .unwrap();
        let data = lua.create_table().unwrap();
        data.set("questRewards", quests).unwrap();
        lua.globals().set("data", data).unwrap();
        let options = source_text("src/Modules/ConfigOptions.lua");
        let quest_lines: Function = lua.load(format!(r#"
-- The spy observes complete lines passed by the original quest consumer. It does
-- not implement ModParser or claim that Charm modifiers are native-supported.
local observed={{}};StripEscapes=function(text)return text end
modLib={{parseMod=function(line)table.insert(observed,line);return{{}},nil end}}
{}
local settings={{}};addQuestModsRewardsConfigOptions(settings)
return function(key,value)
 observed={{}}
 for _,setting in ipairs(settings)do if setting.var==key then setting.apply(value,{{}},{{}});return observed end end
 error('source quest key missing')
end
"#,section(&options,"local function questModsRewards(source, line, modList)","local configSettings = {"))).eval().unwrap();
        Self {
            parse: helpers.get("parse").unwrap(),
            load: helpers.get("load").unwrap(),
            save: helpers.get("save").unwrap(),
            compose: xml.get("ComposeXML").unwrap(),
            quest_lines,
            rounds: if warm { 100 } else { 1 },
            _lua: lua,
        }
    }
    fn parsed(&self, text: &str) -> Table {
        self.parse.call((text, self.rounds)).unwrap()
    }
    fn loaded(&self, text: &str) -> Table {
        self.load.call((text, self.rounds)).unwrap()
    }
    fn recomposed(&self, text: &str) -> String {
        let (out, error): (Option<String>, Option<String>) =
            self.compose.call(self.parsed(text)).unwrap();
        assert!(error.is_none());
        out.unwrap()
    }
}
fn scalar(value: LuaValue) -> Scalar {
    match value {
        LuaValue::Boolean(v) => Scalar::Boolean(v),
        LuaValue::Number(v) => Scalar::Number(v),
        LuaValue::Integer(v) => Scalar::Number(v as f64),
        LuaValue::String(v) => Scalar::Text(v.to_str().unwrap().to_owned()),
        value => panic!("unexpected source scalar {value:?}"),
    }
}
fn scalar_map(table: Table) -> BTreeMap<String, Scalar> {
    table
        .pairs::<String, LuaValue>()
        .map(|pair| {
            let (k, v) = pair.unwrap();
            (k, scalar(v))
        })
        .collect()
}
fn parsed_sets(config: Table) -> Vec<Table> {
    config
        .sequence_values::<Table>()
        .map(Result::unwrap)
        .filter(|node| node.get::<String>("elem").unwrap() == "ConfigSet")
        .collect()
}
fn parsed_values(set: &Table, element: &str) -> BTreeMap<String, Scalar> {
    set.clone()
        .sequence_values::<Table>()
        .map(Result::unwrap)
        .filter(|node| node.get::<String>("elem").unwrap() == element)
        .map(|node| {
            let attrs: Table = node.get("attrib").unwrap();
            let name = attrs.get::<String>("name").unwrap();
            let value = if let Some(s) = attrs.get::<Option<String>>("number").unwrap() {
                Scalar::Number(s.parse().unwrap())
            } else if let Some(s) = attrs.get::<Option<String>>("string").unwrap() {
                Scalar::Text(s)
            } else {
                Scalar::Boolean(attrs.get::<String>("boolean").unwrap() == "true")
            };
            (name, value)
        })
        .collect()
}

#[test]
fn original_loader_migrations_and_saved_defaults_remain_explicit_observations() {
    let text = "<PathOfBuilding2><Config activeConfigSet='99'><ConfigSet id='7' title='Authored'>\
      <Input name='enemyIsBoss' string='uBer atziri'/>\
      <Input name='presetBossSkills' string='Uber Atziri Flameblast'/>\
      <Input name='customMods' string='+3 to Strength\n\t+4 to Dexterity'/>\
      <Input name='disabled' boolean='false'/>\
      <Placeholder name='stringPlaceholder' string='retained by source as input'/>\
      <Placeholder name='numericPlaceholder' number='1.5'/>\
      </ConfigSet><ConfigSet id='3' title='Other'/></Config></PathOfBuilding2>";
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        assert_source_projection(text, &oracle);
        let projection = configuration::project_xml(text).unwrap();
        let authored_projection = projected_values(projection.active_set().inputs());
        assert_eq!(
            authored_projection["enemyIsBoss"],
            Scalar::Text("uBer atziri".into())
        );
        assert_eq!(
            authored_projection["presetBossSkills"],
            Scalar::Text("Uber Atziri Flameblast".into())
        );
        assert!(authored_projection.contains_key("customMods"));
        assert!(!authored_projection.contains_key("stringPlaceholder"));
        let parsed = parsed_sets(oracle.parsed(text));
        let authored = parsed_values(&parsed[0], "Input");
        assert_eq!(authored["enemyIsBoss"], Scalar::Text("uBer atziri".into()));
        let tab = oracle.loaded(text);
        assert_eq!(tab.get::<u32>("activeConfigSetId").unwrap(), 7);
        let sets: Table = tab.get("configSets").unwrap();
        let set: Table = sets.get(7).unwrap();
        let input = scalar_map(set.get("input").unwrap());
        assert_eq!(input["enemyIsBoss"], Scalar::Text("Boss".into()));
        assert_eq!(
            input["presetBossSkills"],
            Scalar::Text("Atziri Flameblast".into())
        );
        assert_eq!(
            input["stringPlaceholder"],
            Scalar::Text("retained by source as input".into())
        );
        assert!(!input.contains_key("customMods"));
        assert_eq!(
            scalar_map(set.get("placeholder").unwrap())["numericPlaceholder"],
            Scalar::Number(1.5)
        );
        let blocks: Table = set.get("customModsList").unwrap();
        assert_eq!(
            blocks
                .get::<Table>(1)
                .unwrap()
                .get::<String>("text")
                .unwrap(),
            "+3 to Strength\n\t+4 to Dexterity"
        );
        let saved: String = oracle.save.call(text).unwrap();
        let saved_sets = parsed_sets(oracle.parsed(&saved));
        let saved_input = parsed_values(&saved_sets[0], "Input");
        assert!(
            !saved_input.contains_key("disabled"),
            "PoB save omits its default false value"
        );
        assert!(
            !saved_input.contains_key("customMods"),
            "migration is not a lossless source rewrite"
        );
        assert_eq!(saved_input["stringPlaceholder"], input["stringPlaceholder"]);
    }
}

fn projected_values(values: &[configuration::ScalarInput<'_>]) -> BTreeMap<String, Scalar> {
    values
        .iter()
        .map(|row| (row.name().to_owned(), row.value().clone()))
        .collect()
}
fn assert_source_projection(text: &str, oracle: &Oracle) {
    let projection = configuration::project_xml(text).unwrap();
    let document = roxmltree::Document::parse(text).unwrap();
    poe_optimizer_import::xml_compat::validate_native_with_configuration(&document).unwrap();
    assert_eq!(projection.source_xml(), text);
    assert_eq!(projection.source_sha256(), digest(text));
    let config = oracle.parsed(text);
    let explicit = parsed_sets(config.clone());
    let sets = if explicit.is_empty() {
        vec![config]
    } else {
        explicit
    };
    assert_eq!(projection.sets().len(), sets.len());
    let loaded = oracle.loaded(text);
    assert_eq!(
        projection.active_set_id(),
        loaded.get::<u32>("activeConfigSetId").unwrap()
    );
    for (projected, source) in projection.sets().iter().zip(sets) {
        let attrs: Table = source.get("attrib").unwrap();
        assert_eq!(
            projected.id(),
            attrs
                .get::<Option<String>>("id")
                .unwrap()
                .map_or(1, |v| v.parse().unwrap())
        );
        assert_eq!(
            projected.title(),
            attrs
                .get::<Option<String>>("title")
                .unwrap()
                .as_deref()
                .unwrap_or("Default")
        );
        assert_eq!(
            projected_values(projected.inputs()),
            parsed_values(&source, "Input")
        );
        assert_eq!(
            projected_values(projected.placeholders()),
            parsed_values(&source, "Placeholder")
        );
        for row in projected.inputs().iter().chain(projected.placeholders()) {
            assert_eq!(&text[row.source_range()], row.source_xml());
            assert_eq!(&text[row.value_source().range()], row.value_source().raw());
            assert_eq!(&text[row.name_source().range()], row.name_source().raw());
            if let Scalar::Text(value) = row.value() {
                assert_eq!(value, row.value_source().decoded());
            }
        }
        let mut previous_end = 0;
        let projected_order = projected
            .records_in_source_order()
            .iter()
            .map(|entry| {
                let (name, range) = match *entry {
                    ConfigurationRecordIndex::Input(i) => {
                        ("Input", projected.inputs()[i].source_range())
                    }
                    ConfigurationRecordIndex::Placeholder(i) => {
                        ("Placeholder", projected.placeholders()[i].source_range())
                    }
                    ConfigurationRecordIndex::Block(i) => {
                        ("CustomModifierBlock", projected.blocks()[i].source_range())
                    }
                    ConfigurationRecordIndex::Unknown(i) => (
                        projected.unknown_records()[i].element_name(),
                        projected.unknown_records()[i].source_range(),
                    ),
                };
                assert!(range.start >= previous_end);
                previous_end = range.end;
                name.to_owned()
            })
            .collect::<Vec<_>>();
        let source_order = source
            .clone()
            .sequence_values::<Table>()
            .map(|node| node.unwrap().get::<String>("elem").unwrap())
            .collect::<Vec<_>>();
        assert_eq!(projected_order, source_order);
        let source_blocks = source
            .sequence_values::<Table>()
            .map(Result::unwrap)
            .filter(|node| node.get::<String>("elem").unwrap() == "CustomModifierBlock")
            .collect::<Vec<_>>();
        assert_eq!(projected.blocks().len(), source_blocks.len());
        for (block, source) in projected.blocks().iter().zip(source_blocks) {
            let attrs: Table = source.get("attrib").unwrap();
            assert_eq!(
                block.title(),
                attrs
                    .get::<Option<String>>("title")
                    .unwrap()
                    .as_deref()
                    .unwrap_or("Default")
            );
            assert_eq!(
                block.enabled(),
                attrs
                    .get::<Option<String>>("enabled")
                    .unwrap()
                    .is_none_or(|v| v == "true")
            );
            assert_eq!(
                block.pob_text(),
                source
                    .get::<Option<String>>(1)
                    .unwrap()
                    .as_deref()
                    .unwrap_or("")
            );
            assert_eq!(&text[block.source_range()], block.source_xml());
            assert_eq!(&text[block.text().range()], block.text().raw());
        }
    }
    let diagnostic = projection.diagnostic();
    for key in ["effective_configuration", "mechanics", "game_legality"] {
        assert_eq!(diagnostic[key], "not_evaluated");
    }
}
fn wrap_saved_config(text: &str) -> String {
    let content = text
        .strip_prefix("<?xml")
        .map_or(text, |s| &s[s.find("?>").unwrap() + 2..]);
    format!("<PathOfBuilding2>{content}</PathOfBuilding2>")
}

#[test]
fn raw_attribute_scalars_sets_entities_and_whitespace_match_original_parser_and_loader() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for gap in ["\n\t", "\r\n\t", "\r", "\t"] {
            for key in [
                "unmodeledChoice",
                "renamedAuthoredValue",
                "arbitrary&amp;key",
            ] {
                let source = format!(
                    "<PathOfBuilding2><Config activeConfigSet='7'>\
                  <ConfigSet id='3' title='Inactive'><Input name='{key}' string='inactive{gap}choice'/></ConfigSet>\
                  <ConfigSet id='7' title='Active &amp; exact'><Input name='{key}' string='first{gap}second &amp; &lt; &gt; &apos; &quot; &amp;#10;'/>\
                  <Input name='fraction' number='1.25'/><Input name='yes' boolean='true'/><Input name='no' boolean='false'/>\
                  <Placeholder name='numericHint' number='27'/><Placeholder name='textHint' string='kept as authored'/>\
                  <CustomModifierBlock title='Untouched' enabled='false'>\n\tunsupported &amp; exact text\n</CustomModifierBlock>\
                  </ConfigSet></Config></PathOfBuilding2>"
                );
                assert_source_projection(&source, &oracle);
                let standard = roxmltree::Document::parse(&source).unwrap();
                let node = standard
                    .descendants()
                    .find(|n| n.has_tag_name("Input"))
                    .unwrap();
                assert!(
                    !node
                        .attribute("string")
                        .unwrap()
                        .contains(['\r', '\n', '\t'])
                );
                let projection = configuration::project(&standard).unwrap();
                assert!(
                    projection.sets()[0].inputs()[0]
                        .value_source()
                        .decoded()
                        .contains(gap)
                );
                let recomposed = wrap_saved_config(&oracle.recomposed(&source));
                assert_source_projection(&recomposed, &oracle);
                let fresh = configuration::project_xml(&recomposed).unwrap();
                assert_eq!(
                    projected_values(fresh.active_set().inputs()),
                    projected_values(projection.active_set().inputs())
                );
                let saved =
                    wrap_saved_config(&oracle.save.call::<String>(source.as_str()).unwrap());
                assert_source_projection(&saved, &oracle);
            }
        }
    }
}

#[test]
fn immutable_user_corpus_projects_all_authored_config_without_claiming_mechanic_support() {
    let directory = repository().join("tests/fixtures/builds/breadth-20260908");
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("index.json")).unwrap()).unwrap();
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for entry in index["builds"].as_array().unwrap() {
            let bytes = std::fs::read(directory.join(entry["xml"].as_str().unwrap())).unwrap();
            let text = std::str::from_utf8(&bytes).unwrap();
            assert_eq!(digest(text), entry["xml_sha256"]);
            assert_source_projection(text, &oracle);
            let projection = configuration::project_xml(text).unwrap();
            let choice = projection
                .active_set()
                .inputs()
                .iter()
                .find(|row| row.name() == "questAct 2Valley of the TitansMedallion")
                .unwrap();
            assert!(choice.value_source().decoded().contains("\n\t"));
            let loaded = oracle.loaded(text);
            let input = scalar_map(loaded.get("input").unwrap());
            assert_eq!(&input[choice.name()], choice.value());
            let lines: Table = oracle
                .quest_lines
                .call((choice.name(), choice.value_source().decoded()))
                .unwrap();
            let lines = lines
                .sequence_values::<String>()
                .map(Result::unwrap)
                .collect::<Vec<_>>();
            assert_eq!(
                lines.len(),
                2,
                "original consumer must see two complete lines"
            );
            assert_eq!(lines[1], "+1 Charm Slot");
            let normalized = choice
                .value_source()
                .decoded()
                .replace(['\r', '\n', '\t'], " ");
            let collapsed: Table = oracle
                .quest_lines
                .call((choice.name(), normalized))
                .unwrap();
            assert_eq!(
                collapsed.raw_len(),
                1,
                "normalization changes original parser inputs"
            );
            assert_eq!(projection.source_xml().as_bytes(), bytes);
        }
    }
}

#[test]
fn requested_and_resolved_active_ids_match_original_fallback_with_legacy_and_empty_layouts() {
    let cases = [
        (
            "<PathOfBuilding2/>",
            ConfigurationLayout::Missing,
            1,
            ActiveSetResolution::DefaultOne,
        ),
        (
            "<PathOfBuilding2><Config/></PathOfBuilding2>",
            ConfigurationLayout::Empty,
            1,
            ActiveSetResolution::DefaultOne,
        ),
        (
            "<PathOfBuilding2><Config><Input name='legacy' string='x\n\ty'/></Config></PathOfBuilding2>",
            ConfigurationLayout::Legacy,
            1,
            ActiveSetResolution::DefaultOne,
        ),
        (
            "<PathOfBuilding2><Config><ConfigSet id='8'/><ConfigSet id='2'/></Config></PathOfBuilding2>",
            ConfigurationLayout::ExplicitSets,
            8,
            ActiveSetResolution::MissingDefaultUsesFirst,
        ),
        (
            "<PathOfBuilding2><Config activeConfigSet='99'><ConfigSet id='8'/><ConfigSet id='2'/></Config></PathOfBuilding2>",
            ConfigurationLayout::ExplicitSets,
            8,
            ActiveSetResolution::MissingRequestedUsesFirst,
        ),
        (
            "<PathOfBuilding2><Config activeConfigSet='2'><ConfigSet id='8'/><ConfigSet id='2'/></Config></PathOfBuilding2>",
            ConfigurationLayout::ExplicitSets,
            2,
            ActiveSetResolution::Requested,
        ),
    ];
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for (text, layout, active, resolution) in cases {
            assert_source_projection(text, &oracle);
            let projection = configuration::project_xml(text).unwrap();
            assert_eq!(projection.layout(), layout);
            assert_eq!(projection.active_set_id(), active);
            assert_eq!(projection.active_set_resolution(), resolution);
        }
    }
}

#[test]
fn unsupported_blocks_and_children_remain_source_fragments_not_silently_applied_configuration() {
    let source = "<PathOfBuilding2><Config><ConfigSet id='1'>\
      <Input name='futureMechanic' string='arbitrary unmodeled choice'/>\
      <CustomModifierBlock title='Disabled' enabled='false'><![CDATA[  unsupported &amp; literal\r\n\ttext  ]]></CustomModifierBlock>\
      <FutureInput meaning='unknown'><Nested/></FutureInput>\
      </ConfigSet></Config></PathOfBuilding2>";
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        assert_source_projection(source, &oracle);
        let projected = configuration::project_xml(source).unwrap();
        let block = &projected.active_set().blocks()[0];
        assert_eq!(block.pob_text(), "  unsupported &amp; literal\r\n\ttext  ");
        let unknown = &projected.active_set().unknown_records()[0];
        assert_eq!(unknown.element_name(), "FutureInput");
        assert_eq!(
            unknown.source_xml(),
            "<FutureInput meaning='unknown'><Nested/></FutureInput>"
        );
        assert_eq!(&source[unknown.source_range()], unknown.source_xml());
        let original = oracle.loaded(source);
        assert_eq!(scalar_map(original.get("input").unwrap()).len(), 1);
    }
}

#[test]
fn ambiguous_or_lossy_source_forms_reject_instead_of_copying_pob_silent_losses() {
    let wrap = |body: &str| {
        format!(
            "<PathOfBuilding2><Config><ConfigSet id='1'>{body}</ConfigSet></Config></PathOfBuilding2>"
        )
    };
    for body in [
        "<Input name='x' string='a&#10;b'/>",
        "<Input name='x' string='a&#x9;b'/>",
        "<Input name = 'x' string='a'/>",
        "<Input name='x' string='a>b'/>",
        "<Input xmlns='urn:foreign' name='x' string='a'/>",
        "<Input name='x' number='1' string='also'/>",
        "<Input name='x' number='NaN'/>",
        "<Input name='x' number='1e309'/>",
        "<Input name='x' number='Infinity'/>",
        "<Input name='x' number='0x10'/>",
        "<Input name='x'/>",
        "<Input string='missing name'/>",
        "<Placeholder name='x' boolean='true'/>",
        "<Input name='x' boolean='yes'/>",
        "<Input name='x' string='a'/><Input name='x' string='b'/>",
        "<Placeholder name='x' number='1'/><Placeholder name='x' number='2'/>",
        "<CustomModifierBlock>a<![CDATA[b]]>c</CustomModifierBlock>",
    ] {
        assert!(configuration::project_xml(&wrap(body)).is_err(), "{body}");
    }
    let unknown_text = wrap("<FutureInput string='unknown\n\tvalue'/>");
    let unknown = configuration::project_xml(&unknown_text).unwrap();
    assert_eq!(
        unknown.active_set().unknown_records()[0].source_xml(),
        "<FutureInput string='unknown\n\tvalue'/>"
    );
    assert!(
        poe_optimizer_import::xml_compat::validate_native_with_configuration(
            &roxmltree::Document::parse(&unknown_text).unwrap()
        )
        .is_err(),
        "unhandled attributes never receive a native whitespace exemption"
    );
    for config in [
        "<Config><ConfigSet id='1'/><ConfigSet id='1'/></Config>",
        "<Config><ConfigSet/></Config>",
        "<Config><Input name='x' string='a'/><ConfigSet id='1'/></Config>",
        "<Config xmlns='urn:foreign'><ConfigSet id='1'/></Config>",
    ] {
        assert!(
            configuration::project_xml(&format!("<PathOfBuilding2>{config}</PathOfBuilding2>"))
                .is_err(),
            "{config}"
        );
    }
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        let missing = parsed_sets(oracle.parsed(&wrap("<Input name='x' string='a&#10;b'/>")));
        assert_eq!(
            parsed_values(&missing[0], "Input")["x"],
            Scalar::Text("ab".into())
        );
        let precedence = oracle.loaded(&wrap("<Input name='x' number='1' string='also'/>"));
        assert_eq!(
            scalar_map(precedence.get("input").unwrap())["x"],
            Scalar::Number(1.0)
        );
        let duplicate = oracle.loaded(&wrap(
            "<Input name='x' string='a'/><Input name='x' string='b'/>",
        ));
        assert_eq!(
            scalar_map(duplicate.get("input").unwrap())["x"],
            Scalar::Text("b".into())
        );
    }
}

#[test]
fn cross_kind_source_order_retains_inputs_that_the_original_loader_overwrites() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for placeholder_last in [false, true] {
            let input = "<Input name='sharedArbitraryKey' string='input value'/>";
            let placeholder = "<Placeholder name='sharedArbitraryKey' string='placeholder value'/>";
            let (first, last) = if placeholder_last {
                (input, placeholder)
            } else {
                (placeholder, input)
            };
            let source = format!(
                "<PathOfBuilding2><Config><ConfigSet id='1'>{first}<FutureSetting/><CustomModifierBlock>unknown mechanic</CustomModifierBlock>{last}</ConfigSet></Config></PathOfBuilding2>"
            );
            assert_source_projection(&source, &oracle);
            let projected = configuration::project_xml(&source).unwrap();
            assert_eq!(
                projected.active_set().inputs()[0].value(),
                &Scalar::Text("input value".into())
            );
            assert_eq!(
                projected.active_set().placeholders()[0].value(),
                &Scalar::Text("placeholder value".into())
            );
            let loaded = scalar_map(oracle.loaded(&source).get("input").unwrap());
            assert_eq!(
                loaded["sharedArbitraryKey"],
                Scalar::Text(
                    if placeholder_last {
                        "placeholder value"
                    } else {
                        "input value"
                    }
                    .into()
                )
            );
        }
    }
}

#[test]
fn finite_decimal_scalar_bits_match_original_config_loader_for_inputs_and_placeholders() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for number in [
            "0",
            "-0",
            " 1.25 ",
            "1.2345678901234567",
            "1.2e-307",
            "1.7976931348623157e308",
            "5e-324",
            "+7.5",
            ".25",
            "25.",
        ] {
            let source = format!(
                "<PathOfBuilding2><Config><ConfigSet id='1'><Input name='renamedNumber' number='{number}'/><Placeholder name='renamedHint' number='{number}'/></ConfigSet></Config></PathOfBuilding2>"
            );
            let projection = configuration::project_xml(&source).unwrap();
            let loaded = oracle.loaded(&source);
            for (rows, key, table) in [
                (projection.active_set().inputs(), "renamedNumber", "input"),
                (
                    projection.active_set().placeholders(),
                    "renamedHint",
                    "placeholder",
                ),
            ] {
                let Scalar::Number(native) = rows[0].value() else {
                    panic!("numeric source kind")
                };
                let original: f64 = loaded.get::<Table>(table).unwrap().get(key).unwrap();
                assert_eq!(
                    native.to_bits(),
                    original.to_bits(),
                    "{table} number={number}"
                );
                assert_eq!(rows[0].value_source().raw(), number);
                assert_eq!(rows[0].value_source().decoded(), number);
            }
        }
    }
}
