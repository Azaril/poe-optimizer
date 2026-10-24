//! Shared Player intrinsic-source proof. Foreign modifier stores stay separate.
use flate2::read::GzDecoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::PathBuf,
    sync::OnceLock,
};

const BASE: &str = "data/owned/poe2/3887ae68/";
const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const MANIFEST: &str = "crates/poe-optimizer-pob/data/pob-source-manifest.json";
const MANIFEST_HASH: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const EVIDENCE_HASH: &str = "79b539481d3e8e27845a2f48bc8d58e115ed96e0092fc711fe81bc703bd30193";
const FILES: &[(&str, &str)] = &[
    (
        "src/Modules/CalcSetup.lua",
        "support-structural-inventories/sources/src_Modules_CalcSetup.lua.gz",
    ),
    (
        "src/Modules/Data.lua",
        "support-structural-inventories/sources/src_Modules_Data.lua.gz",
    ),
    (
        "src/Data/Misc.lua",
        "configuration-enemy-critical-damage-inputs/Misc.lua.gz",
    ),
    (
        "src/Classes/ModStore.lua",
        "configuration-distance-inputs/ModStore.lua.gz",
    ),
    (
        "src/Modules/ModTools.lua",
        "player-support-domains/ModTools.lua.gz",
    ),
    ("src/Classes/ModDB.lua", "life-base-domain/ModDB.lua.gz"),
    (
        "src/Modules/ModParser.lua",
        "support-structural-inventories/sources/src_Modules_ModParser.lua.gz",
    ),
    (
        "src/Modules/CalcActiveSkill.lua",
        "support-structural-inventories/sources/src_Modules_CalcActiveSkill.lua.gz",
    ),
    (
        "src/Modules/CalcPerform.lua",
        "configuration-distance-inputs/CalcPerform.lua.gz",
    ),
    (
        "src/Classes/ConfigTab.lua",
        "configuration-campaign-progression/ConfigTab.lua.gz",
    ),
];
const SPANS: &[(&str, &str, usize, usize)] = &[
    ("common-initializer", "src/Modules/CalcSetup.lua", 26, 114),
    ("player-initializer", "src/Modules/CalcSetup.lua", 829, 899),
    (
        "environment-constructor",
        "src/Modules/CalcSetup.lua",
        717,
        2309,
    ),
    ("actor-construction", "src/Modules/CalcSetup.lua", 717, 790),
    (
        "class-and-life-boundary",
        "src/Modules/CalcSetup.lua",
        828,
        837,
    ),
    (
        "foreign-config-party-cache",
        "src/Modules/CalcSetup.lua",
        902,
        919,
    ),
    (
        "foreign-item-merge",
        "src/Modules/CalcSetup.lua",
        1604,
        1605,
    ),
    (
        "foreign-tree-merge",
        "src/Modules/CalcSetup.lua",
        1635,
        1638,
    ),
    ("foreign-weapons", "src/Modules/CalcSetup.lua", 1852, 1886),
    (
        "external-support-consumers",
        "src/Modules/CalcSetup.lua",
        1898,
        1955,
    ),
    (
        "linked-support-consumers",
        "src/Modules/CalcSetup.lua",
        2078,
        2097,
    ),
    (
        "foreign-group-property",
        "src/Modules/CalcSetup.lua",
        2128,
        2139,
    ),
    ("ui-fallback-skill", "src/Modules/CalcSetup.lua", 2196, 2209),
    ("modstore-constructor", "src/Classes/ModStore.lua", 65, 71),
    ("newmod-wrapper", "src/Classes/ModStore.lua", 142, 144),
    ("newmod-alias", "src/Classes/ModStore.lua", 18, 18),
    ("create-modifier", "src/Modules/ModTools.lua", 57, 91),
    ("moddb-constructor", "src/Classes/ModDB.lua", 25, 29),
    ("moddb-add", "src/Classes/ModDB.lua", 31, 37),
    ("moddb-add-list", "src/Classes/ModDB.lua", 113, 122),
    (
        "active-skill-constructor",
        "src/Modules/CalcActiveSkill.lua",
        144,
        234,
    ),
    ("misc-data-import", "src/Modules/Data.lua", 112, 118),
    ("character-constants", "src/Data/Misc.lua", 144, 245),
    (
        "extra-support-contrast",
        "src/Modules/ModParser.lua",
        2219,
        2253,
    ),
    (
        "linked-support-contrast",
        "src/Modules/ModParser.lua",
        3650,
        3659,
    ),
    (
        "foreign-custom-modifiers",
        "src/Classes/ConfigTab.lua",
        1200,
        1241,
    ),
    (
        "foreign-disabled-weapons",
        "src/Modules/CalcPerform.lua",
        3224,
        3237,
    ),
];
const ARTIFACTS: &[(&str, usize, &str)] = &[
    (
        "player-rule-ownership/ownership.json",
        230984,
        "ce910ff446ff33b60fc37869e42d3c4393d86d34a38c32163e093b26441279b9",
    ),
    (
        "player-rule-ownership/bindings.json",
        3305,
        "fae0bed9195c839f76581d4582db3619a35c302ce98aa9b86ee7798c443168ed",
    ),
    (
        "player-rule-ownership/source-vectors.json",
        3914,
        "123f13f27de1c4c9ce48e09f1079614cd9da720f97a6a69b9c2a6c80aa7c3ef7",
    ),
    (
        "player-intrinsic-life/source-vectors.json",
        237312,
        "4bddcee4321c60a3aa296b6fcc936c79f5407db885ffa3750aeda330262d9b4e",
    ),
    (
        "player-intrinsic-life/bindings.json",
        3334,
        "8fb3bf578750aaadd5f99e465d07ce1fc8a313809e9803a559858f21353a9460",
    ),
];
struct Facts {
    evidence: Value,
    inputs: BTreeMap<String, Value>,
    spans: BTreeMap<String, String>,
    manifest: Value,
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn text(value: &Value) -> &str {
    value.as_str().unwrap()
}
fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|o| o.is_empty()));
        &[]
    }
}
fn bounded(path: &str, limit: usize) -> Vec<u8> {
    let file = File::open(root().join(path)).unwrap();
    assert!(file.metadata().unwrap().len() <= limit as u64);
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes).unwrap();
    assert!(bytes.len() <= limit);
    bytes
}
fn pin(bytes: &[u8], record: &Value) {
    assert_eq!(record["bytes"], bytes.len());
    assert_eq!(record["sha256"], hash(bytes));
}
fn fields(value: &Value, expected: &[&str]) {
    assert_eq!(
        value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        expected.iter().copied().collect()
    );
}
fn exact<'a>(value: &'a Value, key: &str, expected: &Value) -> &'a Value {
    let found: Vec<_> = rows(value).iter().filter(|r| r[key] == *expected).collect();
    assert_eq!(found.len(), 1);
    found[0]
}
fn authenticated_span(source: &str, pin: &Value, expected: (&str, &str, usize, usize)) -> String {
    let (role, path, first, last) = expected;
    assert_eq!(pin["role"], role);
    assert_eq!(pin["path"], path);
    assert_eq!(pin["first"], first);
    assert_eq!(pin["last"], last);
    let lines: Vec<_> = source.lines().collect();
    assert!(first > 0 && last >= first && last <= lines.len() && last - first < 2000);
    let body = lines[first - 1..last].join("\n") + "\n";
    assert_eq!(pin["sha256"], hash(body.as_bytes()));
    body
}
fn facts() -> &'static Facts {
    static FACTS: OnceLock<Facts> = OnceLock::new();
    FACTS.get_or_init(|| {
        let bytes = bounded(
            &format!("{BASE}player-support-domains/evidence.json"),
            65536,
        );
        assert_eq!(hash(&bytes), EVIDENCE_HASH);
        assert!(!bytes.contains(&b'\r'));
        let evidence: Value = serde_json::from_slice(&bytes).unwrap();
        fields(
            &evidence,
            &[
                "schema_version",
                "source_revision",
                "source_manifest",
                "scope",
                "files",
                "spans",
                "artifacts",
                "intrinsic_modifiers",
            ],
        );
        assert_eq!(evidence["schema_version"], 1);
        assert_eq!(evidence["source_revision"], REVISION);
        assert!(!text(&evidence["scope"]).is_empty());
        assert_eq!(evidence["source_manifest"]["path"], MANIFEST);
        assert_eq!(evidence["source_manifest"]["sha256"], MANIFEST_HASH);
        let bytes = bounded(MANIFEST, 1024 * 1024);
        pin(&bytes, &evidence["source_manifest"]);
        let manifest: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(manifest["upstream_revision"], REVISION);
        let mut sources = BTreeMap::new();
        let mut total = 0;
        assert_eq!(rows(&evidence["files"]).len(), FILES.len());
        for (record, (path, archive)) in rows(&evidence["files"]).iter().zip(FILES) {
            assert_eq!(record["path"], *path);
            assert_eq!(record["archive"], format!("{BASE}{archive}"));
            let compressed = bounded(text(&record["archive"]), 1024 * 1024);
            assert_eq!(record["archive_bytes"], compressed.len());
            assert_eq!(record["archive_sha256"], hash(&compressed));
            let mut bytes = Vec::new();
            GzDecoder::new(compressed.as_slice())
                .take(4 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .unwrap();
            total += bytes.len();
            assert!(bytes.len() <= 4 * 1024 * 1024 && total <= 8 * 1024 * 1024);
            pin(&bytes, record);
            pin(&bytes, exact(&manifest["files"], "path", &json!(path)));
            assert!(!bytes.contains(&b'\r'));
            assert!(
                sources
                    .insert(*path, String::from_utf8(bytes).unwrap())
                    .is_none()
            );
        }
        assert_eq!(rows(&evidence["spans"]).len(), SPANS.len());
        let spans: BTreeMap<_, _> = rows(&evidence["spans"])
            .iter()
            .zip(SPANS)
            .map(|(record, expected)| {
                (
                    expected.0.to_owned(),
                    authenticated_span(&sources[expected.1], record, *expected),
                )
            })
            .collect();
        assert_eq!(spans.len(), SPANS.len());
        assert_eq!(rows(&evidence["artifacts"]).len(), ARTIFACTS.len());
        let mut inputs: BTreeMap<String, Value> = BTreeMap::new();
        for (record, (path, count, digest)) in rows(&evidence["artifacts"]).iter().zip(ARTIFACTS) {
            assert_eq!(record["path"], format!("{BASE}{path}"));
            assert_eq!(record["bytes"], *count);
            assert_eq!(record["sha256"], *digest);
            let bytes = bounded(text(&record["path"]), 1024 * 1024);
            pin(&bytes, record);
            assert!(
                inputs
                    .insert((*path).to_owned(), serde_json::from_slice(&bytes).unwrap())
                    .is_none()
            );
        }
        let f = Facts {
            evidence,
            inputs,
            spans,
            manifest,
        };
        check_intrinsics(&f.evidence["intrinsic_modifiers"], &f.spans);
        check_wrappers_and_boundaries(&f.spans);
        check_ownership(&f.inputs);
        check_observed_vectors(&f);
        f
    })
}
pub(super) fn verified_ownership() -> &'static Value {
    &facts().inputs["player-rule-ownership/ownership.json"]
}
pub(super) fn verified_bindings() -> &'static Value {
    &facts().inputs["player-rule-ownership/bindings.json"]
}
pub(super) fn verified_vectors() -> &'static Value {
    &facts().inputs["player-rule-ownership/source-vectors.json"]
}
pub(super) fn verified_life_vectors() -> &'static Value {
    &facts().inputs["player-intrinsic-life/source-vectors.json"]
}
pub(super) fn verified_life_bindings() -> &'static Value {
    &facts().inputs["player-intrinsic-life/bindings.json"]
}

