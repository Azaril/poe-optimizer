"""Offline resistance authoring checks; no source execution or build evaluation."""
import copy
import importlib.util
import json
from pathlib import Path
import sys
import unittest
from unittest import mock
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("resistance_export", ROOT / "scripts/export-owned-resistance.py")
EXPORT = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = EXPORT
SPEC.loader.exec_module(EXPORT)
DATA = ROOT / "data/owned/poe2/3887ae68"
IMPORT = DATA / "import"
DEST = DATA / "resistance"
SOURCE = ROOT / "vendor/path-of-building-poe2"


class ResistanceExportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.outputs = EXPORT.produce(IMPORT / "compiled", IMPORT, DEST / "authoring.json", SOURCE)
        cls.recipe = EXPORT.DATA.decode(cls.outputs["recipe.json"])
        cls.base = EXPORT.load(IMPORT / "compiled/recipe.json")
        cls.ids = EXPORT.DATA.decode(cls.outputs["ids.json"])["allocations"]

    def changed_authoring(self, mutate):
        value = EXPORT.load(DEST / "authoring.json")
        mutate(value)
        original = EXPORT.load
        with mock.patch.object(EXPORT, "load", side_effect=lambda p: value if p == DEST / "authoring.json" else original(p)):
            return EXPORT.produce(IMPORT / "compiled", IMPORT, DEST / "authoring.json", SOURCE)

    def test_exact_reproduction_and_combined_policy_publication(self):
        self.assertEqual(set(self.outputs), {"recipe.json", "ids.json", "items.json", "item-source.json", "source-facts.json"})
        for name, raw in self.outputs.items():
            self.assertEqual(raw, (DEST / name).read_bytes(), name)
        self.assertFalse((DEST / "items-fixed.json").exists())
        self.assertFalse((DEST / "item-source-fixed.json").exists())

    def test_successor_preserves_existing_history_schemas_and_rule_bodies(self):
        self.assertEqual(self.recipe["registry"]["entries"][:2515], self.base["registry"]["entries"])
        self.assertEqual(self.recipe["registry"]["last_issued"], 2524)
        self.assertEqual(len(self.ids), 9)
        for table in ["definitions", "slots"]:
            for row in self.base["schema"][table]:
                self.assertIn(row, self.recipe["schema"][table])
        for owner in self.base["rules"]["owners"]:
            self.assertIn(owner, self.recipe["rules"]["owners"])
        for field in set(self.base["rules"]) - {"definitions", "owners"}:
            self.assertEqual(self.recipe["rules"][field], self.base["rules"][field])
        before, after = copy.deepcopy(self.base["routing"]), copy.deepcopy(self.recipe["routing"])
        before.pop("definitions")
        after.pop("definitions")
        self.assertEqual(before, after)
        self.assertEqual(self.ids["cold-resistance-base-contributions"]["key"], "def.00000000000009d4")
        self.assertEqual(self.ids["elemental-resistance-base-contributions"]["key"], "def.00000000000009d5")

    def test_new_numerical_rules_only_contribute_on_player(self):
        old = len(self.base["rules"]["owners"])
        owners = self.recipe["rules"]["owners"][old:]
        self.assertEqual(len(owners), 8)
        effects = [e["effect"] for o in owners for p in o["programs"]["members"] for e in p["effects"]]
        self.assertEqual(len(effects), 6)
        for owner in owners:
            if owner["owner"]["value"]["kind"] == "modifier":
                for program in owner["programs"]["members"]:
                    self.assertEqual(program["nodes"], [{"id": "amount", "expression": {"kind": "read", "input": "amount"}}])
        self.assertTrue(all(e["kind"] == "contribute" and e["entity"] == "player" and e["contribution"] == "add" for e in effects))
        self.assertEqual({e["stat"]["key"] for e in effects}, {"def.00000000000009d4", "def.00000000000009d5"})
        extra = self.recipe["registry"]["entries"][2515:]
        self.assertFalse(any(e["target"]["value"]["kind"] == "metric" for e in extra))

    def test_mixed_reward_and_unconverted_item_effects_remain_partial(self):
        owners = self.recipe["rules"]["owners"]
        by_id = {o["owner"]["value"]["value"]["key"]: o for o in owners if o["owner"]["kind"] == "definition"}
        self.assertEqual(by_id["def.0000000000000062"]["programs"]["closure"]["kind"], "partial")
        for label in ["flat-life-modifier", "sapphire-ring-template"]:
            row = by_id[self.ids[label]["key"]]
            self.assertEqual(row["programs"]["members"], [])
            self.assertEqual(row["programs"]["closure"]["kind"], "partial")
        reward_values = [o["programs"]["members"][0]["nodes"][0]["expression"]["value"]["value"]["value"] for o in owners if o["owner"]["value"]["kind"] == "reward"]
        self.assertEqual(reward_values, [10.0, 5.0, 5.0, -5.0])

    def test_fixed_policy_uses_generic_signed_integer_captures_and_exact_bindings(self):
        items = EXPORT.DATA.decode(self.outputs["items.json"])
        source = EXPORT.DATA.decode(self.outputs["item-source.json"])
        self.assertEqual(items["definitions"], EXPORT.DATA.schema_identity(self.recipe["schema"]))
        self.assertEqual(source["item_lines"], EXPORT.DATA.owned_digest("owned-item-line-policy-v1", items))
        for rule in [r for r in items["rules"] if r["id"].startswith("fixed-")]:
            self.assertEqual(rule["pattern"][0], {"kind": "numeric_capture", "value": {"capture": "amount", "syntax": "integer", "sign": "optional"}})
            self.assertEqual(rule["captures"][0]["codec"]["value"]["codec"]["value"]["syntax"], "integer")
            self.assertEqual(rule["emissions"][0]["value"]["rolls"][0]["value"], {"kind": "capture", "value": "amount"})
        self.assertFalse(any(e["kind"] in ["item_level", "quality"] for r in items["rules"] for e in r["emissions"]))

    def test_range_endpoints_use_exact_source_sign_grammar_and_shared_ids(self):
        items = EXPORT.DATA.decode(self.outputs["items.json"])
        ranges = [r for r in items["rules"] if r["id"].startswith("ranged-")]
        self.assertEqual(len(ranges), 4)
        for rule in ranges:
            captures = [p["value"] for p in rule["pattern"] if p["kind"] == "numeric_capture"]
            self.assertEqual(captures, [{"capture": "lower", "syntax": "integer", "sign": "optional_minus"},
                                        {"capture": "upper", "syntax": "integer", "sign": "optional_minus"}])
            effect = rule["emissions"][0]["value"]
            self.assertIn(effect["definition"], [self.ids["flat-cold-modifier"], self.ids["flat-elemental-modifier"]])
            self.assertEqual(effect["rolls"][0]["value"]["value"]["rounding"], "symmetric_half_offset")
        self.assertEqual(EXPORT.sha(self.outputs["recipe.json"]), "34e8902225acdd95f2dcdfc34d88c0d3d3a815a313354181ee95304a3e53ddf0")

    def test_source_hashes_and_literal_signed_rounding_are_explicit(self):
        facts = EXPORT.DATA.decode(self.outputs["source-facts.json"])
        self.assertTrue(facts["range_conversion"]["implemented"])
        self.assertFalse(facts["receiver"]["implemented"])
        self.assertFalse(facts["metric_producer"])
        self.assertEqual(facts["original_ring_attribution"]["status"], "pending")
        self.assertIn("exactly this one literal tag", facts["original_ring_attribution"]["diagnostic_only"])
        self.assertEqual(facts["whole_original_native_completion"], "0/5")
        for span in facts["source_spans"]:
            raw = (SOURCE / span["path"]).read_bytes().replace(b"\r\n", b"\n")
            self.assertEqual(EXPORT.sha(raw), span["file_sha256"])
            self.assertEqual(EXPORT.sha(span["text"].encode()), span["sha256"])
        common = next(s for s in facts["source_spans"] if s["path"] == "src/Modules/Common.lua")
        self.assertIn("m_ceil(val - 0.5)", common["text"])

    def test_original_ring_is_one_backing_record_with_eight_distinct_receiving_rows(self):
        raw = (ROOT / "tests/fixtures/builds/breadth-20260908/build-05.xml").read_bytes()
        tree = ET.fromstring(raw)
        item = next(i for i in tree.iter("Item") if i.get("id") == "26")
        self.assertIn("Sapphire Ring", item.text)
        self.assertIn("+(20-30)% to Cold Resistance", item.text)
        self.assertIn("{range:0.5}", item.text)
        self.assertEqual([(r.get("id"), r.get("range")) for r in item.findall("ModRange")], [("1", "0.5"), ("2", "0.5")])
        uses = [(s.get("id"), r.get("name")) for s in tree.iter("ItemSet") for r in s.findall("Slot") if r.get("itemId") == "26"]
        self.assertEqual(len(uses), 8)
        self.assertEqual(len(set(uses)), 8)
        second = ET.fromstring((ROOT / "tests/fixtures/builds/breadth-20260908/build-02.xml").read_bytes())
        self.assertIn("Grand Spear", next(i.text for i in second.iter("Item") if i.get("id") == "26"))

    def test_stale_source_base_and_reward_authoring_reject(self):
        changes = [lambda a: a.update(base_recipe_sha256="0" * 64),
                   lambda a: a["source_spans"][0].update(file_sha256="0" * 64),
                   lambda a: a["source_spans"][0].update(path="../outside.lua"),
                   lambda a: a["reward_contributions"][0]["values"][0].update(amount=999)]
        for change in changes:
            with self.assertRaises(ValueError):
                self.changed_authoring(change)

    def test_collection_limit_rejects_before_source_reads_and_no_nonfinite_wire(self):
        with self.assertRaisesRegex(ValueError, "collection bound"):
            self.changed_authoring(lambda a: a.update(source_spans=a["source_spans"] * 4))
        with self.assertRaises(ValueError):
            EXPORT.DATA.decode(b'{"value":NaN}')
        with self.assertRaises(ValueError):
            EXPORT.DATA.decode(b'{"value":1,"value":2}')


if __name__ == "__main__":
    unittest.main()
