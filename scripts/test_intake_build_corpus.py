"""Standard-library contract checks for caller-controlled corpus observations."""
import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
SPEC = importlib.util.spec_from_file_location("intake", Path(__file__).with_name("intake-build-corpus.py"))
INTAKE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(INTAKE)


def configuration_report(xml):
    xml_hash = hashlib.sha256(xml).hexdigest()
    return {"schema_version": 1, "scope": "configuration_source_projection_v1",
            "status": "source_projected",
            "input": {"format": "raw_xml", "input_sha256": xml_hash,
                      "xml_sha256": xml_hash, "input_bytes": len(xml), "xml_bytes": len(xml)},
            "verification": {"effective_configuration": "not_evaluated",
                             "game_mechanics": "not_evaluated", "build_legality": "not_checked",
                             "reference_calculation": "not_run"},
            "configuration": {"schema_version": 1, "interpretation": "authored_configuration_source",
                              "effective_configuration": "not_evaluated", "mechanics": "not_evaluated",
                              "game_legality": "not_evaluated",
                              "projection": {"source_sha256": xml_hash, "layout": "empty",
                                             "requested_active_set_id": None, "active_set_id": 1,
                                             "active_set_resolution": "default_one",
                                             "sets": [{"id": 1, "inputs": [], "placeholders": [],
                                                       "blocks": [], "unknown_records": []}]}}}


