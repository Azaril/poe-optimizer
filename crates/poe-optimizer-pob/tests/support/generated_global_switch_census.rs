//! Optional source-only companion; catalog scope is not whole-build authority.
use super::*;

const TEST: &str = "generated_global_switch_census::complete_catalog_global_switch_census";
const CHILD: &str = "POE_GENERATED_GLOBAL_CENSUS_CHILD";
const OUTPUT: &str = "POE_GENERATED_GLOBAL_CENSUS_OUT";
const COLLECTOR: &str = include_str!("generated_global_switch_census.lua");
const MAX_WORK: u64 = 2_000_000;
const CASES: [&str; 4] = [
    "original-05",
    "manual-firebolt-with-staff",
    "activate-preset-2",
    "repeat-original-05",
];

fn requests(root: &Path) -> (Json, String) {
    let bytes =
        fs::read(root.join("data/owned/poe2/3887ae68/import/skill-identities.json")).unwrap();
    let catalog: Json = serde_json::from_slice(&bytes).unwrap();
    let mut requests = Vec::new();
    for (game, variant) in [
        (
            "Metadata/Items/Gem/SkillGemAscendancySummonSandDjinn",
            "AscendancySummonSandDjinn",
        ),
        (
            "Metadata/Items/Gem/SkillGemAscendancySummonWaterDjinn",
            "AscendancySummonWaterDjinn",
        ),
        ("Metadata/Items/Gems/SkillGemFirebolt", "Firebolt"),
        ("Metadata/Items/Gems/SkillGemFrostBomb", "FrostBomb"),
    ] {
        let matches: Vec<_> = rows(&catalog["gems"])
            .iter()
            .filter(|g| g["game_id"] == game && g["variant_id"] == variant)
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "exact requested catalog tuple {game}/{variant}"
        );
        requests.push(
            json!({"game_id":game,"variant_id":variant,"effect_list":matches[0]["effect_list"]}),
        );
    }
    (json!(requests), digest(&bytes))
}

fn collect(lua: &Lua, requests: &Json) -> Result<Json, RuntimeError> {
    let census: Function = lua
        .load(COLLECTOR)
        .set_name("@bounded-source-global-switch-census")
        .eval()?;
    let data: Value = lua.globals().get("data")?;
    let value: Value = census.call((data, lua.to_value(requests)?, MAX_WORK))?;
    Ok(lua.from_value(value)?)
}

#[test]
#[ignore = "requires complete pinned PoB; catalog census never certifies whole-build global-switch non-applicability"]
fn complete_catalog_global_switch_census() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join(
        std::env::var_os(OUTPUT)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("runs/owned-generated-global-switch-census-02")),
    );
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        run_child(&root, &out, mode == "on");
        return;
    }
    assert!(
        !out.exists(),
        "choose a fresh {OUTPUT} path; existing source evidence is immutable: {}",
        out.display()
    );
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env(CHILD, mode)
            .env(OUTPUT, &out)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "{}\n{}", path.display(), tail(&path));
                break;
            }
            if started.elapsed() > Duration::from_secs(300) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "global census deadline: {}\n{}",
                    path.display(),
                    tail(&path)
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert!(
        fs::read(out.join("source-jit-off.json")).unwrap()
            == fs::read(out.join("source-jit-on.json")).unwrap(),
        "source census differs across JIT modes"
    );
}

