#!/usr/bin/env python3
"""Author a bounded resistance-contribution successor from reviewed data.

This offline exporter neither executes source nor computes a build/metric. It
preserves the prior recipe, adds explicit input/rule declarations, and emits a
lexically constrained fixed/ranged syntax policy with explicit signed rounding.
"""
import argparse
import copy
import importlib.util
import json
from pathlib import Path
import re
import sys

SPEC = importlib.util.spec_from_file_location("owned_import_export", Path(__file__).with_name("export-owned-import-data.py"))
DATA = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = DATA
SPEC.loader.exec_module(DATA)
tag, subject, pretty, compact, sha = DATA.tag, DATA.subject, DATA.pretty, DATA.compact, DATA.sha
MAX_BYTES = 8 * 1024 * 1024


def load(path):
    return DATA.decode(DATA.read(path, MAX_BYTES))


def complete(members=()):
    return {"members": list(members), "closure": tag("complete")}


def partial(members, owner, code, facet="game_rules"):
    return {"members": list(members), "closure": tag("partial", {"gaps": [{"subject": subject(owner), "facet": facet, "code": code}]})}


def declarations():
    return {name: complete() for name in ["parameters", "choices", "grants", "actors", "skill_grants", "outputs", "sockets"]}


def produce(base_dir, import_inputs, authoring_path, source_root):
    raw = DATA.read(base_dir / "recipe.json", MAX_BYTES)
    base, authoring = DATA.decode(raw), load(authoring_path)
    if sha(raw) != authoring["base_recipe_sha256"] or base["registry"]["last_issued"] != authoring["base_registry_watermark"]:
        raise ValueError("base recipe binding differs")
    if authoring["schema_version"] != 1 or authoring["scope"] != "resistance-contribution-component":
        raise ValueError("unsupported resistance authoring version/scope")
    if len(authoring["source_spans"]) > 32 or len(authoring["reward_contributions"]) > 32 or len(authoring["ring_equipment_slots"]) > 32:
        raise ValueError("authoring collection bound exceeded")
    mapping = load(base_dir / "mapping.json")
    if mapping["definitions"] != base["rules"]["definitions"] or mapping["registry"] != DATA.owned_digest("owned-id-registry-v1", base["registry"]):
        raise ValueError("base mapping binding differs")
    if DATA.schema_identity(base["schema"]) != base["rules"]["definitions"] or base["routing"]["definitions"] != base["rules"]["definitions"]:
        raise ValueError("base schema binding differs")
    source = copy.deepcopy(mapping["source"])
    pins = {f["path"]: f["sha256"] for f in source["files"]}
    spans, source_bytes = [], 0
    root = source_root.resolve()
    for span in authoring["source_spans"]:
        path = (root / span["path"]).resolve()
        if not span["path"].startswith("src/") or not path.is_relative_to(root):
            raise ValueError("source path escapes exact src namespace")
        raw_source = DATA.read(path, MAX_BYTES).replace(b"\r\n", b"\n")
        source_bytes += len(raw_source)
        if source_bytes > 32 * 1024 * 1024 or sha(raw_source) != span["file_sha256"]:
            raise ValueError("source byte/hash bound differs")
        if span["path"] in pins and pins[span["path"]] != sha(raw_source):
            raise ValueError("conflicting source pin")
        pins[span["path"]] = sha(raw_source)
        lines = raw_source.decode().splitlines(keepends=True)
        if not 1 <= span["line"] <= span["end_line"] <= len(lines):
            raise ValueError("source span outside file")
        selected = "".join(lines[span["line"] - 1:span["end_line"]])
        if sha(selected.encode()) != span["sha256"]:
            raise ValueError("source span hash differs")
        spans.append({**span, "text": selected})
    source["files"] = [{"path": name, "sha256": digest} for name, digest in sorted(pins.items())]
    recipe = copy.deepcopy(base)
    registry, schema, rules = recipe["registry"], recipe["schema"], recipe["rules"]
    DATA.validate_rule_wire(rules)
    namespace, ids = registry["namespace"], {}
    old_entries = list(registry["entries"])
    unit = authoring["percentage_points"]
    unit_schema = next((d for d in schema["definitions"] if d["value"]["id"] == unit), None)
    if unit_schema is None or unit_schema["value"]["schema"] != tag("known", {"dimension": "percentage_points"}):
        raise ValueError("exact percentage-points unit is not known")
    def allocate(label, kind, payload, owner=None):
        registry["last_issued"] += 1
        registry["revision"] += 1
        identifier = {"kind": kind, "namespace": namespace, "key": f'def.{registry["last_issued"]:016x}'}
        if owner is not None:
            identifier = {"declaration": {"kind": owner["kind"], "definition": owner}, "slot": identifier}
            address = tag("slot", tag("parameter", identifier))
            schema["slots"].append(tag("parameter", {"id": identifier, "schema": tag("known", payload)}))
        else:
            address = subject(identifier)
            schema["definitions"].append(tag(kind, {"id": identifier, "schema": tag("known", payload)}))
        registry["entries"].append({"sequence": registry["last_issued"], "target": address, "state": tag("active")})
        ids[label] = identifier
        return identifier
    def quantity(value):
        return tag("quantity", {"value": float(value), "unit": unit})
    def quantity_type():
        return tag("quantity", {"unit": unit})
    def value_schema():
        bounds = authoring["computation_envelope"]
        if type(bounds["minimum"]) is not int or type(bounds["maximum"]) is not int or not -1000000 <= bounds["minimum"] <= 0 < bounds["maximum"] <= 1000000:
            raise ValueError("invalid explicit computation envelope")
        return tag("quantity", {"minimum": quantity(bounds["minimum"])["value"], "maximum": quantity(bounds["maximum"])["value"]})
    stats = {name: allocate(name + "-resistance-base-contributions", "stat", {"value": quantity_type(), "targets": ["actor"]}) for name in ["cold", "elemental"]}
    modifiers = {}
    for name in ["cold", "elemental", "life"]:
        ports = declarations()
        definition = allocate("flat-" + name + "-modifier", "modifier", {"declarations": ports})
        value = tag("integer", {"minimum": 0, "maximum": 1000000}) if name == "life" else value_schema()
        slot = allocate("flat-" + name + "-roll", "parameter_slot", {"value": value, "presence": "required_once", "sites": ["modifier_roll"]}, definition)
        ports["parameters"]["members"].append(slot)
        modifiers[name] = (definition, slot)
        programs = []
        if name != "life":
            programs.append({"id": "resistance-contribution", "context": "equipment_use",
                "reads": [{"id": "amount", "value_type": quantity_type(), "source": tag("parameter", {"slot": slot})}],
                "nodes": [{"id": "amount", "expression": {"kind": "read", "input": "amount"}}],
                "effects": [{"id": "add-resistance", "when": None, "effect": {"kind": "contribute", "entity": "player", "stat": stats[name], "contribution": "add", "value": "amount"}}]})
        rules["owners"].append({"owner": subject(definition), "programs": partial([], definition, "life-effect-not-converted") if name == "life" else complete(programs)})
    template = allocate("sapphire-ring-template", "item_template", {})
    payload = next(d["value"]["schema"]["value"] for d in schema["definitions"] if d["value"]["id"] == template)
    payload.update({"item_level": {"minimum": 1, "maximum": 100},
        "equipment_slots": partial(authoring["ring_equipment_slots"], template, "other-equipment-contexts-unreviewed", "input_schema"),
        "socket_destinations": partial([], template, "socket-inputs-unconverted", "input_schema"),
        "modifiers": partial([d for d, _ in modifiers.values()], template, "other-item-modifiers-unconverted", "input_schema"),
        "quality": {"presence": "optional", "allowed_kinds": partial([], template, "item-quality-unconverted", "input_schema")}, "declarations": declarations()})
    rules["owners"].append({"owner": subject(template), "programs": partial([], template, "remaining-item-template-effects-unconverted")})
    reward_facts = load(import_inputs / "reward-source-facts.json")
    reward_rows = {r["config_key"]: r for r in reward_facts["rows"]}
    mapped = {compact(r["source"]): r for r in mapping["entries"]}
    for row in authoring["reward_contributions"]:
        entry = mapped.get(compact(row["source"]))
        if entry is None or entry["outcome"]["value"]["target"] != subject(row["target"]):
            raise ValueError("reward source/owned target binding differs")
        choice = reward_rows[row["source"]["value"]["key"]["value"]]["choice"]
        token = row["source"]["value"]["value"]["value"]
        expected_text = choice["stat_text"] if choice["kind"] == "boolean" and token == "true" else token if choice["kind"] == "option" and token in choice["options"] else None
        if expected_text != row["text"]:
            raise ValueError("reward effect text differs from reviewed outcome")
        values = []
        for line in row["text"].splitlines():
            matched = re.fullmatch(r"([+-]\d+)% to (Cold Resistance|all Elemental Resistances)", line.strip())
            if matched:
                values.append({"amount": int(matched[1]), "stat": "cold" if matched[2] == "Cold Resistance" else "elemental"})
        if values != row["values"] or row["complete_effect_text"] != (len(row["text"].splitlines()) == len(values)):
            raise ValueError("reward numerical recipe differs from explicit source text")
        if any(o["owner"] == subject(row["target"]) for o in rules["owners"]):
            raise ValueError("existing reward program must not be overwritten")
        nodes, effects = [], []
        for index, value in enumerate(values):
            name = f"amount-{index}"
            nodes.append({"id": name, "expression": {"kind": "literal", "value": quantity(value["amount"])}})
            effects.append({"id": name, "when": None, "effect": {"kind": "contribute", "entity": "player", "stat": stats[value["stat"]], "contribution": "add", "value": name}})
        program = {"id": "resistance-contribution", "context": "actor", "reads": [], "nodes": nodes, "effects": effects}
        rules["owners"].append({"owner": subject(row["target"]), "programs": complete([program]) if row["complete_effect_text"] else partial([program], row["target"], "other-selected-reward-effects-unconverted")})
    schema["definitions"].sort(key=lambda d: (DATA.DEFINITION_ORDER.index(d["kind"]), d["value"]["id"]["key"]))
    order = ["parameter", "choice", "grant", "actor", "skill_grant", "action_output"]
    schema["slots"].sort(key=lambda s: (order.index(s["kind"]), DATA.DEFINITION_ORDER.index(s["value"]["id"]["declaration"]["kind"]), s["value"]["id"]["declaration"]["definition"]["key"], s["value"]["id"]["slot"]["key"]))
    identity = DATA.schema_identity(schema)
    rules["definitions"] = identity
    recipe["routing"]["definitions"] = identity
    if registry["entries"][:len(old_entries)] != old_entries:
        raise ValueError("existing registry history changed")
    def codec(integer=False):
        return {"namespace": namespace, "whitespace": "exact", "codec": tag("integer", {"syntax": "integer"}) if integer else tag("quantity", {"syntax": "integer", "unit": unit, "scale": {"numerator": 1, "denominator": 1}})}
    def capture(name, integer=False):
        return {"id": name, "codec": tag("value", codec(integer))}
    def rule(name, pattern, captures, emissions):
        return {"id": name, "pattern": pattern, "captures": captures, "emissions": emissions}
    headers = [rule("sapphire-ring-template", [tag("literal", authoring["range_template"]["name"])], [], [tag("template", {"definition": template})])]
    for name, prefix in [("rarity", "Rarity: "), ("crafted", "Crafted: "), ("prefix", "Prefix: "), ("suffix", "Suffix: "), ("level-requirement", "LevelReq: "), ("implicit-count", "Implicits: "), ("item-level-metadata", "Item Level: "), ("quality-metadata", "Quality: "), ("sockets", "Sockets: "), ("rune", "Rune: ")]:
        headers.append(rule(name, [tag("literal", prefix), tag("capture", "text")], [{"id": "text", "codec": tag("opaque_text")}], [tag("metadata", {"role": "source-preamble-only"})]))
    outputs = {"recipe.json": pretty(recipe), "ids.json": pretty({"schema_version": 1, "namespace": namespace, "allocations": ids})}
    def numeric(name, sign="optional"):
        return tag("numeric_capture", {"capture": name, "syntax": "integer", "sign": sign})
    line_rules = copy.deepcopy(headers)
    for name, suffix in [("cold", "% to Cold Resistance"), ("elemental", "% to all Elemental Resistances"), ("life", " to maximum Life")]:
        definition, slot = modifiers[name]
        pattern = [numeric("amount"), tag("literal", suffix)]
        line_rules.append(rule("fixed-" + name, pattern, [capture("amount", name == "life")],
            [tag("modifier", {"definition": definition, "rolls": [{"slot": slot, "value": tag("capture", "amount")}]})]))
        if name == "life":
            continue
        for spelling, prefix in [("plus", "+("), ("bare", "(")]:
            pattern = [tag("literal", prefix), numeric("lower", "optional_minus"), tag("literal", "-"),
                numeric("upper", "optional_minus"), tag("literal", ")" + suffix)]
            value = tag("interpolate_offset", {"lower": "lower", "upper": "upper", "quantum": quantity(1),
                "rounding": "symmetric_half_offset"})
            line_rules.append(rule("ranged-" + spelling + "-" + name, pattern, [capture("lower"), capture("upper")],
                [tag("modifier", {"definition": definition, "rolls": [{"slot": slot, "value": value}]})]))
    items = {"schema_version": 1, "namespace": namespace, "version": "resistance-lines-v2", "definitions": identity,
        "whitespace": "trim_ascii", "rules": line_rules}
    source_policy = {"schema_version": 1, "namespace": namespace, "version": "resistance-layout-v2", "source": source,
        "item_lines": DATA.owned_digest("owned-item-line-policy-v1", items), "dialect": "pob_exported_single_text_v1",
        "rule_layouts": [{"rule": r["id"], "role": "header" if i < len(headers) else "single_modifier"} for i, r in enumerate(line_rules)],
        "template_layouts": [{"template": template, "load_index_prefix": "no_generated_buff_members"}]}
    outputs["items.json"] = pretty(items)
    outputs["item-source.json"] = pretty(source_policy)
    facts = {"schema_version": 1, "scope": "resistance-contribution-only", "base_recipe_sha256": sha(raw), "recipe_sha256": sha(outputs["recipe.json"]),
        "source": source, "source_spans": spans, "reward_outcomes": authoring["reward_contributions"],
        "receiver": {"implemented": False, "source": "src/Modules/CalcDefence.lua:926-979", "formula": "Absent an override: truncate_toward_zero((sum cold BASE + sum elemental BASE) * max(cold/elemental increase-more multiplier, 0)); final=max(min(total,truncated maximum),truncated minimum).",
            "obligations": ["exact common receiver ownership", "complete contributor membership", "override and increase/more inputs", "maximum/floor policies and special maximum branches", "separate selected player/owned actor identity"]},
        "structural_pattern_boundary": "NumericCapture uses a maximal ASCII integer token before semantic decoding. Fixed captures allow optional plus/minus; source range endpoints allow optional minus only. No successful-codec selection or runtime source execution.",
        "range_conversion": {"implemented": True, "scope": "Plain cold/all-elemental integer endpoint ranges with plus or absent outer sign; explicit source-attributed fraction; literal a+f*(b-a) arithmetic; literal signed source half-offset rounding. Outer minus, decimal endpoints, other line grammar and unsupported lifecycle remain pending.",
            "source_example_only": {"base": "Sapphire Ring", "lower": 20, "upper": 30, "fraction_and_value": [[0, 20], [0.5, 25], [1, 30]]},
            "required_generic_operations": ["maximal numeric lexical capture with explicit sign policy", "literal source signed half-offset rounding", "source-attributed range membership proof without source execution"]},
        "original_ring_attribution": {"status": "pending", "reason": "The exact original carries source modTags used by catalyst/modifier-magnitude semantics. UnsupportedTag blocks promotion of the cold modifier; no source metadata is silently removed.",
            "reviewed_tag": "{tags:cold_resistance,elemental_resistance,elemental,cold,resistance}",
            "diagnostic_only": "Range0/0.5/1 attribution is exercised on explicit in-memory copies removing exactly this one literal tag. This is not native success for the original."},
        "whole_item_closure": False, "metric_producer": False, "source_execution": False, "whole_original_native_completion": "0/5"}
    outputs["source-facts.json"] = pretty(facts)
    if sum(map(len, outputs.values())) > 16 * 1024 * 1024:
        raise ValueError("output byte bound exceeded")
    return outputs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["base-dir", "import-inputs", "authoring", "source-root"]:
        parser.add_argument("--" + name, type=Path, required=True)
    output = parser.add_mutually_exclusive_group(required=True)
    output.add_argument("--output-dir", type=Path)
    output.add_argument("--check-dir", type=Path)
    args = parser.parse_args()
    values = produce(args.base_dir, args.import_inputs, args.authoring, args.source_root)
    if args.check_dir:
        for name, value in values.items():
            if DATA.read(args.check_dir / name, MAX_BYTES) != value:
                raise ValueError("persisted resistance artifact differs: " + name)
        print(f"verified {len(values)} resistance component artifacts")
    else:
        args.output_dir.mkdir(parents=True, exist_ok=False)
        for name, value in values.items():
            with (args.output_dir / name).open("xb") as stream:
                stream.write(value)
        print(f"wrote {len(values)} component artifacts; Rust validation required")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError, RecursionError) as error:
        raise SystemExit(f"owned resistance export: {error}") from error
