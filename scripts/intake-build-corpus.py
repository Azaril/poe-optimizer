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
from pathlib import Path
import signal
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

MAX_INPUT_BYTES = 64 * 1024 * 1024
MAX_ENTRY_BYTES = 16 * 1024 * 1024
MAX_JSON_BYTES = 64 * 1024 * 1024
MAX_XML_BYTES = 32 * 1024 * 1024
MAX_LINES = 10000
MAX_DATA_BYTES = 16 * 1024 * 1024
MAX_OPTIONS_BYTES = 1024 * 1024


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
    parser.add_argument("--data", type=regular_file, help="native dataset; copied and hashed")
    parser.add_argument("--data-sha256", help="external review digest passed to the native CLI")
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
    if args.data is not None and "native" not in dict(backends):
        parser.error("--data requires a native backend observation")
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
                    for name, cli in args.backends:
                        if source_changed:
                            record["evaluations"][name] = {
                                "status": "source_changed",
                                "error": "Imported XML changed during configuration inspection; evaluation not run"}
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
                 for record in records)
    changed_inputs = [name for name, value in provenance.items() if not value["unchanged_after_run"]]
    manifest = {"schema_version":2,"scope":"independent_import_and_fresh_evaluation_observations",
                "configuration_inspection_requested": args.inspect_configuration,
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
