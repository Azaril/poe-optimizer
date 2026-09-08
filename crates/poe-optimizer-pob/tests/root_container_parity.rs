//! Independent original-source root-container observations. These tests do not
//! grant native capability to saved party payloads or CALCS configuration.
#![cfg(not(target_arch = "wasm32"))]

use mlua::{Function, Lua, Table, Value};
use poe_optimizer_core::options::Scalar;
use poe_optimizer_import::build_source::{self, RootSectionKind, SourceElement};
use poe_optimizer_pob::source;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf, sync::OnceLock};

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn original(path: &str) -> &'static str {
    static TEXT: OnceLock<BTreeMap<&str, String>> = OnceLock::new();
    TEXT.get_or_init(|| {
        [
            "runtime/lua/xml.lua",
            "src/Classes/ImportTab.lua",
            "src/Classes/PartyTab.lua",
            "src/Classes/CalcsTab.lua",
            "src/Classes/PassiveTreeView.lua",
            "src/Classes/ConfigTab.lua",
            "src/Classes/DropDownControl.lua",
            "src/Classes/EditControl.lua",
            "src/Modules/Build.lua",
            "src/Modules/CalcSetup.lua",
            "src/Modules/CalcPerform.lua",
        ]
        .into_iter()
        .map(|path| {
            (
                path,
                source::read_verified_text(
                    &repository().join("vendor/path-of-building-poe2"),
                    path,
                )
                .unwrap(),
            )
        })
        .collect()
    })[path]
        .as_str()
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
        lua.load(if warm { "jit.on()" } else { "jit.off()" })
            .exec()
            .unwrap();
        let xml: Table = lua.load(original("runtime/lua/xml.lua")).eval().unwrap();
        lua.globals().set("originalXml", xml).unwrap();
        lua.load(
            "t_insert=table.insert;m_min=math.min;m_max=math.max;\
             ImportTabClass={};PartyTabClass={};CalcsTabClass={};\
             PassiveTreeViewClass={};ConfigTabClass={};DropDownClass={};EditClass={};\
             buildMode={};common={xml=originalXml};\
             wipeTable=function(t)for k in pairs(t)do t[k]=nil end;return t end",
        )
        .exec()
        .unwrap();
        for (path, start, end) in [
            (
                "src/Classes/ImportTab.lua",
                "function ImportTabClass:Load(xml, fileName)",
                "function ImportTabClass:Draw(",
            ),
            (
                "src/Classes/PartyTab.lua",
                "function PartyTabClass:Load(xml, fileName)",
                "function PartyTabClass:Draw(",
            ),
            (
                "src/Classes/PartyTab.lua",
                "function PartyTabClass:ParseBuffs(",
                "function PartyTabClass:setBuffExports(",
            ),
            (
                "src/Classes/CalcsTab.lua",
                "function CalcsTabClass:Load(xml, dbFileName)",
                "function CalcsTabClass:Draw(",
            ),
            (
                "src/Classes/PassiveTreeView.lua",
                "function PassiveTreeViewClass:PassiveTreeView()",
                "function PassiveTreeViewClass:GetCompareJewel(",
            ),
            (
                "src/Classes/ConfigTab.lua",
                "function ConfigTabClass:ImportCalcSettings()",
                "function ConfigTabClass:CreateUndoState()",
            ),
            (
                "src/Classes/DropDownControl.lua",
                "function DropDownClass:SelByValue(value, key)",
                "function DropDownClass:GetSelValueByKey(",
            ),
            (
                "src/Classes/EditControl.lua",
                "function EditClass:SetText(text, notify)",
                "function EditClass:SetPlaceholder(",
            ),
            (
                "src/Modules/Build.lua",
                "function buildMode:LoadDB(xmlText, fileName)",
                "function buildMode:LoadDBFile()",
            ),
            (
                "src/Modules/Build.lua",
                "function buildMode:SaveDB(fileName)",
                "function buildMode:SaveDBFile()",
            ),
        ] {
            lua.load(section(original(path), start, end))
                .set_name(format!("@{path}"))
                .exec()
                .unwrap();
        }
        let party = original("src/Classes/PartyTab.lua");
        let party_constructor = section(
            party,
            "function PartyTabClass:PartyTab(build)",
            "\tlocal theme",
        );
        lua.load(&party[party.find("function PartyTabClass:exportBuffs(").unwrap()..])
            .set_name("@src/Classes/PartyTab.lua")
            .exec()
            .unwrap();
        lua.load(format!(
            "function original_party_defaults(self)\n{}\nend\n\
             function original_party_destinations()\n{}\nreturn partyDestinations end\n\
             function original_calcs_defaults(self)\n{}\nend",
            section(
                party_constructor,
                "\tself.actor = { Aura",
                "\tlocal partyDestinations"
            ),
            section(party, "\tlocal partyDestinations", "\tlocal theme"),
            section(
                original("src/Classes/CalcsTab.lua"),
                "\tself.input = { }",
                "\tself.controls.search",
            ),
        ))
        .exec()
        .unwrap();
        let helpers: Table = lua
            .load(include_str!("support/root_container_oracle.lua"))
            .eval()
            .unwrap();
        Self {
            lua,
            helpers,
            rounds: if warm { 100 } else { 1 },
        }
    }

    fn observe(&self, xml: &str, context: Option<Table>) -> Table {
        let call: Function = self.helpers.get("observe").unwrap();
        let mut observed = None;
        for _ in 0..self.rounds {
            observed = Some(call.call::<Table>((xml, context.clone())).unwrap());
        }
        observed.unwrap()
    }

    fn context(&self) -> Table {
        self.lua.create_table().unwrap()
    }
}