fn run_child(root: &Path, out: &Path, enabled: bool) {
    let (requested, catalog) = requests(root);
    let original =
        fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let manual = super::controls(&original)
        .into_iter()
        .find(|(name, _)| *name == "manual-firebolt-with-staff")
        .unwrap()
        .1;
    let doc = roxmltree::Document::parse(&original).unwrap();
    let skills = doc
        .descendants()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let dormant = change_attributes(&original, skills, &[("activeSkillSet", Some("2"))]);
    let stage = |lua: &Lua| -> Result<Json, RuntimeError> {
        let before = super::observe_stage(lua)?;
        let first = collect(lua, &requested)?;
        let second = collect(lua, &requested)?;
        let after = super::observe_stage(lua)?;
        assert!(
            first == second,
            "raw census must be repeatable without warming maps"
        );
        assert!(before == after, "census changed original state or output");
        let mut result = before;
        result["global_switch_census"] = first;
        Ok(result)
    };
    let cases: Vec<_> = CASES
        .into_iter()
        .zip([&original, &manual, &dormant, &original])
        .map(|(name, xml)| {
            let cold = std::cell::RefCell::new(None);
            let data_ready = |lua: &Lua| -> Result<(), RuntimeError> {
                let first = collect(lua, &requested)?;
                assert!(
                    first == collect(lua, &requested)?,
                    "cold census must not initialize metadata"
                );
                *cold.borrow_mut() = Some(first);
                Ok(())
            };
            let mut case = observe_case_with_stage_and_data_hook(
                root,
                name,
                xml,
                enabled,
                &stage,
                Some(&data_ready),
            );
            case["before_build_census"] = cold.into_inner().unwrap();
            case
        })
        .collect();
    let files = [
        "src/HeadlessWrapper.lua",
        "src/Modules/Build.lua",
        "src/Classes/CalcsTab.lua",
        "src/Classes/SkillsTab.lua",
        "src/Modules/Data.lua",
        "src/Data/Gems.lua",
        "src/Data/Skills/act_int.lua",
        "src/Data/Skills/other.lua",
        "src/Data/SkillStatMap.lua",
        "src/Modules/CalcTools.lua",
        "src/Modules/CalcActiveSkill.lua",
        "src/Modules/CalcSetup.lua",
        "src/Modules/CalcDefence.lua",
        "src/Modules/Calcs.lua",
        "src/Modules/CalcPerform.lua",
        "src/Modules/BuildExportPoE2.lua",
        "src/Modules/ModParser.lua",
        "src/Data/ModCache.lua",
        "src/Classes/ModList.lua",
        "src/Classes/ModStore.lua",
    ];
    let report = json!({
        "schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,
        "manifest_sha256":pinned::manifest_sha256(),"catalog_sha256":catalog,
        "collector_sha256":digest(COLLECTOR.as_bytes()),"requests":requested,
        "business_wrappers":false,"observer_warmed_source_metadata":false,
        "native_inventory_authority":false,"calculation_non_applicability_proved":false,
        "extra_stats_scope":"unproved",
        "consumer_scope":["CalcSetup indexed active-effect filter","CalcDefence Vaal count branch",
            "CalcActiveSkill active-stat-set root lookup and mergeStatSet","CalcTools declared stat generation",
            "SkillsTab selection/filter","BuildExportPoE2 filter"],
        "outside_claim":["copy/persistence/UI intent","external ExtraSkillStat producer closure",
            "native activation","reporting aggregation","all-build or future-data inertness"],
        "files":files.map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})),
        "cases":cases,
    });
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(bytes.len() <= 64 * 1024 * 1024);
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        bytes,
    )
    .unwrap();
    check(&report);
}

