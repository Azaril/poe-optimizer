"""Standard-library contract checks for caller-controlled corpus observations."""
import contextlib
import copy
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


BUILD_XML = b"<PathOfBuilding2><Skills><SkillSet id='9'><Skill label='caller'><Gem nameSpec='Caller'/></Skill></SkillSet></Skills></PathOfBuilding2>"
DATA_HASH = "a" * 64


def build_report(xml=BUILD_XML, definitions=False):
    def span(start, end):
        return {"start": start, "end": end}
    def node(tag, role, children):
        start = xml.index(("<" + tag).encode())
        opening_end = xml.index(b">", start) + 1
        end = (opening_end if xml[opening_end - 2:opening_end] == b"/>" else
               xml.index(("</" + tag + ">").encode(), start) + len(tag) + 3)
        return {"element": {"name": tag, "source_range": span(start, end)},
                "source_use": role, "children": children}
    gem = node("Gem", "gem_instance", [])
    group = node("Skill", "group", [gem])
    # '<Skill' is also a prefix of '<Skills' and '<SkillSet'. Use its unique spelling.
    group["element"]["source_range"]["start"] = xml.index(b"<Skill label")
    saved = node("SkillSet", "saved_set", [group])
    container = node("Skills", "container", [saved])
    config = configuration_report(xml)
    result = {**config, "scope": "build_source_projection_v1",
              "configuration": {"status": "source_projected", "projection": config["configuration"]},
              "build": {"source_sha256": config["input"]["xml_sha256"],
                        "root": {"source_range": span(0, len(xml))},
                        "sections": [{"kind": "skills", "element": container["element"]}]},
              "skills": {"status": "source_projected",
                         "projection": {"source_sha256": config["input"]["xml_sha256"],
                                        "containers": [container]}}}
    result["verification"].update(calculation_context="not_resolved", native_admission="not_checked")
    if definitions:
        data = {"content_sha256": DATA_HASH, "game": "poe2", "schema_version": 13}
        trust = {"status": "custom_unreviewed"}
        lookup = {"schema_version": 1, "source_xml_sha256": config["input"]["xml_sha256"],
                  "interpretation": "authored_identity_before_socket_group_processing",
                  "data": data, "data_trust": trust, "active_set_selection": "not_resolved",
                  "actor_resolution": "not_resolved", "name_matching": "not_run",
                  "socket_group_processing": "not_run", "game_mechanics": "not_evaluated",
                  "records": [{"container_index": 0,
                               "set_source_range": saved["element"]["source_range"],
                               "group_source_range": group["element"]["source_range"],
                               "record": {"kind": "instance", "lookup": {
                                   "source_range": gem["element"]["source_range"],
                                   "resolution": {"kind": "name_only_not_resolved"}}}}]}
        result["definition_lookup"] = {"data": data, "data_trust": trust,
            "game_mechanics": "not_evaluated", "skills": {"status": "looked_up", "lookup": lookup}}
    return result


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
                self.assertEqual(index["schema_version"], 3)
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


    def test_full_build_source_counts_keep_saved_occurrences_and_data_identity_separate(self):
        for definitions in [False, True]:
            report = build_report(definitions=definitions)
            summary = INTAKE.build_source_summary(report, hashlib.sha256(BUILD_XML).hexdigest(),
                                                  len(BUILD_XML), definitions, DATA_HASH if definitions else None)
            self.assertEqual(summary["sections"], {"skills": 1})
            self.assertEqual(summary["skill_source_counts"],
                             {"container": 1, "saved_set": 1, "group": 1, "gem_instance": 1})
            if definitions:
                self.assertEqual(summary["skill_identity_counts"], {"name_only_not_resolved": 1})
                self.assertEqual(summary["data_identity_check"], "selected_snapshot")
            else:
                self.assertNotIn("data", summary)
        report = build_report(definitions=True)
        report["configuration"] = {"status": "not_projected", "error": {"reason": "unknown scalar"}}
        summary = INTAKE.build_source_summary(report, hashlib.sha256(BUILD_XML).hexdigest(), len(BUILD_XML), True, None)
        self.assertEqual(summary["configuration_status"], "not_projected")
        self.assertEqual(summary["data_identity_check"], "reported_by_inspector")
        report["skills"] = {"status": "not_projected", "error": {"reason": "bound"}}
        report["definition_lookup"]["skills"] = {"status": "not_looked_up", "source_error": {"reason": "bound"}}
        summary = INTAKE.build_source_summary(report, hashlib.sha256(BUILD_XML).hexdigest(), len(BUILD_XML), True, None)
        self.assertEqual(summary["skill_identity_status"], "not_looked_up")
        self.assertNotIn("skill_identity_counts", summary)

    def test_nested_selectors_retain_duplicate_occurrences_without_becoming_gem_instances(self):
        selectors = [b"<StatSetIndex skillId='same' index='1'/>",
                     b"<StatSetIndex skillId='same' index='2'/>",
                     b"<MinionSkillIndexLookup skillId='missing'/>"]
        xml = BUILD_XML.replace(b"<Gem nameSpec='Caller'/>",
                                b"<Gem nameSpec='Caller'>" + b"".join(selectors) + b"</Gem>")
        report = build_report(xml, definitions=True)
        gem = report["skills"]["projection"]["containers"][0]["children"][0]["children"][0]["children"][0]
        records = report["definition_lookup"]["skills"]["lookup"]["records"]
        for index, source in enumerate(selectors):
            start = xml.index(source)
            bounds = {"start": start, "end": start + len(source)}
            role = "main_stat_set_selection" if index < 2 else "main_minion_lookup"
            gem["children"].append({"element": {"source_range": bounds},
                                    "source_use": role, "children": []})
            record = {**records[0], "record": {"kind": "effect_selection", "lookup": {
                "source_range": bounds, "source_use": role,
                "matched": {"effect_id": "same"} if index < 2 else None}}}
            records.append(record)
        xml_hash = hashlib.sha256(xml).hexdigest()
        summary = INTAKE.build_source_summary(report, xml_hash, len(xml), True, DATA_HASH)
        self.assertEqual(summary["skill_source_counts"]["gem_instance"], 1)
        self.assertEqual(summary["skill_identity_counts"], {"name_only_not_resolved": 1,
                         "effect_selection_resolved": 2, "effect_selection_unresolved": 1})
        # Same effect ID is still two authored occurrences; dropping either loses evidence.
        missing = copy.deepcopy(report)
        missing["definition_lookup"]["skills"]["lookup"]["records"].pop(1)
        with self.assertRaisesRegex(ValueError, "omitted an authored"):
            INTAKE.build_source_summary(missing, xml_hash, len(xml), True, DATA_HASH)
        records[-1]["record"]["lookup"]["source_use"] = "calcs_minion_lookup"
        with self.assertRaisesRegex(ValueError, "differs from source role"):
            INTAKE.build_source_summary(report, xml_hash, len(xml), True, DATA_HASH)

    def test_build_source_contract_rejects_loss_wrong_ownership_and_mechanic_overclaims(self):
        def records(r): return r["definition_lookup"]["skills"]["lookup"]["records"]
        mutations = [
            lambda r: r.update(schema_version=True),
            lambda r: r["input"].update(xml_sha256="0" * 64),
            lambda r: r["input"].update(xml_bytes=True),
            lambda r: r["verification"].update(native_admission="verified"),
            lambda r: r["build"].update(sections=[]),
            lambda r: r["skills"]["projection"].update(containers=[]),
            lambda r: r["skills"]["projection"]["containers"].append(r["skills"]["projection"]["containers"][0]),
            lambda r: r["definition_lookup"]["skills"]["lookup"].update(records=[]),
            lambda r: records(r).append(copy.deepcopy(records(r)[0])),
            lambda r: records(r)[0].update(container_index=True),
            lambda r: records(r)[0].update(set_source_range=None),
            lambda r: records(r)[0]["record"]["lookup"].update(source_range={"start": 0, "end": 1}),
            lambda r: r["definition_lookup"]["skills"]["lookup"].update(actor_resolution="resolved"),
            lambda r: r["definition_lookup"]["skills"]["lookup"].update(data={"content_sha256": "b" * 64}),
            lambda r: records(r)[0]["record"]["lookup"].update(resolution={"kind": "external_gem", "status": "exact", "candidates": []}),
            lambda r: records(r)[0]["record"]["lookup"].update(resolution={"kind": "external_gem", "status": "ambiguous", "candidates": [{}]}),
            lambda r: r["configuration"].update(status="future"),
        ]
        for index, mutation in enumerate(mutations):
            report = build_report(definitions=True)
            mutation(report)
            with self.subTest(mutation=index), self.assertRaises((ValueError, KeyError, TypeError)):
                INTAKE.build_source_summary(report, hashlib.sha256(BUILD_XML).hexdigest(), len(BUILD_XML), True, DATA_HASH)
        for definitions, report in [(False, build_report(definitions=True)), (True, build_report())]:
            with self.assertRaisesRegex(ValueError, "presence"):
                INTAKE.build_source_summary(report, hashlib.sha256(BUILD_XML).hexdigest(), len(BUILD_XML), definitions, None)
        with self.assertRaisesRegex(ValueError, "different data"):
            INTAKE.build_source_summary(build_report(definitions=True), hashlib.sha256(BUILD_XML).hexdigest(), len(BUILD_XML), True, "f" * 64)

    def test_build_inspection_command_and_failure_are_independent_of_backend_results(self):
        for outcome in ["success", "process_error", "wrong_hash", "wrong_data"]:
            with self.subTest(outcome=outcome), tempfile.TemporaryDirectory() as name:
                root = Path(name)
                source = root / "caller.imports"
                source.write_bytes(b"caller\n")
                data = root / "caller-data.json"
                data.write_bytes(b'{"caller":"data"}')
                expected_data_hash = hashlib.sha256(data.read_bytes()).hexdigest()
                output = root / "observations"
                calls = []
                def fake_invoke(command, directory, prefix, workdir, deadline, timeout):
                    calls.append(prefix)
                    if prefix == "import":
                        Path(command[command.index("--output") + 1]).write_bytes(BUILD_XML)
                        value = {"xml_sha256": hashlib.sha256(BUILD_XML).hexdigest()}
                    elif prefix == "build_source":
                        self.assertEqual(command[1], "inspect-build")
                        self.assertIn("--with-definitions", command)
                        self.assertEqual(Path(command[command.index("--data") + 1]).read_bytes(), data.read_bytes())
                        self.assertEqual(command[command.index("--data-sha256") + 1], expected_data_hash)
                        if outcome == "process_error": return {"status": "process_error", "exit_code": 1}
                        value = build_report(definitions=True)
                        value["definition_lookup"]["data"]["content_sha256"] = expected_data_hash if outcome != "wrong_data" else "b" * 64
                        if outcome == "wrong_hash": value["input"]["xml_sha256"] = "0" * 64
                    else:
                        Path(command[command.index("--export") + 1]).write_bytes(BUILD_XML)
                        value = {"evaluation": {"backend": {"id": prefix}, "build": {}, "coverage": {}, "measurements": []}}
                    (directory / f"{prefix}.stdout").write_text(json.dumps(value), encoding="utf-8")
                    return {"status": "success", "exit_code": 0}
                args = ["--input", str(source), "--output", str(output), "--import-cli", sys.executable,
                        "--backend", f"pob={sys.executable}", "--pob", str(root), "--deadline-seconds", "30",
                        "--inspect-build", "--with-definitions", "--data", str(data), "--data-sha256", expected_data_hash]
                with patch.object(INTAKE, "invoke", side_effect=fake_invoke), contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(INTAKE.main(args), 0 if outcome == "success" else 1)
                index = json.loads((output / "index.json").read_bytes())
                self.assertEqual(index["schema_version"], 3)
                self.assertTrue(index["build_inspection_requested"])
                self.assertTrue(index["definition_lookup_requested"])
                self.assertEqual(calls, ["import", "build_source", "pob"])
                self.assertEqual(index["entries"][0]["evaluations"]["pob"]["status"], "success")
                self.assertEqual(source.read_bytes(), b"caller\n")

    def test_changed_build_inspection_source_stops_remaining_inspection_and_evaluations(self):
        for changer in ["configuration", "build_source"]:
            with self.subTest(changer=changer), tempfile.TemporaryDirectory() as name:
                root = Path(name)
                source = root / "caller.imports"
                source.write_bytes(b"caller\n")
                calls = []
                def fake_invoke(command, directory, prefix, workdir, deadline, timeout):
                    calls.append(prefix)
                    if prefix == "import":
                        Path(command[command.index("--output") + 1]).write_bytes(BUILD_XML)
                        value = {"xml_sha256": hashlib.sha256(BUILD_XML).hexdigest()}
                    elif prefix in {"configuration", "build_source"}:
                        if prefix == changer: Path(command[2]).write_bytes(b"<PathOfBuilding2/>")
                        value = configuration_report(BUILD_XML) if prefix == "configuration" else build_report()
                    else:
                        self.fail("changed source must never reach evaluation")
                    (directory / f"{prefix}.stdout").write_text(json.dumps(value), encoding="utf-8")
                    return {"status": "success", "exit_code": 0}
                args = ["--input", str(source), "--output", str(root / "output"), "--import-cli", sys.executable,
                        "--backend", f"native={sys.executable}", "--deadline-seconds", "30",
                        "--inspect-configuration", "--inspect-build"]
                with patch.object(INTAKE, "invoke", side_effect=fake_invoke), contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(INTAKE.main(args), 1)
                index = json.loads((root / "output/index.json").read_bytes())
                self.assertEqual(index["entries"][0]["evaluations"]["native"]["status"], "source_changed")
                self.assertEqual(calls, ["import", "configuration"] if changer == "configuration" else
                                 ["import", "configuration", "build_source"])

    def test_definition_lookup_requires_explicit_build_inspection(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            source = root / "caller.imports"
            source.write_bytes(b"caller\n")
            args = ["--input", str(source), "--output", str(root / "output"), "--import-cli", sys.executable,
                    "--backend", f"native={sys.executable}", "--deadline-seconds", "30", "--with-definitions"]
            with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                INTAKE.parse_args(args)
            self.assertFalse((root / "output").exists())



if __name__ == "__main__":
    unittest.main()
