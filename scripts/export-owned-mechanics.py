#!/usr/bin/env python3
"""Export reviewed literal mechanics into an existing owned recipe.

This optional offline tool recognizes only manifest-declared numeric row shapes.
It neither executes Lua nor interprets expressions, inheritance, or callbacks.
Owned identities, schemas, operations, and topology are supplied as persisted data.
Rust's production recipe assembler remains the schema/semantic validation gate.
"""
from __future__ import annotations

import argparse
import copy
from dataclasses import dataclass
from decimal import Decimal, InvalidOperation
import hashlib
import json
import math
from pathlib import Path
import re


@dataclass(frozen=True)
class Limits:
    manifest_bytes: int = 1024 * 1024
    recipe_bytes: int = 4 * 1024 * 1024
    catalog_bytes: int = 64 * 1024 * 1024
    source_bytes: int = 4 * 1024 * 1024
    total_source_bytes: int = 32 * 1024 * 1024
    files: int = 64
    tables: int = 64
    rows: int = 4096
    cells: int = 32768
    output_bytes: int = 8 * 1024 * 1024


HARD = Limits()
NUMBER = r"-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?"
TOKEN = re.compile(r"\{\{([a-z][a-z0-9_]*)\}\}")


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def compact(value) -> bytes:
    return json.dumps(value, ensure_ascii=False, allow_nan=False,
                      separators=(",", ":")).encode("utf-8")


def pretty(value) -> bytes:
    return (json.dumps(value, ensure_ascii=False, allow_nan=False,
                       indent=2) + "\n").encode("utf-8")


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def no_constant(value):
    raise ValueError(f"nonfinite JSON literal: {value}")


def read(path: Path, maximum: int) -> bytes:
    with path.open("rb") as stream:
        data = stream.read(maximum + 1)
    if len(data) > maximum:
        raise ValueError(f"file exceeds byte bound: {path}")
    return data


def load(path: Path, maximum: int):
    return json.loads(read(path, maximum), object_pairs_hook=unique,
                      parse_constant=no_constant)


def fields(value, required: set[str], name: str):
    if not isinstance(value, dict) or set(value) != required:
        raise ValueError(f"{name} fields differ from the reviewed format")


def validate_limits(limits: Limits):
    for name, maximum in vars(HARD).items():
        value = getattr(limits, name)
        if type(value) is not int or not 0 < value <= maximum:
            raise ValueError(f"invalid {name} limit")


def integer(value, name: str) -> int:
    if type(value) is not int or not -(2**53 - 1) <= value <= 2**53 - 1:
        raise ValueError(f"{name} must be an exact bounded integer")
    return value


def number(text: str):
    if len(text) > 128 or re.fullmatch(NUMBER, text) is None:
        raise ValueError("literal is not a bounded decimal number")
    try:
        value = float(Decimal(text))
    except (InvalidOperation, OverflowError) as error:
        raise ValueError("invalid numeric literal") from error
    if not math.isfinite(value) or abs(value) > 1e12:
        raise ValueError("numeric literal exceeds finite evidence envelope")
    return 0.0 if value == 0 else value


def matcher(template: str):
    if not isinstance(template, str) or len(template) > 2048:
        raise ValueError("row template exceeds bound")
    tokens = list(TOKEN.finditer(template))
    names = [token.group(1) for token in tokens]
    if not names or len(names) > 32 or len(names) != len(set(names)):
        raise ValueError("row template has empty/duplicate/excess captures")
    for previous, current in zip(tokens, tokens[1:]):
        separator = template[previous.end():current.start()]
        if re.fullmatch(r"[0-9eE+.\-]*", separator) is not None:
            raise ValueError("numeric captures require an unambiguous literal separator")
    parts, last = [], 0
    for token in tokens:
        parts += [re.escape(template[last:token.start()]),
                  f"(?P<{token.group(1)}>{NUMBER})"]
        last = token.end()
    parts.append(re.escape(template[last:]))
    return re.compile("".join(parts)), names


def source_span(span, files):
    fields(span, {"path", "line", "end_line", "sha256"}, "source span")
    text = files.get(span["path"])
    if text is None:
        raise ValueError("span refers to an unpinned source file")
    first, last = integer(span["line"], "line"), integer(span["end_line"], "end line")
    lines = text.splitlines(keepends=True)
    if first < 1 or last < first or last > len(lines):
        raise ValueError("source span exceeds file")
    selected = "".join(lines[first - 1:last])
    if sha(selected.encode("utf-8")) != span["sha256"]:
        raise ValueError("source span hash differs")
    return selected