fn table(parent: &Table, name: &str) -> Table {
    parent.get(name).unwrap()
}
fn text(parent: &Table, name: &str) -> String {
    parent.get(name).unwrap()
}
fn number(parent: &Table, name: &str) -> f64 {
    parent.get(name).unwrap()
}
fn exact_number(parent: &Table, name: &str, expected: f64) {
    assert_eq!(number(parent, name).to_bits(), expected.to_bits(), "{name}");
}
fn scalar_map(values: Table) -> BTreeMap<String, Scalar> {
    values
        .pairs::<String, Value>()
        .map(|entry| {
            let (key, value) = entry.unwrap();
            let scalar = match value {
                Value::Boolean(v) => Scalar::Boolean(v),
                Value::Integer(v) => Scalar::Number(v as f64),
                Value::Number(v) => Scalar::Number(v),
                Value::String(v) => Scalar::Text(v.to_str().unwrap().to_owned()),
                other => panic!("unexpected source scalar {other:?}"),
            };
            (key, scalar)
        })
        .collect()
}

#[test]
fn import_source_flags_defaults_and_save_context_remain_distinct() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        let context = o.context();
        context.set("lastRealm", "context").unwrap();
        let observed = o.observe("<Import/>", Some(context));
        let saved = table(&table(&observed, "import_saved"), "attrib");
        assert_eq!(text(&observed, "selected_realm"), "context");
        assert_eq!(text(&observed, "selected_league"), "Standard");
        assert!(matches!(
            saved.get::<Value>("lastRealm").unwrap(),
            Value::Nil
        ));
        assert!(matches!(
            saved.get::<Value>("lastLeague").unwrap(),
            Value::Nil
        ));
        assert_eq!(text(&saved, "exportParty"), "false");
        assert_eq!(text(&saved, "useGeneratedItemText"), "true");
        for (authored, expected_export, expected_generated) in [
            ("true", true, true),
            ("false", false, false),
            ("TRUE", false, true),
            ("", false, true),
        ] {
            let xml =
                format!("<Import exportParty=\"{authored}\" useGeneratedItemText=\"{authored}\"/>");
            let v = o.observe(&xml, None);
            let build = table(&v, "build");
            assert_eq!(
                table(&build, "partyTab")
                    .get::<bool>("enableExportBuffs")
                    .unwrap(),
                expected_export
            );
            let attrs = table(&table(&v, "import_saved"), "attrib");
            assert_eq!(
                text(&attrs, "useGeneratedItemText"),
                expected_generated.to_string()
            );
        }
        for len in [0, 99, 100] {
            let context = o.context();
            let link = "x".repeat(len);
            context.set("importLink", link.clone()).unwrap();
            let v = o.observe(
                "<Import importLink=\"authored\" lastRealm=\"PoE2\" lastLeague=\"custom\"/>",
                Some(context),
            );
            assert_eq!(text(&v, "selected_realm"), "PoE2");
            assert_eq!(text(&v, "selected_league"), "custom");
            let attrs = table(&table(&v, "import_saved"), "attrib");
            assert_eq!(
                attrs.get::<Option<String>>("importLink").unwrap(),
                (len < 100).then_some(link)
            );
            assert_eq!(
                text(&table(&table(&v, "build"), "importTab"), "importLink"),
                "authored"
            );
        }
        let context = o.context();
        let accounts = o.context();
        accounts.set("local context account", true).unwrap();
        context.set("accounts", accounts).unwrap();
        let v = o.observe(
            "<Import lastAccountHash=\"hash:local context account\"/>",
            Some(context),
        );
        assert_eq!(
            text(
                &table(
                    &table(&table(&table(&v, "build"), "importTab"), "controls"),
                    "accountName"
                ),
                "buf"
            ),
            "local context account"
        );
    }
}

