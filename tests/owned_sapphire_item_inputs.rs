//! Complete the reviewed Sapphire pair and physical inputs; static coverage stays Partial.
#[path = "support/owned_sapphire_item_inputs.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use identity::{correspond, relocate};
use poe_optimizer_core::owned_schema::{
    DefinitionDescriptor, SchemaClosure, SchemaState, SlotDescriptor, SlotPresence,
};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry, owned_release::assemble_owned_release,
    owned_value::ValueCodecKind,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: impl AsRef<Path>, v: &impl Serialize) {
    fs::write(p, serde_json::to_vec(v).unwrap()).unwrap()
}
fn publish(input: &Path, out: &Path) -> Value {
    let r = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(out)
        .output()
        .unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    serde_json::from_slice(&r.stdout).unwrap()
}
#[test]
fn authored_sapphire_inputs_reuse_existing_types_and_keep_static_coverage_open() {
    let e = family::extension();
    let bindings = family::bindings();
    assert_eq!(e.schema.len(), 5);
    assert_eq!(bindings.len(), 1);
    assert!(e.owners.is_empty());
    assert!(e.tables.is_empty() && e.receivers.is_empty());
    assert!(e.operations_version.is_none());
    let membership = family::membership();
    assert!(
        membership
            .modifier_rules
            .iter()
            .map(|v| v.as_str())
            .eq(["ranged-plus-cold"])
    );
    assert_eq!(membership.paired_templates.len(), 1);
    assert_eq!(
        membership.paired_templates[0].template,
        bindings[0].template
    );
    assert_eq!(
        serde_json::to_value(membership.paired_templates[0].generated_members).unwrap(),
        "one_implicit_no_buff_enchant_rune_or_class_members"
    );
    for b in bindings {
        assert_eq!(
            serde_json::to_value(b.construction).unwrap(),
            "fresh_rare_saved_implicit_explicit_v2"
        );
        assert_eq!(b.header_inputs.len(), 2);
        assert_eq!(b.header_inputs[0].rule.as_str(), "rarity");
        assert_eq!(b.header_inputs[1].rule.as_str(), "level-requirement");
        let ValueCodecKind::Option { tokens } = &b.header_inputs[0].codec.codec else {
            panic!()
        };
        assert_eq!(
            tokens.iter().map(|t| t.token.as_str()).collect::<Vec<_>>(),
            ["NORMAL", "MAGIC", "RARE", "UNIQUE", "RELIC"]
        );
        let slots = [
            &b.header_inputs[0].slot,
            &b.corruption_slot,
            &b.header_inputs[1].slot,
            &b.capacity_slot,
        ];
        for (i, slot) in slots.iter().enumerate() {
            let d = e
                .schema
                .iter()
                .find_map(|d| match d {
                    SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(d)) if &d.id == *slot => {
                        Some(d)
                    }
                    _ => None,
                })
                .unwrap();
            let SchemaState::Known(s) = &d.schema else {
                panic!()
            };
            assert_eq!(
                s.presence,
                if i == 2 {
                    SlotPresence::OptionalOnce
                } else {
                    SlotPresence::RequiredOnce
                }
            );
            assert_eq!(
                serde_json::to_value(&s.sites).unwrap(),
                json!(["item_parameter"])
            );
        }
        let d = e
            .schema
            .iter()
            .find_map(|d| match d {
                SchemaExtensionEntry::Definition(DefinitionDescriptor::ItemTemplate(d))
                    if d.id == b.template =>
                {
                    Some(d)
                }
                _ => None,
            })
            .unwrap();
        let SchemaState::Known(s) = &d.schema else {
            panic!()
        };
        assert_eq!(s.declarations.parameters.members.len(), 6);
        assert!(matches!(
            s.declarations.parameters.closure,
            SchemaClosure::Partial { .. }
        ));
        assert!(matches!(
            s.quality.allowed_kinds.closure,
            SchemaClosure::Partial { .. }
        ));
        assert!(s.quality.allowed_kinds.members.is_empty());
        let default = family::defaults()
            .into_iter()
            .find(|d| d.template == b.template)
            .unwrap();
        assert_eq!(default.parameters.len(), 2);
        assert_eq!(serde_json::to_value(default.quality).unwrap(), "absent");
        for p in default.parameters {
            assert!(
                s.declarations
                    .parameters
                    .members
                    .contains(&p.assignment.slot)
            );
        }
    }
    let a = family::authoring();
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        a["source_manifest"]["sha256"]
    );
    let m: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], m["upstream_revision"]);
    assert!(!a["source_files"].as_array().unwrap().is_empty());
    for pin in a["source_files"].as_array().unwrap() {
        let rows: Vec<_> = m["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["path"] == pin["path"])
            .collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["sha256"], pin["sha256"]);
    }
    assert!(root().join(a["source_test"].as_str().unwrap()).is_file());
    cold_authoring();
}
fn source_item(sidecar: &Value, ordinal: u64) -> &Value {
    let rows: Vec<_> = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["source"]["ordinal"] == ordinal)
        .collect();
    assert_eq!(rows.len(), 1);
    let links: Vec<_> = rows[0]["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "item")
        .collect();
    assert_eq!(links.len(), 1);
    &links[0]["value"]
}
fn item<'a>(draft: &'a Value, id: &Value) -> &'a Value {
    draft["draft"]["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| &r["id"] == id)
        .unwrap()
}
fn raw_assignments(template: &Value, requirement: u64, capacity: u64) -> Vec<Value> {
    let b = family::bindings()
        .into_iter()
        .find(|b| serde_json::to_value(&b.template).unwrap() == *template)
        .unwrap();
    let ValueCodecKind::Option { tokens } = &b.header_inputs[0].codec.codec else {
        panic!()
    };
    let rare = &tokens.iter().find(|v| v.token == "RARE").unwrap().value;
    [(&b.header_inputs[0].slot,json!({"kind":"option","value":rare})),(&b.corruption_slot,json!({"kind":"boolean","value":false})),(&b.header_inputs[1].slot,json!({"kind":"integer","value":requirement})),(&b.capacity_slot,json!({"kind":"integer","value":capacity}))].into_iter().map(|(slot,value)|json!({"slot":{"kind":"known","value":slot},"value":{"kind":"known","value":value}})).collect()
}
fn catalyst_assignments(template: &Value) -> Vec<Value> {
    family::defaults().into_iter().find(|v| serde_json::to_value(&v.template).unwrap() == *template).unwrap().parameters.into_iter().map(|p| json!({"slot":{"kind":"known","value":p.assignment.slot},"value":{"kind":"known","value":p.assignment.value}})).collect()
}

