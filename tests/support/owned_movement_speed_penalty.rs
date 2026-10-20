//! Authenticated body movement ingredient and a bounded offline unit correction.
//! Publication rebinds current release commitments after this one recipe transition.
use poe_optimizer_core::{owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{owned_mapping::OwnedIdRegistry, owned_recipe::OwnedRecipeInput};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::PathBuf};
pub const RAW: &str = "def.00000000000029ff";
pub const IGNORE: &str = "def.00000000000033cb";
pub const BASE: &str = "def.00000000000033cc";
pub const PROGRAM: &str = "selected-body-movement-penalty";
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/movement-speed")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub definitions: Vec<DefinitionDescriptor>,
    pub owner: SchemaSubject,
    pub program: RuleProgram,
    pub query: ContributionQuery,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawProfile {
    pub template: ItemTemplateDefId,
    pub program: RuleProgram,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependencies {
    pub definitions: Vec<DefinitionDescriptor>,
    pub raw_profiles: Vec<RawProfile>,
    pub existing_actor_application: ExistingActorRuleApplication,
}
pub fn consumer() -> Consumer {
    read("equipment-penalty.json")
}
pub fn dependencies() -> Dependencies {
    read("equipment-penalty-dependencies.json")
}
fn pin(bytes: &[u8], expected: &Value) {
    assert_eq!(expected["bytes"], bytes.len());
    assert_eq!(expected["sha256"], format!("{:x}", Sha256::digest(bytes)));
}
pub fn check_authored() {
    let authoring_bytes = fs::read(data().join("equipment-penalty-authoring.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&authoring_bytes)),
        "6ef3965ade0a41b6683726247d191c005fa8d6522219f529f17a215a2b0ea158"
    );
    let authoring: Value = serde_json::from_slice(&authoring_bytes).unwrap();
    for entry in authoring["artifacts"].as_array().unwrap() {
        pin(
            &fs::read(data().join(entry["path"].as_str().unwrap())).unwrap(),
            entry,
        );
    }
    let source: Value = read("equipment-penalty-source.json");
    for entry in source["artifacts"].as_array().unwrap() {
        let bytes = fs::read(root().join(entry["path"].as_str().unwrap())).unwrap();
        pin(&bytes, entry);
        if let Some(raw) = entry.get("raw") {
            let mut plain = Vec::new();
            flate2::read::GzDecoder::new(bytes.as_slice())
                .take(4 * 1024 * 1024 + 1)
                .read_to_end(&mut plain)
                .unwrap();
            assert!(plain.len() <= 4 * 1024 * 1024);
            pin(&plain, raw);
        }
    }
    let item = fs::read(
        root().join("data/owned/poe2/3887ae68/item-structural-inventories/evidence/Item.lua.gz"),
    )
    .unwrap();
    let mut code = String::new();
    flate2::read::GzDecoder::new(item.as_slice())
        .read_to_string(&mut code)
        .unwrap();
    assert_eq!(code.matches("modList:NewMod(\"MovementSpeed\", \"BASE\", -self.base.armour.MovementPenalty, self.modSource, { type = \"Condition\", var = \"IgnoreMovementPenalties\", neg = true })").count(),1);
    let catalog: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/defence-profiles/catalog.json")).unwrap(),
    )
    .unwrap();
    let bases: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/item-bases/catalog.json")).unwrap(),
    )
    .unwrap();
    let by_name: BTreeMap<_, _> = bases["bases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| (v["name"].as_str().unwrap(), v))
        .collect();
    let mut categories = BTreeMap::new();
    for row in catalog["profiles"].as_array().unwrap() {
        if row["profile"]["value"]["fields"]
            .get("MovementPenalty")
            .is_some()
        {
            let base = by_name[row["base"].as_str().unwrap()];
            assert_eq!(base["weapon_field"], "absent");
            *categories
                .entry(base["item_type"].as_str().unwrap())
                .or_insert(0usize) += 1;
        }
    }
    assert_eq!(
        categories,
        BTreeMap::from([("Body Armour", 342), ("Shield", 143)])
    );
    let c = consumer();
    assert_eq!(c.definitions.len(), 2);
    assert!(!c.query.groups[0].members.is_complete());
    assert!(c.query.groups[0].members.members.is_empty());
    assert_eq!(c.program.context, RuleEntityKind::Actor);
    assert_eq!(c.program.effects.len(), 1);
    assert_eq!(
        dependencies().existing_actor_application.targets,
        [ExistingActorRuleTarget::Player]
    );
}
fn checked(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}
/// Correct only current raw fraction metadata. No historical bundle reader or repair fallback.
/// The inverse includes every schema/rule/routing/registry field and all numeric bits.
pub fn correct_raw_units(prior: &OwnedRecipeInput) -> Result<OwnedRecipeInput, String> {
    let mut value = serde_json::to_value(prior).map_err(|e| e.to_string())?;
    let original = value.clone();
    let mut pointers = Vec::new();
    for (i, d) in value["schema"]["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        if d["value"]["id"]["key"] == RAW {
            pointers.push(format!(
                "/schema/definitions/{i}/value/schema/value/value/value/unit/key"
            ));
        }
    }
    checked(pointers.len() == 1, "exact raw statistic declaration")?;
    let catalog: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/defence-profiles/catalog.json"))
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let policy: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/item-bases/policy.json"))
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let templates: BTreeMap<_, _> = policy["templates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            (
                v["source_base"].as_str().unwrap(),
                v["template"]["key"].as_str().unwrap(),
            )
        })
        .collect();
    let mut expected: BTreeMap<String, f64> = catalog["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| {
            v["profile"]["value"]["fields"]
                .get("MovementPenalty")
                .map(|n| {
                    (
                        templates[v["base"].as_str().unwrap()].to_owned(),
                        n.as_f64().unwrap(),
                    )
                })
        })
        .collect();
    checked(expected.len() == 485, "complete source penalty census")?;
    for (oi, owner) in value["rules"]["owners"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        for (pi, program) in owner["programs"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            if !serde_json::to_string(program).unwrap().contains(RAW) {
                continue;
            }
            checked(
                program["id"] == "raw-base-defence-profile" && program["reads"] == json!([]),
                "unreviewed raw consumer",
            )?;
            checked(
                owner["owner"]["value"]["kind"] == "item_template",
                "raw owner kind",
            )?;
            let amount = expected
                .remove(
                    owner["owner"]["value"]["value"]["key"]
                        .as_str()
                        .ok_or("raw owner")?,
                )
                .ok_or("missing/duplicate source template")?;
            let effects: Vec<_> = program["effects"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| e["effect"]["stat"]["key"] == RAW)
                .collect();
            checked(
                effects.len() == 1
                    && effects[0]["when"].is_null()
                    && effects[0]["effect"]["kind"] == "derive"
                    && effects[0]["effect"]["entity"] == "current",
                "exact raw producer",
            )?;
            let node = effects[0]["effect"]["value"].as_str().ok_or("raw node")?;
            let nodes: Vec<_> = program["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .filter(|(_, n)| n["id"] == node)
                .collect();
            checked(nodes.len() == 1, "unique raw literal")?;
            let (ni, n) = nodes[0];
            checked(
                n["expression"]["kind"] == "literal"
                    && n["expression"]["value"]["kind"] == "quantity",
                "raw literal shape",
            )?;
            checked(
                n["expression"]["value"]["value"]["value"]
                    .as_f64()
                    .map(f64::to_bits)
                    == Some(amount.to_bits()),
                "raw source amount",
            )?;
            pointers.push(format!("/rules/owners/{oi}/programs/members/{pi}/nodes/{ni}/expression/value/value/unit/key"));
        }
    }
    checked(
        expected.is_empty() && pointers.len() == 486,
        "complete current writer census",
    )?;
    for p in &pointers {
        let slot = value.pointer_mut(p).ok_or("unit pointer")?;
        checked(*slot == "def.0000000000000002", "expected old unit")?;
        *slot = json!("def.0000000000000001");
    }
    let next: OwnedRecipeInput =
        serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    for p in pointers {
        *value.pointer_mut(&p).unwrap() = json!("def.0000000000000002");
    }
    checked(value == original, "full metadata inverse")?;
    let inverse: OwnedRecipeInput = serde_json::from_value(value).map_err(|e| e.to_string())?;
    checked(
        serde_json::to_vec(&inverse).map_err(|e| e.to_string())?
            == serde_json::to_vec(prior).map_err(|e| e.to_string())?,
        "exact canonical byte inverse",
    )?;
    Ok(next)
}
/// Offline current-recipe authoring. The caller must rebind the release's dependent
/// commitments; this never loads, migrates or repairs a runtime package.
pub fn staged_recipe(prior: &OwnedRecipeInput) -> Result<OwnedRecipeInput, String> {
    check_authored();
    let deps = dependencies();
    for d in &deps.definitions {
        checked(
            prior
                .schema
                .definitions
                .iter()
                .filter(|v| v.address() == d.address())
                .count()
                == 1
                && prior
                    .schema
                    .definitions
                    .iter()
                    .find(|v| v.address() == d.address())
                    == Some(d),
            "exact current dependency definition",
        )?;
    }
    checked(
        prior
            .rules
            .existing_actor_rules
            .as_ref()
            .is_some_and(|v| v.members.contains(&deps.existing_actor_application)),
        "exact existing Player binding",
    )?;
    let mut next = correct_raw_units(prior)?;
    for raw in &deps.raw_profiles {
        let found = next
            .rules
            .owners
            .iter()
            .find(|o| o.owner == SchemaSubject::Definition(raw.template.address()))
            .and_then(|o| o.programs.members.iter().find(|p| p.id == raw.program.id));
        checked(
            found == Some(&raw.program),
            "exact raw profile program after unit repair",
        )?;
    }
    let repaired = next.clone();
    checked(
        next.registry.last_issued.get() == 0x33ca,
        "reserved registry predecessor",
    )?;
    let c = consumer();
    let mut registry = OwnedIdRegistry::new(next.registry.clone(), Default::default())
        .map_err(|e| e.to_string())?;
    for definition in &c.definitions {
        let DefinitionDescriptor::Stat(e) = definition else {
            return Err("only stat append".into());
        };
        checked(
            registry
                .allocate_definition::<StatDefinition>()
                .map_err(|e| e.to_string())?
                == e.id,
            "reserved statistic",
        )?;
    }
    next.registry = registry.input().clone();
    next.schema.definitions.extend(c.definitions);
    next.schema
        .definitions
        .sort_by_cached_key(DefinitionDescriptor::address);
    let owner = next
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == c.owner)
        .ok_or("existing Player owner")?;
    checked(
        !owner.programs.is_complete()
            && !owner.programs.members.iter().any(|p| p.id == c.program.id),
        "retain partial owner",
    )?;
    owner.programs.members.push(c.program);
    let queries = next
        .rules
        .contribution_queries
        .as_mut()
        .ok_or("existing query registry")?;
    checked(
        !queries.members.iter().any(|q| q.id == c.query.id),
        "new query",
    )?;
    queries.members.push(c.query);
    let schema = OwnedDefinitionSchemaPackage::new(next.schema.clone(), Default::default())
        .map_err(|e| e.to_string())?;
    next.rules.definitions = schema.identity().clone();
    next.routing.definitions = schema.identity().clone();
    let mut inverse = next.clone();
    inverse.registry = repaired.registry.clone();
    inverse.schema = repaired.schema.clone();
    inverse.rules.definitions = repaired.rules.definitions.clone();
    inverse.routing = repaired.routing.clone();
    inverse
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == consumer().owner)
        .unwrap()
        .programs
        .members
        .pop();
    inverse
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .pop();
    checked(inverse == repaired, "exact new ingredient inverse")?;
    Ok(next)
}
