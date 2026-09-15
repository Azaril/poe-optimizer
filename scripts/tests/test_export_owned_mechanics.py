"""Offline artifact/exporter tests. No Cargo, source VM, or reference evaluation."""
from __future__ import annotations

import copy
from dataclasses import replace
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("owned_mechanics_export", ROOT / "scripts/export-owned-mechanics.py")
EXPORT = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = EXPORT
SPEC.loader.exec_module(EXPORT)
DATA = ROOT / "data/owned/poe2/3887ae68"
SOURCE = ROOT / "vendor/path-of-building-poe2"


class ExportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest = EXPORT.load(DATA / "source-manifest.json", EXPORT.HARD.manifest_bytes)
        cls.recipe = EXPORT.load(DATA / "recipe.json", EXPORT.HARD.recipe_bytes)
        cls.catalog = EXPORT.read(ROOT / "crates/poe-optimizer-data/data/game-data.json", EXPORT.HARD.catalog_bytes)

    def changed_source(self, path, before, after):
        work = ROOT / "runs/owned-timing-01/export-tests"
        work.mkdir(parents=True, exist_ok=True)
        self.assertTrue(work.resolve().is_relative_to(ROOT))
        temporary = tempfile.TemporaryDirectory(prefix="source-", dir=work)
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name).resolve()
        self.assertTrue(root.is_relative_to(work.resolve()))
        manifest = copy.deepcopy(self.manifest)
        for file in manifest["source"]["files"]:
            destination = root / file["path"]
            destination.parent.mkdir(parents=True, exist_ok=True)
            text = (SOURCE / file["path"]).read_text(encoding="utf-8")
            if file["path"] == path:
                self.assertEqual(text.count(before), 1)
                text = text.replace(before, after)
            destination.write_text(text, encoding="utf-8", newline="\n")
        return root, manifest

    def repin(self, root, manifest):
        source_lines = {file["path"]: (root / file["path"]).read_text(encoding="utf-8").splitlines(keepends=True)
                        for file in manifest["source"]["files"]}
        def span_hash(span):
            lines = source_lines[span["path"]]
            span["sha256"] = EXPORT.sha("".join(lines[span["line"] - 1:span["end_line"]]).encode())
        for file in manifest["source"]["files"]:
            file["sha256"] = EXPORT.sha((root / file["path"]).read_bytes())
        for name in ["record_tables", "array_tables", "literal_facts", "algorithm_evidence"]:
            for record in manifest[name]:
                span_hash(record["span"])
        catalog = json.loads(self.catalog)
        identities = catalog["skill_identities"]
        for name in ["gem_declarations", "skill_declarations"]:
            for declaration in identities[name]:
                if (root / declaration["source"]["path"]).is_file():
                    span_hash(declaration["source"])
        for file in manifest["source"]["files"]:
            if file["path"] in identities["source"]["files"]:
                identities["source"]["files"][file["path"]] = file["sha256"]
        data = EXPORT.compact(catalog)
        manifest["catalog_lf_sha256"] = EXPORT.sha(data)
        return data

    def test_persisted_files_reproduce_exactly(self):
        recipe, facts = EXPORT.export(self.manifest, self.recipe, self.catalog, SOURCE)
        self.assertEqual(EXPORT.pretty(recipe), (DATA / "recipe.json").read_bytes())
        self.assertEqual(EXPORT.pretty(facts), (DATA / "mechanics-facts.json").read_bytes())
        self.assertEqual(len(facts["unresolved"]), 2)
        self.assertFalse(facts["source_execution"])
        self.assertFalse(facts["full_build_numeric_coverage"])

    def test_complete_domains_and_distinct_scaling_coordinates(self):
        recipe, facts = EXPORT.export(self.manifest, self.recipe, self.catalog, SOURCE)
        tables = {t["id"]: t for t in recipe["rules"]["tables"]}
        self.assertEqual(len(tables), 7)
        for table in tables.values():
            self.assertEqual((table["minimum"], table["maximum"], len(table["rows"])), (1, 40, 40))
        self.assertEqual(tables["twister.base-damage-factor"]["rows"][0]["value"]["value"], 0.8)
        self.assertEqual(tables["twister.base-damage-factor"]["rows"][18]["value"]["value"], 2.23)
        self.assertEqual(tables["twister.base-damage-factor"]["rows"][-1]["value"]["value"], 5.29)
        self.assertEqual(tables["sniper.actor-level"]["rows"][19], {"kind": "integer", "value": 40})
        self.assertEqual(tables["sniper.spirit-reservation"]["rows"][-1]["value"]["value"], 26.0)
        scaling = next(t for t in facts["tables"] if t["id"] == "sniper-stat-set-scaling-evidence")
        self.assertEqual(scaling["rows"][19]["values"]["scaling_coordinate"], 97.699996948242)
        self.assertNotIn(scaling["id"], tables, "source scaling coordinate is not an actor level")

    def test_changed_reviewed_source_changes_data_without_reallocating_identity(self):
        before = "[1] = { attackSpeedMultiplier = -20, baseMultiplier = 0.8, levelRequirement = 0, cost = { Mana = 4, }, },"
        root, manifest = self.changed_source("src/Data/Skills/act_dex.lua", before, before.replace("0.8,", "0.81,"))
        catalog = self.repin(root, manifest)
        recipe, facts = EXPORT.export(manifest, self.recipe, catalog, root)
        self.assertEqual(recipe["registry"], self.recipe["registry"])
        self.assertEqual(recipe["schema"], self.recipe["schema"])
        self.assertNotEqual(facts["recipe_sha256"], EXPORT.sha(EXPORT.pretty(self.recipe)))
        table = next(t for t in recipe["rules"]["tables"] if t["id"] == "twister.base-damage-factor")
        self.assertEqual(table["rows"][0]["value"]["value"], 0.81)

    def test_unreviewed_source_drift_rejects_before_returning_artifacts(self):
        before = "[1] = { attackSpeedMultiplier = -20, baseMultiplier = 0.8, levelRequirement = 0, cost = { Mana = 4, }, },"
        root, manifest = self.changed_source("src/Data/Skills/act_dex.lua", before, before.replace("0.8,", "0.81,"))
        with self.assertRaisesRegex(ValueError, "hash differs"):
            EXPORT.export(manifest, self.recipe, self.catalog, root)

    def test_even_repinned_source_expressions_are_not_executed(self):
        before = "[1] = { attackSpeedMultiplier = -20, baseMultiplier = 0.8, levelRequirement = 0, cost = { Mana = 4, }, },"
        root, manifest = self.changed_source("src/Data/Skills/act_dex.lua", before, before.replace("0.8,", "0.8 + os.execute('never'),"))
        catalog = self.repin(root, manifest)
        with self.assertRaisesRegex(ValueError, "unsupported literal row shape"):
            EXPORT.export(manifest, self.recipe, catalog, root)

    def test_repinned_duplicate_level_is_not_last_writer_wins(self):
        before = "[1] = { attackSpeedMultiplier = -20, baseMultiplier = 0.8, levelRequirement = 0, cost = { Mana = 4, }, },"
        root, manifest = self.changed_source("src/Data/Skills/act_dex.lua", before, before.replace("[1]", "[2]"))
        catalog = self.repin(root, manifest)
        with self.assertRaisesRegex(ValueError, "duplicate, missing, or reordered"):
            EXPORT.export(manifest, self.recipe, catalog, root)

    def test_missing_domain_and_column_cannot_be_filled(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["record_tables"][0]["maximum"] = 39
        with self.assertRaisesRegex(ValueError, "row census"):
            EXPORT.export(manifest, self.recipe, self.catalog, SOURCE)
        manifest = copy.deepcopy(self.manifest)
        manifest["table_bindings"][0]["column"] = "absent"
        with self.assertRaisesRegex(ValueError, "missing source column"):
            EXPORT.export(manifest, self.recipe, self.catalog, SOURCE)

    def test_integer_table_cannot_silently_round_a_fraction(self):
        manifest = copy.deepcopy(self.manifest)
        binding = next(b for b in manifest["table_bindings"] if b["table"] == "twister.required-character-level")
        binding["column"] = "damage"
        with self.assertRaisesRegex(ValueError, "nonintegral"):
            EXPORT.export(manifest, self.recipe, self.catalog, SOURCE)

    def test_absent_command_ledger_cannot_be_dropped(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["unresolved"] = []
        with self.assertRaisesRegex(ValueError, "missing-reference ledger changed"):
            EXPORT.export(manifest, self.recipe, self.catalog, SOURCE)

    def test_aggregate_bounds_and_invalid_limits(self):
        for limits in [replace(EXPORT.HARD, rows=199), replace(EXPORT.HARD, cells=200),
                       replace(EXPORT.HARD, output_bytes=100), replace(EXPORT.HARD, files=1)]:
            with self.assertRaises(ValueError):
                EXPORT.export(self.manifest, self.recipe, self.catalog, SOURCE, limits)
        with self.assertRaisesRegex(ValueError, "invalid rows limit"):
            EXPORT.export(self.manifest, self.recipe, self.catalog, SOURCE, replace(EXPORT.HARD, rows=0))

    def test_wire_and_source_paths_fail_closed(self):
        with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
            json.loads('{"x":1,"x":2}', object_pairs_hook=EXPORT.unique)
        with self.assertRaisesRegex(ValueError, "nonfinite"):
            json.loads('{"x":NaN}', parse_constant=EXPORT.no_constant)
        manifest = copy.deepcopy(self.manifest)
        manifest["unknown"] = True
        with self.assertRaisesRegex(ValueError, "manifest fields"):
            EXPORT.export(manifest, self.recipe, self.catalog, SOURCE)
        manifest = copy.deepcopy(self.manifest)
        manifest["source"]["files"][0]["path"] = "../outside.lua"
        with self.assertRaisesRegex(ValueError, "escapes root"):
            EXPORT.export(manifest, self.recipe, self.catalog, SOURCE)

    def test_output_directory_is_exclusively_reserved_not_renamed_over(self):
        work = ROOT / "runs/owned-timing-01/export-tests"
        work.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="publish-", dir=work) as temporary:
            root = Path(temporary).resolve()
            self.assertTrue(root.is_relative_to(work.resolve()))
            existing = root / "existing"
            existing.mkdir()
            with self.assertRaises(FileExistsError):
                EXPORT.publish(existing, {"recipe.json": b"content"})
            self.assertEqual(list(existing.iterdir()), [])
            new = root / "new"
            EXPORT.publish(new, {"recipe.json": b"first"})
            with self.assertRaises(FileExistsError):
                EXPORT.publish(new, {"recipe.json": b"second"})
            self.assertEqual((new / "recipe.json").read_bytes(), b"first")
            failed = root / "incomplete"
            with mock.patch.object(Path, "open", side_effect=OSError("injected write failure")):
                with self.assertRaisesRegex(ValueError, "incomplete new directory retained"):
                    EXPORT.publish(failed, {"recipe.json": b"content"})
            self.assertTrue(failed.is_dir())


    def test_changed_quality_coefficient_updates_exact_owned_literal(self):
        before = '{ "twister_%_chance_for_additional_twister", 1, {  } },'
        root, manifest = self.changed_source("src/Data/Skills/act_dex.lua", before, before.replace(", 1,", ", 2,"))
        catalog = self.repin(root, manifest)
        recipe, _ = EXPORT.export(manifest, self.recipe, catalog, root)
        binding = manifest["literal_bindings"][0]
        owner = next(o for o in recipe["rules"]["owners"] if o["owner"] == binding["owner"])
        program = next(p for p in owner["programs"]["members"] if p["id"] == binding["program"])
        literal = next(n for n in program["nodes"] if n["id"] == binding["node"])
        self.assertEqual(literal["expression"]["value"]["value"]["value"], 2.0)
        self.assertEqual(recipe["registry"], self.recipe["registry"])
        self.assertEqual(recipe["rules"]["tables"], self.recipe["rules"]["tables"])

    def test_literal_binding_cannot_mutate_an_operation_or_ambiguous_target(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["literal_bindings"][0]["node"] = "quality"
        with self.assertRaisesRegex(ValueError, "Literal node"):
            EXPORT.export(manifest, self.recipe, self.catalog, SOURCE)
        manifest = copy.deepcopy(self.manifest)
        manifest["literal_bindings"].append(copy.deepcopy(manifest["literal_bindings"][0]))
        with self.assertRaisesRegex(ValueError, "duplicate literal binding"):
            EXPORT.export(manifest, self.recipe, self.catalog, SOURCE)


    def test_reservation_is_scoped_to_each_summoning_action(self):
        ids = EXPORT.load(DATA / "ids.json", EXPORT.HARD.manifest_bytes)["allocations"]
        reservation = ids["sniper-spirit-reservation"]
        schema = next(row["value"]["schema"]["value"] for row in self.recipe["schema"]["definitions"]
                      if row["value"]["id"] == reservation)
        self.assertEqual(schema["targets"], ["action"])
        producers = [(p["id"], p["context"]) for owner in self.recipe["rules"]["owners"]
                     for p in owner["programs"]["members"] for effect in p["effects"]
                     if effect["effect"].get("stat") == reservation]
        self.assertEqual(producers, [("intrinsic-reservation-coefficients", "action")])


    def test_aggregate_limit_is_checked_before_any_row_builder(self):
        for limits in [replace(EXPORT.HARD, rows=199), replace(EXPORT.HARD, cells=200)]:
            with mock.patch.object(EXPORT, "record_rows") as records, mock.patch.object(EXPORT, "array_rows") as arrays:
                with self.assertRaisesRegex(ValueError, "before expansion"):
                    EXPORT.export(self.manifest, self.recipe, self.catalog, SOURCE, limits)
                records.assert_not_called()
                arrays.assert_not_called()

    def test_alternate_quality_and_missing_command_remain_partial(self):
        ids = EXPORT.load(DATA / "ids.json", EXPORT.HARD.manifest_bytes)["allocations"]
        def definition(label):
            return next(row["value"]["schema"]["value"] for row in self.recipe["schema"]["definitions"]
                        if row["value"]["id"] == ids[label])
        def owner(label):
            return next(o for o in self.recipe["rules"]["owners"]
                        if o["owner"] == {"kind": "definition", "value": {"kind": ids[label]["kind"], "value": ids[label]}})
        self.assertEqual(definition("twister-gem")["quality"]["allowed_kinds"]["closure"]["kind"], "partial")
        self.assertEqual(owner("twister-gem")["programs"]["closure"]["kind"], "partial")
        for field in ["grants", "skill_grants"]:
            self.assertEqual(definition("sniper-gem")["declarations"][field]["closure"]["kind"], "partial")
        self.assertEqual(owner("sniper-gem")["programs"]["closure"]["value"]["gaps"][0]["code"], "source-command-supply-unresolved")
        _, facts = EXPORT.export(self.manifest, self.recipe, self.catalog, SOURCE)
        alternate = next(f for f in facts["literal_facts"] if f["id"] == "twister-alternate-quality-unconverted")
        self.assertEqual(alternate["values"]["per_quality"], 1.0)


    def test_quality_rounding_is_per_stat_and_source_algorithm_is_pinned(self):
        ids = EXPORT.load(DATA / "ids.json", EXPORT.HARD.manifest_bytes)["allocations"]
        for binding in self.manifest["literal_bindings"]:
            owner = next(o for o in self.recipe["rules"]["owners"] if o["owner"] == binding["owner"])
            program = next(p for p in owner["programs"]["members"] if p["id"] == binding["program"])
            nodes = {n["id"]: n["expression"] for n in program["nodes"]}
            self.assertEqual(nodes["quality-rounded"], {
                "kind": "round", "value": "quality-bonus",
                "quantum": {"value": 1.0, "unit": ids["percentage-points"]}, "mode": "truncate"})
            if binding["source_fact"] == "twister-standard-quality":
                effect = next(e for e in program["effects"] if e["id"] == "additional-projectile-chance")
                self.assertEqual(effect["effect"]["value"], "quality-rounded")
            else:
                self.assertEqual(nodes["quality-more"]["percent"], "quality-rounded")
        _, facts = EXPORT.export(self.manifest, self.recipe, self.catalog, SOURCE)
        algorithm = facts["algorithm_evidence"][0]
        self.assertEqual(algorithm["span"]["path"], "src/Modules/CalcTools.lua")
        self.assertEqual(algorithm["text"].count("math.modf(stat[2] * skillInstance.quality)"), 2)
        self.assertIn("if includeAltQualityStats and qualityStats then", algorithm["text"])

    def test_algorithm_evidence_rejects_duplicate_unknown_and_excess_claims(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["algorithm_evidence"].append(copy.deepcopy(manifest["algorithm_evidence"][0]))
        with self.assertRaisesRegex(ValueError, "duplicated"):
            EXPORT.export(manifest, self.recipe, self.catalog, SOURCE)
        manifest = copy.deepcopy(self.manifest)
        manifest["algorithm_evidence"][0]["execute"] = True
        with self.assertRaisesRegex(ValueError, "algorithm evidence fields"):
            EXPORT.export(manifest, self.recipe, self.catalog, SOURCE)
        manifest = copy.deepcopy(self.manifest)
        manifest["algorithm_evidence"][0]["claim"] = "a" * 2049
        with self.assertRaisesRegex(ValueError, "claim exceeds bound"):
            EXPORT.export(manifest, self.recipe, self.catalog, SOURCE)

    def test_numeric_captures_cannot_ambiguously_split_the_same_number(self):
        for template in ["{{a}}{{b}}", "{{a}}123{{b}}", "{{a}}e-{{b}}",
                         "".join("{{v" + chr(97 + i) + "}}" for i in range(26))]:
            with mock.patch.object(EXPORT.re, "compile") as compile_pattern:
                with self.assertRaises(ValueError) as error:
                    EXPORT.matcher(template)
                compile_pattern.assert_not_called()
                self.assertIn("unambiguous literal separator", str(error.exception))
        for template, text in [("{{a}},{{b}}", "1,2"), ("{{a}} {{b}}", "1 2")]:
            pattern, _ = EXPORT.matcher(template)
            self.assertIsNotNone(pattern.fullmatch(text))


    def test_implicit_array_keys_cannot_be_relabelled_by_manifest_domain(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["array_tables"][0]["minimum"] = 2
        manifest["array_tables"][0]["maximum"] = 41
        with mock.patch.object(EXPORT, "array_rows") as arrays:
            with self.assertRaisesRegex(ValueError, "array keys must begin at one"):
                EXPORT.export(manifest, self.recipe, self.catalog, SOURCE)
            arrays.assert_not_called()


if __name__ == "__main__":
    unittest.main()
