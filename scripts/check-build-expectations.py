#!/usr/bin/env python3
"""Compare caller-supplied corpus artifacts with fixed diagnostic expectations.

This executes no evaluator and never establishes complete build or mechanic parity.
Exact comparison is the default; finite tolerance is an explicit diagnostic option.
"""
from __future__ import annotations

import argparse
import importlib.util
import math
from pathlib import Path
import struct
import sys

sys.dont_write_bytecode = True
_SPEC = importlib.util.spec_from_file_location(
    "corpus_intake", Path(__file__).with_name("intake-build-corpus.py"))
_INTAKE = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_INTAKE)


def exact(left, right):
    """Keep booleans distinct from numbers, and preserve signed numeric zero."""
    if isinstance(left, bool) or isinstance(right, bool):
        return type(left) is type(right) and left == right
    if isinstance(left, (int, float)) and isinstance(right, (int, float)):
        if left != right:
            return False
        if left == 0:
            return struct.pack("!d", float(left)) == struct.pack("!d", float(right))
        return True
    if type(left) is not type(right):
        return False
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(exact(left[k], right[k]) for k in left)
    if isinstance(left, list):
        return len(left) == len(right) and all(exact(a, b) for a, b in zip(left, right))
    return left == right


def finite_number(value):
    if type(value) not in (int, float):
        return False
    try:
        return math.isfinite(value)
    except OverflowError:
        return False


def measurement_map(rows):
    if not isinstance(rows, list) or not rows:
        raise ValueError("measurements must be a nonempty array")
    result = {}
    for row in rows:
        if not isinstance(row, dict) or set(row) != {"query", "schema_version", "unit", "value"}:
            raise ValueError("invalid measurement fields")
        query = row["query"]
        if (not isinstance(query, dict) or set(query) != {"actor", "id"}
                or not all(isinstance(v, str) and v for v in query.values())):
            raise ValueError("invalid metric query")
        if type(row["schema_version"]) is not int or row["schema_version"] != 1:
            raise ValueError("unsupported metric schema")
        if not isinstance(row["unit"], str) or not row["unit"]:
            raise ValueError("missing metric unit")
        key = (query["actor"], query["id"])
        if key in result:
            raise ValueError(f"duplicate metric query {key}")
        value = row["value"]
        if not isinstance(value, dict):
            raise ValueError("invalid measurement value")
        status = value.get("status")
        if status == "finite":
            number = value.get("value")
            if (set(value) != {"status", "value"} or type(number) not in (int, float)
                    or not finite_number(number)):
                raise ValueError("invalid finite measurement")
        elif status == "non_finite":
            if (set(value) != {"status", "kind"} or value["kind"] not in
                    {"positive_infinity", "negative_infinity", "not_a_number"}):
                raise ValueError("invalid non-finite measurement")
        elif status == "unavailable":
            if (set(value) != {"status", "reason"}
                    or not isinstance(value["reason"], str) or not value["reason"]):
                raise ValueError("invalid unavailable measurement")
        else:
            raise ValueError("unknown measurement availability")
        result[key] = row
    return result


def validate_manifest(manifest):
    if (type(manifest.get("schema_version")) is not int or manifest["schema_version"] != 1
            or manifest.get("scope") != "diagnostic_reference_expectations"
            or manifest.get("whole_build_parity") != "not_established"):
        raise ValueError("unsupported expectation manifest or parity claim")
    cases = manifest.get("cases")
    if not isinstance(cases, list) or not cases:
        raise ValueError("expectation denominator cannot be empty")
    ids, lines = set(), set()
    for case in cases:
        if not isinstance(case, dict):
            raise ValueError("expected case object")
        case_id, line = case.get("id"), case.get("source_line")
        if not isinstance(case_id, str) or not case_id or case_id in ids:
            raise ValueError("missing or duplicate expectation id")
        if type(line) is not int or line < 1 or line in lines:
            raise ValueError("invalid or duplicate expected source line")
        ids.add(case_id)
        lines.add(line)
        measurement_map(case["measurements"])
        if not all(isinstance(case.get(k), dict) for k in ("input", "reference", "binding")):
            raise ValueError("input, reference and binding must be objects")
        version = case["reference"].get("report_schema_version")
        if type(version) is not int or version < 1:
            raise ValueError("expected report schema must be a positive integer")
        status = case["reference"].get("report_status")
        if not isinstance(status, str) or not status:
            raise ValueError("expected report status is missing")
        for key in ("xml_path", "xml_sha256"):
            if not isinstance(case["input"].get(key), str) or not case["input"][key]:
                raise ValueError(f"expected input {key} is missing")
        for key in ("context", "active_skill_set_id", "selected_player", "selected_minion"):
            if key not in case["binding"]:
                raise ValueError(f"missing binding {key}")
    if (not isinstance(manifest.get("source"), dict)
            or not isinstance(manifest["source"].get("corpus_input_sha256"), str)
            or not manifest["source"]["corpus_input_sha256"]):
        raise ValueError("expected corpus input identity is missing")
    for key in ("absolute_tolerance", "relative_tolerance"):
        value = manifest["comparison"][key]
        if not finite_number(value) or value < 0:
            raise ValueError("comparison tolerances must be finite and nonnegative")