/// Whole pinned constructor output names, independent of native conversion.
pub(super) fn verified_intrinsic_modifiers() -> &'static Value {
    &facts().evidence["intrinsic_modifiers"]
}
pub(super) fn check_intrinsic_modifiers(census: &Value) {
    let f = facts();
    check_intrinsics(census, &f.spans);
    check_wrappers_and_boundaries(&f.spans);
}

/// Check the finite reviewed call prefixes, not arbitrary Lua syntax. Complete
/// source/span hashes above authenticate all arguments, control flow and bodies.
fn check_intrinsics(census: &Value, spans: &BTreeMap<String, String>) {
    assert_eq!(rows(census).len(), 148);
    let mut seen = BTreeSet::new();
    let mut totals = BTreeMap::new();
    for (role, first, count, kinds) in [
        ("common-initializer", 26, 84, [53, 20, 9, 2]),
        ("player-initializer", 829, 64, [46, 4, 11, 3]),
    ] {
        let body = &spans[role];
        let mut found = Vec::new();
        let mut classes = 0;
        for (offset, line) in body.lines().enumerate() {
            let Some(call) = line.trim_start().strip_prefix("modDB:NewMod(") else {
                continue;
            };
            let number = first + offset;
            if number == 832 {
                assert_eq!(
                    call,
                    "stat, \"BASE\", classStats[\"base_\"..stat:lower()], \"Base\")"
                );
                classes += 1;
                continue;
            }
            let row = exact(census, "line", &json!(number));
            fields(row, &["line", "name", "kind"]);
            let kind = text(&row["kind"]);
            assert!(["BASE", "FLAG", "INC", "MORE"].contains(&kind));
            assert!(!["ExtraSupport", "LinkedSupport"].contains(&text(&row["name"])));
            assert!(call.starts_with(&format!("\"{}\", \"{kind}\",", text(&row["name"]))));
            assert!(seen.insert(number));
            found.push(kind);
            *totals.entry(kind.to_owned()).or_insert(0) += 1;
        }
        assert_eq!(found.len(), count);
        assert_eq!(classes, usize::from(role == "player-initializer"));
        for (kind, count) in ["BASE", "FLAG", "INC", "MORE"].into_iter().zip(kinds) {
            assert_eq!(
                found.iter().filter(|actual| **actual == kind).count(),
                count
            );
        }
    }
    assert_eq!(seen.len(), rows(census).len());
    assert_eq!(
        totals,
        BTreeMap::from([
            ("BASE".into(), 99),
            ("FLAG".into(), 24),
            ("INC".into(), 20),
            ("MORE".into(), 5)
        ])
    );
    assert!(spans["common-initializer"].ends_with("\tmodDB.conditions[\"Buffed\"] = env.mode_buffs\n\tmodDB.conditions[\"Combat\"] = env.mode_combat\n\tmodDB.conditions[\"Effective\"] = env.mode_effective\nend\n"));
    let player = &spans["player-initializer"];
    assert!(player.starts_with("\tif not cachedPlayerDB then\n"));
    assert!(player.contains("for _, stat in pairs({\"Str\",\"Dex\",\"Int\"}) do"));
    assert!(
        player
            .contains("modDB.multipliers[\"Level\"] = m_max(1, m_min(100, build.characterLevel))")
    );
    assert!(player.contains("calcs.initModDB(env, modDB)"));
    assert!(player.contains("env.configInput.resistancePenalty or -60"));
}
fn check_wrappers_and_boundaries(s: &BTreeMap<String, String>) {
    for (role, opening) in [
        ("common-initializer", "function calcs.initModDB("),
        ("environment-constructor", "function calcs.initEnv("),
        ("modstore-constructor", "function ModStoreClass:ModStore("),
        ("newmod-wrapper", "function ModStoreClass:NewMod("),
        ("create-modifier", "function modLib.createMod("),
        ("moddb-constructor", "function ModDBClass:ModDB("),
        ("moddb-add", "function ModDBClass:AddMod("),
        ("moddb-add-list", "function ModDBClass:AddList("),
        (
            "active-skill-constructor",
            "function calcs.createActiveSkill(",
        ),
    ] {
        assert!(s[role].starts_with(opening) && s[role].ends_with("end\n"));
    }
    for (role, required) in [
        ("newmod-alias", "local mod_createMod = modLib.createMod"),
        ("newmod-wrapper", "self:AddMod(mod_createMod(...))"),
        (
            "create-modifier",
            "name = modName,\n\t\ttype = modType,\n\t\tvalue = modVal,",
        ),
        ("modstore-constructor", "self.parent = parent or false"),
        (
            "modstore-constructor",
            "self.multipliers = { }\n\tself.conditions = { }",
        ),
        (
            "moddb-constructor",
            "self:ModStore(parent)\n\tself.mods = { }",
        ),
        ("moddb-add", "local name = mod.name"),
        ("moddb-add", "t_insert(self.mods[name], mod)"),
        ("moddb-add-list", "t_insert(mods[name], mod)"),
        ("actor-construction", "modDB = new(\"ModDB\"):ModDB()"),
        (
            "actor-construction",
            "env.player = {\n\t\t\tmodDB = env.modDB,",
        ),
        ("actor-construction", "env.modDB.actor.player = env.player"),
        (
            "foreign-config-party-cache",
            "env.modDB:AddList(build.configTab.modList)",
        ),
        (
            "foreign-config-party-cache",
            "env.enemyDB:AddList(build.partyTab.enemyModList)",
        ),
        (
            "foreign-config-party-cache",
            "env.modDB.parent = cachedPlayerDB",
        ),
        ("foreign-item-merge", "mergeDB(env.modDB, env.itemModDB)"),
        (
            "foreign-tree-merge",
            "calcs.buildModListForNodeList(env, env.allocNodes, true)",
        ),
        (
            "foreign-group-property",
            "env.player.modDB:AddMod(modLib.setSource(value.value, groupCfg.slotName or \"\"))",
        ),
        ("foreign-weapons", "UseFacebreakerItemDamage"),
        (
            "foreign-disabled-weapons",
            "modDB:AddList(env.weaponModList1)",
        ),
        ("extra-support-contrast", "mod(\"ExtraSupport\", \"LIST\""),
        ("linked-support-contrast", "mod(\"LinkedSupport\", \"LIST\""),
        (
            "external-support-consumers",
            "env.modDB:Tabulate(\"LIST\", nil, \"LinkedSupport\")",
        ),
        (
            "external-support-consumers",
            "env.modDB:List(groupCfg, \"ExtraSupport\")",
        ),
        (
            "linked-support-consumers",
            "addBestSupport(supportEffect, appliedSupportList, env.mode)",
        ),
        (
            "foreign-custom-modifiers",
            "block.enabled ~= false and block.text and #block.text > 0",
        ),
        ("foreign-custom-modifiers", "modLib.parseMod(strippedLine)"),
        ("foreign-custom-modifiers", "modList:AddMod(mod)"),
        (
            "foreign-custom-modifiers",
            "if not hasBlockText and input.customMods and #input.customMods > 0 then",
        ),
        ("ui-fallback-skill", "if not env.player.mainSkill then"),
        (
            "ui-fallback-skill",
            "grantedEffect = env.data.skills.MeleeUnarmedPlayer",
        ),
        (
            "ui-fallback-skill",
            "calcs.createActiveSkill(defaultEffect, { }, env, env.player)",
        ),
        ("active-skill-constructor", "supportList = supportList,"),
        (
            "active-skill-constructor",
            "for _, supportEffect in ipairs(supportList) do",
        ),
        (
            "misc-data-import",
            "local miscData = LoadModule(\"Data/Misc\")",
        ),
        ("character-constants", "[\"life_per_level\"] = 12,"),
    ] {
        assert!(s[role].contains(required), "{role}: {required}");
    }
}
fn check_ownership(inputs: &BTreeMap<String, Value>) {
    let o = &inputs["player-rule-ownership/ownership.json"];
    let b = &inputs["player-rule-ownership/bindings.json"];
    let life = &inputs["player-intrinsic-life/bindings.json"];
    assert_eq!(b["actor"]["key"], "def.000000000000332a");
    assert_eq!(b["program"], "intrinsic-player-life");
    for field in ["classes", "life", "unit"] {
        assert_eq!(b[field], life[field]);
    }
    assert_eq!(life["per_level"], 12);
    assert_eq!(life["offset"], 16);
    assert_eq!(rows(&o["classes"]).len(), 8);
    assert_eq!(rows(&o["actor"]["programs"]["members"]).len(), 1);
    assert_eq!(
        o["actor"]["owner"],
        json!({"kind":"definition","value":{"kind":"actor","value":b["actor"]}})
    );
    assert_eq!(
        o["actor"]["programs"]["closure"],
        json!({"kind":"partial","value":{"gaps":[{"subject":o["actor"]["owner"],"facet":"game_rules","code":"shared-player-initialization-not-converted"}]}})
    );
    assert_eq!(
        o["applicability"],
        json!({"members":[{"id":b["application"],"owner":b["actor"],"targets":["player"]}],"closure":{"kind":"complete"}})
    );
    for (row, class) in rows(&o["classes"]).iter().zip(rows(&b["classes"])) {
        assert_eq!(
            row["before"]["owner"],
            json!({"kind":"definition","value":{"kind":"class","value":class["class"]}})
        );
        assert_eq!(row["before"]["programs"]["closure"]["kind"], "partial");
        let mut expected = row["before"].clone();
        let programs = expected["programs"]["members"].as_array_mut().unwrap();
        let matching: Vec<_> = programs
            .iter()
            .enumerate()
            .filter(|(_, p)| p["id"] == b["program"])
            .map(|(i, _)| i)
            .collect();
        assert_eq!(matching.len(), 1);
        let mut shared = programs.remove(matching[0]);
        assert_eq!(row["after"], expected, "only shared Life leaves each Class");
        assert_eq!(rows(&shared["effects"]).len(), 1);
        assert_eq!(shared["effects"][0]["effect"]["entity"], "player");
        shared["effects"][0]["effect"]["entity"] = json!("current");
        assert_eq!(shared, o["actor"]["programs"]["members"][0]);
    }
}
fn check_observed_vectors(f: &Facts) {
    let v = &f.inputs["player-intrinsic-life/source-vectors.json"];
    let owned = &f.inputs["player-rule-ownership/source-vectors.json"];
    let b = &f.inputs["player-intrinsic-life/bindings.json"];
    let pin = &owned["inherited_source_vectors"];
    assert_eq!(pin, &f.evidence["artifacts"][3]);
    for field in ["reports", "initializer_branch", "compatibility_boundary"] {
        assert_eq!(owned[field], v[field]);
    }
    assert_eq!(owned["final_life_claim"], false);
    assert_eq!(owned["whole_build_claim"], false);
    assert_eq!(v["status"], "passed");
    let m = &v["report_metadata"];
    assert_eq!(m["manifest_sha256"], MANIFEST_HASH);
    assert_eq!(m["source_revision"], REVISION);
    assert_eq!(m["case_count"], 10);
    assert_eq!(m["complete_loads_per_jit"], 24);
    assert_eq!(rows(&m["files"]).len(), 7);
    for record in rows(&m["files"]) {
        assert_eq!(
            record["sha256"],
            exact(&f.manifest["files"], "path", &record["path"])["sha256"]
        );
    }
    for field in [
        "original_initializer",
        "original_eval_mod",
        "fresh_unhooked_comparison",
        "fresh_repeat",
        "warm_repeat",
        "independent_warm_repeat",
        "warm_comparison_ignores_initializer_history_only",
    ] {
        assert_eq!(m["scope"][field], true);
    }
    for field in [
        "final_life",
        "contributor_closure",
        "generic_level_multiplier_native_admission",
    ] {
        assert_eq!(m["scope"][field], false);
    }
    assert_eq!(rows(&m["original_sources"]).len(), 5);
    for (i, record) in rows(&m["original_sources"]).iter().enumerate() {
        assert_eq!(
            record["path"],
            format!(
                "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
                i + 1
            )
        );
        assert_eq!(
            record["sha256"],
            hash(&bounded(text(&record["path"]), 1024 * 1024))
        );
    }
    assert_eq!(rows(&v["reports"]).len(), 2);
    for field in ["sha256", "bytes"] {
        assert_eq!(v["reports"][0][field], v["reports"][1][field]);
    }
    let values = [1168, 1072, 1132, 1168, 1120, 28, 1108, 1216, 1120, 1120];
    let levels = [96, 88, 93, 96, 92, 1, 91, 100, 92, 92];
    assert_eq!(rows(&v["projections"]).len(), 10);
    assert_eq!(rows(&v["native_cases"]).len(), 10);
    assert_eq!(rows(&owned["native_cases"]).len(), 10);
    for (i, projection) in rows(&v["projections"]).iter().enumerate() {
        let state = &projection["state"];
        assert_eq!(state["constant"], 12);
        assert_eq!(state["hooked"], true);
        let ids: BTreeSet<_> = rows(&b["classes"])
            .iter()
            .map(|c| {
                exact(&state["classes"], "id", &c["source_id"]);
                c["source_id"].as_u64().unwrap()
            })
            .collect();
        for mode in ["MAIN", "CALCS"] {
            let r = &state["modes"][mode];
            assert!(ids.contains(&r["class_id"].as_u64().unwrap()));
            assert_eq!(r["character_level"], levels[i]);
            assert_eq!(r["effective_value"], values[i]);
            assert_eq!(r["base_source_sum"], values[i]);
            assert_eq!(
                r["record"],
                json!({"name":"Life","type":"BASE","value":12,"source":"Base","flags":0,"keyword_flags":0,"tags":[{"type":"Multiplier","var":"Level","base":16}]})
            );
            assert_eq!(r["multiplier"]["effective"], levels[i]);
            assert_eq!(r["multiplier"]["extra_base"], 0);
            assert_eq!(r["multiplier"]["override"], json!({"present":false}));
            for parent in rows(&r["multiplier"]["read_set"]) {
                assert!(rows(&parent["multiplier_level"]).is_empty());
            }
            assert_eq!(r["provenance"]["exact_initialized_record"], true);
            assert_eq!(r["provenance"]["initialized_constant"], 12);
            assert_eq!(r["provenance"]["initialized_stored_level"], levels[i]);
        }
        let r = &state["modes"]["MAIN"];
        assert_eq!(
            v["native_cases"][i],
            json!({"case_index":i,"name":projection["name"],"class_name":r["class_name"],"character_level":r["character_level"],"intrinsic_life":r["effective_value"]})
        );
        let mut expected = v["native_cases"][i].clone();
        expected["source_class_id"] = r["class_id"].clone();
        assert_eq!(owned["native_cases"][i], expected);
    }
    let branch = &v["initializer_branch"];
    assert_eq!(branch["path"], "src/Modules/CalcSetup.lua");
    assert_eq!(branch["first"], 828);
    assert_eq!(branch["last"], 837);
    assert_eq!(branch["class_dependent_life_branch"], false);
    assert_eq!(
        text(&branch["text"]),
        f.spans["class-and-life-boundary"].trim_end_matches('\n')
    );
    let boundary = &v["compatibility_boundary"];
    assert_eq!(boundary["typed_input"], "CharacterLevel");
    assert_eq!(boundary["source_generic_multiplier"], "Multiplier:Level");
    assert_eq!(boundary["global_source_override_impossible"], false);
    assert_eq!(boundary["actual_source_extra_base"], 0);
    assert_eq!(boundary["actual_source_override_present"], false);
}
fn rejected(f: impl FnOnce() + std::panic::UnwindSafe) {
    assert!(std::panic::catch_unwind(f).is_err());
}

