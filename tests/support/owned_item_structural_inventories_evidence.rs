//! Pinned offline source review. Lua tables below are evidence, never native inputs.
use super::{Result, data, ensure, hash, read_bytes, root, rows};
use flate2::read::GzDecoder;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
};

const MANIFEST: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const BOUNDARIES: [(&str, u64, u64); 7] = [
    ("src/Modules/Data.lua", 1096, 1100),
    ("src/Classes/Item.lua", 1068, 1106),
    ("src/Classes/Item.lua", 2694, 2863),
    ("src/Modules/ModParser.lua", 2193, 2200),
    ("src/Modules/CalcSetup.lua", 1098, 1107),
    ("src/Modules/CalcSetup.lua", 1685, 1730),
    ("src/Classes/Item.lua", 2414, 2680),
];
const ITEMS: [(u64, &str, &str, &str, u64); 4] = [
    (28, "Ashen Staff", "Staff", "Weapon 1", 594),
    (27, "Fine Belt", "Belt", "Belt", 590),
    (20, "Rope Cuffs", "Gloves", "Gloves", 574),
    (19, "Tattered Robe", "Body Armour", "Body Armour", 572),
];

fn unpack(pin: &Value, limit: u64) -> Result<Vec<u8>> {
    let name = pin["archive"].as_str().ok_or("retained archive path")?;
    let local = name
        .strip_prefix("evidence/")
        .ok_or("local evidence archive")?;
    ensure(
        !local.contains(['/', '\\']) && !local.starts_with('.'),
        "bounded evidence filename",
    )?;
    let compressed = read_bytes(name, 512 * 1024)?;
    ensure(
        pin["archive_bytes"] == compressed.len() && pin["archive_sha256"] == hash(&compressed),
        "compressed evidence commitment",
    )?;
    let mut raw = Vec::new();
    GzDecoder::new(compressed.as_slice())
        .take(limit + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    ensure(raw.len() as u64 <= limit, "expanded evidence byte limit")?;
    Ok(raw)
}

fn source_files(proof: &Value, manifest: &Value) -> Result<BTreeMap<String, String>> {
    let pins = rows(&proof["source_files"])?;
    ensure(pins.len() == 10, "complete pinned source file census")?;
    let mut files = BTreeMap::new();
    for pin in pins {
        let path = pin["path"].as_str().ok_or("source path")?;
        let source = unpack(pin, 2 * 1024 * 1024)?;
        ensure(
            pin["bytes"] == source.len() && pin["sha256"] == hash(&source),
            "complete pinned source file bytes",
        )?;
        ensure(
            rows(&manifest["files"])?
                .iter()
                .filter(|m| {
                    m["path"] == path && m["sha256"] == pin["sha256"] && m["bytes"] == pin["bytes"]
                })
                .count()
                == 1,
            "independent original source manifest binding",
        )?;
        ensure(
            files
                .insert(
                    path.to_owned(),
                    String::from_utf8(source).map_err(|e| e.to_string())?,
                )
                .is_none(),
            "duplicate source file",
        )?;
    }
    Ok(files)
}

fn span(span: &Value, files: &BTreeMap<String, String>) -> Result<()> {
    let source = files
        .get(span["path"].as_str().ok_or("span source")?)
        .ok_or("missing complete source")?;
    let start = span["start_line"].as_u64().ok_or("span start")? as usize;
    let end = span["end_line"].as_u64().ok_or("span end")? as usize;
    ensure(
        start > 0 && end >= start && end <= source.lines().count(),
        "source span range",
    )?;
    let body = source
        .split('\n')
        .skip(start - 1)
        .take(end - start + 1)
        .collect::<Vec<_>>()
        .join("\n");
    ensure(span["text"] == body, "entire reviewed source span changed")
}

fn field_role(name: &str) -> Option<&'static str> {
    match name {
        "type" | "subType" | "tags" => Some("classification"),
        "quality" => Some("quality_parameter"),
        "socketLimit" | "charmLimit" => Some("socket_capacity"),
        "implicit" | "implicitModTypes" => Some("modifier_recipe"),
        "req" => Some("item_requirements"),
        "armour" => Some("numeric_properties"),
        _ => None,
    }
}

