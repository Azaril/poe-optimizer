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
    properties = authoring["modifier_properties"]
    if not isinstance(properties, list) or not 1 <= len(properties) <= 32:
        raise ValueError("modifier property collection bound exceeded")
    labels, keys = set(), set()
    for row in properties:
        DATA.fields(row, "label property", "modifier property binding")
        if any(not isinstance(row[field], str) or re.fullmatch(r"[a-z][a-z_]{0,63}", row[field]) is None for field in ["label", "property"]):
            raise ValueError("invalid bounded modifier property label/key")
        if row["label"] in labels or row["property"] in keys:
            raise ValueError("duplicate modifier property binding")
        labels.add(row["label"])
        keys.add(row["property"])
    catalyst = authoring["catalyst_inputs"]
    DATA.fields(catalyst, "selections default_amount amount_envelope missing_ordinary_quality", "catalyst input authoring")
    selections = catalyst["selections"]
    if not isinstance(selections, list) or not 1 <= len(selections) <= 32:
        raise ValueError("catalyst selection bound exceeded")
    seen = set()
    for row in selections:
        DATA.fields(row, "key source_name descriptor any_properties", "catalyst selection")
        if any(not isinstance(row[k], str) or not 1 <= len(row[k]) <= 64 for k in ["key", "source_name", "descriptor"]) or re.fullmatch(r"[a-z][a-z_]{0,63}", row["key"]) is None:
            raise ValueError("invalid catalyst selection label")
        if row["key"] in seen or not isinstance(row["any_properties"], list) or not 1 <= len(row["any_properties"]) <= 32:
            raise ValueError("duplicate catalyst key or property bound")
        seen.add(row["key"])
        if any(not isinstance(p, str) or re.fullmatch(r"[a-z][a-z_]{0,63}", p) is None for p in row["any_properties"]):
            raise ValueError("invalid catalyst property selector")
    if len({r["source_name"] for r in selections}) != len(selections) or len({r["descriptor"] for r in selections}) != len(selections):
        raise ValueError("duplicate catalyst selector")
    DATA.fields(catalyst["amount_envelope"], "minimum maximum", "catalyst computation envelope")
    lo, hi = catalyst["amount_envelope"]["minimum"], catalyst["amount_envelope"]["maximum"]
    if type(lo) is not int or type(hi) is not int or not -1000000 <= lo <= 0 < hi <= 1000000 or type(catalyst["default_amount"]) is not int or not lo <= catalyst["default_amount"] <= hi:
        raise ValueError("invalid catalyst amount envelope/default")
    if catalyst["missing_ordinary_quality"] not in ["pending", "absent"]:
        raise ValueError("invalid ordinary quality absence policy")
    scaling = authoring["catalyst_scaling"]
    DATA.fields(scaling, "factor_unit percentage_base additional_properties", "catalyst scaling authoring")
    additional = scaling["additional_properties"]
    if not isinstance(additional, list) or len(additional) > 32 or len(properties) + len(additional) > 32:
        raise ValueError("catalyst property collection bound exceeded")
    for row in additional:
        DATA.fields(row, "label property", "catalyst property binding")
        if any(not isinstance(row[k], str) or re.fullmatch(r"[a-z][a-z_]{0,63}", row[k]) is None for k in ["label", "property"]):
            raise ValueError("invalid catalyst property binding")
        if row["label"] in labels or row["property"] in keys:
            raise ValueError("duplicate catalyst property binding")
        labels.add(row["label"])
        keys.add(row["property"])
    all_properties = properties + additional
    selected_properties = set(p for row in selections for p in row["any_properties"])
    if any(p["property"] in selected_properties and p["label"] != p["property"] for p in all_properties):
        raise ValueError("catalyst property label must match the reviewed source predicate")
    if set(p["property"] for p in additional) != selected_properties - set(p["property"] for p in properties):
        raise ValueError("catalyst predicate coverage must be exact")
    if type(scaling["percentage_base"]) is not int or scaling["percentage_base"] != 100:
        raise ValueError("catalyst source arithmetic constants differ")
    mapping = load(base_dir / "mapping.json")
    if mapping["definitions"] != base["rules"]["definitions"] or mapping["registry"] != DATA.owned_digest("owned-id-registry-v1", base["registry"]):
        raise ValueError("base mapping binding differs")
    if DATA.schema_identity(base["schema"]) != base["rules"]["definitions"] or base["routing"]["definitions"] != base["rules"]["definitions"]:
        raise ValueError("base schema binding differs")
    source = copy.deepcopy(mapping["source"])
    pins = {f["path"]: f["sha256"] for f in source["files"]}
    spans, source_bytes, source_texts = [], 0, {}
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
        source_texts[span["path"]] = raw_source.decode("utf-8")
        lines = source_texts[span["path"]].splitlines(keepends=True)
        if not 1 <= span["line"] <= span["end_line"] <= len(lines):
            raise ValueError("source span outside file")
        selected = "".join(lines[span["line"] - 1:span["end_line"]])
        if sha(selected.encode()) != span["sha256"]:
            raise ValueError("source span hash differs")
        spans.append({**span, "text": selected})
    # Validate only these bounded literal tables; no source interpreter runs.
    item_source = source_texts.get("src/Classes/Item.lua", "")
    def literal_list(pattern):
        matches = re.findall(pattern, item_source, re.S)
        if len(matches) != 1:
            raise ValueError("missing/ambiguous pinned catalyst literal table")
        return re.findall(r'"([^"\\]*)"', matches[0])
    source_names = literal_list(r"local catalystList = (\{[^\n]+\})")
    source_descriptors = literal_list(r"local catalystDescriptorList = (\{[^\n]+\})")
    tag_tables = re.findall(r"local catalystTags = \{(.*?)\n\}", item_source, re.S)
    if len(tag_tables) != 1:
        raise ValueError("missing pinned catalyst property table")
    source_tags = [re.findall(r'"([^"\\]*)"', row) for row in re.findall(r"\{([^{}]*)\}", tag_tables[0])]
    if [r["source_name"] for r in selections] != source_names or [r["descriptor"] for r in selections] != source_descriptors or [r["any_properties"] for r in selections] != source_tags:
        raise ValueError("catalyst authoring differs from pinned literal tables")
    if catalyst["default_amount"] != 20 or not re.search(r"if not quality then\s+quality = 20", item_source):
        raise ValueError("catalyst default differs from reviewed source fallback")
    if catalyst["missing_ordinary_quality"] == "absent":
        base_evidence = authoring["range_template"]
        first, last = base_evidence["source_lines"]
        base_lines = source_texts.get(base_evidence["source_path"], "").splitlines()
        if type(first) is not int or type(last) is not int or not 1 <= first <= last <= len(base_lines) or last - first > 64:
            raise ValueError("ordinary quality base evidence bounds differ")
        fragment = "\n".join(base_lines[first - 1:last])
        if not fragment.startswith('itemBases["' + base_evidence["name"] + '"] = {') or not fragment.rstrip().endswith("}") or re.search(r"\bquality\s*=", fragment):
            raise ValueError("ordinary quality absence lacks exact base evidence")
    if not re.search(r"return \(100 \+ quality\) / 100", item_source):
        raise ValueError("catalyst scalar source expression differs")
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
    factor_unit = scaling["factor_unit"]
    factor_schema = next((d for d in schema["definitions"] if d["value"]["id"] == factor_unit), None)
    if factor_schema is None or factor_schema["value"]["schema"] != tag("known", {"dimension": "dimensionless_factor"}):
        raise ValueError("exact dimensionless factor unit is not known")
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
    # Preserve the first 2,524 registry entries and existing fixed-value meanings.
    # Nominal inputs retain explicit incomplete effective-transform coverage.
    nominal_modifiers = {}
    for name in ["cold", "elemental"]:
        ports = declarations()
        definition = allocate("nominal-" + name + "-modifier", "modifier", {"declarations": ports})
        slot = allocate("nominal-" + name + "-roll", "parameter_slot", {"value": value_schema(), "presence": "required_once", "sites": ["modifier_roll"]}, definition)
        ports["parameters"]["members"].append(slot)
        property_slots = []
        for binding in properties:
            property_slot = allocate("nominal-" + name + "-property-" + binding["property"], "parameter_slot", {"value": tag("boolean"), "presence": "required_once", "sites": ["modifier_roll"]}, definition)
            ports["parameters"]["members"].append(property_slot)
            property_slots.append((binding["property"], property_slot))
        nominal_modifiers[name] = (definition, slot, property_slots)
        payload["modifiers"]["members"].append(definition)
        rules["owners"].append({"owner": subject(definition), "programs": partial([], definition, "nominal-item-scaling-inputs-unconverted")})
    no_catalyst = allocate("catalyst-none", "option", {})
    catalyst_options = [(row, allocate("catalyst-" + row["key"], "option", {})) for row in selections]
    catalyst_kind = allocate("item-catalyst-kind", "parameter_slot", {"value": tag("option", {"allowed": complete([no_catalyst] + [identifier for _, identifier in catalyst_options])}), "presence": "required_once", "sites": ["item_parameter"]}, template)
    catalyst_amount = allocate("item-catalyst-enabled-amount", "parameter_slot", {"value": tag("quantity", {"minimum": quantity(lo)["value"], "maximum": quantity(hi)["value"]}), "presence": "required_once", "sites": ["item_parameter"]}, template)
    payload["declarations"]["parameters"]["members"].extend([catalyst_kind, catalyst_amount])
    # New consumer-required facts append after the entire prior 2,554-ID history.
    for name, (definition, _, property_slots) in nominal_modifiers.items():
        ports = next(d["value"]["schema"]["value"]["declarations"] for d in schema["definitions"] if d["value"]["id"] == definition)
        for binding in additional:
            parameter = allocate("nominal-" + name + "-property-" + binding["property"], "parameter_slot", {"value": tag("boolean"), "presence": "required_once", "sites": ["modifier_roll"]}, definition)
            ports["parameters"]["members"].append(parameter)
            property_slots.append((binding["property"], parameter))
    equipment_kind = allocate("equipment-catalyst-kind", "stat", {"value": tag("option"), "targets": ["equipment_use"]})
    equipment_amount = allocate("equipment-catalyst-enabled-amount", "stat", {"value": quantity_type(), "targets": ["equipment_use"]})
    modifier_scalar = allocate("modifier-catalyst-scalar", "stat", {"value": tag("quantity", {"unit": factor_unit}), "targets": ["modifier"]})
    unscalable_slots = {}
    for name, (definition, _, _) in nominal_modifiers.items():
        slot = allocate("nominal-" + name + "-unscalable", "parameter_slot", {"value": tag("boolean"), "presence": "required_once", "sites": ["modifier_roll"]}, definition)
        next(d["value"]["schema"]["value"]["declarations"]["parameters"]["members"] for d in schema["definitions"] if d["value"]["id"] == definition).append(slot)
        unscalable_slots[name] = slot
    template_program = {"id": "catalyst-inputs", "context": "equipment_use", "reads": [], "nodes": [], "effects": []}
    for name, slot, stat, value_type in [("catalyst-kind", catalyst_kind, equipment_kind, tag("option")), ("catalyst-amount", catalyst_amount, equipment_amount, quantity_type())]:
        template_program["reads"].append({"id": name, "value_type": value_type, "source": tag("parameter", {"slot": slot})})
        template_program["nodes"].append({"id": name, "expression": {"kind": "read", "input": name}})
        template_program["effects"].append({"id": name, "when": None, "effect": {"kind": "derive", "entity": "current", "stat": stat, "value": name}})
    next(o for o in rules["owners"] if o["owner"] == subject(template))["programs"]["members"].append(template_program)
    for name, (definition, amount_slot, property_slots) in nominal_modifiers.items():
        program = {"id": "catalyst-scalar", "context": "equipment_use", "reads": [], "nodes": [], "effects": []}
        def add_node(identifier, kind, **fields):
            program["nodes"].append({"id": identifier, "expression": {"kind": kind, **fields}})
            return identifier
        def add_read(identifier, value_type, source):
            program["reads"].append({"id": identifier, "value_type": value_type, "source": source})
            return add_node(identifier, "read", input=identifier)
        add_read("unscalable", tag("boolean"), tag("parameter", {"slot": unscalable_slots[name]}))
        add_read("catalyst-kind", tag("option"), tag("stat", {"entity": "current", "stat": equipment_kind}))
        add_read("catalyst-amount", quantity_type(), tag("stat", {"entity": "current", "stat": equipment_amount}))
        for property_key, property_slot in property_slots:
            if property_key in selected_properties:
                add_read("property-" + property_key, tag("boolean"), tag("parameter", {"slot": property_slot}))
        matches = []
        for selector, option in catalyst_options:
            key = selector["key"]
            add_node("option-" + key, "literal", value=tag("option", option))
            add_node("selected-" + key, "compare", operation="equal", left="catalyst-kind", right="option-" + key)
            add_node("properties-" + key, "any", values=["property-" + prop for prop in selector["any_properties"]])
            matches.append(add_node("applies-" + key, "all", values=["selected-" + key, "properties-" + key]))
        add_node("catalyst-applicable", "any", values=matches)
        add_node("percentage-base", "literal", value=quantity(scaling["percentage_base"]))
        add_node("enabled-percentage", "add", left="percentage-base", right="catalyst-amount")
        add_node("enabled-factor", "ratio", numerator="enabled-percentage", denominator="percentage-base", unit=factor_unit)
        add_node("one", "literal", value=tag("quantity", {"value": 1.0, "unit": factor_unit}))
        add_node("eligible-factor", "select", condition="catalyst-applicable", when_true="enabled-factor", when_false="one")
        add_node("catalyst-factor", "select", condition="unscalable", when_true="one", when_false="eligible-factor")
        program["effects"].append({"id": "catalyst-scalar", "when": None, "effect": {"kind": "derive", "entity": "modifier", "stat": modifier_scalar, "value": "catalyst-factor"}})
        owner = next(o for o in rules["owners"] if o["owner"] == subject(definition))
        owner["programs"] = {"members": [program], "closure": tag("partial", {"gaps": [
            {"subject": subject(definition), "facet": "game_rules", "code": code} for code in [
                "source-encoding-and-corrupted-range-unproved", "numeric-component-scalability-unproved", "remaining-ordered-magnitude-transforms-unconverted"]]})}
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
    kind_codec = {"namespace": namespace, "whitespace": "exact", "codec": tag("option", {"tokens": [{"token": row["source_name"], "value": identifier} for row, identifier in catalyst_options]})}
    amount_codec = {"namespace": namespace, "whitespace": "exact", "codec": tag("quantity", {"syntax": "decimal", "unit": unit, "scale": {"numerator": 1, "denominator": 1}})}
    for name, prefix, value_codec, parameter in [("catalyst-kind", "Catalyst: ", kind_codec, catalyst_kind), ("catalyst-amount", "CatalystQuality: ", amount_codec, catalyst_amount)]:
        headers.append(rule(name, [tag("literal", prefix), tag("capture", "value")], [{"id": "value", "codec": tag("value", value_codec)}], [tag("item_parameter", {"slot": parameter, "value": tag("capture", "value")})]))
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
            nominal_definition, nominal_slot, property_slots = nominal_modifiers[name]
            rolls = [{"slot": nominal_slot, "value": value}] + [{"slot": property_slot, "value": tag("property", {"property": property_key})} for property_key, property_slot in property_slots]
            # Exact admitted grammar excludes every source unscalable form; this is not a modTags label.
            rolls.append({"slot": unscalable_slots[name], "value": tag("literal", tag("boolean", False))})
            line_rules.append(rule("ranged-" + spelling + "-" + name, pattern, [capture("lower"), capture("upper")],
                [tag("modifier", {"definition": nominal_definition, "rolls": rolls})]))
    items = {"schema_version": 2, "namespace": namespace, "version": "resistance-lines-v5", "definitions": identity,
        "whitespace": "trim_ascii", "rules": line_rules}
    source_policy = {"schema_version": 3, "namespace": namespace, "version": "resistance-layout-v5", "source": source,
        "item_lines": DATA.owned_digest("owned-item-line-policy-v2", items), "dialect": "pob_exported_single_text_v1",
        "rule_layouts": [{"rule": r["id"], "role": "header" if i < len(headers) else "single_modifier"} for i, r in enumerate(line_rules)],
        "template_layouts": [{"template": template, "load_index_prefix": "no_generated_buff_members"}], "property_bindings": all_properties,
        "template_defaults": [{"template": template, "parameters": [
            {"assignment": {"slot": catalyst_kind, "value": tag("option", no_catalyst)}, "headers": ["Catalyst"]},
            {"assignment": {"slot": catalyst_amount, "value": quantity(catalyst["default_amount"])}, "headers": ["CatalystQuality"]}],
            "item_level": "absent", "quality": catalyst["missing_ordinary_quality"]}]}
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
        "catalyst_inputs": {"implemented": True, "selections": selections, "canonical_headers": ["Catalyst", "CatalystQuality"], "descriptor_aliases": "Recorded as source evidence; not admitted by this canonical exported-header policy.",
            "amount_meaning": "Scalar amount in percentage points to use when the catalyst is enabled; not a claim that an amount header was authored.", "missing_amount_default": catalyst["default_amount"], "no_catalyst": no_catalyst,
            "default_authority": "Exact template plus proven canonical source layout and absence of all declared header names and authored/pending field occurrences; explicit zero wins, malformed/unknown input does not default.",
            "ordinary_quality": "Separate ItemRecord.quality; proved omission is explicit absence only for this reviewed template whose pinned base has no quality field. Authored Quality blocks the absence default.", "effective_scaling": False},
        "catalyst_scalar": {"implemented": True, "scope": "Unrounded dimensionless intermediate on the exact modifier occurrence; not a final resistance contribution.", "factor_unit": factor_unit,
            "formula": "If unscalable is true:1. Otherwise if any selected catalyst predicate is true: (100 + enabled amount) / 100; otherwise1. Shared item inputs are transported by exact EquipmentUse identity.",
            "property_predicates": sorted(selected_properties), "unknown_property_behavior": "Required typed inputs never default in the native evaluator. The source adapter alone proves false for absent labels after complete label scanning.",
            "unscalable_input": "Required Boolean member metadata distinct from modTags. Literal false is limited to the exact admitted plain range grammar; other flags/suffixes remain pending.", "final_magnitude_application": False, "remaining_obligations": ["source nominal versus baked encoding", "per-member corruptedRange and numeric-component scalability", "ordered additive and multiplicative magnitude operations", "complete incoming transform membership"]},
        "modifier_properties": {"implemented": True, "bindings": all_properties, "value_encoding": "nominal_integer_range", "property_set": "Every label on the selected source member must be mapped and consumed; unknown or unconsumed labels remain Pending. Absence is not a runtime default.", "effective_scaling": False,
            "remaining_obligations": ["remaining ordered transform inputs and final magnitude application", "source nominal versus baked encoding for other source families", "category and numeric-component scalability", "applicable ordered modifier-magnitude transforms", "complete owning-item inputs and contributors"]},
        "original_ring_attribution": {"status": "pending", "input_status": "nominal_amount_and_twenty_properties_converted", "reason": "The untouched original's five labels and source-attributed range become owned nominal inputs, with fifteen additional catalyst predicates explicitly false from complete source labels. Shared item inputs feed a modifier-occurrence catalyst scalar only; encoding, scalability and ordered magnitude obligations remain Partial. No final effective Contribute is emitted.",
            "reviewed_tag": "{tags:cold_resistance,elemental_resistance,elemental,cold,resistance}",
            "diagnostic_only": "Range0/0.5/1 contrasts edit only XML fractions in memory and retain the actual tag. Nominal conversion is not effective scaling or native original success."},
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
