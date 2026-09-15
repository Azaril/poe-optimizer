#!/usr/bin/env python3
"""Export finite owned allocation costs and unresolved acquired-point budgets.

Offline tooling only. Source JSON is checked by the existing strict tree exporter;
Lua is never executed. The emitted rules require the native allocation constructor.
"""
from __future__ import annotations

import argparse
from dataclasses import dataclass
import importlib.util
import json
from pathlib import Path
import re
import struct
import sys

SPEC = importlib.util.spec_from_file_location("allocation_tree_export", Path(__file__).with_name("export-owned-tree.py"))
TREE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = TREE
SPEC.loader.exec_module(TREE)
decode, read, sha, compact = TREE.decode, TREE.read, TREE.sha, TREE.compact


@dataclass(frozen=True)
class Limits:
    input_bytes: int = 32 * 1024 * 1024
    bundle_bytes: int = 64 * 1024 * 1024
    source_bytes: int = 16 * 1024 * 1024
    output_bytes: int = 4 * 1024 * 1024
    entries: int = 500000
    costs: int = 10000
    budgets: int = 3
    files: int = 32


HARD = Limits()


def fields(value, names, label):
    expected = set(names.split())
    TREE.fields(value, expected, expected, label)


def symbol(value):
    if not isinstance(value, str) or re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}", value) is None:
        raise ValueError("bounded owned symbol required")
    return value


def digest(domain, raw):
    encoded = domain.encode("ascii")
    return sha(b"poe-optimizer-owned-content-v1\0" + struct.pack("<Q", len(encoded)) + encoded + raw)


def tagged(kind, value=None):
    return {"kind": kind} if value is None else {"kind": kind, "value": value}


