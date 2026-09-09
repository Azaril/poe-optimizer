#!/usr/bin/env python3
r"""Record independent import/evaluation observations for caller-supplied PoB lines.

Example (all paths are caller choices):
  python intake-build-corpus.py --input imports.txt --output new-results \
    --import-cli path/to/poe-optimizer --backend native=path/to/poe-optimizer \
    --deadline-seconds 600 --jobs 2

Add --backend pob=path/to/reference-cli --pob path/to/checkout for reference
observations. No build, dataset or executable is selected by this script.
"""
from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import math
import os
import re
from pathlib import Path
import signal
import subprocess
import sys
import time
import xml.etree.ElementTree as ET
import xml.parsers.expat as EXPAT

MAX_INPUT_BYTES = 64 * 1024 * 1024
MAX_ENTRY_BYTES = 16 * 1024 * 1024
MAX_JSON_BYTES = 64 * 1024 * 1024
MAX_XML_BYTES = 32 * 1024 * 1024
MAX_LINES = 10000
MAX_DATA_BYTES = 16 * 1024 * 1024
MAX_OPTIONS_BYTES = 1024 * 1024
MAX_LOADING_TEXT_BYTES = 32 * 1024 * 1024


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


def read_bounded(path: Path, limit: int) -> bytes:
    with path.open("rb") as stream:
        value = stream.read(limit + 1)
    if len(value) > limit:
        raise ValueError(f"{path} exceeds the {limit}-byte input bound")
    return value


def write_json(path: Path, value: object) -> None:
    with path.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(value, stream, indent=2, ensure_ascii=False, allow_nan=False)
        stream.write("\n")


def line_records(data: bytes) -> list[tuple[int, int, bytes]]:
    """Only LF separates entries; enforce the line bound before allocating rows."""
    result = []
    offset = 0
    while offset < len(data):
        if len(result) == MAX_LINES:
            raise ValueError(f"corpus exceeds the {MAX_LINES:,}-line bound")
        newline = data.find(b"\n", offset)
        end = len(data) if newline == -1 else newline + 1
        result.append((len(result) + 1, offset, data[offset:end]))
        offset = end
    return result