def compare_report(case, report, comparison, finite_tolerance=False):
    """Return every independently checkable mismatch, retaining the required row set."""
    issues = []
    if (type(report.get("schema_version")) is not int
            or report["schema_version"] != case["reference"]["report_schema_version"]):
        issues.append("report schema differs")
    if report.get("status") != case["reference"]["report_status"]:
        issues.append("report status differs")
    source = report.get("source")
    if (not isinstance(source, dict)
            or source.get("xml_sha256") != case["input"]["xml_sha256"]):
        issues.append("report source XML differs")
    evaluation = report.get("evaluation")
    if not isinstance(evaluation, dict):
        return issues + ["evaluation is missing"], 0
    if (not isinstance(evaluation.get("backend"), dict)
            or not isinstance(evaluation["backend"].get("id"), str)
            or not evaluation["backend"]["id"]):
        issues.append("backend identity is missing")
    if "context" not in evaluation:
        issues.append("binding is missing: context")
    coverage = evaluation.get("coverage", {})
    if not isinstance(coverage, dict):
        coverage = {}
    for key in ("active_skill_set_id", "selected_player", "selected_minion"):
        if key not in coverage:
            issues.append(f"binding is missing: {key}")
    observed = {"context": evaluation.get("context"),
                **{k: coverage.get(k) for k in
                   ("active_skill_set_id", "selected_player", "selected_minion")}}
    for key, value in observed.items():
        if not exact(value, case["binding"][key]):
            issues.append(f"binding differs: {key}")
    try:
        actual = measurement_map(evaluation.get("measurements"))
    except ValueError as error:
        return issues + [str(error)], 0
    expected = measurement_map(case["measurements"])
    for key in sorted(expected.keys() - actual.keys()):
        issues.append(f"missing metric: {key}")
    for key in sorted(actual.keys() - expected.keys()):
        issues.append(f"unexpected metric: {key}")
    matched = 0
    for key in sorted(expected.keys() & actual.keys()):
        wanted, got = expected[key], actual[key]
        values_match = exact(wanted["value"], got["value"])
        if finite_tolerance and wanted["value"]["status"] == got["value"]["status"] == "finite":
            reference, value = wanted["value"]["value"], got["value"]["value"]
            tolerance = max(comparison["absolute_tolerance"],
                            abs(reference) * comparison["relative_tolerance"])
            if not math.isfinite(tolerance):
                raise ValueError("finite comparison tolerance overflowed")
            values_match = abs(value - reference) <= tolerance
        # Availability is semantic; source wording is retained but not a native API contract.
        if wanted["value"]["status"] == got["value"]["status"] == "unavailable":
            values_match = True
        if (not exact(wanted["unit"], got["unit"])
                or not exact(wanted["schema_version"], got["schema_version"])
                or not values_match):
            issues.append(f"measurement differs: {key}")
        else:
            matched += 1
    return issues, matched


def inside(root, relative):
    if not isinstance(relative, str) or not relative:
        raise ValueError("missing corpus artifact path")
    path = (root / relative).resolve()
    if not path.is_relative_to(root.resolve()):
        raise ValueError("corpus artifact path escapes its result directory")
    return path


