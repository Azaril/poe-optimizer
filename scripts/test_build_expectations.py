"""Independent synthetic contracts for recorded build expectations; no evaluator runs."""
import copy
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
SPEC = importlib.util.spec_from_file_location(
    "build_expectations", Path(__file__).with_name("check-build-expectations.py"))
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)


def digest(value):
    return hashlib.sha256(value).hexdigest()


def measurement(identity, value, actor="player", unit="count"):
    return {"query": {"actor": actor, "id": identity}, "schema_version": 1,
            "unit": unit, "value": value}


def sample_case(line=1):
    xml = f"<PathOfBuilding2 fixture='{line}'/>".encode()
    return {"id": f"case-{line}", "source_line": line,
            "input": {"xml_path": f"case-{line}.xml", "xml_sha256": digest(xml)},
            "reference": {"report_schema_version": 1, "report_status": "success",
                          "backend": {"id": "source-oracle", "version": "fixture"}},
            "binding": {"context": {"enemy_level": 82, "encounter": "pinnacle"},
                        "active_skill_set_id": 7,
                        "selected_player": {"skill_id": "caller-skill", "part": 2},
                        "selected_minion": None},
            "measurements": [
                measurement("life", {"status": "finite", "value": 100.0}),
                measurement("ehp", {"status": "non_finite", "kind": "positive_infinity"}),
                measurement("damage", {"status": "unavailable", "reason": "original wording"})]}


def sample_report(case):
    return {"schema_version": 1, "status": "success",
            "source": {"xml_sha256": case["input"]["xml_sha256"]},
            "evaluation": {"backend": copy.deepcopy(case["reference"]["backend"]),
                           "diagnostic_only": True,
                           "context": copy.deepcopy(case["binding"]["context"]),
                           "coverage": {key: copy.deepcopy(case["binding"][key]) for key in
                                        ("active_skill_set_id", "selected_player", "selected_minion")},
                           "measurements": copy.deepcopy(case["measurements"])}}


COMPARISON = {"absolute_tolerance": 0.001, "relative_tolerance": 0.0001}