def bounded_json(path: Path) -> dict:
    if path.stat().st_size > MAX_JSON_BYTES:
        raise ValueError("JSON output exceeds observation size bound; raw bytes retained")
    def reject_constant(value):
        raise ValueError(f"nonstandard JSON constant {value}")
    def finite_float(value):
        number = float(value)
        if not math.isfinite(number):
            raise ValueError(f"JSON number exceeds finite observation range: {value}")
        return number
    def unique_pairs(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate JSON key {key!r}")
            result[key] = value
        return result
    value = json.loads(path.read_bytes(), parse_constant=reject_constant,
                       parse_float=finite_float, object_pairs_hook=unique_pairs)
    if not isinstance(value, dict):
        raise ValueError("expected a JSON object")
    return value



def configuration_source_summary(report: dict, expected_hash: str, expected_bytes: int) -> dict:
    """Validate the source-report contract before interpreting its counts/selection.

    This checks transport/schema consistency, never game settings or mechanical
    correctness. Full source evidence remains in the invocation stdout artifact.
    """
    def object_value(value, label):
        if not isinstance(value, dict):
            raise ValueError(f"{label} must be an object")
        return value

    def positive_id(value, label):
        if type(value) is not int or not 1 <= value <= 0xffffffff:
            raise ValueError(f"{label} must be a positive u32 integer")
        return value

    if (type(report.get("schema_version")) is not int or report["schema_version"] != 1
            or report.get("scope") != "configuration_source_projection_v1"
            or report.get("status") != "source_projected"):
        raise ValueError("unsupported configuration report schema or scope")
    source = object_value(report["input"], "configuration input")
    if (source.get("format") != "raw_xml" or source.get("input_sha256") != expected_hash
            or source.get("xml_sha256") != expected_hash):
        raise ValueError("configuration source hash/format differs from imported XML")
    for field in ["input_bytes", "xml_bytes"]:
        if type(source.get(field)) is not int or source[field] != expected_bytes:
            raise ValueError(f"configuration {field} differs from imported XML")
    diagnostic = object_value(report["configuration"], "configuration diagnostic")
    if (type(diagnostic.get("schema_version")) is not int or diagnostic["schema_version"] != 1
            or diagnostic.get("interpretation") != "authored_configuration_source"):
        raise ValueError("unsupported configuration diagnostic schema or interpretation")
    for field in ["effective_configuration", "mechanics", "game_legality"]:
        if diagnostic.get(field) != "not_evaluated":
            raise ValueError(f"configuration diagnostic must not claim evaluated {field}")
    verification = object_value(report["verification"], "configuration verification")
    for field, expected in {"effective_configuration": "not_evaluated",
                            "game_mechanics": "not_evaluated", "build_legality": "not_checked",
                            "reference_calculation": "not_run"}.items():
        if verification.get(field) != expected:
            raise ValueError(f"configuration verification must declare {field}={expected}")
    observed = object_value(diagnostic["projection"], "configuration projection")
    if observed.get("source_sha256") != expected_hash:
        raise ValueError("configuration projection hash differs from imported XML")
    layout = observed["layout"]
    if layout not in {"missing", "empty", "legacy", "explicit_sets"}:
        raise ValueError("unknown configuration layout")
    sets = observed["sets"]
    if not isinstance(sets, list) or not 1 <= len(sets) <= 64:
        raise ValueError("configuration sets must be an array of 1..64 sets")
    summaries, ids, record_count = [], [], 0
    for node in sets:
        node = object_value(node, "configuration set")
        set_id = positive_id(node["id"], "configuration set ID")
        if set_id in ids:
            raise ValueError("duplicate configuration set ID")
        ids.append(set_id)
        summary = {"id": set_id}
        for field, summary_field in [("inputs", "inputs"), ("placeholders", "placeholders"),
                                     ("blocks", "custom_modifier_blocks"),
                                     ("unknown_records", "unhandled_records")]:
            records = node[field]
            if not isinstance(records, list) or any(not isinstance(r, dict) for r in records):
                raise ValueError(f"configuration {field} must be an array of record objects")
            record_count += len(records)
            summary[summary_field] = len(records)
        summaries.append(summary)
    if record_count > 4096:
        raise ValueError("configuration exceeds the 4096-record limit")
    if layout in {"missing", "empty", "legacy"} and ids != [1]:
        raise ValueError("implicit configuration layouts require exactly set 1")
    if layout in {"missing", "empty"} and record_count:
        raise ValueError("missing/empty configuration cannot contain authored records")
    requested = observed["requested_active_set_id"]
    if requested is not None:
        positive_id(requested, "requested configuration set ID")
    requested_or_default = 1 if requested is None else requested
    present = requested_or_default in ids
    active = positive_id(observed["active_set_id"], "active configuration set ID")
    expected_active = requested_or_default if present else ids[0]
    expected_resolution = (("requested" if requested is not None else "default_one") if present
                           else ("missing_requested_uses_first" if requested is not None
                                 else "missing_default_uses_first"))
    if active != expected_active or observed["active_set_resolution"] != expected_resolution:
        raise ValueError("configuration active selection/fallback is inconsistent with source sets")
    return {"report_schema_version": report["schema_version"],
            "source_xml_sha256": expected_hash,
            "source_state": {"layout": layout, "requested_active_set_id": requested,
                             "active_set_id": active, "active_set_resolution": expected_resolution,
                             "sets": summaries},
            "verification": verification}



_XML_SPACE = " \t\r\n\v\f"
_NAMED_ENTITIES = {"lt": "<", "gt": ">", "amp": "&", "apos": "'", "quot": '"'}
_SOURCE_TAG = re.compile(rb"<([^\s/>]+)")
_SOURCE_SPACE = re.compile(rb"\s*")
_SOURCE_ATTRIBUTE = re.compile(rb"""([^\s=/>]+)\s*=\s*(["'])(.*?)\2""", re.DOTALL)


def source_named_entities(raw: str) -> str:
    """Exactly one XML.lua named-entity pass; unsupported forms fail closed."""
    result, at = [], 0
    while True:
        start = raw.find("&", at)
        if start < 0:
            result.append(raw[at:])
            return "".join(result)
        result.append(raw[at:start])
        end = raw.find(";", start + 1)
        if end < 0 or raw[start + 1:end] not in _NAMED_ENTITIES:
            raise ValueError("unsupported source entity in item evidence")
        result.append(_NAMED_ENTITIES[raw[start + 1:end]])
        at = end + 1


def source_element_index(xml: bytes) -> dict:
    """Index exact source spans with a bounded XML parser, never item semantics.

    Expat supplies structural offsets only. Attribute evidence is sliced from
    caller bytes so standard XML whitespace normalization cannot replace it.
    """
    if type(xml) is not bytes or len(xml) > MAX_XML_BYTES:
        raise ValueError("source XML bytes exceed inspection binding limit")
    parser = EXPAT.ParserCreate()
    parser.ordered_attributes = True
    parser.SetParamEntityParsing(EXPAT.XML_PARAM_ENTITY_PARSING_NEVER)
    result, stack = {}, []

    def reject_declaration(*_):
        raise ValueError("DTD/entity declarations are not source inspection evidence")
    parser.StartDoctypeDeclHandler = reject_declaration
    parser.EntityDeclHandler = reject_declaration
    parser.ExternalEntityRefHandler = lambda *_: 0

    def start(name, normalized):
        if len(result) >= 100000:
            raise ValueError("source XML node bound exceeded")
        at = parser.CurrentByteIndex
        cursor, quote = at + 1, None
        while cursor < len(xml):
            byte = xml[cursor]
            if quote is not None:
                if byte == quote:
                    quote = None
            elif byte in (34, 39):
                quote = byte
            elif byte == 62:
                break
            cursor += 1
        if cursor == len(xml):
            raise ValueError("unterminated source opening tag")
        opening_end = cursor + 1
        prefix = _SOURCE_TAG.match(xml, at, opening_end)
        if prefix is None:
            raise ValueError("missing source element name")
        attributes, position = [], prefix.end()
        while position < cursor:
            whitespace = _SOURCE_SPACE.match(xml, position, cursor)
            position = whitespace.end()
            if position == cursor or xml[position:position + 2] == b"/>":
                break
            match = _SOURCE_ATTRIBUTE.match(xml, position, cursor)
            if match is None:
                raise ValueError("invalid source attribute token")
            key = match[1].decode("utf-8")
            value_start = match.start(3)
            raw = match[3].decode("utf-8")
            attributes.append((key, value_start, value_start + len(match[3]), raw))
            position = match.end()
        values = dict(zip(normalized[::2], normalized[1::2]))
        namespaces = stack[-1]["namespaces"] if stack else {}
        if any(key == "xmlns" or key.startswith("xmlns:") for key, _, _, _ in attributes):
            namespaces = dict(namespaces)
        for key, _, _, _ in attributes:
            if key == "xmlns":
                namespaces[""] = values[key]
            elif key.startswith("xmlns:") and key != "xmlns:xml":
                namespaces[key[6:]] = values[key]
        if len(namespaces) > 1024:
            raise ValueError("source namespace bound exceeded")
        def expanded(key, attribute=False):
            if ":" in key:
                prefix, local = key.split(":", 1)
                uri = ("http://www.w3.org/XML/1998/namespace" if prefix == "xml"
                       else namespaces.get(prefix))
                if uri is None:
                    raise ValueError("unbound source namespace")
                return local, uri or None
            return key, None if attribute else namespaces.get("") or None
        local, namespace = expanded(name)
        source_attributes = []
        for key, begin, end, raw in attributes:
            if key == "xmlns" or key.startswith("xmlns:"):
                continue
            attr_name, attr_namespace = expanded(key, True)
            source_attributes.append({"name": attr_name, "namespace": attr_namespace,
                "value": {"range": {"start": begin, "end": end}, "raw": raw,
                          "decoded": source_named_entities(raw)}})
        item = {"start": at, "opening_end": opening_end, "empty": xml[cursor - 1:cursor] == b"/",
                "name": local, "namespace": namespace, "has_namespaces": bool(namespaces),
                "namespaces": namespaces, "attributes": source_attributes, "children": []}
        result[at] = item
        if stack:
            stack[-1]["children"].append(item)
        stack.append(item)

    def end(_):
        item = stack.pop()
        item.pop("namespaces")  # only open ancestors need namespace lookup state
        if item["empty"]:
            item["closing_start"] = item["opening_end"]
            item["end"] = item["opening_end"]
        else:
            item["closing_start"] = parser.CurrentByteIndex
            finish = xml.find(b">", item["closing_start"])
            if finish < 0:
                raise ValueError("unterminated source closing tag")
            item["end"] = finish + 1
    parser.StartElementHandler = start
    parser.EndElementHandler = end
    try:
        parser.Parse(xml, True)
    except EXPAT.ExpatError as error:
        raise ValueError(f"invalid source XML while binding inspection: {error}") from error
    return result


def item_source_classification(parent, name, namespace):
    """Source-use classification, with no slot/base/modifier interpretation."""
    names = {"Items": "items", "Item": "item", "ItemSet": "item_set", "Slot": "slot",
             "RuneSlot": "rune_slot", "SocketIdURL": "socket_id_url", "ModRange": "mod_range",
             "TradeSearchWeights": "trade_search_weights", "Stat": "stat", "Tree": "tree",
             "Spec": "spec", "Sockets": "sockets", "Socket": "socket"}
    if namespace:
        return "unknown", "namespace_unknown"
    roles = {(None, "Items"): "container", (None, "Tree"): "tree", (None, "Spec"): "passive_spec",
             ("container", "Item"): "inventory_item", ("container", "ItemSet"): "saved_set",
             ("container", "Slot"): "legacy_equipment_slot",
             ("container", "TradeSearchWeights"): "trade_weights",
             ("saved_set", "Slot"): "equipment_slot", ("saved_set", "RuneSlot"): "character_rune_slot",
             ("saved_set", "SocketIdURL"): "socket_url_metadata",
             ("inventory_item", "ModRange"): "modifier_range_instruction",
             ("tree", "Spec"): "passive_spec", ("passive_spec", "Sockets"): "jewel_sockets",
             ("jewel_sockets", "Socket"): "jewel_assignment"}
    return names.get(name, "unknown"), ("trade_weight" if parent == "trade_weights" else
                                      roles.get((parent, name), "ignored"))


def derive_item_content(fragments, expected_xml=None):
    """Reconstruct only XML.lua text/element records, not ItemsTab or Item loading."""
    consumed, pending, indices = [], [], []
    def flush():
        text = "".join(pending).strip(_XML_SPACE)
        if text:
            consumed.append({"kind": "text", "text_kind": "ordinary",
                             "text": source_named_entities(text), "fragment_indices": list(indices)})
        pending.clear()
        indices.clear()
    for index, fragment in enumerate(fragments):
        kind = fragment["kind"]
        raw = fragment.get("text_source")
        if kind == "text":
            if index and fragments[index - 1]["kind"] == "text":
                raise ValueError("adjacent ordinary source text must be one raw fragment")
            if "<" in raw:
                raise ValueError("ordinary item text contains an unclassified XML construct")
            pending.append(raw)
            indices.append(index)
        elif kind == "comment":
            if not raw.startswith("<!--") or not raw.endswith("-->") or "--" in raw[4:-3]:
                raise ValueError("invalid comment fragment")
            indices.append(index)
        elif kind == "cdata":
            flush()
            if not raw.startswith("<![CDATA[") or not raw.endswith("]]>") or "]]>" in raw[9:-3]:
                raise ValueError("invalid CDATA fragment")
            if expected_xml is not None:
                begin, end = fragment["range"]["start"] + 9, fragment["range"]["end"] - 3
                position = begin
                while True:
                    comment = expected_xml.find(b"<!--", position, end)
                    if comment < 0:
                        break
                    close = expected_xml.find(b"-->", comment + 4)
                    if close < 0:
                        break  # Lua's global pattern leaves an unmatched opener literal.
                    if close + 3 > end:
                        raise ValueError("global comment removal crosses a CDATA boundary")
                    position = close + 3
            text = re.sub(r"<!--.*?-->", "", raw[9:-3], flags=re.DOTALL)
            if "]]>" in text:
                raise ValueError("comment removal changes CDATA structure")
            if text.strip(_XML_SPACE):
                consumed.append({"kind": "text", "text_kind": "cdata", "text": text,
                                 "fragment_indices": [index]})
        elif kind == "processing_instruction":
            flush()
            if not raw.startswith("<?") or not raw.endswith("?>") or ">" in raw[2:-2]:
                raise ValueError("invalid processing instruction fragment")
        elif kind == "element":
            flush()
            consumed.append({"kind": "element", "child_index": fragment["child_index"]})
        else:
            raise ValueError("unknown item fragment kind")
    flush()
    return consumed

def build_source_summary(report: dict, expected_hash: str, expected_bytes: int,
                         definitions_requested: bool, expected_data_hash: str | None,
                         expected_xml: bytes | None = None) -> dict:
    """Validate source/lookup transport and summarize occurrences, not game support.

    Original Rust/Lua tests establish reader semantics. This check binds captured
    reports to this invocation and prevents a lost/duplicated instance or a new
    calculation claim from becoming a successful corpus observation.
    """
    def mapping(value, label):
        if not isinstance(value, dict):
            raise ValueError(f"{label} must be an object")
        return value

    def array(value, limit, label):
        if not isinstance(value, list) or len(value) > limit:
            raise ValueError(f"{label} must be a bounded array")
        return value

    def number(value, lower, upper, label):
        if type(value) is not int or not lower <= value <= upper:
            raise ValueError(f"{label} must be a bounded integer")
        return value

    def source_range(value):
        value = mapping(value, "source range")
        start = number(value["start"], 0, expected_bytes, "range start")
        end = number(value["end"], start + 1, expected_bytes, "range end")
        return start, end

    def label(value):
        if not isinstance(value, str) or not value or len(value) > 128:
            raise ValueError("invalid source/identity classification")
        return value

    def require(values, expected, context):
        mapping(values, context)
        for key, value in expected.items():
            if type(values.get(key)) is not type(value) or values[key] != value:
                raise ValueError(f"{context} must declare {key}={value}")

    source_index = None
    if expected_xml is not None:
        if (type(expected_xml) is not bytes or len(expected_xml) != expected_bytes
                or hashlib.sha256(expected_xml).hexdigest() != expected_hash):
            raise ValueError("supplied source XML bytes differ from the import identity")
        source_index = source_element_index(expected_xml)

    version = report.get("schema_version")
    if type(version) is not int or version not in {1, 2, 3}:
        raise ValueError("unsupported build report schema")
    require(report, {"schema_version": version, "scope": f"build_source_projection_v{version}",
                     "status": "source_projected"}, "build report")
    require(report["input"], {"format": "raw_xml", "input_sha256": expected_hash,
                              "xml_sha256": expected_hash, "input_bytes": expected_bytes,
                              "xml_bytes": expected_bytes}, "build input")
    verification = report["verification"]
    require(verification, {"calculation_context": "not_resolved",
                           "effective_configuration": "not_evaluated", "game_mechanics": "not_evaluated",
                           "build_legality": "not_checked", "native_admission": "not_checked",
                           "reference_calculation": "not_run"}, "build verification")
    build = mapping(report["build"], "build projection")
    require(build, {"source_sha256": expected_hash}, "build projection")
    root_range = source_range(build["root"]["source_range"])
    section_counts, skill_container_ranges, item_container_ranges, item_tree_ranges = {}, [], [], []
    previous_section = root_range[0]
    for section in array(build["sections"], 128, "build sections"):
        bounds = source_range(section["element"]["source_range"])
        if not root_range[0] < bounds[0] < bounds[1] < root_range[1]:
            raise ValueError("section source range is outside root")
        if bounds[0] < previous_section:
            raise ValueError("build sections overlap or lost source order")
        previous_section = bounds[1]
        if section["element"].get("name") == "Skills":
            skill_container_ranges.append(bounds)
        if section["element"].get("name") == "Items":
            item_container_ranges.append(bounds)
        if section["element"].get("name") in {"Tree", "Spec"}:
            item_tree_ranges.append(bounds)
        kind = label(section["kind"])
        section_counts[kind] = section_counts.get(kind, 0) + 1
    result = {"report_schema_version": version, "source_xml_sha256": expected_hash,
              "sections": section_counts, "verification": verification,
              "implementation_sha256": report.get("implementation_sha256")}
    if version == 1:
        if "items" in report:
            raise ValueError("schema-1 inspection cannot claim schema-2 item evidence")
        result["item_source_status"] = "not_reported_by_inspector"
    else:
        require(verification, {"equipment_resolution": "not_resolved",
                               "passive_allocation": "not_checked"}, "item verification")
        if version < 3 or not definitions_requested:
            require(verification, {"item_loading": "not_run"}, "item verification")
        items = mapping(report["items"], "item source result")
        result["item_source_status"] = items["status"]
        if items["status"] == "not_projected":
            mapping(items["error"], "item projection error")
        elif items["status"] != "source_projected":
            raise ValueError("unknown item projection outcome")
        else:
            item_projection = mapping(items["projection"], "item projection")
            require(item_projection, {"source_sha256": expected_hash}, "item projection")
            item_counts, fragment_counts = {}, {}
            inventories, saved_sets, jewels = [], [], []
            loading_sources, inventory_ordinals = [], {}
            item_nodes_left, fragments_left = 32768, 131072
            item_attributes_left, item_attribute_bytes_left, item_text_left = 65536, 2 * 1024 * 1024, 8 * 1024 * 1024
            roles = {"container", "inventory_item", "saved_set", "equipment_slot",
                     "legacy_equipment_slot", "character_rune_slot", "socket_url_metadata",
                     "modifier_range_instruction", "trade_weights", "trade_weight", "tree",
                     "passive_spec", "jewel_sockets", "jewel_assignment", "ignored", "namespace_unknown"}
            def source_value(node, name):
                values = [a["value"] for a in array(node["element"]["attributes"], 128, "item attributes")
                          if a["name"] == name and a.get("namespace") is None]
                if len(values) > 1:
                    raise ValueError("duplicate source attribute in item report")
                return values[0] if values else None
            def validate_item_element(element, bounds):
                nonlocal item_attributes_left, item_attribute_bytes_left
                element = mapping(element, "item source element")
                name, namespace = element["name"], element.get("namespace")
                if not isinstance(name, str) or not name or len(name.encode("utf-8")) > 1024:
                    raise ValueError("invalid item source element name")
                if namespace is not None and not isinstance(namespace, str):
                    raise ValueError("invalid item source element namespace")
                has_namespaces = element.get("has_namespaces")
                if type(has_namespaces) is not bool:
                    raise ValueError("item namespace context must be a boolean")
                attributes = array(element["attributes"], 128, "item source attributes")
                item_attributes_left -= len(attributes)
                item_attribute_bytes_left -= len(name.encode("utf-8")) + len((namespace or "").encode("utf-8"))
                seen_attributes = set()
                for attr in attributes:
                    attr = mapping(attr, "item source attribute")
                    key, uri = attr["name"], attr.get("namespace")
                    if not isinstance(key, str) or not key or len(key.encode("utf-8")) > 1024:
                        raise ValueError("invalid item attribute name")
                    if uri is not None and not isinstance(uri, str):
                        raise ValueError("invalid item attribute namespace")
                    if (key, uri) in seen_attributes:
                        raise ValueError("duplicate item attribute occurrence")
                    seen_attributes.add((key, uri))
                    value = mapping(attr["value"], "item SourceText")
                    raw, decoded = value["raw"], value["decoded"]
                    if not isinstance(raw, str) or not isinstance(decoded, str):
                        raise ValueError("item SourceText requires raw and decoded strings")
                    span = mapping(value["range"], "item attribute range")
                    begin = number(span["start"], bounds[0] + 1, bounds[1] - 1, "attribute start")
                    end = number(span["end"], begin, bounds[1] - 1, "attribute end")
                    encoded = raw.encode("utf-8")
                    if len(encoded) != end - begin or len(encoded) > 65536:
                        raise ValueError("item attribute raw bytes differ from its range or bound")
                    if source_named_entities(raw) != decoded:
                        raise ValueError("item attribute decoding differs from source named entities")
                    if expected_xml is not None and expected_xml[begin:end] != encoded:
                        raise ValueError("item attribute raw bytes differ from imported XML")
                    item_attribute_bytes_left -= len(encoded) + len(key.encode("utf-8")) + len((uri or "").encode("utf-8"))
                if item_attributes_left < 0 or item_attribute_bytes_left < 0:
                    raise ValueError("item aggregate attribute bound exceeded")
                actual = source_index.get(bounds[0]) if source_index is not None else None
                if source_index is not None:
                    if actual is None or actual["end"] != bounds[1]:
                        raise ValueError("item source element range is not an exact imported occurrence")
                    if (name != actual["name"] or namespace != actual["namespace"]
                            or has_namespaces != actual["has_namespaces"]
                            or attributes != actual["attributes"]):
                        raise ValueError("item source element/attributes differ from imported XML")
                return actual

            def visit_item(node, collection, owner_index, parent_bounds, spec, sockets, depth,
                           parent_role=None, inherited_namespace=False):
                nonlocal item_nodes_left, fragments_left, item_text_left
                item_nodes_left -= 1
                if item_nodes_left < 0 or depth > 32:
                    raise ValueError("item source node/depth bound exceeded")
                bounds = source_range(node["element"]["source_range"])
                if not parent_bounds[0] < bounds[0] < bounds[1] < parent_bounds[1]:
                    raise ValueError("item source range is outside its parent")
                actual = validate_item_element(node["element"], bounds)
                namespace = inherited_namespace or node["element"]["has_namespaces"] or node["element"].get("namespace") is not None
                expected_kind, expected_role = item_source_classification(parent_role, node["element"]["name"], namespace)
                role = node["source_use"]
                if role not in roles or role != expected_role or node.get("kind") != expected_kind:
                    raise ValueError("item source role/name/parent/namespace classification differs")
                item_counts[role] = item_counts.get(role, 0) + 1
                if role == "passive_spec":
                    spec, sockets = bounds, None
                if role == "jewel_sockets":
                    sockets = bounds
                children = array(node["children"], 32768, "item children")
                child_ranges = [source_range(child["element"]["source_range"]) for child in children]
                if actual is not None and child_ranges != [(c["start"], c["end"]) for c in actual["children"]]:
                    raise ValueError("item children differ from complete imported source occurrences")
                previous = bounds[0]
                for child_bounds in child_ranges:
                    if not bounds[0] < child_bounds[0] < child_bounds[1] < bounds[1] or child_bounds[0] < previous:
                        raise ValueError("item children overlap, changed order or escaped their owner")
                    previous = child_bounds[1]
                content = mapping(node["ordered_content"], "ordered item content")
                fragments = array(content["fragments"], 131072, "item fragments")
                fragments_left -= len(fragments)
                if fragments_left < 0:
                    raise ValueError("item fragment bound exceeded")
                element_fragments = {}
                previous = None
                for index, fragment in enumerate(fragments):
                    span = source_range(fragment["range"])
                    if not bounds[0] < span[0] < span[1] < bounds[1] or (previous is not None and span[0] != previous):
                        raise ValueError("item fragments overlap, have gaps or escaped their owner")
                    previous = span[1]
                    kind = fragment["kind"]
                    if kind not in {"element", "text", "cdata", "comment", "processing_instruction"}:
                        raise ValueError("unknown item fragment kind")
                    fragment_counts[kind] = fragment_counts.get(kind, 0) + 1
                    if kind == "element":
                        child = number(fragment["child_index"], 0, len(children) - 1, "fragment child index")
                        if child in element_fragments or span != child_ranges[child] or "text_source" in fragment:
                            raise ValueError("item element fragment changed its unique child reference")
                        element_fragments[child] = index
                    elif ("child_index" in fragment or not isinstance(fragment.get("text_source"), str)
                          or len(fragment["text_source"].encode("utf-8")) != span[1] - span[0]):
                        raise ValueError("item raw text fragment differs from its source byte range")
                    if expected_xml is not None and kind != "element" and expected_xml[span[0]:span[1]] != fragment["text_source"].encode("utf-8"):
                        raise ValueError("item raw fragment differs from exact imported XML bytes")
                if actual is not None:
                    body = (actual["opening_end"], actual["closing_start"])
                    if (not fragments and body[0] != body[1]) or (fragments and
                            (source_range(fragments[0]["range"])[0] != body[0] or
                             source_range(fragments[-1]["range"])[1] != body[1])):
                        raise ValueError("item fragments do not cover the complete source body")
                if list(element_fragments) != list(range(len(children))):
                    raise ValueError("item fragments omitted or reordered a child")
                consumed_children, used_text, instructions = [], set(), []
                loading_instructions = []
                previous_entry = -1
                for entry_index, entry in enumerate(array(content["consumed"], 131072, "item consumed records")):
                    if entry["kind"] == "element":
                        child = number(entry["child_index"], 0, len(children) - 1, "consumed child index")
                        position = element_fragments[child]
                        if position <= previous_entry:
                            raise ValueError("consumed item child is duplicated or out of source order")
                        previous_entry = position
                        consumed_children.append(child)
                        if role == "inventory_item" and children[child]["source_use"] == "modifier_range_instruction":
                            instructions.append({"kind": "modifier_range", "consumed_index": entry_index,
                                                 "child_index": child, "source_range": list(child_ranges[child])})
                        if role == "inventory_item":
                            loading_instructions.append({"kind": "mod_range" if children[child]["source_use"] == "modifier_range_instruction" else "ignored",
                                "consumed_index": entry_index, "source_range": list(child_ranges[child]), "text_sha256": None})
                    elif entry["kind"] == "text":
                        indices = array(entry["fragment_indices"], 131072, "text fragment references")
                        if not indices or not isinstance(entry["text"], str) or not entry["text"]:
                            raise ValueError("consumed item text requires source fragments and text")
                        text_kind = entry["text_kind"]
                        if text_kind not in {"ordinary", "cdata"} or (text_kind == "cdata" and len(indices) != 1):
                            raise ValueError("unknown or fragmented CDATA text record")
                        raw_text_present = False
                        for index in indices:
                            index = number(index, 0, len(fragments) - 1, "text fragment index")
                            kind = fragments[index]["kind"]
                            if (index <= previous_entry or index in used_text
                                    or kind not in ({"text", "comment"} if text_kind == "ordinary" else {"cdata"})):
                                raise ValueError("consumed item text changed source order, kind or ownership")
                            raw_text_present |= kind in {"text", "cdata"}
                            previous_entry = index
                            used_text.add(index)
                        if not raw_text_present:
                            raise ValueError("comment-only fragments cannot supply consumed item text")
                        if role == "inventory_item":
                            text_hash = hashlib.sha256(entry["text"].encode("utf-8")).hexdigest()
                            instructions.append({"kind": "text", "consumed_index": entry_index,
                                                 "text_kind": text_kind, "text_sha256": text_hash})
                            loading_instructions.append({"kind": "text", "consumed_index": entry_index,
                                "source_range": [source_range(fragments[indices[0]]["range"])[0],
                                                 source_range(fragments[indices[-1]]["range"])[1]],
                                "text_sha256": text_hash})
                    else:
                        raise ValueError("unknown consumed item record")
                if consumed_children != list(range(len(children))):
                    raise ValueError("item XML-consumed records omitted a child")
                if content["consumed"] != derive_item_content(fragments, expected_xml):
                    raise ValueError("item XML-consumed text/entries differ from complete raw source derivation")
                item_text_left -= sum(len(e["text"].encode("utf-8")) for e in content["consumed"] if e["kind"] == "text")
                if item_text_left < 0:
                    raise ValueError("item aggregate decoded text bound exceeded")
                if role == "inventory_item":
                    inventories.append({"container_index": owner_index, "source_range": list(bounds),
                                        "source_id": source_value(node, "id"), "instructions": instructions})
                    ordinal = inventory_ordinals.get(owner_index, 0)
                    inventory_ordinals[owner_index] = ordinal + 1
                    identity = source_value(node, "id")
                    loading_sources.append({"source_occurrence": {"container_index": owner_index, "item_index": ordinal},
                        "source_range": list(bounds), "authored_id": identity["decoded"] if identity else None,
                        "instructions": loading_instructions})
                elif role == "saved_set":
                    saved_sets.append({"container_index": owner_index, "source_range": list(bounds),
                                       "source_id": source_value(node, "id")})
                elif role == "jewel_assignment":
                    if collection != "trees" or spec is None or sockets is None:
                        raise ValueError("jewel assignment lost its passive-spec ownership")
                    jewels.append({"tree_root_index": owner_index, "spec_source_range": list(spec),
                                   "sockets_source_range": list(sockets), "source_range": list(bounds),
                                   "node_id": source_value(node, "nodeId"), "item_id": source_value(node, "itemId")})
                for child in children:
                    visit_item(child, collection, owner_index, bounds, spec, sockets, depth + 1, role, namespace)
            for collection, ranges in [("containers", item_container_ranges), ("trees", item_tree_ranges)]:
                nodes = array(item_projection[collection], 128, "item source roots")
                if [source_range(node["element"]["source_range"]) for node in nodes] != ranges:
                    raise ValueError("item source roots differ from preserved root sections")
                if source_index is not None:
                    source_root = source_index.get(root_range[0])
                    if source_root is None or source_root["end"] != root_range[1]:
                        raise ValueError("build root differs from exact imported XML")
                    names = {"Items"} if collection == "containers" else {"Tree", "Spec"}
                    expected_roots = [(n["start"], n["end"]) for n in source_root["children"] if n["name"] in names]
                    if ranges != expected_roots:
                        raise ValueError("item roots differ from complete imported source")
                for index, node in enumerate(nodes):
                    root_namespace = source_index[root_range[0]]["has_namespaces"] if source_index is not None else build["root"].get("has_namespaces", False)
                    visit_item(node, collection, index, root_range, None, None, 0, None, root_namespace)
            result.update(item_source_counts=item_counts, item_fragment_counts=fragment_counts,
                          item_inventory=inventories, item_sets=saved_sets, item_jewel_assignments=jewels)
    configuration = mapping(report["configuration"], "configuration result")
    result["configuration_status"] = configuration["status"]
    if configuration["status"] == "source_projected":
        config_report = {**report, "schema_version": 1, "scope": "configuration_source_projection_v1",
                         "configuration": configuration["projection"]}
        result["configuration_source_state"] = configuration_source_summary(
            config_report, expected_hash, expected_bytes)["source_state"]
    elif configuration["status"] != "not_projected" or not isinstance(configuration.get("error"), dict):
        raise ValueError("unknown configuration projection outcome")
    skills = mapping(report["skills"], "skills")
    result["skills_status"] = skills["status"]
    locations, counts, selection_ranges = {}, {}, set()
    if skills["status"] == "source_projected":
        projection = mapping(skills["projection"], "skill projection")
        require(projection, {"source_sha256": expected_hash}, "skill projection")
        remaining = 32768
        def visit(node, container, parent_range, saved_set, group, depth):
            nonlocal remaining
            remaining -= 1
            if remaining < 0 or depth > 32:
                raise ValueError("skill source node/depth bound exceeded")
            bounds = source_range(node["element"]["source_range"])
            if not parent_range[0] < bounds[0] < bounds[1] < parent_range[1]:
                raise ValueError("skill source range is outside its parent")
            role = label(node["source_use"])
            counts[role] = counts.get(role, 0) + 1
            if role == "saved_set":
                saved_set, group = bounds, None
            elif role == "group":
                group = bounds
            if role in {"gem_instance", "main_stat_set_selection", "calcs_stat_set_selection",
                        "main_minion_lookup", "calcs_minion_lookup"}:
                if bounds in locations:
                    raise ValueError("duplicated skill source occurrence")
                locations[bounds] = (container, saved_set, group, role)
                if role != "gem_instance":
                    selection_ranges.add(bounds)
            previous = bounds[0]
            for child in array(node["children"], 32768, "skill children"):
                child_range = source_range(child["element"]["source_range"])
                if child_range[0] < previous:
                    raise ValueError("skill children overlap or lost source order")
                previous = child_range[1]
                visit(child, container, bounds, saved_set, group, depth + 1)
        containers = array(projection["containers"], 128, "skill containers")
        if [source_range(node["element"]["source_range"]) for node in containers] != skill_container_ranges:
            raise ValueError("skill containers differ from preserved root sections")
        for index, node in enumerate(containers):
            visit(node, index, root_range, None, None, 1)
        result["skill_source_counts"] = counts
    elif skills["status"] != "not_projected" or not isinstance(skills.get("error"), dict):
        raise ValueError("unknown skill projection outcome")

    if definitions_requested != ("definition_lookup" in report):
        raise ValueError("definition lookup presence differs from requested inspection")
    if not definitions_requested:
        if "item_loading_implementation_sha256" in report:
            raise ValueError("source-only report cannot claim a loading implementation")
        result["item_loading_status"] = "not_run" if version >= 2 else "not_reported_by_inspector"
        return result
    definitions = mapping(report["definition_lookup"], "definition lookup")
    require(definitions, {"game_mechanics": "not_evaluated"}, "definition lookup")
    data = mapping(definitions["data"], "selected data identity")
    data_hash = data.get("content_sha256")
    if (not isinstance(data_hash, str) or len(data_hash) != 64
            or any(c not in "0123456789abcdef" for c in data_hash)):
        raise ValueError("invalid selected data content hash")
    if expected_data_hash is not None and data_hash != expected_data_hash:
        raise ValueError("identity lookup used different data from the supplied snapshot")
    result.update(data=data, data_trust=definitions["data_trust"],
                  data_identity_check="selected_snapshot" if expected_data_hash else "reported_by_inspector",
                  definition_implementation_sha256=report.get("definition_implementation_sha256"))
    if version == 3:
        result.update(item_loading_summary(report, data, expected_hash,
            loading_sources if result["item_source_status"] == "source_projected" else None))
    else:
        if "items" in definitions or "item_loading_implementation_sha256" in report:
            raise ValueError("legacy build report cannot claim schema-3 loading evidence")
        result["item_loading_status"] = "not_reported_by_inspector"
    identity = mapping(definitions["skills"], "skill definition result")
    if skills["status"] != "source_projected":
        require(identity, {"status": "not_looked_up"}, "skill definition result")
        result["skill_identity_status"] = "not_looked_up"
        return result
    require(identity, {"status": "looked_up"}, "skill definition result")
    lookup = mapping(identity["lookup"], "skill definition lookup")
    require(lookup, {"schema_version": 1, "source_xml_sha256": expected_hash,
                     "interpretation": "authored_identity_before_socket_group_processing",
                     "active_set_selection": "not_resolved", "actor_resolution": "not_resolved",
                     "name_matching": "not_run", "socket_group_processing": "not_run",
                     "game_mechanics": "not_evaluated"}, "skill definition lookup")
    if lookup["data"] != data or lookup["data_trust"] != definitions["data_trust"]:
        raise ValueError("skill lookup data identity/trust differs from selected data")
    seen, identity_counts = set(), {}
    for located in array(lookup["records"], 32768, "identity records"):
        record = mapping(located["record"], "identity record")
        value = mapping(record["lookup"], "identity value")
        bounds = source_range(value["source_range"])
        if bounds not in locations or bounds in seen:
            raise ValueError("identity record is missing its unique source occurrence")
        seen.add(bounds)
        container, saved_set, group, role = locations[bounds]
        def optional_range(value):
            return None if value is None else source_range(value)
        if (type(located["container_index"]) is not int or located["container_index"] != container
                or optional_range(located["set_source_range"]) != saved_set
                or optional_range(located["group_source_range"]) != group):
            raise ValueError("identity lookup changed source set/group ownership")
        if bounds in selection_ranges:
            if record["kind"] != "effect_selection" or value["source_use"] != role:
                raise ValueError("effect selection differs from source role")
            key = "effect_selection_resolved" if value["matched"] is not None else "effect_selection_unresolved"
        else:
            if record["kind"] != "instance":
                raise ValueError("gem instance identity differs from source role")
            resolution = mapping(value["resolution"], "instance resolution")
            kind = resolution["kind"]
            if kind == "external_gem":
                status = resolution["status"]
                candidates = array(resolution["candidates"], 50000, "gem candidates")
                expected_count = {"exact": 1, "single_fallback": 1, "missing": 0}.get(status)
                if status == "ambiguous":
                    if len(candidates) < 2:
                        raise ValueError("ambiguous identity requires multiple candidates")
                elif expected_count is None or len(candidates) != expected_count:
                    raise ValueError("external identity status/candidate count mismatch")
                key = "external_gem_" + status
            elif kind == "explicit_effect":
                key = "explicit_effect_resolved" if resolution["matched"] is not None else "explicit_effect_unresolved"
            elif kind in {"name_only_not_resolved", "missing_identity"}:
                key = kind
            else:
                raise ValueError("unknown instance identity interpretation")
        identity_counts[key] = identity_counts.get(key, 0) + 1
    if seen != set(locations):
        raise ValueError("identity lookup omitted an authored source reference")
    result.update(skill_identity_status="looked_up", skill_identity_counts=identity_counts)
    return result


def item_loading_summary(report: dict, data: dict, source_hash: str, sources: list | None) -> dict:
    """Bind diagnostic loading to every original occurrence; never certify effects."""
    def obj(value, description):
        if not isinstance(value, dict):
            raise ValueError(f"{description} must be an object")
        return value

    def sha(value):
        return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None

    def span(value):
        obj(value, "loading source range")
        start, end = value.get("start"), value.get("end")
        if type(start) is not int or type(end) is not int or not 0 <= start < end:
            raise ValueError("invalid loading source range")
        return [start, end]

    implementation = report.get("item_loading_implementation_sha256")
    if not sha(implementation):
        raise ValueError("missing or invalid loading implementation fingerprint")
    result = {"item_loading_implementation_sha256": implementation}
    value = obj(report["definition_lookup"].get("items"), "item loading result")
    verification = report["verification"].get("item_loading")
    if value.get("status") == "not_reported":
        if verification != "not_reported" or "report" in value:
            raise ValueError("unreported loading cannot claim a completed report")
        if sources is None:
            if value.get("source_error") != report["items"].get("error"):
                raise ValueError("loading projection failure differs from original source failure")
        elif not isinstance(value.get("error"), str) or not value["error"]:
            raise ValueError("unreported loading requires its diagnostic")
        return {**result, "item_loading_status": "not_reported"}
    if value.get("status") != "load_reported" or verification != "reported" or sources is None:
        raise ValueError("loading evidence requires projected source and matching verification")
    loaded = obj(value.get("report"), "item loading report")
    if (type(loaded.get("schema_version")) is not int or loaded["schema_version"] != 1
            or loaded.get("source_sha256") != source_hash or loaded.get("data_identity") != data
            or loaded.get("implementation_sha256") != implementation):
        raise ValueError("loading report changed its schema, source, data or implementation identity")
    records = loaded.get("items")
    if not isinstance(records, list) or len(records) != len(sources):
        raise ValueError("loading report omitted or added inventory occurrences")
    # State remains raw diagnostic metadata. Bound it without interpreting it as
    # numerical capability or freezing the entire engine's future item schema.
    budget = [1_500_000, MAX_LOADING_TEXT_BYTES]
    def metadata(value, depth=0):
        budget[0] -= 1
        if budget[0] < 0 or depth > 32:
            raise ValueError("loading metadata exceeds value/depth bounds")
        if isinstance(value, str):
            budget[1] -= len(value.encode("utf-8"))
            if budget[1] < 0:
                raise ValueError("loading metadata exceeds text bound")
        elif isinstance(value, dict):
            for key, child in value.items():
                metadata(key, depth + 1)
                metadata(child, depth + 1)
        elif isinstance(value, list):
            for child in value:
                metadata(child, depth + 1)
        elif value is not None and (type(value) not in {bool, int, float}
                or (type(value) is float and not math.isfinite(value))):
            raise ValueError("loading metadata has an invalid JSON value")

    statuses, dependencies, summaries = {}, {}, []
    for record, expected in zip(records, sources):
        obj(record, "loaded item occurrence")
        occurrence = obj(record.get("source_occurrence"), "item occurrence")
        if (any(type(occurrence.get(key)) is not int for key in ["container_index", "item_index"])
                or occurrence != expected["source_occurrence"]
                or span(record.get("source_range")) != expected["source_range"]
                or record.get("authored_id") != expected["authored_id"]):
            raise ValueError("loading changed inventory order, ownership, source range or authored ID")
        state = obj(record.get("state"), "loaded item state")
        metadata(state)
        status = record.get("status")
        if status not in {"complete", "no_base", "pending", "source_error"}:
            raise ValueError("unknown item loading status")
        pending = record.get("pending")
        if status == "pending":
            obj(pending, "pending item dependency")
            if (not isinstance(pending.get("kind"), str) or not pending["kind"]
                    or not isinstance(pending.get("message"), str) or not pending["message"]
                    or (pending.get("line_index") is not None and
                        (type(pending["line_index"]) is not int or pending["line_index"] < 0))):
                raise ValueError("pending loading requires a specific dependency")
            metadata(pending)
            dependencies[pending["kind"]] = dependencies.get(pending["kind"], 0) + 1
        elif pending is not None:
            raise ValueError("non-pending item cannot claim a pending dependency")
        instructions = record.get("instructions")
        if not isinstance(instructions, list) or len(instructions) != len(expected["instructions"]) + 2:
            raise ValueError("loading trace omitted constructor, consumed instructions or final assembly")
        stopped, stop_count, instruction_counts = False, 0, {}
        for index, instruction in enumerate(instructions):
            obj(instruction, "item loading instruction")
            if type(instruction.get("index")) is not int or instruction["index"] != index:
                raise ValueError("loading instruction indices changed order")
            if index in {0, len(instructions) - 1}:
                kind = "constructor" if index == 0 else "final_assembly"
                if (instruction.get("kind") != kind or instruction.get("consumed_index") is not None
                        or instruction.get("text_sha256") is not None):
                    raise ValueError("constructor/final instruction claimed authored content")
                if instruction.get("source_range") is not None:
                    raise ValueError("constructor/final instruction claimed an authored range")
            else:
                original = expected["instructions"][index-1]
                if (instruction.get("kind") != original["kind"]
                        or type(instruction.get("consumed_index")) is not int
                        or instruction["consumed_index"] != original["consumed_index"]
                        or instruction.get("text_sha256") != original["text_sha256"]
                        or span(instruction.get("source_range")) != original["source_range"]):
                    raise ValueError("loading instruction differs from original consumed source")
            step = instruction.get("status")
            if step not in {"executed", "pending", "not_executed"}:
                raise ValueError("unknown loading instruction status")
            if stopped and step != "not_executed":
                raise ValueError("loading executed an instruction after a stopped dependency")
            error = instruction.get("error")
            if error is not None and (not isinstance(error, str) or not error):
                raise ValueError("invalid instruction error")
            if instruction["kind"] == "constructor" and (step != "executed" or error is not None):
                raise ValueError("empty constructor must execute without a stop or error")
            if instruction["kind"] == "ignored" and (error is not None or step != ("not_executed" if stopped else "executed")):
                raise ValueError("ignored source instruction cannot stop or fail loading")
            if error is not None:
                metadata(error)
            if error is not None and (status != "source_error" or step != "executed"):
                raise ValueError("instruction error must stop an attempted source-error load")
            if step == "pending" or error is not None:
                stop_count += 1
                stopped = True
            elif step == "not_executed" and not stopped:
                raise ValueError("loading skipped an instruction without a preceding stop")
            instruction_counts[step] = instruction_counts.get(step, 0) + 1
        if (status in {"complete", "no_base"} and stop_count != 0
                or status in {"pending", "source_error"} and stop_count != 1):
            raise ValueError("item loading status differs from its execution trace")
        if status == "source_error" and not any(instruction.get("error") for instruction in instructions):
            raise ValueError("source-error loading requires the original failure diagnostic")
        statuses[status] = statuses.get(status, 0) + 1
        summaries.append({**occurrence, "source_range": expected["source_range"], "authored_id": record.get("authored_id"),
            "status": status, "pending": pending, "instruction_counts": instruction_counts})
    return {**result, "item_loading_status": "reported", "item_loading_counts": statuses,
            "item_loading_dependencies": dependencies, "item_loading_occurrences": summaries}


def xml_summary(path: Path) -> dict:
    data = read_bounded(path, MAX_XML_BYTES)
    if b"<!DOCTYPE" in data.upper() or b"<!ENTITY" in data.upper():
        raise ValueError("XML declarations with entities are not summarized")
    root = ET.fromstring(data)
    build = root.find("Build")
    tree = root.find("Tree")
    skills = root.find("Skills")
    items = root.find("Items")
    config = root.find("Config")

    def groups(parent):
        return [{"attributes": dict(group.attrib),
                 "gems": [dict(gem.attrib) for gem in group.findall("Gem")]}
                for group in parent.findall("Skill")]

    def config_entries(parent):
        return {"inputs": [dict(node.attrib) for node in parent.findall("Input")],
                "custom_modifier_blocks": [
                    {"attributes": dict(node.attrib),
                     "text_bytes": len("".join(node.itertext()).encode("utf-8")),
                     "text_sha256": hashlib.sha256("".join(node.itertext()).encode("utf-8")).hexdigest()}
                    for node in parent.findall("CustomModifierBlock")]}

    return {
        "root": root.tag,
        "attribute_semantics": "standard_xml_normalized_not_pob_source_values",
        "build": dict(build.attrib) if build is not None else None,
        "tree": {"attributes": dict(tree.attrib),
                 "specs": [{"attributes": dict(node.attrib),
                            "overrides": [dict(child.attrib) for child in node.iter()
                                          if child.tag in {"AttributeOverride", "Override"}]}
                           for node in tree.findall("Spec")]}
                if tree is not None else None,
        "skills": {"attributes": dict(skills.attrib),
                   "legacy_groups": groups(skills),
                   "sets": [{"attributes": dict(node.attrib), "groups": groups(node)}
                            for node in skills.findall("SkillSet")]}
                if skills is not None else None,
        "equipment": {"attributes": dict(items.attrib),
                      "item_count": len(items.findall("Item")),
                      "sets": [{"attributes": dict(node.attrib),
                                "slots": [dict(slot.attrib) for slot in node.findall("Slot")]}
                               for node in items.findall("ItemSet")]}
                if items is not None else None,
        "config": {"attributes": dict(config.attrib),
                   "legacy_entries": config_entries(config),
                   "sets": [{"attributes": dict(node.attrib), **config_entries(node)}
                            for node in config.findall("ConfigSet")]}
                if config is not None else None,
    }


def stop_tree(process: subprocess.Popen) -> None:
    try:
        if os.name == "nt":
            subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                           creationflags=subprocess.CREATE_NO_WINDOW, timeout=5, check=False)
        else:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
    finally:
        if process.poll() is None:
            process.kill()
        process.wait(timeout=5)


