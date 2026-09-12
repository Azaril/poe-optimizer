use super::super::super::SourceCopyWitness;
use super::*;
use mlua::MultiValue;
const COPY: &str = r#"function copyTable(tbl, noRecurse)
    local out = {}
    for k,v in pairs(tbl) do
        if not noRecurse and type(v) == 'table' then out[k] = copyTable(v)
        else out[k] = v end
    end
    return out
end"#;
const SOURCE: &str = r#"local function parseMod(tagList, repetitions)
    local modList = {}
    for i=1,repetitions do
        local name='same'
        modList[i] = {
            name=name,
            unpack(tagList)
        }
        modList[i].seen=true
    end
    return modList
end
return function(tagList,repetitions)
    local discarded=parseMod(tagList,repetitions)
    local result=parseMod(tagList,repetitions)
    return copyTable(result)
end, parseMod"#;
fn setup() -> (
    Lua,
    SourceCopyWitness,
    Function,
    Function,
    Function,
    ProducerTailConfig,
) {
    let lua = unsafe { Lua::unsafe_new() };
    let witness = SourceCopyWitness::before_source(&lua).unwrap();
    lua.load(COPY).exec().unwrap();
    let copy: Function = lua.globals().raw_get("copyTable").unwrap();
    let (parser, target): (Function, Function) = lua
        .load(SOURCE)
        .set_name("@tail-fixture.lua")
        .eval()
        .unwrap();
    // Tests read their actual compiled TSETM register, without assuming the
    // synthetic fixture shares the original parser's stack layout.
    let (constructor_slot, count): (usize, usize) = lua
        .load(
            r#"
        local target=...
        local util=require('jit.util')
        local found,count=0,0
        for pc=1,util.funcinfo(target).bytecodes-1 do
            local word=util.funcbc(target,pc)
            if word % 256 == 63 then
                found=math.floor(word / 256) % 256
                count=count+1
            end
        end
        return found,count
    "#,
        )
        .call(target.clone())
        .unwrap();
    assert_eq!(count, 1, "fixture has one actual TSETM");
    let config = ProducerTailConfig {
        source_line: 7,
        argument_local: "tagList".into(),
        constructor_slot,
        limits: ProducerTailLimits::default(),
    };
    (lua, witness, copy, parser, target, config)
}
fn no_hook(lua: &Lua) {
    let debug: Table = lua.globals().raw_get("debug").unwrap();
    let gethook: Function = debug.raw_get("gethook").unwrap();
    let result: MultiValue = gethook.call(()).unwrap();
    assert!(matches!(result.front(), None | Some(Value::Nil)));
}
#[test]
fn derived_tail_keeps_zero_one_multiple_nil_values_and_exact_row_identities() {
    let (lua, witness, copy, parser, target, config) = setup();
    let plain = witness.bind_producer(&parser, &target, 9).unwrap();
    let binding = witness.bind_producer_tail(&plain, config).unwrap();
    let original_unpack: Function = lua.globals().raw_get("unpack").unwrap();
    for text in [
        "return {}",
        "return {7}",
        "return {false,7,9}",
        "return {nil,false,nil,7}",
    ] {
        let input: Table = lua.load(text).eval().unwrap();
        let expected: MultiValue = original_unpack.call(input.clone()).unwrap();
        if text.contains("nil") {
            assert_eq!(expected.len(), 4);
            assert_eq!(expected[0], Value::Nil);
            assert_eq!(expected[2], Value::Nil);
        }
        let observed = witness
            .call_with_producer(
                &lua,
                &parser,
                &copy,
                &binding,
                MultiValue::from_vec(vec![Value::Table(input.clone()), Value::Integer(2)]),
            )
            .unwrap();
        assert!(observed.result.is_ok());
        assert_eq!(observed.producer.activations.len(), 2);
        assert_eq!(observed.producer.stores.len(), 4);
        assert_eq!(observed.producer.tails.len(), 4);
        assert_eq!(observed.producer.tail_entries.len(), 4);
        for (index, tail) in observed.producer.tails.iter().enumerate() {
            assert_eq!(tail.ordinal, index);
            let entry = &observed.producer.tail_entries[tail.line_entry_ordinal];
            assert_eq!(entry.consumed_by_tail, Some(index));
            assert!(!entry.abandoned);
            assert_eq!(entry.constructor, tail.constructor);
            assert!(entry.observed_event < tail.observed_event);
            assert_eq!(tail.activation, index / 2);
            assert_eq!(tail.caller, target);
            assert_eq!(tail.callee, original_unpack);
            assert_eq!(tail.argument, input);
            assert_eq!(tail.argument_slot, 1);
            assert_eq!(tail.argument_local, "tagList");
            assert_eq!(tail.raw_length, expected.len());
            assert!(tail.values.iter().eq(expected.iter()));
            let store = &observed.producer.stores[index];
            assert_eq!(tail.constructor, store.table);
            assert!(tail.observed_event < store.observed_event);
            if index > 0 {
                assert_ne!(
                    tail.constructor,
                    observed.producer.tails[index - 1].constructor
                );
            }
        }
        no_hook(&lua);
    }
    let disabled = witness
        .call_with_producer(
            &lua,
            &parser,
            &copy,
            &plain,
            MultiValue::from_vec(vec![
                Value::Table(lua.create_table().unwrap()),
                Value::Integer(1),
            ]),
        )
        .unwrap();
    assert!(disabled.result.is_ok());
    assert!(disabled.producer.tails.is_empty());
}
#[test]
fn tail_requires_retained_callee_exact_immediate_caller_and_configured_line() {
    let (lua, witness, copy, parser, target, config) = setup();
    let plain = witness.bind_producer(&parser, &target, 9).unwrap();
    let binding = witness.bind_producer_tail(&plain, config.clone()).unwrap();
    let (_, other): (Function, Function) = lua
        .load(SOURCE)
        .set_name("@tail-fixture.lua")
        .eval()
        .unwrap();
    assert_eq!(other.info().source, target.info().source);
    assert_ne!(other, target);
    lua.globals().raw_set("other", other).unwrap();
    let mixed: Function = lua
        .load(
            r#"local parseMod=...
        return function(input)
            local unrelated=other(input,1)
            local own=parseMod(input,1)
            return copyTable(own)
        end"#,
        )
        .call(target.clone())
        .unwrap();
    let mixed_binding = witness
        .bind_producer_tail(
            &witness.bind_producer(&mixed, &target, 9).unwrap(),
            config.clone(),
        )
        .unwrap();
    let input: Table = lua.load("return {false,9}").eval().unwrap();
    let observed = witness
        .call_with_producer(
            &lua,
            &mixed,
            &copy,
            &mixed_binding,
            MultiValue::from_vec(vec![Value::Table(input.clone())]),
        )
        .unwrap();
    assert!(observed.result.is_ok());
    assert_eq!(
        observed.producer.tails.len(),
        1,
        "same source in another Function is excluded"
    );
    let mut wrong_line = config;
    wrong_line.source_line = 6;
    let wrong = witness.bind_producer_tail(&plain, wrong_line).unwrap();
    let observed = witness
        .call_with_producer(
            &lua,
            &parser,
            &copy,
            &wrong,
            MultiValue::from_vec(vec![Value::Table(input.clone()), Value::Integer(1)]),
        )
        .unwrap();
    assert!(observed.result.is_ok());
    assert!(observed.producer.tails.is_empty());
    lua.load("unpack=function(t) return 17 end").exec().unwrap();
    let rebound = witness.call_with_producer(
        &lua,
        &parser,
        &copy,
        &binding,
        MultiValue::from_vec(vec![Value::Table(input), Value::Integer(1)]),
    );
    assert!(
        rebound
            .unwrap_err()
            .to_string()
            .contains("plain environment and raw original unpack")
    );
    no_hook(&lua);
}
#[test]
fn tail_bounds_are_sticky_through_protected_errors_and_remove_the_hook() {
    let (lua, witness, copy, _, target, config) = setup();
    let parser: Function = lua
        .load(
            r#"local parseMod=...
        return function(input)
            local ok,result=pcall(parseMod,input,2)
            return ok,result
        end"#,
        )
        .call(target.clone())
        .unwrap();
    let plain = witness.bind_producer(&parser, &target, 9).unwrap();
    for (limits, input, needle) in [
        (
            ProducerTailLimits {
                max_values: 0,
                ..ProducerTailLimits::default()
            },
            "return {1}",
            "value bound",
        ),
        (
            ProducerTailLimits {
                max_events: 1,
                ..ProducerTailLimits::default()
            },
            "return {}",
            "event bound",
        ),
        (
            ProducerTailLimits {
                max_text_bytes: 32,
                ..ProducerTailLimits::default()
            },
            "return {string.rep('x',64)}",
            "text bound",
        ),
    ] {
        let mut bounded = config.clone();
        bounded.limits = limits;
        let binding = witness.bind_producer_tail(&plain, bounded).unwrap();
        let input: Table = lua.load(input).eval().unwrap();
        let result = witness.call_with_producer(
            &lua,
            &parser,
            &copy,
            &binding,
            MultiValue::from_vec(vec![Value::Table(input)]),
        );
        assert!(result.unwrap_err().to_string().contains(needle));
        no_hook(&lua);
    }
    let binding = witness.bind_producer_tail(&plain, config).unwrap();
    let source_error = witness
        .call_with_producer(
            &lua,
            &parser,
            &copy,
            &binding,
            MultiValue::from_vec(vec![Value::Table(lua.create_table().unwrap())]),
        )
        .unwrap();
    assert!(
        source_error.result.is_ok(),
        "prior sticky state is scoped to its call"
    );
    assert_eq!(source_error.producer.tails.len(), 2);
    no_hook(&lua);
}