class ReportContracts(unittest.TestCase):
    def setUp(self):
        self.case = sample_case()
        self.report = sample_report(self.case)

    def compare(self, tolerance=False):
        return CHECK.compare_report(self.case, self.report, COMPARISON, tolerance)

    def test_exact_report_matches_independent_metric_order(self):
        self.report["evaluation"]["measurements"].reverse()
        self.assertEqual(self.compare(), ([], 3))

    def test_missing_and_unexpected_queries_do_not_shrink_required_set(self):
        rows = self.report["evaluation"]["measurements"]
        rows.pop()
        rows.append(measurement("unrequested", {"status": "finite", "value": 1}))
        issues, count = self.compare()
        self.assertEqual(count, 2)
        self.assertTrue(any("missing metric" in value for value in issues), issues)
        self.assertTrue(any("unexpected metric" in value for value in issues), issues)
        self.assertEqual(len(self.case["measurements"]), 3)

    def test_duplicate_query_is_invalid_even_when_values_identical(self):
        rows = self.report["evaluation"]["measurements"]
        rows.append(copy.deepcopy(rows[0]))
        issues, count = self.compare()
        self.assertEqual(count, 0)
        self.assertTrue(any("duplicate metric" in value for value in issues), issues)

    def test_actor_is_part_of_query_identity(self):
        self.report["evaluation"]["measurements"][0]["query"]["actor"] = "minion"
        issues, count = self.compare()
        self.assertEqual(count, 2)
        self.assertEqual(sum("metric:" in issue for issue in issues), 2)

    def test_finite_tolerance_requires_explicit_opt_in(self):
        self.report["evaluation"]["measurements"][0]["value"]["value"] = 100.005
        self.assertTrue(self.compare()[0])
        self.assertEqual(self.compare(True), ([], 3))
        self.report["evaluation"]["measurements"][0]["value"]["value"] = 100.02
        self.assertTrue(self.compare(True)[0])

    def test_default_exact_preserves_negative_zero_and_large_integer_identity(self):
        for wanted, got in [(0.0, -0.0), (9007199254740993, 9007199254740992)]:
            with self.subTest(wanted=wanted, got=got):
                self.case["measurements"][0]["value"]["value"] = wanted
                self.report["evaluation"]["measurements"][0]["value"]["value"] = got
                self.assertTrue(self.compare()[0])

    def test_unavailable_prose_may_change_but_status_and_infinity_kind_may_not(self):
        rows = self.report["evaluation"]["measurements"]
        rows[2]["value"]["reason"] = "new detailed diagnostic wording"
        self.assertEqual(self.compare(True), ([], 3))
        for value in [{"status": "finite", "value": 0},
                      {"status": "non_finite", "kind": "not_a_number"}]:
            with self.subTest(value=value):
                rows[2]["value"] = value
                self.assertTrue(self.compare(True)[0])
        rows[2] = copy.deepcopy(self.case["measurements"][2])
        rows[1]["value"]["kind"] = "negative_infinity"
        self.assertTrue(self.compare(True)[0])

    def test_malformed_measurements_are_rejected_not_counted(self):
        invalid = [True, math.inf, math.nan, 10**400]
        for number in invalid:
            with self.subTest(number=str(number)[:30]):
                self.report["evaluation"]["measurements"][0]["value"]["value"] = number
                issues, count = self.compare()
                self.assertTrue(issues)
                self.assertEqual(count, 0)
        self.report = sample_report(self.case)
        for rows in [None, [], {}, [None]]:
            with self.subTest(rows=rows):
                self.report["evaluation"]["measurements"] = rows
                self.assertEqual(self.compare()[1], 0)
                self.assertTrue(self.compare()[0])

    def test_units_and_metric_schema_cannot_be_hidden_by_tolerance(self):
        for field, value in [("unit", "percent"), ("schema_version", True), ("schema_version", 2)]:
            with self.subTest(field=field):
                self.report = sample_report(self.case)
                self.report["evaluation"]["measurements"][0][field] = value
                self.assertTrue(self.compare(True)[0])

    def test_source_context_and_each_selector_are_bound(self):
        self.report["source"]["xml_sha256"] = "0" * 64
        self.assertIn("report source XML differs", self.compare()[0])
        for field, value in [("context", {"enemy_level": 81, "encounter": "pinnacle"}),
                             ("active_skill_set_id", 8),
                             ("selected_player", {"skill_id": "caller-skill", "part": 1}),
                             ("selected_minion", {"skill_id": "another-actor"})]:
            with self.subTest(field=field):
                self.report = sample_report(self.case)
                target = self.report["evaluation"] if field == "context" else self.report["evaluation"]["coverage"]
                target[field] = value
                self.assertIn(f"binding differs: {field}", self.compare()[0])

    def test_missing_explicit_null_selector_does_not_equal_present_null(self):
        del self.report["evaluation"]["coverage"]["selected_minion"]
        self.assertTrue(self.compare()[0])

    def test_report_schema_status_and_backend_identity_are_required(self):
        for field, value in [("schema_version", True), ("schema_version", 1.0),
                             ("status", "failed"), ("status", None)]:
            with self.subTest(field=field, value=value):
                self.report = sample_report(self.case)
                self.report[field] = value
                self.assertTrue(self.compare()[0])
        for backend in [None, {}, {"id": True}, {"id": 7}, {"id": ""}]:
            with self.subTest(backend=backend):
                self.report = sample_report(self.case)
                self.report["evaluation"]["backend"] = backend
                self.assertTrue(self.compare()[0])

    def test_different_backend_is_recorded_comparison_not_an_identity_requirement(self):
        self.report["evaluation"]["backend"] = {"id": "native-caller", "version": "different"}
        self.assertEqual(self.compare(), ([], 3))

    def test_missing_evaluation_and_malformed_source_have_controlled_diagnostics(self):
        for field, value in [("evaluation", None), ("source", None), ("source", [])]:
            with self.subTest(field=field, value=value):
                self.report = sample_report(self.case)
                self.report[field] = value
                self.assertTrue(self.compare()[0])


