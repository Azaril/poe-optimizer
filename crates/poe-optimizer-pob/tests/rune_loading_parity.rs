//! Complete original rune loading methods remain the independent oracle.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/rune_order_witness.rs"]
mod order_witness;
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
        let (original_item, error) = source.try_parse(&raw);
        assert!(error.is_none(), "{raw}: {error:?}");
        let before = source.source.before();
        let mut original_parser = reference::OriginalDependencies::new(&source.source.oracle);
        let mut preassembly = ItemLoadMachine::new(snapshot.item_loading());
        preassembly.apply_text(&raw, &mut original_parser).unwrap();
        assert_eq!(
            preassembly.pending().map(|p| p.kind),
            Some(DependencyKind::Assembly)
        );
        reference::compare_state(preassembly.state(), &before);
        assert_eq!(original_parser.calls, source.calls());
        let mut provider = BuiltinItemLoadProvider::new(&snapshot);
        let mut machine = ItemLoadMachine::new(snapshot.item_loading());
        machine.apply_text(&raw, &mut provider).unwrap();
        assert_eq!(
            machine.status(),
            ItemLoadStatus::Complete,
            "{raw}: {:?}",
            machine.pending()
        );
        assert!(machine.pending().is_none());
        assert!(
            machine
                .assembly_progress()
                .is_some_and(|item| item.is_complete())
        );
        assert!(
            machine.assembled().is_none(),
            "ParseRaw assembly is not final Load registration"
        );
        compare_completed_parse_projection(
            &source,
            &machine,
            &before,
            &source.source.snapshot(&original_item),
            &original_item,
        );
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

/// ItemState retains parsed header socket groups; BuildModList's rewritten
/// sockets live in the authoritative owned graph. Compare those two explicitly
/// declared stages without changing the existing exact compare_state contract.
fn compare_completed_parse_projection(
    source: &RuneSource,
    machine: &ItemLoadMachine<'_>,
    before: &Table,
    after: &Table,
    original_item: &Table,
) -> serde_json::Value {
    use poe_optimizer_import::item_loading::assembly::AssemblyValue;
    let graph = machine
        .assembly_progress()
        .expect("owned ParseRaw assembly progress");
    assert!(graph.is_complete());
    let source_sockets: Table = original_item.raw_get("sockets").unwrap();
    assert!(source_sockets.metatable().is_none());
    let n = source_sockets.raw_len();
    assert!(n <= 128);
    let native_id = graph
        .field(graph.root(), "sockets")
        .and_then(AssemblyValue::as_table)
        .expect("owned sockets table");
    let native_sockets = graph.table(native_id).unwrap();
    assert!(native_sockets.fields.is_empty());
    assert_eq!(native_sockets.indexed.len(), n);
    let mut source_ids = Vec::new();
    let mut native_ids = Vec::new();
    let mut groups = Vec::new();
    let mut visited = 0;
    for entry in source_sockets.pairs::<Value, Value>() {
        let (key, _) = entry.unwrap();
        visited += 1;
        assert!(visited <= 128);
        let index = match key {
            Value::Integer(i) => i as f64,
            Value::Number(v) => v,
            other => panic!("unexpected source socket key {other:?}"),
        };
        assert!(index.is_finite() && index.fract() == 0.0 && index >= 1.0 && index <= n as f64);
    }
    assert_eq!(visited, n);
    for i in 1..=n {
        let source_row: Table = source_sockets.raw_get(i).unwrap();
        assert!(source_row.metatable().is_none());
        assert_eq!(
            source_row.clone().pairs::<Value, Value>().take(2).count(),
            1
        );
        let source_group: f64 = source_row.raw_get("group").unwrap();
        assert!(source_group.is_finite());
        let row_id = graph
            .index(native_id, i as i64)
            .and_then(AssemblyValue::as_table)
            .expect("owned socket row");
        let row = graph.table(row_id).unwrap();
        assert!(row.indexed.is_empty());
        assert_eq!(row.fields.len(), 1);
        let native_group = graph
            .field(row_id, "group")
            .and_then(AssemblyValue::as_number)
            .expect("owned group");
        assert_eq!(
            native_group.to_bits(),
            source_group.to_bits(),
            "postassembly socket group"
        );
        source_ids.push(source_row.to_pointer());
        native_ids.push(row_id);
        groups.push(source_group);
    }
    for i in 0..n {
        for j in 0..n {
            assert_eq!(
                source_ids[i] == source_ids[j],
                native_ids[i] == native_ids[j],
                "whole socket row aliases"
            );
        }
    }
    // This is a fresh observation root, not a source Item or native dependency.
    // All fields except the documented retained header sockets use the actual
    // after-ParseRaw snapshot. Keep both original raw snapshots in the receipt.
    let projection = source.source.oracle.lua.create_table().unwrap();
    for entry in after.pairs::<Value, Value>() {
        let (key, value) = entry.unwrap();
        projection.raw_set(key, value).unwrap();
    }
    projection
        .raw_set("sockets", before.raw_get::<Table>("sockets").unwrap())
        .unwrap();
    reference::compare_state(machine.state(), &projection);
    serde_json::json!({
        "scope":"ItemState parsed-header sockets + other post-ParseRaw loading fields; separate complete owned sockets graph compared against live original sockets",
        "parsed_header_socket_groups":machine.state().sockets,
        "source_postassembly_socket_groups":groups,
        "owned_socket_values_keys_and_row_aliases_equal":true,
        "whole_item_alias_parity_claim":false
    })
}