fn check(report: &Json) {
    assert_eq!(rows(&report["cases"]).len(), CASES.len());
    assert!(case(report, "original-05")["states"] == case(report, "repeat-original-05")["states"]);
    for c in rows(&report["cases"]) {
        let cold = &c["before_build_census"]["requests"][3];
        assert_eq!(
            cold["effects"][0]["has_global_before"], false,
            "cold Frost metadata must expose the late-map control"
        );
        assert_eq!(cold["declared_scope_status"], "refused");
        assert!(!rows(&cold["global_tag_paths"]).is_empty());
        assert!(rows(&cold["effects"][0]["parts"]).iter().any(|part| {
            rows(&part["maps"]).iter().any(|map| {
                map["origin"] == "global_raw" && map["value"].to_string().contains("GlobalEffect")
            })
        }));
        for stage in STAGES {
            let census = &c["states"][stage]["global_switch_census"];
            assert_eq!(census["calculation_non_applicability_proved"], false);
            assert_eq!(census["unresolved"], json!(["extra_stat_scope_unproved"]));
            assert_eq!(rows(&census["requests"]).len(), 4);
            assert!(census["work"].as_u64().unwrap() <= MAX_WORK);
            for (index, r) in rows(&census["requests"]).iter().enumerate() {
                assert_eq!(r["calculation_non_applicability_proved"], false);
                assert_eq!(r["vaal"], false);
                let effects: Vec<_> = rows(&r["effects"])
                    .iter()
                    .map(|e| e["id"].clone())
                    .collect();
                assert_eq!(json!(effects), report["requests"][index]["effect_list"]);
                if index < 3 {
                    assert_eq!(
                        r["declared_scope_status"], "complete_without_global_tag",
                        "{}/{stage}: {r}",
                        c["name"]
                    );
                } else {
                    assert_eq!(r["declared_scope_status"], "refused");
                    assert!(
                        !rows(&r["global_tag_paths"]).is_empty(),
                        "Frost must expose its latent global tag"
                    );
                }
                for effect in rows(&r["effects"]) {
                    assert_eq!(effect["has_global_before"], effect["has_global_after"]);
                }
            }
        }
    }
    // Complete source ownership independently records selected and dormant
    // Tree/Item groups; manual Firebolt must remain a separate authored source.
    let base = &case(report, "original-05")["states"]["fresh"];
    assert_eq!(base["selection"]["skills"], 4);
    assert!(
        rows(&base["runtime_groups"])
            .iter()
            .any(|g| g["selected"] == false)
    );
    for (source, effect, owner_key) in [
        ("Tree:13289", "SummonSandDjinnPlayer", "source_node_id"),
        ("Tree:32705", "SummonWaterDjinnPlayer", "source_node_id"),
        (STAFF_SOURCE, FIREBOLT, "source_item_id"),
    ] {
        let groups: Vec<_> = rows(&base["runtime_groups"])
            .iter()
            .filter(|g| g["selected"] == true && g["state"]["fields"]["source"] == source)
            .collect();
        assert_eq!(groups.len(), 1);
        assert!(groups[0]["state"][owner_key].as_u64().is_some());
        assert!(
            rows(&groups[0]["gems"])
                .iter()
                .any(|g| g["state"]["fields"]["skillId"] == effect)
        );
    }
    let manual = &case(report, "manual-firebolt-with-staff")["states"]["fresh"];
    assert!(rows(&manual["saved_groups"]).iter().any(|g| {
        g["attributes"]["source"].is_null()
            && rows(&g["gems"])
                .iter()
                .any(|gem| gem["source"]["attributes"]["skillId"] == FIREBOLT)
    }));
    assert!(rows(&manual["runtime_groups"]).iter().any(|g| {
        g["selected"] == true
            && g["state"]["fields"]["source"] == STAFF_SOURCE
            && rows(&g["gems"])
                .iter()
                .any(|gem| gem["state"]["fields"]["skillId"] == FIREBOLT)
    }));
    assert_eq!(
        case(report, "activate-preset-2")["states"]["fresh"]["selection"]["skills"],
        2
    );
}

