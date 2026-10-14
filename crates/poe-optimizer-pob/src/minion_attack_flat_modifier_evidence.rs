//! Bounded original parser/delivery/query execution; no native or item availability claim.
use super::*;
use mlua::LuaSerdeExt;
#[path = "../../../tests/support/owned_minion_attack_flat_modifier_evidence.rs"]
mod evidence;

const QUERY: &str = r#"
return function(lines,cfg,fullLife,deliver)
    local origin=new('ModList'):ModList()
    local all={}
    for _,line in ipairs(lines) do
        local records,remaining=modLib.parseMod(line)
        assert(records and not remaining,'incomplete original parse')
        for _,record in ipairs(records) do
            origin:AddMod(record)
            all[#all+1]=record
        end
    end
    local before=copyTable(all,true)
    local contextBefore=copyTable(cfg,true)
    local values=origin:List(cfg,'MinionModifier')
    local parent=new('ModDB'):ModDB()
    parent.conditions.FullLife=fullLife
    local recipient={modDB=parent}
    deliver(origin,cfg,recipient)
    local child=new('ModList'):ModList(parent)
    local raw={}
    local identity=true
    local seen={}
    for _,value in ipairs(values) do
        local mod=value.mod
        raw[#raw+1]=mod
        assert(not seen[mod],'distinct parser inputs must retain distinct records')
        seen[mod]=true
        local found=0
        for _,actual in ipairs(parent.mods[mod.name] or {}) do
            if actual==mod then found=found+1 end
        end
        identity=identity and found==1
    end
    local function query(store,context)
        local minimum=store:Sum('BASE',context,'PhysicalMin')
        local maximum=store:Sum('BASE',context,'PhysicalMax')
        local mins=store:Tabulate('BASE',context,'PhysicalMin')
        local maxs=store:Tabulate('BASE',context,'PhysicalMax')
        local same=true
        for _,rows in ipairs({mins,maxs}) do
            for _,row in ipairs(rows) do same=same and seen[row.mod]==true end
        end
        return {minimum=minimum,maximum=maximum,minimum_records=mins,maximum_records=maxs,
            minimum_count=#mins,maximum_count=#maxs,original_record_identity=same}
    end
    local contrast=copyTable(cfg,true)
    assert(AND64(contrast.keywordFlags,KeywordFlag.Attack)==KeywordFlag.Attack)
    contrast.keywordFlags=contrast.keywordFlags-KeywordFlag.Attack
    local direct=query(parent,cfg)
    local inherited=query(child,cfg)
    local again=query(child,cfg)
    local repeatEqual=direct.minimum==again.minimum and direct.maximum==again.maximum
        and direct.minimum_count==again.minimum_count and direct.maximum_count==again.maximum_count
    return {records_before=before,records_after=all,context_before=contextBefore,context_after=cfg,
        outer_list=values,raw=raw,original_payload_identity=identity,parent_identity=child.parent==parent,
        direct=direct,inherited=inherited,non_attack_context=contrast,
        non_attack_direct=query(parent,contrast),non_attack_inherited=query(child,contrast),
        query_repeat_equal=repeatEqual}
end
"#;

fn capture(provided: &BTreeMap<String, String>) -> Result<Json> {
    let (lua, order, ()) =
        crate::unique_requirements_extract::host_with_observer(provided, |_| Ok(()))?;
    let original = source(provided, "src/Modules/CalcPerform.lua")?;
    let body = crate::game_data::section(
        original,
        "local function addMinionModifiers(",
        "\nlocal function setSpectreSource",
    )?;
    let first = original[..original.find(body).unwrap()]
        .bytes()
        .filter(|b| *b == b'\n')
        .count()
        + 1;
    let deliver: Function = lua
        .load(format!(
            "{}{}\nreturn addMinionModifiers",
            "\n".repeat(first - 1),
            body
        ))
        .set_name("@src/Modules/CalcPerform.lua")
        .eval()?;
    let query: Function = lua
        .load(QUERY)
        .set_name("@owned-test/minion-attack-flat-query.lua")
        .eval()?;
    let data: Table = lua.globals().get("data")?;
    let mods: Table = data.get("itemMods")?;
    let exclusive: Table = mods.get("Exclusive")?;
    let declarations = evidence::FAMILIES
        .into_iter()
        .map(|(id, _, bounds)| {
            Ok(json!({"id":id,"bounds":bounds,"body":plain(exclusive.raw_get(id)?,0)?}))
        })
        .collect::<Result<Vec<_>>>()?;
    let prior = evidence::prior();
    let source_context = &prior["observations"][0];
    let projection = &source_context["cfg"];
    let mut cfg = projection["fields"].clone();
    cfg["skillCond"] = projection["skill_conditions"].clone();
    cfg["skillStats"] = projection["skill_stats"].clone();
    cfg["skillGem"] = projection["skill_gem"].clone();
    cfg["skillGemTags"] = projection["skill_gem_tags"].clone();
    // No query in the admitted family reads these object identities. Keep their
    // observed plain metadata, never pretend to rebuild the original Skill VM.
    cfg["grantedEffect"] = projection["granted_effect"].clone();
    let mut rows = Vec::new();
    for case in evidence::cases() {
        let context: Table = lua.to_value(&cfg)?.as_table().unwrap().clone();
        let types = lua.create_table()?;
        for row in projection["skill_types"].as_array().unwrap() {
            types.raw_set(row["id"].as_u64().unwrap(), row["value"].as_bool().unwrap())?;
        }
        context.raw_set("skillTypes", types)?;
        let result: Table = query.call((
            lua.to_value(&case["lines"])?,
            context,
            case["full_life"].as_bool().unwrap(),
            deliver.clone(),
        ))?;
        let mut observed = plain(Value::Table(result.clone()), 0)?;
        let raw: Table = result.raw_get("raw")?;
        observed["raw"] = json!(
            raw.sequence_values::<Value>()
                .map(|r| plain(r?, 0))
                .collect::<Result<Vec<_>>>()?
        );
        rows.push(json!({"input":case,"result":observed}));
    }
    let order = order.lock().unwrap().clone();
    for required in [
        "src/Classes/ModStore.lua",
        "src/Classes/ModList.lua",
        "src/Classes/ModDB.lua",
        "src/Modules/ModParser.lua",
        "src/Data/ModItemExclusive.lua",
    ] {
        assert!(order.iter().any(|p| p == required), "{required}");
    }
    let files: BTreeMap<_, _> = order
        .iter()
        .map(|p| (p.clone(), hash(provided[p].as_bytes())))
        .chain([(
            "src/Modules/CalcPerform.lua".into(),
            hash(original.as_bytes()),
        )])
        .collect();
    Ok(
        json!({"schema_version":1,"kind":"original-minion-attack-flat-affix-queries",
        "upstream_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","scope":evidence::scope(),
        "dependency":{"path":evidence::PRIOR,"sha256":evidence::PRIOR_SHA},
        "source_context":source_context,"declarations":declarations,"cases":rows,
        "module_order":order,"files":files,
        "delivery_helper":{"path":"src/Modules/CalcPerform.lua","first_line":first,"body":body,"sha256":hash(body.as_bytes())}}),
    )
}

#[test]
#[ignore = "authors bounded original affix parser and query evidence"]
fn author_original_minion_attack_flat_affix_queries() {
    let actual = capture(sources()).unwrap();
    std::fs::write(
        root().join("runs/owned-minion-attack-flat-modifier-candidate.json"),
        serde_json::to_vec_pretty(&actual).unwrap(),
    )
    .unwrap();
    evidence::check(&actual);
    let path = root().join(evidence::PACKET);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec_pretty(&actual).unwrap()).unwrap();
}

#[test]
fn fresh_original_affix_queries_repeat_and_match_retained() {
    let actual = capture(sources()).unwrap();
    evidence::check(&actual);
    assert_eq!(
        actual,
        capture(sources()).unwrap(),
        "independent fresh original query repeat"
    );
    let retained: Json =
        serde_json::from_slice(&std::fs::read(root().join(evidence::PACKET)).unwrap()).unwrap();
    assert_eq!(actual, retained);
}

#[test]
fn changed_original_keyword_parser_and_query_behavior_cannot_reuse_proof() {
    let mut changed = sources().clone();
    let path = "src/Classes/ModDB.lua";
    let original = &changed[path];
    let needle = "MatchKeywordFlags(keywordFlags, mod.keywordFlags)";
    assert!(original.contains(needle));
    changed.insert(path.into(), original.replace(needle, "true"));
    let actual = capture(&changed).unwrap();
    assert_eq!(
        actual["cases"][0]["result"]["non_attack_direct"]["minimum"].as_f64(),
        Some(5.)
    );
    assert!(std::panic::catch_unwind(|| evidence::check(&actual)).is_err());
}
