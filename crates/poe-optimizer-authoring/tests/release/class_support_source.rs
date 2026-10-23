//! Finite Class-source proof against complete archived inputs, without Lua execution.
use flate2::read::GzDecoder;
use poe_optimizer_import::owned_mapping::MappingPackageInput;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs::File,
    io::Read,
    path::PathBuf,
    sync::OnceLock,
};

const BASE: &str = "data/owned/poe2/3887ae68/";
const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const EVIDENCE_HASH: &str = "fd3ece39e9d32c661d5a754e05c9568d270c437bce80775d2e52afcf03efa39e";
const MANIFEST: &str = "crates/poe-optimizer-pob/data/pob-source-manifest.json";
const MANIFEST_HASH: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const TREE: &str = "src/TreeData/0_5/tree.json";
const TREE_HASH: &str = "6449fe534c0265b21f59f8213254bd3f37a445887582c40aab04ad11cede3e95";
pub(super) const SOURCES: &[(u64, &str)] = &[
    (1, "def.0000000000000a1e"),
    (10, "def.0000000000000a1f"),
    (11, "def.0000000000000a20"),
    (2, "def.0000000000000a21"),
    (6, "def.0000000000000a22"),
    (7, "def.0000000000000a23"),
    (8, "def.0000000000000a24"),
    (9, "def.0000000000000a25"),
];
const FILES: &[(&str, &str)] = &[
    (
        "src/TreeData/0_5/tree.lua",
        "empty-passive-socket-intrinsics/tree.lua.gz",
    ),
    (
        TREE,
        "passive-structural-inventories/sources/src_TreeData_0_5_tree.json.gz",
    ),
    (
        "src/Classes/PassiveTree.lua",
        "passive-structural-inventories/sources/src_Classes_PassiveTree.lua.gz",
    ),
    (
        "src/Classes/PassiveSpec.lua",
        "passive-structural-inventories/sources/src_Classes_PassiveSpec.lua.gz",
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
        "src/Modules/CalcSetup.lua",
        "support-structural-inventories/sources/src_Modules_CalcSetup.lua.gz",
    ),
    (
        "src/Modules/ModParser.lua",
        "support-structural-inventories/sources/src_Modules_ModParser.lua.gz",
    ),
];
const SPANS: &[(&str, &str, usize, usize)] = &[
    ("classes-literal", "src/TreeData/0_5/tree.lua", 274, 776),
    ("tree-constructor", "src/Classes/PassiveTree.lua", 59, 446),
    ("select-class", "src/Classes/PassiveSpec.lua", 655, 678),
    ("misc-data-import", "src/Modules/Data.lua", 112, 118),
    ("unarmed-table", "src/Modules/Data.lua", 626, 636),
    ("character-constants", "src/Data/Misc.lua", 144, 245),
    (
        "environment-constructor",
        "src/Modules/CalcSetup.lua",
        717,
        2309,
    ),
    (
        "class-attribute-dispatch",
        "src/Modules/CalcSetup.lua",
        827,
        835,
    ),
    (
        "foreign-configuration-and-cache",
        "src/Modules/CalcSetup.lua",
        908,
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
    (
        "unarmed-and-foreign-weapons",
        "src/Modules/CalcSetup.lua",
        1852,
        1886,
    ),
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
];
const ARTIFACTS: &[(&str, usize, &str)] = &[
    (
        "current/mapping.json",
        3165465,
        "3b627b9e7195ba18a44f83a73aff8fbffc238162ccfc273992fc129179cdb1a4",
    ),
    (
        "class-bases/policy.json",
        985,
        "67d071f9966fd418e6cd4d14d68393b7573c92c04917ed58f4e8884b7f249afe",
    ),
    (
        "intrinsic-attack/catalog.json",
        2235,
        "e1a7fd72a1a764cf9d5cde523abb697f2184143bea19b85495f29bb7ecbc3371",
    ),
    (
        "intrinsic-attack/policy.json",
        1962,
        "0e8c34270b25a5b58285789474a0eef7e0f6e98dae77e12ed5b36a89f25ac1c2",
    ),
    (
        "attribute-stages/count-bindings.json",
        3100,
        "4b6501509beebe49e0da197f657f57df4ac86341e1f7f04e84a87990c9e7bc71",
    ),
];
struct Facts {
    classes: Value,
    inputs: BTreeMap<String, Value>,
    mapping: MappingPackageInput,
    spans: BTreeMap<String, String>,
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
    let actual: BTreeSet<_> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        actual,
        expected.iter().copied().collect(),
        "complete reviewed fields"
    );
}
fn exact<'a>(rows: &'a Value, pointer: &str, id: &Value) -> &'a Value {
    let found: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r.pointer(pointer) == Some(id))
        .collect();
    assert_eq!(found.len(), 1, "exact source identity");
    found[0]
}
fn authenticate(evidence: &Value) -> Facts {
    fields(
        evidence,
        &[
            "schema_version",
            "source_revision",
            "source_manifest",
            "scope",
            "files",
            "spans",
            "artifacts",
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
    let files = evidence["files"].as_array().unwrap();
    assert_eq!(files.len(), FILES.len());
    for (record, (path, archive)) in files.iter().zip(FILES) {
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
        assert!(bytes.len() <= 4 * 1024 * 1024 && total <= 12 * 1024 * 1024);
        pin(&bytes, record);
        if *path == TREE {
            assert_eq!(record["sha256"], TREE_HASH);
            assert_eq!(bytes.len(), 2_049_036);
        } else {
            pin(&bytes, exact(&manifest["files"], "/path", &json!(path)));
        }
        assert!(!bytes.contains(&b'\r'));
        assert!(
            sources
                .insert(*path, String::from_utf8(bytes).unwrap())
                .is_none()
        );
    }
    let spans = evidence["spans"].as_array().unwrap();
    assert_eq!(spans.len(), SPANS.len());
    let mut checked_spans = BTreeMap::new();
    for (record, (role, path, first, last)) in spans.iter().zip(SPANS) {
        assert_eq!(record["role"], *role);
        assert_eq!(record["path"], *path);
        assert_eq!(record["first"], *first);
        assert_eq!(record["last"], *last);
        let lines: Vec<_> = sources[path].lines().collect();
        assert!(*first > 0 && *last >= *first && *last <= lines.len() && last - first < 2000);
        let body = lines[first - 1..*last].join("\n") + "\n";
        assert_eq!(record["sha256"], hash(body.as_bytes()));
        assert!(checked_spans.insert((*role).to_owned(), body).is_none());
    }
    let mut inputs: BTreeMap<String, Value> = BTreeMap::new();
    let artifacts = evidence["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), ARTIFACTS.len());
    for (record, (path, count, digest)) in artifacts.iter().zip(ARTIFACTS) {
        assert_eq!(record["path"], format!("{BASE}{path}"));
        assert_eq!(record["bytes"], *count);
        assert_eq!(record["sha256"], *digest);
        let bytes = bounded(text(&record["path"]), 4 * 1024 * 1024);
        pin(&bytes, record);
        assert!(
            inputs
                .insert((*path).to_owned(), serde_json::from_slice(&bytes).unwrap())
                .is_none()
        );
    }
    let raw: Value = serde_json::from_str(&sources[TREE]).unwrap();
    assert!(
        raw.get("characterData").is_none(),
        "the reviewed class fallback is explicit"
    );
    let f = Facts {
        classes: raw["classes"].clone(),
        mapping: serde_json::from_value(inputs["current/mapping.json"].clone()).unwrap(),
        inputs,
        spans: checked_spans,
    };
    check_classes(&f.classes, &f.spans["classes-literal"]);
    check_unarmed(
        &f.classes,
        &f.inputs["intrinsic-attack/catalog.json"],
        &f.spans["unarmed-table"],
    );
    check_mapping(&f.classes, &f.inputs["current/mapping.json"]);
    check_boundaries(&f);
    f
}
fn facts() -> &'static Facts {
    static FACTS: OnceLock<Facts> = OnceLock::new();
    FACTS.get_or_init(|| {
        let bytes = bounded(&format!("{BASE}class-support-domains/evidence.json"), 65536);
        assert_eq!(hash(&bytes), EVIDENCE_HASH);
        authenticate(&serde_json::from_slice(&bytes).unwrap())
    })
}
pub(super) fn verified_classes() -> &'static Value {
    &facts().classes
}
pub(super) fn verified_mapping() -> &'static MappingPackageInput {
    &facts().mapping
}
pub(super) fn verified_unarmed() -> &'static Value {
    &facts().inputs["intrinsic-attack/catalog.json"]
}
pub(super) fn verified_count_bindings() -> &'static Value {
    &facts().inputs["attribute-stages/count-bindings.json"]
}
pub(super) fn verified_class_policy() -> &'static Value {
    &facts().inputs["class-bases/policy.json"]
}
pub(super) fn verified_attack_policy() -> &'static Value {
    &facts().inputs["intrinsic-attack/policy.json"]
}

