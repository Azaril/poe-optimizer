//! Authenticated finite actor data acquisition, without build/UI/evaluator state.
//! The pinned constructors run only offline in an isolated, bounded Lua state.
//! No source program or recursive legacy table crosses the owned DTO boundary.
use crate::{game_data, owned_item_bases::BoundedBytes, source};
use mlua::{
    Function, HookTriggers, Lua, LuaOptions, MultiValue, StdLib, Table, UserData, Value, VmState,
};
use poe_optimizer_import::{
    owned_actor_baselines::*,
    owned_mapping::{ExternalSourceSystem, SourceFilePin, SourcePin},
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use thiserror::Error;

const DATA: &str = "src/Modules/Data.lua";
const MINIONS: &str = "src/Data/Minions.lua";
const SPECTRES: &str = "src/Data/Spectres.lua";
const MISC: &str = "src/Data/Misc.lua";
const PATHS: [&str; 4] = [DATA, MINIONS, SPECTRES, MISC];
const HOOK_INTERVAL: usize = 100;

#[derive(Clone, Copy, Debug)]
pub struct ActorBaselineExportLimits {
    pub max_source_bytes: usize,
    pub max_lua_bytes: usize,
    pub max_instructions: usize,
    pub max_profiles: usize,
    pub max_fields: usize,
    pub max_list_entries: usize,
    pub max_total_entries: usize,
    pub max_text_bytes: usize,
    pub max_catalog_bytes: usize,
}
impl Default for ActorBaselineExportLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 4 * 1024 * 1024,
            max_lua_bytes: 64 * 1024 * 1024,
            max_instructions: 2_000_000,
            max_profiles: 4096,
            max_fields: 128,
            max_list_entries: 1024,
            max_total_entries: 250_000,
            max_text_bytes: 1024,
            max_catalog_bytes: 8 * 1024 * 1024,
        }
    }
}
impl ActorBaselineExportLimits {
    fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (name, value, maximum) in [
            ("source bytes", self.max_source_bytes, hard.max_source_bytes),
            ("Lua bytes", self.max_lua_bytes, hard.max_lua_bytes),
            ("instructions", self.max_instructions, hard.max_instructions),
            ("profiles", self.max_profiles, hard.max_profiles),
            ("fields", self.max_fields, hard.max_fields),
            ("list entries", self.max_list_entries, hard.max_list_entries),
            (
                "total entries",
                self.max_total_entries,
                hard.max_total_entries,
            ),
            ("text bytes", self.max_text_bytes, hard.max_text_bytes),
            (
                "catalog bytes",
                self.max_catalog_bytes,
                hard.max_catalog_bytes,
            ),
        ] {
            if value == 0 || value > maximum {
                return Err(invalid(format!("invalid {name} limit")));
            }
        }
        Ok(())
    }
}
#[derive(Debug, Error)]
pub enum ActorBaselineExportError {
    #[error("owned actor-baseline export: {0}")]
    Invalid(String),
    #[error(transparent)]
    Source(#[from] source::SourceError),
    #[error(transparent)]
    Lua(#[from] mlua::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, ActorBaselineExportError>;
fn invalid(message: impl Into<String>) -> ActorBaselineExportError {
    ActorBaselineExportError::Invalid(message.into())
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActorBaselineExportEvidence {
    pub schema_version: u32,
    pub upstream_revision: String,
    pub source_manifest_sha256: String,
    pub extractor_sha256: String,
    pub extraction_source_files: Vec<SourceFilePin>,
    pub catalog_sha256: String,
    pub profiles: usize,
    pub child_skills: usize,
    pub unconverted_modifiers: usize,
    pub zero_attack_times: usize,
    pub numeric_field_names: Vec<String>,
    pub boolean_field_names: Vec<String>,
}
/// Hashes are audit evidence, not an authentication token. Only verified source
/// acquisition can construct this private result and authorize content checks.
#[derive(Debug)]
pub struct AuthenticatedActorBaselineExport {
    catalog: ActorBaselineCatalog,
    catalog_bytes: Vec<u8>,
    evidence: ActorBaselineExportEvidence,
}
impl AuthenticatedActorBaselineExport {
    pub fn catalog(&self) -> &ActorBaselineCatalog {
        &self.catalog
    }
    /// Exact identity-bearing representation: pretty JSON plus one LF.
    pub fn catalog_bytes(&self) -> &[u8] {
        &self.catalog_bytes
    }
    pub fn evidence(&self) -> &ActorBaselineExportEvidence {
        &self.evidence
    }
    pub fn validate_catalog(&self, candidate: &ActorBaselineCatalog) -> Result<()> {
        if catalog_bytes(candidate, self.catalog_bytes.len())? != self.catalog_bytes {
            return Err(invalid(
                "catalog differs from independently authenticated extraction",
            ));
        }
        Ok(())
    }
}

pub fn export_owned_actor_baselines(
    root: &Path,
    limits: ActorBaselineExportLimits,
) -> Result<AuthenticatedActorBaselineExport> {
    limits.validate()?;
    let source_manifest_sha256 = source::verify(root)?;
    let mut sources = BTreeMap::new();
    let mut pins = Vec::new();
    let mut source_bytes = 0usize;
    for path in PATHS {
        let text = source::read_verified_text(root, path)?;
        source_bytes = source_bytes
            .checked_add(text.len())
            .ok_or_else(|| invalid("source byte count overflow"))?;
        if source_bytes > limits.max_source_bytes {
            return Err(invalid("source byte limit"));
        }
        pins.push(SourceFilePin {
            path: path.into(),
            sha256: game_data::hash(text.as_bytes()),
        });
        sources.insert(path, text);
    }
    pins.sort_by(|a, b| a.path.cmp(&b.path));
    let pin = SourcePin {
        system: ExternalSourceSystem::PathOfBuilding2,
        revision: source::UPSTREAM_REVISION.into(),
        files: pins.clone(),
    };
    let catalog = extract(&sources, pin, limits)?;
    let catalog_bytes = catalog_bytes(&catalog, limits.max_catalog_bytes)?;
    let mut numeric = BTreeSet::new();
    let mut boolean = BTreeSet::new();
    for row in &catalog.profiles {
        for (key, value) in [
            ("attackTime", row.attack_time),
            ("damage", row.damage_scale),
            ("damageSpread", row.damage_spread),
            ("critChance", row.critical_chance),
            ("attackRange", row.attack_range),
        ] {
            if value.is_some() {
                numeric.insert(key.to_owned());
            }
        }
        for (key, value) in [
            ("hostile", row.hostile),
            (
                "baseDamageIgnoresAttackSpeed",
                row.base_damage_ignores_attack_speed,
            ),
        ] {
            if value.is_some() {
                boolean.insert(key.to_owned());
            }
        }
        for (key, value) in &row.extra_facts {
            match value {
                ActorScalarFact::Number(_) => {
                    numeric.insert(key.clone());
                }
                ActorScalarFact::Boolean(_) => {
                    boolean.insert(key.clone());
                }
                ActorScalarFact::Text(_) => {}
            }
        }
    }
    let evidence = ActorBaselineExportEvidence {
        schema_version: 1,
        upstream_revision: source::UPSTREAM_REVISION.into(),
        source_manifest_sha256,
        extractor_sha256: game_data::hash(
            format!(
                "owned-actor-baseline-export-v1\n{}\n{}\n{}",
                include_str!("owned_actor_baselines.rs").replace("\r\n", "\n"),
                include_str!("owned_item_bases.rs").replace("\r\n", "\n"),
                include_str!("../../poe-optimizer-import/src/owned_actor_baselines.rs")
                    .replace("\r\n", "\n"),
            )
            .as_bytes(),
        ),
        extraction_source_files: pins,
        catalog_sha256: game_data::hash(&catalog_bytes),
        profiles: catalog.profiles.len(),
        child_skills: catalog.profiles.iter().map(|p| p.child_skills.len()).sum(),
        unconverted_modifiers: catalog
            .profiles
            .iter()
            .filter_map(|p| p.unconverted_modifiers.as_ref())
            .map(Vec::len)
            .sum(),
        zero_attack_times: catalog
            .profiles
            .iter()
            .filter(|p| p.attack_time == Some(0.0))
            .count(),
        numeric_field_names: numeric.into_iter().collect(),
        boolean_field_names: boolean.into_iter().collect(),
    };
    Ok(AuthenticatedActorBaselineExport {
        catalog,
        catalog_bytes,
        evidence,
    })
}

fn bounded_lua(limits: ActorBaselineExportLimits) -> Result<Lua> {
    let lua = Lua::new_with(StdLib::JIT, LuaOptions::default())?;
    if lua.used_memory() > limits.max_lua_bytes {
        return Err(invalid("Lua memory limit below initialization"));
    }
    lua.set_memory_limit(limits.max_lua_bytes)?;
    lua.load("jit.off(); jit.flush(); jit=nil").exec()?;
    let count = Arc::new(AtomicUsize::new(0));
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(HOOK_INTERVAL as u32),
        move |_, _| {
            if count.fetch_add(HOOK_INTERVAL, Ordering::Relaxed) + HOOK_INTERVAL
                >= limits.max_instructions
            {
                return Err(mlua::Error::RuntimeError(
                    "actor baseline instruction limit".into(),
                ));
            }
            Ok(VmState::Continue)
        },
    )?;
    Ok(lua)
}
#[derive(Clone)]
struct ModifierToken(ActorModifierCoverage);
impl UserData for ModifierToken {}
fn constructor(
    lua: &Lua,
    kind: ActorModifierConstructor,
    limits: ActorBaselineExportLimits,
    budget: EntryBudget,
) -> Result<Function> {
    Ok(lua.create_function(move |_, args: MultiValue| {
        budget.charge(1).map_err(mlua::Error::external)?;
        let name = text(args.front().cloned().unwrap_or(Value::Nil), limits)
            .map_err(mlua::Error::external)?;
        let operation = match kind {
            ActorModifierConstructor::Modifier => {
                text(args.get(1).cloned().unwrap_or(Value::Nil), limits)
                    .map_err(mlua::Error::external)?
            }
            ActorModifierConstructor::Flag => "FLAG".into(),
        };
        // Remaining arguments may include nested tags. They are deliberately
        // opaque: none are evaluated, interpreted, or asserted converted.
        Ok(ModifierToken(ActorModifierCoverage {
            constructor: kind,
            name,
            operation,
        }))
    })?)
}
fn extract(
    sources: &BTreeMap<&str, String>,
    source: SourcePin,
    limits: ActorBaselineExportLimits,
) -> Result<ActorBaselineCatalog> {
    let lua = bounded_lua(limits)?;
    let mut budget = EntryBudget(Arc::new(AtomicUsize::new(limits.max_total_entries)));
    let modifier = constructor(
        &lua,
        ActorModifierConstructor::Modifier,
        limits,
        budget.clone(),
    )?;
    let flag = constructor(&lua, ActorModifierConstructor::Flag, limits, budget.clone())?;
    let mut rows = BTreeMap::new();
    for path in [MINIONS, SPECTRES] {
        let function: Function = lua
            .load(
                sources
                    .get(path)
                    .ok_or_else(|| invalid("missing profile source"))?,
            )
            .set_name(path)
            .set_mode(mlua::chunk::ChunkMode::Text)
            .set_environment(lua.create_table()?)
            .eval()?;
        let profiles: Table = function.call((modifier.clone(), flag.clone()))?;
        plain(&profiles)?;
        let mut module_count = 0;
        for pair in profiles.pairs::<Value, Value>() {
            charge(&mut budget, 1)?;
            module_count += 1;
            if module_count > limits.max_profiles {
                return Err(invalid("profile count limit"));
            }
            let (key, value) = pair?;
            let key = text(key, limits)?;
            let Value::Table(table) = value else {
                return Err(invalid("actor profile is not a table"));
            };
            let mut row = profile(key.clone(), path, table, limits, &mut budget)?;
            if path == SPECTRES {
                // Pinned Data.lua's only profile-field mutation during final
                // merge. This remains an unconverted finite string fact.
                row.extra_facts.insert(
                    "limit".into(),
                    ActorScalarFact::Text("ActiveSpectreLimit".into()),
                );
                if !row.unconverted_fields.iter().any(|f| f == "limit") {
                    row.unconverted_fields.push("limit".into());
                    row.unconverted_fields.sort();
                }
            }
            // Preserve the source constructor's final assignment and merge
            // precedence, including intentional duplicate Spectre keys.
            rows.insert(key, row);
            if rows.len() > limits.max_profiles {
                return Err(invalid("profile count limit"));
            }
        }
    }
    if rows.is_empty() {
        return Err(invalid("actor catalog has no profiles"));
    }
    let misc: Table = lua
        .load(
            sources
                .get(MISC)
                .ok_or_else(|| invalid("missing level source"))?,
        )
        .set_name(MISC)
        .set_mode(mlua::chunk::ChunkMode::Text)
        .set_environment(lua.create_table()?)
        .eval()?;
    plain(&misc)?;
    let levels = list(misc.raw_get("minionLevelTable")?, limits, &mut budget)?;
    let summon_levels = levels
        .into_iter()
        .map(|v| {
            let value = number(v)?;
            if value < 1.0 || value > u32::MAX as f64 || value.fract() != 0.0 {
                return Err(invalid("invalid summon actor level"));
            }
            Ok(value as u32)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut curve = |name| -> Result<ActorDamageLevelTable> {
        let rows = list(misc.raw_get(name)?, limits, &mut budget)?
            .into_iter()
            .map(number)
            .collect::<Result<Vec<_>>>()?;
        if rows.is_empty() {
            return Err(invalid("empty damage curve"));
        }
        Ok(ActorDamageLevelTable {
            first_level: 1,
            rows,
        })
    };
    if summon_levels.is_empty() {
        return Err(invalid("empty summon level table"));
    }
    Ok(ActorBaselineCatalog {
        schema_version: OWNED_ACTOR_BASELINE_VERSION,
        source,
        profiles: rows.into_values().collect(),
        summon_levels: ActorSummonLevelTable {
            first_level: 1,
            rows: summon_levels,
        },
        allied_damage: curve("monsterAllyDamageTable")?,
        hostile_damage: curve("monsterDamageTable")?,
    })
}
fn profile(
    key: String,
    module: &str,
    table: Table,
    limits: ActorBaselineExportLimits,
    budget: &mut EntryBudget,
) -> Result<ActorBaselineProfile> {
    plain(&table)?;
    let mut fields = BTreeMap::new();
    for entry in table.pairs::<Value, Value>() {
        charge(budget, 1)?;
        if fields.len() >= limits.max_fields {
            return Err(invalid("profile field count limit"));
        }
        let (key, value) = entry?;
        fields.insert(text(key, limits)?, value);
    }
    let mut numeric = |name| optional(fields.remove(name).unwrap_or(Value::Nil), number);
    let attack_time = numeric("attackTime")?;
    let damage_scale = numeric("damage")?;
    let damage_spread = numeric("damageSpread")?;
    let critical_chance = numeric("critChance")?;
    let attack_range = numeric("attackRange")?;
    let name = text(fields.remove("name").unwrap_or(Value::Nil), limits)?;
    let weapon_family = optional(fields.remove("weaponType1").unwrap_or(Value::Nil), |v| {
        text(v, limits)
    })?;
    let hostile = optional(fields.remove("hostile").unwrap_or(Value::Nil), boolean)?;
    let base_damage_ignores_attack_speed = optional(
        fields
            .remove("baseDamageIgnoresAttackSpeed")
            .unwrap_or(Value::Nil),
        boolean,
    )?;
    let child_skills = list(
        fields.remove("skillList").unwrap_or(Value::Nil),
        limits,
        budget,
    )?
    .into_iter()
    .map(|v| text(v, limits))
    .collect::<Result<Vec<_>>>()?;
    let unconverted_modifiers = optional(fields.remove("modList").unwrap_or(Value::Nil), |v| {
        list(v, limits, budget)?
            .into_iter()
            .map(|v| {
                let Value::UserData(token) = v else {
                    return Err(invalid("unrecognized modifier coverage token"));
                };
                Ok(token.borrow::<ModifierToken>()?.0.clone())
            })
            .collect::<Result<Vec<_>>>()
    })?;
    let mut unconverted_fields: Vec<_> = fields.keys().cloned().collect();
    let unconverted_flags = optional(fields.remove("extraFlags").unwrap_or(Value::Nil), |v| {
        let Value::Table(table) = v else {
            return Err(invalid("actor extra flags are not a table"));
        };
        plain(&table)?;
        let mut flags = BTreeMap::new();
        for entry in table.pairs::<Value, Value>() {
            charge(budget, 1)?;
            if flags.len() >= limits.max_fields {
                return Err(invalid("extra flag count limit"));
            }
            let (key, value) = entry?;
            flags.insert(text(key, limits)?, boolean(value)?);
        }
        Ok(flags)
    })?;
    let mut extra_facts = BTreeMap::new();
    for (key, value) in fields {
        let value = match value {
            Value::Boolean(v) => ActorScalarFact::Boolean(v),
            Value::Integer(_) | Value::Number(_) => ActorScalarFact::Number(number(value)?),
            Value::String(_) => ActorScalarFact::Text(text(value, limits)?),
            Value::Table(t) => {
                plain(&t)?;
                continue;
            }
            _ => return Err(invalid("unconverted field has an unsupported value kind")),
        };
        extra_facts.insert(key, value);
    }
    unconverted_fields.sort();
    Ok(ActorBaselineProfile {
        key,
        name,
        source_module: module.into(),
        attack_time,
        damage_scale,
        damage_spread,
        critical_chance,
        attack_range,
        weapon_family,
        hostile,
        base_damage_ignores_attack_speed,
        child_skills,
        extra_facts,
        unconverted_fields,
        unconverted_modifiers,
        unconverted_flags,
    })
}
fn optional<T>(value: Value, f: impl FnOnce(Value) -> Result<T>) -> Result<Option<T>> {
    if matches!(value, Value::Nil) {
        Ok(None)
    } else {
        f(value).map(Some)
    }
}
fn number(value: Value) -> Result<f64> {
    let value = match value {
        Value::Integer(v) => v as f64,
        Value::Number(v) => v,
        _ => return Err(invalid("expected a numeric actor fact")),
    };
    if !value.is_finite() {
        return Err(invalid("actor fact must be finite"));
    }
    Ok(value)
}
fn boolean(value: Value) -> Result<bool> {
    if let Value::Boolean(v) = value {
        Ok(v)
    } else {
        Err(invalid("expected a Boolean actor fact"))
    }
}
fn text(value: Value, limits: ActorBaselineExportLimits) -> Result<String> {
    let Value::String(value) = value else {
        return Err(invalid("expected actor text"));
    };
    if value.as_bytes().is_empty() || value.as_bytes().len() > limits.max_text_bytes {
        return Err(invalid("actor text byte limit"));
    }
    let value = value.to_str()?.to_owned();
    if value.contains('\0') {
        return Err(invalid("actor text contains NUL"));
    }
    Ok(value)
}
fn plain(table: &Table) -> Result<()> {
    if table.metatable().is_some() {
        return Err(invalid("actor tables cannot have metatables"));
    }
    Ok(())
}
#[derive(Clone)]
struct EntryBudget(Arc<AtomicUsize>);
impl EntryBudget {
    fn charge(&self, n: usize) -> Result<()> {
        self.0
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |left| {
                left.checked_sub(n)
            })
            .map_err(|_| invalid("total entry limit"))?;
        Ok(())
    }
}
fn charge(left: &mut EntryBudget, n: usize) -> Result<()> {
    left.charge(n)
}
fn list(
    value: Value,
    limits: ActorBaselineExportLimits,
    budget: &mut EntryBudget,
) -> Result<Vec<Value>> {
    let Value::Table(table) = value else {
        return Err(invalid("expected an actor list"));
    };
    plain(&table)?;
    let mut rows = BTreeMap::new();
    for entry in table.pairs::<Value, Value>() {
        charge(budget, 1)?;
        if rows.len() >= limits.max_list_entries {
            return Err(invalid("actor list entry limit"));
        }
        let (key, value) = entry?;
        let key = number(key)?;
        if key < 1.0 || key > limits.max_list_entries as f64 || key.fract() != 0.0 {
            return Err(invalid("actor list key is not a bounded positive integer"));
        }
        rows.insert(key as usize, value);
    }
    if rows.keys().copied().ne(1..=rows.len()) {
        return Err(invalid("actor list is sparse"));
    }
    Ok(rows.into_values().collect())
}
fn catalog_bytes(catalog: &ActorBaselineCatalog, limit: usize) -> Result<Vec<u8>> {
    // JSON serializes nonfinite Rust floats as null. Reject them first so an
    // invalid Some(NaN) can never authenticate as an absent optional fact.
    for profile in &catalog.profiles {
        if [
            profile.attack_time,
            profile.damage_scale,
            profile.damage_spread,
            profile.critical_chance,
            profile.attack_range,
        ]
        .into_iter()
        .flatten()
        .any(|v| !v.is_finite())
            || profile
                .extra_facts
                .values()
                .any(|v| matches!(v, ActorScalarFact::Number(n) if !n.is_finite()))
        {
            return Err(invalid("candidate actor fact must be finite"));
        }
    }
    if catalog
        .allied_damage
        .rows
        .iter()
        .chain(&catalog.hostile_damage.rows)
        .any(|v| !v.is_finite())
    {
        return Err(invalid("candidate actor curve must be finite"));
    }
    let mut output = BoundedBytes {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer_pretty(&mut output, catalog)?;
    output
        .write_all(b"\n")
        .map_err(|_| invalid("catalog byte limit"))?;
    Ok(output.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inputs(minions: &str) -> BTreeMap<&'static str, String> {
        BTreeMap::from([
            (MINIONS, minions.into()),
            (SPECTRES, "return function(mod, flag) return {} end".into()),
            (MISC, "return {minionLevelTable={2},monsterAllyDamageTable={3.5},monsterDamageTable={8.5}}".into()),
        ])
    }
    fn pin() -> SourcePin {
        SourcePin {
            system: ExternalSourceSystem::PathOfBuilding2,
            revision: "test".into(),
            files: vec![],
        }
    }
    #[test]
    fn opaque_nested_modifier_tags_and_missing_empty_false_zero_are_preserved() {
        let sources = inputs(
            "return function(mod,flag) return { A={name='actor',attackTime=0,hostile=false,skillList={},modList={},extraFlags={}}, B={name='actor',skillList={'b','a'},life=0,custom=false,modList={mod('Nested','FLAG',30,0,0,{type='GlobalEffect'}),flag('Flagged',{nested={1,2}})}} } end",
        );
        let catalog = extract(&sources, pin(), Default::default()).unwrap();
        let a = &catalog.profiles[0];
        let b = &catalog.profiles[1];
        assert_eq!(a.attack_time, Some(0.0));
        assert_eq!(a.hostile, Some(false));
        assert_eq!(a.unconverted_modifiers, Some(vec![]));
        assert_eq!(a.unconverted_flags, Some(BTreeMap::new()));
        assert_eq!(b.attack_time, None);
        assert_eq!(b.hostile, None);
        assert_eq!(b.unconverted_flags, None);
        assert_eq!(b.child_skills, ["b", "a"]);
        assert_eq!(b.extra_facts["life"], ActorScalarFact::Number(0.0));
        assert_eq!(b.extra_facts["custom"], ActorScalarFact::Boolean(false));
        assert_eq!(
            b.unconverted_modifiers.as_ref().unwrap()[1].constructor,
            ActorModifierConstructor::Flag
        );
    }
    #[test]
    fn bounded_constructor_rejects_sparse_lists_wrong_types_and_nonfinite_facts() {
        for body in [
            "{name='a',skillList={[2]='b'}}",
            "{name='a',skillList={},attackTime='0'}",
            "{name='a',skillList={},hostile=1}",
            "{name='a',skillList={},damage=0/0}",
            "{name='a',skillList={},life=1/0}",
            "{name='a',skillList={},modList={{name='fake'}}}",
            "{name='a',skillList={},extraFlags={foo=1}}",
        ] {
            let sources = inputs(&format!(
                "return function(mod,flag) return {{ A={body} }} end"
            ));
            assert!(
                extract(&sources, pin(), Default::default()).is_err(),
                "{body}"
            );
        }
    }
    #[test]
    fn empty_environment_and_shared_instruction_memory_limits_are_enforced() {
        let limits = ActorBaselineExportLimits {
            max_instructions: 1000,
            ..Default::default()
        };
        let sources = inputs("return function() while true do end end");
        assert!(extract(&sources, pin(), limits).is_err());
        let discarded = inputs(
            "return function(mod,flag) for i=1,100 do mod('discarded','BASE',1) end return {A={name='a',skillList={}}} end",
        );
        assert!(
            extract(
                &discarded,
                pin(),
                ActorBaselineExportLimits {
                    max_total_entries: 10,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(extract(&discarded, pin(), Default::default()).is_ok());
        let sources = inputs("return function() return os.execute('anything') end");
        assert!(extract(&sources, pin(), Default::default()).is_err());
        assert!(
            bounded_lua(ActorBaselineExportLimits {
                max_lua_bytes: 1,
                ..Default::default()
            })
            .is_err()
        );
        let sources = inputs("return function() return {A={name='a',skillList={'x','y'}}} end");
        assert!(
            extract(
                &sources,
                pin(),
                ActorBaselineExportLimits {
                    max_list_entries: 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            extract(
                &sources,
                pin(),
                ActorBaselineExportLimits {
                    max_fields: 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}