fn cold_authoring() {
    let binding = family::cold::bindings();
    let source = family::cold::source_binding();
    assert_eq!(source.input, binding.input);
    let source_json = serde_json::to_value(source).unwrap();
    for (category, option) in [
        ("explicit", binding.explicit),
        ("implicit", binding.implicit),
        ("enchant", binding.enchant),
    ] {
        assert_eq!(
            source_json["values"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| v["category"] == category
                    && v["value"] == serde_json::to_value(&option).unwrap())
                .count(),
            1
        );
    }
    let roll = family::cold::roll();
    assert_eq!(roll.slot, binding.slot);
    assert_eq!(
        serde_json::to_value(roll.value).unwrap(),
        json!({"kind":"context_option","value":{"input":binding.input}})
    );
    let e = family::cold::extension();
    assert!(e.owners.is_empty() && e.tables.is_empty() && e.receivers.is_empty());
    let slot = e
        .schema
        .iter()
        .find_map(|v| match v {
            SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(s)) if s.id == binding.slot => {
                Some(s)
            }
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(s) = &slot.schema else {
        panic!()
    };
    assert_eq!(s.presence, SlotPresence::RequiredOnce);
    let d = e
        .schema
        .iter()
        .find_map(|v| match v {
            SchemaExtensionEntry::Definition(DefinitionDescriptor::Modifier(d))
                if d.id == binding.modifier =>
            {
                Some(d)
            }
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(d) = &d.schema else {
        panic!()
    };
    assert_eq!(d.declarations.parameters.members.len(), 24);
    assert!(matches!(
        d.declarations.parameters.closure,
        SchemaClosure::Partial { .. }
    ));
    let rules = family::cold::prior_rules();
    assert_eq!(rules.len(), 4);
    for r in rules {
        let [poe_optimizer_import::owned_item_lines::ItemEmission::Modifier { definition, rolls }] =
            r.emissions.as_slice()
        else {
            panic!()
        };
        assert_eq!(definition, &binding.modifier);
        assert_eq!(rolls.len(), 23);
        assert!(!rolls.iter().any(|r| r.slot == binding.slot));
    }
    assert_eq!(family::cold::source_conditions().len(), 2);
    let DefinitionDescriptor::ItemTemplate(d) = family::correction() else {
        panic!()
    };
    let SchemaState::Known(d) = d.schema else {
        panic!()
    };
    assert_eq!(d.declarations.parameters.members.len(), 2);
    assert!(matches!(
        d.declarations.parameters.closure,
        SchemaClosure::Partial { .. }
    ));
}
fn local(id: &Value) -> String {
    id["local"].as_str().unwrap().to_owned()
}
fn source_text(sidecar: &Value, ordinal: u64) -> &Value {
    sidecar["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["source"]["ordinal"] == ordinal)
        .unwrap()
}
fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    for v in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(v);
    }
    assert_eq!(sa["schema_version"], 14);
    assert_eq!(sb["schema_version"], 14);
    let raw_a = a.clone();
    let raw_b = b.clone();
    let mut changes = vec![];
    let cold = family::cold::bindings();
    let cold_definition = serde_json::to_value(&cold.modifier).unwrap();
    assert_eq!(
        sa["item_texts"].as_array().unwrap().len(),
        sb["item_texts"].as_array().unwrap().len()
    );
    for (x, y) in sa["item_texts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .zip(sb["item_texts"].as_array_mut().unwrap())
    {
        assert_eq!(x["source"], y["source"]);
        assert_eq!(x["content_entry"], y["content_entry"]);
        let ordinal = x["source"]["ordinal"].as_u64().unwrap();
        let members: Vec<Value> = raw_a["draft"]["items"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|i| i["modifiers"]["members"].as_array().unwrap())
            .filter(|m| m["definition"]["value"] == cold_definition)
            .cloned()
            .collect();
        let proven = x["attribution"]["layout"]["status"] == "proven";
        let mut touched = false;
        assert_eq!(
            x["lines"].as_array().unwrap().len(),
            y["lines"].as_array().unwrap().len()
        );
        let attribution = x["attribution"].clone();
        let new_layout = y["attribution"]["layout"].clone();
        for (p, q) in x["lines"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .zip(y["lines"].as_array_mut().unwrap())
        {
            assert_eq!(p["index"], q["index"]);
            assert_eq!(p["text"], q["text"]);
            let cold_rows: Vec<_> = p["modifiers"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|id| members.iter().any(|m| m["id"] == **id))
                .cloned()
                .collect();
            if cold_rows.is_empty() {
                continue;
            }
            assert_eq!(cold_rows.len(), 1);
            assert_eq!(p["modifiers"].as_array().unwrap().len(), 1);
            touched = true;
            let line = attribution["lines"]
                .as_array()
                .unwrap()
                .iter()
                .find(|l| l["index"] == p["index"])
                .unwrap();
            let rule = p["outcome"]["value"]["rule"].clone();
            assert_eq!(line["rule"], rule);
            let old_id = cold_rows[0].clone();
            if proven {
                assert_eq!(q["modifiers"].as_array().unwrap().len(), 1);
                let category = line["member"]["category"].as_str().unwrap();
                let option = match category {
                    "explicit" => &cold.explicit,
                    "implicit" => &cold.implicit,
                    "enchant" => &cold.enchant,
                    _ => panic!("category"),
                };
                let expected = json!({"slot":cold.slot,"value":{"kind":"option","value":option}});
                let rolls = q["outcome"]["value"]["emissions"][0]["value"]["rolls"]
                    .as_array_mut()
                    .unwrap();
                assert_eq!(rolls.len(), 24);
                let at = rolls.iter().position(|v| v == &expected).unwrap();
                rolls.remove(at);
                let previous =
                    p["outcome"]["value"]["emissions"][0]["value"]["rolls_closure"].clone();
                assert_eq!(previous["kind"], "partial");
                // ConvertedItemEmission omits only Complete roll closures.
                // The retained draft occurrence is independently checked below.
                let current = q["outcome"]["value"]["emissions"][0]["value"]
                    .as_object_mut()
                    .unwrap();
                assert!(!current.contains_key("rolls_closure"));
                current.insert("rolls_closure".to_owned(), previous);
                assert_eq!(
                    p["outcome"], q["outcome"],
                    "only exact source category added"
                );
                changes.push(json!({"source":ordinal,"line":p["index"],"old":old_id,"new":q["modifiers"][0],"category":option,"withdrawn":false}));
            } else {
                assert!(q["modifiers"].as_array().unwrap().is_empty());
                assert_eq!(
                    q["outcome"],
                    json!({"kind":"pending","value":{"reason":{"kind":"missing_context_option","value":{"input":cold.input}},"candidates":[rule]}})
                );
                assert_eq!(new_layout["status"], "pending");
                changes.push(json!({"source":ordinal,"line":p["index"],"old":old_id,"withdrawn":true,"candidate":rule,"raw":p["text"]}));
                p["modifiers"] = json!([]);
                p["outcome"] = q["outcome"].clone();
            }
        }
        if touched && x["issues"] != y["issues"] {
            assert_eq!((case, ordinal), (5, 587));
            assert_eq!(x["issues"], json!([]));
            assert_eq!(
                y["issues"],
                json!([{"problem":"schema_partial","lines":[]}])
            );
            // The explicit Complete-two-to-Partial-six template correction
            // introduces this static diagnostic; physical proof is separate.
            x["issues"] = y["issues"].clone();
        }
    }
    let withdrawn: Vec<_> = changes
        .iter()
        .filter(|v| v["withdrawn"] == true)
        .map(|v| local(&v["old"]))
        .collect();
    let complete = changes.len() - withdrawn.len();
    assert_eq!(withdrawn.len(), [3, 4, 1, 3, 4][case - 1]);
    assert_eq!(complete, [0, 2, 0, 0, 1][case - 1]);
    let mut retired = BTreeSet::new();
    for (x, y) in a["draft"]["items"]["members"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .zip(b["draft"]["items"]["members"].as_array_mut().unwrap())
    {
        for change in &changes {
            let Some(at) = x["modifiers"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .position(|m| m["id"] == change["old"])
            else {
                continue;
            };
            let before = &mut x["modifiers"]["members"][at];
            assert_eq!(
                before["rolls"]["completion"]["code"],
                "modifier-roll-schema-partial"
            );
            assert_eq!(before["rolls"]["members"].as_array().unwrap().len(), 23);
            assert!(retired.insert(local(&before["rolls"]["completion"]["id"])));
            if change["withdrawn"] == true {
                x["modifiers"]["members"].as_array_mut().unwrap().remove(at);
            } else {
                let after = y["modifiers"]["members"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|m| m["id"] == change["new"])
                    .unwrap();
                assert_eq!(after["rolls"]["completion"], json!({"kind":"complete"}));
                let expected = json!({"slot":{"kind":"known","value":cold.slot},"value":{"kind":"known","value":{"kind":"option","value":change["category"]}}});
                let rolls = after["rolls"]["members"].as_array_mut().unwrap();
                assert_eq!(rolls.len(), 24);
                let at = rolls.iter().position(|v| v == &expected).unwrap();
                rolls.remove(at);
                before["rolls"]
                    .as_object_mut()
                    .unwrap()
                    .remove("completion");
                after["rolls"].as_object_mut().unwrap().remove("completion");
            }
        }
    }
    let ring_old = if case == 5 {
        Some(source_item(&sa, 587).clone())
    } else {
        None
    };
    let ring_new = if case == 5 {
        Some(source_item(&sb, 587).clone())
    } else {
        None
    };
    if let (Some(old_id), Some(new_id)) = (&ring_old, &ring_new) {
        let original = item(&raw_a, old_id);
        let updated = item(&raw_b, new_id);
        assert_eq!(updated["modifiers"]["members"].as_array().unwrap().len(), 2);
        for m in updated["modifiers"]["members"].as_array().unwrap() {
            assert_eq!(m["rolls"]["completion"], json!({"kind":"complete"}));
            assert_eq!(m["rolls"]["members"].as_array().unwrap().len(), 24);
        }
        assert_eq!(
            updated["modifier_order"],
            json!({"kind":"known","value":[updated["modifiers"]["members"][0]["id"],updated["modifiers"]["members"][1]["id"]]})
        );
        assert_eq!(original["quality"], updated["quality"]);
        assert_eq!(original["item_level"], updated["item_level"]);
        let before = a["draft"]["items"]["members"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|v| &v["id"] == old_id)
            .unwrap();
        let after = b["draft"]["items"]["members"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|v| &v["id"] == new_id)
            .unwrap();
        for (field, code) in [
            ("parameters", "item-parameters-not-converted"),
            ("modifiers", "item-modifiers-not-converted"),
        ] {
            assert_eq!(before[field]["completion"]["code"], code);
            assert!(retired.insert(local(&before[field]["completion"]["id"])));
            assert_eq!(after[field]["completion"], json!({"kind":"complete"}));
            before[field].as_object_mut().unwrap().remove("completion");
            after[field].as_object_mut().unwrap().remove("completion");
        }
        assert_eq!(
            before["modifier_order"]["code"],
            "item-modifier-order-not-converted"
        );
        assert!(retired.insert(local(&before["modifier_order"]["id"])));
        before.as_object_mut().unwrap().remove("modifier_order");
        after.as_object_mut().unwrap().remove("modifier_order");
        assert_eq!(after["parameters"]["members"].as_array().unwrap().len(), 6);
        for expected in raw_assignments(&after["template"]["value"], 12, 0) {
            let rows = after["parameters"]["members"].as_array_mut().unwrap();
            let at = rows.iter().position(|v| v == &expected).unwrap();
            rows.remove(at);
        }
        assert_eq!(
            before["parameters"]["members"],
            after["parameters"]["members"]
        );
        assert_eq!(
            before["parameters"]["members"],
            json!(catalyst_assignments(&before["template"]["value"]))
        );
        let ring_uses: Vec<_> = raw_b["draft"]["equipment"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|u| u["item"]["value"] == *new_id)
            .collect();
        assert_eq!(ring_uses.len(), 8);
        assert_eq!(
            ring_uses
                .iter()
                .map(|u| local(&u["id"]))
                .collect::<BTreeSet<_>>()
                .len(),
            8
        );
        let selection = selected::selection(xml, new);
        let preset = raw_b["draft"]["equipment_presets"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| local(&v["id"]) == local(&selection["build"]["equipment"]))
            .unwrap();
        assert_eq!(
            ring_uses
                .iter()
                .filter(|u| preset["equipment"]["members"]
                    .as_array()
                    .unwrap()
                    .contains(&u["id"]))
                .count(),
            2
        );
    }
    let pa = a["draft"]["allocator"].clone();
    let na = b["draft"]["allocator"].clone();
    let issued = |v: &Value| u64::from_str_radix(v["last_issued"].as_str().unwrap(), 16).unwrap();
    assert_eq!(
        issued(&pa) - issued(&na),
        2 * withdrawn.len() as u64 + complete as u64 + if case == 5 { 3 } else { 0 }
    );
    b["draft"]["allocator"] = pa.clone();
    let mut ids = BTreeMap::new();
    correspond(
        &a,
        &mut b,
        &mut ids,
        "exact inputs except declared cold/ring changes",
    );
    for row in sa["origins"].as_array_mut().unwrap() {
        row["links"].as_array_mut().unwrap().retain(|l| {
            !(l["kind"] == "issue" && retired.contains(&local(&l["value"])))
                && !(l["kind"] == "modifier" && withdrawn.contains(&local(&l["value"])))
        });
    }
    let policy = sb["item_source_policy"].clone();
    let lines = sb["item_policy"].clone();
    for (x, y) in sa["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .zip(sb["item_texts"].as_array_mut().unwrap())
    {
        assert_eq!(y["attribution"]["policy"], policy);
        assert_eq!(y["attribution"]["item_lines"], lines);
        y["attribution"]["policy"] = x["attribution"]["policy"].clone();
        y["attribution"]["item_lines"] = x["attribution"]["item_lines"].clone();
        if case == 5 && x["source"]["ordinal"] == 587 {
            assert!(x.get("parameter_inputs").is_none());
            let proof = y
                .as_object_mut()
                .unwrap()
                .remove("parameter_inputs")
                .unwrap();
            assert_eq!(proof.as_array().unwrap().len(), 4);
            let binding = &family::bindings()[0];
            let template = serde_json::to_value(&binding.template).unwrap();
            for expected in raw_assignments(&template, 12, 0) {
                let matches: Vec<_> = proof
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|r| {
                        r["slot"] == expected["slot"]["value"]
                            && r["value"] == expected["value"]["value"]
                    })
                    .collect();
                assert_eq!(matches.len(), 1);
                let row = matches[0];
                if row["slot"] == serde_json::to_value(&binding.corruption_slot).unwrap() {
                    assert_eq!(row["origin"], json!({"kind":"fresh_uncorrupted"}));
                } else if row["slot"] == serde_json::to_value(&binding.capacity_slot).unwrap() {
                    assert_eq!(row["origin"], json!({"kind":"absent_socket_header"}));
                } else {
                    let h = binding
                        .header_inputs
                        .iter()
                        .find(|h| serde_json::to_value(&h.slot).unwrap() == row["slot"])
                        .unwrap();
                    assert_eq!(row["origin"]["kind"], "header");
                    assert_eq!(row["origin"]["rule"], h.rule.as_str());
                    assert_eq!(row["origin"]["capture"], h.capture.as_str());
                    let line = x["lines"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|l| l["index"] == row["origin"]["line"])
                        .unwrap();
                    assert_eq!(
                        line["text"],
                        if h.rule.as_str() == "rarity" {
                            "Rarity: RARE"
                        } else {
                            "LevelReq: 12"
                        }
                    );
                }
            }
        }
    }
    for field in [
        "draft",
        "definitions",
        "registry",
        "mapping",
        "policy",
        "item_policy",
        "item_source_policy",
        "skill_roles",
        "reward_policy",
        "tree_policy",
    ] {
        sb[field] = sa[field].clone();
    }
    assert_eq!(sb["allocator_after"], na);
    assert_eq!(sa["allocator_after"], pa);
    sb["allocator_after"] = pa;
    correspond(
        &sa,
        &mut sb,
        &mut ids,
        "all retained source text/candidates/evidence/links",
    );
    let before = selected::finalize(
        xml,
        old,
        &out.join(format!("original-{case:02}-prior-selection.json")),
    );
    let after = selected::finalize(
        xml,
        new,
        &out.join(format!("original-{case:02}-selection.json")),
    );
    write(
        out.join(format!("original-{case:02}-prior-selected-report.json")),
        &before,
    );
    write(
        out.join(format!("original-{case:02}-selected-report.json")),
        &after,
    );
    let mut old_issues = before["finalization"]["issues"].clone();
    let mut new_issues = after["finalization"]["issues"].clone();
    selected::canonical(&mut old_issues);
    selected::canonical(&mut new_issues);
    relocate(&mut new_issues, &ids);
    let count = old_issues.as_array().unwrap().len();
    old_issues
        .as_array_mut()
        .unwrap()
        .retain(|v| !retired.contains(&local(&v["id"])));
    let mut paths = BTreeMap::new();
    for (i, item) in raw_b["draft"]["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        for (j, m) in item["modifiers"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            let mut id = m["id"].clone();
            relocate(&mut id, &ids);
            let old = raw_a["draft"]["items"]["members"][i]["modifiers"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .position(|v| v["id"] == id)
                .unwrap();
            paths.insert(
                format!("items.members[{i}].modifiers.members[{old}]."),
                format!("items.members[{i}].modifiers.members[{j}]."),
            );
        }
    }
    for previous in old_issues.as_array().unwrap() {
        let current = new_issues
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|v| v["id"] == previous["id"])
            .unwrap();
        if current["path"] != previous["path"] {
            let p = previous["path"].as_str().unwrap();
            let (from, to) = paths
                .iter()
                .find(|(k, _)| p.starts_with(k.as_str()))
                .unwrap();
            assert_eq!(current["path"], format!("{}{}", to, &p[from.len()..]));
            current["path"] = previous["path"].clone();
        }
    }
    assert_eq!(
        old_issues, new_issues,
        "every other selected obligation remains"
    );
    let removed = count - new_issues.as_array().unwrap().len();
    assert_eq!(removed, [3, 1, 1, 3, 4][case - 1]);
    let mut first = selected::selection(xml, old);
    let mut second = selected::selection(xml, new);
    selected::canonical(&mut first);
    selected::canonical(&mut second);
    relocate(&mut second, &ids);
    assert_eq!(first, second);
    json!({"original":case,"selected_before":count,"selected_after":new_issues.as_array().unwrap().len(),"selected_withdrawals":([3,1,1,3,0][case-1]),"selected_positive_retirements":if case==5{4}else{0},"cold_completed":complete,"cold_withdrawn":withdrawn.len(),"cold_changes":changes,"retired":retired,"ring_raw_assignments":if case==5{4}else{0},"ring_uses":if case==5{8}else{0},"selected_ring_uses":if case==5{2}else{0},"allocator_after":na,"selected_issue_summary":after["selected_issue_summary"]})
}

fn probes(package: &Path, out: &Path) -> usize {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let start = original.find("<Item id=\"26\">").unwrap();
    let end = start + original[start..].find("</Item>").unwrap() + 7;
    let raw = &original[start..end];
    let run = |label: &str, changed: String| {
        assert_ne!(changed, raw, "{label}");
        let path = out.join(format!("probe-{label}.xml"));
        fs::write(
            &path,
            format!("{}{}{}", &original[..start], changed, &original[end..]),
        )
        .unwrap();
        let dest = out.join(format!("probe-{label}"));
        release::normalize(package, &path, 5, &dest);
        let draft: Value = read(dest.join("draft.json"));
        let sidecar: Value = read(dest.join("sidecar.json"));
        let physical = item(&draft, source_item(&sidecar, 587)).clone();
        let text = source_text(&sidecar, 587).clone();
        (physical, text)
    };
    // These inputs are outside the sealed two-member/fresh-construction scope.
    // Independent item level, known modifier values and quality remain untouched.
    let cases = [
        ("missing-level", raw.replace("LevelReq: 12\n", "")),
        (
            "duplicate-level",
            raw.replace("LevelReq: 12", "LevelReq: 12\nLevelReq: 77"),
        ),
        (
            "alternate-level",
            raw.replace("LevelReq: 12", "Requires Level: 12"),
        ),
        (
            "malformed-level",
            raw.replace("LevelReq: 12", "LevelReq: NaN"),
        ),
        (
            "corrupted",
            raw.replace("Implicits: 1", "Corrupted\nImplicits: 1"),
        ),
        (
            "duplicate-rarity",
            raw.replace("Rarity: RARE", "Rarity: RARE\nRarity: RARE"),
        ),
        (
            "unknown-member",
            raw.replace("Implicits: 1", "Implicits: 1\nUnknown modifier"),
        ),
        (
            "extra-member",
            raw.replace(
                "+10 to maximum Life",
                "+10 to maximum Life\n+3 to maximum Life",
            ),
        ),
        ("all-explicit", raw.replace("Implicits: 1", "Implicits: 0")),
        ("all-implicit", raw.replace("Implicits: 1", "Implicits: 2")),
        (
            "occupied-rune",
            raw.replace(
                "Implicits: 1",
                "Sockets: S\nRune: Lesser Iron Rune\nImplicits: 1",
            ),
        ),
        (
            "explicit-catalyst",
            raw.replace("Implicits: 1", "Catalyst: Neural\nImplicits: 1"),
        ),
        (
            "explicit-catalyst-amount",
            raw.replace("Implicits: 1", "CatalystQuality: 0\nImplicits: 1"),
        ),
        (
            "raw-spirit-header",
            raw.replace("Implicits: 1", "Spirit: 77\nImplicits: 1"),
        ),
        (
            "zero-range",
            raw.replace("+(20-30)% to Cold Resistance", "+(0-0)% to Cold Resistance"),
        ),
    ];
    for (label, changed) in &cases {
        let (physical, text) = run(label, changed.clone());
        assert_eq!(
            physical["parameters"]["completion"]["kind"], "pending",
            "{label}"
        );
        assert!(text.get("parameter_inputs").is_none(), "{label}");
    }
    let quality = [
        ("quality-zero", "Quality: 0"),
        ("quality-twenty", "Quality: 20"),
        ("quality-malformed", "Quality: NaN"),
        ("quality-alias", "Quality (Mana Modifiers): +20%"),
        ("quality-catalyst-style", "Quality: +20% (Mana Modifiers)"),
        ("quality-duplicate", "Quality: 0\nQuality: 20"),
    ];
    for (label, header) in quality {
        let (physical, text) = run(
            label,
            raw.replace("Implicits: 1", &format!("{header}\nImplicits: 1")),
        );
        assert_eq!(text["defaults"]["quality_absent"], false, "{label}");
        assert_ne!(
            physical["quality"],
            json!({"kind":"known","value":null}),
            "{label}"
        );
        // The existing Partial quality-kind declaration can leave an explicit
        // quality unresolved. This proof never changes that schema or defaults it.
    }
    let (physical, _) = run(
        "explicit-level-77",
        raw.replace("LevelReq: 12", "LevelReq: 77"),
    );
    assert_eq!(
        physical["parameters"]["completion"],
        json!({"kind":"complete"})
    );
    for expected in raw_assignments(&physical["template"]["value"], 77, 0) {
        assert!(
            physical["parameters"]["members"]
                .as_array()
                .unwrap()
                .contains(&expected)
        );
    }
    let (physical, text) = run(
        "explicit-empty-socket",
        raw.replace("Implicits: 1", "Sockets: S\nRune: None\nImplicits: 1"),
    );
    assert_eq!(
        physical["parameters"]["completion"],
        json!({"kind":"complete"})
    );
    for expected in raw_assignments(&physical["template"]["value"], 12, 1) {
        assert!(
            physical["parameters"]["members"]
                .as_array()
                .unwrap()
                .contains(&expected)
        );
    }
    let capacity = serde_json::to_value(&family::bindings()[0].capacity_slot).unwrap();
    let evidence = text["parameter_inputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["slot"] == capacity)
        .unwrap();
    assert_eq!(evidence["origin"]["kind"], "empty_socket_capacity");
    let line = text["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["index"] == evidence["origin"]["line"])
        .unwrap();
    assert_eq!(line["text"], "Sockets: S");
    let cold = family::cold::bindings();
    let definition = serde_json::to_value(&cold.modifier).unwrap();
    let original_line = raw
        .lines()
        .find(|v| v.contains("+(20-30)% to Cold Resistance"))
        .unwrap();
    let refused = [
        ("cold-fractured", format!("{{fractured}}{original_line}")),
        ("cold-desecrated", format!("{{desecrated}}{original_line}")),
        (
            "cold-combined-flags",
            format!("{{fractured}}{{desecrated}}{original_line}"),
        ),
        (
            "cold-unknown-tag",
            "{tags:unknown_property}{range:0.5}+(20-30)% to Cold Resistance".to_owned(),
        ),
        ("cold-variant", format!("{{variant:1}}{original_line}")),
        ("cold-rune", format!("{{rune}}{original_line}")),
        ("cold-fixed-minus", "-25% to Cold Resistance".to_owned()),
        (
            "cold-bare-range",
            "{range:0.5}(20-30)% to Cold Resistance".to_owned(),
        ),
        (
            "cold-zero-range",
            "{range:0.5}+(0-0)% to Cold Resistance".to_owned(),
        ),
    ];
    for (label, line) in &refused {
        let (physical, text) = run(label, raw.replace(original_line, line));
        assert!(
            !physical["modifiers"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["definition"]["value"] == definition),
            "{label}"
        );
        assert!(
            text["lines"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["text"] == *line),
            "{label}"
        );
        assert_eq!(
            physical["modifiers"]["completion"]["kind"], "pending",
            "{label}"
        );
    }
    // Fixed plus zero is independently admitted by its existing recipe. It is
    // not in the paired-inventory allowlist and cannot close the physical item.
    let (physical, _) = run(
        "cold-fixed-plus-zero",
        raw.replace(original_line, "+0% to Cold Resistance"),
    );
    let cold_rows: Vec<_> = physical["modifiers"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["definition"]["value"] == definition)
        .collect();
    assert_eq!(cold_rows.len(), 1);
    assert_eq!(
        cold_rows[0]["rolls"]["completion"],
        json!({"kind":"complete"})
    );
    assert_eq!(
        cold_rows[0]["rolls"]["members"].as_array().unwrap().len(),
        24
    );
    let category = json!({"slot":{"kind":"known","value":cold.slot},"value":{"kind":"known","value":{"kind":"option","value":cold.implicit}}});
    assert!(
        cold_rows[0]["rolls"]["members"]
            .as_array()
            .unwrap()
            .contains(&category)
    );
    let amount = cold_rows[0]["rolls"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["slot"]["value"]["slot"]["key"] == "def.0000000000002543")
        .unwrap();
    assert_eq!(amount["value"]["value"]["value"]["value"], 0.0);
    assert_eq!(physical["modifiers"]["completion"]["kind"], "pending");
    // Transport the unrounded range interpolation. Native numeric programs own
    // the source-backed rounding of 25.5 to 26.
    let (physical, _) = run(
        "cold-fractional-range",
        raw.replace(
            "+(20-30)% to Cold Resistance",
            "+(21-30)% to Cold Resistance",
        ),
    );
    let row = physical["modifiers"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["definition"]["value"] == definition)
        .unwrap();
    let amount = row["rolls"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["slot"]["value"]["slot"]["key"] == "def.0000000000002543")
        .unwrap();
    assert_eq!(amount["value"]["value"]["value"]["value"], 25.5);
    cases.len() + quality.len() + 2 + refused.len() + 2
}
#[test]
#[ignore = "requires the exact checked Solar predecessor"]
fn real_sapphire_item_inputs_preserve_all_requests_and_independent_coverage() {
    let p = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SAPPHIRE_INPUTS_PRIOR").expect("explicit prior"),
    );
    let before = release::inventory(&p);
    let prior = release::load(&p);
    let next = family::stage(&prior);
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_SAPPHIRE_INPUTS_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("publication"));
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    for (src, dst) in [(&out.join("endpoint.json"), &package), (&package, &rebuilt)] {
        assert_eq!(
            publish(src, dst),
            serde_json::to_value(next.receipt()).unwrap()
        );
    }
    let inv = release::inventory(&package);
    assert_eq!(inv, release::inventory(&rebuilt));
    for (name, hash) in &before {
        if name.starts_with("queries-") {
            assert_eq!(inv.get(name), Some(hash));
        }
    }
    for policy in ["item_modifier_membership", "item_parameter_inputs"] {
        for field in ["definitions", "item_lines", "item_source"] {
            let mut bad = serde_json::to_value(next.input()).unwrap();
            let old = match field {
                "definitions" => serde_json::to_value(&prior.receipt().definitions).unwrap(),
                "item_lines" => serde_json::to_value(prior.receipt().items).unwrap(),
                _ => serde_json::to_value(prior.receipt().item_source).unwrap(),
            };
            bad["normalization"][policy][field] = old;
            assert!(
                assemble_owned_release(serde_json::from_value(bad).unwrap(), Default::default())
                    .is_err(),
                "stale {policy}.{field}"
            );
        }
    }

    let mut reports = vec![];
    for case in 1..=5 {
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let a = out.join(format!("prior-original-{case:02}"));
        let b = out.join(format!("original-{case:02}"));
        release::normalize(&p, &xml, case, &a);
        release::normalize(&package, &xml, case, &b);
        reports.push(compare(case, &fs::read(xml).unwrap(), &a, &b, &out));
    }
    let count = probes(&package, &out);
    assert_eq!(before, release::inventory(&p));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"definitions":next.receipt().definitions,"registry":next.receipt().registry,"prior_provenance":prior.input().provenance.len(),"final_provenance":next.input().provenance.len(),"queries":next.receipt().query_rows,"originals":reports,"probes":count,"stale_binding_rejections":6,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
