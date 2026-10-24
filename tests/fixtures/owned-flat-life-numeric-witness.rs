//! Full-source numeric boundary audit against the actual released Life program.
//! This does not certify source-text admission, factor production or full builds.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/player_resource_source.rs"]
mod resource;
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::FiniteQuantity, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact};
use poe_optimizer_pob::source as pinned;
use resource::{hash, observe};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

const OBSERVER: &str = r##"
local function original(f,suffix,line)
 local d=debug.getinfo(f,"S")
 assert(d.what=="Lua" and d.source:gsub("\\","/"):sub(-#suffix)==suffix and d.linedefined==line)
 return f
end
local format=original(itemLib.formatValue,"Modules/ItemTools.lua",45)
local symmetric=original(roundSymmetric,"Modules/Common.lua",744)
local before=build.calcsTab.mainOutput.Life
assert(debug.gethook()==nil)
local metadata=data.modScalability["# to maximum Life"]
assert(#metadata==1 and metadata[1].isScalable and metadata[1].formats==nil)
local cases={
 {"zero",0,1,1},{"identity",17,1,1},{"raw-upper",1000000,1,1},
 {"fraction",17.5,1,1},{"below-half",0.49999999999999994,1,1},
 {"half",0.5,1,1},{"above-half",0.5000000000000001,1,1},
 {"corruption",10,1.25,1},{"corruption-zero",10,0,1},
 {"fraction-and-corruption",25.5,1.5,1},
 {"ordered-factors",25.5,1.5,1.2},{"negative-magnitude",25.5,1.5,-1.2},
 {"magnitude-zero",17,1,0},{"small-factor",1,0.0000001,1},
 {"corruption-upper",999999,1000000,1},
 {"both-upper",1000000,1000000,1},
 {"large-magnitude",999999,999999,1001}
}
local results={}
for _,c in ipairs(cases) do
 local text=format(c[2],c[3],c[4],1,nil,nil)
 results[#results+1]={name=c[1],raw=c[2],corruption=c[3],magnitude=c[4],text=text,value=assert(tonumber(text))}
end
local exact=1000997998001001
local transport={before=exact,rounded=symmetric(exact,0),short=tostring(exact),full=string.format("%.17g",exact)}
assert(itemLib.formatValue==format and roundSymmetric==symmetric and debug.gethook()==nil)
assert(build.calcsTab.mainOutput.Life==before)
return {cases=results,transport=transport,original_output_preserved=true,original_function_preserved=true}
"##;

fn child(root: &Path, out: &Path, jit: bool) {
    let release =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_LIFE_NUMERIC_RELEASE").unwrap());
    let release = if release.is_absolute() {
        release
    } else {
        root.join(release)
    };
    let read = |name: &str| fs::read(release.join(name)).unwrap();
    let receipt: Value = serde_json::from_slice(&read("release.json")).unwrap();
    assert_eq!(
        receipt["input"],
        "26c0a5022e550ae9f2d2488c27b761a58917f2153aeda4b402f465a7d7c0bf76"
    );
    let schema = OwnedDefinitionSchemaPackage::new(
        serde_json::from_slice(&read("schema.json")).unwrap(),
        Default::default(),
    )
    .unwrap();
    let rules: RulePackageInput = serde_json::from_slice(&read("rules.json")).unwrap();
    let compiled = CompiledRulePackage::compile(&rules, &schema, Default::default()).unwrap();
    assert_eq!(json!(schema.identity()), receipt["definitions"]);
    assert_eq!(
        compiled.identity().to_string(),
        receipt["compiled_rules"].as_str().unwrap()
    );
    let owner = rules
        .owners
        .iter()
        .find(|o| json!(o.owner)["value"]["value"]["key"] == "def.0000000000003100")
        .unwrap();
    assert!(!owner.programs.is_complete());
    let program = owner
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == "effective-amount")
        .unwrap();
    assert_eq!(program.reads.len(), 3);
    let program_sha256 = hash(&serde_json::to_vec(program).unwrap());
    let mut scratch = compiled.new_scratch();
    let originals = resource::originals(root);
    let mut rows = vec![];
    let mut discrepancies = vec![];
    for (i, xml) in originals
        .iter()
        .chain(std::iter::once(&originals[4]))
        .enumerate()
    {
        let observed = observe(root, xml, None, jit, OBSERVER);
        assert_eq!(observed["state"]["original_output_preserved"], true);
        assert_eq!(observed["state"]["original_function_preserved"], true);
        assert_eq!(
            observed["state"]["transport"],
            json!({"before":1_000_997_998_001_001_u64,"rounded":1_000_997_998_001_001_u64,"short":"1.000997998001e+15","full":"1000997998001001"})
        );
        if let Some(first) = rows.first() {
            let first: &Value = first;
            assert_eq!(observed["state"], first["observed"]["state"]);
        }
        for c in observed["state"]["cases"].as_array().unwrap() {
            let facts: Vec<_> = program
                .reads
                .iter()
                .map(|r| {
                    let field = match r.id.as_str() {
                        "component" => "raw",
                        "corruption-factor" => "corruption",
                        "magnitude-factor" => "magnitude",
                        _ => panic!(),
                    };
                    let ComputedValueType::Quantity { unit } = &r.value_type else {
                        panic!()
                    };
                    RuleFact {
                        read: r.id.clone(),
                        value: ParameterValue::Quantity(
                            FiniteQuantity::new(c[field].as_f64().unwrap(), unit.clone()).unwrap(),
                        ),
                    }
                })
                .collect();
            let result = compiled
                .evaluate(&owner.owner, &program.id, &facts, &schema, &mut scratch)
                .unwrap();
            assert_eq!(result.owner_programs_closure, owner.programs.closure);
            let EffectDisposition::Applied {
                value: ParameterValue::Quantity(value),
            } = &result.effects[0].disposition
            else {
                panic!("{result:?}")
            };
            if value.value() != c["value"].as_f64().unwrap() {
                assert_eq!(c["name"], "large-magnitude");
                assert_eq!(value.value(), 1_000_997_998_001_001.0);
                assert_eq!(c["value"], 1_000_997_998_001_000_u64);
                discrepancies.push(json!({"observation":i,"case":c,"native":value.value()}));
            }
        }
        rows.push(json!({"index":i,"xml_sha256":hash(xml.as_bytes()),"observed":observed}));
    }
    let warm = observe(root, &originals[4], Some(&originals[1]), jit, OBSERVER);
    assert_eq!(warm, rows[4]["observed"]);
    let report = json!({"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":hash(OBSERVER.as_bytes()),"witness_sha256":hash(include_bytes!("owned_flat_life_numeric.rs")),
        "driver_sha256":hash(include_bytes!("support/player_resource_source.rs")),
        "files":(["src/Modules/ItemTools.lua","src/Modules/Common.lua","src/Data/ModScalability.lua"].map(|p|json!({"path":p,"sha256":pinned::expected_file_sha256(p).unwrap()}))),
        "release_input":receipt["input"],"program_sha256":program_sha256,"cases":rows,"warm_restoration":warm,"complete_loads":8,"discrepancies":discrepancies,"whole_build_parity":false,"numeric_domain_closed":false});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if jit { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    assert_eq!(
        discrepancies.len(),
        6,
        "retain the exact observed transport discrepancy in every fresh context"
    );
}

#[test]
#[ignore = "requires current Life package, full source and fresh numeric output"]
fn released_life_numeric_audit_retains_source_decimal_transport_discrepancy() {
    resource::supervise(
        "released_life_numeric_audit_retains_source_decimal_transport_discrepancy",
        "POE_LIFE_NUMERIC_CHILD",
        "POE_OPTIMIZER_TEST_LIFE_NUMERIC_SOURCE_OUT",
        child,
    );
}