fn strict_order_case(
    source: &RuneSource,
    catalog: &ItemLoadingCatalog,
    raw: &str,
    expected_runes: &[&str],
    expected_level: f64,
    builtin: bool,
) -> serde_json::Value {
    use poe_optimizer_engine::item_runes::{
        RuneBudget, VectorPolicy, find_combination, has_strict_vector_order,
    };
    let lua = &source.source.oracle.lua;
    let class: Table = lua
        .globals()
        .get::<Table>("common")
        .unwrap()
        .get::<Table>("classes")
        .unwrap()
        .get("Item")
        .unwrap();
    let entry: Function = class.get("ParseRaw").unwrap();
    let target = order_witness::original_parse(lua, &entry);
    let update_entry: Function = class.get("UpdateRunes").unwrap();
    let update: Function = lua
        .globals()
        .get::<Table>("rune_original_functions")
        .unwrap()
        .get("update")
        .unwrap();
    assert_eq!(
        update.info().source.as_deref(),
        Some("@src/Classes/Item.lua")
    );
    assert_eq!(
        (update.info().line_defined, update.info().last_line_defined),
        (Some(2106), Some(2178))
    );
    let item = source.source.oracle.parse("");
    source.clear();
    lua.globals().set("rune_enabled", true).unwrap();
    let captured = order_witness::capture(lua, &entry, &target, &update, &item, raw);
    lua.globals().set("rune_enabled", false).unwrap();
    let (witness, source_error) = captured.unwrap();
    assert!(
        source_error.is_none(),
        "complete original ParseRaw failed: {source_error:?}"
    );
    assert!(
        witness.calls_complete && witness.exact_function_rechecked && witness.prior_hook_restored
    );
    assert_eq!(class.get::<Function>("ParseRaw").unwrap(), entry);
    assert_eq!(class.get::<Function>("UpdateRunes").unwrap(), update_entry);
    assert_eq!(
        witness.update_calls, 1,
        "full actual UpdateRunes call count"
    );
    let calls = source.calls();
    let before = source.source.before();
    let after = source.source.snapshot(&item);
    let regular = witness
        .searches
        .iter()
        .filter(|r| r.source_line == 1551)
        .collect::<Vec<_>>();
    assert_eq!(regular.len(), 1, "one regular combined rune line");
    let first = regular[0];
    let vectors = first
        .candidates
        .iter()
        .map(|r| r.values.as_slice())
        .collect::<Vec<_>>();
    let policy = VectorPolicy {
        missing_value: catalog.policy().rune_loading.vector_default,
        epsilon: catalog.policy().rune_loading.vector_tolerance,
    };
    assert!(has_strict_vector_order(&vectors, policy, &mut RuneBudget::default()).unwrap());
    let result = find_combination(
        &vectors,
        &first.target,
        2.0,
        None,
        policy,
        &mut RuneBudget::default(),
    )
    .unwrap()
    .unwrap();
    assert!(
        result.ambiguous_minimum,
        "this case needs the unique-order admission proof"
    );
    assert_eq!(
        Some(&result.counts),
        first.counts.as_ref(),
        "actual first DFS touched counts, including zero entries"
    );
    assert_eq!(Some(result.count), first.count);
    assert_eq!(first.count, Some(2));
    let selected = first
        .candidates
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            first
                .counts
                .as_ref()
                .unwrap()
                .get(&(i + 1))
                .filter(|&&n| n > 0)
                .map(|&n| (r.name.clone(), n))
        })
        .collect::<Vec<_>>();
    assert_eq!(selected.iter().map(|(_, n)| n).sum::<usize>(), 2);
    assert_eq!(
        first
            .candidates
            .iter()
            .map(|r| r.values.clone())
            .collect::<Vec<_>>(),
        vec![vec![20.0], vec![18.0], vec![16.0], vec![14.0]]
    );
    let saved = item
        .get::<Table>("runes")
        .unwrap()
        .sequence_values::<String>()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    assert_eq!(saved, expected_runes);
    assert_eq!(
        item.get::<Table>("requirements")
            .unwrap()
            .get::<f64>("runeLevel")
            .unwrap(),
        expected_level
    );
    if first.should_fix {
        assert!(first.saved_runes_at_search.is_empty());
    } else {
        assert_eq!(first.saved_runes_at_search, expected_runes);
    }
    let mut provider = reference::OriginalDependencies::new(&source.source.oracle);
    let mut machine = ItemLoadMachine::new(catalog);
    machine.apply_text(raw, &mut provider).unwrap();
    assert_eq!(
        machine.pending().map(|p| p.kind),
        Some(DependencyKind::Assembly),
        "{:?}",
        machine.pending()
    );
    reference::compare_state(machine.state(), &before);
    assert_eq!(
        provider.calls, calls,
        "ordered parser text/combined parameter projection"
    );
    let mut completed_projection = serde_json::Value::Null;
    if builtin {
        let snapshot = bundled_snapshot().unwrap();
        let mut provider = BuiltinItemLoadProvider::new(&snapshot);
        let mut machine = ItemLoadMachine::new(snapshot.item_loading());
        machine.apply_text(raw, &mut provider).unwrap();
        assert_eq!(
            machine.status(),
            ItemLoadStatus::Complete,
            "{:?}",
            machine.pending()
        );
        assert!(machine.pending().is_none());
        assert!(
            machine
                .assembly_progress()
                .is_some_and(|item| item.is_complete())
        );
        assert!(
            machine.assembled().is_none(),
            "ParseRaw assembly is not final Load registration"
        );
        completed_projection =
            compare_completed_parse_projection(source, &machine, &before, &after, &item);
        assert_eq!(
            machine
                .state()
                .parser_calls
                .iter()
                .map(|r| (r.text.clone(), r.combined))
                .collect::<Vec<_>>(),
            calls
        );
    }
    // Both controls execute complete original methods, with the scoped local hook
    // absent. This finite projection has the historical harness's alias limits.
    let control = source.source.oracle.parse(raw);
    assert_eq!(
        canonical(Value::Table(after.clone())),
        canonical(Value::Table(source.source.snapshot(&control)))
    );
    let updates = source.rows("rune_updates");
    assert_eq!(updates.len(), 1);
    assert!(updates[0].get::<bool>("ok").unwrap());
    use sha2::{Digest, Sha256};
    let source_text = runtime::verified("src/Classes/Item.lua").unwrap();
    serde_json::json!({
        "source":{"path":"src/Classes/Item.lua","sha256":format!("{:x}",Sha256::digest(source_text.as_bytes())),"parse_span":[468,1803],"update_span":[2106,2178],"identity":"actual retained original Functions via delegating capture chain"},
        "scope":"complete original ParseRaw/UpdateRunes; source-parser lane compares pre-assembly loading state; built-in lane compares completed ParseRaw loading projection and requires complete owned assembly progress; no final Load registration",
        "observer":"bounded exact-function coroutine hook; interpreted, no warm claim",
        "parser_calls_scope":"ordered text and combined boolean parameters; not actual call arity",
        "finite_projection_alias_equivalence":false,
        "owned_assembly_parity_in_separate_item_assembly_target":true,
        "raw":raw,"witness":witness,"selected_first_counts":selected,
        "saved_runes":saved,"rune_level":expected_level,"parser_calls":calls,
        "source_before_assembly":canonical(Value::Table(before)),
        "source_after_parse":canonical(Value::Table(after)),
        "update_before":canonical(updates[0].get("before").unwrap()),
        "update_after":canonical(updates[0].get("after").unwrap()),
        "unhooked_control_finite_projection_equal":true,
        "original_parser_lane_preassembly_equal":true,
        "builtin_lane_complete_parse_loading_projection_equal":builtin,
        "builtin_lane_final_registration_claim":false,
        "completed_loading_projection":completed_projection
    })
}
fn write_order_report(name: &str, value: &serde_json::Value) {
    let output = std::env::var_os("POE_RUNE_ORDER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| runtime::repository().join("runs/r2af-rune-order-01/source-order"));
    std::fs::create_dir_all(&output).unwrap();
    std::fs::write(
        output.join(format!("{name}.json")),
        serde_json::to_vec_pretty(value).unwrap(),
    )
    .unwrap();
}
#[test]
fn original_saved_item_runes_keep_headers_despite_multiple_ordered_minima() {
    use sha2::{Digest, Sha256};
    let snapshot = bundled_snapshot().unwrap();
    let source = RuneSource::new();
    for (build, id) in [("build-01", "3"), ("build-02", "6")] {
        let relative = format!("tests/fixtures/builds/breadth-20260908/{build}.xml");
        let bytes = std::fs::read(runtime::repository().join(&relative)).unwrap();
        let xml = std::str::from_utf8(&bytes).unwrap();
        let document = roxmltree::Document::parse(xml).unwrap();
        let items = document
            .descendants()
            .filter(|n| n.has_tag_name("Item") && n.attribute("id") == Some(id))
            .collect::<Vec<_>>();
        assert_eq!(items.len(), 1);
        let item = items[0];
        let raw = item
            .children()
            .filter(|n| n.is_text())
            .filter_map(|n| n.text())
            .collect::<String>();
        assert_eq!(
            raw.lines().filter(|line| line.starts_with("Rune:")).count(),
            2
        );
        let mut report = strict_order_case(
            &source,
            snapshot.item_loading(),
            &raw,
            &["Greater Iron Rune", "Greater Iron Rune"],
            30.0,
            true,
        );
        report["input"] = serde_json::json!({"kind":"unaltered actual XML item text","path":relative,"file_sha256":format!("{:x}",Sha256::digest(&bytes)),"item_id":id,"item_range":[item.range().start,item.range().end],"raw_sha256":format!("{:x}",Sha256::digest(raw.as_bytes()))});
        assert_eq!(
            report["selected_first_counts"],
            serde_json::json!([["Perfect Iron Rune", 1], ["Iron Rune", 1]])
        );
        write_order_report(&format!("{build}-item-{id}-saved"), &report);
        let derived = raw
            .split_inclusive('\n')
            .filter(|line| !line.starts_with("Rune:"))
            .collect::<String>();
        let mut report = strict_order_case(
            &source,
            snapshot.item_loading(),
            &derived,
            &["Perfect Iron Rune", "Iron Rune"],
            50.0,
            true,
        );
        report["input"] = serde_json::json!({"kind":"derived by removing only saved Rune header lines","source_path":relative,"source_item_id":id,"source_raw_sha256":format!("{:x}",Sha256::digest(raw.as_bytes())),"raw_sha256":format!("{:x}",Sha256::digest(derived.as_bytes()))});
        write_order_report(&format!("{build}-item-{id}-headerless"), &report);
    }
}
#[test]
fn unique_vector_order_is_independent_of_custom_name_and_insertion_order() {
    for (index, names) in [
        "{'Alpha','Zulu','Middle','Lower'}",
        "{'Lower','Middle','Zulu','Alpha'}",
    ]
    .iter()
    .enumerate()
    {
        let source = RuneSource::new();
        let catalog = custom_catalog(
            &source,
            &format!(
                r#"
          local out={{}}; local amounts={{Alpha=18,Zulu=20,Middle=16,Lower=14}}
          for _,name in ipairs({names}) do
            out[name]={{armour={{'+'..amounts[name]..' to maximum Life',type='Rune',levelReq=name=='Zulu' and 50 or 30,statOrder={{1}}}}}}
          end
          return {{runes=out}}
        "#
            ),
        );
        for (label, runes, expected, level) in [
            (
                "saved",
                vec!["Alpha", "Alpha"],
                vec!["Alpha", "Alpha"],
                30.0,
            ),
            ("headerless", vec![], vec!["Zulu", "Middle"], 50.0),
        ] {
            let raw = raw(
                "Rusted Greathelm",
                "S S",
                &runes,
                "Implicits: 1\n{enchant}{rune}+36 to maximum Life",
            );
            let mut report = strict_order_case(&source, &catalog, &raw, &expected, level, false);
            assert_eq!(
                report["selected_first_counts"],
                serde_json::json!([["Zulu", 1], ["Middle", 1]])
            );
            report["input"] = serde_json::json!({"kind":"test-supplied source-shaped rune catalog and raw text; complete original methods","insertion_recipe":names,"header_kind":label});
            write_order_report(&format!("custom-order-{index}-{label}"), &report);
        }
    }
}