def record_rows(spec, files, limits):
    fields(spec, {"id", "span", "opening", "closing", "row_template",
                  "minimum", "maximum", "columns"}, "record table")
    minimum, maximum = integer(spec["minimum"], "minimum"), integer(spec["maximum"], "maximum")
    count = maximum - minimum + 1
    if not 0 < count <= limits.rows:
        raise ValueError("table row domain exceeds bound")
    text = source_span(spec["span"], files)
    lines = text.splitlines()
    if len(lines) != count + 2 or lines[0].strip() != spec["opening"] or lines[-1].strip() != spec["closing"]:
        raise ValueError("record table shape or complete row census differs")
    pattern, names = matcher(spec["row_template"])
    if set(names) != {"level", *spec["columns"]} or len(spec["columns"]) != len(set(spec["columns"])):
        raise ValueError("record columns differ from numeric captures")
    if count * len(spec["columns"]) > limits.cells:
        raise ValueError("table cell bound exceeded")
    rows = []
    for expected, line in zip(range(minimum, maximum + 1), lines[1:-1], strict=True):
        matched = pattern.fullmatch(line.strip())
        if matched is None:
            raise ValueError(f"unsupported literal row shape at level {expected}")
        captures = matched.groupdict()
        if re.fullmatch(r"[0-9]+", captures.pop("level")) is None or int(matched["level"]) != expected:
            raise ValueError("duplicate, missing, or reordered source level")
        rows.append({"level": expected, "values": {name: number(captures[name]) for name in spec["columns"]},
                     "literal_tokens": {name: captures[name] for name in spec["columns"]},
                     "source_line": spec["span"]["line"] + len(rows) + 1})
    return {"id": spec["id"], "span": spec["span"], "minimum": minimum, "maximum": maximum, "rows": rows}


def array_rows(spec, files, limits):
    fields(spec, {"id", "span", "prefix", "suffix", "minimum", "maximum", "column"}, "array table")
    minimum, maximum = integer(spec["minimum"], "minimum"), integer(spec["maximum"], "maximum")
    count = maximum - minimum + 1
    if minimum != 1:
        raise ValueError("implicit source array keys must begin at one")
    if not 0 < count <= limits.rows:
        raise ValueError("array table domain exceeds bound")
    text = source_span(spec["span"], files).strip()
    if not text.startswith(spec["prefix"]) or not spec["suffix"] or not text.endswith(spec["suffix"]):
        raise ValueError("array literal boundaries differ")
    body = text[len(spec["prefix"]):-len(spec["suffix"])].strip()
    if not body.endswith(","):
        raise ValueError("reviewed array requires a trailing comma")
    tokens = [part.strip() for part in body[:-1].split(",")]
    if len(tokens) != count:
        raise ValueError("array row census differs")
    return {"id": spec["id"], "span": spec["span"], "minimum": minimum, "maximum": maximum,
            "rows": [{"level": level, "values": {spec["column"]: number(token)},
                      "literal_tokens": {spec["column"]: token}, "source_line": spec["span"]["line"]}
                     for level, token in zip(range(minimum, maximum + 1), tokens, strict=True)]}



def preflight_tables(manifest, limits):
    """Charge the complete expansion before any row builder allocates results."""
    records, arrays = manifest["record_tables"], manifest["array_tables"]
    if len(records) + len(arrays) > limits.tables:
        raise ValueError("table collection bound exceeded")
    rows_left, cells_left = limits.rows, limits.cells
    for spec, record in [(s, True) for s in records] + [(s, False) for s in arrays]:
        required = {"id", "span", "opening", "closing", "row_template", "minimum", "maximum", "columns"} if record else {"id", "span", "prefix", "suffix", "minimum", "maximum", "column"}
        fields(spec, required, "record table" if record else "array table")
        minimum = integer(spec["minimum"], "minimum")
        if not record and minimum != 1:
            raise ValueError("implicit source array keys must begin at one")
        count = integer(spec["maximum"], "maximum") - minimum + 1
        if not 0 < count <= limits.rows:
            raise ValueError("table row domain exceeds bound")
        columns = spec["columns"] if record else [spec["column"]]
        if not isinstance(columns, list) or not 0 < len(columns) <= 31 or any(not isinstance(c, str) or re.fullmatch(r"[a-z][a-z0-9_]*", c) is None for c in columns) or len(set(columns)) != len(columns) or "level" in columns:
            raise ValueError("invalid numeric table columns")
        cells = count * len(columns)
        if count > rows_left or cells > cells_left:
            raise ValueError("aggregate table row/cell bound exceeded before expansion")
        rows_left -= count
        cells_left -= cells