def compare_files(manifest_path, corpus_path, backend, finite_tolerance=False):
    manifest = _INTAKE.bounded_json(manifest_path)
    validate_manifest(manifest)
    corpus = _INTAKE.bounded_json(corpus_path)
    if type(corpus.get("schema_version")) is not int or corpus["schema_version"] != 5:
        raise ValueError("unsupported corpus index schema")
    entries = {}
    if not isinstance(corpus.get("entries"), list):
        raise ValueError("corpus entries must be an array")
    for entry in corpus["entries"]:
        if not isinstance(entry, dict):
            raise ValueError("expected corpus entry object")
        line = entry.get("line")
        if type(line) is not int or line < 1 or line in entries:
            raise ValueError("invalid or duplicate observed source line")
        entries[line] = entry
    global_issues = []
    provenance = corpus.get("provenance")
    if not isinstance(provenance, dict) or not isinstance(provenance.get("input"), dict):
        raise ValueError("corpus input provenance is missing")
    if (provenance["input"].get("sha256")
            != manifest["source"]["corpus_input_sha256"]):
        global_issues.append("corpus input identity differs")
    extra = sorted(entries.keys() - {c["source_line"] for c in manifest["cases"]})
    if extra:
        global_issues.append(f"untracked source lines: {extra}")
    results = []
    for case in manifest["cases"]:
        result = {"id": case["id"], "source_line": case["source_line"],
                  "status": "blocked", "required_measurements": len(case["measurements"]),
                  "matched_measurements": 0, "issues": list(global_issues)}
        results.append(result)
        entry = entries.get(case["source_line"])
        if entry is None:
            result["issues"].append("source line is missing")
            continue
        evaluations = entry.get("evaluations", {})
        invocation = evaluations.get(backend) if isinstance(evaluations, dict) else None
        if (not isinstance(invocation, dict) or invocation.get("status") != "success"
                or type(invocation.get("exit_code")) is not int or invocation["exit_code"] != 0):
            result["issues"].append("backend did not produce a successful report")
            result["invocation_status"] = invocation.get("status") if isinstance(invocation, dict) else "not_run"
            continue
        try:
            fixture = (manifest_path.parent / case["input"]["xml_path"]).resolve()
            if _INTAKE.digest(fixture) != case["input"]["xml_sha256"]:
                raise ValueError("frozen expectation XML changed")
            directory = inside(corpus_path.parent, entry["directory"])
            source = inside(directory, "imported.xml")
            if _INTAKE.digest(source) != case["input"]["xml_sha256"]:
                raise ValueError("observed input XML differs")
            output = inside(directory, invocation["stdout"])
            if _INTAKE.digest(output) != invocation.get("stdout_sha256"):
                raise ValueError("report differs from its recorded digest")
            report = _INTAKE.bounded_json(output)
            issues, count = compare_report(case, report, manifest["comparison"], finite_tolerance)
            result["issues"].extend(issues)
            result["matched_measurements"] = count
            evaluation = report.get("evaluation")
            if not isinstance(evaluation, dict):
                evaluation = {}
            result["observed_backend"] = evaluation.get("backend")
            result["reference_backend"] = case["reference"].get("backend")
            result["same_backend_identity"] = exact(result["observed_backend"], result["reference_backend"])
            result["observed_diagnostic_only"] = evaluation.get("diagnostic_only")
            result["report_sha256"] = _INTAKE.digest(output)
            result["reference_report_sha256"] = case["reference"].get("report_sha256")
            result["same_reference_report"] = result["report_sha256"] == result["reference_report_sha256"]
            result["status"] = "mismatch" if result["issues"] else "matched"
        except (OSError, ValueError, KeyError, TypeError) as error:
            result["issues"].append(str(error))
    return {"schema_version": 1, "scope": "diagnostic_reference_observation_comparison",
            "whole_build_parity": "not_established", "native_build_completion": "not_assessed",
            "source_only_resources": "not_compared_by_public_metric_check",
            "comparison_mode": "finite_tolerance" if finite_tolerance else "exact_numbers",
            "backend_label": backend, "expectations_sha256": _INTAKE.digest(manifest_path),
            "corpus_index_sha256": _INTAKE.digest(corpus_path),
            "required_cases": len(results), "matched_cases": sum(r["status"] == "matched" for r in results),
            "global_issues": global_issues, "cases": results}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--expectations", required=True, type=Path)
    parser.add_argument("--corpus", required=True, type=Path, help="Existing corpus index.json")
    parser.add_argument("--backend", required=True, help="Backend label recorded by the intake runner")
    parser.add_argument("--output", required=True, type=Path, help="New comparison JSON file")
    parser.add_argument("--finite-tolerance", action="store_true")
    args = parser.parse_args(argv)
    result = compare_files(args.expectations, args.corpus, args.backend, args.finite_tolerance)
    _INTAKE.write_json(args.output, result)
    return 0 if result["matched_cases"] == result["required_cases"] and not result["global_issues"] else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Expectation comparison failed: {error}", file=sys.stderr)
        raise SystemExit(2)