#[test]
fn finite_census_refuses_late_global_vaal_shape_and_budget_gaps() {
    let lua = Lua::new();
    let collector: Function = lua.load(COLLECTOR).eval().unwrap();
    lua.globals().set("collector", collector).unwrap();
    let result:Value=lua.load(r#"
local meta={__index=function() error("census must not trigger lazy map") end}
local effect={id="effect",statSets={},qualityStats={{"late",1}},levels={[1]={}}}
effect.statMap=setmetatable({_grantedEffect=effect},meta)
local gem={gameId="game",variantId="variant",grantedEffectList={effect}}
local data={gemsByGameId={game={variant=gem}},skills={effect=effect},skillStatMap={},skillStatMapMeta=meta}
local request={{game_id="game",variant_id="variant",effect_list={"effect"}}}
local clean=collector(data,request,10000)
data.skillStatMap.late={{name="x",type="BASE",value=1,{type="GlobalEffect"}}}
local late=collector(data,request,10000)
assert(effect.hasGlobalEffect==nil and rawget(effect.statMap,"late")==nil)
data.skillStatMap.late=nil;gem.vaalGem=true
local vaal=collector(data,request,10000)
gem.vaalGem=false;effect.qualityStats={[2]={"late",1}}
local shape=collector(data,request,10000)
effect.qualityStats={{"late",1}};data.skillStatMap.late=function() return {} end
local dynamic=collector(data,request,10000)
data.skillStatMap.late=nil;gem.grantedEffectList[2]=effect
local extra=collector(data,request,10000)
gem.grantedEffectList[2]=nil
local hidden={__index={baseMods={{name="hidden",type="BASE",value=1,{type="GlobalEffect"}}}}}
setmetatable(effect,hidden)
local effect_meta=collector(data,request,10000)
setmetatable(effect,nil)
local part={levels={},stats={}}
part.statMap=setmetatable({_grantedEffect=effect},meta)
effect.statSets={setmetatable(part,hidden)}
local part_meta=collector(data,request,10000)
effect.statSets={}
local container_meta={}
for i,target in ipairs({data,data.gemsByGameId,data.gemsByGameId.game,data.skills,gem,data.skillStatMap}) do
 setmetatable(target,hidden)
 container_meta[i]=collector(data,request,10000)
 setmetatable(target,nil)
end
meta.__newindex=function() error("unreviewed map mutation") end
local authority_extra=collector(data,request,10000)
meta.__newindex=nil
setmetatable(meta,hidden)
local authority_meta=collector(data,request,10000)
setmetatable(meta,nil)
setmetatable(effect.statMap,{__index=meta.__index})
local map_identity=collector(data,request,10000)
setmetatable(effect.statMap,meta)
effect.qualityStats={}
local overridden_set={levels={},stats={"late"}}
overridden_set.statMap=setmetatable({_grantedEffect=effect,late={}},meta)
effect.statSets={overridden_set}
data.skillStatMap.late={{name="hidden_root",type="BASE",value=1,{type="GlobalEffect"}}}
local root_lookup=collector(data,request,10000)
assert(rawget(effect.statMap,"late")==nil and effect.hasGlobalEffect==nil)
local budget=pcall(collector,data,request,1)
return {clean=clean,late=late,vaal=vaal,shape=shape,dynamic=dynamic,extra=extra,budget=budget,
 effect_meta=effect_meta,part_meta=part_meta,container_meta=container_meta,
 authority_extra=authority_extra,authority_meta=authority_meta,map_identity=map_identity,root_lookup=root_lookup}
"#).eval().unwrap();
    let result: Json = lua.from_value(result).unwrap();
    assert_eq!(
        result["clean"]["requests"][0]["declared_scope_status"],
        "complete_without_global_tag"
    );
    assert_eq!(
        result["clean"]["calculation_non_applicability_proved"],
        false
    );
    for name in [
        "late",
        "vaal",
        "shape",
        "dynamic",
        "extra",
        "effect_meta",
        "part_meta",
        "authority_extra",
        "authority_meta",
        "map_identity",
        "root_lookup",
    ] {
        assert_eq!(
            result[name]["requests"][0]["declared_scope_status"], "refused",
            "{name}"
        );
    }
    assert_eq!(rows(&result["container_meta"]).len(), 6);
    for census in rows(&result["container_meta"]) {
        assert_eq!(census["requests"][0]["declared_scope_status"], "refused");
        assert!(
            rows(&census["requests"][0]["issues"])
                .iter()
                .any(|issue| issue["code"] == "unreviewed-catalog-container")
        );
    }
    let root_effect = &result["root_lookup"]["requests"][0]["effects"][0];
    assert_eq!(root_effect["has_global_before"], false);
    assert_eq!(root_effect["has_global_after"], false);
    assert!(
        rows(&root_effect["parts"][0]["maps"])
            .iter()
            .any(|map| map["stat"] == "late"
                && map["origin"] == "global_raw"
                && map["lookup"] == "active_stat_set_through_root")
    );
    assert!(
        rows(&result["root_lookup"]["requests"][0]["global_tag_paths"])
            .iter()
            .any(|path| path
                .as_str()
                .unwrap()
                .starts_with("effects/1/parts/1/statMap/late/"))
    );
    assert_eq!(result["budget"], false);
}