def invoke(command: list[str], directory: Path, prefix: str, workdir: Path,
           deadline: float, timeout: float) -> dict:
    """Each invocation has independent files and a shared absolute corpus deadline."""
    stdout = directory / f"{prefix}.stdout"
    stderr = directory / f"{prefix}.stderr"
    record = {"command": command, "stdout": stdout.name, "stderr": stderr.name}
    write_json(directory / f"{prefix}.command.json", {"argv": command, "cwd": str(workdir)})
    remaining = min(timeout, deadline - time.monotonic())
    if remaining <= 0:
        record["status"] = "deadline_before_start"
        return record
    started = time.monotonic()
    process = None
    try:
        with stdout.open("xb") as out, stderr.open("xb") as err:
            options = ({"creationflags": subprocess.CREATE_NO_WINDOW | subprocess.CREATE_NEW_PROCESS_GROUP}
                       if os.name == "nt" else {"start_new_session": True})
            process = subprocess.Popen(command, cwd=workdir, stdout=out, stderr=err, **options)
            try:
                record["exit_code"] = process.wait(timeout=remaining)
                record["status"] = "success" if process.returncode == 0 else "process_error"
            except subprocess.TimeoutExpired:
                record["status"] = "timeout"
                try:
                    stop_tree(process)
                except (OSError, subprocess.SubprocessError) as cleanup_error:
                    record["cleanup_error"] = str(cleanup_error)
                record["exit_code"] = process.poll()
    except (OSError, subprocess.SubprocessError) as error:
        record["status"] = "invocation_error"
        record["error"] = f"{type(error).__name__}: {error}"
        if process is not None and process.poll() is None:
            try:
                stop_tree(process)
            except (OSError, subprocess.SubprocessError) as cleanup_error:
                record["cleanup_error"] = str(cleanup_error)
    record["elapsed_seconds"] = time.monotonic() - started
    for label, path in [("stdout", stdout), ("stderr", stderr)]:
        if path.exists():
            record[f"{label}_bytes"] = path.stat().st_size
            record[f"{label}_sha256"] = digest(path)
    return record


