#!/usr/bin/env python3
"""Package exact recorded reference reports for offline producer contract tests.

This runs no evaluator, changes no report bytes, and makes no owned parity claim.
The caller supplies the fixed expectation manifest and the recorded corpus root.
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import re
import sys

sys.dont_write_bytecode = True
_SPEC = importlib.util.spec_from_file_location(
    "fixed_expectations", Path(__file__).with_name("check-build-expectations.py"))
_CHECK = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_CHECK)
MAX_REPORT_BYTES = 8 * 1024 * 1024
MAX_CASES = 1024
MAX_TOTAL_REPORT_BYTES = 64 * 1024 * 1024


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read(path, maximum):
    with path.open("rb") as stream:
        data = stream.read(maximum + 1)
    if len(data) > maximum:
        raise ValueError(f"{path} exceeds input bound")
    return data


def package(expectations: Path, corpus: Path):
    manifest_bytes = read(expectations, MAX_REPORT_BYTES)
    manifest = _CHECK._INTAKE.bounded_json(expectations)
    _CHECK.validate_manifest(manifest)
    if len(manifest["cases"]) > MAX_CASES:
        raise ValueError("case count exceeds bound")
    files, rows = {}, []
    total_report_bytes = 0
    for case in manifest["cases"]:
        name = case["id"]
        if re.fullmatch(r"[A-Za-z0-9_-]{1,128}", name) is None:
            raise ValueError("case ID is not a portable fixture filename")
        source_file = expectations.parent / case["input"]["xml_path"]
        if digest(read(source_file, MAX_REPORT_BYTES)) != case["input"]["xml_sha256"]:
            raise ValueError("protected source XML differs from fixed expectation")
        path = corpus / f"line-{case['source_line']:05d}" / "pob.stdout"
        raw = read(path, MAX_REPORT_BYTES)
        total_report_bytes += len(raw)
        if total_report_bytes > MAX_TOTAL_REPORT_BYTES:
            raise ValueError("aggregate report byte bound exceeded")
        if digest(raw) != case["reference"]["report_sha256"]:
            raise ValueError("recorded report differs from fixed expectation digest")
        report = _CHECK._INTAKE.bounded_json(path)
        issues, count = _CHECK.compare_report(case, report, manifest["comparison"])
        if issues or count != len(case["measurements"]):
            raise ValueError(f"fixed reference binding differs for {name}: {issues}")
        if report["evaluation"]["backend"] != case["reference"]["backend"]:
            raise ValueError("recorded backend identity differs")
        output = io.BytesIO()
        # GzipFile fixes the OS marker too; filename and clock never enter bytes.
        with gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=0,
                           compresslevel=9) as stream:
            stream.write(raw)
        compressed = output.getvalue()
        filename = name + ".report.json.gz"
        files[filename] = compressed
        rows.append({
            "id": name, "source_line": case["source_line"], "file": filename,
            "source_xml_sha256": case["input"]["xml_sha256"],
            "report_sha256": digest(raw), "report_bytes": len(raw),
            "gzip_sha256": digest(compressed), "gzip_bytes": len(compressed),
            "report_schema_version": report["schema_version"],
            "measurement_rows": len(report["evaluation"]["measurements"]),
        })
    artifact = {"schema_version": 1, "scope": "exact_recorded_reference_reports",
                "whole_build_parity": "not_established",
                "expectations_digest_format": "sha256_lf_normalized_bytes",
                "expectations_sha256": digest(manifest_bytes.replace(b"\r\n", b"\n")),
                "cases": rows}
    files["manifest.json"] = (json.dumps(artifact, indent=2) + "\n").encode()
    return files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--expectations", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    files = package(args.expectations, args.corpus)
    if args.check:
        for name, data in files.items():
            if read(args.output / name, MAX_REPORT_BYTES) != data:
                raise ValueError(f"recorded fixture differs: {name}")
    else:
        if args.output.exists():
            raise ValueError("new fixture destination must not already exist")
        args.output.mkdir(parents=True)
        for name, data in files.items():
            with (args.output / name).open("xb") as stream:
                stream.write(data)
    print(f"{len(files) - 1} exact reports; {sum(map(len, files.values()))} compressed bytes including manifest")


if __name__ == "__main__":
    main()