#[test]
fn mutated_tail_bindings_revalidate_limits_line_and_original_callee_before_call() {
    let (lua, witness, copy, parser, target, config) = setup();
    let plain = witness.bind_producer(&parser, &target, 9).unwrap();
    let binding = witness.bind_producer_tail(&plain, config).unwrap();
    let replacement: Function = lua
        .load("return function() error('must not run') end")
        .eval()
        .unwrap();
    let mut changed = binding.clone();
    changed.tail.as_mut().unwrap().original_unpack = replacement;
    let mut oversized = binding.clone();
    oversized.tail.as_mut().unwrap().config.limits.max_values = usize::MAX;
    let mut text = binding.clone();
    text.tail.as_mut().unwrap().config.argument_local = "x".repeat(257);
    let mut line = binding.clone();
    line.tail.as_mut().unwrap().config.source_line = 500;
    for bad in [changed, oversized, text, line] {
        let error = witness
            .call_with_producer(&lua, &parser, &copy, &bad, MultiValue::new())
            .unwrap_err();
        assert!(error.to_string().contains("producer tail"), "{error}");
        no_hook(&lua);
    }
    let good = witness
        .call_with_producer(
            &lua,
            &parser,
            &copy,
            &binding,
            MultiValue::from_vec(vec![
                Value::Table(lua.create_table().unwrap()),
                Value::Integer(1),
            ]),
        )
        .unwrap();
    assert!(good.result.is_ok());
    assert_eq!(good.producer.tails.len(), 2);
    no_hook(&lua);
}