#[test]
fn tree_view_original_defaults_pair_loading_and_saved_values() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        for (attrs, x, y, zoom) in [
            ("", 0.0, 0.0, 3.0),
            ("zoomX=\"18\"", 0.0, 0.0, 3.0),
            ("zoomY=\"-5\"", 0.0, 0.0, 3.0),
            ("zoomX=\"18\" zoomY=\"-5\" zoomLevel=\"4\"", 18.0, -5.0, 4.0),
        ] {
            let v = o.observe(&format!("<TreeView {attrs}/>"), None);
            let t = table(&table(&v, "build"), "treeView");
            exact_number(&t, "zoomX", x);
            exact_number(&t, "zoomY", y);
            exact_number(&t, "zoomLevel", zoom);
            assert_eq!(number(&t, "zoom"), 1.2_f64.powf(zoom));
        }
        let v = o.observe("<TreeView searchStr=\"A&amp;B &lt;x&gt; &quot; &apos;\" showStatDifferences=\"false\"/>", None);
        let t = table(&table(&v, "build"), "treeView");
        assert_eq!(text(&t, "searchStrSaved"), "A&B <x> \" '");
        assert!(!t.get::<bool>("showStatDifferences").unwrap());
        assert_eq!(
            text(&table(&table(&v, "tree_saved"), "attrib"), "searchStr"),
            text(&t, "searchStr")
        );
    }
}

#[test]
fn party_incoming_source_parsing_is_independent_of_export_and_saved_exports_are_inert() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        for export in ["true", "false"] {
            let xml = format!(
                r#"<PathOfBuilding2><Import exportParty="{export}"/><Party destination="EnemyMods" append="true" ShowAdvanceTools="true">
<ImportedBuffs name="PartyMemberStats"><![CDATA[Life=123
Speed|percent=25
Nested.Value=7
CustomFlag]]></ImportedBuffs>
<ImportedBuffs name="EnemyConditions">Condition:Chilled
PlainFlag</ImportedBuffs>
<ExportedBuffs name="PartyMemberStats">Life=999</ExportedBuffs>
<ImportedBuffs name="Unrecognized">arbitrary retained input</ImportedBuffs>
<Future unknown="yes"/>
</Party></PathOfBuilding2>"#
            );
            let v = o.observe(&xml, None);
            let p = table(&table(&v, "build"), "partyTab");
            let actor = table(&p, "actor");
            let output = table(&actor, "output");
            exact_number(&output, "Life", 123.0);
            exact_number(&output, "Speed", 0.25);
            exact_number(&table(&output, "Nested"), "Value", 7.0);
            let flags = table(&actor, "modDB");
            let flag: Table = flags.get(1).unwrap();
            assert_eq!(flag.get::<String>(1).unwrap(), "CustomFlag");
            let enemies = table(&p, "enemyModList");
            let condition: Table = enemies.get(1).unwrap();
            assert_eq!(
                condition.get::<String>(1).unwrap(),
                "Condition:Party:Chilled"
            );
            assert_eq!(condition.get::<String>(4).unwrap(), "Party");
            assert_eq!(table(&p, "buffExports").raw_len(), 0);
            let saved = table(&v, "party_saved");
            assert_eq!(saved.raw_len(), 2);
            let attrs = table(&saved, "attrib");
            assert_eq!(text(&attrs, "destination"), "EnemyMods");
            assert_eq!(text(&attrs, "append"), "true");
            assert_eq!(text(&attrs, "ShowAdvanceTools"), "true");
        }
    }
}

