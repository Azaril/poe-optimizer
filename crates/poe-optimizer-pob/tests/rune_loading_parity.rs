//! Complete original rune loading methods remain the independent oracle.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_loading_native.rs"]
mod reference;
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
#[allow(dead_code)]
#[path = "support/base_buff_source.rs"]
mod source;
use mlua::{Function, Table, Value};
use poe_optimizer_data::game_data::bundled_snapshot;
use poe_optimizer_data::item_loading::ItemLoadingCatalog;
use poe_optimizer_import::item_loading::*;

fn canonical(value: Value) -> serde_json::Value {
    match value {
        Value::Nil => serde_json::json!(["nil"]),
        Value::Boolean(v) => serde_json::json!(["boolean", v]),
        Value::Integer(v) => {
            serde_json::json!(["number", format!("{:016x}", (v as f64).to_bits())])
        }
        Value::Number(v) => serde_json::json!(["number", format!("{:016x}", v.to_bits())]),
        Value::String(v) => serde_json::json!(["string", v.to_str().unwrap().to_owned()]),
        Value::Table(t) => {
            let mut rows = t
                .pairs::<Value, Value>()
                .map(|r| {
                    let (k, v) = r.unwrap();
                    (canonical(k), canonical(v))
                })
                .collect::<Vec<_>>();
            rows.sort_by_key(|(k, _)| k.to_string());
            serde_json::json!(["table", rows])
        }
        other => panic!("unsupported source snapshot {other:?}"),
    }
}
const OBSERVER: &str = r#"
-- These wrappers delegate every complete original method. They record state
-- around source calls and restore observer phase variables after source errors.
local class=common.classes.Item
local parseRaw,update,classify,build=class.ParseRaw,class.UpdateRunes,class.GetSocketedAugmentTypes,class.BuildModList
local parse,format=modLib.parseMod,itemLib.applyRange
rune_original_functions={update=update,classify=classify,display=class.ApplySocketedRuneDisplayScalars}
rune_enabled=false
local current,updating,building
local function snapshot()return current and buff_snapshot(current)end
function class:ParseRaw(...)
 if not rune_enabled then return parseRaw(self,...)end
 local old=current;current=self
 local ok,result=pcall(parseRaw,self,...);current=old
 if not ok then error(result,0)end
 return result
end
function class:BuildModList(...)
 local old=building;building=true
 local ok,result=pcall(build,self,...);building=old
 if not ok then error(result,0)end
 return result