def positive(value: str) -> float:
    number = float(value)
    if not math.isfinite(number) or number <= 0:
        raise argparse.ArgumentTypeError("must be a finite positive number")
    return number


def regular_file(value: str) -> Path:
    try:
        path = Path(value).resolve(strict=True)
    except OSError as error:
        raise argparse.ArgumentTypeError(str(error)) from error
    if not path.is_file():
        raise argparse.ArgumentTypeError(f"not a regular file: {value}")
    return path


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--input", required=True, type=regular_file)
    parser.add_argument("--output", required=True, type=Path, help="new directory; never reused")
    parser.add_argument("--import-cli", required=True, type=regular_file)
    parser.add_argument("--backend", action="append", required=True, metavar="native=CLI|pob=CLI")
    parser.add_argument("--inspect-configuration", action="store_true",
                        help="collect source configuration evidence using the explicit import CLI")
    parser.add_argument("--inspect-build", action="store_true",
                        help="collect full source/container and skill occurrence evidence")
    parser.add_argument("--with-definitions", action="store_true",
                        help="look up injected identities during --inspect-build; does not evaluate effects")
    parser.add_argument("--data", type=regular_file, help="native/inspection dataset; copied and hashed")
    parser.add_argument("--data-sha256", help="external review digest passed to native evaluation and build inspection")
    parser.add_argument("--options", type=regular_file, help="evaluation options; copied and hashed")
    parser.add_argument("--pob", type=Path, help="required explicit checkout when using the pob backend")
    parser.add_argument("--workdir", type=Path, default=Path.cwd())
    parser.add_argument("--jobs", type=int, default=1, help="maximum concurrent CLI processes (1..64)")
    parser.add_argument("--deadline-seconds", required=True, type=positive, help="whole corpus deadline")
    parser.add_argument("--evaluation-timeout-seconds", type=positive, default=90)
    parser.add_argument("--import-timeout-seconds", type=positive, default=30)
    args = parser.parse_args(argv)
    if not 1 <= args.jobs <= 64:
        parser.error("--jobs must be 1..64")
    args.workdir = args.workdir.resolve(strict=True)
    if not args.workdir.is_dir():
        parser.error("--workdir must be a directory")
    args.output = args.output.resolve()
    backends = []
    for value in args.backend:
        name, separator, path = value.partition("=")
        if not separator or name not in {"native", "pob"} or name in dict(backends):
            parser.error("each --backend must be a distinct native=CLI or pob=CLI")
        try:
            backends.append((name, regular_file(path)))
        except argparse.ArgumentTypeError as error:
            parser.error(str(error))
    args.backends = backends
    if "pob" in dict(backends):
        if args.pob is None or not args.pob.is_dir():
            parser.error("pob observations require an explicit --pob directory")
        args.pob = args.pob.resolve(strict=True)
    if args.data_sha256 and (args.data is None or len(args.data_sha256) != 64
                            or any(c not in "0123456789abcdefABCDEF" for c in args.data_sha256)):
        parser.error("--data-sha256 requires --data and 64 hexadecimal characters")
    if args.with_definitions and not args.inspect_build:
        parser.error("--with-definitions requires --inspect-build")
    if args.data is not None and "native" not in dict(backends) and not args.inspect_build:
        parser.error("--data requires a native backend or build inspection")
    return args