def export(manifest, recipe, catalog_bytes: bytes, source_root: Path, limits: Limits = HARD):
    validate_limits(limits)
    fields(manifest, {"schema_version", "source", "catalog_lf_sha256", "identities",
                      "record_tables", "array_tables", "literal_facts", "algorithm_evidence", "table_bindings", "literal_bindings", "unresolved"}, "manifest")
    fields(recipe, {"schema_version", "registry", "schema", "rules", "routing"}, "owned recipe")
    fields(recipe["rules"], {"schema_version", "namespace", "release", "semantics_version", "operations_version", "definitions", "tables", "owners", "receivers"}, "owned rules")
    if recipe["rules"]["schema_version"] != 2 or recipe["rules"]["operations_version"] != "owned-domain-operations-v5":
        raise ValueError("unsupported owned rule package version/operations")
    if manifest["schema_version"] != 1 or recipe["schema_version"] != 1:
        raise ValueError("unsupported manifest/recipe version")
    if len(compact(manifest)) > limits.manifest_bytes or len(compact(recipe)) > limits.recipe_bytes:
        raise ValueError("manifest/recipe byte bound exceeded")
    normalized_catalog = catalog_bytes.replace(b"\r\n", b"\n")
    if len(catalog_bytes) > limits.catalog_bytes or sha(normalized_catalog) != manifest["catalog_lf_sha256"]:
        raise ValueError("identity catalog source pin differs")
    catalog = json.loads(normalized_catalog, object_pairs_hook=unique, parse_constant=no_constant)["skill_identities"]
    if catalog["schema_version"] != 1 or catalog["capability"] != "identity_only":
        raise ValueError("unreviewed identity catalog capability/version")
    source = manifest["source"]
    fields(source, {"system", "revision", "files"}, "source")
    if source["system"] != "path_of_building2" or source["revision"] != catalog["source"]["upstream_revision"]:
        raise ValueError("source revision/system differs")
    if not 0 < len(source["files"]) <= limits.files:
        raise ValueError("source file collection exceeds bound")
    root = source_root.resolve()
    files, total = {}, 0
    for file in source["files"]:
        fields(file, {"path", "sha256"}, "source file")
        path = (root / file["path"]).resolve()
        if not path.is_relative_to(root) or file["path"] in files:
            raise ValueError("source path escapes root or repeats")
        raw = read(path, limits.source_bytes).replace(b"\r\n", b"\n")
        total += len(raw)
        if total > limits.total_source_bytes or sha(raw) != file["sha256"]:
            raise ValueError("source aggregate bound/hash differs")
        files[file["path"]] = raw.decode("utf-8")
    identities = []
    if len(manifest["identities"]) > limits.tables:
        raise ValueError("identity selection bound exceeded")
    for item in manifest["identities"]:
        fields(item, {"gem_key", "skill_id", "gem_definition", "skill_definition"}, "identity")
        gems = [row for row in catalog["gems"] if row["key"] == item["gem_key"]]
        skills = [row for row in catalog["skills"] if row["id"] == item["skill_id"]]
        if len(gems) != 1 or len(skills) != 1 or gems[0]["primary_effect_id"] != item["skill_id"]:
            raise ValueError("selected gem/primary identity is missing or ambiguous")
        gem, skill = gems[0], skills[0]
        g = [row for row in catalog["gem_declarations"] if row["index"] == gem["winning_declaration"]]
        s = [row for row in catalog["skill_declarations"] if row["index"] == skill["winning_declaration"]]
        if len(g) != 1 or len(s) != 1 or g[0]["key"] != gem["key"] or s[0]["id"] != skill["id"]:
            raise ValueError("winning declaration identity differs")
        for declaration in (g[0], s[0]):
            source_span(declaration["source"], files)
        identities.append({**item, "gem": gem, "skill": skill,
                           "gem_source": g[0]["source"], "skill_source": s[0]["source"]})
    selected_keys = {row["gem_key"] for row in manifest["identities"]}
    missing = [row for row in catalog["missing_references"] if row["gem_key"] in selected_keys]
    if missing != manifest["unresolved"] or any(row["effect_id"] in {s["id"] for s in catalog["skills"]} for row in missing):
        raise ValueError("missing-reference ledger changed; review required")
    preflight_tables(manifest, limits)
    tables = [record_rows(spec, files, limits) for spec in manifest["record_tables"]]
    tables += [array_rows(spec, files, limits) for spec in manifest["array_tables"]]
    if len({table["id"] for table in tables}) != len(tables):
        raise ValueError("duplicate exported table identity")
    if sum(len(table["rows"]) for table in tables) > limits.rows or sum(len(row["values"]) for table in tables for row in table["rows"]) > limits.cells:
        raise ValueError("aggregate table row/cell bound exceeded")
    literal_facts = []
    if len(manifest["literal_facts"]) > limits.tables:
        raise ValueError("literal fact collection exceeds bound")
    for spec in manifest["literal_facts"]:
        fields(spec, {"id", "span", "template"}, "literal fact")
        text = source_span(spec["span"], files).strip()
        pattern, names = matcher(spec["template"])
        matched = pattern.fullmatch(text)
        if matched is None:
            raise ValueError("literal fact shape differs")
        literal_facts.append({"id": spec["id"], "span": spec["span"],
                              "values": {name: number(matched[name]) for name in names},
                              "literal_tokens": matched.groupdict()})
    algorithm_evidence, evidence_bytes_used, algorithm_ids = [], 0, set()
    if len(manifest["algorithm_evidence"]) > limits.tables:
        raise ValueError("algorithm evidence collection exceeds bound")
    for spec in manifest["algorithm_evidence"]:
        fields(spec, {"id", "span", "claim"}, "algorithm evidence")
        if not isinstance(spec["id"], str) or not spec["id"] or spec["id"] in algorithm_ids:
            raise ValueError("algorithm evidence identity is empty or duplicated")
        if not isinstance(spec["claim"], str) or not 0 < len(spec["claim"]) <= 2048:
            raise ValueError("algorithm evidence claim exceeds bound")
        algorithm_ids.add(spec["id"])
        text = source_span(spec["span"], files)
        evidence_bytes_used += len(text.encode("utf-8"))
        if evidence_bytes_used > limits.output_bytes:
            raise ValueError("algorithm evidence text bound exceeded")
        algorithm_evidence.append({**spec, "text": text})
    result = copy.deepcopy(recipe)
    if len(manifest["literal_bindings"]) > limits.tables:
        raise ValueError("literal binding collection exceeds bound")
    literal_by_id = {fact["id"]: fact for fact in literal_facts}
    if len(literal_by_id) != len(literal_facts):
        raise ValueError("duplicate literal fact identity")
    bound_literals = set()
    for binding in manifest["literal_bindings"]:
        fields(binding, {"owner", "program", "node", "source_fact", "column"}, "literal binding")
        address = (compact(binding["owner"]), binding["program"], binding["node"])
        if address in bound_literals:
            raise ValueError("duplicate literal binding")
        bound_literals.add(address)
        owner = [row for row in result["rules"]["owners"] if row["owner"] == binding["owner"]]
        programs = [p for row in owner for p in row["programs"]["members"] if p["id"] == binding["program"]]
        nodes = [n for p in programs for n in p["nodes"] if n["id"] == binding["node"]]
        source_fact = literal_by_id.get(binding["source_fact"])
        if len(owner) != 1 or len(programs) != 1 or len(nodes) != 1 or source_fact is None:
            raise ValueError("literal binding owner/program/node/fact is missing or ambiguous")
        token = source_fact["literal_tokens"].get(binding["column"])
        expression = nodes[0]["expression"]
        if token is None or expression.get("kind") != "literal":
            raise ValueError("literal binding requires an exact source column and Literal node")
        value = expression["value"]
        if value["kind"] == "quantity":
            value["value"]["value"] = number(token)
        elif value["kind"] == "integer":
            exact = Decimal(token)
            if exact != exact.to_integral_value():
                raise ValueError("nonintegral literal cannot populate integer node")
            value["value"] = integer(int(exact), "literal value")
        else:
            raise ValueError("literal export supports integer/quantity nodes only")
    owned_tables = result["rules"]["tables"]
    if len(owned_tables) > limits.tables or len({t["id"] for t in owned_tables}) != len(owned_tables):
        raise ValueError("owned table identity/bound differs")
    owned = {table["id"]: table for table in owned_tables}
    exported = {table["id"]: table for table in tables}
    if len(manifest["table_bindings"]) != len(owned) or len({b["table"] for b in manifest["table_bindings"]}) != len(owned):
        raise ValueError("every owned table needs one exact source binding")
    for binding in manifest["table_bindings"]:
        fields(binding, {"table", "source_table", "column"}, "table binding")
        table, values = owned.get(binding["table"]), exported.get(binding["source_table"])
        if table is None or values is None or (table["minimum"], table["maximum"]) != (values["minimum"], values["maximum"]):
            raise ValueError("owned table/source domain differs")
        rows = []
        for row in values["rows"]:
            token = row["literal_tokens"].get(binding["column"])
            if token is None:
                raise ValueError("missing source column")
            if table["value_type"] == {"kind": "integer"}:
                exact = Decimal(token)
                if exact != exact.to_integral_value():
                    raise ValueError("nonintegral source cannot populate integer table")
                rows.append({"kind": "integer", "value": integer(int(exact), "table cell")})
            elif table["value_type"].get("kind") == "quantity":
                rows.append({"kind": "quantity", "value": {"value": number(token), "unit": table["value_type"]["value"]["unit"]}})
            else:
                raise ValueError("literal export supports integer/quantity tables only")
        table["rows"] = rows
    recipe_bytes = pretty(result)
    evidence = {"schema_version": 1, "scope": "reviewed-literal-mechanics-components", "source": source,
                "catalog_lf_sha256": manifest["catalog_lf_sha256"], "manifest_sha256": sha(pretty(manifest)),
                "recipe_sha256": sha(recipe_bytes), "identities": identities, "tables": tables,
                "literal_facts": literal_facts, "algorithm_evidence": algorithm_evidence, "unresolved": missing,
                "full_build_numeric_coverage": False, "source_execution": False}
    evidence_bytes = pretty(evidence)
    if len(recipe_bytes) > limits.recipe_bytes or len(recipe_bytes) + len(evidence_bytes) > limits.output_bytes:
        raise ValueError("output byte bound exceeded")
    return result, evidence


