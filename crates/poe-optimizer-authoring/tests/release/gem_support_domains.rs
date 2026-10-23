//! Source capability proofs checked against the maintained KDL/DSL release.
//! These offline source identities never become native calculation dispatch.
use super::tracked_release;
use poe_optimizer_core::{owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_data::skill_identities::SkillIdentityCatalog;
use poe_optimizer_import::owned_mapping::{
    ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, SourceComponent,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

const GAP: &str = "additional-gem-support-origins-not-converted";
const IDENTITIES: &str = "data/owned/poe2/3887ae68/import/skill-identities.json";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    read_json(&format!(
        "data/owned/poe2/3887ae68/gem-support-domains/{name}"
    ))
}
fn read_json<T: DeserializeOwned>(path: &str) -> T {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn key(name: &str) -> OwnedDefinitionKey {
    name.parse().unwrap()
}
fn rows(value: &Value) -> Option<&[Value]> {
    value
        .as_array()
        .map(Vec::as_slice)
        .or_else(|| value.as_object().filter(|o| o.is_empty()).map(|_| &[][..]))
}
fn identities() -> SkillIdentityCatalog {
    SkillIdentityCatalog::new(read_json(IDENTITIES)).unwrap()
}
mod catalog_evidence {
    use super::*;
    pub fn read<T: DeserializeOwned>(name: &str) -> T {
        read_json(&format!(
            "data/owned/poe2/3887ae68/empty-payload-inventory/{name}"
        ))
    }
}
/// Review the finite missing-effect classifications against the existing
/// catalog. Exact generated blocks preserve IsSupport independently of whether
/// the runtime skill has been implemented. This does not construct that skill.
pub fn export_classes(
    proof: &Value,
    catalog: &SkillIdentityCatalog,
) -> Option<BTreeMap<String, bool>> {
    if proof["schema_version"] != 1
        || proof["source_revision"] != catalog.data().source.upstream_revision
    {
        return None;
    }
    let missing: BTreeSet<_> = catalog
        .data()
        .missing_references
        .iter()
        .map(|r| r.effect_id.as_str())
        .collect();
    let mut classes = BTreeMap::new();
    let mut positions = BTreeSet::new();
    let mut control_classes = BTreeSet::new();
    for (field, is_control) in [("records", false), ("controls", true)] {
        for row in proof[field].as_array()? {
            let id = row["effect_id"].as_str()?;
            let support = row["support"].as_bool()?;
            if is_control {
                if support != (catalog.skill_by_id(id)?.support == Some(true)) {
                    return None;
                }
                control_classes.insert(support);
            } else if !missing.contains(id) || classes.insert(id.to_owned(), support).is_some() {
                return None;
            }
            let flags = if support { "" } else { "#flags\n" };
            let expected = format!("#skill {id}\n#set {id}\n{flags}#mods\n#skillEnd\n");
            let blocks = row["blocks"].as_array()?;
            if blocks.is_empty() {
                return None;
            }
            for block in blocks {
                let line = block["line"].as_u64()?;
                if line == 0 || !positions.insert(line) || block["text"] != expected {
                    return None;
                }
            }
        }
    }
    (classes.keys().map(String::as_str).collect::<BTreeSet<_>>() == missing
        && control_classes == BTreeSet::from([false, true]))
    .then_some(classes)
}

/// This proves support-source classification only: the primary may be an authored
/// support; any additional support needs unimplemented origin authority.
/// Display order, active level/quality, source groups and final DPS are irrelevant.
pub fn assignment_only(
    record: &Value,
    catalog: &SkillIdentityCatalog,
    exported: &BTreeMap<String, bool>,
) -> Option<bool> {
    let gem = catalog.gem_by_key(record["gem_id"].as_str()?)?;
    let primary = record["primary_effect"].as_str()?;
    if !record["selector_resolves_same"].as_bool()?
        || primary != gem.primary_effect_id
        || record["game_id"] != gem.game_id
        || record["variant_id"] != gem.variant_id
    {
        return None;
    }
    let effects = rows(&record["effects"])?;
    let mut ids = BTreeSet::new();
    let mut additional = BTreeSet::new();
    let mut only = true;
    let mut found_primary = false;
    for effect in effects {
        let id = effect["id"].as_str()?;
        if !ids.insert(id) {
            return None;
        }
        let support = effect["support"].as_bool()?;
        if support != (catalog.skill_by_id(id)?.support == Some(true)) {
            return None;
        }
        if id == primary {
            found_primary = support == record["primary_support"].as_bool()?;
        } else {
            additional.insert(id);
            only &= !support;
        }
    }
    if !found_primary || ids != gem.effect_list.iter().map(String::as_str).collect() {
        return None;
    }
    let supplied = rows(&record["additional_effects"])?;
    let actual: BTreeSet<_> = supplied.iter().map(Value::as_str).collect::<Option<_>>()?;
    if actual.len() != supplied.len()
        || actual != additional
        || actual != gem.additional_effects.iter().map(String::as_str).collect()
    {
        return None;
    }
    let references = rows(&record["declared_references"])?;
    // The older observer flattens reference kinds. Reconcile every field against
    // the existing typed catalog; stat-set metadata is not an effect origin.
    // Use constructed fields so setup-generated additions are also accounted for.
    let mut expected = BTreeMap::new();
    for (prefix, entries) in [
        (
            "additionalGrantedEffectId",
            &gem.constructed_additional_effects,
        ),
        ("additionalStatSet", &gem.declared_additional_stat_sets),
    ] {
        for entry in entries {
            expected.insert(format!("{prefix}{}", entry.index), entry.id.as_str());
        }
    }
    if references.len() != expected.len() {
        return None;
    }
    for reference in references {
        let field = reference["field"].as_str()?;
        let id = reference["id"].as_str()?;
        if expected.remove(field)? != id
            || reference["present_in_constructed_effect_list"].as_bool()? != ids.contains(id)
            || reference["resolves_as_effect"].as_bool()? != catalog.skill_by_id(id).is_some()
        {
            return None;
        }
    }
    let constructed: BTreeSet<_> = gem
        .constructed_additional_effects
        .iter()
        .map(|reference| reference.id.as_str())
        .collect();
    if constructed.len() != gem.constructed_additional_effects.len()
        || !additional.is_subset(&constructed)
    {
        return None;
    }
    for id in constructed.difference(&additional) {
        // A loaded effect omitted from construction is a different discrepancy.
        // Only the exact reviewed missing-runtime classification is admitted.
        if catalog.skill_by_id(id).is_some() {
            return None;
        }
        only &= !*exported.get(*id)?;
    }
    Some(only)
}

fn authenticate_evidence(authoring: &Value) {
    assert_eq!(authoring["kind"], "gem-support-source-domains");
    assert_eq!(authoring["schema_version"], 1);
    for field in ["artifacts", "evidence"] {
        for pin in authoring[field].as_array().unwrap() {
            let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
            assert_eq!(pin["bytes"], bytes.len());
            assert_eq!(pin["sha256"], hash(&bytes));
        }
    }
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    let observed: Value = catalog_evidence::read("authoring.json");
    // The retained source rows are pinned offline evidence. The old publication
    // helper additionally consulted local runs/JIT reports; those untracked
    // artifacts are not a portable source of truth for this release target.
    for proof in [authoring, &observed] {
        assert_eq!(proof["source_manifest_sha256"], hash(&manifest_bytes));
        assert_eq!(proof["source_revision"], manifest["upstream_revision"]);
        for pin in proof["source_files"].as_array().unwrap() {
            assert!(manifest["files"].as_array().unwrap().contains(pin));
            let text = fs::read_to_string(
                root()
                    .join("vendor/path-of-building-poe2")
                    .join(pin["path"].as_str().unwrap()),
            )
            .unwrap()
            .replace("\r\n", "\n");
            assert_eq!(pin["bytes"], text.len());
            assert_eq!(pin["sha256"], hash(text.as_bytes()));
        }
    }
    assert!(
        authoring["source_files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|pin| pin["path"] == "src/Export/Scripts/skillGemList.lua")
    );
    assert!(
        authoring["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|pin| pin["path"] == IDENTITIES)
    );
    authenticate_exports(&read("export-classifications.json"));
}

fn authenticate_exports(proof: &Value) {
    // The generated template is not in the Lua manifest. Its reviewed SHA-256
    // authenticates normalized source bytes without Git access or a Lua host.
    let text = fs::read_to_string(
        root()
            .join("vendor/path-of-building-poe2")
            .join(proof["template"]["path"].as_str().unwrap()),
    )
    .unwrap()
    .replace("\r\n", "\n");
    assert_eq!(proof["template"]["bytes"], text.len());
    assert_eq!(proof["template"]["sha256"], hash(text.as_bytes()));
    let lines: Vec<_> = text.lines().collect();
    for field in ["records", "controls"] {
        for row in proof[field].as_array().unwrap() {
            let header = format!("#skill {}", row["effect_id"].as_str().unwrap());
            let actual: Vec<_> = lines
                .iter()
                .enumerate()
                .filter(|(_, line)| **line == header)
                .map(|(i, _)| i + 1)
                .collect();
            let blocks = row["blocks"].as_array().unwrap();
            assert_eq!(
                actual,
                blocks
                    .iter()
                    .map(|b| b["line"].as_u64().unwrap() as usize)
                    .collect::<Vec<_>>(),
                "every duplicate export occurrence"
            );
            for block in blocks {
                let start = block["line"].as_u64().unwrap() as usize - 1;
                let expected = block["text"].as_str().unwrap();
                assert_eq!(
                    lines[start..start + expected.lines().count()].join("\n") + "\n",
                    expected
                );
            }
        }
    }
}

fn check_authored() {
    let authoring: Value = read("authoring.json");
    authenticate_evidence(&authoring);
    let records: Vec<Value> = catalog_evidence::read("source-records.json");
    let bindings: Vec<Value> = catalog_evidence::read("bindings.json");
    let catalog = identities();
    let exported = export_classes(&read("export-classifications.json"), &catalog).unwrap();
    assert_eq!(records.len(), bindings.len());
    assert_eq!(records.len(), catalog.data().gems.len());
    let release = tracked_release().release();
    let mut expected = Vec::new();
    let mut source_ids = BTreeSet::new();
    let mut owners = BTreeSet::new();
    let (mut known, mut unmapped, mut unresolved) = (0, 0, 0);
    for (record, binding) in records.iter().zip(&bindings) {
        assert_eq!(record["gem_id"], binding["source_gem"]);
        assert_eq!(record["game_id"], binding["game_id"]);
        assert_eq!(record["variant_id"], binding["variant_id"]);
        assert!(source_ids.insert(record["gem_id"].as_str().unwrap()));
        let gem: GemDefId = serde_json::from_value(binding["gem"].clone()).unwrap();
        let owner = SchemaSubject::Definition(gem.address());
        assert!(owners.insert(gem.clone()));
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text(binding["game_id"].as_str().unwrap().into()),
            variant_id: SourceComponent::Text(binding["variant_id"].as_str().unwrap().into()),
        });
        assert!(matches!(release.mapping().lookup(&selector),
            Some(MappingOutcome::Mapped { target: SchemaSubject::Definition(
                DefinitionAddress::Gem(found)), basis: MappingBasis::Exact }) if *found == gem));
        let domain = match assignment_only(record, &catalog, &exported) {
            Some(true) => {
                known += 1;
                SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly)
            }
            decision => {
                unmapped += 1;
                let code = if decision.is_none() {
                    unresolved += 1;
                    "gem-effect-classification-unresolved"
                } else {
                    GAP
                };
                SchemaState::Unmapped {
                    gaps: vec![SchemaGap {
                        subject: owner.clone(),
                        facet: SchemaFacet::GameRules,
                        code: key(code),
                    }],
                }
            }
        };
        expected.push(SupportSourceDomainDeclaration { owner, domain });
    }
    let mut actual: Vec<_> = release
        .input()
        .recipe
        .rules
        .support_discovery
        .as_ref()
        .unwrap()
        .providers
        .iter()
        .filter(|row| {
            matches!(
                &row.owner,
                SchemaSubject::Definition(DefinitionAddress::Gem(_))
            )
        })
        .cloned()
        .collect();
    let sort = |rows: &mut Vec<SupportSourceDomainDeclaration>| {
        rows.sort_by_key(|row| match &row.owner {
            SchemaSubject::Definition(address) => address.key().clone(),
            _ => unreachable!("only exact Gem definitions were selected"),
        });
    };
    sort(&mut actual);
    sort(&mut expected);
    assert_eq!(
        actual, expected,
        "every maintained Gem domain has independent source evidence"
    );
    assert_eq!(
        authoring["domains"],
        json!({"total":966,"known":known,"unmapped":unmapped,
        "unresolved_classification":unresolved,"missing_runtime_effects":exported.len()})
    );
    assert_eq!(known + unmapped, 966);
    assert_eq!((known, unmapped, unresolved), (937, 29, 0));
}

#[cfg(test)]
mod tests {
    use super as family;
    use super::{catalog_evidence, *};
    #[test]
    fn entire_catalog_has_explicit_known_or_unmapped_gem_source_domains() {
        family::check_authored();
    }

    #[test]
    fn additional_supports_and_missing_effect_evidence_cannot_claim_absence() {
        let records: Vec<Value> = catalog_evidence::read("source-records.json");
        let catalog = family::identities();
        assert!(
            records
                .iter()
                .any(|r| family::assignment_only(r, &catalog, &BTreeMap::new()) == Some(false))
        );
        let record = records
            .iter()
            .find(|r| {
                r["additional_effects"]
                    .as_array()
                    .is_some_and(|v| !v.is_empty())
                    && family::assignment_only(r, &catalog, &BTreeMap::new()) == Some(true)
            })
            .unwrap();
        let mut control = record.clone();
        let primary = control["primary_effect"].clone();
        let additional = control["effects"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|e| e["id"] != primary)
            .unwrap();
        additional["support"] = json!(true);
        assert_eq!(
            family::assignment_only(&control, &catalog, &BTreeMap::new()),
            None,
            "classification must match authenticated identities"
        );
        for pointer in [
            "/effects/0/support",
            "/primary_support",
            "/additional_effects",
            "/declared_references",
            "/selector_resolves_same",
        ] {
            let mut broken = record.clone();
            *broken.pointer_mut(pointer).unwrap() = Value::Null;
            assert_eq!(
                family::assignment_only(&broken, &catalog, &BTreeMap::new()),
                None,
                "{pointer}"
            );
        }
        for field in ["resolves_as_effect", "present_in_constructed_effect_list"] {
            let mut broken = record.clone();
            broken["declared_references"][0][field] = json!(false);
            assert_eq!(
                family::assignment_only(&broken, &catalog, &BTreeMap::new()),
                None
            );
        }
        let mut duplicated = record.clone();
        let extra = duplicated["effects"][0].clone();
        duplicated["effects"].as_array_mut().unwrap().push(extra);
        assert_eq!(
            family::assignment_only(&duplicated, &catalog, &BTreeMap::new()),
            None
        );
    }

    #[test]
    fn stat_set_metadata_is_not_an_additional_effect_origin() {
        let catalog = family::identities();
        let records: Vec<Value> = catalog_evidence::read("source-records.json");
        let ice = records
            .iter()
            .find(|r| r["primary_effect"] == "IceNovaPlayer")
            .unwrap();
        assert_eq!(ice["declared_references"].as_array().unwrap().len(), 2);
        assert_eq!(
            family::assignment_only(ice, &catalog, &BTreeMap::new()),
            Some(true)
        );

        // The distinction applies across the catalog, not only to the selected build.
        let mut stat_set_gems = 0;
        for record in &records {
            let gem = catalog
                .gem_by_key(record["gem_id"].as_str().unwrap())
                .unwrap();
            if !gem.declared_additional_stat_sets.is_empty() {
                stat_set_gems += 1;
                assert!(
                    family::assignment_only(record, &catalog, &BTreeMap::new()).is_some(),
                    "{}",
                    gem.key
                );
            }
        }
        assert_eq!(stat_set_gems, 111);

        for (field, value) in [
            ("field", json!("additionalGrantedEffectId1")),
            ("field", json!("unknownReference1")),
            ("id", json!("unrelated-stat-set")),
            ("resolves_as_effect", json!(true)),
            ("present_in_constructed_effect_list", json!(true)),
        ] {
            let mut broken = ice.clone();
            broken["declared_references"][0][field] = value;
            assert_eq!(
                family::assignment_only(&broken, &catalog, &BTreeMap::new()),
                None
            );
        }
        let mut incomplete = ice.clone();
        incomplete["declared_references"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert_eq!(
            family::assignment_only(&incomplete, &catalog, &BTreeMap::new()),
            None
        );
        let mut duplicate = ice.clone();
        duplicate["declared_references"][1] = duplicate["declared_references"][0].clone();
        assert_eq!(
            family::assignment_only(&duplicate, &catalog, &BTreeMap::new()),
            None
        );
    }

    #[test]
    fn absent_additional_effects_do_not_inherit_stat_set_treatment() {
        let catalog = family::identities();
        let records: Vec<Value> = catalog_evidence::read("source-records.json");
        let mut unresolved = 0;
        for record in records {
            let gem = catalog
                .gem_by_key(record["gem_id"].as_str().unwrap())
                .unwrap();
            if gem
                .constructed_additional_effects
                .iter()
                .any(|r| catalog.skill_by_id(&r.id).is_none())
            {
                unresolved += 1;
                assert_eq!(
                    family::assignment_only(&record, &catalog, &BTreeMap::new()),
                    None
                );
                let mut omitted = record;
                omitted["declared_references"] = json!([]);
                assert_eq!(
                    family::assignment_only(&omitted, &catalog, &BTreeMap::new()),
                    None
                );
            }
        }
        assert_eq!(unresolved, 8);
    }

    #[test]
    fn reviewed_export_classification_does_not_invent_runtime_effects() {
        let catalog = family::identities();
        let proof: Value = family::read("export-classifications.json");
        let exported = family::export_classes(&proof, &catalog).unwrap();
        assert_eq!(exported.len(), 8);
        let records: Vec<Value> = catalog_evidence::read("source-records.json");
        let before = serde_json::to_value(catalog.data()).unwrap();
        let mut admitted = 0;
        for record in &records {
            if family::assignment_only(record, &catalog, &BTreeMap::new()).is_none() {
                admitted += 1;
                assert_eq!(
                    family::assignment_only(record, &catalog, &exported),
                    Some(true)
                );
                let gem = catalog
                    .gem_by_key(record["gem_id"].as_str().unwrap())
                    .unwrap();
                let id = &gem.constructed_additional_effects[0].id;
                assert!(catalog.skill_by_id(id).is_none());
                assert!(!gem.effect_list.contains(id));
                let mut unreviewed = exported.clone();
                unreviewed.remove(id);
                assert_eq!(family::assignment_only(record, &catalog, &unreviewed), None);
                let mut support = exported.clone();
                support.insert(id.clone(), true);
                assert_eq!(
                    family::assignment_only(record, &catalog, &support),
                    Some(false),
                    "a classified additional support still requires origin authority"
                );
            }
        }
        assert_eq!(admitted, 8);
        assert_eq!(serde_json::to_value(catalog.data()).unwrap(), before);
    }

    #[test]
    fn export_proof_requires_exact_categories_identities_and_occurrences() {
        let catalog = family::identities();
        let proof: Value = family::read("export-classifications.json");
        for (pointer, value) in [
            ("/source_revision", json!("unreviewed-revision")),
            ("/records/0/support", json!(true)),
            ("/records/0/effect_id", json!("unreviewed-effect")),
            ("/records/0/blocks/0/text", json!("#flags\n")),
            ("/records/0/blocks/0/line", json!(0)),
            ("/records/0/blocks", json!([])),
            ("/controls", json!([])),
        ] {
            let mut broken = proof.clone();
            *broken.pointer_mut(pointer).unwrap() = value;
            assert!(
                family::export_classes(&broken, &catalog).is_none(),
                "{pointer}"
            );
        }
        let mut omitted = proof.clone();
        omitted["records"].as_array_mut().unwrap().pop();
        assert!(family::export_classes(&omitted, &catalog).is_none());
        let mut duplicate = proof;
        duplicate["records"][1] = duplicate["records"][0].clone();
        assert!(family::export_classes(&duplicate, &catalog).is_none());
    }

    #[test]
    fn display_order_does_not_supply_support_capability_or_drop_additional_effects() {
        let records: Vec<Value> = catalog_evidence::read("source-records.json");
        let catalog = family::identities();
        for mut record in records {
            let expected = family::assignment_only(&record, &catalog, &BTreeMap::new());
            record["effects"].as_array_mut().unwrap().reverse();
            assert_eq!(
                family::assignment_only(&record, &catalog, &BTreeMap::new()),
                expected
            );
        }
    }
}
