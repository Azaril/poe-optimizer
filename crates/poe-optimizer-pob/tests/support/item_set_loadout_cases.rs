//! Direct calls on the unchanged, already imported source owners.
//! Each call has its own observer lifetime; no native parity is asserted here.
use mlua::{Function, Lua, LuaSerdeExt, MultiValue, Table, Value};
use serde_json::{Value as Json, json};
use std::{collections::BTreeSet, io};

const MAX_CASES: usize = 4096;
const MAX_NAMES: usize = 1024;
const MAX_NAME_BYTES: usize = 262144;
const MAX_INPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_JSON_BYTES: usize = 128 * 1024 * 1024;
const MAX_RETURNS: usize = 64;
const MAX_ACTIVATION_TARGETS: usize = 4;

fn failure(message: impl Into<String>) -> mlua::Error {
    mlua::Error::RuntimeError(format!("loadout direct harness: {}", message.into()))
}

#[derive(Default)]
struct Budget {
    cases: usize,
    input_bytes: usize,
    json_bytes: usize,
}
impl Budget {
    fn name(&mut self, value: &mlua::LuaString) -> mlua::Result<String> {
        let n = value.as_bytes().len();
        self.input_bytes = self
            .input_bytes
            .checked_add(n)
            .ok_or_else(|| failure("input overflow"))?;
        if n > MAX_NAME_BYTES || self.input_bytes > MAX_INPUT_BYTES {
            return Err(failure("lookup input byte bound"));
        }
        Ok(value.to_str()?.to_owned())
    }
    fn retain(&mut self, value: &Json) -> mlua::Result<()> {
        struct Count(usize);
        impl io::Write for Count {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.0 = self
                    .0
                    .checked_sub(bytes.len())
                    .ok_or_else(|| io::Error::other("direct JSON byte bound"))?;
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let remaining = MAX_JSON_BYTES.saturating_sub(self.json_bytes);
        let mut count = Count(remaining);
        serde_json::to_writer(&mut count, value).map_err(mlua::Error::external)?;
        self.json_bytes += remaining - count.0;
        Ok(())
    }
}

fn pack(lua: &Lua, values: &MultiValue) -> mlua::Result<Table> {
    if values.len() > MAX_RETURNS {
        return Err(failure("actual return arity bound"));
    }
    let out = lua.create_table()?;
    out.raw_set("n", values.len())?;
    for (index, value) in values.iter().enumerate() {
        out.raw_set(index + 1, value.clone())?;
    }
    Ok(out)
}

// Only errors from the direct original Function invocation can enter here.
// A successful finish additionally proves no sticky observer failure occurred.
fn source_error(error: &mlua::Error) -> bool {
    let mut current = error;
    for _ in 0..16 {
        match current {
            mlua::Error::RuntimeError(message) => {
                return message.contains(".lua:")
                    && !message.contains("item-set lifecycle:")
                    && !message.contains("loadout direct harness:")
                    && !message.contains("stack overflow")
                    && !message.contains("instruction limit")
                    && !message.contains("deadline")
                    && !message.contains("memory allocation");
            }
            mlua::Error::CallbackError { cause, .. } => current = cause,
            _ => return false,
        }
    }
    false
}

fn identities(first: &Table, current: &Table) -> mlua::Result<Json> {
    let first_n: usize = first.raw_get("n")?;
    let current_n: usize = current.raw_get("n")?;
    let mut positions = Vec::new();
    for index in 1..=first_n.max(current_n) {
        let old: Value = first.raw_get(index)?;
        let new: Value = current.raw_get(index)?;
        positions.push(json!({
            "position":index,
            "first_is_table":matches!(&old, Value::Table(_)),
            "current_is_table":matches!(&new, Value::Table(_)),
            "same_table":match (&old,&new) {
                (Value::Table(a),Value::Table(b))=>Some(a.to_pointer()==b.to_pointer()),
                _=>None,
            }
        }));
    }
    Ok(json!({"first_n":first_n,"current_n":current_n,"positions":positions}))
}

struct Completed {
    case: Json,
    observation: Json,
    result: Option<Table>,
}

#[allow(clippy::too_many_arguments)]
fn call(
    lua: &Lua,
    module: &Table,
    observed: bool,
    function: &Function,
    args: MultiValue,
    label: &str,
    operation: &str,
    origin: Json,
    argument: Json,
    first: Option<&Table>,
    argument_pack: Option<&Table>,
    budget: &mut Budget,
) -> mlua::Result<Completed> {
    if budget.cases >= MAX_CASES {
        return Err(failure("direct call count bound"));
    }
    budget.cases += 1;
    let options = lua.create_table()?;
    options.raw_set("loadouts", true)?;
    let capture: Table = module
        .raw_get::<Function>("start")?
        .call((observed, options))?;
    let finish: Function = capture.raw_get("finish")?;
    // Keep call errors as data until observer verification/cleanup has succeeded.
    // Snapshot/serialization failures never become source error observations.
    let work = (|| {
        let snapshot: Function = capture.raw_get("loadout_snapshot")?;
        let before: Table = snapshot.call(None::<Table>)?;
        let before_snapshot = super::loadout_snapshot_json(lua, before)?;
        let argument_before = argument_pack
            .map(|pack| snapshot.call::<Table>(pack.clone()))
            .transpose()?
            .map(|state| super::loadout_snapshot_json(lua, state))
            .transpose()?;
        let invoked = function.call::<MultiValue>(args);
        let result = match &invoked {
            Ok(values) => Some(pack(lua, values)?),
            Err(_) => None,
        };
        let state: Table = snapshot.call(result.clone())?;
        let mut row = json!({
            "label":label,"operation":operation,"origin":origin,"argument":argument,
            "before_snapshot":before_snapshot,
            "snapshot":super::loadout_snapshot_json(lua,state)?,
        });
        if let Some(before) = argument_before {
            row["argument_before_snapshot"] = before;
            let state: Table = snapshot.call(argument_pack.expect("retained argument").clone())?;
            row["argument_after_snapshot"] = super::loadout_snapshot_json(lua, state)?;
        }
        if let Some(first) = first {
            let state: Table = snapshot.call(first.clone())?;
            row["retained_first_result_snapshot"] = super::loadout_snapshot_json(lua, state)?;
            if let Some(current) = &result {
                row["first_result_identity"] = identities(first, current)?;
            }
        }
        Ok::<_, mlua::Error>((invoked, result, row))
    })();
    // Always finish, including after original errors and snapshot/bound failures.
    let report = finish.call::<Table>(());
    let (invoked, result, mut row) = match (work, report.as_ref()) {
        (Err(error), Err(cleanup)) => {
            return Err(failure(format!(
                "call/snapshot failed: {error}; cleanup failed: {cleanup}"
            )));
        }
        (Err(error), _) => return Err(error),
        (Ok(_), Err(error)) => return Err(error.clone()),
        (Ok(value), Ok(_)) => value,
    };
    let report = report?;
    let status = match invoked {
        Ok(values) => {
            let incomplete: Table = report.raw_get("incomplete_calls")?;
            if incomplete.raw_len() != 0 {
                return Err(failure("successful call has incomplete observer entries"));
            }
            json!({"kind":"returned","actual_return_count":values.len()})
        }
        Err(error) if source_error(&error) => {
            let message = error.to_string();
            if message.len() > MAX_NAME_BYTES {
                return Err(failure("source error text bound"));
            }
            json!({"kind":"source_error","error":message})
        }
        Err(error) => return Err(error),
    };
    row["status"] = status;
    let mut observation = json!({"label":label});
    for key in [
        "events",
        "functions",
        "incomplete_calls",
        "scope",
        "bounds",
        "retained_objects",
        "rows",
        "text_bytes",
    ] {
        observation[key] = lua.from_value(report.raw_get(key)?)?;
    }
    observation["loadout_states"] = super::loadout_states_json(lua, &report)?;
    observation["finite_post_loadouts"] =
        super::loadout_snapshot_json(lua, report.raw_get("finite_post_loadouts")?)?;
    budget.retain(&row)?;
    budget.retain(&observation)?;
    Ok(Completed {
        case: row,
        observation,
        result,
    })
}

struct Name {
    value: mlua::LuaString,
    text: String,
    origin: Json,
}

fn names(
    table: &Table,
    origin: &str,
    out: &mut Vec<Name>,
    budget: &mut Budget,
) -> mlua::Result<()> {
    // This is the actual positive ipairs prefix, not a sorted/deduplicated list.
    for index in 1..=MAX_NAMES + 1 {
        let value: Value = table.raw_get(index)?;
        if matches!(value, Value::Nil) {
            return Ok(());
        }
        if out.len() >= MAX_NAMES {
            return Err(failure("lookup occurrence bound"));
        }
        let Value::String(value) = value else {
            return Err(failure("non-string original display occurrence"));
        };
        let text = budget.name(&value)?;
        out.push(Name {
            value,
            text,
            origin: json!({"kind":origin,"index":index}),
        });
    }
    Err(failure("lookup prefix bound"))
}

fn known_titles(
    build: &Table,
    known: &mut BTreeSet<String>,
    budget: &mut Budget,
) -> mlua::Result<()> {
    for (owner_key, map_key, order_key) in [
        ("itemsTab", "itemSets", "itemSetOrderList"),
        ("skillsTab", "skillSets", "skillSetOrderList"),
        ("configTab", "configSets", "configSetOrderList"),
    ] {
        let owner: Table = build.raw_get(owner_key)?;
        let map: Table = owner.raw_get(map_key)?;
        let order: Table = owner.raw_get(order_key)?;
        for index in 1..=MAX_NAMES + 1 {
            let key: Value = order.raw_get(index)?;
            if matches!(key, Value::Nil) {
                break;
            }
            if index > MAX_NAMES {
                return Err(failure("named set prefix bound"));
            }
            let row: Table = map.raw_get(key)?;
            if let Value::String(title) = row.raw_get::<Value>("title")? {
                known.insert(budget.name(&title)?);
            }
        }
    }
    Ok(())
}

const ID_FIELDS: [&str; 4] = ["specId", "itemSetId", "skillSetId", "configSetId"];
const DOMAINS: [&str; 4] = ["tree", "items", "skills", "config"];

fn id_value(value: Value) -> mlua::Result<Json> {
    Ok(match value {
        Value::Nil => json!({"kind":"nil"}),
        Value::Boolean(value) => json!({"kind":"boolean","value":value}),
        Value::Integer(value) => json!({"kind":"number","value":value as f64}),
        Value::Number(value) if value.is_finite() => json!({"kind":"number","value":value}),
        Value::String(value) if value.as_bytes().len() <= MAX_NAME_BYTES => {
            json!({"kind":"string","value":value.to_str()?.to_owned()})
        }
        _ => return Err(failure("unrepresented activation ID value")),
    })
}
fn truthy_id(value: &Json) -> bool {
    value["kind"] != "nil" && !(value["kind"] == "boolean" && value["value"] == false)
}
fn ids(table: &Table) -> mlua::Result<Vec<Json>> {
    ID_FIELDS
        .into_iter()
        .map(|field| id_value(table.raw_get(field)?))
        .collect()
}
fn active_ids(build: &Table) -> mlua::Result<Vec<Json>> {
    [
        ("treeTab", "activeSpec"),
        ("itemsTab", "activeItemSetId"),
        ("skillsTab", "activeSkillSetId"),
        ("configTab", "activeConfigSetId"),
    ]
    .into_iter()
    .map(|(owner, field)| id_value(build.raw_get::<Table>(owner)?.raw_get(field)?))
    .collect()
}
fn named_ids(values: &[Json]) -> Json {
    Json::Object(
        ID_FIELDS
            .into_iter()
            .zip(values.iter().cloned())
            .map(|(key, value)| (key.into(), value))
            .collect(),
    )
}
fn changed_domains(requested: &[Json], active: &[Json]) -> Vec<&'static str> {
    DOMAINS
        .into_iter()
        .enumerate()
        .filter_map(|(i, name)| (requested[i] != active[i]).then_some(name))
        .collect()
}
struct RetainedLookup {
    label: String,
    pack: Table,
    ids: Vec<Json>,
}
fn retained_lookup(done: &Completed) -> mlua::Result<Option<RetainedLookup>> {
    let Some(pack) = &done.result else {
        return Ok(None);
    };
    if pack.raw_get::<usize>("n")? != 1 {
        return Err(failure(
            "lookup activation input is not one original return",
        ));
    }
    match pack.raw_get::<Value>(1)? {
        Value::Nil => Ok(None),
        Value::Table(table) => Ok(Some(RetainedLookup {
            label: done.case["label"]
                .as_str()
                .ok_or_else(|| failure("lookup label"))?
                .into(),
            pack: pack.clone(),
            ids: ids(&table)?,
        })),
        _ => Err(failure("lookup activation input is not a table or nil")),
    }
}