fn bases(proof: &Value, files: &BTreeMap<String, String>) -> Result<()> {
    let families = rows(&proof["families"])?;
    ensure(families.len() == ITEMS.len(), "exact four base records")?;
    for (index, (id, name, _, _, ordinal)) in ITEMS.into_iter().enumerate() {
        let family = &families[index];
        ensure(
            family["source_item_id"] == id
                && family["source_base"] == name
                && family["source_ordinal"] == ordinal,
            "exact source item and base identity",
        )?;
        span(&family["base_source"], files)?;
        let body = family["base_source"]["text"]
            .as_str()
            .ok_or("whole base source")?;
        let lines = body.lines().collect::<Vec<_>>();
        ensure(
            lines.first() == Some(&format!("itemBases[\"{name}\"] = {{").as_str())
                && lines.last() == Some(&"}"),
            "whole literal base row",
        )?;
        let fields = rows(&family["field_classification"])?;
        ensure(
            fields.len() + 2 == lines.len(),
            "every base field classified",
        )?;
        let mut names = BTreeSet::new();
        for (line, field) in lines[1..lines.len() - 1].iter().zip(fields) {
            let name = field["field"].as_str().ok_or("base field name")?;
            ensure(names.insert(name), "duplicate base field classification")?;
            let role = field_role(name).ok_or("unreviewed base field or structural factory")?;
            ensure(
                field["role"] == role
                    && field["literal"] == *line
                    && line.trim_start().starts_with(&format!("{name} = ")),
                "exact literal field classification",
            )?;
            ensure(
                !line.contains("function") && !line.contains("setmetatable"),
                "callable or dynamic base payload",
            )?;
        }
        let mappings = rows(&family["mapping_entries"])?;
        ensure(mappings.len() == 1, "exact single base mapping")?;
        let mapping = &mappings[0];
        ensure(
            mapping["source"]
                == json!({"kind":"definition","value":{"kind":"item_template","value":{
                    "base":{"kind":"text","value":name},"prototype":{"kind":"missing"},"variant":{"kind":"missing"}
                }}})
                && mapping["outcome"]["kind"] == "mapped"
                && mapping["outcome"]["value"]["basis"] == json!({"kind":"exact"})
                && mapping["outcome"]["value"]["target"]["kind"] == "definition"
                && mapping["outcome"]["value"]["target"]["value"]["kind"] == "item_template"
                && mapping["outcome"]["value"]["target"]["value"]["value"]["key"] == family["id"],
            "exact mapped base owner",
        )?;
        for facet in ["grants", "actors", "skill_grants"] {
            ensure(
                family["proposed_intrinsic_membership"][facet] == json!([]),
                "intrinsic membership must remain empty",
            )?;
        }
        let item = &family["original_05_item"];
        ensure(
            item["template"] == family["id"]
                && rows(&item["item_source_ordinals"])?.contains(&json!(ordinal)),
            "exact original item to source occurrence join",
        )?;
        let expected = match id {
            28 => vec!["def.00000000000031c6", "def.00000000000031ad"],
            27 => vec![
                "def.0000000000003172",
                "def.000000000000318c",
                "def.0000000000003100",
            ],
            _ => vec!["def.0000000000003100"],
        };
        let modifiers = rows(&item["modifiers"])?;
        ensure(
            modifiers.len() == expected.len(),
            "exact physical modifier census",
        )?;
        for (modifier, key) in modifiers.iter().zip(expected) {
            ensure(
                modifier["definition"]["kind"] == "known"
                    && modifier["definition"]["value"]["key"] == key,
                "existing physical modifier owner",
            )?;
        }
    }
    Ok(())
}

fn metadata_files(metadata: &Value, manifest: &Value) -> Result<()> {
    let files = rows(&metadata["files"])?;
    let mut paths = BTreeSet::new();
    for file in files {
        let path = file["path"].as_str().ok_or("witness source path")?;
        ensure(paths.insert(path), "duplicate witness source file")?;
        ensure(
            rows(&manifest["files"])?
                .iter()
                .filter(|m| m["path"] == file["path"] && m["sha256"] == file["sha256"])
                .count()
                == 1,
            "witness source manifest",
        )?;
    }
    Ok(())
}