end
function class:UpdateRunes(...)
 if not rune_enabled then return update(self,...)end
 local old,was=current,updating;current=self;updating=true
 local event={before=snapshot()};rune_updates[#rune_updates+1]=event
 local ok,result=pcall(update,self,...)
 event.after=snapshot();event.ok=ok;if not ok then event.error=tostring(result)end
 current=old;updating=was
 if not ok then error(result,0)end
 return result
end
function class:GetSocketedAugmentTypes(...)
 local a,b=classify(self,...)
 if rune_enabled and not building then rune_contexts[#rune_contexts+1]={broad=a,specific=b}end
 return a,b
end
function modLib.parseMod(text,combined,...)
 if not rune_enabled or building then return parse(text,combined,...)end
 local event={text=text,combined=combined==true,before=snapshot(),phase=updating and 'update_runes' or 'parse_raw'}
 rune_calls[#rune_calls+1]=event
 local ok,mods,extra=pcall(parse,text,combined,...)
 event.after=snapshot();event.ok=ok
 if not ok then event.error=tostring(mods);error(mods,0)end
 event.has_modifiers=mods~=nil;event.extra=extra
 return mods,extra
end
function itemLib.applyRange(text,range,scalar,corrupted)
 if rune_enabled and not building then
  rune_formats[#rune_formats+1]={text=text,range=range,scalar=scalar,corrupted=corrupted,before=snapshot()}
 end
 return format(text,range,scalar,corrupted)
end
function rune_clear()
 rune_calls={};rune_updates={};rune_contexts={};rune_formats={}
end
"#;
struct RuneSource {
    source: source::Source,
}
impl RuneSource {
    fn new() -> Self {
        let source = source::Source::new();
        source
            .oracle
            .lua
            .load(OBSERVER)
            .set_name("@test-only-complete-rune-observer")
            .exec()
            .unwrap();
        Self { source }
    }
    fn clear(&self) {
        self.source.clear();
        self.source
            .oracle
            .lua
            .globals()
            .get::<Function>("rune_clear")
            .unwrap()
            .call::<()>(())
            .unwrap();
    }
    fn try_parse(&self, raw: &str) -> (Table, Option<mlua::Error>) {
        let item = self.source.oracle.parse("");
        self.clear();
        let globals = self.source.oracle.lua.globals();
        globals.set("rune_enabled", true).unwrap();
        let error = item
            .get::<Function>("ParseRaw")
            .unwrap()
            .call::<()>((item.clone(), raw))
            .err();
        globals.set("rune_enabled", false).unwrap();
        (item, error)
    }
    fn rows(&self, key: &str) -> Vec<Table> {
        self.source
            .oracle
            .lua
            .globals()
            .get::<Table>(key)
            .unwrap()
            .sequence_values::<Table>()
            .map(Result::unwrap)
            .collect()
    }
    fn calls(&self) -> Vec<(String, bool)> {
        self.rows("rune_calls")
            .into_iter()
            .map(|t| (t.get("text").unwrap(), t.get("combined").unwrap()))
            .collect()
    }
}
fn raw(base: &str, sockets: &str, runes: &[&str], body: &str) -> String {
    let mut out =
        format!("Rarity: Normal\n{base}\nItem Level: 80\nQuality: 0\nSockets: {sockets}\n");
    for name in runes {
        out.push_str(&format!("Rune: {name}\n"));
    }
    out.push_str(body);
    out
}
/// Only test-owned definitions are replaced; complete original methods remain
/// unchanged, and the same validated source-shaped catalog is injected natively.
fn custom_catalog(source: &RuneSource, script: &str) -> ItemLoadingCatalog {
    let definitions = source
        .source
        .oracle
        .lua
        .load(script)
        .eval::<Table>()
        .unwrap();
    let snapshot = bundled_snapshot().unwrap();
    let mut data = snapshot.item_loading().data().clone();
    if let Some(bases) = definitions.get::<Option<Table>>("bases").unwrap() {
        for row in bases.pairs::<String, Table>() {
            let (name, fields) = row.unwrap();
            let original = source.source.base(&name).unwrap();
            let native = data.bases.iter_mut().find(|b| b.name == name).unwrap();
            for (key, value) in reference::metadata(fields.clone()).fields {
                if key == "type" {
                    native.item_type = value.as_str().unwrap().into();
                }
                native.fields.fields.insert(key, value);
            }
            for row in fields.pairs::<String, Value>() {
                let (key, value) = row.unwrap();
                original.set(key, value).unwrap();
            }
        }
    }
    if let Some(runes) = definitions.get::<Option<Table>>("runes").unwrap() {
        data.modifier_tables
            .insert("Runes".into(), reference::metadata(runes.clone()));
        source
            .source
            .oracle
            .lua
            .globals()
            .get::<Table>("data")
            .unwrap()
            .get::<Table>("itemMods")
            .unwrap()
            .set("Runes", runes)
            .unwrap();
    }
    ItemLoadingCatalog::new(data).unwrap()
}
#[test]
fn original_complete_rune_methods_are_authenticated_and_observation_preserves_results() {
    let source = RuneSource::new();
    let functions = source
        .source
        .oracle
        .lua
        .globals()
        .get::<Table>("rune_original_functions")
        .unwrap();
    for (name, line, last) in [
        ("update", 2106, 2178),
        ("classify", 2338, 2352),
        ("display", 2180, 2194),
    ] {
        let f = functions.get::<Function>(name).unwrap();
        let info = f.info();
        assert_eq!(info.source.as_deref(), Some("@src/Classes/Item.lua"));
        assert_eq!(info.line_defined, Some(line));
        assert_eq!(info.last_line_defined, Some(last));
    }
    for raw in [
        raw("Twig Focus", "S", &["Greater Iron Rune"], ""),
        raw("Stocky Mitts", "S", &["Greater Desert Rune"], ""),
        raw(
            "Rusted Greathelm",
            "S S",
            &["None", "None"],
            "+10 to maximum Life",
        ),
    ] {
        let (observed, error) = source.try_parse(&raw);
        assert!(error.is_none(), "{raw}: {error:?}");
        let observed = canonical(Value::Table(source.source.snapshot(&observed)));
        let control = source.source.oracle.parse(&raw);
        assert_eq!(
            observed,
            canonical(Value::Table(source.source.snapshot(&control))),
            "observer control {raw}"
        );
    }
}
#[test]
fn original_update_runes_combines_without_passing_the_parser_combined_argument() {
    let source = RuneSource::new();
    source.source.oracle.lua.load(r#"data.itemMods.Runes={Caller={armour={'+10 to maximum Life',type='Rune',levelReq=5,statOrder={1},bonded={'+3 to Strength',statOrder={2}}}}}"#).exec().unwrap();
    let raw = raw("Rusted Greathelm", "S S", &["Caller", "Caller"], "");
    let (_, error) = source.try_parse(&raw);
    assert!(error.is_none(), "{error:?}");
    let calls = source.calls();
    assert_eq!(
        calls,
        vec![
            ("+10 to maximum Life".into(), false),
            ("+3 to Strength".into(), false),
            ("+20 to maximum Life".into(), false),
            ("+6 to Strength".into(), false)
        ]
    );
    let updates = source.rows("rune_updates");
    assert_eq!(updates.len(), 1);
    assert!(updates[0].get::<bool>("ok").unwrap());
    let before = source.source.before();
    let rows = before.get::<Table>("runeModLines").unwrap();
    assert_eq!(rows.raw_len(), 2);
    assert_eq!(
        rows.get::<Table>(1).unwrap().get::<String>("line").unwrap(),
        "+20 to maximum Life"
    );
    assert_eq!(
        rows.get::<Table>(2).unwrap().get::<String>("line").unwrap(),
        "Bonded: +6 to Strength"
    );
}

fn paired_before(catalog: &ItemLoadingCatalog, source: &RuneSource, raw: &str) -> ItemState {
    let (_, error) = source.try_parse(raw);
    assert!(
        !source.source.stages().is_empty(),
        "source failed before assembly: {raw}: {error:?}"
    );
    let mut provider = reference::OriginalDependencies::new(&source.source.oracle);
    let mut machine = ItemLoadMachine::new(catalog);
    machine.apply_text(raw, &mut provider).unwrap();
    assert_eq!(
        machine.pending().map(|p| p.kind),
        Some(DependencyKind::Assembly),
        "{raw}: {:?}",
        machine.pending()
    );
    reference::compare_state(machine.state(), &source.source.before());
    assert_eq!(
        provider.calls,
        source.calls(),
        "all original parser calls for {raw}"
    );
    let formats = source.rows("rune_formats");
    assert_eq!(
        machine.state().format_calls.len(),
        formats.len(),
        "formatter call count"
    );
    for (request, expected) in machine.state().format_calls.iter().zip(formats) {
        assert_eq!(
            request.text,
            expected.get::<String>("text").unwrap(),
            "formatter text"
        );
        for (field, value) in [
            ("range", request.range),
            ("scalar", request.scalar),
            ("corrupted", request.corrupted_range),
        ] {
            reference::assert_number(
                value,
                reference::number(expected.get(field).unwrap()),
                field,
            );
        }
    }
    let source_calls = source.rows("rune_calls");
    assert_eq!(machine.state().parser_calls.len(), source_calls.len());
    for (request, event) in machine.state().parser_calls.iter().zip(source_calls) {
        if event.get::<String>("phase").unwrap() == "update_runes" {
            assert_eq!(
                request.line_index, None,
                "generated request has no invented raw line"
            );
            assert!(request.origin.is_some(), "generated request provenance");
            assert!(
                !request.combined,
                "original generated parser has no combined argument"
            );
        } else {
            assert!(request.line_index.is_some());
            assert!(request.origin.is_none());
        }
    }
    machine.into_state()
}
#[test]
fn authored_known_none_unknown_and_inactive_names_preserve_complete_original_state() {
    let snapshot = bundled_snapshot().unwrap();
    let source = RuneSource::new();
    let mut count = 0;
    for (base, sockets, runes) in [
        ("Twig Focus", "S", vec!["Greater Iron Rune"]),
        ("Stocky Mitts", "S", vec!["Greater Desert Rune"]),
        ("Rusted Greathelm", "S S", vec!["None", "None"]),
        ("Rusted Greathelm", "S", vec!["Missing Caller Rune"]),
        (
            "Rusted Greathelm",
            "S",
            vec!["Greater Iron Rune", "Missing Caller Rune"],
        ),
        ("Twig Focus", "S", vec!["Greater Iron Rune", "None"]),
        ("Rusted Greathelm", "", vec!["None"]),
    ] {
        let raw = raw(base, sockets, &runes, "+10 to maximum Life");
        paired_before(snapshot.item_loading(), &source, &raw);
        count += 1;
    }
    eprintln!("Complete original authored-name domains: {count} paired states");
}
#[test]
fn injected_rebuild_normal_bonded_order_and_all_record_requirements_match_original() {
    let source = RuneSource::new();
    let catalog = custom_catalog(
        &source,
        r#"return {runes={
      Caller={armour={'+10 to maximum Life',type='Rune',levelReq=5,statOrder={1},bonded={'+3 to Strength',statOrder={2}}},weapon={type='Rune',levelReq=71}},
      Other={armour={'+4 to Dexterity',type='SoulCore',levelReq=8,statOrder={1}}},
      Inactive={gloves={type='Idol',levelReq=79}}
    }}"#,
    );
    for (names, sockets, expected_level) in [
        (vec!["Caller"], "S", 71.0),
        (vec!["Caller", "Caller"], "S S", 71.0),
        (vec!["Caller", "Other"], "S S", 71.0),
        (vec!["Caller", "Inactive"], "S", 79.0),
    ] {
        let state = paired_before(
            &catalog,
            &source,
            &raw("Rusted Greathelm", sockets, &names, ""),
        );
        assert_eq!(
            state.requirements["runeLevel"],
            ItemNumber::new(expected_level)
        );
        for row in &state.rune_mod_lines {
            assert!(row.source_line.is_none());
            assert!(!row.rune_origins.is_empty());
        }
        let origins = state
            .parser_calls
            .iter()
            .map(|p| p.origin.as_ref().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(origins[0].socket_index, 1);
        assert_eq!(origins[0].slot_key, "armour");
        assert_eq!(origins[0].definition_line_index, 1);
        assert!(!origins[0].bonded);
        assert!(!origins[0].combined);
        assert!(origins[1].bonded);
        if names == ["Caller", "Caller"] {
            assert!(origins[2].combined && origins[3].combined);
            assert_eq!(origins[2].socket_index, 2);
        }
    }
}
#[test]
fn raw_bonded_disabled_and_implicit_routing_match_original() {
    let source = RuneSource::new();
    let catalog = custom_catalog(
        &source,
        r#"return {runes={Caller={armour={'+10 to maximum Life',type='Rune',levelReq=5,statOrder={1},bonded={'+3 to Strength',statOrder={2}}}}}}"#,
    );
    for body in [
        "Implicits: 2\n{rune}Bonded: +3 to Strength\n+7 to Dexterity\n+8 to Intelligence",
        "Implicits: 2\n{rune}{disabled}Bonded: +3 to Strength\n+7 to Dexterity\n+8 to Intelligence",
        "{rune}{disabled}+10 to maximum Life\n{rune}{disabled}+10 to maximum Life",
        "{rune}{enchant}+10 to maximum Life\n{enchant}+4 to Dexterity",
        "{rune}Bonded: +3 to Strength\n{rune}+10 to maximum Life",
    ] {
        paired_before(
            &catalog,
            &source,
            &raw("Rusted Greathelm", "S", &["Caller"], body),
        );
    }
}

#[test]
fn contextual_slots_overrides_and_duplicate_contributions_match_original() {
    let source = RuneSource::new();
    let catalog = custom_catalog(
        &source,
        r#"return {runes={Caller={
      armour={'+10 to maximum Life',type='SoulCore',levelReq=5,statOrder={1}},
      helmet={'+3 to Strength',type='Rune',levelReq=7,statOrder={2}},
      weapon={'+4 to Dexterity',type='SoulCore',levelReq=8,statOrder={3}},
      gloves={'+5 to Intelligence',type='Rune',levelReq=9,statOrder={4}}
    }}}"#,
    );
    for (body, keys) in [
        ("", vec!["armour", "helmet"]),
        (
            "This Item gains bonuses from Socketed Items as though it was a Gloves",
            vec!["armour", "gloves"],
        ),
        (
            "This Item gains bonuses from Socketed Items as though it was a Armour",
            vec!["armour", "armour"],
        ),
        (
            "This Item gains bonuses from Socketed Soul Cores as though it was also a Armour",
            vec!["armour", "helmet", "armour"],
        ),
        (
            "This Item gains bonuses from Socketed Soul Cores as though it was also a Weapon",
            vec!["armour", "helmet", "weapon"],
        ),
        (
            "Variant: First\nVariant: Second\nSelected Variant: 1\n{variant:2}This Item gains bonuses from Socketed Items as though it was a Gloves",
            vec!["armour", "helmet"],
        ),
        (
            "This Item gains bonuses from Socketed Items as though it was a Gloves\nThis Item gains bonuses from Socketed Items as though it was a Helmet",
            vec!["armour", "helmet"],
        ),
    ] {
        let state = paired_before(
            &catalog,
            &source,
            &raw("Rusted Greathelm", "S", &["Caller"], body),
        );
        let selected = state
            .parser_calls
            .iter()
            .filter_map(|r| r.origin.as_ref().map(|o| o.slot_key.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            selected, keys,
            "ordered selected slots, including duplicates"
        );
    }
}
struct StopParser<'a> {
    original: reference::OriginalDependencies<'a>,
    stop: usize,
    attempted: usize,
}
impl ItemLoadProvider for StopParser<'_> {
    fn parse_modifier(&mut self, r: &ParseRequest) -> DependencyResult<ParseOutcome> {
        let index = self.attempted;
        self.attempted += 1;
        if index == self.stop {
            DependencyResult::Unavailable("test-selected parser boundary".into())
        } else {
            self.original.parse_modifier(r)
        }
    }
    fn format_line(&mut self, r: &FormatRequest) -> DependencyResult<String> {
        self.original.format_line(r)
    }
    fn lookup_unique(&mut self, r: &UniqueRequest) -> DependencyResult<Option<UniqueOutcome>> {
        self.original.lookup_unique(r)
    }
}
#[test]
fn parser_deferrals_keep_original_new_row_and_combined_row_assignment_prefixes() {
    let source = RuneSource::new();
    let catalog = custom_catalog(
        &source,
        r#"return {runes={Caller={armour={'+10 to maximum Life',type='Rune',levelReq=5,statOrder={1},bonded={'+3 to Strength',statOrder={2}}}}}}"#,
    );
    let raw = raw(
        "Rusted Greathelm",
        "S S",
        &["Caller", "Caller"],
        "{rune}{disabled}+10 to maximum Life",
    );
    let (_, error) = source.try_parse(&raw);
    assert!(error.is_none(), "{error:?}");
    let calls = source.rows("rune_calls");
    for (stop, call) in calls.iter().enumerate() {
        let mut provider = StopParser {
            original: reference::OriginalDependencies::new(&source.source.oracle),
            stop,
            attempted: 0,
        };
        let mut machine = ItemLoadMachine::new(&catalog);
        machine.apply_text(&raw, &mut provider).unwrap();
        assert_eq!(
            machine.pending().unwrap().kind,
            DependencyKind::ModifierParser
        );
        reference::compare_state(machine.state(), &call.get::<Table>("before").unwrap());
        assert_eq!(machine.state().parser_calls.len(), stop + 1);
        let actual = machine
            .state()
            .parser_calls
            .iter()
            .map(|r| (r.text.clone(), r.combined))
            .collect::<Vec<_>>();
        assert_eq!(actual, source.calls()[..=stop]);
        if let Some(origin) = &machine.state().parser_calls.last().unwrap().origin {
            assert_eq!(machine.pending().unwrap().line_index, None);
            if origin.combined {
                assert!(
                    machine
                        .state()
                        .rune_mod_lines
                        .iter()
                        .any(|r| r.rune_origins.len() == 2)
                );
            }
        }
    }
}
#[test]
fn unequal_combination_capture_count_preserves_source_error_state() {
    let source = RuneSource::new();
    let catalog = custom_catalog(
        &source,
        r#"return {runes={
      First={armour={'Adds 12.34 damage',type='Rune',levelReq=5,statOrder={1}}},
      Second={armour={'Adds 1.5 damage',type='Rune',levelReq=5,statOrder={1}}}
    }}"#,
    );
    let raw = raw("Rusted Greathelm", "S S", &["First", "Second"], "");
    let (item, error) = source.try_parse(&raw);
    let error = error.expect("original numeric-arity error");
    assert!(error.to_string().contains("local 'e'"), "{error}");
    let mut provider = reference::OriginalDependencies::new(&source.source.oracle);
    let mut machine = ItemLoadMachine::new(&catalog);
    assert!(machine.apply_text(&raw, &mut provider).is_err());
    assert_eq!(machine.status(), ItemLoadStatus::SourceError);
    reference::compare_state(machine.state(), &source.source.snapshot(&item));
    assert_eq!(machine.state().rune_mod_lines[0].line, "Adds 12.34 damage");
    assert_eq!(provider.calls, source.calls());
}
#[test]
fn ambiguous_minimum_vectors_stop_before_inventing_original_annotations() {
    let source = RuneSource::new();
    let catalog = custom_catalog(
        &source,
        r#"return {runes={
      First={armour={'+10 to maximum Life',type='Rune',levelReq=5,statOrder={1}}},
      Second={armour={'+10 to maximum Life',type='Rune',levelReq=5,statOrder={1}}}
    }}"#,
    );
    let raw = raw("Rusted Greathelm", "S", &["First"], "");
    let (_, error) = source.try_parse(&raw);
    assert!(error.is_none(), "{error:?}");
    let mut provider = reference::OriginalDependencies::new(&source.source.oracle);
    let mut machine = ItemLoadMachine::new(&catalog);
    machine.apply_text(&raw, &mut provider).unwrap();
    assert_eq!(
        machine.pending().unwrap().kind,
        DependencyKind::RuneReconstruction
    );
    assert!(
        machine
            .pending()
            .unwrap()
            .message
            .contains("multiple minimum count vectors"),
        "{:?}",
        machine.pending()
    );
    reference::compare_state(
        machine.state(),
        &source.rows("rune_updates")[0]
            .get::<Table>("after")
            .unwrap(),
    );
    assert!(machine.state().rune_mod_lines[0].rune_count.is_none());
    let source_row = source
        .source
        .before()
        .get::<Table>("runeModLines")
        .unwrap()
        .get::<Table>(1)
        .unwrap();
    assert_eq!(source_row.get::<usize>("runeCount").unwrap(), 1);
}
#[test]
fn original_classifier_precedence_and_display_scalar_lifecycle_are_observed_directly() {
    let source = RuneSource::new();
    let functions = source
        .source
        .oracle
        .lua
        .globals()
        .get::<Table>("rune_original_functions")
        .unwrap();
    let classify = functions.get::<Function>("classify").unwrap();
    for (definition, broad, specific) in [
        (
            "{base={type='Quarterstaff',subType='Warstaff',weapon={},armour={},tags={wand=true}}}",
            Some("weapon"),
            "quarterstaff",
        ),
        (
            "{base={type='Shield',subType='Evasion',armour={},tags={staff=true}}}",
            Some("armour"),
            "buckler",
        ),
        (
            "{base={type='Wand',tags={wand=true}}}",
            Some("caster"),
            "wand",
        ),
        ("{base={type='Ring',tags={}}}", None, "ring"),
        (
            "{base={type='Quarterstaff',weapon={},tags={}},socketedAugmentTypeOverride='gloves'}",
            Some("armour"),
            "gloves",
        ),
    ] {
        let item = source
            .source
            .oracle
            .lua
            .load(format!("return {definition}"))
            .eval::<Table>()
            .unwrap();
        let actual: (Option<String>, String) = classify.call(item).unwrap();
        assert_eq!(actual, (broad.map(str::to_owned), specific.into()));
    }
    let display = functions.get::<Function>("display").unwrap();
    for (effects, kind, already, expected) in [
        ("{}", "Rune", "nil", ItemNumber::Nil),
        (
            "{socketedAugmentItemEffectModifier=.25,socketedRuneEffectModifier=-.25}",
            "Rune",
            "nil",
            ItemNumber::Nil,
        ),
        (
            "{socketedAugmentItemEffectModifier=-1}",
            "Rune",
            "nil",
            ItemNumber::new(0.0),
        ),
        (
            "{socketedAugmentItemEffectModifier=.5}",
            "Rune",
            "0",
            ItemNumber::Nil,
        ),
        (
            "{socketedAugmentItemEffectModifier=.5}",
            "Idol",
            "false",
            ItemNumber::new(1.5),
        ),
        (
            "{socketedAugmentItemEffectModifier=0/0}",
            "Rune",
            "nil",
            ItemNumber::NaN,
        ),
    ] {
        let item=source.source.oracle.lua.load(format!("local item={effects};item.runeModLines={{{{augmentType='{kind}',displayValueScalar=9,socketedRuneEffectAlreadyApplied={already}}}}};return item")).eval::<Table>().unwrap();
        display.call::<()>(item.clone()).unwrap();
        let row = item
            .get::<Table>("runeModLines")
            .unwrap()
            .get::<Table>(1)
            .unwrap();
        reference::assert_number(
            reference::number(row.get("displayValueScalar").unwrap()),
            expected,
            "original display scalar",
        );
    }
}

