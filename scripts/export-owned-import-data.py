#!/usr/bin/env python3
"""Build explicit offline import seed artifacts from reviewed injected metadata.

No source execution or numerical extraction occurs. The standalone identity
catalog is projected unchanged; finite reward/slot policy data is authored into
a new registry tail. Existing recipe identities, schemas and rule bodies remain
unchanged. Rust constructors and the production assembler validate every output.
"""
from __future__ import annotations

import argparse
import copy
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import re


@dataclass(frozen=True)
class Limits:
    snapshot_bytes: int = 32 * 1024 * 1024
    artifact_bytes: int = 8 * 1024 * 1024
    source_file_bytes: int = 8 * 1024 * 1024
    source_total_bytes: int = 48 * 1024 * 1024
    output_bytes: int = 16 * 1024 * 1024
    source_files: int = 64
    catalog_rows: int = 10_000
    seed_rows: int = 4_096
    reward_rules: int = 256
    reward_options: int = 4_096
    queries: int = 1_024


HARD = Limits()
DEFINITION_ORDER = "class ascendancy reward item_template modifier gem skill passive_node point_pool equipment_slot encounter metric option action_part action_mode action_stat_set usage_policy skill_link_role socket_slot unit quality external_input stat capability".split()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def compact(value):
    return json.dumps(value, ensure_ascii=False, allow_nan=False, separators=(",", ":")).encode()


def pretty(value):
    return (json.dumps(value, ensure_ascii=False, allow_nan=False, indent=2) + "\n").encode()


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def reject_constant(value):
    raise ValueError(f"nonfinite JSON literal: {value}")


def decode(data):
    return json.loads(data, object_pairs_hook=unique, parse_constant=reject_constant)


def read(path, maximum):
    with Path(path).open("rb") as stream:
        data = stream.read(maximum + 1)
    if len(data) > maximum:
        raise ValueError(f"input byte bound exceeded: {path}")
    return data


def fields(value, expected, label):
    if not isinstance(value, dict) or set(value) != set(expected.split()):
        raise ValueError(f"unreviewed {label} fields")


def bounded(value, maximum, label):
    if not isinstance(value, list) or len(value) > maximum:
        raise ValueError(f"{label} collection bound exceeded")


def tag(kind, value=None):
    return {"kind": kind} if value is None else {"kind": kind, "value": value}


def text(value):
    return tag("text", value)


def subject(identifier):
    return tag("definition", tag(identifier["kind"], identifier))


def owned_digest(domain, value):
    encoded = domain.encode()
    return sha(b"poe-optimizer-owned-content-v1\0" + len(encoded).to_bytes(8, "little") + encoded + compact(value))


def schema_identity(schema):
    return {"game": schema["namespace"]["game"], "release": schema["release"],
            "schema_version": schema["schema_version"], "content_sha256": sha(compact(schema)),
            "semantics_version": schema["semantics_version"]}


def mapping_order(entry):
    source = entry["source"]
    value = source["value"]
    if source["kind"] == "definition":
        inner = value["value"]
        if value["kind"] == "gem":
            return (0, 5, inner["game_id"]["value"], inner["variant_id"]["value"])
        return (0, 6, inner["effect_id"]["value"])
    if source["kind"] == "catalog":
        return (1, 1, value["key"]["value"])
    return (5, value["key"]["value"], ["input", "placeholder", "default"].index(value["source"]),
            ["parameter", "choice", "reward", "option"].index(value["role"]), value["value"]["value"])


