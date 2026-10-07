//! Exact whole-draft preservation for physical Gem inventory publications.
//! Staging helpers prove the intended schema/rule changes separately. This helper
//! authenticates each complete dependency set, then permits only specified
//! physical-list completions and their exact source provenance.
use crate::selected;
use poe_optimizer_core::{
    build_identity::BuildLineage,
    data::DataIdentity,
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::ItemTemplateDefId,
    owned_draft::{DraftLimits, decode_draft},
    owned_schema::{DefinitionDescriptor, SchemaDefinitionId, SchemaState, SlotAddress},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{
        ImportQueryTemplate, NormalizationLimits, NormalizationPolicy, PhysicalGemInputInventory,
    },
    owned_release::{OwnedReleaseReceipt, StagedOwnedRelease},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    ops::Range,
    path::Path,
};

pub struct InventoryExpectation<'a> {
    pub physical: &'a PhysicalGemInputInventory,
    pub occurrences: [usize; 5],
    pub parameter_count: usize,
}
pub struct Comparison<'a> {
    pub prior: &'a StagedOwnedRelease,
    pub next: &'a StagedOwnedRelease,
    pub prior_path: &'a Path,
    pub package: &'a Path,
    pub out: &'a Path,
    pub families: &'a [InventoryExpectation<'a>],
    pub selected_before: [usize; 5],
    pub selected_after: [usize; 5],
    /// Opt in only when the family's checked migration changes schema content.
    pub rebind_definitions: bool,
}
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
pub struct Location {
    pub family: usize,
    pub ordinal: usize,
    pub group: usize,
    pub set: usize,
    // Used by the Ice Nova and Sniper mutation controls, not every consumer.
    #[allow(dead_code)]
    pub set_id: String,
    pub selected: bool,
    pub range: Range<usize>,
    #[allow(dead_code)]
    pub group_header: Range<usize>,
    pub attributes: Vec<(String, String)>,
    #[allow(dead_code)]
    pub group_attributes: Vec<(String, String)>,
    pub children: Vec<usize>,
}
pub struct Frame {
    pub locations: Vec<Location>,
    pub source_sets: BTreeMap<usize, usize>,
}
pub fn frame(xml: &[u8], physical_rows: &[&PhysicalGemInputInventory]) -> Frame {
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([93; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let e = SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let rows = e.rows();
    let skills = rows
        .iter()
        .find(|r| r.occurrence().name() == "Skills")
        .unwrap();
    let active = skills
        .attribute("activeSkillSet")
        .unwrap()
        .decoded()
        .unwrap();
    let mut source_sets = BTreeMap::new();
    for row in rows {
        let mut ancestor = Some(row.occurrence().id());
        while let Some(id) = ancestor {
            let a = &rows[id.ordinal() as usize];
            if a.occurrence().name() == "SkillSet" {
                source_sets.insert(
                    row.occurrence().id().ordinal() as usize,
                    id.ordinal() as usize,
                );
                break;
            }
            ancestor = a.occurrence().parent();
        }
    }
    let text = std::str::from_utf8(xml).unwrap();
    let locations = rows
        .iter()
        .filter(|r| {
            r.occurrence().name() == "Gem"
                && r.attribute("gemId").is_some_and(|a| {
                    physical_rows
                        .iter()
                        .any(|p| a.decoded().unwrap() == p.game_id)
                })
        })
        .map(|row| {
            let family = physical_rows
                .iter()
                .position(|p| {
                    row.attribute("gemId").unwrap().decoded().unwrap() == p.game_id
                        && row.attribute("variantId").unwrap().decoded().unwrap() == p.variant_id
                })
                .unwrap();
            let physical = physical_rows[family];
            assert_eq!(
                row.attribute("variantId").unwrap().decoded().unwrap(),
                physical.variant_id
            );
            assert_eq!(
                row.attribute("skillId").unwrap().decoded().unwrap(),
                physical.skill_id
            );
            assert_eq!(
                row.attribute("nameSpec").unwrap().decoded().unwrap(),
                physical.name_spec
            );
            let group = &rows[row.occurrence().parent().unwrap().ordinal() as usize];
            assert_eq!(group.occurrence().name(), "Skill");
            let set = &rows[group.occurrence().parent().unwrap().ordinal() as usize];
            assert_eq!(set.occurrence().name(), "SkillSet");
            let set_id = set.attribute("id").unwrap().decoded().unwrap().to_owned();
            let start = group.occurrence().range().start;
            Location {
                family,
                ordinal: row.occurrence().id().ordinal() as usize,
                group: group.occurrence().id().ordinal() as usize,
                set: set.occurrence().id().ordinal() as usize,
                selected: set_id == active,
                set_id,
                range: row.occurrence().range(),
                group_header: start..start + text[start..].find('>').unwrap() + 1,
                attributes: row
                    .attributes()
                    .iter()
                    .map(|a| (a.origin().name.clone(), a.decoded().unwrap().to_owned()))
                    .collect(),
                group_attributes: group
                    .attributes()
                    .iter()
                    .map(|a| (a.origin().name.clone(), a.decoded().unwrap().to_owned()))
                    .collect(),
                children: {
                    let range = row.occurrence().range();
                    rows.iter()
                        .filter(|child| {
                            child.occurrence().id() != row.occurrence().id()
                                && child.occurrence().range().start >= range.start
                                && child.occurrence().range().end <= range.end
                        })
                        .map(|child| child.occurrence().id().ordinal() as usize)
                        .collect()
                },
            }
        })
        .collect();
    Frame {
        locations,
        source_sets,
    }
}
pub fn origin(sidecar: &Value, ordinal: usize) -> &Value {
    let rows: Vec<_> = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["source"]["ordinal"] == ordinal)
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0]
}
pub fn origin_mut(sidecar: &mut Value, ordinal: usize) -> &mut Value {
    sidecar["origins"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["source"]["ordinal"] == ordinal)
        .unwrap()
}
pub fn link(sidecar: &Value, ordinal: usize, kind: &str) -> Value {
    let rows: Vec<_> = origin(sidecar, ordinal)["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["kind"] == kind)
        .collect();
    assert_eq!(rows.len(), 1, "exact {kind} source link at {ordinal}");
    rows[0]["value"].clone()
}
pub fn member<'a>(draft: &'a Value, field: &str, id: &Value) -> &'a Value {
    let rows: Vec<_> = draft["draft"][field]["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| &r["id"] == id)
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0]
}
pub fn preset_usage(draft: &Value, sidecar: &Value, loc: &Location) -> (Value, Value) {
    let preset_id = link(sidecar, loc.set, "skill_preset");
    let skill = link(sidecar, loc.ordinal, "skill");
    let preset = member(draft, "skill_presets", &preset_id);
    assert_eq!(
        preset["skills"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|id| **id == skill)
            .count(),
        1
    );
    let usage = &preset["usage_preferences"];
    assert_eq!(usage["completion"]["kind"], "pending");
    assert_eq!(
        usage["completion"]["code"],
        "usage-preferences-not-converted"
    );
    (preset_id, usage.clone())
}
pub fn authenticate(
    sidecar: &Value,
    directory: &Path,
    package: &Path,
    case: usize,
    release: &StagedOwnedRelease,
) {
    let receipt: OwnedReleaseReceipt = read(package.join("release.json"));
    assert_eq!(&receipt, release.receipt());
    assert!(
        receipt.evaluation.is_none(),
        "input closure supplies no evaluation coverage"
    );
    let policy: NormalizationPolicy = read(package.join("normalization.json"));
    assert_eq!(&policy, release.normalization());
    let query_name = format!("original-{case:02}");
    let queries: Vec<ImportQueryTemplate> =
        read(package.join(format!("queries-{query_name}.json")));
    assert_eq!(
        queries,
        release
            .query_sets()
            .iter()
            .find(|q| q.name.as_str() == query_name)
            .unwrap()
            .queries
    );
    // V19/V20 change the sidecar envelope; the policy commitment retains the
    // exact production v3 policy/query tuple, without a new policy domain.
    let expected = digest_owned(
        "owned-normalization-policy-v3",
        &(policy, queries),
        NormalizationLimits::default().max_policy_bytes,
    )
    .unwrap();
    let definitions: DataIdentity = serde_json::from_value(sidecar["definitions"].clone()).unwrap();
    assert_eq!(definitions, receipt.definitions);
    // These fields are typed commitments, not JSON numbers or arbitrary paths
    // removed by a recursive canonicalizer.
    for (field, expected) in [
        ("policy", expected),
        ("mapping", receipt.mapping),
        ("mapping_source", *release.mapping().source_identity()),
        ("registry", receipt.registry),
        ("skill_roles", receipt.roles),
        ("reward_policy", receipt.rewards),
        ("item_policy", receipt.items),
        ("item_source_policy", receipt.item_source),
    ] {
        let actual: OwnedContentDigest = serde_json::from_value(sidecar[field].clone()).unwrap();
        assert_eq!(actual, expected, "exact checked {field} dependency");
    }
    let tree: Option<OwnedContentDigest> =
        serde_json::from_value(sidecar["tree_policy"].clone()).unwrap();
    assert_eq!(tree, receipt.tree);
    for row in sidecar["item_texts"].as_array().unwrap() {
        for (field, expected) in [
            ("item_lines", receipt.items),
            ("policy", receipt.item_source),
        ] {
            let actual: OwnedContentDigest =
                serde_json::from_value(row["attribution"][field].clone()).unwrap();
            assert_eq!(actual, expected, "exact nested item attribution {field}");
        }
    }
    let limits = DraftLimits::default();
    let draft = decode_draft(&fs::read(directory.join("draft.json")).unwrap(), limits).unwrap();
    let actual: OwnedContentDigest = serde_json::from_value(sidecar["draft"].clone()).unwrap();
    assert_eq!(actual, draft.digest(limits.input.max_wire_bytes).unwrap());
}
fn assert_queries(c: &Comparison<'_>) {
    assert_eq!(
        c.prior.query_sets(),
        c.next.query_sets(),
        "every ordered query remains exact"
    );
    assert_eq!(c.prior.receipt().query_rows, 110);
    assert_eq!(c.next.receipt().query_rows, 110);
    for query in c.prior.query_sets() {
        let file = format!("queries-{}.json", query.name.as_str());
        assert_eq!(
            fs::read(c.prior_path.join(&file)).unwrap(),
            fs::read(c.package.join(&file)).unwrap(),
            "query publication remains byte-identical: {file}"
        );
    }
}
/// Exact source occurrence whose existing template parameter inventory was completed.
/// This permits one static item-text diagnostic retirement, never a draft issue
/// retirement, input-value change or numerical coverage promotion.
pub struct ItemParameterCompletion {
    pub original: usize,
    pub template: ItemTemplateDefId,
    pub source_ordinal: u32,
    pub content_entry: usize,
}