#[test]
fn calcs_typed_inputs_defaults_unknowns_and_section_context_are_source_observations() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        let context = o.context();
        let sections: Table = o.lua.load("return{{id='known',subSection={{id='part',collapsed=false},{id='other',collapsed=true}}}}") .eval().unwrap();
        context.set("sections", sections).unwrap();
        let v = o.observe(r#"<Calcs><Input name="skill_number" number="3"/><Input name="custom" boolean="true"/>
<Input name="arbitrary" string="x&amp;y"/><Section id="known" subsection="part" collapsed="true"/>
<Section id="unrecognized" subsection="part" collapsed="false"/><Future value="retained by projection"/></Calcs>"#, Some(context));
        let c = table(&table(&v, "build"), "calcsTab");
        let values = scalar_map(table(&c, "input"));
        assert_eq!(values["misc_buffMode"], Scalar::Text("EFFECTIVE".into()));
        assert_eq!(values["skill_number"], Scalar::Number(3.0));
        assert_eq!(values["custom"], Scalar::Boolean(true));
        assert_eq!(values["arbitrary"], Scalar::Text("x&y".into()));
        let saved = table(&v, "calcs_saved");
        assert_eq!(saved.raw_len(), 6);
        let mut inputs = BTreeMap::new();
        let mut sections = Vec::new();
        for node in saved.sequence_values::<Table>() {
            let node = node.unwrap();
            let attrs = table(&node, "attrib");
            if text(&node, "elem") == "Input" {
                inputs.insert(text(&attrs, "name"), attrs);
            } else {
                sections.push(attrs);
            }
        }
        assert_eq!(text(&inputs["skill_number"], "number"), "3");
        assert_eq!(text(&inputs["custom"], "boolean"), "true");
        assert_eq!(text(&sections[0], "subsection"), "part");
        assert_eq!(text(&sections[0], "collapsed"), "true");
        assert_eq!(text(&sections[1], "subsection"), "other");
    }
}

fn exact_element(source: &SourceElement<'_>, original: &Table, xml: &str) {
    assert_eq!(source.source_xml(), &xml[source.source_range()]);
    assert_eq!(source.name(), text(original, "elem"));
    let attrs = table(original, "attrib");
    assert_eq!(
        source.attributes().len(),
        attrs.clone().pairs::<String, String>().count()
    );
    for attr in source.attributes() {
        assert_eq!(attr.value().raw(), &xml[attr.value().range()]);
        assert_eq!(attr.value().decoded(), text(&attrs, attr.name()));
    }
}

#[test]
fn source_projection_preserves_all_five_corpus_root_containers_and_typed_calcs() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        for index in 1..=5 {
            let xml = std::fs::read_to_string(repository().join(format!(
                "tests/fixtures/builds/breadth-20260908/build-{index:02}.xml"
            )))
            .unwrap();
            let projected = build_source::project_xml(&xml).unwrap();
            assert_eq!(projected.source_xml(), xml);
            assert_eq!(
                projected.source_sha256(),
                format!("{:x}", Sha256::digest(xml.as_bytes()))
            );
            let observed = o.observe(&xml, None);
            let loaded_calcs = scalar_map(table(
                &table(&table(&observed, "build"), "calcsTab"),
                "input",
            ));
            let node: Function = o.helpers.get("node").unwrap();
            let mut compared = 0;
            for section in projected.sections().iter().filter(|s| {
                matches!(
                    s.kind(),
                    RootSectionKind::Import
                        | RootSectionKind::Party
                        | RootSectionKind::Calcs
                        | RootSectionKind::TreeView
                )
            }) {
                compared += 1;
                let original: Table = node.call((xml.as_str(), section.element().name())).unwrap();
                exact_element(section.element(), &original, &xml);
                let children: Vec<Table> = original
                    .sequence_values::<Value>()
                    .filter_map(|v| match v.unwrap() {
                        Value::Table(t) => Some(t),
                        _ => None,
                    })
                    .collect();
                assert_eq!(section.records().len(), children.len());
                for (record, original) in section.records().iter().zip(children) {
                    exact_element(record.element(), &original, &xml);
                    if let Some(input) = record.scalar_input() {
                        assert!(record.scalar_error().is_none());
                        let actual = &loaded_calcs[input.name()];
                        if let (Scalar::Number(a), Scalar::Number(b)) = (actual, input.value()) {
                            assert_eq!(a.to_bits(), b.to_bits());
                        } else {
                            assert_eq!(actual, input.value());
                        }
                    }
                }
            }
            assert_eq!(compared, 4);
        }
    }
}