class CorpusTests(unittest.TestCase):
    def test_lines_preserve_offsets_bom_blank_and_mixed_endings(self):
        raw = b"\xef\xbb\xbfone\r\n\n two \nlast"
        rows = INTAKE.line_records(raw)
        self.assertEqual([row[0] for row in rows], [1, 2, 3, 4])
        self.assertEqual(b"".join(row[2] for row in rows), raw)
        for _, offset, line in rows:
            self.assertEqual(raw[offset:offset + len(line)], line)
        self.assertEqual(INTAKE.line_records(b""), [])
        self.assertEqual(INTAKE.line_records(b"x\n"), [(1, 0, b"x\n")])

    def test_line_bound_is_enforced_during_scanning(self):
        with patch.object(INTAKE, "MAX_LINES", 3):
            self.assertEqual(len(INTAKE.line_records(b"\n\n\n")), 3)
            with self.assertRaisesRegex(ValueError, "3-line bound"):
                INTAKE.line_records(b"\n" * 100_000)

    def test_xml_summary_preserves_set_ownership_and_configuration_inputs(self):
        with tempfile.TemporaryDirectory() as name:
            path = Path(name) / "build.xml"
            source = b'''<PathOfBuilding2>
                <Skills activeSkillSet="2">
                  <Skill label="Legacy"><Gem skillId="legacy"/></Skill>
                  <SkillSet id="1" title="First"><Skill enabled="true"><Gem skillId="one"/></Skill></SkillSet>
                  <SkillSet id="2" title="Second"><Skill enabled="false"><Gem skillId="two"/></Skill></SkillSet>
                </Skills>
                <Config activeConfigSet="2"><Input name="legacy" number="7"/>
                  <ConfigSet id="1"><Input name="enemyLevel" number="80"/></ConfigSet>
                  <ConfigSet id="2"><Input name="enemyLevel" number="90"/>
                    <CustomModifierBlock title="Notes" enabled="false">unmodeled text</CustomModifierBlock>
                  </ConfigSet>
                </Config>
            </PathOfBuilding2>'''
            path.write_bytes(source)
            result = INTAKE.xml_summary(path)
            self.assertEqual([node["attributes"]["id"] for node in result["skills"]["sets"]], ["1", "2"])
            self.assertEqual(result["skills"]["sets"][1]["groups"][0]["gems"][0]["skillId"], "two")
            self.assertEqual(result["skills"]["legacy_groups"][0]["attributes"]["label"], "Legacy")
            self.assertEqual(result["config"]["sets"][1]["inputs"], [{"name": "enemyLevel", "number": "90"}])
            block = result["config"]["sets"][1]["custom_modifier_blocks"][0]
            self.assertEqual(block["attributes"], {"title": "Notes", "enabled": "false"})
            self.assertEqual(block["text_sha256"], hashlib.sha256(b"unmodeled text").hexdigest())
            self.assertEqual(path.read_bytes(), source)

    def test_strict_json_and_read_bounds(self):
        with tempfile.TemporaryDirectory() as name:
            path = Path(name) / "input"
            for content in [b'{"x":NaN}', b'{"x":1,"x":2}', b'{"x":1e400}', b'[]', b'{']:
                path.write_bytes(content)
                with self.assertRaises(ValueError):
                    INTAKE.bounded_json(path)
            path.write_bytes(b'{"x":1}')
            self.assertEqual(INTAKE.bounded_json(path), {"x": 1})
            with self.assertRaises(ValueError):
                INTAKE.read_bounded(path, 2)
            self.assertEqual(INTAKE.read_bounded(path, 7), path.read_bytes())

    def test_invocations_keep_failure_timeout_and_deadline_independent(self):
        with tempfile.TemporaryDirectory() as name:
            path = Path(name)
            failed = INTAKE.invoke([sys.executable, "-c", "import sys; print('failure'); sys.exit(7)"],
                                   path, "failed", path, time.monotonic() + 30, 5)
            self.assertEqual(failed["status"], "process_error")
            self.assertEqual(failed["exit_code"], 7)
            timed = INTAKE.invoke([sys.executable, "-c", "import time; time.sleep(30)"],
                                  path, "timed", path, time.monotonic() + 30, 0.2)
            self.assertEqual(timed["status"], "timeout")
            expired = INTAKE.invoke([sys.executable, "-c", "raise Exception()"],
                                    path, "expired", path, time.monotonic() - 1, 5)
            self.assertEqual(expired["status"], "deadline_before_start")
            self.assertFalse((path / "expired.stdout").exists())
            good = INTAKE.invoke([sys.executable, "-c", "print('okay')"],
                                 path, "good", path, time.monotonic() + 30, 5)
            self.assertEqual(good["status"], "success")
            self.assertEqual(good["stdout_sha256"], INTAKE.digest(path / "good.stdout"))

    def test_entries_and_backend_failures_preserve_source_without_fallback(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            source = root / "caller.imports"
            raw = b"bad\r\ngood\n"
            source.write_bytes(raw)
            output = root / "new-observations"
            calls = []
            xml_bytes = b'<PathOfBuilding2><Build level="22"/><Tree activeSpec="1"><Spec nodes="7"/></Tree></PathOfBuilding2>'

            def fake_invoke(command, directory, prefix, workdir, deadline, timeout):
                calls.append((directory.name, prefix))
                stdout = directory / f"{prefix}.stdout"
                (directory / f"{prefix}.stderr").write_bytes(b"")
                if prefix == "import" and directory.name == "line-00001":
                    stdout.write_bytes(b"failed")
                    return {"status": "process_error", "exit_code": 1}
                if prefix == "import":
                    Path(command[command.index("--output") + 1]).write_bytes(xml_bytes)
                    value = {"xml_sha256": hashlib.sha256(xml_bytes).hexdigest()}
                else:
                    self.assertEqual(Path(command[2]).read_bytes(), xml_bytes)
                    Path(command[command.index("--export") + 1]).write_bytes(xml_bytes)
                    value = {"evaluation": {"backend": {"id": prefix}, "build": {"level": 22},
                                            "coverage": {} if prefix == "pob" else None,
                                            "measurements": []}}
                stdout.write_text(json.dumps(value), encoding="utf-8")
                return {"status": "success", "exit_code": 0}

            args = ["--input", str(source), "--output", str(output), "--import-cli", sys.executable,
                    "--backend", f"native={sys.executable}", "--backend", f"pob={sys.executable}",
                    "--pob", str(root), "--deadline-seconds", "30", "--jobs", "2"]
            with patch.object(INTAKE, "invoke", side_effect=fake_invoke), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(INTAKE.main(args), 1)
            index = json.loads((output / "index.json").read_bytes())
            self.assertEqual(source.read_bytes(), raw)
            self.assertEqual((output / "corpus.source").read_bytes(), raw)
            self.assertEqual((output / "line-00001/source.import").read_bytes(), b"bad\r\n")
            self.assertEqual(index["failed_entries"], 2)
            self.assertEqual(index["entries"][0]["status"], "import_failed")
            results = index["entries"][1]["evaluations"]
            self.assertEqual(results["native"]["status"], "evaluation_evidence_error")
            self.assertEqual(results["pob"]["status"], "success")
            self.assertNotIn(("line-00001", "native"), calls)
            self.assertEqual(index["changed_inputs"], [])
            with self.assertRaises(FileExistsError):
                INTAKE.main(args)

    def test_configuration_observations_remain_independent_and_validate_source_identity(self):
        for outcome in ["success", "process_error", "wrong_hash", "malformed", "malformed_records", "overclaim"]:
            with self.subTest(outcome=outcome), tempfile.TemporaryDirectory() as name:
                root = Path(name)
                source = root / "caller.imports"
                source.write_bytes(b"caller\n")
                output = root / "observations"
                xml = b"<PathOfBuilding2><Config/></PathOfBuilding2>"
                xml_hash = hashlib.sha256(xml).hexdigest()
                calls = []
                def fake_invoke(command, directory, prefix, workdir, deadline, timeout):
                    calls.append(prefix)
                    stdout = directory / f"{prefix}.stdout"
                    if prefix == "import":
                        Path(command[command.index("--output") + 1]).write_bytes(xml)
                        value = {"xml_sha256": xml_hash}
                    elif prefix == "configuration":
                        self.assertEqual(command[1], "inspect-configuration")
                        self.assertEqual(Path(command[2]).read_bytes(), xml)
                        if outcome == "process_error":
                            return {"status": "process_error", "exit_code": 1}
                        value = configuration_report(xml)
                        if outcome == "wrong_hash":
                            value["input"]["xml_sha256"] = "0" * 64
                        if outcome == "malformed":
                            value["configuration"] = []
                        if outcome == "malformed_records":
                            value["configuration"]["projection"]["sets"][0]["inputs"] = "not an array"
                        if outcome == "overclaim":
                            value["verification"]["effective_configuration"] = "verified"
                    else:
                        Path(command[command.index("--export") + 1]).write_bytes(xml)
                        value = {"evaluation": {"backend": {"id": prefix}, "build": {},
                                                "coverage": {}, "measurements": []}}
                    stdout.write_text(json.dumps(value), encoding="utf-8")
                    return {"status": "success", "exit_code": 0}
                args = ["--input", str(source), "--output", str(output), "--import-cli", sys.executable,
                        "--backend", f"native={sys.executable}", "--deadline-seconds", "30",
                        "--inspect-configuration"]
                with patch.object(INTAKE, "invoke", side_effect=fake_invoke), contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(INTAKE.main(args), 0 if outcome == "success" else 1)
                index = json.loads((output / "index.json").read_bytes())
                self.assertEqual(index["schema_version"], 2)
                self.assertTrue(index["configuration_inspection_requested"])
                entry = index["entries"][0]
                self.assertEqual(entry["status"], "imported")
                self.assertEqual(entry["evaluations"]["native"]["status"], "success")
                self.assertEqual(calls, ["import", "configuration", "native"])
                self.assertEqual(source.read_bytes(), b"caller\n")
                self.assertEqual(entry["configuration"]["status"], "success" if outcome == "success" else
                                 "process_error" if outcome == "process_error" else "configuration_evidence_error")


    def test_configuration_contract_rejects_wrong_types_and_unsupported_claims(self):
        xml = b"<PathOfBuilding2><Config/></PathOfBuilding2>"
        xml_hash = hashlib.sha256(xml).hexdigest()
        edits = [
            lambda r: r.update(schema_version=True),
            lambda r: r["input"].update(input_bytes=True),
            lambda r: r["input"].update(input_sha256="f" * 64),
            lambda r: r["configuration"].update(schema_version=2),
            lambda r: r["configuration"].update(interpretation="effective_configuration"),
            lambda r: r["configuration"].update(game_legality="verified"),
            lambda r: r.update(verification=[]),
            lambda r: r["verification"].pop("reference_calculation"),
            lambda r: r["configuration"]["projection"].update(layout="future"),
            lambda r: r["configuration"]["projection"].update(sets=[]),
            lambda r: r["configuration"]["projection"]["sets"][0].update(id=True),
            lambda r: r["configuration"]["projection"]["sets"][0].update(inputs={}),
            lambda r: r["configuration"]["projection"]["sets"][0].update(placeholders="bad"),
            lambda r: r["configuration"]["projection"]["sets"][0].update(blocks=[42]),
            lambda r: r["configuration"]["projection"]["sets"][0].update(unknown_records=None),
            lambda r: r["configuration"]["projection"].update(requested_active_set_id=False),
            lambda r: r["configuration"]["projection"].update(active_set_id=2),
            lambda r: r["configuration"]["projection"].update(active_set_resolution="requested"),
            lambda r: r["configuration"]["projection"]["sets"].append(r["configuration"]["projection"]["sets"][0].copy()),
        ]
        for edit in edits:
            report = configuration_report(xml)
            edit(report)
            with self.subTest(report=report), self.assertRaises((ValueError, TypeError, KeyError)):
                INTAKE.configuration_source_summary(report, xml_hash, len(xml))
        report = configuration_report(xml)
        projection = report["configuration"]["projection"]
        projection.update(layout="explicit_sets", requested_active_set_id=99,
                          active_set_id=2, active_set_resolution="missing_requested_uses_first")
        projection["sets"][0]["id"] = 2
        summary = INTAKE.configuration_source_summary(report, xml_hash, len(xml))
        self.assertEqual(summary["source_state"]["active_set_id"], 2)

    def test_changed_inspection_source_is_retained_and_never_sent_to_backends(self):
        for outcome in ["rewritten", "deleted", "rewritten_process_error"]:
            with self.subTest(outcome=outcome), tempfile.TemporaryDirectory() as name:
                root = Path(name)
                source = root / "caller.imports"
                source.write_bytes(b"caller\n")
                output = root / "observations"
                xml = b"<PathOfBuilding2><Config/></PathOfBuilding2>"
                changed = b"<PathOfBuilding2><Config><Input name='changed' number='1'/></Config></PathOfBuilding2>"
                calls = []
                def fake_invoke(command, directory, prefix, workdir, deadline, timeout):
                    calls.append(prefix)
                    if prefix == "import":
                        Path(command[command.index("--output") + 1]).write_bytes(xml)
                        value = {"xml_sha256": hashlib.sha256(xml).hexdigest()}
                    elif prefix == "configuration":
                        target = Path(command[2])
                        self.assertEqual(target.read_bytes(), xml)
                        if outcome == "deleted":
                            target.unlink()
                        else:
                            target.write_bytes(changed)
                        # A self-consistent report for the altered bytes must not
                        # replace the hash that was pinned before this invocation.
                        value = configuration_report(changed)
                    else:
                        self.fail("changed source must not reach any evaluation backend")
                    (directory / f"{prefix}.stdout").write_text(json.dumps(value), encoding="utf-8")
                    return {"status": "process_error" if outcome == "rewritten_process_error" and prefix == "configuration" else "success",
                            "exit_code": 1 if outcome == "rewritten_process_error" and prefix == "configuration" else 0}
                args = ["--input", str(source), "--output", str(output), "--import-cli", sys.executable,
                        "--backend", f"native={sys.executable}", "--backend", f"pob={sys.executable}",
                        "--pob", str(root), "--deadline-seconds", "30", "--inspect-configuration"]
                with patch.object(INTAKE, "invoke", side_effect=fake_invoke), contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(INTAKE.main(args), 1)
                index = json.loads((output / "index.json").read_bytes())
                entry = index["entries"][0]
                self.assertEqual(calls, ["import", "configuration"])
                self.assertEqual(entry["configuration"]["status"], "configuration_evidence_error")
                self.assertTrue(entry["configuration"]["source_changed"])
                self.assertEqual(entry["configuration"]["invocation_status"], "process_error" if outcome == "rewritten_process_error" else "success")
                self.assertEqual(entry["configuration"]["source_before_inspection_sha256"], hashlib.sha256(xml).hexdigest())
                for backend in ["native", "pob"]:
                    self.assertEqual(entry["evaluations"][backend]["status"], "source_changed")
                self.assertEqual(source.read_bytes(), b"caller\n")
                target = output / "line-00001/imported.xml"
                if outcome == "deleted":
                    self.assertFalse(target.exists())
                    self.assertIsNone(entry["configuration"]["source_after_inspection_sha256"])
                else:
                    self.assertEqual(target.read_bytes(), changed)
                    self.assertEqual(entry["configuration"]["source_after_inspection_sha256"], hashlib.sha256(changed).hexdigest())



if __name__ == "__main__":
    unittest.main()