fn constructions(proof: &Value, manifest: &Value) -> Result<()> {
    let witnesses = rows(&proof["static_witnesses"])?;
    ensure(witnesses.len() == 3, "complete construction witness census")?;
    for (index, witness) in witnesses.iter().enumerate() {
        let reports = rows(&witness["reports"])?;
        ensure(reports.len() == 2, "independent JIT off/on reports")?;
        let raw = unpack(witness, 4 * 1024 * 1024)?;
        for report in reports {
            ensure(
                report["sha256"] == hash(&raw) && report["bytes"] == raw.len(),
                "exact historical full construction report",
            )?;
        }
        let report: Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
        let mut metadata = report.clone();
        metadata
            .as_object_mut()
            .ok_or("report object")?
            .remove("cases");
        ensure(
            metadata == witness["metadata"],
            "whole construction metadata",
        )?;
        ensure(
            metadata["source_hash"] == MANIFEST
                && metadata["evidence"]["manifest_sha256"] == MANIFEST
                && metadata["evidence"]["native_input_closure"] == false
                && metadata["evidence"]["native_owner_coverage"] == false,
            "construction observation does not itself grant coverage",
        )?;
        metadata_files(&metadata["evidence"], manifest)?;
        let cases = rows(&report["cases"])?;
        ensure(
            cases.len() == witness["case_count"],
            "complete retained construction controls",
        )?;
        let originals = cases
            .iter()
            .filter(|c| c["name"] == "original")
            .collect::<Vec<_>>();
        ensure(originals.len() == 1, "one exact original construction")?;
        let state = &originals[0]["state"];
        ensure(
            state["original_functions_preserved"] == true,
            "original construction functions",
        )?;
        let items = if index == 0 {
            vec![&state["item"]]
        } else {
            rows(&state["items"])?.iter().collect()
        };
        let projected = rows(&witness["original_items"])?;
        ensure(items.len() == projected.len(), "all original source items")?;
        for (item, saved) in items.into_iter().zip(projected) {
            for field in ["base_facts", "selected_slots", "xml"] {
                ensure(
                    item[field] == saved[field],
                    "whole construction item projection",
                )?;
            }
            ensure(
                saved["base_mods"] == item["loaded"]["base_mods"],
                "complete physical base modifier records",
            )?;
            ensure(
                saved["id"] == item.get("id").cloned().unwrap_or(json!(28)),
                "source construction item ID",
            )?;
            let grant = item["loaded"].get("granted_skills");
            ensure(
                saved["grant_field_captured"] == grant.is_some()
                    && saved["loaded_granted_skills"] == grant.cloned().unwrap_or(Value::Null),
                "missing grant observation must stay missing",
            )?;
            let id = saved["id"].as_u64().ok_or("construction item ID")?;
            let spec = ITEMS
                .iter()
                .find(|spec| spec.0 == id)
                .ok_or("unknown construction item")?;
            let expected_xml_id = id.to_string();
            ensure(
                saved["base_facts"]["name"] == spec.1
                    && saved["base_facts"]["type"] == spec.2
                    && saved["selected_slots"] == json!([spec.3])
                    && saved["xml"]["attributes"]["id"].as_str() == Some(expected_xml_id.as_str()),
                "exact construction source/base/slot join",
            )?;
        }
    }
    Ok(())
}

fn runtime(proof: &Value, manifest: &Value) -> Result<()> {
    let witness = &proof["runtime_witness"];
    let metadata = &witness["metadata"];
    ensure(
        metadata["manifest_sha256"] == MANIFEST
            && metadata["native_inventory_authority"] == false
            && metadata["native_build_parity"] == false
            && metadata["field_non_applicability_certificate"] == false
            && metadata["all_suppliers_or_transforms_proved"] == false
            && metadata["business_wrappers"] == false
            && metadata["dormant_presets_covered"] == false,
        "runtime observation authority stays bounded",
    )?;
    metadata_files(metadata, manifest)?;
    let reports = rows(&witness["reports"])?;
    ensure(
        reports.len() == 2
            && reports[0]["sha256"] == reports[1]["sha256"]
            && reports[0]["bytes"] == reports[1]["bytes"],
        "independent equal runtime reports",
    )?;
    let observations = rows(&witness["rows"])?;
    ensure(
        observations.len() == 504,
        "complete four-item runtime projection",
    )?;
    let mut unique = BTreeSet::new();
    let mut census = BTreeMap::<u64, usize>::new();
    for row in observations {
        let case = row["case_index"].as_u64().ok_or("case index")?;
        let stage = row["stage"].as_str().ok_or("stage")?;
        let environment = row["environment_index"].as_u64().ok_or("environment")?;
        let item = &row["item"];
        let id = item["id"].as_u64().ok_or("runtime item ID")?;
        let (_, name, kind, slot, _) = ITEMS
            .iter()
            .find(|spec| spec.0 == id)
            .ok_or("unknown runtime item")?;
        ensure(
            case < 2
                && row["name"] == ["original-05", "repeat-original-05"][case as usize]
                && ["fresh", "rebuilt_once", "rebuilt_twice"].contains(&stage)
                && environment < 21
                && unique.insert((case, stage, environment, id)),
            "exact unique runtime occurrence census",
        )?;
        ensure(
            row["original_functions_preserved"] == true
                && row["hook_removed"] == true
                && ["MAIN", "CALCS", "CALCULATOR"]
                    .iter()
                    .any(|mode| row["mode"] == *mode)
                && row["axes"] == json!({"config":1,"items":2,"passives":3,"skills":4}),
            "exact original runtime execution frame",
        )?;
        let source = format!("Item:{id}:New Item, {name}");
        ensure(
            item["saved_object_exact"] == true
                && item["selected_item_id"] == id
                && item["source"] == source
                && item["type"] == *kind
                && item["slot"] == *slot,
            "exact selected item occurrence",
        )?;
        let expected = if id == 28 {
            json!({"kind":"raw_table","has_metatable":false,"fields":[{
                "key":1,"key_type":"number","value":{"kind":"raw_table","has_metatable":false,"fields":[
                    {"key":"level","key_type":"string","value":11},
                    {"key":"skillId","key_type":"string","value":"FireboltPlayer"},
                    {"key":"source","key_type":"string","value":source}
                ]}
            }]})
        } else {
            json!({"kind":"raw_table","has_metatable":false,"fields":{}})
        };
        ensure(
            item["grants"] == expected,
            "exact observed modifier-supplied grants",
        )?;
        *census.entry(id).or_default() += 1;
    }
    ensure(
        census.len() == 4 && census.values().all(|count| *count == 126),
        "complete per-item runtime census",
    )
}

