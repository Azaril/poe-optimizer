//! Source-only Magnified Area delivery. No native inventory or full-build claim.
use super::*;
use std::collections::BTreeSet;
use std::ops::Range;

const TEST_NAME: &str =
    "magnified_area_support::complete_magnified_area_delivery_uses_original_recipients";
const MODE: &str = "POE_MAGNIFIED_AREA_SOURCE_CHILD";
const OUTPUT: &str = "POE_MAGNIFIED_AREA_SOURCE_OUT";
const DELIVERY: &str = include_str!("magnified_area_support_delivery.lua");
const I: &str = "SupportMagnifiedAreaPlayer";
const II: &str = "SupportMagnifiedAreaPlayerTwo";
const ICE: &str = "IceNovaPlayer";
// Thirty-six cases retain complete occurrence, admission and delivery evidence
// for three stages. The analogous 22-case Bidding receipt is already 40.8 MB.
const REPORT_LIMIT: usize = 128 * 1024 * 1024;

#[test]
#[ignore = "requires complete pinned PoB; finite Magnified Area numerical delivery in both JIT modes"]
fn complete_magnified_area_delivery_uses_original_recipients() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join(
        std::env::var_os(OUTPUT)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("runs/owned-magnified-area-source-01")),
    );
    if let Some(mode) = std::env::var_os(MODE) {
        assert!(mode == "off" || mode == "on");
        child(&root, &out, mode == "on");
        return;
    }
    assert!(
        std::env::var_os(CHILD).is_none(),
        "unset historical preparation child selector"
    );
    assert!(
        !out.exists(),
        "immutable source evidence exists; choose fresh {OUTPUT}"
    );
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST_NAME, "--ignored", "--nocapture"])
            .env(MODE, mode)
            .env(OUTPUT, &out)
            .env_remove(CHILD)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = process.try_wait().unwrap() {
                assert!(status.success(), "{}\n{}", path.display(), tail(&path));
                break;
            }
            if start.elapsed() > Duration::from_secs(600) {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!(
                    "Magnified source deadline: {}\n{}",
                    path.display(),
                    tail(&path)
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    json_evidence::assert_files_equal(
        &out.join("source-jit-off.json"),
        &out.join("source-jit-on.json"),
        "Magnified Area source JIT parity",
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
        cases.push(observe_source(
            root,
            &format!("original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
            None,
        ));
    }
    let original = std::str::from_utf8(&originals[4]).unwrap();
    for (name, xml, control) in controls(original) {
        roxmltree::Document::parse(&xml).unwrap();
        assert_ne!(xml, original);
        cases.push(observe_source(root, &name, &xml, enabled, Some(control)));
    }
    for (i, bytes) in originals.iter().enumerate() {
        cases.push(observe_source(
            root,
            &format!("repeat-original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
            None,
        ));
    }
    assert_eq!(cases.len(), 36);
    let mut files = FILES.to_vec();
    files.extend([
        "src/Data/SkillStatMap.lua",
        "src/Data/Skills/act_int.lua",
        "src/Modules/CalcOffence.lua",
        "src/Classes/ModStore.lua",
        "src/Classes/ModDB.lua",
        "src/Classes/ModList.lua",
    ]);
    let report = json!({
        "schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,
        "manifest_sha256":pinned::manifest_sha256(),"observer_sha256":digest(DELIVERY.as_bytes()),
        "files":files.into_iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),
        "original_sources":originals.iter().enumerate().map(|(i,b)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(b)})).collect::<Vec<_>>(),
        "business_wrappers":false,"source_tables_mutated":false,"source_cfg_modified":false,
        "native_build_parity":false,"native_inventory_authority":false,
        "final_cost_or_radius_formula_authority":false,"canonical_parity_lifecycle_selected":false,
        "purpose":"finite original support-source delivery and consumers; primary/child costs are separate from native topology and final payable cost",
        "observation_order":"exact original per-channel record order; no cross-channel arithmetic combined",
        "numeric_tolerance":0,"lifecycle_stages":STAGES,"cases":cases
    });
    let bytes = serde_json::to_vec(&report).unwrap();
    let suffix = if enabled { "on" } else { "off" };
    let raw_path = out.join(format!("source-jit-{suffix}.raw.json"));
    fs::write(&raw_path, &bytes).unwrap();
    eprintln!(
        "Magnified raw source evidence: {} bytes at {}",
        bytes.len(),
        raw_path.display()
    );
    assert!(
        bytes.len() <= REPORT_LIMIT,
        "Magnified report is {} bytes; bound {REPORT_LIMIT}; raw diagnostics retained at {}",
        bytes.len(),
        raw_path.display()
    );
    check_replays(&report);
    check(&report);
    fs::write(out.join(format!("source-jit-{suffix}.json")), bytes).unwrap();
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(
            fs::read(dir.join(format!("build-{:02}.xml", i + 1))).unwrap(),
            *bytes
        );
    }
}