def produce(snapshot_bytes, base_bytes, ids_bytes, mechanics_bytes, facts_bytes,
            reward_bytes, query_bytes, authoring_bytes, source_root, limits=HARD):
    for name, hard in vars(HARD).items():
        value = getattr(limits, name)
        if type(value) is not int or not 0 < value <= hard:
            raise ValueError(f"invalid {name} limit")
    if len(snapshot_bytes) > limits.snapshot_bytes or any(len(b) > limits.artifact_bytes for b in
            [base_bytes, ids_bytes, mechanics_bytes, facts_bytes, reward_bytes, query_bytes, authoring_bytes]):
        raise ValueError("input byte bound exceeded")
    base, ids, mechanics, facts, rewards, queries, authoring = map(decode,
        [base_bytes, ids_bytes, mechanics_bytes, facts_bytes, reward_bytes, query_bytes, authoring_bytes])
    fields(authoring, "schema_version scope catalog_policy seed_bindings equipment_slots extra_source_files quality_label quality_unit_label normalization_version reward_version item_version item_source_version", "authoring")
    if authoring["schema_version"] != 1 or authoring["scope"] != "reviewed-owned-import-seed":
        raise ValueError("unsupported authoring format")
    normalized_snapshot = snapshot_bytes.replace(b"\r\n", b"\n")
    if sha(normalized_snapshot) != mechanics["catalog_lf_sha256"] or sha(base_bytes) != facts["recipe_sha256"] or sha(mechanics_bytes) != facts["manifest_sha256"]:
        raise ValueError("base recipe/source evidence binding mismatch")
    snapshot = decode(normalized_snapshot)
    if snapshot["manifest"]["schema_version"] != 40:
        raise ValueError("unreviewed snapshot version")
    catalog = snapshot["skill_identities"]
    fields(catalog, "schema_version capability source gem_declarations skill_declarations gems skills missing_references", "identity catalog")
    if catalog["schema_version"] != 1 or catalog["capability"] != "identity_only":
        raise ValueError("unreviewed catalog capability")
    for name in ["gems", "skills", "gem_declarations", "skill_declarations", "missing_references"]:
        bounded(catalog[name], limits.catalog_rows, name)
    namespace = base["registry"]["namespace"]
    if ids["namespace"] != namespace or base["schema"]["namespace"] != namespace or base["rules"]["definitions"] != schema_identity(base["schema"]) or base["routing"]["definitions"] != base["rules"]["definitions"]:
        raise ValueError("base identity/schema binding mismatch")
    source = {"system": "path_of_building2", "revision": catalog["source"]["upstream_revision"], "files": []}
    source_hashes = dict(catalog["source"]["files"])
    for pin in [mechanics["source"], rewards["source"]]:
        if pin["system"] != source["system"] or pin["revision"] != source["revision"]:
            raise ValueError("foreign source revision/system")
        for file in pin["files"]:
            if file["path"] in source_hashes and source_hashes[file["path"]] != file["sha256"]:
                raise ValueError("conflicting source file hash")
            source_hashes[file["path"]] = file["sha256"]
    bounded(authoring["extra_source_files"], limits.source_files, "extra source")
    for file in authoring["extra_source_files"]:
        fields(file, "path sha256", "extra source file")
        if file["path"] in source_hashes and source_hashes[file["path"]] != file["sha256"]:
            raise ValueError("conflicting source file hash")
        source_hashes[file["path"]] = file["sha256"]
    if len(source_hashes) > limits.source_files:
        raise ValueError("source file collection bound exceeded")
    root, total_source = Path(source_root).resolve(), 0
    for name, expected in sorted(source_hashes.items()):
        path = (root / name).resolve()
        if not name.startswith("src/") or "\\" in name or not path.is_relative_to(root):
            raise ValueError("source path is not an exact src/ identity")
        raw = read(path, limits.source_file_bytes).replace(b"\r\n", b"\n")
        total_source += len(raw)
        if total_source > limits.source_total_bytes or (expected is not None and sha(raw) != expected):
            raise ValueError("source aggregate bound/hash mismatch")
        source["files"].append({"path": name, "sha256": sha(raw)})
    bounded(authoring["seed_bindings"], limits.seed_rows, "seed bindings")
    bounded(authoring["equipment_slots"], limits.seed_rows, "equipment slots")
    bounded(rewards["rows"], limits.reward_rules, "reward rules")
    if rewards["schema_version"] != 1 or rewards["effects_compiled"] or rewards["package"]["schema_version"] != 40:
        raise ValueError("unreviewed reward evidence")
    option_count = 0
    for row in rewards["rows"]:
        choice = row["choice"]
        if choice["kind"] == "option":
            bounded(choice["options"], limits.reward_options, "reward options")
            option_count += len(choice["options"])
    if option_count > limits.reward_options:
        raise ValueError("aggregate reward option bound exceeded")
    seed = copy.deepcopy(base)
    registry, schema = seed["registry"], seed["schema"]
    initial_count = len(registry["entries"])
    bounded(registry["entries"], limits.seed_rows, "base registry")
    new_allocations = sum(1 if r["choice"]["kind"] == "boolean" else 2 * len(r["choice"]["options"]) - 1 for r in rewards["rows"]) + len(authoring["equipment_slots"])
    if initial_count + new_allocations > limits.seed_rows:
        raise ValueError("combined seed registry bound exceeded")
    if registry["last_issued"] != initial_count or registry["revision"] < initial_count:
        raise ValueError("base registry history is not contiguous")
    ledger, entries = [], []
    def allocate(kind, label, payload):
        registry["last_issued"] += 1
        registry["revision"] += 1
        identifier = {"kind": kind, "namespace": namespace, "key": f'def.{registry["last_issued"]:016x}'}
        registry["entries"].append({"sequence": registry["last_issued"], "target": subject(identifier), "state": tag("active")})
        schema["definitions"].append(tag(kind, {"id": identifier, "schema": tag("known", payload)}))
        ledger.append({"label": label, "id": identifier})
        return identifier
    def mapped(selector, identifier, alias=None):
        basis = tag("exact") if alias is None else tag("reviewed_alias", {"reason": alias})
        entries.append({"source": selector, "outcome": tag("mapped", {"target": subject(identifier), "basis": basis})})
    gems = {g["key"]: g for g in catalog["gems"]}
    skills = {s["id"]: s for s in catalog["skills"]}
    if len(gems) != len(catalog["gems"]) or len(skills) != len(catalog["skills"]):
        raise ValueError("duplicate final catalog identity")
    for binding in authoring["seed_bindings"]:
        fields(binding, "owned_label catalog_key source", "seed binding")
        identifier = ids["allocations"][binding["owned_label"]]
        selector = binding["source"]
        if identifier["kind"] == "gem":
            gem = gems[binding["catalog_key"]]
            exact = tag("definition", tag("gem", {"game_id": text(gem["game_id"]), "variant_id": text(gem["variant_id"])}))
        elif identifier["kind"] == "skill":
            skill = skills[binding["catalog_key"]]
            exact = tag("definition", tag("skill", {"effect_id": text(skill["id"])}))
        else:
            raise ValueError("seed binding has unsupported target family")
        if selector != exact or not any(e["target"] == subject(identifier) and e["state"] == tag("active") for e in registry["entries"][:initial_count]):
            raise ValueError("seed binding is not exact active catalog identity")
        mapped(selector, identifier)
    declarations = {name: {"members": [], "closure": tag("complete")} for name in ["parameters", "choices", "grants", "actors", "skill_grants", "outputs", "sockets"]}
    def config_selector(key, token, lane, role):
        return tag("configuration", {"key": text(key), "source": lane, "role": role, "value": text(token)})
    def reward(key, token, is_default):
        identifier = allocate("reward", f"reward:{key}:{token}", {"declarations": copy.deepcopy(declarations)})
        selector = config_selector(key, token, "input", "reward")
        mapped(selector, identifier)
        if is_default:
            mapped(config_selector(key, token, "default", "reward"), identifier, "declared-default-equals-fixed-input-outcome")
        return {"kind": "reward", "selector": selector, "parameters": []}
    reward_rules, reward_keys = [], set()
    for row in rewards["rows"]:
        key, choice = row["config_key"], row["choice"]
        if key in reward_keys:
            raise ValueError("duplicate reward input key")
        reward_keys.add(key)
        outcomes = []
        if choice["kind"] == "boolean":
            if type(choice["default"]) is not bool or not choice["stat_text"]:
                raise ValueError("malformed boolean reward metadata")
            selected = reward(key, "true", choice["default"])
            outcomes = [{"when": tag("boolean", False), "outcome": tag("none")}, {"when": tag("boolean", True), "outcome": selected}]
            codec = tag("boolean", {"tokens": [{"token": "false", "value": False}, {"token": "true", "value": True}]})
            lane, default = "input_boolean", tag("boolean", choice["default"])
        elif choice["kind"] == "option":
            options = choice["options"]
            if len(options) != len(set(options)) or choice["default"] not in options or choice["none_token"] not in options:
                raise ValueError("malformed option reward metadata")
            tokens, default = [], None
            for token in options:
                identifier = allocate("option", f"option:{key}:{token}", {})
                mapped(config_selector(key, token, "input", "option"), identifier)
                if token == choice["default"]:
                    default = tag("option", identifier)
                    mapped(config_selector(key, token, "default", "option"), identifier, "declared-default-equals-fixed-input-outcome")
                selected = tag("none") if token == choice["none_token"] else reward(key, token, token == choice["default"])
                outcomes.append({"when": tag("option", identifier), "outcome": selected})
                tokens.append({"token": token, "value": identifier})
            codec, lane = tag("option", {"tokens": tokens}), "input_string"
        else:
            raise ValueError("unsupported reward choice")
        recipe = {"id": "quest-recipe-" + sha(key.encode()), "codec": {"namespace": namespace, "whitespace": "exact", "codec": codec},
                  "tiers": [{"selectors": [{"lane": lane, "name": key}], "duplicates": "last_in_source_order"}], "missing": {"kind": "explicit", "value": default}}
        reward_rules.append({"recipe": recipe, "outcomes": outcomes})
    equipment, source_slots = [], set()
    for group in authoring["equipment_slots"]:
        fields(group, "label source_slots scope loadouts", "equipment slot")
        bounded(group["source_slots"], limits.seed_rows, "source slots")
        if not group["source_slots"] or group["scope"] not in ["selected", "shared"]:
            raise ValueError("invalid equipment scope")
        if (group["scope"] == "selected" and len(group["loadouts"]) != len(group["source_slots"])) or (group["scope"] == "shared" and group["loadouts"]):
            raise ValueError("equipment source/loadout relation differs")
        destination = allocate("equipment_slot", group["label"], {"scope": group["scope"]})
        for index, name in enumerate(group["source_slots"]):
            if name in source_slots:
                raise ValueError("duplicate equipment source slot")
            source_slots.add(name)
            selector = tag("catalog", {"kind": "equipment_slot", "key": text(name), "version": tag("missing"), "variant": tag("missing")})
            mapped(selector, destination, None if index == 0 else "weapon-swap-same-receiving-slot")
            scope = tag("shared") if group["scope"] == "shared" else tag("selected", {"loadouts": [group["loadouts"][index]]})
            equipment.append({"source_slot": text(name), "destination": destination, "scope": scope})
    schema["definitions"].sort(key=lambda row: (DEFINITION_ORDER.index(row["kind"]), row["value"]["id"]["key"]))
    identity = schema_identity(schema)
    seed["rules"]["definitions"] = identity
    seed["routing"]["definitions"] = identity
    entries.sort(key=mapping_order)
    if len({compact(e["source"]) for e in entries}) != len(entries):
        raise ValueError("duplicate source selector")
    mapping = {"schema_version": 1, "namespace": namespace, "registry": owned_digest("owned-id-registry-v1", registry), "definitions": identity,
               "source": source, "policy_version": authoring["catalog_policy"]["version"], "entries": entries}
    reward_policy = {"schema_version": 1, "namespace": namespace, "version": authoring["reward_version"], "definitions": identity,
                     "mapping": owned_digest("owned-external-mapping-v1", mapping), "rules": reward_rules}
    def attribute_recipe(name, boolean=False):
        codec = tag("boolean", {"tokens": [{"token": "true", "value": True}, {"token": "false", "value": False}]}) if boolean else tag("integer", {"syntax": "integer"})
        return {"id": "reviewed-enabled" if boolean else "reviewed-level", "codec": {"namespace": namespace, "whitespace": "exact", "codec": codec},
                "tiers": [{"selectors": [{"lane": "attribute", "name": name}], "duplicates": "reject"}],
                "missing": {"kind": "explicit", "value": tag("boolean", True)} if boolean else tag("pending")}
    quality, unit = ids["allocations"][authoring["quality_label"]], ids["allocations"][authoring["quality_unit_label"]]
    if quality["kind"] != "quality" or unit["kind"] != "unit":
        raise ValueError("quality/unit binding family differs")
    amount = {"id": "reviewed-explicit-gem-quality", "codec": {"namespace": namespace, "whitespace": "exact", "codec": tag("quantity", {
        "syntax": "decimal", "unit": unit, "scale": {"numerator": 1, "denominator": 1}})},
        "tiers": [{"selectors": [{"lane": "attribute", "name": "quality"}], "duplicates": "reject"}], "missing": tag("pending")}
    normalization = {"version": authoring["normalization_version"], "namespace": namespace,
        "character_level": attribute_recipe("level"), "gem_level": attribute_recipe("level"), "gem_enabled": attribute_recipe("enabled", True),
        "group_enabled": attribute_recipe("enabled", True), "manual_skill_sources": [tag("missing"), text("")], "empty_item_keys": [text("0")],
        "generated_support_prefixes": [], "allocation_attribute": "nodes", "single_active_support_target": True, "equipment_loadouts": equipment,
        "gem_quality": tag("attributes", {"definitions": identity, "amount": amount, "kind_attribute": "qualityId", "kinds": [{"source": tag("missing"), "kind": quality}]})}
    items = {"schema_version": 1, "namespace": namespace, "version": authoring["item_version"], "definitions": identity, "whitespace": "exact", "rules": []}
    item_source = {"schema_version": 1, "namespace": namespace, "version": authoring["item_source_version"], "source": source,
        "item_lines": owned_digest("owned-item-line-policy-v1", items), "dialect": "pob_exported_single_text_v1", "rule_layouts": [], "template_layouts": []}
    outputs = {"recipe-seed.json": seed, "skill-identities.json": catalog, "source-pin.json": source, "skill-catalog-policy.json": authoring["catalog_policy"],
        "mapping-seed.json": mapping, "normalization-policy-seed.json": normalization, "reward-policy-seed.json": reward_policy,
        "item-policy-seed.json": items, "item-source-policy-seed.json": item_source, "reward-source-facts.json": rewards,
        "allocations.json": {"schema_version": 1, "base_registry": owned_digest("owned-id-registry-v1", base["registry"]), "seed_registry": mapping["registry"], "entries": ledger}}
    bounded(queries["cases"], limits.queries, "query cases")
    query_evidence, total_queries = [], 0
    for ordinal, case in enumerate(queries["cases"], 1):
        rows = case["measurements"]
        bounded(rows, limits.queries, "query rows")
        total_queries += len(rows)
        if total_queries > limits.queries:
            raise ValueError("aggregate query bound exceeded")
        projected = []
        for index, row in enumerate(rows):
            query = row["query"]
            if query["actor"] not in ["player", "selected_minion"]:
                raise ValueError("unreviewed requested query target")
            target = tag("player") if query["actor"] == "player" else tag("unresolved", "reference-selected-minion-unresolved")
            projected.append({"id": f"reference-{index:02}", "metric": tag("catalog", {"kind": "metric", "key": text(query["id"]), "version": tag("missing"), "variant": tag("missing")}), "target": target})
        filename = f"queries/original-{ordinal:02}.json"
        outputs[filename] = projected
        query_evidence.append({"file": filename, "source_line": case["source_line"], "input": case["input"], "rows": len(projected), "numerical_results_used": False})
    encoded = {name: pretty(value) for name, value in outputs.items()}
    evidence = {"schema_version": 1, "scope": "owned-import-input-seed", "base_recipe_sha256": sha(base_bytes), "catalog_snapshot_lf_sha256": sha(normalized_snapshot),
        "mechanics_manifest_sha256": sha(mechanics_bytes), "reward_facts_sha256": sha(reward_bytes), "query_manifest_lf_sha256": sha(query_bytes.replace(b"\r\n", b"\n")),
        "authoring_sha256": sha(authoring_bytes), "seed_schema": identity, "source": source,
        "counts": {"base_allocations": initial_count, "new_allocations": len(ledger), "seed_watermark": registry["last_issued"], "gems": len(catalog["gems"]), "skills": len(catalog["skills"]), "reward_rules": len(reward_rules), "equipment_relations": len(equipment)},
        "queries": query_evidence, "source_execution": False, "numeric_coverage": False,
        "artifacts": [{"path": name, "bytes": len(data), "sha256": sha(data)} for name, data in sorted(encoded.items())]}
    encoded["provenance.json"] = pretty(evidence)
    if any(len(data) > limits.artifact_bytes for data in encoded.values()) or sum(map(len, encoded.values())) > limits.output_bytes:
        raise ValueError("output byte bound exceeded")
    return encoded