pub fn check(proof: &Value) -> Result<()> {
    ensure(proof["schema_version"] == 1, "current proof format")?;
    ensure(
        proof["license"]["upstream_path"] == "LICENSE.md"
            && proof["license"]["path"] == "evidence/LICENSE.md",
        "retained upstream license path",
    )?;
    let license = read_bytes("evidence/LICENSE.md", 128 * 1024)?;
    ensure(
        proof["license"]["bytes"] == license.len() && proof["license"]["sha256"] == hash(&license),
        "complete upstream license commitment",
    )?;
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"))
            .map_err(|e| e.to_string())?;
    ensure(
        hash(&manifest_bytes) == MANIFEST && proof["source_manifest_sha256"] == MANIFEST,
        "original manifest authority",
    )?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;
    ensure(
        proof["source_revision"] == manifest["upstream_revision"],
        "source revision",
    )?;
    let files = source_files(proof, &manifest)?;
    let boundaries = rows(&proof["structural_boundaries"])?;
    ensure(
        boundaries.len() == BOUNDARIES.len(),
        "complete structural writer review",
    )?;
    for (bound, (path, first, last)) in boundaries.iter().zip(BOUNDARIES) {
        ensure(
            bound["path"] == path && bound["start_line"] == first && bound["end_line"] == last,
            "exact reviewed writer boundary",
        )?;
        span(bound, &files)?;
    }
    let build = boundaries[2]["text"].as_str().ok_or("BuildModList body")?;
    ensure(
        build.starts_with("function ItemClass:BuildModList()")
            && build.ends_with("\nend")
            && build.contains("self.grantedSkills = { }")
            && build.contains("for _, skill in ipairs(baseList:List(nil, \"ExtraSkill\")) do")
            && build.matches("t_insert(self.grantedSkills,").count() == 1,
        "reviewed sole modifier-list grant writer",
    )?;
    for list in [
        "enchant",
        "rune",
        "classRequirement",
        "implicit",
        "explicit",
    ] {
        ensure(
            build.contains(&format!("ipairs(self.{list}ModLines)")),
            "all modifier routes retained",
        )?;
    }
    // Complete pinned files and exact reviewed intervals carry the authority.
    // These assertions expose the reviewed branch/ownership facts, not a Lua parser.
    ensure(
        boundaries[4]["text"]
            .as_str()
            .ok_or("item receiving body")?
            .contains("grantedSkill.sourceItem = item"),
        "exact supplying item retained",
    )?;
    bases(proof, &files)?;
    constructions(proof, &manifest)?;
    runtime(proof, &manifest)?;
    let corpus = rows(&proof["corpus"])?;
    ensure(corpus.len() == 5, "all five source membership inventories")?;
    for (index, row) in corpus.iter().enumerate() {
        ensure(row["original"] == index + 1, "exact original order")?;
        ensure(
            rows(&row["items"])?.len() == [0, 0, 0, 1, 4][index],
            "exact selected template census",
        )?;
    }
    let earlier = &corpus[3]["items"][0];
    ensure(
        earlier["template"] == "def.0000000000001d75"
            && earlier["parameters_completion"] == "pending"
            && earlier["modifiers_completion"] == "pending",
        "Original04 unresolved physical inputs remain pending",
    )?;
    let uses = rows(&corpus[4]["items"])?
        .iter()
        .try_fold(0usize, |total, item| {
            Ok::<_, String>(total + rows(&item["equipment_uses"])?.len())
        })?;
    ensure(
        uses == 13,
        "all thirteen Original05 equipment occurrences retained",
    )?;
    ensure(data().is_dir(), "retained packet directory")
}