#[test]
#[ignore = "validates an explicitly supplied captured report only; no source execution or fresh/JIT proof"]
fn validate_saved_magnified_report_without_source_execution() {
    let path = PathBuf::from(
        std::env::var_os("POE_MAGNIFIED_AREA_VALIDATE_REPORT")
            .expect("set explicit captured report path"),
    );
    assert!(fs::metadata(&path).unwrap().len() <= REPORT_LIMIT as u64);
    let report: Json = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["manifest_sha256"], pinned::manifest_sha256());
    assert_eq!(report["observer_sha256"], digest(DELIVERY.as_bytes()));
    assert_eq!(report["native_inventory_authority"], false);
    assert_eq!(report["native_build_parity"], false);
    check_replays(&report);
    check(&report);
    eprintln!(
        "Captured-report assertions passed: {}; no source execution, fresh replay or JIT comparison was performed",
        path.display()
    );
}
fn check_replays(report: &Json) {
    for i in 1..=5 {
        let a = case(report, &format!("original-{i:02}"));
        let b = case(report, &format!("repeat-original-{i:02}"));
        assert_eq!(
            json_evidence::first_difference(&a["states"], &b["states"], "original-replay"),
            None,
            "independent Original{i:02} replay"
        );
        assert_eq!(a["xml_sha256"], b["xml_sha256"]);
    }
}