fn background(value: &Value, class: bool) {
    let mut keys = vec!["height", "image", "section", "width", "x", "y"];
    if class {
        keys.extend(["active", "bg"]);
    }
    fields(value, &keys);
    for key in ["height", "width", "x", "y"] {
        assert!(value[key].as_f64().unwrap().is_finite());
    }
    for key in ["image", "section"] {
        assert!(!text(&value[key]).is_empty());
    }
    if class {
        for key in ["active", "bg"] {
            fields(&value[key], &["height", "width"]);
            assert!(value[key]["height"].as_u64().unwrap() > 0);
            assert!(value[key]["width"].as_u64().unwrap() > 0);
        }
    }
}
fn class_shape(classes: &Value) {
    let rows = classes.as_array().unwrap();
    assert_eq!(rows.len(), SOURCES.len());
    let mut ids = BTreeSet::new();
    for row in rows {
        fields(
            row,
            &[
                "ascendancies",
                "background",
                "base_dex",
                "base_int",
                "base_str",
                "integerId",
                "name",
            ],
        );
        assert!(ids.insert(row["integerId"].as_u64().unwrap()));
        assert!(!text(&row["name"]).is_empty());
        let base = ["base_dex", "base_int", "base_str"].map(|key| row[key].as_u64().unwrap());
        assert_eq!(base.iter().sum::<u64>(), 29);
        background(&row["background"], true);
        let ascendancies = row["ascendancies"].as_array().unwrap();
        assert!((2..=4).contains(&ascendancies.len()));
        let mut asc_ids = BTreeSet::new();
        for asc in ascendancies {
            let mut keys = vec!["background", "id", "internalId", "name"];
            // Replacement links select distinct Ascendancy definitions. They are
            // preserved here, never interpreted as intrinsic Class mechanics.
            for key in ["replace", "replaceBy"] {
                if let Some(value) = asc.get(key) {
                    assert!(!text(value).is_empty());
                    keys.push(key);
                }
            }
            fields(asc, &keys);
            for key in ["id", "internalId", "name"] {
                assert!(!text(&asc[key]).is_empty());
            }
            assert!(asc_ids.insert(text(&asc["internalId"])));
            background(&asc["background"], false);
        }
    }
    assert_eq!(ids, SOURCES.iter().map(|(id, _)| *id).collect());
}
/// A bounded serializer for the already checked finite JSON shape. It neither
/// parses source nor evaluates Lua; whole-body equality authenticates all fields.
fn literal(
    out: &mut String,
    key: &str,
    value: &Value,
    depth: usize,
    comma: bool,
    work: &mut usize,
) {
    *work += 1;
    assert!(depth <= 6 && *work <= 2048);
    write!(out, "{}{}=", "\t".repeat(depth), key).unwrap();
    match value {
        Value::Array(rows) => {
            out.push_str("{\n");
            for (i, row) in rows.iter().enumerate() {
                literal(
                    out,
                    &format!("[{}]", i + 1),
                    row,
                    depth + 1,
                    i + 1 != rows.len(),
                    work,
                );
            }
            write!(out, "{}}}", "\t".repeat(depth)).unwrap();
        }
        Value::Object(rows) => {
            out.push_str("{\n");
            for (i, (key, row)) in rows.iter().enumerate() {
                let key = if key.chars().all(|c| c.is_ascii_alphanumeric()) {
                    key.clone()
                } else {
                    format!("[{}]", serde_json::to_string(key).unwrap())
                };
                literal(out, &key, row, depth + 1, i + 1 != rows.len(), work);
            }
            write!(out, "{}}}", "\t".repeat(depth)).unwrap();
        }
        Value::String(value) => {
            assert!(
                value.len() <= 256
                    && value.chars().all(|c| c.is_ascii() && !c.is_ascii_control())
                    && !value.contains('\\')
            );
            out.push_str(&serde_json::to_string(value).unwrap());
        }
        Value::Number(value) => {
            assert!(value.as_f64().unwrap().is_finite());
            out.push_str(&value.to_string());
        }
        _ => panic!("unreviewed class literal kind"),
    }
    out.push_str(if comma { ",\n" } else { "\n" });
}
fn check_classes(classes: &Value, body: &str) {
    class_shape(classes);
    let mut rendered = String::new();
    literal(&mut rendered, "classes", classes, 1, true, &mut 0);
    assert_eq!(body, rendered, "all eight complete executed Class records");
}
fn check_mapping(classes: &Value, mapping: &Value) {
    let rows: Vec<_> = mapping["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row.pointer("/source/value/kind") == Some(&json!("class")))
        .collect();
    assert_eq!(rows.len(), SOURCES.len());
    for (id, target) in SOURCES {
        exact(classes, "/integerId", &json!(id));
        let row = exact(
            &mapping["entries"],
            "/source/value/value/key/value",
            &json!(id.to_string()),
        );
        assert_eq!(
            row["source"],
            json!({"kind":"definition","value":{"kind":"class","value":{"key":{"kind":"text","value":id.to_string()}}}})
        );
        assert_eq!(
            row["outcome"],
            json!({"kind":"mapped","value":{"target":{"kind":"definition","value":{"kind":"class","value":{"kind":"class","namespace":{"game":"poe2","version":"owned-mechanics-v1"},"key":target}}},"basis":{"kind":"exact"}}})
        );
    }
}
fn check_unarmed(classes: &Value, catalog: &Value, body: &str) {
    fields(catalog, &["schema_version", "source", "classes"]);
    assert_eq!(catalog["schema_version"], 1);
    assert_eq!(catalog["source"]["revision"], REVISION);
    let rows = catalog["classes"].as_array().unwrap();
    assert_eq!(rows.len(), 9);
    let mut ordered = BTreeMap::new();
    for row in rows {
        fields(row, &["class_key", "fields"]);
        let id = text(&row["class_key"]).parse::<u64>().unwrap();
        assert!(ordered.insert(id, &row["fields"]).is_none());
        fields(
            &row["fields"],
            &[
                "AttackRate",
                "CritChance",
                "PhysicalMax",
                "PhysicalMin",
                "type",
            ],
        );
        assert_eq!(row["fields"]["type"], "None");
        assert_eq!(row["fields"]["CritChance"].as_f64().unwrap(), 500.0 / 100.0);
    }
    assert_eq!(
        ordered.keys().copied().collect::<BTreeSet<_>>(),
        SOURCES.iter().map(|(id, _)| *id).chain([0]).collect()
    );
    let mut rendered = "data.unarmedWeaponData = {\n".to_owned();
    for (id, row) in ordered {
        let name = if id == 0 {
            "Scion"
        } else {
            text(&exact(classes, "/integerId", &json!(id))["name"])
        };
        let numeric = |key: &str| {
            let n = row[key].as_f64().unwrap();
            assert!(n.is_finite() && n > 0.0);
            n
        };
        writeln!(rendered, "\t[{id}] = {{ type = \"None\", AttackRate = {}, CritChance = data.characterConstants[\"unarmed_base_critical_strike_chance\"] / 100, PhysicalMin = {}, PhysicalMax = {} }}, -- {name}", numeric("AttackRate"), numeric("PhysicalMin"), numeric("PhysicalMax")).unwrap();
    }
    rendered.push_str("}\n");
    assert_eq!(
        body, rendered,
        "all nine unarmed rows, including explicitly excluded Scion"
    );
}
fn check_boundaries(f: &Facts) {
    let s = &f.spans;
    assert!(
        s["tree-constructor"].starts_with("function PassiveTreeClass:PassiveTree(")
            && s["tree-constructor"].ends_with("end\n")
    );
    for exact in [
        "for k, v in pairs(assert(loadstring(treeText))()) do",
        "classes[class.integerId] = class",
        "class.classes = class.ascendancies",
        "class.startNodeId = node.id",
        "ascendClass.startNodeId = node.id",
    ] {
        assert!(s["tree-constructor"].contains(exact));
    }
    assert!(
        s["select-class"].starts_with("function PassiveSpecClass:SelectClass(")
            && s["select-class"].ends_with("end\n")
    );
    for exact in [
        "local class = self.tree.classes[classId]",
        "local startNode = self.nodes[class.startNodeId]",
        "self.allocNodes[startNode.id] = startNode",
        "self:SelectAscendClass(0)",
    ] {
        assert!(s["select-class"].contains(exact));
    }
    assert!(
        s["environment-constructor"].starts_with("function calcs.initEnv(")
            && s["environment-constructor"].ends_with("end\n")
    );
    assert!(s["class-attribute-dispatch"].contains("classStats = env.spec.tree.characterData and env.spec.tree.characterData[env.classId] or env.spec.tree.classes[env.classId]"));
    assert!(
        s["class-attribute-dispatch"]
            .contains("for _, stat in pairs({\"Str\",\"Dex\",\"Int\"}) do")
    );
    assert!(
        s["class-attribute-dispatch"].contains(
            "modDB:NewMod(stat, \"BASE\", classStats[\"base_\"..stat:lower()], \"Base\")"
        )
    );
    assert!(s["misc-data-import"].contains("local miscData = LoadModule(\"Data/Misc\")\nfor k, v in pairs(miscData) do\n\tdata[k] = v\nend"));
    assert!(
        s["character-constants"].starts_with("data.characterConstants = {\n")
            && s["character-constants"].ends_with("}\n")
    );
    assert!(
        s["character-constants"].contains("\t[\"unarmed_base_critical_strike_chance\"] = 500,\n")
    );
    for exact in [
        "copyTable(env.data.unarmedWeaponData[env.classId])",
        "env.player.itemList[\"Weapon 1\"].weaponData[1]",
        "env.modDB.mods.Keystone",
        "CanAttackAsOneHandMaceUnarmed",
        "UseFacebreakerItemDamage",
    ] {
        assert!(s["unarmed-and-foreign-weapons"].contains(exact));
    }
    // These are real origin factories/consumers and foreign merges. The Class
    // proof must not certify the resulting aggregate Player database or roots.
    assert!(s["extra-support-contrast"].contains("mod(\"ExtraSupport\", \"LIST\""));
    assert!(s["linked-support-contrast"].contains("mod(\"LinkedSupport\", \"LIST\""));
    assert!(
        s["external-support-consumers"]
            .contains("env.modDB:Tabulate(\"LIST\", nil, \"LinkedSupport\")")
    );
    assert!(s["external-support-consumers"].contains("env.modDB:List(groupCfg, \"ExtraSupport\")"));
    assert!(
        s["linked-support-consumers"]
            .contains("addBestSupport(supportEffect, appliedSupportList, env.mode)")
    );
    assert!(
        s["foreign-configuration-and-cache"].contains("modDB:AddList(build.configTab.modList)")
    );
    assert!(s["foreign-item-merge"].contains("mergeDB(env.modDB, env.itemModDB)"));
    assert!(
        s["foreign-tree-merge"]
            .contains("calcs.buildModListForNodeList(env, env.allocNodes, true)")
    );
    assert_eq!(
        f.inputs["intrinsic-attack/policy.json"]["excluded_classes"],
        json!(["0"])
    );
    assert_eq!(
        f.inputs["intrinsic-attack/policy.json"]["catalog_sha256"],
        ARTIFACTS[2].2
    );
    assert_eq!(f.inputs["class-bases/policy.json"]["source_path"], TREE);
    assert_eq!(
        f.inputs["class-bases/policy.json"]["expected_class_fields"],
        json!([
            "ascendancies",
            "background",
            "base_dex",
            "base_int",
            "base_str",
            "integerId",
            "name"
        ])
    );
}
fn rejected(f: impl FnOnce() + std::panic::UnwindSafe) {
    assert!(std::panic::catch_unwind(f).is_err());
}

#[test]
fn all_eight_complete_class_sources_and_nine_unarmed_rows_are_authenticated() {
    let f = facts();
    assert_eq!(f.classes.as_array().unwrap().len(), 8);
    assert_eq!(verified_unarmed()["classes"].as_array().unwrap().len(), 9);
    assert!(!SOURCES.iter().any(|(id, _)| *id == 0));
    let witch = exact(&f.classes, "/integerId", &json!(1));
    assert_eq!(
        exact(&witch["ascendancies"], "/id", &json!("Lich"))["replaceBy"],
        "Abyssal Lich"
    );
    assert_eq!(
        exact(&witch["ascendancies"], "/id", &json!("Abyssal Lich"))["replace"],
        "Lich"
    );
}
#[test]
fn class_proof_rejects_unreviewed_nested_behavior_and_changed_source_identity() {
    let f = facts();
    for pointer in [
        "/0",
        "/0/background",
        "/0/ascendancies/0",
        "/0/ascendancies/0/background",
    ] {
        let mut changed = f.classes.clone();
        changed
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("ExtraSupport".into(), json!("unreviewed"));
        rejected(|| check_classes(&changed, &f.spans["classes-literal"]));
    }
    let mut changed = f.classes.clone();
    changed[0]["integerId"] = changed[1]["integerId"].clone();
    rejected(|| check_classes(&changed, &f.spans["classes-literal"]));
    let mut changed = f.classes.clone();
    changed[0]["base_str"] = json!(8);
    changed[0]["base_int"] = json!(6); // same total does not bless different source values
    rejected(|| check_classes(&changed, &f.spans["classes-literal"]));
    let mut mapping = f.inputs["current/mapping.json"].clone();
    let row = mapping["entries"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| {
            row.pointer("/source/value/kind") == Some(&json!("class"))
                && row.pointer("/source/value/value/key/value") == Some(&json!("1"))
        })
        .unwrap();
    row["outcome"]["value"]["target"]["value"]["value"]["key"] = json!(SOURCES[1].1);
    rejected(|| check_mapping(&f.classes, &mapping));
}
#[test]
fn unarmed_proof_requires_complete_fields_constant_projection_and_explicit_exclusion() {
    let f = facts();
    for field in ["CritChance", "PhysicalMax"] {
        let mut changed = verified_unarmed().clone();
        changed["classes"][0]["fields"][field] = json!(99);
        rejected(|| check_unarmed(&f.classes, &changed, &f.spans["unarmed-table"]));
    }
    let mut changed = verified_unarmed().clone();
    changed["classes"][0]["fields"]["ExtraSupport"] = json!([]);
    rejected(|| check_unarmed(&f.classes, &changed, &f.spans["unarmed-table"]));
    let mut changed = verified_unarmed().clone();
    changed["classes"].as_array_mut().unwrap().remove(0);
    rejected(|| check_unarmed(&f.classes, &changed, &f.spans["unarmed-table"]));
    let changed = f.spans["unarmed-table"].replace("PhysicalMin = 2", "PhysicalMin = 3");
    rejected(|| check_unarmed(&f.classes, verified_unarmed(), &changed));
}
#[test]
fn evidence_rejects_changed_revision_and_incomplete_constructor_boundaries() {
    facts(); // Rejecting a mutation is meaningful only with a passing baseline.
    let bytes = bounded(&format!("{BASE}class-support-domains/evidence.json"), 65536);
    let evidence: Value = serde_json::from_slice(&bytes).unwrap();
    let mut changed = evidence.clone();
    changed["source_revision"] = json!("different source");
    rejected(|| {
        authenticate(&changed);
    });
    let mut changed = evidence;
    changed["spans"][1]["last"] = json!(445);
    rejected(|| {
        authenticate(&changed);
    });
}