#[test]
fn array_shaped_base_tags_are_a_table_with_absent_caster_keys() {
    let source = RuneSource::new();
    let catalog = custom_catalog(
        &source,
        r#"return {
      bases={['Amber Amulet']={tags={1}}},
      runes={Caller={amulet={'+10 to maximum Life',type='Rune',levelReq=5,statOrder={1}}}}
    }"#,
    );
    let state = paired_before(
        &catalog,
        &source,
        &raw("Amber Amulet", "S", &["Caller"], ""),
    );
    assert_eq!(state.rune_mod_lines.len(), 1);
    assert_eq!(
        state.parser_calls[0].origin.as_ref().unwrap().slot_key,
        "amulet"
    );
}

#[test]
fn tied_missing_zero_vectors_can_change_bonded_type_despite_unique_capped_counts() {
    let source = RuneSource::new();
    let catalog = custom_catalog(
        &source,
        r#"return {runes={
      First={armour={'Cannot be Frozen',type='Rune',levelReq=5,statOrder={1},bonded={'1##',statOrder={2}}}},
      Second={armour={'Cannot be Ignited',type='SoulCore',levelReq=5,statOrder={1},bonded={'1#0',statOrder={2}}}}
    }}"#,
    );
    // This labelled legal-order probe changes only iteration of the injected
    // dictionary. Complete original methods and the original table.sort run.
    source.source.oracle.lua.load(r#"
      local original=pairs
      rune_order_control=nil
      function pairs(t)
       if t==data.itemMods.Runes and rune_order_control then
        local at=0
        return function()at=at+1;local key=rune_order_control[at];if key then return key,t[key]end end
       end
       return original(t)
      end
    "#).exec().unwrap();
    let raw = raw(
        "Rusted Greathelm",
        "S S",
        &["First", "Second"],
        "This Item gains bonuses from Socketed Items as though it was a Armour",
    );
    let mut observed = Vec::new();
    for order in ["{'First','Second'}", "{'Second','First'}"] {
        source
            .source
            .oracle
            .lua
            .load(format!("rune_order_control={order}"))
            .exec()
            .unwrap();
        let (_, error) = source.try_parse(&raw);
        assert!(error.is_none(), "{error:?}");
        let rows = source.source.before().get::<Table>("runeModLines").unwrap();
        assert_eq!(rows.raw_len(), 4);
        let mut types = Vec::new();
        for row in rows.sequence_values::<Table>() {
            let row = row.unwrap();
            if row.get::<Option<bool>>("bonded").unwrap() == Some(true) {
                types.push(row.get::<String>("augmentType").unwrap());
            } else {
                assert_eq!(row.get::<usize>("runeCount").unwrap(), 1);
            }
        }
        observed.push(types);
    }
    assert_eq!(
        observed,
        vec![vec!["SoulCore", "SoulCore"], vec!["Rune", "Rune"]]
    );
    source
        .source
        .oracle
        .lua
        .globals()
        .set("rune_order_control", Value::Nil)
        .unwrap();
    let mut provider = reference::OriginalDependencies::new(&source.source.oracle);
    let mut machine = ItemLoadMachine::new(&catalog);
    machine.apply_text(&raw, &mut provider).unwrap();
    assert_eq!(
        machine.pending().unwrap().kind,
        DependencyKind::RuneReconstruction
    );
    assert!(
        machine
            .pending()
            .unwrap()
            .message
            .contains("tied source sort order"),
        "{:?}",
        machine.pending()
    );
    assert_eq!(
        machine.state().rune_mod_lines[0].rune_count,
        Some(ItemNumber::new(1.0))
    );
    assert_eq!(
        machine.state().rune_mod_lines[1].rune_count,
        Some(ItemNumber::new(1.0))
    );
    assert_eq!(
        machine.state().rune_mod_lines[2].augment_type.as_deref(),
        Some("Rune")
    );
    assert_eq!(
        machine.state().rune_mod_lines[3].augment_type.as_deref(),
        Some("SoulCore")
    );
    assert_eq!(provider.calls, source.calls());
}

