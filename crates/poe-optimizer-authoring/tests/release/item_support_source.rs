//! Four intrinsic base constructors. Physical modifiers retain separate authority.
//! Reuse the retained source checker, without its historical migration publisher.
use flate2::read::GzDecoder;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::PathBuf, sync::OnceLock};

#[path = "../../../../tests/support/owned_item_structural_inventories_evidence.rs"]
mod structural_evidence;

type Result<T> = std::result::Result<T, String>;
const BASE: &str = "data/owned/poe2/3887ae68/";
const HASH: &str = "a94f75f096cb496f7729a6fcc1723c038e6371a6cd79a2be958baf15ac80fc4c";
pub(super) const TEMPLATES: [&str; 4] = [
    "def.0000000000001d75",
    "def.0000000000001e84",
    "def.0000000000002007",
    "def.000000000000238c",
];
const SPANS: &[(&str, &str, usize, usize)] = &[
    ("item-slot", "src/Classes/Item.lua", 2414, 2669),
    ("item-slot-dispatch", "src/Classes/Item.lua", 2671, 2680),
    ("item-build", "src/Classes/Item.lua", 2694, 2863),
    ("modlist-class", "src/Classes/ModList.lua", 17, 20),
    ("modlist-constructor", "src/Classes/ModList.lua", 24, 27),
    ("modlist-add", "src/Classes/ModList.lua", 29, 31),
    ("modlist-add-list", "src/Classes/ModList.lua", 90, 96),
    ("modstore-constructor", "src/Classes/ModStore.lua", 65, 71),
    ("scale-add", "src/Classes/ModStore.lua", 82, 119),
    ("newmod-alias", "src/Classes/ModStore.lua", 18, 18),
    ("newmod", "src/Classes/ModStore.lua", 142, 144),
    ("create-mod", "src/Modules/ModTools.lua", 57, 91),
    ("set-source", "src/Modules/ModTools.lua", 277, 283),
    ("extra-support", "src/Modules/ModParser.lua", 2219, 2253),
    ("linked-support", "src/Modules/ModParser.lua", 3650, 3659),
    ("support-consumers", "src/Modules/CalcSetup.lua", 1898, 1955),
    ("linked-consumers", "src/Modules/CalcSetup.lua", 2078, 2097),
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn data() -> PathBuf {
    root().join(BASE).join("item-structural-inventories")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn ensure(ok: bool, why: &str) -> Result<()> {
    ok.then_some(()).ok_or_else(|| why.to_owned())
}
fn rows(value: &Value) -> Result<&[Value]> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or("array required".into())
}
fn bounded(path: PathBuf, limit: u64) -> Result<Vec<u8>> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    ensure(
        file.metadata().map_err(|e| e.to_string())?.len() <= limit,
        "artifact limit",
    )?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    ensure(bytes.len() as u64 <= limit, "artifact limit")?;
    Ok(bytes)
}
fn read_bytes(name: &str, limit: u64) -> Result<Vec<u8>> {
    bounded(data().join(name), limit)
}
fn json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}
fn text(value: &Value) -> &str {
    value.as_str().unwrap()
}
fn unpack(path: PathBuf, pin: &Value) -> String {
    let compressed = bounded(path, 512 * 1024).unwrap();
    assert_eq!(pin["archive_bytes"], compressed.len());
    assert_eq!(pin["archive_sha256"], hash(&compressed));
    let mut bytes = Vec::new();
    GzDecoder::new(compressed.as_slice())
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 2 * 1024 * 1024);
    assert_eq!(pin["bytes"], bytes.len());
    assert_eq!(pin["sha256"], hash(&bytes));
    String::from_utf8(bytes).unwrap()
}
struct Facts {
    evidence: Value,
    proof: Value,
    dependencies: Value,
    migration: Value,
    spans: BTreeMap<String, String>,
}
fn facts() -> &'static Facts {
    static FACTS: OnceLock<Facts> = OnceLock::new();
    FACTS.get_or_init(|| {
        let bytes = bounded(
            root().join(BASE).join("item-support-domains/evidence.json"),
            64 * 1024,
        )
        .unwrap();
        assert_eq!(hash(&bytes), HASH);
        let evidence = json(&bytes);
        let artifacts = rows(&evidence["artifacts"]).unwrap();
        assert_eq!(artifacts.len(), 3);
        let mut inputs = Vec::new();
        for (pin, name) in
            artifacts
                .iter()
                .zip(["source-vectors.json", "dependencies.json", "migration.json"])
        {
            assert_eq!(
                pin["path"],
                format!("{BASE}item-structural-inventories/{name}")
            );
            let bytes = read_bytes(name, 2 * 1024 * 1024).unwrap();
            assert_eq!(pin["bytes"], bytes.len());
            assert_eq!(pin["sha256"], hash(&bytes));
            inputs.push(json(&bytes));
        }
        let proof = inputs.remove(0);
        structural_evidence::check(&proof).unwrap();
        assert_eq!(evidence["source_revision"], proof["source_revision"]);
        assert_eq!(evidence["item_templates"], serde_json::json!(TEMPLATES));
        assert_eq!(evidence["schema_version"], 1);
        for field in [
            "numerical_completeness",
            "modifier_source_completeness",
            "source_discovery_completeness",
        ] {
            assert_eq!(evidence[field], false);
        }
        let manifest = json(
            &bounded(
                root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"),
                256 * 1024,
            )
            .unwrap(),
        );
        let mut files = BTreeMap::new();
        for pin in rows(&proof["source_files"]).unwrap() {
            files.insert(
                text(&pin["path"]).to_owned(),
                unpack(data().join(text(&pin["archive"])), pin),
            );
        }
        let extra = rows(&evidence["additional_files"]).unwrap();
        assert_eq!(extra.len(), 3);
        for (pin, (path, archive)) in extra.iter().zip([
            (
                "src/Classes/ModList.lua",
                "mana-supplier-sources/passive-ModList.lua.gz",
            ),
            (
                "src/Classes/ModStore.lua",
                "configuration-distance-inputs/ModStore.lua.gz",
            ),
            (
                "src/Modules/ModTools.lua",
                "player-support-domains/ModTools.lua.gz",
            ),
        ]) {
            assert_eq!(pin["path"], path);
            assert_eq!(pin["archive"], format!("{BASE}{archive}"));
            assert_eq!(
                rows(&manifest["files"])
                    .unwrap()
                    .iter()
                    .filter(|entry| entry["path"] == path
                        && entry["bytes"] == pin["bytes"]
                        && entry["sha256"] == pin["sha256"])
                    .count(),
                1
            );
            assert!(
                files
                    .insert(
                        path.to_owned(),
                        unpack(root().join(text(&pin["archive"])), pin)
                    )
                    .is_none()
            );
        }
        let spans = checked_spans(&evidence, &files).unwrap();
        check_outputs(&evidence["constructor_outputs"], &spans).unwrap();
        check_boundaries(&spans);
        Facts {
            evidence,
            proof,
            dependencies: inputs.remove(0),
            migration: inputs.remove(0),
            spans,
        }
    })
}
fn checked_spans(
    evidence: &Value,
    files: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    let pins = rows(&evidence["spans"])?;
    ensure(pins.len() == SPANS.len(), "complete boundary inventory")?;
    let mut result = BTreeMap::new();
    for (pin, (role, path, first, last)) in pins.iter().zip(SPANS) {
        ensure(
            pin["role"] == *role
                && pin["path"] == *path
                && pin["first"] == *first
                && pin["last"] == *last,
            "exact constructor boundary",
        )?;
        let file = files.get(*path).ok_or("missing full source")?;
        let body = file
            .split('\n')
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n");
        ensure(
            pin["bytes"] == body.len() && pin["sha256"] == hash(body.as_bytes()),
            "changed constructor bytes",
        )?;
        result.insert((*role).to_owned(), body);
    }
    Ok(result)
}
fn check_outputs(outputs: &Value, spans: &BTreeMap<String, String>) -> Result<()> {
    let calls: Vec<_> = [("item-slot", 2414), ("item-build", 2694)]
        .into_iter()
        .flat_map(|(name, first)| {
            spans[name]
                .lines()
                .enumerate()
                .filter_map(move |(i, line)| {
                    (line.contains(":NewMod(") || line.contains("modLib.createMod("))
                        .then_some((first + i, line))
                })
        })
        .collect();
    let outputs = rows(outputs)?;
    ensure(
        calls.len() == 6 && outputs.len() == 6,
        "whole constructor output inventory",
    )?;
    for (((line, call), row), (expected_line, role, name)) in calls.iter().zip(outputs).zip([
        (
            2451,
            "slot-quality",
            "\"Multiplier:QualityOn\"..slotName, \"BASE\"",
        ),
        (2562, "armour-movement", "\"MovementSpeed\", \"BASE\""),
        (
            2618,
            "unreachable-jewel",
            "\"Multiplier:GrandSpectrum\", \"BASE\"",
        ),
        (2620, "unreachable-jewel", "\"MinionModifier\", \"LIST\""),
        (2825, "cosmetic-armour-data", "\"ArmourData\", \"LIST\""),
        (2829, "cosmetic-armour-data", "\"ArmourData\", \"LIST\""),
    ]) {
        ensure(
            *line == expected_line
                && row["line"] == *line
                && row["role"] == role
                && row["text"] == *call
                && call.contains(name),
            "changed constructor output or ownership",
        )?;
    }
    Ok(())
}
fn check_boundaries(s: &BTreeMap<String, String>) {
    for (name, opening) in [
        ("item-slot", "function ItemClass:BuildModListForSlotNum("),
        (
            "item-slot-dispatch",
            "function ItemClass:BuildModListsForSlots(",
        ),
        ("item-build", "function ItemClass:BuildModList()"),
        ("modlist-constructor", "function ModListClass:ModList("),
        ("modlist-add", "function ModListClass:AddMod("),
        ("modlist-add-list", "function ModListClass:AddList("),
        ("modstore-constructor", "function ModStoreClass:ModStore("),
        ("scale-add", "function ModStoreClass:ScaleAddMod("),
        ("newmod", "function ModStoreClass:NewMod("),
        ("create-mod", "function modLib.createMod("),
        ("set-source", "function modLib.setSource("),
        ("extra-support", "local function extraSupport("),
    ] {
        assert!(s[name].starts_with(opening) && s[name].ends_with("\nend"));
    }
    for (name, expected) in [
        ("modlist-class", "newClass(\"ModList\", \"ModStore\")"),
        ("modlist-constructor", "self:ModStore(parent)"),
        ("modlist-add", "t_insert(self, mod)"),
        ("modstore-constructor", "self.parent = parent or false"),
        ("newmod-alias", "local mod_createMod = modLib.createMod"),
        ("newmod", "self:AddMod(mod_createMod(...))"),
        (
            "create-mod",
            "name = modName,\n\t\ttype = modType,\n\t\tvalue = modVal,",
        ),
        ("set-source", "mod.source = source"),
        ("scale-add", "local scaledMod = copyTable(mod)"),
        ("scale-add", "self:AddMod(scaledMod)"),
        ("item-slot", "local mod = copyTable(baseMod)"),
        ("item-slot", "mod.sourceSlot = slotName"),
        ("item-slot", "modList:AddMod(mod)"),
        ("item-slot", "elseif self.type == \"Jewel\" then"),
        ("item-build", "local baseList = new(\"ModList\"):ModList()"),
        (
            "item-build",
            "targetList:AddMod(modLib.setSource(mod, self.modSource))",
        ),
        ("item-build", "targetList:ScaleAddMod(mod, effectModifier)"),
        (
            "item-build",
            "for _, skill in ipairs(baseList:List(nil, \"ExtraSkill\")) do",
        ),
        ("extra-support", "mod(\"ExtraSupport\", \"LIST\""),
        ("linked-support", "mod(\"LinkedSupport\", \"LIST\""),
        (
            "support-consumers",
            "env.modDB:Tabulate(\"LIST\", nil, \"LinkedSupport\")",
        ),
        (
            "support-consumers",
            "env.modDB:List(groupCfg, \"ExtraSupport\")",
        ),
        (
            "linked-consumers",
            "addBestSupport(supportEffect, appliedSupportList, env.mode)",
        ),
    ] {
        assert!(s[name].contains(expected), "{name}: {expected}");
    }
    // Parsed physical records are allowed to generate supports. These five
    // lanes and buff records are foreign to the intrinsic ItemTemplate domain.
    for name in [
        "enchant",
        "rune",
        "classRequirement",
        "implicit",
        "explicit",
        "buff",
    ] {
        assert!(s["item-build"].contains(&format!("ipairs(self.{name}ModLines)")));
    }
}
pub(super) fn proof() -> &'static Value {
    &facts().proof
}
pub(super) fn dependencies() -> &'static Value {
    &facts().dependencies
}
pub(super) fn migration() -> &'static Value {
    &facts().migration
}

