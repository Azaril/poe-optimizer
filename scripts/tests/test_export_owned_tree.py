"""Finite exporter acceptance and rejection; no original builds are compiler inputs."""
from dataclasses import replace
import copy
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location("owned_tree_export",ROOT/"scripts/export-owned-tree.py")
EXPORT=importlib.util.module_from_spec(spec);sys.modules[spec.name]=EXPORT;spec.loader.exec_module(EXPORT)
DATA=ROOT/"data/owned/poe2/3887ae68/tree"
SOURCE=ROOT/"vendor/path-of-building-poe2"

class TreeExportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest=EXPORT.decode((DATA/"source-manifest.json").read_bytes())
        cls.outputs=EXPORT.produce((DATA/"source-manifest.json").read_bytes(),SOURCE)
        cls.tree=EXPORT.decode((SOURCE/cls.manifest["tree_file"]).read_bytes())
    def changed(self,mutate,manifest_mutate=lambda x:None):
        tree=copy.deepcopy(self.tree);mutate(tree)
        manifest=copy.deepcopy(self.manifest);manifest_mutate(manifest)
        with tempfile.TemporaryDirectory(prefix="owned-tree-export-") as directory:
            root=Path(directory)
            for pin in manifest["source"]["files"]:
                p=root/pin["path"];p.parent.mkdir(parents=True,exist_ok=True)
                data=EXPORT.pretty(tree) if pin["path"]==manifest["tree_file"] else (SOURCE/pin["path"]).read_bytes().replace(b"\r\n",b"\n")
                p.write_bytes(data);pin["sha256"]=EXPORT.sha(data)
            return EXPORT.produce(EXPORT.pretty(manifest),root)
    def test_persisted_export_reproduces_exactly(self):
        for name,data in self.outputs.items():self.assertEqual(data,(DATA/name).read_bytes())
    def test_full_catalog_roots_choices_edges_and_explicit_missing_links(self):
        c=EXPORT.decode(self.outputs["tree-catalog.json"]);nodes={n["key"]:n for n in c["nodes"]}
        self.assertEqual(len(nodes),4914);self.assertEqual(len(c["classes"]),8)
        self.assertEqual(sum(len(x["ascendancies"]) for x in c["classes"]),23)
        self.assertEqual(sum(n["kind"]["kind"]=="implicit_root" for n in nodes.values()),28)
        self.assertEqual(sum(n["kind"]["kind"]=="attribute" for n in nodes.values()),293)
        self.assertEqual(sum(n["kind"]["kind"]=="attached_choice" for n in nodes.values()),15)
        self.assertEqual(nodes["37397"]["kind"]["value"]["parent"],"60287")
        self.assertEqual(len(c["edges"]),5187);self.assertEqual(len(c["unresolved_edges"]),14)
        self.assertTrue(all(e["right"] not in nodes for e in c["unresolved_edges"]))
        self.assertEqual(EXPORT.decode(self.outputs["source-facts.json"])["self_connections_excluded"],["35653"])
        roots=[a["root"] for cl in c["classes"] for a in cl["ascendancies"] if a["key"] in ["Witch3","Witch3b"]]
        self.assertEqual(roots,["23710","23710"])
    def test_unknown_semantic_node_field_rejects_after_repin(self):
        with self.assertRaisesRegex(ValueError,"unreviewed fields"):
            self.changed(lambda t:t["nodes"]["4739"].update(newSemantic=True))
    def test_unknown_class_field_rejects_after_repin(self):
        with self.assertRaisesRegex(ValueError,"unreviewed fields"):
            self.changed(lambda t:t["classes"][0].update(newRule=1))
    def test_conflicting_attribute_outcomes_reject(self):
        def mutate(t):
            n=next(n for n in t["nodes"].values() if n.get("isAttribute"));n["options"][0]["stats"].append("unreviewed outcome")
        with self.assertRaisesRegex(ValueError,"different option semantics"):self.changed(mutate)
    def test_duplicate_attribute_lane_rejects(self):
        with self.assertRaisesRegex(ValueError,"duplicate attribute lane"):
            self.changed(lambda t:None,lambda m:m["attribute_lanes"].append(m["attribute_lanes"][0]))
    def test_orphaned_option_rejects(self):
        def mutate(t):
            t["nodes"]["37397"]["connections"]=[]
            for n in t["nodes"].values():n["connections"]=[e for e in n["connections"] if e["id"]!=37397]
        with self.assertRaisesRegex(ValueError,"one parent"):self.changed(mutate)
    def test_mismatched_numeric_identity_rejects(self):
        with self.assertRaisesRegex(ValueError,"node key mismatch"):self.changed(lambda t:t["nodes"]["4739"].update(skill=1))
    def test_source_pin_and_duplicate_json_are_strict(self):
        m=copy.deepcopy(self.manifest);m["source"]["files"][0]["sha256"]="0"*64
        with self.assertRaisesRegex(ValueError,"source pin differs"):EXPORT.produce(EXPORT.pretty(m),SOURCE)
        raw=(DATA/"source-manifest.json").read_bytes().replace(b"{",b'{"schema_version":1,',1)
        with self.assertRaisesRegex(ValueError,"duplicate JSON key"):EXPORT.produce(raw,SOURCE)
    def test_budgets_reject_before_unbounded_expansion(self):
        for limits in [replace(EXPORT.HARD,rows=100),replace(EXPORT.HARD,entries=100),replace(EXPORT.HARD,output_bytes=100),replace(EXPORT.HARD,input_bytes=100),replace(EXPORT.HARD,total_source_bytes=100),replace(EXPORT.HARD,rows=0)]:
            with self.assertRaises(ValueError):EXPORT.produce((DATA/"source-manifest.json").read_bytes(),SOURCE,limits)
    def test_streaming_output_stops_before_visiting_later_values(self):
        # The later object is deliberately unserializable. A full-document
        # serializer would raise TypeError before checking the output ceiling.
        with patch.object(EXPORT.json,"dumps",side_effect=AssertionError("must stream")):
            with self.assertRaisesRegex(ValueError,"output byte bound"):
                EXPORT.bounded_pretty(["x"*64,object()],16)
        value={"unicode":"\u2603","rows":[1,2]};expected=EXPORT.pretty(value)
        self.assertEqual(EXPORT.bounded_pretty(value,len(expected)),expected)
        with self.assertRaisesRegex(ValueError,"output byte bound"):
            EXPORT.bounded_pretty(value,len(expected)-1)

    def test_output_budget_is_shared_across_both_artifacts(self):
        exact=sum(map(len,self.outputs.values()))
        manifest=(DATA/"source-manifest.json").read_bytes()
        with patch.object(EXPORT.json,"dumps",side_effect=AssertionError("must stream")):
            self.assertEqual(EXPORT.produce(manifest,SOURCE,replace(EXPORT.HARD,output_bytes=exact)),self.outputs)
            with self.assertRaisesRegex(ValueError,"output byte bound"):
                EXPORT.produce(manifest,SOURCE,replace(EXPORT.HARD,output_bytes=exact-1))

    def test_inherited_stat_memberships_charge_each_view_before_expansion(self):
        def mutate(t):
            n=t["nodes"]["4739"]
            n["stats"]=["repeated source stat"]*500
            n["isSwitchable"]=True
            n["options"]={"view-"+str(i):{} for i in range(500)}
        # Only about one thousand extra source entries, but 250,000 emitted
        # memberships. Repeated references cannot bypass the 200,000 cap.
        with self.assertRaisesRegex(ValueError,"aggregate tree entry bound"):
            self.changed(mutate)

    def test_source_budget_counts_raw_crlf_bytes_before_normalization(self):
        manifest=(DATA/"source-manifest.json").read_bytes()
        with tempfile.TemporaryDirectory(prefix="owned-tree-raw-budget-") as directory:
            root=Path(directory);raw_total=0;normalized_total=0
            for pin in self.manifest["source"]["files"]:
                normalized=(SOURCE/pin["path"]).read_bytes().replace(b"\r\n",b"\n")
                raw=normalized.replace(b"\n",b"\r\n")
                path=root/pin["path"];path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(raw)
                raw_total+=len(raw);normalized_total+=len(normalized)
            self.assertGreater(raw_total,normalized_total)
            self.assertEqual(EXPORT.produce(manifest,root,replace(EXPORT.HARD,total_source_bytes=raw_total)),self.outputs)
            with self.assertRaisesRegex(ValueError,"input byte bound"):
                EXPORT.produce(manifest,root,replace(EXPORT.HARD,total_source_bytes=raw_total-1))

    def test_class_view_and_unlock_are_evidence_not_compiled_rules(self):
        c=EXPORT.decode(self.outputs["tree-catalog.json"]);n=next(n for n in c["nodes"] if n["key"]=="4739")
        self.assertEqual(n["stats"],["10% increased Spell Damage"])
        self.assertTrue(any(v["selector"]=="Witch" and len(v["stats"])==2 for v in n["views"]))
        n=next(n for n in c["nodes"] if n["key"]=="42078");self.assertEqual(set(n["unlock"]),{"59657","49356"})
        facts=EXPORT.decode(self.outputs["source-facts.json"])
        self.assertFalse(facts["source_execution"]);self.assertFalse(facts["numerical_rules_compiled"]);self.assertFalse(facts["legality_established"])
    def test_source_path_escape_rejects(self):
        m=copy.deepcopy(self.manifest);m["source"]["files"][0]["path"]="../outside.json"
        with self.assertRaisesRegex(ValueError,"source path"):EXPORT.produce(EXPORT.pretty(m),SOURCE)

if __name__=="__main__":unittest.main()