#[test]
fn builtin_native_backends_rebuild_non_none_runes_through_original_annotations() {
    let snapshot = bundled_snapshot().unwrap();
    let source = RuneSource::new();
    for (base, rune) in [
        ("Twig Focus", "Greater Iron Rune"),
        ("Stocky Mitts", "Greater Desert Rune"),
    ] {
        let raw = raw(base, "S", &[rune], "+10 to maximum Life");
        let (_, error) = source.try_parse(&raw);
        assert!(error.is_none(), "{raw}: {error:?}");
        let mut provider = BuiltinItemLoadProvider::new(&snapshot);
        let mut machine = ItemLoadMachine::new(snapshot.item_loading());
        machine.apply_text(&raw, &mut provider).unwrap();
        assert_eq!(
            machine.pending().map(|p| p.kind),
            Some(DependencyKind::Assembly),
            "{raw}: {:?}",
            machine.pending()
        );
        reference::compare_state(machine.state(), &source.source.before());
        assert_eq!(
            machine
                .state()
                .parser_calls
                .iter()
                .map(|r| (r.text.clone(), r.combined))
                .collect::<Vec<_>>(),
            source.calls()
        );
        assert_eq!(machine.state().rune_mod_lines.len(), 3);
        assert!(
            machine
                .state()
                .rune_mod_lines
                .iter()
                .any(|row| row.rune_count == Some(ItemNumber::new(1.0)))
        );
        assert_eq!(
            machine.state().requirements["runeLevel"],
            ItemNumber::new(30.0)
        );
        assert_eq!(
            machine
                .state()
                .parser_calls
                .iter()
                .filter(|r| r.origin.is_some())
                .count(),
            3
        );
    }
}