class FileContracts(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.expected_dir = self.root / "expectations"
        self.results = self.root / "results"
        self.expected_dir.mkdir()
        self.results.mkdir()
        self.manifest_path = self.expected_dir / "expected.json"
        self.corpus_path = self.results / "index.json"
        self.manifest = {"schema_version": 1, "scope": "diagnostic_reference_expectations",
                         "whole_build_parity": "not_established",
                         "source": {"corpus_input_sha256": digest(b"caller-input\n")},
                         "comparison": copy.deepcopy(COMPARISON),
                         "cases": [sample_case(1), sample_case(2)]}
        self.corpus = {"schema_version": 5, "provenance": {
            "input": {"sha256": self.manifest["source"]["corpus_input_sha256"]}}, "entries": []}
        for case in self.manifest["cases"]:
            line = case["source_line"]
            xml = f"<PathOfBuilding2 fixture='{line}'/>".encode()
            (self.expected_dir / case["input"]["xml_path"]).write_bytes(xml)
            directory = self.results / f"line-{line}"
            directory.mkdir()
            (directory / "imported.xml").write_bytes(xml)
            self.corpus["entries"].append({"line": line, "directory": directory.name,
                "evaluations": {"recorded": {"status": "success", "exit_code": 0,
                                             "stdout": "report.json"}}})
            self.set_report(line - 1, sample_report(case))
        self.save()

    def write_json(self, path, value):
        path.write_text(json.dumps(value, allow_nan=False) + "\n", encoding="utf8")

    def set_report(self, index, report, rehash=True):
        entry = self.corpus["entries"][index]
        path = self.results / entry["directory"] / "report.json"
        self.write_json(path, report)
        if rehash:
            entry["evaluations"]["recorded"]["stdout_sha256"] = digest(path.read_bytes())

    def save(self):
        self.write_json(self.manifest_path, self.manifest)
        self.write_json(self.corpus_path, self.corpus)

    def compare(self, backend="recorded", tolerance=False):
        self.save()
        return CHECK.compare_files(self.manifest_path, self.corpus_path, backend, tolerance)

    def args(self, output):
        return ["--expectations", str(self.manifest_path), "--corpus", str(self.corpus_path),
                "--backend", "recorded", "--output", str(output)]

    def test_complete_artifacts_match_without_establishing_build_or_native_parity(self):
        with patch.object(CHECK._INTAKE, "invoke", side_effect=AssertionError("backend invoked")):
            result = self.compare()
        self.assertEqual((result["required_cases"], result["matched_cases"]), (2, 2))
        self.assertEqual([row["required_measurements"] for row in result["cases"]], [3, 3])
        self.assertEqual(result["whole_build_parity"], "not_established")
        self.assertEqual(result["native_build_completion"], "not_assessed")
        self.assertEqual(result["source_only_resources"], "not_compared_by_public_metric_check")

    def test_missing_source_line_retains_full_denominator(self):
        self.corpus["entries"].pop()
        result = self.compare()
        self.assertEqual((result["required_cases"], result["matched_cases"]), (2, 1))
        self.assertEqual(result["cases"][1]["status"], "blocked")
        self.assertEqual(result["cases"][1]["required_measurements"], 3)

    def test_failed_missing_and_nonzero_backend_cannot_disappear_from_denominator(self):
        for invocation in [None, {"status": "failed", "exit_code": 1},
                           {"status": "timed_out"}, {"status": "success", "exit_code": 1},
                           {"status": "success", "exit_code": False},
                           {"status": "success", "exit_code": 0.0}]:
            with self.subTest(invocation=invocation):
                self.corpus["entries"][1]["evaluations"]["recorded"] = invocation
                result = self.compare()
                self.assertEqual((result["required_cases"], result["matched_cases"]), (2, 1))
                self.assertEqual(result["cases"][1]["status"], "blocked")
        result = self.compare(backend="not-recorded")
        self.assertEqual((result["required_cases"], result["matched_cases"]), (2, 0))

    def test_truthy_malformed_backend_record_is_blocked_not_an_uncaught_attribute_error(self):
        for invocation in [True, "invalid", [1]]:
            with self.subTest(invocation=invocation):
                self.corpus["entries"][1]["evaluations"]["recorded"] = invocation
                result = self.compare()
                self.assertEqual((result["required_cases"], result["matched_cases"]), (2, 1))

    def test_empty_denominator_duplicate_case_ids_and_expected_lines_are_invalid(self):
        original = copy.deepcopy(self.manifest)
        variants = [[], [copy.deepcopy(original["cases"][0])] * 2]
        other = copy.deepcopy(original["cases"])
        other[1]["source_line"] = other[0]["source_line"]
        variants.append(other)
        for cases in variants:
            with self.subTest(cases=cases):
                self.manifest["cases"] = cases
                with self.assertRaises(ValueError):
                    self.compare()

    def test_duplicate_observed_lines_and_noninteger_schemas_are_invalid(self):
        self.corpus["entries"][1]["line"] = 1
        with self.assertRaises(ValueError):
            self.compare()
        self.corpus["entries"][1]["line"] = 2
        for value in [True, 1.0]:
            with self.subTest(manifest_schema=value):
                self.manifest["schema_version"] = value
                with self.assertRaises(ValueError):
                    self.compare()
        self.manifest["schema_version"] = 1
        self.corpus["schema_version"] = 5.0
        with self.assertRaises(ValueError):
            self.compare()

    def test_extra_source_line_is_reported_and_prevents_any_matched_case(self):
        extra = copy.deepcopy(self.corpus["entries"][0])
        extra["line"] = 9
        self.corpus["entries"].append(extra)
        result = self.compare()
        self.assertEqual(result["required_cases"], 2)
        self.assertEqual(result["matched_cases"], 0)
        self.assertTrue(result["global_issues"])

    def test_changed_corpus_and_xml_identities_cannot_match(self):
        self.corpus["provenance"]["input"]["sha256"] = "f" * 64
        self.assertEqual(self.compare()["matched_cases"], 0)
        self.corpus["provenance"]["input"]["sha256"] = self.manifest["source"]["corpus_input_sha256"]
        (self.results / "line-1" / "imported.xml").write_bytes(b"changed source")
        result = self.compare()
        self.assertEqual(result["matched_cases"], 1)
        self.assertTrue(any("input XML differs" in issue for issue in result["cases"][0]["issues"]))

    def test_changed_frozen_fixture_and_mutated_report_digest_are_blocked(self):
        (self.expected_dir / "case-2.xml").write_bytes(b"changed frozen fixture")
        report = sample_report(self.manifest["cases"][0])
        report["evaluation"]["measurements"][0]["value"]["value"] = 999
        self.set_report(0, report, rehash=False)
        result = self.compare()
        self.assertEqual(result["matched_cases"], 0)
        self.assertTrue(any("recorded digest" in issue for issue in result["cases"][0]["issues"]))
        self.assertTrue(any("expectation XML changed" in issue for issue in result["cases"][1]["issues"]))

    def test_output_and_directory_traversal_are_blocked_before_outside_artifacts(self):
        original = copy.deepcopy(self.corpus)
        for target, path in [("directory", ".."), ("directory", str(self.expected_dir)),
                             ("stdout", "../../outside.json"),
                             ("stdout", str(self.expected_dir / "expected.json"))]:
            with self.subTest(target=target, path=path):
                self.corpus = copy.deepcopy(original)
                entry = self.corpus["entries"][0]
                container = entry if target == "directory" else entry["evaluations"]["recorded"]
                container[target] = path
                result = self.compare()
                self.assertEqual(result["matched_cases"], 1)
                self.assertTrue(any("escapes" in issue for issue in result["cases"][0]["issues"]))

    def test_missing_or_nonobject_evaluation_keeps_a_failed_case_in_the_report(self):
        for evaluation in [None, [], "invalid"]:
            with self.subTest(evaluation=evaluation):
                report = sample_report(self.manifest["cases"][0])
                report["evaluation"] = evaluation
                self.set_report(0, report)
                result = self.compare()
                self.assertEqual((result["required_cases"], result["matched_cases"]), (2, 1))
                self.assertEqual(result["cases"][0]["matched_measurements"], 0)
                self.assertTrue(result["cases"][0]["issues"])

    def test_duplicate_expected_metrics_and_invalid_tolerances_reject_manifest(self):
        original = copy.deepcopy(self.manifest)
        rows = self.manifest["cases"][0]["measurements"]
        rows.append(copy.deepcopy(rows[0]))
        with self.assertRaises(ValueError):
            self.compare()
        for value in [True, -1, 10**400]:
            with self.subTest(tolerance=str(value)[:30]):
                self.manifest = copy.deepcopy(original)
                self.manifest["comparison"]["relative_tolerance"] = value
                with self.assertRaises(ValueError):
                    self.compare()

    def test_tolerance_overflow_is_blocked_instead_of_accepting_infinite_threshold(self):
        self.manifest["comparison"]["relative_tolerance"] = 1e308
        result = self.compare(tolerance=True)
        self.assertEqual((result["required_cases"], result["matched_cases"]), (2, 0))
        self.assertTrue(all(any("overflow" in issue for issue in row["issues"])
                            for row in result["cases"]))

    def test_changed_backend_metadata_is_disclosed_without_blocking_cross_backend_values(self):
        report = sample_report(self.manifest["cases"][0])
        report["evaluation"]["backend"] = {"id": "native-caller", "version": "other"}
        self.set_report(0, report)
        result = self.compare()
        self.assertEqual(result["matched_cases"], 2)
        self.assertFalse(result["cases"][0]["same_backend_identity"])
        self.assertEqual(result["cases"][0]["observed_backend"]["id"], "native-caller")

    def test_main_returns_mismatch_and_writes_every_case(self):
        self.corpus["entries"].pop()
        self.save()
        output = self.root / "comparison.json"
        self.assertEqual(CHECK.main(self.args(output)), 1)
        result = json.loads(output.read_text(encoding="utf8"))
        self.assertEqual(len(result["cases"]), 2)

    def test_main_exact_default_and_explicit_tolerance(self):
        report = sample_report(self.manifest["cases"][0])
        report["evaluation"]["measurements"][0]["value"]["value"] = 100.005
        self.set_report(0, report)
        self.save()
        exact = self.root / "exact.json"
        tolerant = self.root / "tolerant.json"
        self.assertEqual(CHECK.main(self.args(exact)), 1)
        self.assertEqual(CHECK.main(self.args(tolerant) + ["--finite-tolerance"]), 0)
        self.assertEqual(json.loads(exact.read_text(encoding="utf8"))["comparison_mode"], "exact_numbers")
        self.assertEqual(json.loads(tolerant.read_text(encoding="utf8"))["comparison_mode"], "finite_tolerance")

    def test_main_never_overwrites_existing_output_or_input(self):
        output = self.root / "existing.json"
        output.write_bytes(b"caller-owned bytes")
        for path in [output, self.manifest_path, self.corpus_path]:
            with self.subTest(path=path):
                before = path.read_bytes()
                with self.assertRaises(FileExistsError):
                    CHECK.main(self.args(path))
                self.assertEqual(path.read_bytes(), before)

    def test_cli_malformed_json_uses_controlled_error_exit_and_creates_no_output(self):
        self.manifest_path.write_text('{"schema_version":1,"schema_version":1}', encoding="utf8")
        output = self.root / "absent.json"
        result = subprocess.run([sys.executable, "-B", str(Path(CHECK.__file__)), *self.args(output)],
                                capture_output=True, text=True, check=False, timeout=10)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("duplicate JSON key", result.stderr)
        self.assertNotIn("Traceback", result.stderr)
        self.assertFalse(output.exists())


class TrackedManifestContract(unittest.TestCase):
    def test_every_declared_fixture_matches_its_frozen_repository_bytes(self):
        repository = Path(__file__).resolve().parents[1]
        manifest_path = repository / "tests/fixtures/breadth-expectations/originals-v1.json"
        manifest = CHECK._INTAKE.bounded_json(manifest_path)
        CHECK.validate_manifest(manifest)
        fixture_root = (repository / "tests/fixtures").resolve()
        # Iterate declarations; no production build IDs, numerical outputs, or
        # ignored corpus-run paths are copied into this test's expectations.
        for case in manifest["cases"]:
            with self.subTest(case=case["id"]):
                fixture = (manifest_path.parent / case["input"]["xml_path"]).resolve()
                self.assertTrue(fixture.is_relative_to(fixture_root))
                self.assertTrue(fixture.is_file())
                self.assertEqual(digest(fixture.read_bytes()), case["input"]["xml_sha256"])


if __name__ == "__main__":
    unittest.main()