def publish(directory: Path, outputs: dict[str, bytes]):
    """Reserve a new export directory; runtime bundle publication is the assembler's job."""
    directory.parent.mkdir(parents=True, exist_ok=True)
    # This is the no-clobber operation, not a racy existence precheck + rename.
    directory.mkdir(exist_ok=False)
    try:
        for name, data in outputs.items():
            with (directory / name).open("xb") as stream:
                stream.write(data)
    except OSError as error:
        raise ValueError(f"export write failed; incomplete new directory retained at {directory}") from error


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--recipe", type=Path, required=True)
    parser.add_argument("--identity-catalog", type=Path, required=True)
    parser.add_argument("--source-root", type=Path, required=True)
    destination = parser.add_mutually_exclusive_group(required=True)
    destination.add_argument("--output-dir", type=Path)
    destination.add_argument("--check-facts", type=Path)
    args = parser.parse_args()
    recipe, evidence = export(load(args.manifest, HARD.manifest_bytes), load(args.recipe, HARD.recipe_bytes),
                              read(args.identity_catalog, HARD.catalog_bytes), args.source_root)
    outputs = {"recipe.json": pretty(recipe), "mechanics-facts.json": pretty(evidence)}
    if args.check_facts:
        if outputs["recipe.json"] != read(args.recipe, HARD.recipe_bytes) or outputs["mechanics-facts.json"] != read(args.check_facts, HARD.output_bytes):
            raise ValueError("persisted owned recipe/evidence differ from source export")
        print(f"verified {len(recipe['rules']['tables'])} owned finite tables; no source execution")
    else:
        publish(args.output_dir, outputs)
        print(f"published {args.output_dir}; Rust assembler validation remains required")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError, RecursionError) as error:
        raise SystemExit(f"owned mechanics export: {error}") from error