#[test]
fn projection_keeps_source_whitespace_entities_duplicates_unknowns_and_scalar_errors() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        for newline in ["\n", "\r\n"] {
            let xml = format!(
                "<PathOfBuilding2><Import importLink=\"one{newline}\ttwo &amp; &lt; &gt; &quot; &apos;\" future=\"opaque\"/><Party><ImportedBuffs name=\"unknown\"><![CDATA[alpha{newline}\tbeta]]></ImportedBuffs><Future key=\"retain\"/></Party><Calcs><Input name=\"renamed\" string=\"one{newline}\ttwo\"/><Input name=\"renamed\" string=\"last\"/><Input name=\"bad-number\" number=\"invalid\"/><Input name=\"bad-bool\" boolean=\"TRUE\"/><Future arbitrary=\"preserved\"/></Calcs><TreeView zoomX=\"99\" searchStr=\"one{newline}\ttwo\"/><FutureRoot key=\"retained\"/><Import importLink=\"second\"/></PathOfBuilding2>"
            );
            let projected = build_source::project_xml(&xml).unwrap();
            assert_eq!(projected.source_xml(), xml);
            assert_eq!(projected.sections().len(), 6);
            assert_eq!(projected.sections()[4].kind(), RootSectionKind::Unknown);
            assert_eq!(projected.sections()[5].kind(), RootSectionKind::Import);
            let observed = o.observe(&xml, None);
            let build = table(&observed, "build");
            assert_eq!(
                projected.sections()[0]
                    .element()
                    .attribute("importLink")
                    .unwrap()
                    .decoded(),
                text(&table(&build, "importTab"), "importLink")
            );
            let records = projected.sections()[2].records();
            assert_eq!(records.len(), 5);
            assert_eq!(
                records[0].scalar_input().unwrap().value(),
                &Scalar::Text(format!("one{newline}\ttwo"))
            );
            assert_eq!(
                records[1].scalar_input().unwrap().value(),
                &Scalar::Text("last".into())
            );
            assert!(records[2].scalar_error().is_some());
            assert!(records[3].scalar_error().is_some());
            assert!(records[4].scalar_input().is_none());
            let inputs = scalar_map(table(&table(&build, "calcsTab"), "input"));
            assert_eq!(inputs["renamed"], Scalar::Text("last".into()));
            assert!(!inputs.contains_key("bad-number"));
            assert_eq!(inputs["bad-bool"], Scalar::Boolean(false));
            exact_number(&table(&build, "treeView"), "zoomX", 0.0);
            // Source Save omits unknown attrs/children; projection retains them.
            assert!(
                projected.sections()[0]
                    .element()
                    .attribute("future")
                    .is_some()
            );
            assert!(matches!(
                table(&table(&observed, "import_saved"), "attrib")
                    .get::<Value>("future")
                    .unwrap(),
                Value::Nil
            ));
        }
    }
    for body in [
        "<Import importLink=\"&#10;\"/>",
        "<Import importLink =\"x\"/>",
        "<Import importLink=\"x>y\"/>",
        "<Import importLink=\"one\" importLink=\"two\"/>",
    ] {
        assert!(
            build_source::project_xml(&format!("<PathOfBuilding2>{body}</PathOfBuilding2>"))
                .is_err(),
            "{body}"
        );
    }
    let xml = "<PathOfBuilding2 xmlns:p=\"urn:future\"><p:Party p:flag=\"x\"/><Calcs><p:Input name=\"future\" string=\"x\"/></Calcs></PathOfBuilding2>";
    assert!(build_source::project_xml(xml).is_err());
    let xml = "<PathOfBuilding2><Party xmlns=\"urn:future\"/><Calcs><Input xmlns=\"urn:future\" name=\"future\" string=\"x\"/></Calcs></PathOfBuilding2>";
    let projected = build_source::project_xml(xml).unwrap();
    assert_eq!(projected.sections()[0].kind(), RootSectionKind::Unknown);
    assert_eq!(
        projected.sections()[0].element().namespace(),
        Some("urn:future")
    );
    assert!(
        projected.sections()[1].records()[0]
            .scalar_input()
            .is_none()
    );
}

