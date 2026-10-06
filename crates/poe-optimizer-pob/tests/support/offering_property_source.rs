//! Original physical Offering property assembly; no native fallback or cap law.
use super::magnified_area_support::{edit_gem, edit_group, focus, set_attr, template};
use super::*;
use std::collections::BTreeSet;

const TEST_NAME: &str = "offering_property::offering_final_inputs_follow_actual_property_consumers";
const OUTPUT: &str = "POE_OFFERING_PROPERTY_SOURCE_OUT";
const MODE: &str = "POE_OFFERING_PROPERTY_SOURCE_CHILD";
const OBSERVER: &str = include_str!("offering_property_source.lua");
const PAIN: &str = "PainOfferingPlayer";
const I: &str = "ProlongedDurationSupportPlayer";
const II: &str = "ProlongedDurationSupportPlayerTwo";

#[test]
#[ignore = "requires pinned PoB; actual Offering property consumption in both JIT modes"]
fn offering_final_inputs_follow_actual_property_consumers() {
    super::physical_support::run_modes(
        super::physical_support::Witness {
            name: TEST_NAME,
            child_env: MODE,
            output_env: OUTPUT,
            default_output: "runs/owned-offering-property-source-01",
            label: "Offering properties",
        },
        child,
    );
}

fn child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let dir = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json = serde_json::from_slice(&fs::read(dir.join("index.json")).unwrap()).unwrap();
    let originals: Vec<_> = (1..=5)
        .map(|i| fs::read(dir.join(format!("build-{i:02}.xml"))).unwrap())
        .collect();
    let mut cases = vec![];
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(digest(bytes), index["builds"][i]["xml_sha256"]);
        cases.push(observe(
            root,
            &format!("original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
            true,
            None,
        ));
    }
    let original = std::str::from_utf8(&originals[4]).unwrap();
    let controls = controls(original);
    for (name, xml, control) in &controls {
        cases.push(observe(
            root,
            name,
            xml,
            enabled,
            true,
            Some(control.clone()),
        ));
    }
    for (i, bytes) in originals.iter().enumerate() {
        cases.push(observe(
            root,
            &format!("repeat-original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
            true,
            None,
        ));
    }
    cases.push(observe(
        root,
        "unhooked-original-05",
        original,
        enabled,
        false,
        None,
    ));
    let copies = controls
        .iter()
        .find(|c| c.0 == "distinct-offering-copies")
        .unwrap();
    cases.push(observe(
        root,
        "unhooked-distinct-offering-copies",
        &copies.1,
        enabled,
        false,
        Some(copies.2.clone()),
    ));
    assert_eq!(cases.len(), 26);
    let mut files = FILES.to_vec();
    files.extend([
        "src/Classes/Item.lua",
        "src/Classes/ItemsTab.lua",
        "src/Classes/ModStore.lua",
        "src/Classes/ModDB.lua",
        "src/Classes/ModList.lua",
        "src/Modules/ItemTools.lua",
        "src/Modules/ModParser.lua",
        "src/Data/Skills/act_int.lua",
    ]);
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":digest(OBSERVER.as_bytes()),"lifecycle_sha256":digest(LIFECYCLE.as_bytes()),
        "files":files.into_iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),
        "original_sources":originals.iter().enumerate().map(|(i,b)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(b)})).collect::<Vec<_>>(),
        "business_wrappers":false,"source_tables_mutated":false,"diagnostic_queries_as_consumption":false,
        "native_build_parity":false,"native_inventory_authority":false,"fallback_semantics_authority":false,
        "game_level_cap_authority":false,"fractional_game_domain_authority":false,"curve_approximation_authority":false,
        "observation":"original call/return hooks, exact original result/cache/object joins; no method replacement or diagnostic query substitution",
        "lifecycle_stages":STAGES,"numeric_tolerance":0,"cases":cases});
    let bytes = serde_json::to_vec(&report).unwrap();
    let suffix = if enabled { "on" } else { "off" };
    let raw = out.join(format!("source-jit-{suffix}.raw.json"));
    fs::write(&raw, &bytes).unwrap();
    eprintln!(
        "Offering property evidence: {} bytes at {}",
        bytes.len(),
        raw.display()
    );
    assert!(bytes.len() <= 128 * 1024 * 1024, "bounded source report");
    check(&report);
    fs::write(out.join(format!("source-jit-{suffix}.json")), bytes).unwrap();
    for (i, bytes) in originals.iter().enumerate() {
        assert!(
            fs::read(dir.join(format!("build-{:02}.xml", i + 1))).unwrap() == *bytes,
            "original fixture changed"
        );
    }
}

