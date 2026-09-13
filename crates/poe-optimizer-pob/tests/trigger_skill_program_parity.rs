//! Complete unchanged triggerExtraSkill source and public wrappers against native programs.
//! These are parser/metadata tests, not triggered actor or whole-build parity.
#[allow(dead_code)]
#[path = "support/callback_factories_source.rs"]
mod factory_source;
#[path = "support/mod_parser_native_observer.rs"]
mod native_observer;
#[allow(dead_code)]
#[path = "support/explosion_helper_observer.rs"]
mod observer;
#[allow(dead_code)]
#[path = "support/mod_parser_public_source.rs"]
mod public_source;
#[allow(dead_code)]
#[path = "support/parser_program_source.rs"]
mod raw_source;
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
use factory_source::{FactorySource, upvalue};
use mlua::{Function, MultiValue, Table, Value};
use poe_optimizer_data::modifier_parser::*;
use poe_optimizer_engine::{
    lua_pattern::MatchBudget, modifier_parser::CompiledModifierParser, parser_program::*,
};
use std::collections::BTreeSet;
const PATTERN: &str = "^curse enemies with (%D+) on %a+$";
struct Original {
    source: FactorySource,
    wrapper: Function,
    helper: Function,
    insert: Function,
    lookup: Table,
}
impl Original {
    fn new() -> Self {
        let source = FactorySource::new();
        let lua = &source.public.source.lua;
        let wrapper: Function = source.special.raw_get(PATTERN).unwrap();
        let helper = upvalue(lua, &wrapper, "triggerExtraSkill")
            .as_function()
            .unwrap()
            .clone();
        let insert = upvalue(lua, &helper, "t_insert")
            .as_function()
            .unwrap()
            .clone();
        let lookup = upvalue(lua, &helper, "gemIdLookup")
            .as_table()
            .unwrap()
            .clone();
        let result = Self {
            source,
            wrapper,
            helper,
            insert,
            lookup,
        };
        result.verify();
        result
    }
    fn verify(&self) {
        let lua = &self.source.public.source.lua;
        assert_eq!(
            self.source.special.raw_get::<Function>(PATTERN).unwrap(),
            self.wrapper
        );
        assert_eq!(
            upvalue(lua, &self.wrapper, "triggerExtraSkill").as_function(),
            Some(&self.helper)
        );
        assert_eq!(
            upvalue(lua, &self.helper, "t_insert").as_function(),
            Some(&self.insert)
        );
        assert_eq!(
            upvalue(lua, &self.helper, "mod").as_function(),
            Some(&self.source.create_mod)
        );
        assert_eq!(
            upvalue(lua, &self.helper, "gemIdLookup").as_table(),
            Some(&self.lookup)
        );
        assert_eq!(
            lua.globals()
                .get::<Table>("table")
                .unwrap()
                .get::<Function>("insert")
                .unwrap(),
            self.insert
        );
        assert_eq!(self.insert.info().what, "C");
        assert_eq!(
            self.lookup.raw_get::<String>("enfeeble").unwrap(),
            "EnfeeblePlayer"
        );
        for (function, first, last) in [(&self.wrapper, 3633, 3633), (&self.helper, 2201, 2218)] {
            let info = function.info();
            assert_eq!(info.source.as_deref(), Some("@src/Modules/ModParser.lua"));
            assert_eq!(info.line_defined, Some(first));
            assert_eq!(info.last_line_defined, Some(last));
        }
    }
    fn text(&self, text: impl AsRef<[u8]>) -> Value {
        self.source.text(text)
    }
    fn table(&self) -> Table {
        self.source.public.source.lua.create_table().unwrap()
    }
    fn ingress(&self, args: Vec<Value>) -> MultiValue {
        // Observe the exact values after LuaJIT's host-number ingress (notably NaN).
        self.source
            .public
            .source
            .lua
            .load("return ...")
            .call(MultiValue::from_vec(args))
            .unwrap()
    }
}
fn callback_id(owner: &ModifierParserCatalog, function: &Function) -> ParserCallbackId {
    let info = function.info();
    let ids = owner
        .data()
        .callbacks
        .iter()
        .enumerate()
        .filter_map(|(index, c)| {
            let ParserCallbackKind::Lua { source } = &c.kind else {
                return None;
            };
            (info.source.as_deref() == Some(format!("@{}", source.path).as_str())
                && info.line_defined == Some(source.line as usize)
                && info.last_line_defined == Some(source.end_line as usize))
            .then_some(ParserCallbackId(index as u32 + 1))
        })
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 1, "unique actual original declaration");
    ids[0]
}
fn family(owner: &ModifierParserCatalog, helper: ParserCallbackId) -> BTreeSet<ParserCallbackId> {
    owner
        .data()
        .callbacks
        .iter()
        .enumerate()
        .filter_map(|(index, c)| {
            c.upvalues
                .iter()
                .any(|u| u.value == ParserValue::Callback(helper))
                .then_some(ParserCallbackId(index as u32 + 1))
        })
        .collect()
}
fn plan() -> CompiledParserPrograms {
    CompiledParserPrograms::new(raw_source::extraction().catalog()).unwrap()
}
fn pair(
    original: &Original,
    plan: &CompiledParserPrograms,
    function: &Function,
    args: Vec<Value>,
    label: &str,
) -> MultiValue {
    original.verify();
    let args = original.ingress(args);
    let input = raw_source::from_source(&observer::graph(&original.source, args.clone()));
    let source = function
        .call::<MultiValue>(args)
        .unwrap_or_else(|e| panic!("{label}: source {e}"));
    let expected = observer::graph(&original.source, source.clone());
    let native = plan
        .execute(
            callback_id(plan.catalog().owner(), function),
            &input,
            ProgramLimits::default(),
        )
        .unwrap_or_else(|e| panic!("{label}: native {e:?}"));
    assert_eq!(
        raw_source::native_graph(native.graph()),
        expected,
        "{label}: full graph, aliases, return pack"
    );
    original.verify();
    source
}
fn rows(result: &MultiValue) -> Table {
    assert_eq!(
        result.len(),
        1,
        "helper always returns one list, including empty"
    );
    result.front().unwrap().as_table().unwrap().clone()
}
fn extra_skill(result: &MultiValue) -> Table {
    let row: Table = rows(result).raw_get(1).unwrap();
    assert_eq!(row.raw_get::<String>("name").unwrap(), "ExtraSkill");
    assert_eq!(row.raw_get::<String>("type").unwrap(), "LIST");
    let value: Table = row.raw_get("value").unwrap();
    assert_eq!(
        value.raw_get::<String>("skillId").unwrap(),
        "EnfeeblePlayer"
    );
    assert!(value.raw_get::<bool>("triggered").unwrap());
    value
}
#[test]
fn complete_helper_preserves_level_conversion_options_nested_modifiers_and_empty_lists() {
    let original = Original::new();
    let plan = plan();
    for level in [
        Value::Nil,
        Value::Boolean(false),
        Value::Number(0.0),
        Value::Number(-0.0),
        Value::Number(17.25),
        Value::Number(f64::INFINITY),
        Value::Number(f64::NEG_INFINITY),
        Value::Number(f64::NAN),
        original.text(" 7e0 "),
        original.text("0x8"),
        original.text("invalid"),
    ] {
        let result = pair(
            &original,
            &plan,
            &original.helper,
            vec![original.text("enfeeble skill"), level],
            "level conversion",
        );
        extra_skill(&result);
    }
    for mask in 0..8 {
        let options = original.table();
        options.raw_set("noSupports", mask & 1 != 0).unwrap();
        options.raw_set("ignoreHexproof", mask & 2 != 0).unwrap();
        options.raw_set("onCrit", mask & 4 != 0).unwrap();
        options.raw_set("triggerChance", "37").unwrap();
        // False avoids mutation of caller-owned options; wrapper-local sourceSkill is tested below.
        options.raw_set("sourceSkill", false).unwrap();
        let result = pair(
            &original,
            &plan,
            &original.helper,
            vec![
                original.text("enfeeble"),
                Value::Number(17.0),
                Value::Table(options),
            ],
            "options",
        );
        let value = extra_skill(&result);
        assert_eq!(value.raw_get::<bool>("noSupports").unwrap(), mask & 1 != 0);
        assert_eq!(value.raw_get::<f64>("triggerChance").unwrap(), 37.0);
        assert!(!value.raw_get::<bool>("source").unwrap());
        assert_eq!(
            rows(&result).raw_len(),
            1 + usize::from(mask & 2 != 0) + usize::from(mask & 4 != 0)
        );
    }
    for name in ["missing test skill", "Enfeeble", "", "skill"] {
        let result = pair(
            &original,
            &plan,
            &original.helper,
            vec![original.text(name), Value::Number(17.0)],
            "unknown lookup",
        );
        assert_eq!(rows(&result).raw_len(), 0);
    }
    let options = original.table();
    options.raw_set("ignoreHexproof", true).unwrap();
    options.raw_set("onCrit", true).unwrap();
    let result = pair(
        &original,
        &plan,
        &original.helper,
        vec![original.text("unknown"), Value::Nil, Value::Table(options)],
        "unknown with metadata branches",
    );
    assert_eq!(
        rows(&result).raw_len(),
        2,
        "later branches still emit metadata for missing lookup"
    );
}
#[test]
fn complete_helper_source_failures_remain_source_errors_at_the_actual_caller() {
    let original = Original::new();
    let plan = plan();
    let helper = callback_id(plan.catalog().owner(), &original.helper);
    for (args, line) in [
        (vec![Value::Nil], 2203),
        (vec![Value::Boolean(true)], 2203),
        (
            vec![original.text("enfeeble"), Value::Nil, Value::Boolean(true)],
            2204,
        ),
        (
            {
                let options = original.table();
                options.raw_set("sourceSkill", true).unwrap();
                vec![original.text("enfeeble"), Value::Nil, Value::Table(options)]
            },
            2205,
        ),
    ] {
        let args = original.ingress(args);
        let input = raw_source::from_source(&observer::graph(&original.source, args.clone()));
        let source = original.helper.call::<MultiValue>(args).unwrap_err();
        assert!(
            observer::source_error_at(&source, "attempt", line),
            "{source}"
        );
        let native = plan
            .execute(helper, &input, ProgramLimits::default())
            .unwrap_err();
        assert_eq!(native.kind, ProgramRuntimeErrorKind::Source, "{native:?}");
        assert_eq!(native.callback, Some(helper));
        let program = plan.catalog().for_callback(helper).unwrap();
        let text = runtime::verified(&program.provenance.source.path).unwrap();
        let declaration = text
            .lines()
            .skip(program.provenance.source.line as usize - 1)
            .take(
                (program.provenance.source.end_line - program.provenance.source.line + 1) as usize,
            )
            .collect::<Vec<_>>()
            .join("\n");
        let location = native.location.expect("actual failing operation");
        let offset = program.provenance.function_start as usize;
        let reached_line = program.provenance.source.line as usize
            + declaration[..offset + location.start as usize]
                .bytes()
                .filter(|b| *b == b'\n')
                .count();
        assert_eq!(
            reached_line, line,
            "native span starts at the actual failing source statement"
        );
    }
    original.verify();
}
#[test]
fn direct_helper_borrowed_option_mutation_is_an_explicit_frontier() {
    let original = Original::new();
    let plan = plan();
    let options = original.table();
    options.raw_set("sourceSkill", "spark skill skill").unwrap();
    let args = original.ingress(vec![
        original.text("enfeeble"),
        Value::Number(17.0),
        Value::Table(options.clone()),
    ]);
    let input = raw_source::from_source(&observer::graph(&original.source, args.clone()));
    let source = original.helper.call::<MultiValue>(args).unwrap();
    assert_eq!(options.raw_get::<String>("sourceSkill").unwrap(), "spark");
    assert_eq!(
        extra_skill(&source).raw_get::<String>("source").unwrap(),
        "spark"
    );
    let helper = callback_id(plan.catalog().owner(), &original.helper);
    let native = plan
        .execute(helper, &input, ProgramLimits::default())
        .unwrap_err();
    assert_eq!(native.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
    assert_eq!(native.callback, Some(helper));
    assert!(native.message.contains("borrowed"), "{native:?}");
    original.verify();
}
#[test]
fn all_fifty_complete_original_wrappers_match_native_raw_returns() {
    let original = Original::new();
    let plan = plan();
    let owner = plan.catalog().owner();
    let ids = family(owner, callback_id(owner, &original.helper));
    assert_eq!(ids.len(), 50);
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("support/trigger_skill_lines.json")).unwrap();
    let mut reached = BTreeSet::new();
    for case in cases {
        let function: Function = original
            .source
            .special
            .raw_get(case["pattern"].as_str().unwrap())
            .unwrap();
        let id = callback_id(owner, &function);
        assert!(ids.contains(&id));
        assert_eq!(
            upvalue(
                &original.source.public.source.lua,
                &function,
                "triggerExtraSkill"
            )
            .as_function(),
            Some(&original.helper)
        );
        let info = function.info();
        assert_eq!(info.line_defined, info.last_line_defined);
        let text = runtime::verified("src/Modules/ModParser.lua").unwrap();
        let declaration = text.lines().nth(info.line_defined.unwrap() - 1).unwrap();
        let params = declaration
            .split_once("function(")
            .unwrap()
            .1
            .split_once(')')
            .unwrap()
            .0;
        for skill in ["enfeeble", "unknown test skill", "summon phantasm skill"] {
            let args = params
                .split(',')
                .map(|p| match p.trim() {
                    "_" => Value::Nil,
                    "num" | "chance" => Value::Number(37.0),
                    "level" => original.text("17"),
                    "skill" => original.text(skill),
                    "sourceSkill" => original.text("spark skill"),
                    other => panic!("unreviewed original parameter: {other}"),
                })
                .collect();
            let output = pair(
                &original,
                &plan,
                &function,
                args,
                &format!("wrapper {} {skill}", id.0),
            );
            if skill == "enfeeble" {
                extra_skill(&output);
            }
            rows(&output);
        }
        reached.insert(id);
    }
    assert_eq!(reached, ids);
}
fn mutate_nested(table: &mut poe_optimizer_engine::modifier_parser::ModifierTable) {
    use poe_optimizer_engine::modifier_parser::ModifierValue;
    for value in table.fields.values_mut().chain(table.indexed.values_mut()) {
        if let ModifierValue::Table(child) = value {
            mutate_nested(std::sync::Arc::make_mut(child));
        }
    }
    table
        .fields
        .insert("caller mutation".into(), ModifierValue::Boolean(true));
}
fn public_pair(original: &Original, native: &CompiledModifierParser, line: &str) {
    let owner = native.catalog();
    let source = original.source.public.raw(line.as_bytes()).unwrap();
    extra_skill(&source);
    let expected = observer::graph(&original.source, source.clone());
    let mut actual = native
        .parse(line.as_bytes(), &mut MatchBudget::default())
        .unwrap();
    assert_eq!(
        native_observer::capture(&actual, owner).unwrap(),
        expected,
        "public {line}"
    );
    // Exercise recursive copy isolation, including the nested onCrit modifier.
    for row in rows(&source).clone().sequence_values::<Table>() {
        let value: Table = row.unwrap().raw_get("value").unwrap();
        value.raw_set("caller mutation", true).unwrap();
        if let Value::Table(nested) = value.raw_get::<Value>("mod").unwrap() {
            nested
                .raw_get::<Table>("value")
                .unwrap()
                .raw_set("caller mutation", true)
                .unwrap();
        }
    }
    rows(&source).raw_set("caller mutation", true).unwrap();
    mutate_nested(actual.modifiers.as_mut().unwrap());
    let again = original.source.public.raw(line.as_bytes()).unwrap();
    assert_eq!(
        observer::graph(&original.source, again),
        expected,
        "original copy {line}"
    );
    let again = native
        .parse(line.as_bytes(), &mut MatchBudget::default())
        .unwrap();
    assert_eq!(
        native_observer::capture(&again, owner).unwrap(),
        expected,
        "native repeat {line}"
    );
}
#[test]
fn complete_public_parser_preserves_natural_selection_and_proves_shadowed_aliases() {
    let original = Original::new();
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let owner = snapshot.modifier_parser();
    let helper = callback_id(owner, &original.helper);
    let ids = family(owner, helper);
    for id in &ids {
        assert_eq!(
            owner.data().programs.admissions[id].role,
            ParserProgramRole::Special
        );
    }
    assert_eq!(
        owner.data().programs.admissions[&helper].role,
        ParserProgramRole::Helper
    );
    let native = CompiledModifierParser::new(owner).unwrap();
    let scan = upvalue(
        &original.source.public.source.lua,
        &original.source.internal,
        "scan",
    )
    .as_function()
    .unwrap()
    .clone();
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("support/trigger_skill_lines.json")).unwrap();
    let mut reached = BTreeSet::new();
    let mut shadowed = Vec::new();
    for case in &cases {
        let line = case["line"].as_str().unwrap();
        let function: Function = original
            .source
            .special
            .raw_get(case["pattern"].as_str().unwrap())
            .unwrap();
        let (selected, remainder, _): (Function, mlua::LuaString, Value) =
            scan.call((line, original.source.special.clone())).unwrap();
        assert!(remainder.as_bytes().is_empty());
        let actual = callback_id(owner, &selected);
        assert!(
            ids.contains(&actual),
            "natural selection remains in the proved family"
        );
        if selected != function {
            shadowed.push((case.clone(), function));
        }
        reached.insert(actual);
        public_pair(&original, &native, line);
    }
    let natural_count = reached.len();
    // Labelled authored aliases preserve original functions and capture grammar while
    // exposing callbacks shadowed by another natural pattern. No winning key is invented.
    let mut data = owner.data().clone();
    let special = data.dictionaries[&ParserDictionary::Special];
    let mut aliases = Vec::new();
    for (index, (case, function)) in shadowed.iter().enumerate() {
        let prefix = format!("testtrigger{index} ");
        let pattern = format!(
            "^{prefix}{}",
            case["pattern"].as_str().unwrap().strip_prefix('^').unwrap()
        );
        let line = format!("{prefix}{}", case["line"].as_str().unwrap());
        let id = callback_id(owner, function);
        original.source.add(&pattern, function.clone());
        data.tables[special.0 as usize - 1]
            .fields
            .insert(pattern, ParserValue::Callback(id));
        aliases.push((line, function.clone(), id));
    }
    data.programs.admissions.clear();
    for (id, permission) in &owner.data().programs.admissions {
        let program = data
            .programs
            .data
            .programs
            .iter()
            .find(|p| p.callback == *id)
            .unwrap();
        let bound = ParserProgramAdmission::bind(
            &data,
            program,
            permission.role,
            "authored trigger alias fixture: exact original callback and capture grammar",
        )
        .unwrap();
        data.programs.admissions.insert(*id, bound);
    }
    let alias_owner = ModifierParserCatalog::new(data).unwrap();
    let alias_native = CompiledModifierParser::new(&alias_owner).unwrap();
    for (line, function, id) in aliases {
        let (selected, remainder, _): (Function, mlua::LuaString, Value) = scan
            .call((line.as_str(), original.source.special.clone()))
            .unwrap();
        assert_eq!(selected, function, "exact authored alias selection");
        assert!(remainder.as_bytes().is_empty());
        public_pair(&original, &alias_native, &line);
        reached.insert(id);
    }
    eprintln!(
        "trigger public: {} natural inputs, {natural_count} callbacks, {} authored shadowed aliases",
        cases.len(),
        shadowed.len()
    );
    assert_eq!(reached, ids);
    original.verify();
}