#[test]
fn full_bases_and_named_constructor_outputs_keep_modifier_factories_separate() {
    let f = facts();
    for (family, id) in rows(&f.proof["families"]).unwrap().iter().zip(TEMPLATES) {
        assert_eq!(family["id"], id);
        let fields = rows(&family["field_classification"]).unwrap();
        assert!(!fields.iter().any(|field| ["weapon", "flask", "charm", "jewel"].contains(&text(&field["field"]))));
        assert!(fields.iter().any(|field| field["field"] == "type" && !text(&field["literal"]).contains("Jewel")));
    }
    check_outputs(&f.evidence["constructor_outputs"], &f.spans).unwrap();
    // This source distinction is positive: ExtraSkill remains a real modifier
    // grant, while ExtraSupport and LinkedSupport remain admitted external kinds.
    assert_eq!(
        f.proof["firebolt_owned_relation"]["modifier"]["value"]["id"]["key"],
        "def.00000000000031c6"
    );
}

#[test]
fn changed_or_extra_constructor_outputs_cannot_inherit_the_domain_proof() {
    let f = facts();
    check_outputs(&f.evidence["constructor_outputs"], &f.spans).unwrap();
    let mut outputs = f.evidence["constructor_outputs"].clone();
    outputs.as_array_mut().unwrap().pop();
    assert!(check_outputs(&outputs, &f.spans).is_err());
    for foreign in ["ExtraSupport", "LinkedSupport"] {
        let mut spans = f.spans.clone();
        spans.get_mut("item-slot").unwrap().push_str(&format!(
            "\n\tmodList:NewMod(\"{foreign}\", \"LIST\", {{}})"
        ));
        assert!(check_outputs(&f.evidence["constructor_outputs"], &spans).is_err());
        let mut spans = f.spans.clone();
        let line = text(&f.evidence["constructor_outputs"][1]["text"]);
        let changed = line.replace("MovementSpeed", foreign);
        spans
            .get_mut("item-slot")
            .unwrap()
            .clone_from(&f.spans["item-slot"].replace(line, &changed));
        let mut rows = f.evidence["constructor_outputs"].clone();
        rows[1]["text"] = changed.into();
        assert!(check_outputs(&rows, &spans).is_err());
    }
}

#[test]
fn missing_fields_crossed_owners_and_shortened_constructor_proofs_are_refused() {
    let f = facts();
    for i in 0..4 {
        let mut proof = f.proof.clone();
        proof["families"][i]["field_classification"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(
            structural_evidence::check(&proof)
                .unwrap_err()
                .contains("every base field")
        );
        let mut proof = f.proof.clone();
        proof["families"][i]["mapping_entries"][0]["outcome"]["value"]["target"]["value"]["value"]
            ["key"] = "unreviewed-owner".into();
        assert!(
            structural_evidence::check(&proof)
                .unwrap_err()
                .contains("mapped base owner")
        );
    }
    let mut proof = f.proof.clone();
    proof["structural_boundaries"][2]["end_line"] = 2813.into();
    assert!(
        structural_evidence::check(&proof)
            .unwrap_err()
            .contains("writer boundary")
    );
}