pub fn compare_original(case: usize, xml: &[u8], c: &Comparison<'_>) -> Value {
    compare_original_with_item_parameter_completions(case, xml, c, &[])
}

pub fn compare_original_with_item_parameter_completions(
    case: usize,
    xml: &[u8],
    c: &Comparison<'_>,
    completions: &[ItemParameterCompletion],
) -> Value {
    assert!((1..=5).contains(&case));
    let out = c.out;
    assert_queries(c);
    if c.rebind_definitions {
        assert_ne!(c.prior.receipt().definitions, c.next.receipt().definitions);
    } else {
        assert_eq!(c.prior.receipt().definitions, c.next.receipt().definitions);
    }
    let old = out.join(format!("prior-original-{case:02}"));
    let new = out.join(format!("original-{case:02}"));
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    authenticate(&sa, &old, c.prior_path, case, c.prior);
    authenticate(&sb, &new, c.package, case, c.next);
    assert_eq!(sa["schema_version"], sb["schema_version"]);
    let direct_targets = c.prior.normalization().direct_support_targets.is_some();
    assert_eq!(
        direct_targets,
        c.next.normalization().direct_support_targets.is_some(),
        "this preservation replay does not introduce or remove target authority"
    );
    let generated_inputs = c.prior.normalization().generated_skill_inputs.is_some();
    assert_eq!(
        generated_inputs,
        c.next.normalization().generated_skill_inputs.is_some(),
        "this preservation replay does not introduce generated-input authority"
    );
    // Both endpoints run the single current importer. Configuration accounting
    // uses proof schema23; generated-field accounting without it uses schema22.
    // Earlier declarations without that policy use their current applicable
    // proof shape; this is not a branch retaining an old importer algorithm.
    let expected_version = if c.next.normalization().configuration_inputs.is_some() {
        23
    } else if generated_inputs {
        22
    } else if direct_targets && matches!(case, 1 | 5) {
        20
    } else {
        19
    };
    for sidecar in [&sa, &sb] {
        // These unchanged originals are freshly normalized on both endpoints and
        // attach intrinsic range provenance. This assertion does not constrain
        // range-free controls using the shared authenticator directly.
        assert_eq!(sidecar["schema_version"], expected_version);
        assert_eq!(
            sidecar["source_sha256"],
            format!("{:x}", Sha256::digest(xml))
        );
        assert_eq!(sidecar["source_bytes"], xml.len());
    }
    for v in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(v);
    }
    assert_eq!(
        a["draft"]["allocator"], b["draft"]["allocator"],
        "local IDs and watermark remain exact"
    );
    assert_eq!(sa["allocator_after"], sb["allocator_after"]);
    let physical_rows: Vec<_> = c.families.iter().map(|e| e.physical).collect();
    let unique: BTreeSet<_> = physical_rows
        .iter()
        .map(|p| (&p.game_id, &p.variant_id))
        .collect();
    assert_eq!(
        unique.len(),
        physical_rows.len(),
        "explicit unique source families"
    );
    let f = frame(xml, &physical_rows);
    for (index, expected) in c.families.iter().enumerate() {
        assert_eq!(
            f.locations.iter().filter(|l| l.family == index).count(),
            expected.occurrences[case - 1],
            "every selected and archived occurrence of {}",
            expected.physical.game_id
        );
    }
    let mut retired = BTreeSet::new();
    for loc in &f.locations {
        let expected = &c.families[loc.family];
        let gem_id = link(&sa, loc.ordinal, "gem");
        assert_eq!(link(&sb, loc.ordinal, "gem"), gem_id);
        let gem = member(&a, "gems", &gem_id);
        let next = member(&b, "gems", &gem_id);
        assert_eq!(
            gem["definition"],
            json!({"kind":"known","value":expected.physical.gem})
        );
        for field in ["id", "definition", "level", "quality"] {
            assert_eq!(gem[field], next[field]);
        }
        assert_eq!(gem["parameters"]["members"], next["parameters"]["members"]);
        assert_eq!(
            gem["parameters"]["members"].as_array().unwrap().len(),
            expected.parameter_count
        );
        assert_eq!(
            gem["parameters"]["completion"]["code"],
            "gem-parameters-not-converted"
        );
        assert_eq!(next["parameters"]["completion"], json!({"kind":"complete"}));
        let issue = gem["parameters"]["completion"]["id"].clone();
        assert!(retired.insert(issue["local"].as_str().unwrap().to_owned()));
        let (preset, usage) = preset_usage(&a, &sa, loc);
        assert_eq!(preset_usage(&b, &sb, loc), (preset.clone(), usage.clone()));
        let usage_issue = &usage["completion"]["id"];
        let existing: Vec<_> = sa["origins"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| {
                r["links"]
                    .as_array()
                    .unwrap()
                    .contains(&json!({"kind":"issue","value":usage_issue}))
            })
            .collect();
        assert!(!existing.is_empty(), "real prior usage obligation");
        for row in existing {
            let ordinal = row["source"]["ordinal"].as_u64().unwrap() as usize;
            assert_eq!(f.source_sets.get(&ordinal), Some(&loc.set));
        }
        let links = origin_mut(&mut sa, loc.ordinal)["links"]
            .as_array_mut()
            .unwrap();
        let before = links.len();
        links.retain(|v| *v != json!({"kind":"issue","value":issue}));
        assert_eq!(before - links.len(), 1);
        for source in [loc.ordinal, loc.group] {
            let links = origin_mut(&mut sa, source)["links"].as_array_mut().unwrap();
            for value in [
                json!({"kind":"skill_preset","value":preset}),
                json!({"kind":"issue","value":usage_issue}),
            ] {
                if !links.contains(&value) {
                    links.push(value);
                }
            }
        }
        let skill = link(&sa, loc.ordinal, "skill");
        for child in &loc.children {
            let links = origin_mut(&mut sa, *child)["links"].as_array_mut().unwrap();
            for value in [
                json!({"kind":"gem","value":gem_id}),
                json!({"kind":"skill","value":skill}),
            ] {
                if !links.contains(&value) {
                    links.push(value);
                }
            }
        }
        let gem = a["draft"]["gems"]["members"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|g| g["id"] == gem_id)
            .unwrap();
        gem["parameters"]["completion"] = json!({"kind":"complete"});
    }
    assert!(
        a == b,
        "all original inputs, IDs, presets, preferences, query rows and remaining obligations survive"
    );
    // Only explicitly authenticated release-bound commitments may change.
    // Source identity/metadata, allocator state, text interpretations, origins
    // and every other field must survive without filtering.
    let dependency_fields = [
        "definitions",
        "mapping",
        "registry",
        "skill_roles",
        "reward_policy",
        "item_policy",
        "item_source_policy",
    ];
    for field in dependency_fields {
        if c.rebind_definitions {
            sb[field] = sa[field].clone();
        } else {
            assert_eq!(sa[field], sb[field], "unchanged checked {field}");
        }
    }
    if c.rebind_definitions {
        for row in sb["item_texts"].as_array_mut().unwrap() {
            row["attribution"]["item_lines"] = json!(c.prior.receipt().items);
            row["attribution"]["policy"] = json!(c.prior.receipt().item_source);
        }
    }
    for field in ["draft", "policy", "tree_policy"] {
        sb[field] = sa[field].clone();
    }
    let retired_item_diagnostics =
        retire_item_parameter_diagnostics(case, xml, c, completions, &mut sa, &sb);
    assert!(
        sa == sb,
        "only exact retired physical issue, item parameter diagnostic and same-preset deferred usage provenance changes"
    );
    let mut x = selected::selection(xml, &old);
    let mut y = selected::selection(xml, &new);
    selected::canonical(&mut x);
    selected::canonical(&mut y);
    assert_eq!(x, y, "original saved MAIN/CALCS request selection");
    let before = selected::finalize_with_definitions(
        xml,
        &old,
        &out.join(format!("prior-selected-{case:02}.json")),
        &c.prior_path.join("schema.json"),
    );
    let after = selected::finalize_with_definitions(
        xml,
        &new,
        &out.join(format!("selected-{case:02}.json")),
        &c.package.join("schema.json"),
    );
    for (report, directory) in [(&before, &old), (&after, &new)] {
        assert!(
            report["intent_validation"]["schema_issues"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let sidecar: Value = read(directory.join("sidecar.json"));
        assert_eq!(report["draft_digest"], sidecar["draft"]);
        assert_eq!(report["finalization"]["draft_digest"], sidecar["draft"]);
    }
    let mut x = before["finalization"]["issues"].clone();
    let mut y = after["finalization"]["issues"].clone();
    selected::canonical(&mut x);
    selected::canonical(&mut y);
    let count = x.as_array().unwrap().len();
    assert_eq!(count, c.selected_before[case - 1]);
    x.as_array_mut()
        .unwrap()
        .retain(|v| !retired.contains(v["id"]["local"].as_str().unwrap()));
    assert_eq!(x, y, "all other selected obligations survive");
    assert_eq!(y.as_array().unwrap().len(), c.selected_after[case - 1]);
    let mut report = json!({"original":case,"physical_lists_completed":retired.len(),"selected_before":count,"selected_after":y.as_array().unwrap().len(),"exact_local_ids":true,"usage_pending":true});
    if !completions.is_empty() {
        report["retired_item_parameter_diagnostics"] = json!(retired_item_diagnostics);
    }
    report
}

fn retire_item_parameter_diagnostics(
    case: usize,
    xml: &[u8],
    c: &Comparison<'_>,
    completions: &[ItemParameterCompletion],
    before: &mut Value,
    after: &Value,
) -> usize {
    let mut seen = BTreeSet::new();
    for expected in completions {
        assert!((1..=5).contains(&expected.original));
        assert!(
            seen.insert((
                expected.original,
                expected.source_ordinal,
                expected.content_entry
            )),
            "duplicate exact diagnostic expectation"
        );
    }
    let mut retired = 0;
    for expected in completions.iter().filter(|e| e.original == case) {
        let address = expected.template.address();
        let old: Vec<_> = c
            .prior
            .input()
            .recipe
            .schema
            .definitions
            .iter()
            .filter(|d| d.address() == address)
            .collect();
        let new: Vec<_> = c
            .next
            .input()
            .recipe
            .schema
            .definitions
            .iter()
            .filter(|d| d.address() == address)
            .collect();
        assert_eq!(old.len(), 1);
        assert_eq!(new.len(), 1);
        let (DefinitionDescriptor::ItemTemplate(old), DefinitionDescriptor::ItemTemplate(new)) =
            (old[0], new[0])
        else {
            panic!("exact item template descriptors")
        };
        let (SchemaState::Known(old), SchemaState::Known(new)) = (&old.schema, &new.schema) else {
            panic!("known item template schemas")
        };
        assert!(!old.declarations.parameters.is_complete());
        assert!(new.declarations.parameters.is_complete());
        assert_eq!(
            old.declarations.parameters.members,
            new.declarations.parameters.members
        );
        for slot in &old.declarations.parameters.members {
            let address = SlotAddress::Parameter(slot.clone());
            let old: Vec<_> = c
                .prior
                .input()
                .recipe
                .schema
                .slots
                .iter()
                .filter(|s| s.address() == address)
                .collect();
            let new: Vec<_> = c
                .next
                .input()
                .recipe
                .schema
                .slots
                .iter()
                .filter(|s| s.address() == address)
                .collect();
            assert_eq!(old.len(), 1);
            assert_eq!(new.len(), 1);
            assert_eq!(old, new, "parameter schema/presence/site remain exact");
        }
        let source = json!({"source_sha256":format!("{:x}", Sha256::digest(xml)),"ordinal":expected.source_ordinal});
        let matching = |sidecar: &Value| {
            sidecar["item_texts"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .filter_map(|(i, row)| {
                    (row["source"] == source && row["content_entry"] == expected.content_entry)
                        .then_some(i)
                })
                .collect::<Vec<_>>()
        };
        let old_indices = matching(before);
        let new_indices = matching(after);
        assert_eq!(old_indices.len(), 1);
        assert_eq!(
            old_indices, new_indices,
            "exact source occurrence and ordered item-text position"
        );
        let index = old_indices[0];
        let old = &mut before["item_texts"][index];
        let new = &after["item_texts"][index];
        for row in [&*old, new] {
            assert_eq!(row["attribution"]["item"], source);
            assert_eq!(row["attribution"]["content_entry"], expected.content_entry);
            assert_eq!(
                row["attribution"]["default_scope"],
                json!({"kind":"proven","template":expected.template})
            );
        }
        assert_eq!(
            old["issues"],
            json!([{"problem":"schema_partial","lines":[]}])
        );
        assert_eq!(new["issues"], json!([]));
        old["issues"] = json!([]);
        assert_eq!(
            *old, *new,
            "every other item-text field and correspondence survives"
        );
        retired += 1;
    }
    retired
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}
pub fn header(name: &str, attrs: &[(String, String)], changes: &[(&str, Option<&str>)]) -> String {
    let mut text = format!("<{name}");
    for (key, value) in attrs {
        if !changes.iter().any(|(k, _)| *k == key) {
            text.push_str(&format!(" {key}=\"{}\"", escape(value)));
        }
    }
    for (key, value) in changes {
        if let Some(value) = value {
            text.push_str(&format!(" {key}=\"{}\"", escape(value)));
        }
    }
    text.push('>');
    text
}