#[test]
fn main_and_calcs_selectors_and_party_consumers_use_different_source_context() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        let setup = original("src/Modules/CalcSetup.lua");
        let select: Function = o
            .lua
            .load(format!(
                r#"
return function(mode,calcsMode,calcsGroup,mainGroup,count)
 local env={{mode=mode,calcsInput={{misc_buffMode=calcsMode,skill_number=calcsGroup}}}}
 local build={{mainSocketGroup=mainGroup,skillsTab={{socketGroupList={{}}}}}}
 for i=1,count do table.insert(build.skillsTab.socketGroupList,{{}})end
 {}
 {}
 return env,build
end"#,
                section(setup, "\t-- Set buff mode", "\tclassStats ="),
                section(
                    setup,
                    "\t\t-- Determine main skill group",
                    "\t\t-- Process supports"
                )
            ))
            .eval()
            .unwrap();
        for _ in 0..o.rounds {
            for mode in ["MAIN", "CALCS", "CALCULATOR"] {
                for (buff, buffs, combat, effective) in [
                    ("EFFECTIVE", true, true, true),
                    ("COMBAT", true, true, false),
                    ("BUFFED", true, false, false),
                    ("UNBUFFED", false, false, false),
                    ("unknown", false, false, false),
                ] {
                    let (env, build): (Table, Table) = select.call((mode, buff, 2, 4, 3)).unwrap();
                    assert_eq!(
                        env.get::<bool>("mode_buffs").unwrap(),
                        mode != "CALCS" || buffs
                    );
                    assert_eq!(
                        env.get::<bool>("mode_combat").unwrap(),
                        mode != "CALCS" || combat
                    );
                    assert_eq!(
                        env.get::<bool>("mode_effective").unwrap(),
                        mode != "CALCS" || effective
                    );
                    exact_number(
                        &env,
                        "mainSocketGroup",
                        if mode == "CALCS" { 2.0 } else { 3.0 },
                    );
                    exact_number(
                        &build,
                        "mainSocketGroup",
                        if mode == "CALCS" { 4.0 } else { 3.0 },
                    );
                }
                let (env, _): (Table, Table) = select.call((mode, "UNBUFFED", 99, 99, 0)).unwrap();
                exact_number(&env, "mainSocketGroup", 1.0);
            }
        }
        let consume:Function=o.lua.load(format!(r#"
return function(mode,enabled)
 local marker={{source='incoming party'}}
 local build={{partyTab={{actor=marker,enableExportBuffs=enabled,enemyModList={{'incoming enemy'}}}}}}
 local env={{mode=mode,build=build,player={{}},enemyDB={{AddList=function(self,list)self.received=list end}}}}
 {}
 {}
 return env,partyTabEnableExportBuffs,marker
end"#,section(setup,"\t\t-- Add mods from the party tab","\t\tcachedPlayerDB,"),section(original("src/Modules/CalcPerform.lua"),"\tenv.partyMembers = env.build.partyTab.actor","\tenv.minion ="))).eval().unwrap();
        for mode in ["MAIN", "CALCS", "CALCULATOR"] {
            for enabled in [false, true] {
                let (env, export, marker): (Table, bool, Table) =
                    consume.call((mode, enabled)).unwrap();
                assert_eq!(table(&env, "partyMembers"), marker);
                assert_eq!(table(&table(&env, "player"), "partyMembers"), marker);
                assert_eq!(
                    table(&table(&env, "enemyDB"), "received")
                        .get::<String>(1)
                        .unwrap(),
                    "incoming enemy"
                );
                assert_eq!(export, enabled && mode != "CALCULATOR");
            }
        }
    }
}