#[test]
fn complete_intrinsic_census_and_name_preserving_constructors_have_independent_authority() {
    let f = facts();
    assert_eq!(rows(&f.evidence["intrinsic_modifiers"]).len(), 148);
    assert_eq!(verified_bindings()["actor"]["key"], "def.000000000000332a");
    assert_eq!(
        rows(&verified_ownership()["applicability"]["members"]).len(),
        1
    );
    assert_eq!(verified_life_bindings()["per_level"], 12);
    assert_eq!(rows(&verified_life_vectors()["projections"]).len(), 10);
    assert_eq!(verified_vectors()["whole_build_claim"], false);
}
#[test]
fn census_controls_reject_added_missing_retargeted_and_list_sources() {
    let f = facts();
    let mut changed = f.evidence["intrinsic_modifiers"].clone();
    changed.as_array_mut().unwrap().remove(0);
    rejected(|| check_intrinsics(&changed, &f.spans));
    for field in ["name", "kind"] {
        let mut changed = f.evidence["intrinsic_modifiers"].clone();
        changed[0][field] = json!(if field == "name" {
            "ExtraSupport"
        } else {
            "LIST"
        });
        rejected(|| check_intrinsics(&changed, &f.spans));
    }
    let mut changed = f.spans.clone();
    changed
        .get_mut("player-initializer")
        .unwrap()
        .push_str("\t\tmodDB:NewMod(\"ExtraSupport\", \"LIST\", {})\n");
    rejected(|| check_intrinsics(&f.evidence["intrinsic_modifiers"], &changed));
}
#[test]
fn actual_foreign_factories_and_fallback_cannot_be_blessed_as_intrinsic_absence() {
    let f = facts();
    for (role, from, to) in [
        (
            "create-modifier",
            "name = modName,",
            "name = \"ExtraSupport\",",
        ),
        (
            "foreign-config-party-cache",
            "env.modDB:AddList(build.configTab.modList)",
            "-- omitted foreign configuration",
        ),
        (
            "ui-fallback-skill",
            "calcs.createActiveSkill(defaultEffect, { }, env, env.player)",
            "calcs.createActiveSkill(defaultEffect, supportList, env, env.player)",
        ),
    ] {
        let mut changed = f.spans.clone();
        assert!(changed[role].contains(from));
        changed.insert(role.to_owned(), changed[role].replace(from, to));
        rejected(|| check_wrappers_and_boundaries(&changed));
    }
    let record = &f.evidence["spans"][0];
    let source = format!("{}{}", "\n".repeat(25), f.spans["common-initializer"]);
    let mut changed = record.clone();
    changed["last"] = json!(113);
    rejected(|| {
        authenticated_span(&source, &changed, SPANS[0]);
    });
}
#[test]
fn retained_life_provenance_and_exact_actor_ownership_refuse_false_closure() {
    let f = facts();
    let mut changed = f.inputs.clone();
    changed
        .get_mut("player-rule-ownership/ownership.json")
        .unwrap()["applicability"]["members"]
        .as_array_mut()
        .unwrap()
        .clear();
    rejected(|| check_ownership(&changed));
    let mut changed = f.inputs.clone();
    changed
        .get_mut("player-rule-ownership/ownership.json")
        .unwrap()["actor"]["programs"]["closure"] = json!({"kind":"complete"});
    rejected(|| check_ownership(&changed));
    let mut changed = Facts {
        evidence: f.evidence.clone(),
        inputs: f.inputs.clone(),
        spans: f.spans.clone(),
        manifest: f.manifest.clone(),
    };
    changed
        .inputs
        .get_mut("player-intrinsic-life/source-vectors.json")
        .unwrap()["projections"][4]["state"]["modes"]["MAIN"]["multiplier"]["override"] =
        json!({"present":true,"value":92});
    rejected(|| check_observed_vectors(&changed));
}