#[test]
fn string_slot_selection_preserves_original_success_and_error_frontiers() {
    let source = RuneSource::new();
    let catalog = custom_catalog(
        &source,
        r#"return {runes={
          Caller={armour={'+10 to maximum Life',type='Rune',levelReq=5,statOrder={1}}},
          Inert={weapon='Unselected string slot'}
        }}"#,
    );
    let state = paired_before(
        &catalog,
        &source,
        &raw("Rusted Greathelm", "S", &["Caller"], ""),
    );
    assert_eq!(state.rune_mod_lines.len(), 1);
    assert_eq!(state.rune_mod_lines[0].line, "+10 to maximum Life");
    assert_eq!(
        state.rune_mod_lines[0].rune_count,
        Some(ItemNumber::new(1.0))
    );
    assert_eq!(state.requirements["runeLevel"], ItemNumber::new(5.0));
    let catalog = custom_catalog(
        &source,
        r#"return {runes={Caller={armour='Selected string slot'}}}"#,
    );
    let raw = raw("Rusted Greathelm", "S", &["Caller"], "");
    let (item, error) = source.try_parse(&raw);
    let error = error.expect("original selected string slot reaches ipairs");
    assert!(error.to_string().contains("ipairs"), "{error}");
    let mut provider = reference::OriginalDependencies::new(&source.source.oracle);
    let mut machine = ItemLoadMachine::new(&catalog);
    assert!(machine.apply_text(&raw, &mut provider).is_err());
    assert_eq!(machine.status(), ItemLoadStatus::SourceError);
    reference::compare_state(machine.state(), &source.source.snapshot(&item));
    assert_eq!(provider.calls, source.calls());
}