def bind_policies(seed_outputs, compiled, limits=HARD):
    """Explicit offline successor edit after exact preservation/binding checks.

    This is not a loader repair path. Rust has already produced the immutable
    bundle; every old descriptor, mapping and rule body must remain unchanged.
    """
    if sum(map(len, compiled.values())) > limits.output_bytes or any(len(b) > limits.artifact_bytes for b in compiled.values()):
        raise ValueError("compiled artifact byte bound exceeded")
    transition = decode(compiled["transition.json"])
    for artifact in transition["artifacts"]:
        raw = compiled.get(artifact["file"])
        if raw is None or len(raw) != artifact["bytes"] or sha(raw) != artifact["sha256"]:
            raise ValueError("compiled transition artifact hash mismatch")
    before = decode(seed_outputs["recipe-seed.json"])
    previous_mapping = decode(seed_outputs["mapping-seed.json"])
    after = decode(compiled["recipe.json"])
    mapping = decode(compiled["mapping.json"])
    before_identity, after_identity = before["rules"]["definitions"], after["rules"]["definitions"]
    if transition["before_registry"] != owned_digest("owned-id-registry-v1", before["registry"]) or transition["before_definitions"] != before_identity or transition["before_mapping"] != owned_digest("owned-external-mapping-v1", previous_mapping):
        raise ValueError("transition does not begin at the exact seed")
    if transition["after_registry"] != owned_digest("owned-id-registry-v1", after["registry"]) or transition["after_definitions"] != after_identity or transition["after_mapping"] != owned_digest("owned-external-mapping-v1", mapping):
        raise ValueError("transition successor identity differs")
    if mapping["definitions"] != after_identity or mapping["registry"] != transition["after_registry"] or mapping["source"] != previous_mapping["source"] or mapping["policy_version"] != previous_mapping["policy_version"]:
        raise ValueError("mapping successor context differs")
    if schema_identity(after["schema"]) != after_identity or after["registry"]["namespace"] != before["registry"]["namespace"]:
        raise ValueError("successor schema identity/namespace differs")
    for field, filename in [("schema", "schema.json"), ("registry", "registry.json"), ("rules", "rules.json"), ("routing", "routing.json")]:
        if after[field] != decode(compiled[filename]):
            raise ValueError("compiled recipe and separate artifact differ")
    old_entries, new_entries = before["registry"]["entries"], after["registry"]["entries"]
    if new_entries[:len(old_entries)] != old_entries or after["registry"]["revision"] < before["registry"]["revision"] or after["registry"]["last_issued"] < before["registry"]["last_issued"]:
        raise ValueError("registry successor changed existing history")
    for field in ["definitions", "slots"]:
        existing = {compact(row["value"]["id"]): row for row in after["schema"][field]}
        if len(existing) != len(after["schema"][field]) or any(existing.get(compact(row["value"]["id"])) != row for row in before["schema"][field]):
            raise ValueError("successor changed existing schema declaration")
    for field in ["rules", "routing"]:
        old, new = copy.deepcopy(before[field]), copy.deepcopy(after[field])
        old.pop("definitions")
        new.pop("definitions")
        if old != new:
            raise ValueError("successor changed semantic rule/routing content")
    mapped = {compact(row["source"]): row for row in mapping["entries"]}
    if len(mapped) != len(mapping["entries"]) or any(mapped.get(compact(row["source"])) != row for row in previous_mapping["entries"]):
        raise ValueError("successor changed existing source mapping")
    normalization = decode(seed_outputs["normalization-policy-seed.json"])
    rewards = decode(seed_outputs["reward-policy-seed.json"])
    items = decode(seed_outputs["item-policy-seed.json"])
    item_source = decode(seed_outputs["item-source-policy-seed.json"])
    if normalization["gem_quality"]["value"]["definitions"] != before_identity or rewards["definitions"] != before_identity or items["definitions"] != before_identity or rewards["mapping"] != transition["before_mapping"] or item_source["item_lines"] != owned_digest("owned-item-line-policy-v1", items) or item_source["source"] != previous_mapping["source"]:
        raise ValueError("policy does not bind exact seed dependencies")
    normalization["gem_quality"]["value"]["definitions"] = after_identity
    rewards["definitions"], rewards["mapping"] = after_identity, transition["after_mapping"]
    items["definitions"] = after_identity
    item_source["item_lines"] = owned_digest("owned-item-line-policy-v1", items)
    outputs = {"policies/normalization.json": pretty(normalization), "policies/rewards.json": pretty(rewards),
               "policies/items.json": pretty(items), "policies/item-source.json": pretty(item_source)}
    receipt = {"schema_version": 1, "scope": "explicit-checked-import-policy-successor",
               "before_definitions": before_identity, "after_definitions": after_identity,
               "before_mapping": transition["before_mapping"], "after_mapping": transition["after_mapping"],
               "compiled_transition_sha256": sha(compiled["transition.json"]),
               "preserved_definition_count": len(before["schema"]["definitions"]), "preserved_slot_count": len(before["schema"]["slots"]),
               "source_execution": False, "numeric_coverage": False,
               "artifacts": [{"path": name, "sha256": sha(data), "bytes": len(data)} for name, data in sorted(outputs.items())]}
    outputs["policies/transition.json"] = pretty(receipt)
    if sum(map(len, outputs.values())) > limits.output_bytes:
        raise ValueError("bound policy output byte bound exceeded")
    return outputs