#[test]
fn pre_gget_guard_rejects_self_removing_index_before_hidden_original_unpack_call() {
    let (lua, witness, copy, parser, target, config) = setup();
    let binding = witness
        .bind_producer_tail(&witness.bind_producer(&parser, &target, 9).unwrap(), config)
        .unwrap();
    lua.globals().raw_set("hidden_index_calls", 0).unwrap();
    let environment: Table = lua
        .load(
            r#"
        local original_unpack=unpack
        local environment={}
        setmetatable(environment,{__index=function(t,key)
            hidden_index_calls=hidden_index_calls+1
            setmetatable(t,nil)
            return original_unpack({function(_) return nil end})
        end})
        return environment
    "#,
        )
        .eval()
        .unwrap();
    assert!(target.set_environment(environment).unwrap());
    let error = witness
        .call_with_producer(
            &lua,
            &parser,
            &copy,
            &binding,
            MultiValue::from_vec(vec![
                Value::Table(lua.create_table().unwrap()),
                Value::Integer(1),
            ]),
        )
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("plain environment and raw original unpack"),
        "{error}"
    );
    assert_eq!(
        lua.globals().raw_get::<i64>("hidden_index_calls").unwrap(),
        0,
        "the proof guard must run before GGET can invoke and hide its metamethod"
    );
    no_hook(&lua);
    assert!(target.set_environment(lua.globals()).unwrap());
    let valid = witness
        .call_with_producer(
            &lua,
            &parser,
            &copy,
            &binding,
            MultiValue::from_vec(vec![
                Value::Table(lua.create_table().unwrap()),
                Value::Integer(1),
            ]),
        )
        .unwrap();
    assert!(valid.result.is_ok());
    assert_eq!(valid.producer.tails.len(), 2);
    no_hook(&lua);
}
