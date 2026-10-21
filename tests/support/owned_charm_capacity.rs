//! Equipment-local capacity and one Player delivery; no final charm limit claim.
use poe_optimizer_core::{
    owned_content::digest_owned, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{owned_mapping::OwnedIdRegistry, owned_recipe::OwnedRecipeInput};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf};

pub const LOCAL: &str = "def.00000000000033cd";
pub const CAPACITY: &str = "def.00000000000033ce";
pub const PLAYER: &str = "def.00000000000033cf";
pub const RETIRED: &str = "local-charm-capacity-delivery-unconverted";
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/fine-belt-modifiers")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub definitions: Vec<DefinitionDescriptor>,
    pub modifier: SchemaSubject,
    pub template: SchemaSubject,
    pub modifier_program: RuleProgram,
    pub template_program: RuleProgram,
    pub query: ContributionQuery,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependencies {
    pub input: String,
    pub recipe: String,
    pub registry_last_issued: i64,
    pub modifier_descriptor: DefinitionDescriptor,
    pub slots: Vec<SlotDescriptor>,
    pub definitions: Vec<DefinitionDescriptor>,
    pub owners: Vec<DefinitionRules>,
    pub actual_rolls: Value,
    pub fixture: Value,
}
pub fn consumer() -> Consumer {
    read("charm-capacity.json")
}
pub fn dependencies() -> Dependencies {
    read("charm-capacity-dependencies.json")
}
fn pin(bytes: &[u8], p: &Value) {
    assert_eq!(p["bytes"], bytes.len());
    assert_eq!(p["sha256"], format!("{:x}", Sha256::digest(bytes)));
}
pub fn check_authored() {
    let bytes = fs::read(data().join("charm-capacity-authoring.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "6fc3dca2c8994fb4092898841aeb5a6f6e992cc027ebd91e34e3fa36725ef52a"
    );
    let a: Value = serde_json::from_slice(&bytes).unwrap();
    for p in a["artifacts"].as_array().unwrap() {
        pin(
            &fs::read(data().join(p["path"].as_str().unwrap())).unwrap(),
            p,
        );
    }
    let source: Value = read("charm-capacity-source.json");
    let mut sources = Vec::new();
    for p in source["artifacts"].as_array().unwrap() {
        let bytes = fs::read(root().join(p["path"].as_str().unwrap())).unwrap();
        pin(&bytes, p);
        let mut raw = Vec::new();
        flate2::read::GzDecoder::new(bytes.as_slice())
            .take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut raw)
            .unwrap();
        assert!(raw.len() <= 4 * 1024 * 1024);
        pin(&raw, &p["raw"]);
        sources.push(String::from_utf8(raw).unwrap());
    }
    assert!(
        sources[0].contains("itemBases[\"Fine Belt\"] = {\n\ttype = \"Belt\",\n\tcharmLimit = 0,")
    );
    assert!(sources[1].contains(
        "self.charmLimit = self.base.charmLimit + calcLocal(modList, \"CharmLimit\", \"BASE\", 0)"
    ));
    for s in source["sections"].as_array().unwrap() {
        let text = &sources[s["artifact"].as_u64().unwrap() as usize];
        let start = s["start_byte"].as_u64().unwrap() as usize;
        let end = s["end_byte"].as_u64().unwrap() as usize;
        pin(text.as_bytes().get(start..end).unwrap(), s);
    }
    let local = &source["sections"][0];
    let body = &sources[1][local["start_byte"].as_u64().unwrap() as usize
        ..local["end_byte"].as_u64().unwrap() as usize];
    assert!(body.starts_with("local function calcLocal(modList, name, type, flags)\n"));
    assert!(body.contains("mod.name == name and mod.type == type and mod.flags == flags and mod.keywordFlags == 0 and (not mod[1] or mod[1].type == \"InSlot\")"));
    assert!(body.contains("result = result + mod.value\n"));
    assert!(body.contains("t_remove(modList, i)"));
    assert!(body.ends_with("\treturn result\nend\n"));
    for p in source["admission"].as_array().unwrap() {
        pin(
            &fs::read(root().join(p["path"].as_str().unwrap())).unwrap(),
            p,
        );
    }
    assert!(sources[2].contains("if item.charmLimit then\n\t\t\t\t\tenv.modDB:NewMod(\"CharmLimit\", \"BASE\", item.charmLimit, item.title)"));
    let c = consumer();
    let d = dependencies();
    assert_eq!(a["before"], d.input);
    assert_eq!(d.registry_last_issued, 13260);
    assert_eq!(a["retired"], RETIRED);
    assert_eq!(c.definitions.len(), 3);
    assert_eq!(d.owners[0].owner, c.modifier);
    assert_eq!(d.owners[1].owner, c.template);
    assert_eq!(d.owners[0].programs.members.len(), 4);
    assert!(!d.owners[0].programs.is_complete() && !d.owners[1].programs.is_complete());
    assert!(!c.query.groups[0].members.is_complete());
    assert_eq!(c.query.groups[0].members.members.len(), 1);
    assert_eq!(d.actual_rolls["completion"]["kind"], "complete");
    assert_eq!(d.actual_rolls["members"].as_array().unwrap().len(), 25);
    pin(
        &fs::read(root().join(d.fixture["source"]["path"].as_str().unwrap())).unwrap(),
        &d.fixture["source"],
    );
    let origins = d.fixture["origins"].as_array().unwrap();
    assert_eq!(origins.len(), 2);
    for (origin, ordinal) in origins.iter().zip([590, 591]) {
        assert_eq!(origin["source"]["ordinal"], ordinal);
        assert_eq!(
            origin["source"]["source_sha256"],
            d.fixture["source"]["sha256"]
        );
        assert!(
            origin["links"]
                .as_array()
                .unwrap()
                .iter()
                .any(|link| link["kind"] == "modifier" && link["value"] == d.fixture["modifier"])
        );
        assert!(
            origin["links"]
                .as_array()
                .unwrap()
                .iter()
                .any(|link| link["kind"] == "item" && link["value"] == d.fixture["item"])
        );
    }
}
/// Offline recipe authoring only. The current validated release must be
/// reauthored after movement/Direct publication; these provisional IDs cannot
/// be installed on a different registry frontier.
pub fn staged_recipe(prior: &OwnedRecipeInput) -> Result<OwnedRecipeInput, String> {
    check_authored();
    let c = consumer();
    let d = dependencies();
    let recipe = digest_owned("owned-recipe-input-v1", prior, 64 * 1024 * 1024)
        .map_err(|e| e.to_string())?;
    if recipe.to_string() != d.recipe {
        return Err(
            "exact authenticated predecessor recipe required; reauthor the candidate".into(),
        );
    }
    if prior.registry.last_issued.get() != d.registry_last_issued {
        return Err("reauthor against current validated registry frontier".into());
    }
    for old in &d.owners {
        if prior.rules.owners.iter().find(|x| x.owner == old.owner) != Some(old) {
            return Err("exact prior owner required".into());
        }
    }
    for old in d
        .definitions
        .iter()
        .chain(std::iter::once(&d.modifier_descriptor))
    {
        if prior
            .schema
            .definitions
            .iter()
            .find(|x| x.address() == old.address())
            != Some(old)
        {
            return Err("exact definition dependency required".into());
        }
    }
    for old in &d.slots {
        if prior
            .schema
            .slots
            .iter()
            .find(|x| x.address() == old.address())
            != Some(old)
        {
            return Err("exact roll dependency required".into());
        }
    }
    let mut next = prior.clone();
    let mut registry = OwnedIdRegistry::new(next.registry.clone(), Default::default())
        .map_err(|e| e.to_string())?;
    for row in &c.definitions {
        let DefinitionDescriptor::Stat(row) = row else {
            return Err("stat-only allocation".into());
        };
        if registry
            .allocate_definition::<StatDefinition>()
            .map_err(|e| e.to_string())?
            != row.id
        {
            return Err("reauthor allocated IDs".into());
        }
    }
    next.registry = registry.input().clone();
    next.schema.definitions.extend(c.definitions);
    next.schema
        .definitions
        .sort_by_cached_key(DefinitionDescriptor::address);
    let modifier = next
        .rules
        .owners
        .iter_mut()
        .find(|x| x.owner == c.modifier)
        .unwrap();
    let SchemaClosure::Partial { gaps } = &mut modifier.programs.closure else {
        return Err("partial prior modifier required".into());
    };
    if gaps.iter().filter(|g| g.code.as_str() == RETIRED).count() != 1 {
        return Err("one exact delivery obligation required".into());
    }
    gaps.retain(|g| g.code.as_str() != RETIRED);
    modifier.programs.members.push(c.modifier_program);
    next.rules
        .owners
        .iter_mut()
        .find(|x| x.owner == c.template)
        .unwrap()
        .programs
        .members
        .push(c.template_program);
    let queries = next
        .rules
        .contribution_queries
        .as_mut()
        .ok_or("explicit query registry required")?;
    if queries.members.iter().any(|q| q.id == c.query.id) {
        return Err("query already present".into());
    }
    queries.members.push(c.query);
    let schema = OwnedDefinitionSchemaPackage::new(next.schema.clone(), Default::default())
        .map_err(|e| e.to_string())?;
    next.rules.definitions = schema.identity().clone();
    next.routing.definitions = schema.identity().clone();
    let mut inverse = next.clone();
    inverse.registry = prior.registry.clone();
    inverse.schema = prior.schema.clone();
    inverse.rules.definitions = prior.rules.definitions.clone();
    inverse.routing.definitions = prior.routing.definitions.clone();
    for old in d.owners {
        let owner = inverse
            .rules
            .owners
            .iter_mut()
            .find(|x| x.owner == old.owner)
            .unwrap();
        *owner = old;
    }
    inverse
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .pop();
    if inverse != *prior {
        return Err("whole recipe inverse failed".into());
    }
    Ok(next)
}
