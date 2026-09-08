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


if __name__ == "__main__":
    unittest.main()
