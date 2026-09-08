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


def item_report():
    payload = (b"<Items><Item id='42'> first<!--c-->second<ModRange id='1' range='.5'/>"
               b"<![CDATA[ tail ]]></Item><ItemSet id='8'><Slot name='Amulet' itemId='42'/>"
               b"<SocketIdURL nodeId='5'/></ItemSet></Items>"
               b"<Tree><Spec title='first'><Sockets><Socket nodeId='5' itemId='42'/></Sockets></Spec></Tree>"
               b"<Spec title='legacy'><Sockets><Socket nodeId='6' itemId='42'/></Sockets></Spec>")
    xml = BUILD_XML.replace(b"</PathOfBuilding2>", payload + b"</PathOfBuilding2>")
    report = build_report(xml)
    report.update(schema_version=2, scope="build_source_projection_v2")
    report["verification"].update(item_loading="not_run", equipment_resolution="not_resolved",
                                  passive_allocation="not_checked")
    def node(token, tag, role, children=(), attributes=None, after=0, fragments=None, consumed=None):
        start = xml.index(token, after)
        opening_end = xml.index(b">", start) + 1
        end = opening_end if xml[opening_end-2:opening_end] == b"/>" else xml.index(("</"+tag+">").encode(), start)+len(tag)+3
        attrs = []
        for key, value in (attributes or {}).items():
            value_start = xml.index((key+"='").encode(), start)+len(key)+2
            attrs.append({"name": key, "namespace": None, "value": {"range": {"start": value_start, "end": value_start+len(value)},
                                                 "raw": value, "decoded": value}})
        result = {"element": {"name": tag, "namespace": None, "has_namespaces": False,
                              "source_range": {"start": start, "end": end}, "attributes": attrs},
                  "kind": INTAKE.item_source_classification(None, tag, False)[0],
                  "source_use": role, "children": list(children)}
        result["ordered_content"] = {"fragments": fragments if fragments is not None else
            [{"kind": "element", "range": child["element"]["source_range"], "child_index": i}
             for i, child in enumerate(children)], "consumed": consumed if consumed is not None else
            [{"kind": "element", "child_index": i} for i in range(len(children))]}
        return result
    mod = node(b"<ModRange ", "ModRange", "modifier_range_instruction", attributes={"id": "1", "range": ".5"})
    fragments = []
    for kind, raw in [("text", b" first"), ("comment", b"<!--c-->"), ("text", b"second"),
                      ("element", b"<ModRange id='1' range='.5'/>"), ("cdata", b"<![CDATA[ tail ]]>")]:
        start = xml.index(raw)
        fragments.append({"kind": kind, "range": {"start": start, "end": start+len(raw)},
                          **({"child_index": 0} if kind == "element" else {"text_source": raw.decode("utf-8")})})
    item = node(b"<Item id=", "Item", "inventory_item", [mod], {"id": "42"}, fragments=fragments,
                consumed=[{"kind": "text", "text_kind": "ordinary", "text": "firstsecond", "fragment_indices": [0,1,2]},
                          {"kind": "element", "child_index": 0},
                          {"kind": "text", "text_kind": "cdata", "text": " tail ", "fragment_indices": [4]}])
    slot = node(b"<Slot ", "Slot", "equipment_slot", attributes={"name": "Amulet", "itemId": "42"})
    url = node(b"<SocketIdURL ", "SocketIdURL", "socket_url_metadata", attributes={"nodeId": "5"})
    saved = node(b"<ItemSet ", "ItemSet", "saved_set", [slot,url], {"id": "8"})
    container = node(b"<Items>", "Items", "container", [item,saved])
    specs = []
    for title, identity in [("first","5"), ("legacy","6")]:
        begin = xml.index(("<Spec title='"+title+"'>").encode())
        socket = node(("<Socket nodeId='"+identity+"'").encode(), "Socket", "jewel_assignment",
                      attributes={"nodeId": identity, "itemId": "42"})
        sockets = node(b"<Sockets>", "Sockets", "jewel_sockets", [socket], after=begin)
        specs.append(node(("<Spec title='"+title+"'>").encode(), "Spec", "passive_spec", [sockets], {"title": title}))
    tree = node(b"<Tree>", "Tree", "tree", [specs[0]])
    for kind, root in [("items",container), ("tree",tree), ("legacy_spec",specs[1])]:
        report["build"]["sections"].append({"kind": kind, "element": root["element"]})
    report["items"] = {"status": "source_projected", "projection": {
        "source_sha256": hashlib.sha256(xml).hexdigest(), "containers": [container], "trees": [tree,specs[1]]}}
    return xml, report


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
                self.assertEqual(index["schema_version"], 4)
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
                self.assertEqual(index["schema_version"], 4)
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

    def test_item_report_retains_load_order_and_distinct_passive_spec_jewel_owners(self):
        xml, report = item_report()
        summary = INTAKE.build_source_summary(report, hashlib.sha256(xml).hexdigest(), len(xml), False, None)
        self.assertEqual(summary["report_schema_version"], 2)
        self.assertEqual(summary["item_source_counts"]["socket_url_metadata"], 1)
        self.assertEqual(summary["item_source_counts"]["jewel_assignment"], 2)
        self.assertEqual([i["kind"] for i in summary["item_inventory"][0]["instructions"]],
                         ["text", "modifier_range", "text"])
        self.assertEqual([j["tree_root_index"] for j in summary["item_jewel_assignments"]], [0, 1])
        self.assertNotEqual(summary["item_jewel_assignments"][0]["spec_source_range"],
                            summary["item_jewel_assignments"][1]["spec_source_range"])
        self.assertEqual(summary["item_sets"][0]["source_id"]["decoded"], "8")
        report["items"] = {"status": "not_projected", "error": {"reason": "source bound"}}
        partial = INTAKE.build_source_summary(report, hashlib.sha256(xml).hexdigest(), len(xml), False, None)
        self.assertEqual(partial["item_source_status"], "not_projected")
        self.assertEqual(partial["skills_status"], "source_projected")
        legacy = INTAKE.build_source_summary(build_report(), hashlib.sha256(BUILD_XML).hexdigest(), len(BUILD_XML), False, None)
        self.assertEqual(legacy["item_source_status"], "not_reported_by_inspector")
        self.assertNotIn("item_inventory", legacy)

    def test_item_report_rejects_lost_reordered_reowned_content_and_admission_claims(self):
        def inventory(r): return r["items"]["projection"]["containers"][0]["children"][0]
        mutations = [
            lambda r: r["verification"].update(item_loading="applied"),
            lambda r: r["items"]["projection"].update(trees=[]),
            lambda r: r["items"]["projection"].update(source_sha256="0"*64),
            lambda r: inventory(r)["ordered_content"]["consumed"].pop(1),
            lambda r: inventory(r)["ordered_content"]["consumed"].reverse(),
            lambda r: inventory(r)["ordered_content"]["fragments"][3].update(child_index=True),
            lambda r: inventory(r)["ordered_content"]["fragments"][0].update(text_source="lost"),
            lambda r: inventory(r)["ordered_content"]["consumed"][2].update(fragment_indices=[0]),
            lambda r: inventory(r)["ordered_content"]["consumed"][0].update(fragment_indices=[0,0]),
            lambda r: r["items"]["projection"]["trees"][1].update(source_use="ignored"),
            lambda r: r.update(schema_version=1, scope="build_source_projection_v1"),
        ]
        for index, mutation in enumerate(mutations):
            xml, report = item_report()
            mutation(report)
            with self.subTest(mutation=index), self.assertRaises((ValueError, KeyError, TypeError)):
                INTAKE.build_source_summary(report, hashlib.sha256(xml).hexdigest(), len(xml), False, None)


    def test_item_report_rejects_missing_forged_text_and_invalid_source_fields(self):
        def item(r): return r["items"]["projection"]["containers"][0]["children"][0]
        mutations = [
            lambda r: item(r)["ordered_content"]["consumed"].pop(0),
            lambda r: item(r)["ordered_content"]["consumed"][0].update(text="forged different item"),
            lambda r: item(r)["element"]["attributes"][0].update(value=17),
            lambda r: item(r)["element"]["attributes"][0]["value"].update(decoded=17),
            lambda r: item(r)["element"]["attributes"][0]["value"]["range"].update(start=True),
            lambda r: item(r)["element"]["attributes"][0]["value"].update(decoded="wrong"),
            lambda r: item(r).update(source_use="ignored"),
            lambda r: item(r).update(kind="unknown"),
            lambda r: item(r)["children"][0].update(source_use="ignored"),
        ]
        for index, mutation in enumerate(mutations):
            for bind_source in [False, True]:
                xml, report = item_report()
                mutation(report)
                with self.subTest(mutation=index, bind_source=bind_source), self.assertRaises((ValueError, KeyError, TypeError)):
                    INTAKE.build_source_summary(report, hashlib.sha256(xml).hexdigest(), len(xml), False, None,
                                                expected_xml=xml if bind_source else None)

    def test_exact_imported_bytes_reject_internally_consistent_forged_source_and_omissions(self):
        def item(r): return r["items"]["projection"]["containers"][0]["children"][0]
        def fake_raw_and_text(r):
            node = item(r)
            node["ordered_content"]["fragments"][0]["text_source"] = " wrong"
            node["ordered_content"]["consumed"][0]["text"] = "wrongsecond"
        def fake_attribute(r):
            item(r)["element"]["attributes"][0]["value"].update(raw="99", decoded="99")
        def omit_text_fragment(r):
            item(r)["ordered_content"]["fragments"].pop()
            item(r)["ordered_content"]["consumed"].pop()
        def omit_attribute(r):
            item(r)["element"]["attributes"] = []
        def fake_namespace(r):
            def hide(node):
                node["element"]["has_namespaces"] = True
                node.update(kind="unknown", source_use="namespace_unknown")
                for child in node["children"]: hide(child)
            hide(r["items"]["projection"]["containers"][0])
        for mutate in [fake_raw_and_text, fake_attribute, omit_text_fragment, omit_attribute, fake_namespace]:
            xml, report = item_report()
            mutate(report)
            with self.subTest(mutation=mutate.__name__), self.assertRaises(ValueError):
                INTAKE.build_source_summary(report, hashlib.sha256(xml).hexdigest(), len(xml), False, None,
                                            expected_xml=xml)
        xml, report = item_report()
        INTAKE.build_source_summary(report, hashlib.sha256(xml).hexdigest(), len(xml), False, None, expected_xml=xml)
        for bad in [xml[:-1], xml.replace(b"first", b"wrong")]:
            with self.assertRaisesRegex(ValueError, "import identity"):
                INTAKE.build_source_summary(report, hashlib.sha256(xml).hexdigest(), len(xml), False, None,
                                            expected_xml=bad)

    def test_item_text_derivation_matches_comment_cdata_pi_and_named_entity_order(self):
        fragments = [{"kind": kind, "text_source": raw} for kind, raw in [
            ("text", " \r\n a&amp;"), ("comment", "<!-- join -->"), ("text", "b  "),
            ("cdata", "<![CDATA[ \r\n&amp; x<!--deleted-->y ]]>" ),
            ("cdata", "<![CDATA[ <!-- all whitespace --> \t]]>"),
            ("text", "\u00a0"), ("processing_instruction", "<?cut here?>"),
            ("text", " &amp;lt; "), ("cdata", "<![CDATA[unterminated <!-- literal]]>"),
        ]]
        expected = [
            {"kind": "text", "text_kind": "ordinary", "text": "a&b", "fragment_indices": [0, 1, 2]},
            {"kind": "text", "text_kind": "cdata", "text": " \r\n&amp; xy ", "fragment_indices": [3]},
            {"kind": "text", "text_kind": "ordinary", "text": "\u00a0", "fragment_indices": [5]},
            {"kind": "text", "text_kind": "ordinary", "text": "&lt;", "fragment_indices": [7]},
            {"kind": "text", "text_kind": "cdata", "text": "unterminated <!-- literal", "fragment_indices": [8]},
        ]
        self.assertEqual(INTAKE.derive_item_content(fragments), expected)
        with self.assertRaisesRegex(ValueError, "structure"):
            INTAKE.derive_item_content([{"kind": "cdata", "text_source": "<![CDATA[a]<!--remove-->]>b]]>"}])
        with self.assertRaisesRegex(ValueError, "entity"):
            INTAKE.source_named_entities("&#10;")
        self.assertEqual(INTAKE.source_named_entities("&lt;&gt;&amp;&quot;&apos;\r\n\t"), "<>&\"'\r\n\t")

    def test_item_fragment_boundaries_are_exact_and_global_comment_removal_cannot_cross_cdata(self):
        for fragments in [
            [{"kind": "text", "text_source": "split"}, {"kind": "text", "text_source": "text"}],
            [{"kind": "comment", "text_source": "<!--one--><!--two-->"}],
        ]:
            with self.assertRaises(ValueError): INTAKE.derive_item_content(fragments)
        xml = b"<Item><![CDATA[ab<!-- ]]><!-- ends --></Item>"
        start = xml.index(b"<![CDATA[")
        end = xml.index(b"]]>") + 3
        fragments = [{"kind": "cdata", "range": {"start": start, "end": end},
                      "text_source": xml[start:end].decode("utf-8")}]
        with self.assertRaisesRegex(ValueError, "crosses a CDATA boundary"):
            INTAKE.derive_item_content(fragments, expected_xml=xml)
        no_end = b"<Item><![CDATA[ab<!-- ]]></Item>"
        self.assertEqual(INTAKE.derive_item_content(fragments, expected_xml=no_end),
                         [{"kind": "text", "text_kind": "cdata", "text": "ab<!-- ", "fragment_indices": [0]}])

    def test_item_source_index_preserves_raw_attribute_bytes_and_namespace_ownership(self):
        xml = ("<PathOfBuilding2><Items><ItemSet id='' title='caf\u00e9\r\n\t&amp;lt;'/></Items>"
               "<Items xmlns='urn:foreign'><Item xmlns='' id='1'/></Items></PathOfBuilding2>").encode("utf-8")
        indexed = INTAKE.source_element_index(xml)
        saved = next(node for node in indexed.values() if node["name"] == "ItemSet")
        self.assertEqual(saved["attributes"][0]["value"]["raw"], "")
        self.assertEqual(saved["attributes"][0]["value"]["range"]["start"],
                         saved["attributes"][0]["value"]["range"]["end"])
        title = saved["attributes"][1]["value"]
        self.assertEqual(title["raw"], "caf\u00e9\r\n\t&amp;lt;")
        self.assertEqual(title["decoded"], "caf\u00e9\r\n\t&lt;")
        self.assertEqual(xml[title["range"]["start"]:title["range"]["end"]], title["raw"].encode("utf-8"))
        foreign = next(node for node in indexed.values() if node["name"] == "Items" and node["namespace"])
        self.assertTrue(foreign["has_namespaces"])
        reset = foreign["children"][0]
        self.assertIsNone(reset["namespace"])
        self.assertTrue(reset["has_namespaces"])  # explicit empty default binding remains authored
        self.assertEqual(INTAKE.item_source_classification("namespace_unknown", "Item", True),
                         ("unknown", "namespace_unknown"))
        for bad in [b"<!DOCTYPE PathOfBuilding2><PathOfBuilding2/>",
                    b"<PathOfBuilding2><Item></PathOfBuilding2>"]:
            with self.assertRaises(ValueError): INTAKE.source_element_index(bad)

    def test_item_depth_zero_boundary_matches_portable_projection(self):
        def report_at_depth(depth):
            root_open, items_open = b"<PathOfBuilding2>", b"<Items>"
            xml = root_open + items_open + b"<Future>" * depth + b"</Future>" * depth + b"</Items></PathOfBuilding2>"
            opening_base = len(root_open) + len(items_open)
            branch = []
            for level in reversed(range(depth)):
                start = opening_base + level * len(b"<Future>")
                end = opening_base + depth * len(b"<Future>") + (depth - level) * len(b"</Future>")
                span = {"start": start, "end": end}
                children = branch
                branch = [{"kind": "unknown", "source_use": "ignored",
                           "element": {"name": "Future", "namespace": None, "has_namespaces": False,
                                       "source_range": span, "attributes": []},
                           "children": children,
                           "ordered_content": {
                               "fragments": [{"kind": "element", "range": child["element"]["source_range"], "child_index": i}
                                             for i, child in enumerate(children)],
                               "consumed": [{"kind": "element", "child_index": i} for i in range(len(children))]}}]
            container = {"kind": "items", "source_use": "container",
                         "element": {"name": "Items", "namespace": None, "has_namespaces": False,
                                     "source_range": {"start": len(root_open), "end": xml.index(b"</Items>") + len(b"</Items>")},
                                     "attributes": []},
                         "children": branch,
                         "ordered_content": {
                             "fragments": [{"kind": "element", "range": child["element"]["source_range"], "child_index": i}
                                           for i, child in enumerate(branch)],
                             "consumed": [{"kind": "element", "child_index": i} for i in range(len(branch))]}}
            base = configuration_report(xml)
            base.update(schema_version=2, scope="build_source_projection_v2",
                        build={"source_sha256": hashlib.sha256(xml).hexdigest(),
                               "root": {"source_range": {"start": 0, "end": len(xml)}},
                               "sections": [{"kind": "items", "element": container["element"]}]},
                        configuration={"status": "source_projected", "projection": base["configuration"]},
                        skills={"status": "source_projected",
                                "projection": {"source_sha256": hashlib.sha256(xml).hexdigest(), "containers": []}},
                        items={"status": "source_projected",
                               "projection": {"source_sha256": hashlib.sha256(xml).hexdigest(), "containers": [container], "trees": []}})
            base["verification"].update(calculation_context="not_resolved", native_admission="not_checked",
                                        item_loading="not_run", equipment_resolution="not_resolved", passive_allocation="not_checked")
            return xml, base
        xml, report = report_at_depth(32)
        summary = INTAKE.build_source_summary(report, hashlib.sha256(xml).hexdigest(), len(xml), False, None,
                                              expected_xml=xml)
        self.assertEqual(summary["item_source_counts"]["ignored"], 32)
        xml, report = report_at_depth(33)
        with self.assertRaisesRegex(ValueError, "depth"):
            INTAKE.build_source_summary(report, hashlib.sha256(xml).hexdigest(), len(xml), False, None,
                                       expected_xml=xml)

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