fn activation_history(
    lua: &Lua,
    module: &Table,
    observed: bool,
    build: &Table,
    lookups: &[RetainedLookup],
    budget: &mut Budget,
) -> mlua::Result<Json> {
    let function: Function = build.raw_get("SetActiveLoadout")?;
    let initial = active_ids(build)?;
    let mut complete = Vec::<&RetainedLookup>::new();
    let mut complete_occurrences = 0;
    for lookup in lookups {
        if lookup.ids.iter().all(truthy_id) {
            complete_occurrences += 1;
            if !complete.iter().any(|old| old.ids == lookup.ids) {
                complete.push(lookup);
            }
        }
    }
    let current = complete.iter().position(|lookup| lookup.ids == initial);
    let mut selected = Vec::new();
    if let Some(index) = current {
        selected.push(complete[index]);
    }
    for lookup in &complete {
        if lookup.ids != initial && selected.len() < MAX_ACTIVATION_TARGETS {
            selected.push(*lookup);
        }
    }
    let partial = lookups.iter().find(|lookup| !truthy_id(&lookup.ids[0]));
    let available_domains: BTreeSet<_> = complete
        .iter()
        .flat_map(|lookup| changed_domains(&lookup.ids, &initial))
        .collect();
    let mut gaps = Vec::new();
    for domain in DOMAINS {
        if !available_domains.contains(domain) {
            gaps.push(format!(
                "No complete natural lookup changes {domain} from the initial active ID."
            ));
        }
    }
    if current.is_none() {
        gaps.push("No complete natural lookup matches all initial active IDs.".into());
    }
    if partial.is_none() {
        gaps.push("No natural returned table lacks a truthy specId.".into());
    }
    if complete.len() > selected.len() {
        gaps.push("Additional complete lookup tuples are retained but not activated by the bounded selection.".into());
    }
    let selection = json!({
        "complete_lookup_occurrences":complete_occurrences,"distinct_complete_tuples":complete.len(),
        "current_match_available":current.is_some(),"initial_active_ids":named_ids(&initial),
        "available_changed_domains":available_domains,"max_complete_targets":MAX_ACTIVATION_TARGETS,
        "selected_lookup_labels":selected.iter().map(|lookup| &lookup.label).collect::<Vec<_>>(),
        "selected_partial_lookup":partial.map(|lookup| &lookup.label),"gaps":gaps,
    });
    budget.retain(&selection)?;
    let mut cases = Vec::new();
    let mut observations = Vec::new();
    let nil = call(
        lua,
        module,
        observed,
        &function,
        MultiValue::from_vec(vec![Value::Table(build.clone()), Value::Nil]),
        "activate_nil",
        "SetActiveLoadout",
        json!({"kind":"direct_supplied_nil"}),
        json!({"kind":"nil","active_ids_before":named_ids(&active_ids(build)?)}),
        None,
        None,
        budget,
    )?;
    cases.push(nil.case);
    observations.push(nil.observation);
    let mut plans: Vec<_> = selected
        .iter()
        .enumerate()
        .map(|(index, lookup)| {
            (
                format!("activate_lookup_{}", index + 1),
                *lookup,
                "selected_complete_lookup",
            )
        })
        .collect();
    if let Some(last) = selected.last() {
        plans.push((
            "activate_repeat_last_lookup".into(),
            *last,
            "repeat_retained_lookup",
        ));
    }
    if let Some(partial) = partial {
        plans.push((
            "activate_partial_lookup".into(),
            partial,
            "natural_no_spec_lookup",
        ));
    }
    for (label, lookup, kind) in plans {
        let before = active_ids(build)?;
        // Invoke the exact retained source result table; no IDs or table are rebuilt.
        let value: Table = lookup.pack.raw_get(1)?;
        if ids(&value)? != lookup.ids {
            return Err(failure(
                "retained lookup request was mutated before activation",
            ));
        }
        let done = call(
            lua,
            module,
            observed,
            &function,
            MultiValue::from_vec(vec![Value::Table(build.clone()), Value::Table(value)]),
            &label,
            "SetActiveLoadout",
            json!({"kind":kind,"lookup_label":lookup.label}),
            json!({"kind":"retained_lookup_result","lookup_label":lookup.label,
                "requested_ids":named_ids(&lookup.ids),"active_ids_before":named_ids(&before),
                "changed_domains_before":changed_domains(&lookup.ids,&before)}),
            None,
            Some(&lookup.pack),
            budget,
        )?;
        cases.push(done.case);
        observations.push(done.observation);
    }
    let last = &observations
        .last()
        .ok_or_else(|| failure("no activation capture"))?["finite_post_loadouts"];
    budget.retain(last)?;
    let final_snapshot = last.clone();
    Ok(
        json!({"activation_cases":cases,"activation_observations":observations,
        "activation_selection":selection,"post_activation_exact_graph":final_snapshot["graph"],
        "post_activation_identity":final_snapshot["identity"]}),
    )
}

