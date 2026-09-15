"""Bounded offline import-data tests; no source VM, Cargo or build evaluation."""
import copy
from dataclasses import replace
import importlib.util
from pathlib import Path
import sys
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("owned_import_export", ROOT / "scripts/export-owned-import-data.py")
EXPORT = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = EXPORT
SPEC.loader.exec_module(EXPORT)
DATA = ROOT / "data/owned/poe2/3887ae68"
INPUTS = [ROOT / "crates/poe-optimizer-data/data/game-data.json", DATA / "recipe.json", DATA / "ids.json",
          DATA / "source-manifest.json", DATA / "mechanics-facts.json",
          ROOT / "crates/poe-optimizer-import/tests/fixtures/owned-quest-rewards-v1.json",
          ROOT / "tests/fixtures/breadth-expectations/originals-v1.json", DATA / "import/authoring.json"]


class ImportExportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs = [EXPORT.read(p, EXPORT.HARD.snapshot_bytes if i == 0 else EXPORT.HARD.artifact_bytes) for i, p in enumerate(INPUTS)]
        cls.source = ROOT / "vendor/path-of-building-poe2"
        cls.outputs = EXPORT.produce(*cls.inputs, cls.source)

    def produce_changed(self, index, value, limits=EXPORT.HARD):
        inputs = list(self.inputs)
        inputs[index] = EXPORT.pretty(value)
        return EXPORT.produce(*inputs, self.source, limits)

    def test_persisted_seed_artifacts_reproduce_exactly(self):
        for name, data in self.outputs.items():
            self.assertEqual(data, (DATA / "import" / name).read_bytes(), name)

    def test_rule_registry_survives_with_no_implicit_old_wire_migration(self):
        for value in [EXPORT.decode(self.inputs[1]), EXPORT.decode(self.outputs["recipe-seed.json"])]:
            self.assertEqual(value["rules"]["schema_version"], 2)
            self.assertEqual(value["rules"]["receivers"], {"members": [], "closure": {"kind": "complete"}})
        original = EXPORT.decode(self.inputs[1])["rules"]
        for changed in ["missing", "old_version", "old_operations"]:
            rules = copy.deepcopy(original)
            if changed == "missing":
                rules.pop("receivers")
            elif changed == "old_operations":
                rules["operations_version"] = "owned-domain-operations-v5"
            else:
                rules["schema_version"] = 1
            with self.assertRaisesRegex(ValueError, "owned rule"):
                EXPORT.validate_rule_wire(rules)

    def test_standalone_catalog_is_exact_identity_projection(self):
        original = EXPORT.decode(self.inputs[0])["skill_identities"]
        catalog = EXPORT.decode(self.outputs["skill-identities.json"])
        self.assertEqual(catalog, original)
        self.assertEqual((len(catalog["gems"]), len(catalog["skills"]), len(catalog["missing_references"])), (966, 1436, 16))
        missing = [r for r in catalog["missing_references"] if r["gem_key"] == "Metadata/Items/Gems/SkillGemSkeletalSniper"]
        self.assertEqual(len(missing), 2)
        self.assertTrue(all(r["effect_id"] == "CommandSkeletalSniperPlayer" for r in missing))

    def test_existing_occurrences_descriptors_and_rule_bodies_are_preserved(self):
        base = EXPORT.decode(self.inputs[1])
        seed = EXPORT.decode(self.outputs["recipe-seed.json"])
        self.assertEqual(seed["registry"]["entries"][:38], base["registry"]["entries"])
        self.assertEqual(seed["registry"]["last_issued"], 119)
        self.assertEqual(seed["registry"]["revision"], 119)
        for row in base["schema"]["definitions"]:
            self.assertEqual([r for r in seed["schema"]["definitions"] if r["value"]["id"] == row["value"]["id"]], [row])
        self.assertEqual(seed["schema"]["slots"], base["schema"]["slots"])
        for field in ["rules", "routing"]:
            before, after = copy.deepcopy(base[field]), copy.deepcopy(seed[field])
            before.pop("definitions")
            after.pop("definitions")
            self.assertEqual(before, after)
        counts = {}
        for entry in seed["registry"]["entries"][38:]:
            kind = entry["target"]["value"]["kind"]
            counts[kind] = counts.get(kind, 0) + 1
        self.assertEqual(counts, {"reward": 31, "option": 30, "equipment_slot": 20})

    def test_six_exact_mapping_targets_reuse_owned_recipe_ids(self):
        ids = EXPORT.decode(self.inputs[2])["allocations"]
        seed = EXPORT.decode(self.outputs["mapping-seed.json"])
        exact = [e for e in seed["entries"] if e["source"]["kind"] == "definition"]
        self.assertEqual(len(exact), 6)
        self.assertEqual({e["outcome"]["value"]["target"]["value"]["value"]["key"] for e in exact},
                         {ids[label]["key"] for label in ["twister-gem", "sniper-gem", "twister-skill", "sniper-skill", "sniper-basic-attack-skill", "sniper-gas-arrow-skill"]})
        twister = next(e for e in exact if e["outcome"]["value"]["target"] == EXPORT.subject(ids["twister-gem"]))
        self.assertEqual(twister["source"]["value"]["value"]["game_id"]["value"], "Metadata/Items/Gem/SkillGemTwister")
        self.assertNotEqual(twister["source"]["value"]["value"]["game_id"]["value"], "Metadata/Items/Gems/SkillGemTwister")

    def test_recipe_mapping_and_policy_identity_chain_is_explicit(self):
        recipe = EXPORT.decode(self.outputs["recipe-seed.json"])
        mapping = EXPORT.decode(self.outputs["mapping-seed.json"])
        identity = EXPORT.schema_identity(recipe["schema"])
        self.assertEqual(mapping["definitions"], identity)
        self.assertEqual(mapping["registry"], EXPORT.owned_digest("owned-id-registry-v1", recipe["registry"]))
        reward = EXPORT.decode(self.outputs["reward-policy-seed.json"])
        self.assertEqual(reward["definitions"], identity)
        self.assertEqual(reward["mapping"], EXPORT.owned_digest("owned-external-mapping-v1", mapping))

    def test_reward_defaults_aliases_and_none_are_retained_without_effect_claim(self):
        policy = EXPORT.decode(self.outputs["reward-policy-seed.json"])
        self.assertEqual(len(policy["rules"]), 17)
        for rule in policy["rules"]:
            self.assertEqual(rule["recipe"]["tiers"][0]["duplicates"], "last_in_source_order")
            self.assertEqual(rule["recipe"]["missing"]["kind"], "explicit")
            self.assertTrue(any(case["outcome"] == {"kind": "none"} for case in rule["outcomes"]))
        mappings = EXPORT.decode(self.outputs["mapping-seed.json"])["entries"]
        defaults = [e for e in mappings if e["source"]["kind"] == "configuration" and e["source"]["value"]["source"] == "default"]
        self.assertTrue(defaults)
        self.assertTrue(all(e["outcome"]["value"]["basis"]["kind"] == "reviewed_alias" for e in defaults))
        self.assertFalse(EXPORT.decode(self.outputs["reward-source-facts.json"])["effects_compiled"])

    def test_quality_loadouts_and_empty_item_boundaries_match_reviewed_scope(self):
        policy = EXPORT.decode(self.outputs["normalization-policy-seed.json"])
        quality = policy["gem_quality"]["value"]
        self.assertEqual(quality["kinds"][0]["kind"]["key"], "def.0000000000000006")
        self.assertEqual(quality["amount"]["codec"]["codec"]["value"]["unit"]["key"], "def.0000000000000002")
        self.assertEqual(quality["amount"]["missing"], {"kind": "pending"})
        self.assertEqual(quality["kinds"][0]["source"], {"kind": "missing"})
        equipment = policy["equipment_loadouts"]
        self.assertEqual(len(equipment), 22)
        self.assertEqual(sum(e["scope"]["kind"] == "shared" for e in equipment), 18)
        self.assertEqual(equipment[0]["destination"], equipment[1]["destination"])
        self.assertNotEqual(equipment[0]["scope"], equipment[1]["scope"])
        self.assertEqual(EXPORT.decode(self.outputs["item-policy-seed.json"])["rules"], [])
        layout = EXPORT.decode(self.outputs["item-source-policy-seed.json"])
        self.assertEqual((layout["rule_layouts"], layout["template_layouts"]), ([], []))
        self.assertEqual(layout["source"], EXPORT.decode(self.outputs["source-pin.json"]))
        self.assertEqual(layout["schema_version"], 3)
        self.assertEqual(layout["property_bindings"], [])
        self.assertEqual(layout["template_defaults"], [])
        items = EXPORT.decode(self.outputs["item-policy-seed.json"])
        self.assertEqual(items["schema_version"], 2)
        self.assertEqual(layout["item_lines"], EXPORT.owned_digest("owned-item-line-policy-v2", items))
        self.assertNotEqual(layout["item_lines"], EXPORT.owned_digest("owned-item-line-policy-v1", items))

    def test_only_query_identity_actor_and_order_are_consumed(self):
        manifest = EXPORT.decode(self.inputs[6])
        for case in manifest["cases"]:
            for row in case["measurements"]:
                row["value"] = 999999999
                row["unrelated_result_payload"] = {"not_an_input": True}
        altered = self.produce_changed(6, manifest)
        for name, original in self.outputs.items():
            if name != "provenance.json":
                self.assertEqual(altered[name], original, name)
        rows = [EXPORT.decode(self.outputs[f"queries/original-{i:02}.json"]) for i in range(1, 6)]
        self.assertEqual([len(r) for r in rows], [22] * 5)
        for case, projected in zip(manifest["cases"], rows, strict=True):
            expected = ["player" if m["query"]["actor"] == "player" else "unresolved" for m in case["measurements"]]
            self.assertEqual([r["target"]["kind"] for r in projected], expected)
            self.assertEqual([r["metric"]["value"]["key"]["value"] for r in projected],
                             [m["query"]["id"] for m in case["measurements"]])

    def test_seed_selector_does_not_accept_display_or_catalog_key_as_external_id(self):
        authoring = EXPORT.decode(self.inputs[7])
        authoring["seed_bindings"][0]["source"]["value"]["value"]["game_id"] = EXPORT.text("Metadata/Items/Gems/SkillGemTwister")
        with self.assertRaisesRegex(ValueError, "exact active catalog identity"):
            self.produce_changed(7, authoring)

    def test_foreign_or_stale_source_and_base_bindings_are_rejected(self):
        base = EXPORT.decode(self.inputs[1])
        base["registry"]["revision"] += 1
        with self.assertRaisesRegex(ValueError, "binding mismatch"):
            self.produce_changed(1, base)
        authoring = EXPORT.decode(self.inputs[7])
        authoring["extra_source_files"][0]["sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            self.produce_changed(7, authoring)

    def test_exact_source_paths_have_no_crlf_or_unrooted_alias(self):
        pin = EXPORT.decode(self.outputs["source-pin.json"])
        self.assertEqual(len(pin["files"]), 29)
        self.assertTrue(all(f["path"].startswith("src/") for f in pin["files"]))
        skills = [f for f in pin["files"] if f["path"].endswith("Classes/SkillsTab.lua")]
        self.assertEqual(len(skills), 1)
        self.assertEqual(skills[0]["sha256"], "ce6ca9b06d8ff66fe39f3fc15b511c8d544a90807fcfbebb90322d589a0d0e08")
        authoring = EXPORT.decode(self.inputs[7])
        authoring["extra_source_files"][0]["path"] = "Classes/Item.lua"
        with self.assertRaisesRegex(ValueError, "exact src/"):
            self.produce_changed(7, authoring)

    def test_wire_and_aggregate_resource_limits_reject_before_publication(self):
        with self.assertRaisesRegex(ValueError, "duplicate JSON"):
            EXPORT.decode(b'{"x":1,"x":2}')
        for limits in [replace(EXPORT.HARD, seed_rows=118), replace(EXPORT.HARD, reward_options=29),
                       replace(EXPORT.HARD, source_files=28), replace(EXPORT.HARD, queries=109),
                       replace(EXPORT.HARD, output_bytes=100), replace(EXPORT.HARD, catalog_rows=100)]:
            with self.assertRaises(ValueError):
                EXPORT.produce(*self.inputs, self.source, limits)
        with self.assertRaisesRegex(ValueError, "invalid"):
            EXPORT.produce(*self.inputs, self.source, replace(EXPORT.HARD, queries=0))


    def test_bound_policies_reproduce_after_checked_real_successor(self):
        compiled = EXPORT.read_compiled(DATA / "import/compiled")
        policies = EXPORT.bind_policies(self.outputs, compiled)
        for name, data in policies.items():
            self.assertEqual(data, (DATA / "import" / name).read_bytes(), name)
        transition = EXPORT.decode(policies["policies/transition.json"])
        self.assertNotEqual(transition["before_definitions"], transition["after_definitions"])
        self.assertEqual((transition["preserved_definition_count"], transition["preserved_slot_count"]), (105, 14))

    def test_policy_rebinding_rejects_stale_seed_and_changed_mapping_even_if_rehashed(self):
        compiled = EXPORT.read_compiled(DATA / "import/compiled")
        seed = dict(self.outputs)
        policy = EXPORT.decode(seed["reward-policy-seed.json"])
        policy["mapping"] = "0" * 64
        seed["reward-policy-seed.json"] = EXPORT.pretty(policy)
        with self.assertRaisesRegex(ValueError, "exact seed dependencies"):
            EXPORT.bind_policies(seed, compiled)
        changed = dict(compiled)
        mapping = EXPORT.decode(changed["mapping.json"])
        previous = EXPORT.decode(self.outputs["mapping-seed.json"])["entries"][0]
        row = next(r for r in mapping["entries"] if r["source"] == previous["source"])
        row["outcome"]["value"]["basis"] = EXPORT.tag("reviewed_alias", {"reason": "changed"})
        changed["mapping.json"] = EXPORT.pretty(mapping)
        transition = EXPORT.decode(changed["transition.json"])
        transition["after_mapping"] = EXPORT.owned_digest("owned-external-mapping-v1", mapping)
        for artifact in transition["artifacts"]:
            raw = changed[artifact["file"]]
            artifact["bytes"], artifact["sha256"] = len(raw), EXPORT.sha(raw)
        changed["transition.json"] = EXPORT.pretty(transition)
        with self.assertRaisesRegex(ValueError, "changed existing source mapping"):
            EXPORT.bind_policies(self.outputs, changed)
        self.assertEqual(compiled, EXPORT.read_compiled(DATA / "import/compiled"))

    def test_policy_rebinding_checks_publication_hashes_and_combined_bounds(self):
        compiled = EXPORT.read_compiled(DATA / "import/compiled")
        changed = dict(compiled)
        changed["rules.json"] += b" "
        with self.assertRaisesRegex(ValueError, "artifact hash mismatch"):
            EXPORT.bind_policies(self.outputs, changed)
        with self.assertRaisesRegex(ValueError, "byte bound"):
            EXPORT.bind_policies(self.outputs, compiled, replace(EXPORT.HARD, output_bytes=100))


if __name__ == "__main__":
    unittest.main()