fn install(lua: &Lua) -> Result<Function, RuntimeError> {
    lua.globals().set("offeringPropertyPhase", "before")?;
    Ok(lua
        .load(OBSERVER)
        .set_name("@offering-property-call-observer-install")
        .eval()?)
}
fn snapshot(lua: &Lua) -> Result<Json, RuntimeError> {
    lua.globals().set("offeringPropertyPhase", "observe")?;
    let value: Value = lua
        .load(OBSERVER)
        .set_name("@offering-property-call-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn stage(lua: &Lua) -> Result<Json, RuntimeError> {
    let a = snapshot(lua)?;
    let b = snapshot(lua)?;
    assert_eq!(
        json_evidence::first_difference(&a, &b, "observer-noninterference"),
        None
    );
    Ok(a)
}
fn rebuild_observed(lua: &Lua) -> Result<(), RuntimeError> {
    let cleanup = install(lua)?;
    // Use the same fixed normal frame transition without installing the
    // support-admission observer's separate hook over this property hook.
    let result = lua
        .load(
            r#"
local function original(f,path,line)
 local i=debug.getinfo(f,"S");local p=i.source:gsub("\\","/")
 assert(i.what=="Lua" and p:sub(-#path)==path and i.linedefined==line);return f
end
local callback=original(runCallback,"HeadlessWrapper.lua",17)
local frame=original(build.OnFrame,"Modules/Build.lua",1285)
local output=original(build.calcsTab.BuildOutput,"Classes/CalcsTab.lua",486)
assert(output==djinnOriginals.refs.calcs_tab_output and build.buildFlag==false)
local revision=build.outputRevision;local main,calcs=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
build.buildFlag=true;callback("OnFrame")
assert(build.buildFlag==false and build.outputRevision==revision+1)
assert(build.calcsTab.mainEnv~=main and build.calcsTab.calcsEnv~=calcs)
assert(runCallback==callback and build.OnFrame==frame and build.calcsTab.BuildOutput==output)
"#,
        )
        .set_name("@offering-property-original-frame-rebuild")
        .exec();
    let removed = cleanup.call::<()>(());
    result?;
    removed?;
    Ok(())
}
fn observe(
    root: &Path,
    name: &str,
    xml: &str,
    enabled: bool,
    instrumented: bool,
    control: Option<Json>,
) -> Json {
    eprintln!(
        "Offering property case {name}, JIT {}",
        if enabled { "on" } else { "off" }
    );
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("offeringPropertyXml", xml)?;
        lua.globals().set("offeringPropertyJit", enabled)?;
        lua.globals()
            .set("offeringPropertyInstrumented", instrumented)?;
        lua.load("if offeringPropertyJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let before_build = |lua: &Lua| -> Result<Function, RuntimeError> {
        lua.globals().set("djinnPhase", "before")?;
        let lifecycle: Function = lua
            .load(LIFECYCLE)
            .set_name("@offering-property-original-lifecycle")
            .eval()?;
        let cleanup = install(lua)?;
        let combine: Function = lua.load("return function(observer,auth) return function() local ok,err=pcall(observer);auth();if not ok then error(err,0)end end end").eval()?;
        Ok(combine.call((cleanup, lifecycle))?)
    };
    let observer = |lua: &Lua| -> Result<Json, RuntimeError> {
        let fresh = stage(lua)?;
        rebuild_observed(lua)?;
        let rebuilt_once = stage(lua)?;
        rebuild_observed(lua)?;
        let rebuilt_twice = stage(lua)?;
        Ok(json!({"fresh":fresh,"rebuilt_once":rebuilt_once,"rebuilt_twice":rebuilt_twice}))
    };
    let scratch = tempfile::tempdir().unwrap();
    let observed = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        xml,
        None,
        name == "fractional-raw-level-diagnostic",
        Some(&before),
        Some(&before_build),
        Some(&observer),
    )
    .unwrap_or_else(|e| panic!("{name}: source failed: {e}"));
    assert_eq!(observed["configuration_method_wrappers"], false);
    assert_eq!(observed["original_build_output_available"], true);
    let states = &observed["additional_observation"];
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([109; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    for stage in STAGES {
        for row in rows(&states[stage]["saved"]) {
            let source = evidence
                .rows()
                .iter()
                .find(|r| {
                    Some(u64::from(r.occurrence().id().ordinal())) == row["source_ordinal"].as_u64()
                })
                .unwrap();
            assert_eq!(source.occurrence().name(), "Gem");
            assert_eq!(
                source.attributes().len(),
                row["attributes"].as_object().unwrap().len()
            );
            for a in source.attributes() {
                assert_eq!(row["attributes"][&a.origin().name], a.decoded().unwrap());
            }
        }
    }
    json!({"name":name,"xml_sha256":digest(xml.as_bytes()),"source_identity":evidence.identity(),"independent_source_bindings_verified":true,"control":control,"instrumented":instrumented,"states":states})
}

fn item_change(xml: &str, slot: &str, remove: bool) -> (String, Json) {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let items = doc.descendants().find(|n| n.has_tag_name("Items")).unwrap();
    let active = items.attribute("activeItemSet").unwrap();
    let set = items
        .children()
        .find(|n| n.has_tag_name("ItemSet") && n.attribute("id") == Some(active))
        .unwrap();
    let selected = set
        .children()
        .find(|n| n.has_tag_name("Slot") && n.attribute("name") == Some(slot))
        .unwrap();
    let id = selected.attribute("itemId").unwrap();
    let item = items
        .children()
        .find(|n| n.has_tag_name("Item") && n.attribute("id") == Some(id))
        .unwrap();
    let text = item.children().find(|n| n.is_text()).unwrap();
    let raw = &xml[text.range()];
    let line = "+1 to Level of all Minion Skills";
    assert_eq!(raw.matches(line).count(), 1);
    let next = if remove {
        raw.replacen(line, "", 1)
    } else {
        format!(
            "{}\n100% increased bonuses gained from equipped rings and amulets\n",
            raw.trim_end()
        )
    };
    let mut changed = xml.to_owned();
    changed.replace_range(text.range(), &next);
    roxmltree::Document::parse(&changed).unwrap();
    (
        changed,
        json!({"slot":slot,"item_id":id,"removed_line":if remove {Some(line)}else{None},"added_copy_percent":if remove {0}else{100},"roll_legality_authority":false}),
    )
}
fn controls(original: &str) -> Vec<(String, String, Json)> {
    let focused = focus(original, PAIN, None, 1, 1);
    let doc = roxmltree::Document::parse(original).unwrap();
    let archived = doc
        .descendants()
        .find(|n| n.has_tag_name("Gem") && n.attribute("skillId") == Some(II))
        .unwrap();
    let archived = &original[archived.range()];
    let raw = template(original, PAIN, PAIN);
    let second = set_attr(&set_attr(&raw, "level", "5"), "quality", "13");
    let copies = edit_group(&focused, PAIN, |g| {
        set_attr(
            &g.replacen("</Skill>", &format!("{second}</Skill>"), 1),
            "mainActiveSkillCalcs",
            "2",
        )
    });
    let mut output = vec![];
    for (name, xml, support, levels, qualities, diagnostic) in [
        (
            "focused-tier-i",
            focused.clone(),
            Some(I),
            vec![22.0],
            vec![0],
            false,
        ),
        (
            "archived-tier-ii",
            edit_group(&focused, PAIN, |g| edit_gem(g, I, |_| archived.to_owned())),
            Some(II),
            vec![22.0],
            vec![0],
            false,
        ),
        (
            "support-remove",
            edit_group(&focused, PAIN, |g| edit_gem(g, I, |_| String::new())),
            None,
            vec![22.0],
            vec![0],
            false,
        ),
        (
            "support-disable",
            edit_group(&focused, PAIN, |g| {
                edit_gem(g, I, |s| set_attr(s, "enabled", "false"))
            }),
            None,
            vec![22.0],
            vec![0],
            false,
        ),
        (
            "raw-level",
            edit_group(&focused, PAIN, |g| {
                edit_gem(g, PAIN, |s| set_attr(s, "level", "5"))
            }),
            Some(I),
            vec![7.0],
            vec![0],
            false,
        ),
        (
            "raw-quality",
            edit_group(&focused, PAIN, |g| {
                edit_gem(g, PAIN, |s| set_attr(s, "quality", "13"))
            }),
            Some(I),
            vec![22.0],
            vec![13],
            false,
        ),
        (
            "distinct-offering-copies",
            copies.clone(),
            Some(I),
            vec![22.0, 7.0],
            vec![0, 13],
            false,
        ),
        (
            "fractional-raw-level-diagnostic",
            edit_group(&focused, PAIN, |g| {
                edit_gem(g, PAIN, |s| set_attr(s, "level", "5.5"))
            }),
            Some(I),
            vec![],
            vec![0],
            true,
        ),
        (
            "repeat-focused-tier-i",
            focused.clone(),
            Some(I),
            vec![22.0],
            vec![0],
            false,
        ),
        (
            "repeat-distinct-offering-copies",
            copies,
            Some(I),
            vec![22.0, 7.0],
            vec![0, 13],
            false,
        ),
    ] {
        output.push((name.to_owned(),xml,json!({"support":support,"expected_levels":levels,"expected_qualities":qualities,"fractional_diagnostic":diagnostic,"item_changes":[]})));
    }
    for (name, slot, remove, level) in [
        ("helmet-level-remove", "Helmet", true, 21),
        ("amulet-level-remove", "Amulet", true, 21),
        ("amulet-copy-bonus", "Amulet", false, 23),
    ] {
        let (xml, item) = item_change(&focused, slot, remove);
        output.push((name.to_owned(),xml,json!({"support":I,"expected_levels":[level],"expected_qualities":[0],"fractional_diagnostic":false,"item_changes":[item]})));
    }
    let (xml, a) = item_change(&focused, "Helmet", true);
    let (xml, b) = item_change(&xml, "Amulet", true);
    output.push(("both-level-remove".into(),xml,json!({"support":I,"expected_levels":[20],"expected_qualities":[0],"fractional_diagnostic":false,"item_changes":[a,b]})));
    assert_eq!(output.len(), 14);
    output
}

fn named<'a>(r: &'a Json, name: &str) -> &'a Json {
    let found: Vec<_> = rows(&r["cases"])
        .iter()
        .filter(|c| c["name"] == name)
        .collect();
    assert_eq!(found.len(), 1);
    found[0]
}
fn same(a: &Json, b: &Json, label: &str) {
    assert_eq!(json_evidence::first_difference(a, b, label), None);
}
fn check_context(c: &Json, stage: &Json, fractional: bool) {
    for k in [
        "actor_is_player",
        "exact_source",
        "no_attached_minion",
        "no_summoning_parent",
        "distinct_source_cache",
    ] {
        assert_eq!(c[k], true);
    }
    assert_eq!(c["disabled"], false);
    assert_eq!(
        c["source_catalog"],
        "Metadata/Items/Gems/SkillGemPainOffering"
    );
    assert!(
        c["final"]["level"]
            .as_f64()
            .is_some_and(|n| n >= 1.0 && n.fract() == 0.0)
    );
    assert!(
        c["final"]["quality"]
            .as_f64()
            .is_some_and(|n| n >= 0.0 && n.fract() == 0.0)
    );
    assert!(c["final_root_level"].is_object() && c["final_stat_set_level"].is_object());
    let ordinary = rows(&c["ordinary"]);
    assert_eq!(ordinary.len(), 1);
    let o = &ordinary[0];
    assert_eq!(o["query_store_is_actor"], true);
    let mut delta = [0.0; 2];
    let mut partition = BTreeSet::new();
    for (key, matched) in [("matched", true), ("rejected", false)] {
        for index in rows(&o[key]) {
            let index = index.as_u64().unwrap() as usize;
            assert!(index > 0 && partition.insert(index));
            let record = &o["candidates"][index - 1];
            assert_eq!(record["mod"]["name"], "GemProperty");
            same(
                &record["value"],
                &record["mod"]["value"],
                "original candidate payload",
            );
            if matched {
                match record["value"]["key"].as_str().unwrap() {
                    "level" => delta[0] += record["value"]["value"].as_f64().unwrap(),
                    "quality" => delta[1] += record["value"]["value"].as_f64().unwrap(),
                    other => panic!("unreviewed property {other}"),
                }
            }
            for copy in rows(&o["copy_indices"][index - 1]) {
                let copy = &stage["copies"][copy.as_u64().unwrap() as usize - 1];
                assert!(
                    rows(&copy["insertions"])
                        .iter()
                        .any(|i| i["record"] == record["mod"])
                );
            }
        }
    }
    assert_eq!(partition.len(), rows(&o["candidates"]).len());
    for (index, key) in ["level", "quality"].into_iter().enumerate() {
        assert_eq!(
            o["after"][key].as_f64().unwrap(),
            o["before"][key].as_f64().unwrap() + delta[index]
        );
    }
    let supports = rows(&c["supported"]);
    assert!(!supports.is_empty());
    for s in supports {
        assert_eq!(s["query_parent_is_actor"], true);
        assert_eq!(s["cache_identity_preserved"], true);
        same(
            &s["before"],
            &s["after"],
            "query itself did not apply properties",
        );
        for p in rows(&s["properties"]) {
            assert_eq!(p["mod"]["name"], "SupportedGemProperty");
        }
        for candidate in rows(&s["supports"]) {
            assert_eq!(candidate["exact_source"], true);
            assert_ne!(
                candidate["source"]["source_ordinal"],
                c["source"]["source_ordinal"]
            );
        }
    }
    let a = rows(&c["assembly"]);
    assert_eq!(a.len(), 1);
    same(&a[0]["before"], &o["after"], "ordinary before assembly");
    same(&a[0]["after"], &c["final"], "assembly final");
    let merges = rows(&a[0]["merges"]);
    assert_eq!(merges.len(), 1);
    assert_eq!(merges[0]["exact_destination"], true);
    assert_eq!(merges[0]["stat_set_exact"], true);
    let mut prepared = a[0]["before"].clone();
    for p in rows(&supports[0]["properties"]) {
        let v = &p["value"];
        if v["keyword"] == "grants_active_skill" {
            let key = v["key"].as_str().unwrap();
            prepared[key] = json!(prepared[key].as_f64().unwrap() + v["value"].as_f64().unwrap());
        }
    }
    for key in ["level", "quality"] {
        assert_eq!(prepared[key].as_f64(), merges[0]["before"][key].as_f64());
    }
    let validations = rows(&c["validation"]);
    assert!(!validations.is_empty());
    for v in validations {
        assert_eq!(v["after_lookup"], true);
        if !fractional {
            assert_eq!(v["before_lookup"], true);
            same(&v["before"], &v["after"], "integral valid level unchanged");
        }
    }
    // Physical source recovery can occur in ProcessSocketGroup before ordinary
    // properties are assembled. Its separate source_validation trace must not
    // be mistaken for a final native level rule.
}
fn check(r: &Json) {
    assert_eq!(rows(&r["cases"]).len(), 26);
    let mut total = 0;
    for case in rows(&r["cases"]) {
        assert_eq!(case["independent_source_bindings_verified"], true);
        for stage in STAGES {
            let s = &case["states"][stage];
            for k in [
                "original_methods_preserved",
                "hook_removed",
                "jit_mode_preserved",
            ] {
                assert_eq!(s[k], true);
            }
            for k in ["business_wrappers", "source_tables_mutated"] {
                assert_eq!(s[k], false);
            }
            if case["instrumented"] == true {
                for c in rows(&s["contexts"]) {
                    check_context(c, s, case["control"]["fractional_diagnostic"] == true);
                    total += 1;
                }
            }
            if !case["control"].is_null() && case["control"]["fractional_diagnostic"] != true {
                let control = &case["control"];
                for mode in ["MAIN", "CALCS"] {
                    let contexts: Vec<_> = rows(&s["contexts"])
                        .iter()
                        .filter(|c| c["mode"] == mode)
                        .collect();
                    assert_eq!(contexts.len(), rows(&control["expected_levels"]).len());
                    let mut sources = BTreeSet::new();
                    for (i, c) in contexts.iter().enumerate() {
                        assert_eq!(
                            c["final"]["level"].as_f64(),
                            control["expected_levels"][i].as_f64()
                        );
                        assert_eq!(c["final"]["quality"], control["expected_qualities"][i]);
                        assert!(sources.insert(c["source"]["source_ordinal"].as_u64().unwrap()));
                        if case["instrumented"] == true {
                            let supported = &c["supported"][0];
                            let prolonged: Vec<_> = rows(&supported["supports"])
                                .iter()
                                .filter(|c| {
                                    [Some(I), Some(II)].contains(&c["effect"].as_str())
                                        && c["is_supporting"] == true
                                })
                                .collect();
                            assert_eq!(prolonged.len(), usize::from(!control["support"].is_null()));
                            if let Some(p) = prolonged.first() {
                                assert_eq!(p["effect"], control["support"]);
                                assert_eq!(p["admitted"], true);
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(total > 50);
    let fractional = named(r, "fractional-raw-level-diagnostic");
    assert!(
        rows(&fractional["states"]["fresh"]["contexts"])
            .iter()
            .any(|c| rows(&c["source_validation"])
                .iter()
                .any(|v| v["before_lookup"] == false && v["before"] != v["after"])),
        "fractional diagnostic must expose original loader recovery"
    );
    for (a, b) in (1..=5)
        .map(|i| {
            (
                format!("original-{i:02}"),
                format!("repeat-original-{i:02}"),
            )
        })
        .chain([
            ("focused-tier-i".into(), "repeat-focused-tier-i".into()),
            (
                "distinct-offering-copies".into(),
                "repeat-distinct-offering-copies".into(),
            ),
        ])
    {
        same(
            &named(r, &a)["states"],
            &named(r, &b)["states"],
            "independent replay",
        );
    }
    for name in ["original-05", "distinct-offering-copies"] {
        let hooked = named(r, name);
        let unhooked = named(r, &format!("unhooked-{name}"));
        same(
            &hooked["xml_sha256"],
            &unhooked["xml_sha256"],
            "same unhooked input",
        );
        for stage in STAGES {
            same(
                &hooked["states"][stage]["outcomes"],
                &unhooked["states"][stage]["outcomes"],
                "hook outcome noninterference",
            );
        }
    }
    for stage in STAGES {
        for mode in ["MAIN", "CALCS"] {
            let s = &named(r, "original-05")["states"][stage];
            let c = rows(&s["contexts"])
                .iter()
                .find(|c| c["mode"] == mode)
                .unwrap();
            assert_eq!(c["raw"]["level"], 20);
            assert_eq!(c["final"]["level"], 22);
            assert_eq!(c["final"]["quality"], 0);
            assert!(
                rows(&c["supported"][0]["properties"]).is_empty(),
                "selected supported-property domain changed"
            );
            let ordinary = &c["ordinary"][0];
            assert_eq!(rows(&ordinary["matched"]).len(), 3);
            assert!(rows(&ordinary["rejected"]).is_empty());
            for slot in ["Helmet", "Amulet"] {
                let item = &s["equipped"][mode][slot];
                assert_eq!(item["present"], true);
                assert_eq!(item["exact_registered"], true);
                let prefix = format!("Item:{}:", item["id"].as_u64().unwrap());
                let records: Vec<_> = rows(&ordinary["candidates"])
                    .iter()
                    .filter(|r| {
                        r["mod"]["source"]
                            .as_str()
                            .is_some_and(|p| p.starts_with(&prefix))
                    })
                    .collect();
                assert_eq!(records.len(), 1);
                let record = &records[0]["mod"];
                assert_eq!(record["sourceSlot"], slot);
                assert_eq!(record["type"], "LIST");
                assert_eq!(record["flags"], 0);
                assert_eq!(record["keywordFlags"], 0);
                assert!(rows(&record["tags"]).is_empty());
                same(
                    &record["value"],
                    &json!({"key":"level","keyOfScaledMod":"value","keyword":"minion","value":1}),
                    "exact equipped property",
                );
            }
            let zero: Vec<_> = rows(&ordinary["candidates"])
                .iter()
                .enumerate()
                .filter(|(_, r)| r["mod"]["value"]["value"] == 0)
                .collect();
            assert_eq!(zero.len(), 1);
            let indices = rows(&ordinary["copy_indices"][zero[0].0]);
            assert_eq!(indices.len(), 1);
            let source_copy = &s["copies"][indices[0].as_u64().unwrap() as usize - 1];
            assert_eq!(source_copy["scale"], 0);
            assert_eq!(
                source_copy["amulet"]["id"],
                s["equipped"][mode]["Amulet"]["id"]
            );
            same(
                &source_copy["amulet"]["original_record"]["value"],
                &json!({"key":"level","keyOfScaledMod":"value","keyword":"minion","value":1}),
                "exact copy input",
            );
            let copies: Vec<_> = rows(&s["copies"])
                .iter()
                .filter(|p| p.get("amulet").is_some())
                .collect();
            assert!(!copies.is_empty());
            for copy in copies {
                for k in [
                    "exact_registered",
                    "copied_from_actual_list",
                    "exact_copy_argument",
                    "exact_destination",
                ] {
                    assert_eq!(copy["amulet"][k], true);
                }
                assert_eq!(copy["scale"], 0);
                assert_eq!(copy["before"], copy["input_after"]);
            }
        }
    }
}