pub(super) fn run(lua: &Lua, module: &Table, observed: bool) -> mlua::Result<Json> {
    let build: Table = lua.globals().raw_get("build")?;
    let sync: Function = build.raw_get("SyncLoadouts")?;
    let lookup: Function = build.raw_get("GetLoadoutByName")?;
    let tree: Table = build.raw_get("treeTab")?;
    let common: Table = lua.globals().raw_get("common")?;
    let classes: Table = common.raw_get("classes")?;
    let tree_class: Table = classes.raw_get("TreeTab")?;
    let spec_list: Function = tree_class.raw_get("GetSpecList")?;
    let mut budget = Budget::default();
    let mut cases = Vec::new();
    let mut observations = Vec::new();
    let mut first = None::<Table>;
    for (index, (label, value)) in [
        ("sync_true_first", Value::Boolean(true)),
        ("sync_true_repeat", Value::Boolean(true)),
        ("sync_nil", Value::Nil),
        ("sync_false", Value::Boolean(false)),
    ]
    .into_iter()
    .enumerate()
    {
        let argument = match &value {
            Value::Boolean(v) => json!({"kind":"boolean","value":v}),
            _ => json!({"kind":"nil"}),
        };
        let done = call(
            lua,
            module,
            observed,
            &sync,
            MultiValue::from_vec(vec![Value::Table(build.clone()), value]),
            label,
            "SyncLoadouts",
            json!({"kind":"direct","sequence":index+1}),
            argument,
            first.as_ref(),
            None,
            &mut budget,
        )?;
        if index == 0 {
            first = done.result;
        }
        cases.push(done.case);
        observations.push(done.observation);
    }
    let done = call(
        lua,
        module,
        observed,
        &spec_list,
        MultiValue::from_vec(vec![Value::Table(tree)]),
        "spec_display_list",
        "GetSpecList",
        json!({"kind":"direct"}),
        json!({"kind":"no_argument"}),
        None,
        None,
        &mut budget,
    )?;
    let mut requests = Vec::new();
    let controls: Table = build.raw_get("controls")?;
    let dropdown: Table = controls.raw_get("buildLoadouts")?;
    names(
        &dropdown.raw_get("list")?,
        "post_sync_dropdown",
        &mut requests,
        &mut budget,
    )?;
    let spec_list_available = done.result.is_some();
    if let Some(result) = &done.result {
        let value: Value = result.raw_get(1)?;
        let Value::Table(list) = value else {
            return Err(failure("GetSpecList returned no display table"));
        };
        names(&list, "GetSpecList_result_1", &mut requests, &mut budget)?;
    }
    cases.push(done.case);
    observations.push(done.observation);
    let dropdown_occurrences = requests
        .iter()
        .filter(|r| r.origin["kind"] == "post_sync_dropdown")
        .count();
    let spec_occurrences = requests.len() - dropdown_occurrences;
    let mut known = requests
        .iter()
        .map(|r| r.text.clone())
        .collect::<BTreeSet<_>>();
    known_titles(&build, &mut known, &mut budget)?;
    let link_fields = [
        "treeListSpecialLinks",
        "itemListSpecialLinks",
        "skillListSpecialLinks",
        "configListSpecialLinks",
    ];
    let mut absent = None;
    for suffix in 1..=MAX_NAMES {
        let plain = format!("R2anAbsentLoadout{suffix}");
        let link = format!("R2anAbsentLink{suffix}");
        let linked = format!("{plain} {{{link}}}");
        if known.contains(&plain) || known.contains(&linked) {
            continue;
        }
        let mut missing = true;
        for field in link_fields {
            let map: Table = build.raw_get(field)?;
            missing &= matches!(map.raw_get::<Value>(link.as_str())?, Value::Nil);
        }
        if missing {
            absent = Some((plain, linked));
            break;
        }
    }
    let (plain, linked) = absent.ok_or_else(|| failure("absent-name search bound"))?;
    for (kind, text) in [("absent_plain", plain), ("absent_link", linked)] {
        let value = lua.create_string(&text)?;
        let text = budget.name(&value)?;
        requests.push(Name { value,text,origin:json!({"kind":kind,"title_and_display_absent":true,"all_four_link_keys_absent":true}) });
    }
    let mut retained = Vec::new();
    for (index, request) in requests.into_iter().enumerate() {
        let done = call(
            lua,
            module,
            observed,
            &lookup,
            MultiValue::from_vec(vec![
                Value::Table(build.clone()),
                Value::String(request.value),
            ]),
            &format!("lookup_{}", index + 1),
            "GetLoadoutByName",
            request.origin,
            json!({"kind":"string","value":request.text}),
            None,
            None,
            &mut budget,
        )?;
        if let Some(lookup) = retained_lookup(&done)? {
            retained.push(lookup);
        }
        cases.push(done.case);
        observations.push(done.observation);
    }
    let last = observations
        .last()
        .ok_or_else(|| failure("no completed capture"))?;
    budget.retain(&last["finite_post_loadouts"])?;
    let final_snapshot = last["finite_post_loadouts"].clone();
    // Keep the validated lookup poststate before directed activations mutate selections.
    let activation = activation_history(lua, module, observed, &build, &retained, &mut budget)?;
    let mut result = json!({
        "cases":cases,"observations":observations,
        "post_loadouts_exact_graph":final_snapshot["graph"],
        "post_loadouts_identity":final_snapshot["identity"],
        "lookup_occurrences":{"dropdown":dropdown_occurrences,"spec_display":spec_occurrences,"absent":2},
        "scope":{"unchanged_imported_owners":true,"fresh_capture_per_call":true,
            "actual_multivalue_packs":true,"duplicate_display_occurrences_retained":true,
            "dropdown_commands_also_queried":true,"first_sync_return_handles_retained":first.is_some(),
            "spec_display_list_available":spec_list_available,
            "token_ids_not_cross_host_semantics":true,"native_parity":false},
        "gaps":["No authored titles, missing owners, duplicate links, sparse orders, or version changes are introduced; directed activations use retained natural lookup results.",
            "Post-import direct calls do not establish initialization-time or full native build parity."],
        "bounds":{"calls":MAX_CASES,"display_occurrences":MAX_NAMES,"lookup_calls":MAX_NAMES+2,"name_bytes":MAX_NAME_BYTES,
            "aggregate_input_bytes":MAX_INPUT_BYTES,"retained_json_bytes":MAX_JSON_BYTES,"actual_returns":MAX_RETURNS},
        "usage":{"calls":budget.cases,"input_bytes":budget.input_bytes,"retained_case_and_observation_json_bytes":budget.json_bytes},
    });
    let Json::Object(activation) = activation else {
        unreachable!("activation history is an object")
    };
    result
        .as_object_mut()
        .expect("direct history")
        .extend(activation);
    Ok(result)
}