#[test]
fn legacy_calcs_migration_uses_effective_config_emptiness_and_removes_all_26_old_keys() {
    let config_source = original("src/Classes/ConfigTab.lua");
    let migration = section(
        config_source,
        "function ConfigTabClass:ImportCalcSettings()",
        "function ConfigTabClass:CreateUndoState()",
    );
    let mappings: Vec<(&str, &str)> = migration
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("import(\"")?;
            let (old, rest) = rest.split_once("\", \"")?;
            Some((old, rest.strip_suffix("\")").unwrap()))
        })
        .collect();
    assert_eq!(mappings.len(), 26);
    for warm in [false, true] {
        let o = Oracle::new(warm);
        let gate: Function = o
            .lua
            .load(format!(
                "return function(self)\n{}\nend",
                section(
                    original("src/Modules/Build.lua"),
                    "\tif next(self.configTab.input) == nil then",
                    "\t-- Build calculation output tables"
                )
            ))
            .eval()
            .unwrap();
        let make: Function = o.helpers.get("config").unwrap();
        for occupied in [false, true] {
            let inputs = o.context();
            let calcs = o.context();
            if occupied {
                inputs.set("unrelated source default", false).unwrap();
            }
            for (index, (old, _)) in mappings.iter().enumerate() {
                calcs.set(*old, index as f64 + 0.25).unwrap();
            }
            calcs.set("unrelated calcs key", "retained").unwrap();
            let tab: Table = make.call((inputs.clone(), calcs.clone())).unwrap();
            let build = o.context();
            build.set("configTab", tab).unwrap();
            gate.call::<()>(build).unwrap();
            for (index, (old, new)) in mappings.iter().enumerate() {
                if occupied {
                    exact_number(&calcs, old, index as f64 + 0.25);
                    assert!(matches!(inputs.get::<Value>(*new).unwrap(), Value::Nil));
                } else {
                    exact_number(&inputs, new, index as f64 + 0.25);
                    assert!(matches!(calcs.get::<Value>(*old).unwrap(), Value::Nil));
                }
            }
            assert_eq!(text(&calcs, "unrelated calcs key"), "retained");
        }
        // The method itself copies nil too; direct invocation removes a
        // pre-existing destination when its old source key is absent.
        let inputs = o.context();
        inputs.set("conditionLowLife", true).unwrap();
        let tab: Table = make.call((inputs.clone(), o.context())).unwrap();
        table(&o.lua.globals(), "ConfigTabClass")
            .get::<Function>("ImportCalcSettings")
            .unwrap()
            .call::<()>(tab)
            .unwrap();
        assert!(matches!(
            inputs.get::<Value>("conditionLowLife").unwrap(),
            Value::Nil
        ));
    }
}