def main(argv=None) -> int:
    args = parse_args(argv)
    started = time.monotonic()
    deadline = started + args.deadline_seconds
    source = read_bounded(args.input, MAX_INPUT_BYTES)
    lines = line_records(source)
    snapshots = {"input": source}
    for name, path, limit in [("data", args.data, MAX_DATA_BYTES),
                               ("options", args.options, MAX_OPTIONS_BYTES)]:
        if path is not None:
            snapshots[name] = read_bounded(path, limit)
    args.output.mkdir(parents=False, exist_ok=False)
    (args.output / "corpus.source").write_bytes(source)
    watched = {"input": args.input, "import_cli": args.import_cli}
    watched.update({f"{name}_cli": cli for name, cli in args.backends})
    inputs = {}
    for name, path in [("data", args.data), ("options", args.options)]:
        if path is not None:
            target = args.output / f"{name}.source.json"
            target.write_bytes(snapshots[name])
            inputs[name] = target
            watched[name] = path
    provenance = {name: {"path": str(path), "sha256":
                          hashlib.sha256(snapshots[name]).hexdigest()
                          if name in snapshots else digest(path)}
                  for name, path in watched.items()}

    def entry(row):
        number, offset, raw = row
        directory = args.output / f"line-{number:05d}"
        directory.mkdir()
        payload = directory / "source.import"
        payload.write_bytes(raw)
        record = {"line": number, "source_offset": offset, "source_bytes": len(raw),
                  "source_sha256": hashlib.sha256(raw).hexdigest(), "directory": directory.name,
                  "evaluations": {}}
        if not raw.strip(b" \t\r\n"):
            record["status"] = "blank_line"
            write_json(directory / "index.json", record)
            return record
        if len(raw) > MAX_ENTRY_BYTES:
            record["status"] = "entry_too_large"
            write_json(directory / "index.json", record)
            return record
        try:
            xml = directory / "imported.xml"
            imported = invoke([str(args.import_cli), "import", str(payload), "--output", str(xml)],
                              directory, "import", args.workdir, deadline, args.import_timeout_seconds)
            record["import"] = imported
            if imported["status"] != "success":
                record["status"] = "import_failed"
            else:
                try:
                    record["import_metadata"] = bounded_json(directory / "import.stdout")
                    record["imported_xml_sha256"] = digest(xml)
                    if record["import_metadata"].get("xml_sha256") != record["imported_xml_sha256"]:
                        raise ValueError("import reported hash differs from exported XML")
                    record["original"] = xml_summary(xml)
                    record["status"] = "imported"
                except (OSError, ValueError, ET.ParseError) as error:
                    record["status"] = "import_evidence_error"
                    record["error"] = f"{type(error).__name__}: {error}"
                # Fresh evaluations stay independent of one another. A malformed
                # observation index does not replace or fabricate imported XML.
                if xml.is_file():
                    source_changed = False
                    if args.inspect_configuration:
                        # Pin the source established by import before invoking a helper.
                        # Never accept a newly reported hash of rewritten XML as the same input.
                        expected_hash = record.get("imported_xml_sha256") or digest(xml)
                        expected_bytes = xml.stat().st_size
                        result = invoke([str(args.import_cli), "inspect-configuration", str(xml)],
                                        directory, "configuration", args.workdir, deadline,
                                        args.import_timeout_seconds)
                        record["configuration"] = result
                        result["invocation_status"] = result["status"]
                        result["source_before_inspection_sha256"] = expected_hash
                        try:
                            after_hash = digest(xml)
                        except OSError:
                            after_hash = None
                        result["source_after_inspection_sha256"] = after_hash
                        source_changed = after_hash != expected_hash
                        result["source_changed"] = source_changed
                        if source_changed:
                            result["status"] = "configuration_evidence_error"
                            result["error"] = "Imported XML changed during configuration inspection; changed source is retained and will not be evaluated"
                        elif result["status"] == "success":
                            try:
                                report = bounded_json(directory / "configuration.stdout")
                                result.update(configuration_source_summary(report, expected_hash, expected_bytes))
                            except (OSError, ValueError, TypeError, KeyError) as error:
                                result["status"] = "configuration_evidence_error"
                                result["error"] = f"{type(error).__name__}: {error}"
                    if args.inspect_build:
                        if source_changed:
                            record["build_source"] = {"status": "source_changed",
                                "error": "Imported XML changed during an earlier inspection; build inspection not run"}
                        else:
                            expected_hash = record.get("imported_xml_sha256") or digest(xml)
                            expected_bytes = xml.stat().st_size
                            command = [str(args.import_cli), "inspect-build", str(xml)]
                            definitions_requested = args.with_definitions or args.data is not None
                            if args.with_definitions:
                                command.append("--with-definitions")
                            if args.data is not None:
                                command.extend(["--data", str(inputs["data"])])
                                if args.data_sha256:
                                    command.extend(["--data-sha256", args.data_sha256])
                            result = invoke(command, directory, "build_source", args.workdir,
                                            deadline, args.import_timeout_seconds)
                            record["build_source"] = result
                            result["invocation_status"] = result["status"]
                            result["source_before_inspection_sha256"] = expected_hash
                            try:
                                after_hash = digest(xml)
                            except OSError:
                                after_hash = None
                            result["source_after_inspection_sha256"] = after_hash
                            source_changed = after_hash != expected_hash
                            result["source_changed"] = source_changed
                            if source_changed:
                                result["status"] = "build_source_evidence_error"
                                result["error"] = "Imported XML changed during build inspection; changed source is retained and will not be evaluated"
                            elif result["status"] == "success":
                                try:
                                    report = bounded_json(directory / "build_source.stdout")
                                    result.update(build_source_summary(report, expected_hash, expected_bytes,
                                                  definitions_requested, provenance["data"]["sha256"] if args.data else None,
                                                  expected_xml=read_bounded(xml, MAX_XML_BYTES)))
                                except (OSError, ValueError, TypeError, KeyError) as error:
                                    result["status"] = "build_source_evidence_error"
                                    result["error"] = f"{type(error).__name__}: {error}"
                    for name, cli in args.backends:
                        if source_changed:
                            record["evaluations"][name] = {
                                "status": "source_changed",
                                "error": "Imported XML changed during source inspection; evaluation not run"}
                            continue
                        budget = min(args.evaluation_timeout_seconds, max(0, deadline-time.monotonic()))
                        command = [str(cli), "evaluate", str(xml), "--backend", name, "--raw",
                                   "--timeout-seconds", str(max(1, math.ceil(budget))),
                                   "--export", str(directory / f"{name}.export.xml")]
                        if args.options is not None:
                            command.extend(["--options", str(inputs["options"])])
                        if name == "pob":
                            command.extend(["--pob", str(args.pob)])
                        if name == "native" and args.data is not None:
                            command.extend(["--data", str(inputs["data"])])
                            if args.data_sha256:
                                command.extend(["--data-sha256", args.data_sha256])
                        result = invoke(command,directory,name,args.workdir,deadline,args.evaluation_timeout_seconds)
                        record["evaluations"][name] = result
                        if result["status"] == "success":
                            try:
                                report = bounded_json(directory / f"{name}.stdout")
                                evaluation = report["evaluation"]
                                if not isinstance(evaluation,dict):
                                    raise ValueError("evaluation must be a JSON object")
                                coverage = evaluation.get("coverage", {})
                                if not isinstance(coverage, dict):
                                    raise ValueError("evaluation coverage must be a JSON object")
                                for key in ["backend", "build"]:
                                    if not isinstance(evaluation.get(key), dict):
                                        raise ValueError(f"evaluation {key} must be a JSON object")
                                if not isinstance(evaluation.get("measurements", []), list):
                                    raise ValueError("evaluation measurements must be a JSON array")
                                result["report_schema_version"] = report.get("schema_version")
                                result["realized"] = {"backend":evaluation["backend"], "build":evaluation["build"],
                                    "coverage_schema_version": coverage.get("schema_version"),
                                    "active_skill_set_id": coverage.get("active_skill_set_id"),
                                    "selected_player":coverage.get("selected_player"),
                                    "selected_minion":coverage.get("selected_minion"),
                                    "diagnostic_only":evaluation.get("diagnostic_only"),
                                    "warnings":evaluation.get("warnings",[]),
                                    "measurement_count":len(evaluation.get("measurements",[]))}
                                export=directory / f"{name}.export.xml"
                                result["export_sha256"] = digest(export)
                                result["export_selection"] = xml_summary(export)
                            except (OSError, ValueError, TypeError, KeyError, ET.ParseError) as error:
                                result["status"] = "evaluation_evidence_error"
                                result["error"] = f"{type(error).__name__}: {error}"
        except Exception as error:
            record["status"] = "entry_error"
            record["error"] = f"{type(error).__name__}: {error}"
        write_json(directory / "index.json",record)
        return record

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
        records = list(pool.map(entry, lines))
    for name, path in watched.items():
        try:
            provenance[name]["unchanged_after_run"] = digest(path)==provenance[name]["sha256"]
        except OSError:
            provenance[name]["unchanged_after_run"] = False
    failed = sum(record.get("status") not in {"imported","blank_line"}
                 or any(value["status"]!="success" for value in record["evaluations"].values())
                 or record.get("configuration", {}).get("status", "success") != "success"
                 or record.get("build_source", {}).get("status", "success") != "success"
                 for record in records)
    changed_inputs = [name for name, value in provenance.items() if not value["unchanged_after_run"]]
    manifest = {"schema_version":5,"scope":"independent_import_and_fresh_evaluation_observations",
                "configuration_inspection_requested": args.inspect_configuration,
                "build_inspection_requested": args.inspect_build,
                "definition_lookup_requested": args.inspect_build and (args.with_definitions or args.data is not None),
                "claims":{"container_import_only":True,"build_legality_verified":False,"numerical_parity_verified":False},
                "provenance":provenance,"changed_inputs":changed_inputs,"pob_path":str(args.pob) if args.pob else None,
                "jobs":args.jobs,"deadline_seconds":args.deadline_seconds,
                "elapsed_seconds":time.monotonic()-started,"failed_entries":failed,"entries":records}
    write_json(args.output / "index.json",manifest)
    print(json.dumps({"index":str(args.output / "index.json"),"entries":len(records),"failed_entries":failed}))
    return 1 if failed or changed_inputs else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError) as error:
        print(f"Corpus intake failed: {error}",file=sys.stderr)
        sys.exit(2)