#[cfg(test)]
mod activation_mechanics {
    use super::*;

    #[test]
    fn activation_input_keeps_exact_lookup_table_and_nil_fields() {
        let lua = Lua::new();
        let original = lua.create_table().unwrap();
        original.raw_set("specId", 2).unwrap();
        original.raw_set("itemSetId", 7).unwrap();
        let result = pack(
            &lua,
            &MultiValue::from_vec(vec![Value::Table(original.clone())]),
        )
        .unwrap();
        let done = Completed {
            case: json!({"label":"supplied_lookup"}),
            observation: Json::Null,
            result: Some(result),
        };
        let retained = retained_lookup(&done).unwrap().unwrap();
        let actual: Table = retained.pack.raw_get(1).unwrap();
        assert_eq!(actual.to_pointer(), original.to_pointer());
        assert!(!retained.ids.iter().all(truthy_id));
        assert!(truthy_id(&retained.ids[0]));
        assert_eq!(retained.ids[2], json!({"kind":"nil"}));
        original.raw_set("itemSetId", 8).unwrap();
        assert_ne!(ids(&actual).unwrap(), retained.ids);
    }

    #[test]
    fn activation_id_comparison_keeps_scalar_types_and_source_truthiness() {
        assert_eq!(
            id_value(Value::Integer(2)).unwrap(),
            id_value(Value::Number(2.0)).unwrap()
        );
        assert!(!truthy_id(&id_value(Value::Nil).unwrap()));
        assert!(!truthy_id(&id_value(Value::Boolean(false)).unwrap()));
        assert!(truthy_id(&id_value(Value::Number(0.0)).unwrap()));
        assert!(id_value(Value::Number(f64::INFINITY)).is_err());
        let lua = Lua::new();
        assert_ne!(
            id_value(Value::Integer(2)).unwrap(),
            id_value(Value::String(lua.create_string("2").unwrap())).unwrap()
        );
        assert!(id_value(Value::Table(lua.create_table().unwrap())).is_err());
    }
}