#[test]
fn build_root_load_order_first_import_link_and_deferred_trees_are_original_semantics() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        let dispatch = section(
            original("src/Modules/Build.lua"),
            "\tlocal deferredPassiveTrees = { }",
            "\tif next(self.configTab.input) == nil then",
        );
        let run: Function = o
            .lua
            .load(format!(
                r#"
return function(xml,existingLink)
 local events={{}};local function record(event)table.insert(events,event)end
 local self=setmetatable({{xmlSectionList={{}},importLink=existingLink,
  Load=function(_,node)record('firstBuild:'..node.attrib.id)end,
  Save=function(_,node)node.attrib={{observed='Build saved first'}}end,
  CloseBuild=function()error('unexpected close')end}},{{__index=buildMode}})
 self.savers={{}};self.legacyLoaders={{}}
 for _,name in ipairs({{'Import','Party','Calcs','TreeView','Skills','Items','Config'}})do
  self.savers[name]={{Load=function(_,node)record(node.elem..':'..(node.attrib.id or ''))end,
  PostLoad=function()record('post:'..name)end,
  Save=function(_,node)node.attrib={{observed=name}}end}}
 end
 self.treeTab={{Load=function(_,node)record('deferred:'..node.elem..':'..node.attrib.id)end,
  Save=function(_,node)node.attrib={{observed='Tree'}}end}}
 self.savers.Tree=self.treeTab;self.legacyLoaders.Spec=self.treeTab
 assert(not self:LoadDB(xml,'source oracle'))
 {}
 local saved=self:SaveDB('source oracle')
 return self,events,saved
end"#,
                dispatch
            ))
            .eval()
            .unwrap();
        let xml = r#"<PathOfBuilding2><Calcs id="c"/><Tree id="t1"/><Build id="b1"/><Import id="i1" importLink="first"/><Unknown/><Items id="it"/><Spec id="old"/><Build id="b2"/><Import id="i2" importLink="second"/><Tree id="t2"/></PathOfBuilding2>"#;
        for link in [None, Some("context override")] {
            let (build, events, saved): (Table, Table, String) = run.call((xml, link)).unwrap();
            assert_eq!(text(&build, "importLink"), link.unwrap_or("first"));
            let events: Vec<String> = events.sequence_values().map(Result::unwrap).collect();
            assert_eq!(
                &events[..8],
                &[
                    "firstBuild:b1",
                    "Calcs:c",
                    "Import:i1",
                    "Items:it",
                    "Import:i2",
                    "deferred:Tree:t1",
                    "deferred:Spec:old",
                    "deferred:Tree:t2"
                ]
            );
            assert!(events[8..].iter().all(|event| event.starts_with("post:")));
            assert_eq!(table(&build, "xmlSectionList").raw_len(), 10);
            let document = roxmltree::Document::parse(&saved).unwrap();
            let names: Vec<&str> = document
                .root_element()
                .children()
                .filter(roxmltree::Node::is_element)
                .map(|n| n.tag_name().name())
                .collect();
            assert_eq!(names[0], "Build");
            assert_eq!(names.len(), 9);
            assert!(!names.contains(&"Unknown"));
        }
        let (build, _, _): (Table, Table, String) = run
            .call((
                "<PathOfBuilding2><Import/><Import importLink=\"second\"/></PathOfBuilding2>",
                None::<String>,
            ))
            .unwrap();
        assert!(matches!(
            build.get::<Value>("importLink").unwrap(),
            Value::Nil
        ));
    }
}

#[test]
fn party_save_uses_separate_current_export_context_in_source_order() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        for enabled in [false, true] {
            let observed=o.observe(&format!("<PathOfBuilding2><Import exportParty=\"{enabled}\"/><Party><ExportedBuffs name=\"Aura\">stale imported export text</ExportedBuffs></Party></PathOfBuilding2>"),None);
            let party = table(&table(&observed, "build"), "partyTab");
            let exports = table(&party, "buffExports");
            for key in [
                "PlayerMods",
                "Aura",
                "Curse",
                "Warcry",
                "Link",
                "EnemyConditions",
                "EnemyMods",
            ] {
                let entry = o.context();
                entry.set("ConvertedToText", true).unwrap();
                entry
                    .set("string", format!("current context {key}"))
                    .unwrap();
                exports.set(key, entry).unwrap();
            }
            let save: Function = o.helpers.get("save").unwrap();
            let (saved, _): (Table, String) = save.call((party, "Party")).unwrap();
            let nodes: Vec<Table> = saved.sequence_values().map(Result::unwrap).collect();
            if !enabled {
                assert!(nodes.is_empty());
                continue;
            }
            assert_eq!(nodes.len(), 7);
            for ((node, name), key) in nodes
                .iter()
                .zip([
                    "PartyMemberStats",
                    "Aura",
                    "Curse",
                    "Warcry Skills",
                    "Link Skills",
                    "EnemyConditions",
                    "EnemyMods",
                ])
                .zip([
                    "PlayerMods",
                    "Aura",
                    "Curse",
                    "Warcry",
                    "Link",
                    "EnemyConditions",
                    "EnemyMods",
                ])
            {
                assert_eq!(text(node, "elem"), "ExportedBuffs");
                assert_eq!(text(&table(node, "attrib"), "name"), name);
                assert_eq!(
                    node.get::<String>(1).unwrap(),
                    format!("current context {key}")
                );
            }
        }
    }
}