def canonical_key(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


class Budget:
    def __init__(self, limits):
        for name, maximum in vars(HARD).items():
            value = getattr(limits, name)
            if type(value) is not int or not 0 < value <= maximum:
                raise ValueError("invalid limit: " + name)
        self.limits = limits
        self.remaining = limits.entries

    def charge(self, count):
        self.remaining -= count
        if self.remaining < 0:
            raise ValueError("aggregate entry bound")

    def rows(self, values, maximum=None):
        if not isinstance(values, list) or len(values) > (maximum or self.limits.entries):
            raise ValueError("collection bound")
        self.charge(len(values))
        return values


def safe_path(root, name):
    if not isinstance(name, str) or not name or "\\" in name or Path(name).is_absolute():
        raise ValueError("unsafe artifact/source path")
    path = (root / name).resolve()
    if not path.is_relative_to(root.resolve()):
        raise ValueError("unsafe artifact/source path")
    return path


def source_context(source):
    fields(source, "system revision files", "source")
    if source["system"] != "path_of_building2" or not isinstance(source["revision"], str) or not re.fullmatch(r"[0-9a-f]{40}", source["revision"]):
        raise ValueError("unreviewed source context")


def pin_map(source, budget):
    source_context(source)
    pins = {}
    for row in budget.rows(source["files"], budget.limits.files):
        fields(row, "path sha256", "pin")
        if row["path"] in pins or not isinstance(row["sha256"], str) or not re.fullmatch(r"[0-9a-f]{64}", row["sha256"]):
            raise ValueError("duplicate pin or invalid hash")
        pins[row["path"]] = row["sha256"]
    return pins


def load_bundle(directory, budget):
    """Verify both manifest layers before decoding any large dependent artifact."""
    limits = budget.limits
    raw_transition = read(directory / "transition.json", min(limits.input_bytes, limits.bundle_bytes))
    transition = decode(raw_transition)
    fields(transition, "schema_version document_kind input before after preserved_registry_entries preserved_definitions preserved_slots query_sets query_rows items item_source tree schema_policy item_policy_mode source_execution calculation whole_build_parity artifacts", "tree bundle transition")
    if type(transition["schema_version"]) is not int or transition["schema_version"] != 1 or transition["document_kind"] != "owned_successor_bundle" or transition["source_execution"] is not False:
        raise ValueError("unsupported bundle transition")
    remaining = limits.bundle_bytes - len(raw_transition)
    records = {}
    kept = {}
    needed = {"registry.json", "schema.json", "mapping.json", "normalization.json", "tree-normalization.json", "manifest.json"}
    for row in budget.rows(transition["artifacts"], limits.files):
        fields(row, "file bytes sha256", "artifact")
        name = row["file"]
        if not isinstance(name, str) or Path(name).name != name or "/" in name or name in records or not name.endswith(".json") or name == "transition.json":
            raise ValueError("duplicate or unsafe artifact")
        if type(row["bytes"]) is not int or row["bytes"] < 0 or row["bytes"] > min(limits.input_bytes, remaining):
            raise ValueError("bundle byte bound")
        data = read(safe_path(directory, name), min(limits.input_bytes, remaining))
        remaining -= len(data)
        if len(data) != row["bytes"] or sha(data) != row["sha256"]:
            raise ValueError("artifact bytes/hash differ: " + name)
        records[name] = row
        if name in needed:
            kept[name[:-5]] = data
    if not needed <= records.keys():
        raise ValueError("required manifested tree artifact missing")
    package = {name: decode(data) for name, data in kept.items()}
    manifest = package["manifest"]
    fields(manifest, "schema_version document_kind recipe namespace registry definitions rules compiled_rules routing operations_version rule_semantics_version partial_rule_owners partial_route_outputs coverage calculation whole_build_parity artifacts", "recipe manifest")
    if type(manifest["schema_version"]) is not int or manifest["schema_version"] != 1 or manifest["document_kind"] != "owned_recipe_manifest":
        raise ValueError("unsupported recipe manifest")
    names = set()
    for row in budget.rows(manifest["artifacts"], limits.files):
        fields(row, "file bytes sha256", "recipe artifact")
        if row["file"] in names or records.get(row["file"]) != row:
            raise ValueError("recipe manifest binding differs")
        names.add(row["file"])
    if names != {"registry.json", "schema.json", "rules.json", "routing.json"}:
        raise ValueError("recipe artifact membership differs")
    return package, kept, transition


def typed_id(value, kind, namespace):
    fields(value, "kind namespace key", "owned definition identity")
    if value["kind"] != kind or value["namespace"] != namespace:
        raise ValueError("owned identity kind/namespace differs")
    symbol(value["key"])
    return value


def mapped(index, selector, kind, namespace):
    row = index.get(canonical_key(selector))
    if row is None:
        raise ValueError("positive exact mapping missing")
    fields(row, "kind value", "mapping outcome")
    if row["kind"] != "mapped":
        raise ValueError("positive exact mapping required")
    fields(row["value"], "target basis", "mapped outcome")
    if row["value"]["basis"] != tagged("exact"):
        raise ValueError("exact mapping basis required")
    target = row["value"]["target"]
    fields(target, "kind value", "mapping target")
    fields(target["value"], "kind value", "definition target")
    if target["kind"] != "definition" or target["value"]["kind"] != kind:
        raise ValueError("mapping domain differs")
    return typed_id(target["value"]["value"], kind, namespace)


def definition_selector(version, token):
    return tagged("definition", tagged("passive_node", {"tree_version": tagged("text", version), "node_id": tagged("text", token), "view": tagged("missing")}))


def pool_selector(version, pool):
    return tagged("catalog", {"kind": "point_pool", "key": tagged("text", pool), "version": tagged("text", version), "variant": tagged("text", "tree-pool")})


def gap(pool, code):
    return {"subject": tagged("definition", tagged("point_pool", pool)), "facet": "game_rules", "code": code}


def source_cost(node):
    # CountAllocNodes tests nil, not Lua truthiness; an explicit false is still
    # non-nil. Preserve this admitted source arithmetic in the offline transform.
    return 0 if "isFreeAllocate" in node else 1


def produce(manifest_bytes, policy_bytes, catalog_bytes, source_root, bundle_dir, limits=HARD):
    budget = Budget(limits)
    if any(len(b) > limits.input_bytes for b in [manifest_bytes, policy_bytes, catalog_bytes]):
        raise ValueError("input byte bound")
    budget.charge(3)
    policy = decode(policy_bytes)
    fields(policy, "schema_version release source cost_algorithm budget_algorithm", "allocation export policy")
    if type(policy["schema_version"]) is not int or policy["schema_version"] != 1 or policy["cost_algorithm"] != "pinned-count-alloc-nodes-v1" or policy["budget_algorithm"] != "shared-max-each-scope-and-ascendancy-v1":
        raise ValueError("unsupported allocation export algorithm")
    symbol(policy["release"])
    manifest = decode(manifest_bytes)
    if type(manifest.get("schema_version")) is not int or manifest["schema_version"] != 1:
        raise ValueError("unsupported source manifest version")
    # This independently validates all finite source fields, flags and identities.
    tree_limits = TREE.Limits(input_bytes=min(limits.input_bytes, TREE.HARD.input_bytes), total_source_bytes=min(limits.source_bytes, TREE.HARD.total_source_bytes), output_bytes=TREE.HARD.output_bytes, rows=min(limits.entries, TREE.HARD.rows), entries=min(limits.entries, TREE.HARD.entries), string_bytes=TREE.HARD.string_bytes)
    reproduced = TREE.produce(manifest_bytes, source_root, tree_limits)
    catalog = decode(catalog_bytes)
    if catalog != decode(reproduced["tree-catalog.json"]):
        raise ValueError("classified catalog differs from strict source export")
    base_pins = pin_map(manifest["source"], budget)
    review_pins = pin_map(policy["source"], budget)
    if {k:v for k,v in policy["source"].items() if k != "files"} != {k:v for k,v in manifest["source"].items() if k != "files"}:
        raise ValueError("source review context differs")
    for name, value in base_pins.items():
        if review_pins.get(name) != value:
            raise ValueError("review source pins do not preserve tree pins")
    if not {"src/Classes/PassiveSpec.lua", "src/Modules/Build.lua", "src/Data/QuestRewards.lua", "src/Modules/Calcs.lua"} <= review_pins.keys():
        raise ValueError("cost/budget source proof pins missing")
    remaining = limits.source_bytes
    source_data = {}
    for name, expected in review_pins.items():
        raw = read(safe_path(source_root, name), min(limits.input_bytes, remaining))
        remaining -= len(raw)
        normalized = raw.replace(b"\r\n", b"\n")
        if sha(normalized) != expected:
            raise ValueError("review source pin differs: " + name)
        if name == manifest["tree_file"]:
            source_data = decode(normalized)["nodes"]
    package, raws, transition = load_bundle(bundle_dir, budget)
    schema, registry, mapping, tree = (package[n] for n in ["schema", "registry", "mapping", "tree-normalization"])
    fields(schema, "schema_version namespace release semantics_version definitions slots", "schema envelope")
    fields(registry, "schema_version namespace revision last_issued entries", "registry envelope")
    fields(mapping, "schema_version namespace registry definitions source policy_version entries", "mapping envelope")
    fields(tree, "schema_version namespace registry definitions mapping normalization content", "tree policy envelope")
    namespace = schema["namespace"]
    fields(namespace, "game version", "namespace")
    symbol(namespace["game"]); symbol(namespace["version"])
    if namespace["game"] != "poe2" or (type(schema["schema_version"]) is not int or schema["schema_version"] != 2) or any(type(p["schema_version"]) is not int or p["schema_version"] != 1 for p in [registry, mapping, tree]):
        raise ValueError("unsupported schema/package version")
    definitions = {"game": namespace["game"], "release": schema["release"], "schema_version": 2, "content_sha256": sha(raws["schema"]), "semantics_version": schema["semantics_version"]}
    identities = {"registry": digest("owned-id-registry-v1", raws["registry"]), "mapping": digest("owned-external-mapping-v1", raws["mapping"]), "normalization": digest("owned-normalization-policy-v3", raws["normalization"]), "tree": digest("owned-tree-normalization-policy-v1", raws["tree-normalization"])}
    if transition["after"]["definitions"] != definitions or transition["tree"] != identities["tree"]:
        raise ValueError("transition schema/tree binding differs")
    for name in ["registry", "mapping", "normalization"]:
        if transition["after"][name] != identities[name]:
            raise ValueError("transition content binding differs")
    if package["manifest"]["definitions"] != definitions or package["manifest"]["registry"] != identities["registry"]:
        raise ValueError("manifest endpoint binding differs")
    for p in [registry, mapping, tree]:
        if p["namespace"] != namespace:
            raise ValueError("package namespace differs")
    if mapping["registry"] != identities["registry"] or mapping["definitions"] != definitions or any(tree[k] != identities[k] for k in ["registry", "mapping", "normalization"]) or tree["definitions"] != definitions:
        raise ValueError("tree/schema/mapping policy binding differs")
    content = tree["content"]
    fields(content, "version source catalog policy tree_version classes ascendancies tokens attributes syntax", "tree policy content")
    if content["tree_version"] != catalog["tree_version"] or content["source"] != catalog["source"] or content["catalog"] != digest("owned-tree-catalog-v1", compact(catalog)):
        raise ValueError("exact tree catalog binding differs")
    mapping_pins = pin_map(mapping["source"], budget)
    if any(mapping_pins.get(k) != v for k,v in base_pins.items()) or mapping["source"]["revision"] != manifest["source"]["revision"]:
        raise ValueError("mapping source footprint differs")
    active = set()
    for entry in budget.rows(registry["entries"]):
        fields(entry, "sequence target state", "registry entry")
        if entry["state"] == tagged("active"):
            address = canonical_key(entry["target"])
            if address in active:
                raise ValueError("duplicate registry identity")
            active.add(address)
    descriptors = {}
    for row in budget.rows(schema["definitions"]):
        fields(row, "kind value", "definition descriptor")
        fields(row["value"], "id schema", "definition entry")
        identity = typed_id(row["value"]["id"], row["kind"], namespace)
        address = canonical_key(identity)
        if address in descriptors:
            raise ValueError("duplicate schema identity")
        descriptors[address] = row["value"]["schema"]
    budget.rows(schema["slots"])
    def known(identity, kind):
        typed_id(identity, kind, namespace)
        if canonical_key(tagged("definition", tagged(kind, identity))) not in active:
            raise ValueError("definition not active in registry")
        state = descriptors.get(canonical_key(identity))
        fields(state, "kind value", "known definition schema")
        if state["kind"] != "known":
            raise ValueError("known schema required")
        return state["value"]
    index = {}
    for row in budget.rows(mapping["entries"]):
        fields(row, "source outcome", "mapping entry")
        address = canonical_key(row["source"])
        if address in index:
            raise ValueError("duplicate mapping selector")
        index[address] = row["outcome"]
    version = catalog["tree_version"]
    pools = {name: mapped(index, pool_selector(version, name), "point_pool", namespace) for name in ["ordinary", "ascendancy"]}
    if pools["ordinary"] == pools["ascendancy"]:
        raise ValueError("point pool identities collide")
    for name, scope in [("ordinary", "either"), ("ascendancy", "shared")]:
        ps = known(pools[name], "point_pool")
        fields(ps, "scope", "point pool schema")
        if ps["scope"] != scope:
            raise ValueError("point pool scope differs")
    roles = {}
    for row in budget.rows(content["tokens"]):
        fields(row, "token role", "tree role")
        if row["token"] in roles:
            raise ValueError("duplicate tree token role")
        roles[row["token"]] = row["role"]
    if set(roles) != {n["key"] for n in catalog["nodes"]}:
        raise ValueError("tree role membership differs")
    costs, zero_cost_tokens, excluded = [], [], {"implicit_root": 0, "attached_choice": 0, "unsupported": 0}
    seen = set()
    for node in budget.rows(catalog["nodes"]):
        token, category = node["key"], node["kind"]["kind"]
        role = roles[token]
        expected = "allocation" if category in ["allocation", "attribute"] else "unresolved" if category == "unsupported" else category
        if role.get("kind") != expected:
            raise ValueError("source classification/tree role differs")
        fields(role, "kind value", "tree role state")
        role = role["value"]
        if expected == "unresolved":
            fields(role, "code", "unresolved role"); excluded[category] += 1; continue
        if expected == "attached_choice":
            fields(role, "parent slot option", "attached choice role")
            parent = mapped(index, definition_selector(version, node["kind"]["value"]["parent"]), "passive_node", namespace)
            if role["parent"] != parent:
                raise ValueError("attached choice parent differs")
            fields(role["slot"], "declaration slot", "attached slot")
            fields(role["slot"]["declaration"], "kind definition", "slot owner")
            if role["slot"]["declaration"] != {"kind": "passive_node", "definition": parent}:
                raise ValueError("attached slot declaration differs")
            typed_id(role["slot"]["slot"], "choice_slot", namespace)
            known(role["option"], "option")
            if canonical_key(tagged("slot", tagged("choice", role["slot"]))) not in active:
                raise ValueError("attached slot not active in registry")
            excluded[category] += 1; continue
        fields(role, "node pool" if expected == "allocation" else "node", "physical role")
        identity = mapped(index, definition_selector(version, token), "passive_node", namespace)
        if role["node"] != identity or canonical_key(identity) in seen:
            raise ValueError("physical role identity differs or duplicates")
        seen.add(canonical_key(identity))
        ns = known(identity, "passive_node")
        fields(ns, "pools adjacent declarations", "passive node schema")
        fields(ns["pools"], "members closure", "passive pool membership")
        if ns["pools"]["closure"] != tagged("complete"):
            raise ValueError("complete exact point pool membership required")
        if expected == "implicit_root":
            if ns["pools"]["members"]:
                raise ValueError("implicit root has paid pool")
            excluded[category] += 1; continue
        pool = pools[node["kind"]["value"]["pool"]]
        if role["pool"] != pool or ns["pools"]["members"] != [pool]:
            raise ValueError("allocation point pool differs")
        if len(costs) >= limits.costs:
            raise ValueError("cost row bound")
        budget.charge(1)
        points = source_cost(source_data[token])
        if points == 0: zero_cost_tokens.append(token)
        costs.append({"node": identity, "pool": pool, "points": points})
    if limits.budgets < 3:
        raise ValueError("budget row bound")
    budgets = []
    for name, pool_name, usage in [("ordinary-total", "ordinary", tagged("shared_plus_maximum_scoped")), ("ordinary-each-loadout", "ordinary", {"kind": "each_scope", "include_shared": False}), ("ascendancy-total", "ascendancy", tagged("total"))]:
        pool = pools[pool_name]
        budget.charge(3)
        budgets.append({"id": name, "pools": [pool], "usage": usage, "capacity": tagged("unmapped", {"gaps": [gap(pool, "acquired-capacity-not-converted")]})})
    costs.sort(key=lambda row: (row["node"]["key"], row["pool"]["key"]))
    rules = {"schema_version": 1, "namespace": namespace, "release": policy["release"], "definitions": definitions, "costs": costs, "budgets": {"members": budgets, "closure": tagged("partial", {"gaps": [gap(pools[name], "additional-point-budget-constraints-not-converted") for name in ["ordinary", "ascendancy"]]})}}
    outputs = {"rules.json": TREE.bounded_pretty(rules, limits.output_bytes)}
    facts = {"schema_version": 1, "scope": "owned-allocation-cost-and-budget-component", "source": policy["source"], "source_manifest_sha256": sha(manifest_bytes), "export_policy_sha256": sha(policy_bytes), "catalog_sha256": sha(catalog_bytes), "bundle_transition_sha256": sha(read(bundle_dir / "transition.json", limits.input_bytes)), "bindings": {**identities, "definitions": definitions}, "algorithms": {"cost": policy["cost_algorithm"], "budget": policy["budget_algorithm"]}, "cost_rows": len(costs), "zero_cost_source_tokens": zero_cost_tokens, "excluded_source_rows": excluded, "budget_rows": len(budgets), "capacity": "acquired_progression_unmapped", "budget_membership": "partial_additional_point_constraints_not_converted", "access": "outside_this_artifact", "source_execution": False, "legality_established": False, "artifact": {"file": "rules.json", "bytes": len(outputs["rules.json"]), "sha256": sha(outputs["rules.json"])}}
    outputs["source-facts.json"] = TREE.bounded_pretty(facts, limits.output_bytes - len(outputs["rules.json"]))
    return outputs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["manifest", "policy", "catalog", "source-root", "bundle"]:
        parser.add_argument("--" + name, type=Path, required=True)
    dest = parser.add_mutually_exclusive_group(required=True)
    dest.add_argument("--output-dir", type=Path); dest.add_argument("--check-dir", type=Path)
    args = parser.parse_args()
    outputs = produce(read(args.manifest, HARD.input_bytes), read(args.policy, HARD.input_bytes), read(args.catalog, HARD.input_bytes), args.source_root, args.bundle)
    if args.check_dir:
        for name, data in outputs.items():
            if read(args.check_dir / name, HARD.output_bytes) != data:
                raise ValueError("persisted allocation artifact differs: " + name)
        print("verified owned allocation costs and unresolved capacities; no source execution")
    else:
        args.output_dir.mkdir(parents=True, exist_ok=False)
        for name, data in outputs.items():
            with (args.output_dir / name).open("xb") as stream:
                stream.write(data)
        print("wrote allocation inputs; native validation required; acquired capacities unresolved")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError, RecursionError) as error:
        raise SystemExit("owned allocation export: " + str(error)) from error
