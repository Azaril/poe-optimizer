#!/usr/bin/env python3
"""Export finite quest input metadata; never compile/evaluate reward stat text.

The optional existing `extract-game-data` CLI produces the injected catalog.
This smaller follow-on export uses its configuration metadata only, verifies the
catalog's vendor source pins, and requires the caller's exact catalog digest.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

MAX_PACKAGE_BYTES = 64 * 1024 * 1024
MAX_SOURCE_BYTES = 4 * 1024 * 1024
MAX_SOURCE_FILES = 128
MAX_TOTAL_SOURCE_BYTES = 16 * 1024 * 1024
MAX_OUTPUT_BYTES = 1024 * 1024
MAX_REWARDS = 1024


def read(path: Path, maximum: int) -> bytes:
    with path.open("rb") as stream:
        data = stream.read(maximum + 1)
    if len(data) > maximum:
        raise ValueError(f"{path} exceeds byte bound")
    return data


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def reject_constant(value):
    raise ValueError(f"nonfinite JSON value: {value}")


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def export(data: bytes, expected: str, source_root: Path) -> dict:
    if digest(data) != expected:
        raise ValueError("catalog digest differs from explicit source pin")
    package = json.loads(data, object_pairs_hook=unique_object,
                         parse_constant=reject_constant)
    metadata = package["configuration"]
    if metadata["schema_version"] != 2 or metadata["capability"] != "metadata_only":
        raise ValueError("unreviewed configuration metadata version/capability")
    root = source_root.resolve()
    files = []
    total_source_bytes = 0
    if len(metadata["source"]["files"]) > MAX_SOURCE_FILES:
        raise ValueError("source file collection exceeds bound")
    for name, expected_hash in sorted(metadata["source"]["files"].items()):
        path = (root / name).resolve()
        if not path.is_relative_to(root):
            raise ValueError("source pin escapes injected source root")
        normalized = read(path, MAX_SOURCE_BYTES).replace(b"\r\n", b"\n")
        total_source_bytes += len(normalized)
        if total_source_bytes > MAX_TOTAL_SOURCE_BYTES:
            raise ValueError("aggregate source byte bound exceeded")
        if digest(normalized) != expected_hash:
            raise ValueError(f"vendor LF source pin differs: {name}")
        files.append({"path": name, "sha256": expected_hash})
    rows = []
    seen = set()
    for definition in metadata["definitions"]:
        source = definition["source"]
        quest = source["quest"]
        if quest is None:
            continue
        if len(rows) >= MAX_REWARDS:
            raise ValueError("quest collection exceeds bound")
        key = definition["key"]
        if not key or key in seen:
            raise ValueError("empty or duplicate quest configuration key")
        seen.add(key)
        record = quest["record"]
        def text(name):
            field = record[name]
            if field["kind"] != "text" or not isinstance(field["value"], str):
                raise ValueError(f"quest {name} is not source text")
            return field["value"]
        if key != "quest" + text("Description") + text("Area") + text("Info"):
            raise ValueError("generated key differs from recorded quest identity")
        if record.get("useConfig") == {"kind": "boolean", "value": False}:
            raise ValueError("excluded quest unexpectedly emitted a definition")
        defaults = definition["defaults"]
        if defaults["placeholder"] is not None:
            raise ValueError("quest placeholder needs an explicit export policy")
        if definition["widget"] == "check":
            default = defaults["input"]
            if (type(default) is not bool or defaults["option_index"] is not None
                    or definition["options"] or definition["scalar_kinds"] != ["boolean"]):
                raise ValueError("invalid fixed reward input metadata")
            choice = {"kind": "boolean", "default": default,
                      "stat_text": text("Stat")}
        elif definition["widget"] == "list":
            options = definition["options"]
            index = defaults["option_index"]
            if (defaults["input"] is not None or type(index) is not int
                    or not 1 <= index <= len(options)
                    or definition["scalar_kinds"] != ["text"]):
                raise ValueError("invalid option reward default metadata")
            values = [option["value"] for option in options]
            if (any(not isinstance(value, str) for value in values)
                    or len(set(values)) != len(values)
                    or [option["index"] for option in options] != list(range(1, len(options) + 1))):
                raise ValueError("invalid ordered option token metadata")
            declared = record["Options"]
            if (declared["kind"] != "array"
                    or any(value["kind"] != "text" for value in declared["value"])
                    or values != ["None"] + [value["value"] for value in declared["value"]]):
                raise ValueError("generated options differ from recorded quest values")
            choice = {"kind": "option", "default": values[index - 1],
                      "none_token": values[0], "options": values}
        else:
            raise ValueError("unreviewed quest widget")
        rows.append({"config_key": key, "source_index": quest["source_index"],
                     "location": source["location"], "generator": source["generator"],
                     "source_record": record, "choice": choice})
    if not rows:
        raise ValueError("empty reward metadata export")
    rows.sort(key=lambda row: row["config_key"])
    return {
        "schema_version": 1,
        "scope": "quest_input_metadata_fixture",
        "effects_compiled": False,
        "package": {"sha256": expected, "schema_version": package["manifest"]["schema_version"],
                    "release": package["manifest"]["release"],
                    "semantics_version": package["manifest"]["semantics_version"]},
        "source": {"system": "path_of_building2",
                   "revision": metadata["source"]["upstream_revision"], "files": files},
        "defaults_evidence": {name: metadata["source"][name]
                              for name in ["create_config_set", "get_default_state"]},
        "rows": rows,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data", type=Path, required=True)
    parser.add_argument("--data-sha256", required=True)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--check", action="store_true", help="compare without writing")
    args = parser.parse_args()
    artifact = export(read(args.data, MAX_PACKAGE_BYTES), args.data_sha256, args.source_root)
    data = (json.dumps(artifact, ensure_ascii=False, indent=2) + "\n").encode()
    if len(data) > MAX_OUTPUT_BYTES:
        raise ValueError("fixture exceeds output bound")
    if args.check:
        if read(args.output, MAX_OUTPUT_BYTES) != data:
            raise ValueError("saved fixture differs from reproducible metadata export")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with args.output.open("xb") as stream:
            stream.write(data)
    print(f"{len(artifact['rows'])} reward rows; {len(data)} bytes; sha256={digest(data)}")


if __name__ == "__main__":
    main()