def read_compiled(directory, limits=HARD):
    names = ["transition.json", "recipe.json", "registry.json", "schema.json", "rules.json",
             "routing.json", "mapping.json", "roles.json", "manifest.json"]
    return {name: read(directory / name, limits.artifact_bytes) for name in names}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["snapshot", "base-recipe", "base-ids", "mechanics-manifest", "mechanics-facts", "reward-facts", "query-manifest", "authoring", "source-root"]:
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--compiled", type=Path, help="Explicit validated successor bundle; author checked policy copies")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--output-dir", type=Path)
    mode.add_argument("--check-dir", type=Path)
    args = parser.parse_args()
    values = [read(args.snapshot, HARD.snapshot_bytes)] + [read(getattr(args, name), HARD.artifact_bytes) for name in
        ["base_recipe", "base_ids", "mechanics_manifest", "mechanics_facts", "reward_facts", "query_manifest", "authoring"]]
    outputs = produce(*values, args.source_root)
    if args.compiled:
        outputs.update(bind_policies(outputs, read_compiled(args.compiled)))
    if args.check_dir:
        for name, data in outputs.items():
            if read(args.check_dir / name, HARD.artifact_bytes) != data:
                raise ValueError(f"persisted artifact differs: {name}")
        print(f"verified {len(outputs)} import seed artifacts; no source execution")
    else:
        args.output_dir.mkdir(parents=True, exist_ok=False)
        try:
            for name, data in outputs.items():
                (args.output_dir / name).parent.mkdir(parents=True, exist_ok=True)
                with (args.output_dir / name).open("xb") as stream:
                    stream.write(data)
        except OSError as error:
            raise ValueError(f"incomplete new output directory retained: {args.output_dir}") from error
        print(f"wrote {len(outputs)} import seed artifacts; Rust validation required")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError, RecursionError) as error:
        raise SystemExit(f"owned import export: {error}") from error