fn observe_source(
    root: &Path,
    name: &str,
    xml: &str,
    enabled: bool,
    control: Option<Json>,
) -> Json {
    let mut observed = observe_with_extra(root, name, xml, enabled, control, Some(DELIVERY));
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([103; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    assert_eq!(
        observed["source_identity"],
        serde_json::to_value(evidence.identity()).unwrap()
    );
    for stage in STAGES {
        for context in rows(&observed["states"][stage]["delivery"]["contexts"]) {
            let binding = &context["group"];
            let Some(ordinal) = binding["source_ordinal"].as_u64() else {
                // A regenerated group has no invented authored location. The
                // numerical controls below require real authored support origins.
                assert_eq!(binding["source_present"], true, "{name}/{stage}");
                assert!(context["source"]["source_ordinal"].is_null());
                assert!(context["source"]["saved_attributes"].is_null());
                assert!(rows(&context["candidates"]).is_empty());
                for child in rows(&context["children"]) {
                    assert!(rows(&child["candidates"]).is_empty());
                }
                continue;
            };
            let group = evidence
                .rows()
                .get(usize::try_from(ordinal).unwrap())
                .unwrap();
            assert_eq!(u64::from(group.occurrence().id().ordinal()), ordinal);
            assert_eq!(group.occurrence().name(), "Skill");
            assert_eq!(
                group.attribute("source").is_some(),
                binding["source_present"] == true
            );
            assert_eq!(
                group.attribute("source").map(|a| a.decoded().unwrap()),
                binding["source"].as_str()
            );
            let set = evidence.row(group.occurrence().parent().unwrap()).unwrap();
            assert_eq!(set.occurrence().name(), "SkillSet");
            assert_eq!(
                set.attribute("id")
                    .unwrap()
                    .decoded()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap(),
                binding["preset"].as_u64().unwrap()
            );
            check_saved_origin(&context["source"], group, &evidence);
            for candidate in rows(&context["candidates"]) {
                check_saved_origin(&candidate["origin"], group, &evidence);
            }
            for child in rows(&context["children"]) {
                for candidate in rows(&child["candidates"]) {
                    check_saved_origin(&candidate["origin"], group, &evidence);
                }
            }
        }
    }
    observed["independent_source_bindings_verified"] = json!(true);
    observed
}
pub(super) fn check_saved_origin(
    origin: &Json,
    group: &poe_optimizer_import::owned_source::SourceEvidenceRow<'_>,
    evidence: &SourceProjectEvidence<'_>,
) {
    let ordinal = origin["source_ordinal"].as_u64().unwrap();
    let row = evidence
        .rows()
        .get(usize::try_from(ordinal).unwrap())
        .unwrap();
    assert_eq!(u64::from(row.occurrence().id().ordinal()), ordinal);
    assert_eq!(row.occurrence().name(), "Gem");
    assert_eq!(row.occurrence().parent(), Some(group.occurrence().id()));
    let gems: Vec<_> = group
        .children()
        .iter()
        .map(|id| evidence.row(*id).unwrap())
        .filter(|r| r.occurrence().name() == "Gem")
        .collect();
    let position = usize::try_from(origin["position"].as_u64().unwrap()).unwrap();
    assert!(position > 0 && position <= gems.len());
    assert_eq!(gems[position - 1].occurrence().id(), row.occurrence().id());
    let attributes = origin["saved_attributes"].as_object().unwrap();
    assert_eq!(attributes.len(), row.attributes().len());
    for attribute in row.attributes() {
        assert!(attribute.origin().namespace.is_none());
        assert_eq!(
            attributes[&attribute.origin().name],
            attribute.decoded().unwrap()
        );
    }
    assert_eq!(
        origin["skill_id"],
        row.attribute("skillId").unwrap().decoded().unwrap()
    );
}
fn case<'a>(report: &'a Json, name: &str) -> &'a Json {
    let matches: Vec<_> = rows(&report["cases"])
        .iter()
        .filter(|c| c["name"] == name)
        .collect();
    assert_eq!(matches.len(), 1);
    matches[0]
}
pub(super) fn group_range(xml: &str, effect: &str) -> (Range<usize>, usize) {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let skills = doc
        .descendants()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let active = skills.attribute("activeSkillSet").unwrap();
    let set = skills
        .children()
        .find(|n| n.has_tag_name("SkillSet") && n.attribute("id") == Some(active))
        .unwrap();
    let groups: Vec<_> = set.children().filter(|n| n.has_tag_name("Skill")).collect();
    let matches: Vec<_> = groups
        .iter()
        .enumerate()
        .filter(|(_, n)| {
            n.attribute("source").is_none()
                && n.children()
                    .find(|c| c.has_tag_name("Gem"))
                    .is_some_and(|c| c.attribute("skillId") == Some(effect))
        })
        .collect();
    assert_eq!(matches.len(), 1);
    (matches[0].1.range(), matches[0].0 + 1)
}
pub(super) fn set_attr(text: &str, name: &str, value: &str) -> String {
    let doc = roxmltree::Document::parse(text).unwrap();
    let mut out = text.to_owned();
    if let Some(a) = doc.root_element().attributes().find(|a| a.name() == name) {
        out.replace_range(a.range(), &format!("{name}=\"{value}\""));
    } else {
        let at = text.find('>').unwrap();
        let at = if text.as_bytes()[at - 1] == b'/' {
            at - 1
        } else {
            at
        };
        out.insert_str(at, &format!(" {name}=\"{value}\""));
    }
    out
}
pub(super) fn edit_group(xml: &str, effect: &str, edit: impl FnOnce(&str) -> String) -> String {
    let (range, _) = group_range(xml, effect);
    let replacement = edit(&xml[range.clone()]);
    let mut out = xml.to_owned();
    out.replace_range(range, &replacement);
    out
}
pub(super) fn edit_gem(group: &str, effect: &str, edit: impl FnOnce(&str) -> String) -> String {
    let doc = roxmltree::Document::parse(group).unwrap();
    let gems: Vec<_> = doc
        .root_element()
        .children()
        .filter(|n| n.has_tag_name("Gem") && n.attribute("skillId") == Some(effect))
        .collect();
    assert_eq!(gems.len(), 1);
    let range = gems[0].range();
    let mut out = group.to_owned();
    out.replace_range(range.clone(), &edit(&group[range]));
    out
}
pub(super) fn template(xml: &str, effect: &str, support: &str) -> String {
    let (range, _) = group_range(xml, effect);
    let group = &xml[range];
    let doc = roxmltree::Document::parse(group).unwrap();
    let gems: Vec<_> = doc
        .root_element()
        .children()
        .filter(|n| n.has_tag_name("Gem") && n.attribute("skillId") == Some(support))
        .collect();
    assert_eq!(gems.len(), 1);
    group[gems[0].range()].to_owned()
}
pub(super) fn focus(
    xml: &str,
    effect: &str,
    child: Option<usize>,
    main: usize,
    calcs: usize,
) -> String {
    let (_, index) = group_range(xml, effect);
    let mut out = edit_group(xml, effect, |g| {
        let g = set_attr(
            &set_attr(g, "mainActiveSkill", "1"),
            "mainActiveSkillCalcs",
            "1",
        );
        edit_gem(&g, effect, |gem| {
            if let Some(child) = child {
                let mut gem = set_attr(
                    &set_attr(gem, "skillMinionSkill", &child.to_string()),
                    "skillMinionSkillCalcs",
                    &child.to_string(),
                );
                assert!(gem.trim_end().ends_with("/>"));
                let at = gem.rfind("/>").unwrap();
                gem.replace_range(at..at+2,&format!("><MinionSkillIndexLookup grantedEffect=\"{effect}\"><MinionSkillIndexMap skillIndex=\"{child}\" statSetIndex=\"{main}\"/></MinionSkillIndexLookup><MinionSkillIndexLookupCalcs grantedEffect=\"{effect}\"><MinionSkillIndexMap skillIndex=\"{child}\" statSetIndex=\"{calcs}\"/></MinionSkillIndexLookupCalcs></Gem>"));
                gem
            } else {
                // SkillsTab:376-384 resets the legacy scalar selector tables,
                // then loads these per-effect children. CalcSetup:2032-2034
                // consumes the effect-keyed maps; scalar attributes are inert.
                let mut gem = gem.to_owned();
                assert!(gem.trim_end().ends_with("/>"));
                let at = gem.rfind("/>").unwrap();
                gem.replace_range(at..at + 2, &format!("><StatSetIndex grantedEffect=\"{effect}\" index=\"{main}\"/><StatSetCalcsIndex grantedEffect=\"{effect}\" index=\"{calcs}\"/></Gem>"));
                gem
            }
        })
    });
    let doc = roxmltree::Document::parse(&out).unwrap();
    let build = doc.descendants().find(|n| n.has_tag_name("Build")).unwrap();
    let range = build.range();
    let next = set_attr(&out[range.clone()], "mainSocketGroup", &index.to_string());
    out.replace_range(range, &next);
    let doc = roxmltree::Document::parse(&out).unwrap();
    let input = doc
        .descendants()
        .find(|n| n.has_tag_name("Calcs"))
        .unwrap()
        .children()
        .find(|n| n.has_tag_name("Input") && n.attribute("name") == Some("skill_number"))
        .unwrap();
    let range = input.range();
    let next = set_attr(&out[range.clone()], "number", &index.to_string());
    out.replace_range(range, &next);
    out
}
fn controls(original: &str) -> Vec<(String, String, Json)> {
    let mut out = vec![];
    let first = template(original, SAND, I);
    let second = template(original, ICE, II);
    for child in 1..=3 {
        let set = if child <= 2 { 2 } else { 1 };
        let focused = focus(original, SAND, Some(child), 1, set);
        for (tier, winner, xml) in [
            ("i", I, focused.clone()),
            (
                "ii",
                II,
                edit_group(&focused, SAND, |g| edit_gem(g, I, |_| second.clone())),
            ),
        ] {
            out.push((format!("sand-child-{child}-tier-{tier}"),xml,
                json!({"kind":"focus","effect":SAND,"child":child,"main_set":1,"calcs_set":set,"winner":winner})));
        }
    }
    let sand = focus(original, SAND, Some(1), 1, 2);
    let ice = focus(original, ICE, None, 1, 2);
    out.push((
        "ice-tier-ii".into(),
        ice.clone(),
        json!({"kind":"focus","effect":ICE,"winner":II,"main_set":1,"calcs_set":2}),
    ));
    out.push((
        "ice-tier-i".into(),
        edit_group(&ice, ICE, |g| edit_gem(g, II, |_| first.clone())),
        json!({"kind":"focus","effect":ICE,"winner":I,"main_set":1,"calcs_set":2}),
    ));
    for (label, effect, support, focused, child) in [
        ("sand", SAND, I, &sand, Some(1)),
        ("ice", ICE, II, &ice, None),
    ] {
        for remove in [false, true] {
            let name = format!("{label}-{}", if remove { "remove" } else { "disable" });
            let changed = edit_group(focused, effect, |g| {
                edit_gem(g, support, |gem| {
                    if remove {
                        String::new()
                    } else {
                        set_attr(gem, "enabled", "false")
                    }
                })
            });
            out.push((
                name,
                changed,
                json!({"kind":"absent","effect":effect,"child":child}),
            ));
        }
    }
    for append in [false, true] {
        let changed = edit_group(&sand, SAND, |g| {
            edit_gem(g, I, |old| {
                if append {
                    format!("{old}{second}")
                } else {
                    format!("{second}{old}")
                }
            })
        });
        out.push((
            format!("family-{}", if append { "ii-last" } else { "i-last" }),
            changed,
            json!({"kind":"family","effect":SAND,"child":1,"winner":if append{II}else{I}}),
        ));
    }
    for quality in [0, 15] {
        let changed = edit_group(&sand, SAND, |g| {
            edit_gem(g, I, |old| {
                format!("{old}{}", set_attr(old, "quality", &quality.to_string()))
            })
        });
        out.push((format!("same-definition-quality-{quality}"),changed,
            json!({"kind":"quality","effect":SAND,"child":1,"winner":I,"quality":quality,"appended_wins":quality>0})));
    }
    for child in 1..=5 {
        let focused = focus(original, WATER, Some(child), 1, 1);
        let changed = edit_group(&focused, WATER, |g| {
            g.replace("</Skill>", &format!("{second}</Skill>"))
        });
        out.push((format!("water-child-{child}-tier-ii"),changed,
            json!({"kind":"focus","effect":WATER,"child":child,"main_set":1,"calcs_set":1,"winner":II})));
    }
    let water = focus(original, WATER, Some(1), 1, 1);
    out.push((
        "water-child-1-tier-i".into(),
        edit_group(&water, WATER, |g| {
            g.replace("</Skill>", &format!("{first}</Skill>"))
        }),
        json!({"kind":"focus","effect":WATER,"child":1,"main_set":1,"calcs_set":1,"winner":I}),
    ));
    for (label, root, command) in [
        ("sand", SAND, "CommandSandDjinnKnifeThrowPlayer"),
        ("water", WATER, "CommandWaterDjinnBubblePlayer"),
    ] {
        for (tier, winner, gem) in [("i", I, &first), ("ii", II, &second)] {
            let focused = focus(original, root, None, 1, 1);
            let changed = edit_group(&focused, root, |g| {
                let g = set_attr(
                    &set_attr(g, "mainActiveSkill", "2"),
                    "mainActiveSkillCalcs",
                    "2",
                );
                if root == SAND {
                    edit_gem(&g, I, |_| gem.clone())
                } else {
                    g.replace("</Skill>", &format!("{gem}</Skill>"))
                }
            });
            out.push((
                format!("{label}-command-tier-{tier}"),
                changed,
                json!({"kind":"focus","effect":command,"main_set":1,"calcs_set":1,"winner":winner}),
            ));
        }
    }
    assert_eq!(out.len(), 26);
    out
}
fn applied<'a>(q: &'a Json, channel: &str) -> Vec<&'a Json> {
    rows(&q["channels"][channel]["applied"])
        .iter()
        .filter(|r| r.get("source_effect").is_some())
        .collect()
}
fn accepted(context: &Json) -> Vec<&Json> {
    rows(&context["candidates"])
        .iter()
        .filter(|s| s["accepted"] == true)
        .collect()
}
fn check_queries(context: &Json, label: &str) {
    let q = &context["queries"];
    assert_eq!(q["cfg_effect_exact"], true, "{label}");
    assert_eq!(q["exact_stat_set"], true, "{label}");
    assert_eq!(q["original_query_methods"], true, "{label}");
    let choices = accepted(context);
    assert!(choices.len() <= 1, "{label}");
    for c in rows(&context["candidates"]) {
        assert_eq!(c["exact_definition"], true);
        assert_eq!(c["origin"]["enabled"], true);
        assert!(c["origin"]["source_ordinal"].is_u64());
    }
    let disabled = q["skill_flags"]["disable"] == true;
    let selected = choices.first().filter(|_| !disabled);
    for (channel, kind) in [
        ("AreaOfEffect", "INC"),
        ("SupportManaMultiplier", "MORE"),
        ("Damage", "MORE"),
    ] {
        let raw = rows(&q["channels"][channel]["raw_source_records"]);
        let present = selected.is_some_and(|s| channel != "Damage" || s["effect"] == II);
        assert_eq!(raw.len(), usize::from(present), "{label}/{channel}/raw");
        for (i, r) in raw.iter().enumerate() {
            assert_eq!(r["channel_index"], i + 1);
            let m = &r["record"];
            let s = selected.unwrap();
            assert_eq!(r["source_effect"], s["effect"]);
            assert_eq!(m["name"], channel);
            assert_eq!(m["type"], kind);
            let expected = match channel {
                "AreaOfEffect" => {
                    if s["effect"] == I {
                        35
                    } else {
                        45
                    }
                }
                "SupportManaMultiplier" => 30,
                "Damage" => 0,
                _ => unreachable!(),
            };
            assert_eq!(m["value"], expected, "{label}/{channel}");
            assert_eq!(m["keyword_flags"], 0);
            assert!(rows(&m["tags"]).is_empty());
            assert_eq!(
                m["flags"],
                if channel == "Damage" {
                    q["area_flag_value"].clone()
                } else {
                    json!(0)
                }
            );
        }
        let effective = applied(q, channel);
        // Original Tabulate suppresses zero-valued non-OVERRIDE records. The
        // neutral Area-flagged Damage lane is proved by its complete raw record
        // and exact cfg flags, not a fabricated Tabulate result.
        assert_eq!(
            effective.len(),
            usize::from(present && channel != "Damage"),
            "{label}/{channel}/applied"
        );
        for r in effective {
            let indices = rows(&r["source_record_indices"]);
            assert_eq!(indices.len(), 1);
            let i = indices[0].as_u64().unwrap() as usize;
            assert!(i > 0 && i <= raw.len());
            assert_eq!(r["record"], raw[i - 1]["record"]);
            assert_eq!(r["source_effect"], raw[i - 1]["source_effect"]);
            assert_eq!(r["value"], r["record"]["value"]);
        }
    }
}
fn controlled_parent<'a>(c: &'a Json, stage: &str, mode: &str) -> &'a Json {
    let found: Vec<_> = rows(&c["states"][stage]["delivery"]["contexts"])
        .iter()
        .filter(|p| {
            p["mode"] == mode
                && p["effect"] == c["control"]["effect"]
                && p["group"]["source_present"] == false
        })
        .collect();
    assert_eq!(found.len(), 1, "{}/{stage}/{mode}", c["name"]);
    found[0]
}
fn check(report: &Json) {
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 36);
    let mut selected_endpoints = BTreeSet::new();
    let mut selector_failures = Vec::new();
    for c in cases {
        for stage in STAGES {
            let d = &c["states"][stage]["delivery"];
            for field in ["original_methods_preserved", "jit_mode_preserved"] {
                assert_eq!(d[field], true);
            }
            for field in [
                "source_cfg_modified",
                "source_tables_mutated",
                "business_wrappers",
            ] {
                assert_eq!(d[field], false);
            }
            assert_eq!(d["immutable_snapshot"]["verified"], true);
            assert_eq!(rows(&d["definitions"]).len(), 2);
            for (definition, id, area) in [(0, I, 35), (1, II, 45)] {
                let def = &rows(&d["definitions"])[definition];
                assert_eq!(def["effect"], id);
                assert_eq!(def["mod_source"], format!("Skill:{id}"));
                let levels = rows(&def["levels"]["positions"]);
                assert_eq!(levels.len(), 1);
                assert_eq!(levels[0]["index"], 1);
                assert_eq!(levels[0]["value"]["manaMultiplier"], 30);
                let sets = rows(&def["stat_sets"]);
                assert_eq!(sets.len(), 1);
                let constants = rows(&sets[0]["constants"]["positions"]);
                assert_eq!(constants.len(), 2);
                let area_stat = rows(&constants[0]["value"]["positions"]);
                assert_eq!(area_stat[0]["value"], "base_skill_area_of_effect_+%");
                assert_eq!(area_stat[1]["value"], area);
                let damage_stat = rows(&constants[1]["value"]["positions"]);
                assert_eq!(
                    damage_stat[0]["value"],
                    "support_increased_area_damage_+%_final"
                );
                assert_eq!(damage_stat[1]["value"], 0);
            }
            for p in rows(&d["contexts"]) {
                assert_eq!(p["actor_is_player"], true);
                assert_eq!(p["constructor_observed"], p["effect"] != ICE);
                let label = format!(
                    "{}/{stage}/{}/{}/{}",
                    c["name"], p["mode"], p["effect"], p["group"]["index"]
                );
                check_queries(p, &label);
                for child in rows(&p["children"]) {
                    for f in ["exact_actor", "exact_summoner", "shared_support_list"] {
                        assert_eq!(child[f], true);
                    }
                    check_queries(child, &format!("{label}/{}", child["effect"]));
                    // Shared candidates retain the identical authored occurrence, but
                    // acceptance and local numerical delivery belong to each action.
                    for s in rows(&child["candidates"]) {
                        let parent: Vec<_> = rows(&p["candidates"])
                            .iter()
                            .filter(|x| x["effect"] == s["effect"])
                            .collect();
                        assert_eq!(parent.len(), 1);
                        assert_eq!(parent[0]["origin"], s["origin"]);
                    }
                }
            }
            if !c["control"].is_null() {
                for mode in ["MAIN", "CALCS"] {
                    let p = controlled_parent(c, stage, mode);
                    assert_eq!(p["selected"], true);
                    let children = rows(&p["children"]);
                    let target = if let Some(child) = c["control"]["child"].as_u64() {
                        let selected: Vec<_> =
                            children.iter().filter(|v| v["selected"] == true).collect();
                        assert_eq!(selected.len(), 1);
                        assert_eq!(selected[0]["index"], child);
                        selected[0]
                    } else {
                        p
                    };
                    assert_eq!(target["queries"]["output_available"], true);
                    assert_eq!(target["queries"]["output_is_selected_actor"], true);
                    selected_endpoints.insert((
                        target["effect"].as_str().unwrap().to_owned(),
                        target["stat_set_index"].as_u64().unwrap(),
                    ));
                    let candidates = rows(&p["candidates"]);
                    if c["control"]["kind"] == "absent" {
                        assert!(candidates.is_empty());
                    }
                    if let Some(winner) = c["control"]["winner"].as_str() {
                        assert_eq!(candidates.len(), 1);
                        assert_eq!(candidates[0]["effect"], winner);
                    }
                    if c["control"]["kind"] == "focus" {
                        let expected = &c["control"][if mode == "MAIN" {
                            "main_set"
                        } else {
                            "calcs_set"
                        }];
                        if target["stat_set_index"] != *expected {
                            selector_failures.push(format!(
                                "{}/{stage}/{mode}/{}: actual {} expected {}",
                                c["name"], target["effect"], target["stat_set_index"], expected
                            ));
                        }
                    }
                    if c["control"]["kind"] == "quality" {
                        assert_eq!(candidates[0]["quality"], c["control"]["quality"]);
                        let ordinal = candidates[0]["origin"]["position"].as_u64().unwrap();
                        // The original Magnified source is the third gem in the manual Sand group.
                        assert_eq!(
                            ordinal,
                            if c["control"]["appended_wins"] == true {
                                4
                            } else {
                                3
                            }
                        );
                    }
                }
            }
        }
    }
    // The real original keeps two independent Magnified sources; neither may be
    // copied from the other skill group's authored support occurrence.
    for stage in STAGES {
        for mode in ["MAIN", "CALCS"] {
            let original = case(report, "original-05");
            let contexts = rows(&original["states"][stage]["delivery"]["contexts"]);
            let mut origins = BTreeSet::new();
            for (effect, support) in [(SAND, I), (ICE, II)] {
                let p: Vec<_> = contexts
                    .iter()
                    .filter(|p| {
                        p["mode"] == mode
                            && p["effect"] == effect
                            && p["group"]["source_present"] == false
                    })
                    .collect();
                assert_eq!(p.len(), 1);
                let candidate = rows(&p[0]["candidates"]);
                assert_eq!(candidate.len(), 1);
                assert_eq!(candidate[0]["effect"], support);
                assert!(origins.insert(candidate[0]["origin"]["source_ordinal"].as_u64().unwrap()));
            }
        }
    }
    // Contrast original calculated values, without reimplementing rounding,
    // radius, reservation or resource cost conversion formulas.
    for stage in STAGES {
        for mode in ["MAIN", "CALCS"] {
            let with = controlled_parent(case(report, "ice-tier-ii"), stage, mode);
            let without = controlled_parent(case(report, "ice-remove"), stage, mode);
            assert!(
                with["queries"]["output"]["AreaOfEffectMod"]
                    .as_f64()
                    .unwrap()
                    > without["queries"]["output"]["AreaOfEffectMod"]
                        .as_f64()
                        .unwrap()
            );
            assert!(
                with["queries"]["output"]["ManaCost"].as_f64().unwrap()
                    > without["queries"]["output"]["ManaCost"].as_f64().unwrap()
            );
            let disabled = controlled_parent(case(report, "ice-disable"), stage, mode);
            assert_eq!(disabled["queries"]["output"], without["queries"]["output"]);
        }
    }
    eprintln!(
        "Magnified content/contribution/identity assertions passed; checking exact selected endpoint coverage"
    );
    assert!(
        selector_failures.is_empty(),
        "controlled source selectors failed: {selector_failures:?}"
    );
    assert_eq!(
        selected_endpoints.len(),
        14,
        "five Sand sets, five Water sets, two player Commands and two Ice Nova sets execute: {selected_endpoints:?}"
    );
}
