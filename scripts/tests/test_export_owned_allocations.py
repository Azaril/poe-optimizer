"""Offline cost export laws; no Lua, original builds, or native runtime execution."""
from contextlib import contextmanager
from dataclasses import replace
import copy
import importlib.util
import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("owned_allocations_export", ROOT / "scripts/export-owned-allocations.py")
EXPORT = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = EXPORT
SPEC.loader.exec_module(EXPORT)
BASE = ROOT / "data/owned/poe2/3887ae68"
SOURCE = ROOT / "vendor/path-of-building-poe2"


class AllocationExportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest = (BASE / "tree/source-manifest.json").read_bytes()
        cls.policy = (BASE / "allocations/source-policy.json").read_bytes()
        cls.catalog = (BASE / "tree/tree-catalog.json").read_bytes()
        cls.outputs = cls.produce()
        cls.rules = EXPORT.decode(cls.outputs["rules.json"])
        cls.facts = EXPORT.decode(cls.outputs["source-facts.json"])
        cls.tree = EXPORT.decode((SOURCE / EXPORT.decode(cls.manifest)["tree_file"]).read_bytes())
        cls.roles = EXPORT.decode((BASE / "current/tree-normalization.json").read_bytes())

    @classmethod
    def produce(cls, *, manifest=None, policy=None, catalog=None, source=SOURCE, bundle=BASE / "current", limits=EXPORT.HARD):
        return EXPORT.produce(cls.manifest if manifest is None else manifest, cls.policy if policy is None else policy, cls.catalog if catalog is None else catalog, source, bundle, limits)

    @contextmanager
    def changed_bundle(self, mutate):
        # Rebind precisely the consumed identity graph after a typed mutation.
        # These adversarial fixtures do not claim a complete successor publication.
        with tempfile.TemporaryDirectory(prefix="owned-allocation-bundle-") as directory:
            bundle = Path(directory)
            for source in (BASE / "current").glob("*.json"):
                shutil.copyfile(source, bundle / source.name)
            names = ["registry", "mapping", "tree-normalization", "manifest", "transition"]
            docs = {n: EXPORT.decode((bundle / (n + ".json")).read_bytes()) for n in names}
            mutate(docs)
            registry = EXPORT.digest("owned-id-registry-v1", EXPORT.compact(docs["registry"]))
            docs["mapping"]["registry"] = registry
            mapping = EXPORT.digest("owned-external-mapping-v1", EXPORT.compact(docs["mapping"]))
            docs["tree-normalization"].update(registry=registry, mapping=mapping)
            tree = EXPORT.digest("owned-tree-normalization-policy-v1", EXPORT.compact(docs["tree-normalization"]))
            docs["transition"]["after"].update(registry=registry, mapping=mapping)
            docs["transition"]["tree"] = tree
            docs["manifest"]["registry"] = registry
            for n in ["registry", "mapping", "tree-normalization"]:
                (bundle / (n + ".json")).write_bytes(EXPORT.compact(docs[n]))
            def refresh(rows):
                for row in rows:
                    data = (bundle / row["file"]).read_bytes()
                    row.update(bytes=len(data), sha256=EXPORT.sha(data))
            refresh(docs["manifest"]["artifacts"])
            (bundle / "manifest.json").write_bytes(EXPORT.compact(docs["manifest"]))
            refresh(docs["transition"]["artifacts"])
            (bundle / "transition.json").write_bytes(EXPORT.compact(docs["transition"]))
            yield bundle

    def test_shipped_outputs_reproduce_byte_for_byte(self):
        for name, data in self.outputs.items():
            self.assertEqual(data, (BASE / "allocations" / name).read_bytes())

    def test_costs_cover_physical_allocations_and_exclude_roots_options_images(self):
        roles = {r["token"]: r["role"] for r in self.roles["content"]["tokens"]}
        expected = {(r["value"]["node"]["key"], r["value"]["pool"]["key"]) for r in roles.values() if r["kind"] == "allocation"}
        actual = {(r["node"]["key"], r["pool"]["key"]) for r in self.rules["costs"]}
        self.assertEqual(actual, expected)
        self.assertEqual(len(actual), 4503)
        self.assertEqual(len(actual), len(self.rules["costs"]))
        self.assertEqual(set(self.facts["zero_cost_source_tokens"]), {"8415", "9988", "28254"})
        self.assertEqual(sum(r["points"] == 0 for r in self.rules["costs"]), 3)
        self.assertEqual(self.facts["excluded_source_rows"], {"implicit_root": 28, "attached_choice": 15, "unsupported": 368})

    def test_free_cost_tracks_source_presence_without_rounding_or_level_defaults(self):
        node = copy.deepcopy(self.tree["nodes"]["8415"])
        self.assertEqual(EXPORT.source_cost(node), 0)
        node.pop("isFreeAllocate")
        self.assertEqual(EXPORT.source_cost(node), 1)
        node["isFreeAllocate"] = False
        # The pinned accounting branch tests nil; false is also non-nil.
        self.assertEqual(EXPORT.source_cost(node), 0)
        self.assertEqual(EXPORT.source_cost(self.tree["nodes"]["42078"]), 1)

    def test_coupled_budgets_have_explicit_unknown_acquired_capacities(self):
        rows = self.rules["budgets"]["members"]
        self.assertEqual([r["usage"] for r in rows], [{"kind": "shared_plus_maximum_scoped"}, {"kind": "each_scope", "include_shared": False}, {"kind": "total"}])
        self.assertEqual(rows[0]["pools"], rows[1]["pools"])
        self.assertNotEqual(rows[1]["pools"], rows[2]["pools"])
        for row in rows:
            self.assertEqual(row["capacity"]["kind"], "unmapped")
            self.assertEqual(row["capacity"]["value"]["gaps"][0]["code"], "acquired-capacity-not-converted")
        self.assertEqual(self.rules["budgets"]["closure"]["kind"], "partial")
        self.assertEqual(self.facts["access"], "outside_this_artifact")
        self.assertFalse(self.facts["source_execution"])
        self.assertFalse(self.facts["legality_established"])
        self.assertNotIn("source", self.rules)
        self.assertEqual(self.facts["artifact"]["sha256"], EXPORT.sha(self.outputs["rules.json"]))

    def test_unknown_policy_fields_and_bool_versions_reject(self):
        for edit in [lambda p: p.update(extra_rule=1), lambda p: p.update(schema_version=True), lambda p: p.update(cost_algorithm="other")]:
            policy = EXPORT.decode(self.policy)
            edit(policy)
            with self.assertRaises(ValueError):
                self.produce(policy=EXPORT.compact(policy))
        with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
            self.produce(policy=self.policy.replace(b"{", b'{"schema_version":1,', 1))

    def test_unknown_source_fields_reject_even_after_repin(self):
        tree = copy.deepcopy(self.tree)
        tree["nodes"]["8415"]["newPointCost"] = 2
        manifest, policy = EXPORT.decode(self.manifest), EXPORT.decode(self.policy)
        with tempfile.TemporaryDirectory(prefix="owned-allocation-source-") as directory:
            source = Path(directory)
            for pin in policy["source"]["files"]:
                raw = EXPORT.compact(tree) if pin["path"] == manifest["tree_file"] else (SOURCE / pin["path"]).read_bytes().replace(b"\r\n", b"\n")
                p = source / pin["path"]
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_bytes(raw)
                pin["sha256"] = EXPORT.sha(raw)
            hashes = {p["path"]: p["sha256"] for p in policy["source"]["files"]}
            for pin in manifest["source"]["files"]:
                pin["sha256"] = hashes[pin["path"]]
            with self.assertRaisesRegex(ValueError, "unreviewed fields"):
                self.produce(manifest=EXPORT.compact(manifest), policy=EXPORT.compact(policy), source=source)

    def test_pins_paths_and_source_context_reject(self):
        edits = [lambda p: p["source"]["files"][0].update(sha256="0" * 64), lambda p: p["source"]["files"].append(p["source"]["files"][0]), lambda p: p["source"].update(revision="f" * 40), lambda p: p["source"]["files"][0].update(path="../outside.lua")]
        for edit in edits:
            p = EXPORT.decode(self.policy)
            edit(p)
            with self.assertRaises(ValueError):
                self.produce(policy=EXPORT.compact(p))

    def test_bundle_manifest_hash_tampering_rejects(self):
        with self.changed_bundle(lambda _: None) as bundle:
            p = bundle / "schema.json"
            p.write_bytes(p.read_bytes() + b" ")
            with self.assertRaisesRegex(ValueError, "artifact bytes/hash differ"):
                self.produce(bundle=bundle)

    def test_duplicate_role_and_positive_mapping_requirements(self):
        def duplicate(d):
            d["tree-normalization"]["content"]["tokens"].append(d["tree-normalization"]["content"]["tokens"][0])
        with self.changed_bundle(duplicate) as bundle:
            with self.assertRaisesRegex(ValueError, "duplicate tree token"):
                self.produce(bundle=bundle)
        def unmapped(d):
            row = next(r for r in d["mapping"]["entries"] if r["source"].get("value", {}).get("kind") == "passive_node")
            row["outcome"] = {"kind": "unmapped", "value": {"reason": "not-converted"}}
        with self.changed_bundle(unmapped) as bundle:
            with self.assertRaisesRegex(ValueError, "positive exact mapping"):
                self.produce(bundle=bundle)

    def test_role_domain_namespace_and_pool_conflicts_reject(self):
        def edit(d, which):
            row = next(r["role"]["value"] for r in d["tree-normalization"]["content"]["tokens"] if r["role"]["kind"] == "allocation")
            if which == "namespace": row["node"]["namespace"]["version"] = "foreign"
            elif which == "kind": row["node"]["kind"] = "skill"
            else: row["pool"]["key"] = "def.unknown"
        for which in ["namespace", "kind", "pool"]:
            with self.changed_bundle(lambda d: edit(d, which)) as bundle:
                with self.assertRaisesRegex(ValueError, "identity differs|point pool differs"):
                    self.produce(bundle=bundle)

    def test_stale_catalog_and_base_policy_binding_reject(self):
        catalog = EXPORT.decode(self.catalog)
        catalog["nodes"][0]["stats"] = ["changed known text"]
        with self.assertRaisesRegex(ValueError, "classified catalog differs"):
            self.produce(catalog=EXPORT.compact(catalog))
        def stale(d): d["tree-normalization"]["normalization"] = "0" * 64
        with self.changed_bundle(stale) as bundle:
            with self.assertRaisesRegex(ValueError, "policy binding differs"):
                self.produce(bundle=bundle)

    def test_limits_reject_input_work_cost_and_budget_expansion(self):
        for limits in [replace(EXPORT.HARD, input_bytes=20), replace(EXPORT.HARD, bundle_bytes=100), replace(EXPORT.HARD, source_bytes=100), replace(EXPORT.HARD, entries=100), replace(EXPORT.HARD, costs=4502), replace(EXPORT.HARD, budgets=2), replace(EXPORT.HARD, files=1), replace(EXPORT.HARD, entries=0)]:
            with self.assertRaises(ValueError):
                self.produce(limits=limits)
        self.assertEqual(self.produce(limits=replace(EXPORT.HARD, costs=4503)), self.outputs)

    def test_output_bound_shared_and_streamed(self):
        exact = sum(map(len, self.outputs.values()))
        self.assertEqual(self.produce(limits=replace(EXPORT.HARD, output_bytes=exact)), self.outputs)
        with self.assertRaisesRegex(ValueError, "output byte bound"):
            self.produce(limits=replace(EXPORT.HARD, output_bytes=exact - 1))
        with patch.object(EXPORT.TREE.json, "dumps", side_effect=AssertionError("must stream")):
            with self.assertRaisesRegex(ValueError, "output byte bound"):
                EXPORT.TREE.bounded_pretty(["x" * 64, object()], 10)

    def test_existing_output_directory_is_not_overwritten(self):
        import subprocess
        with tempfile.TemporaryDirectory(prefix="owned-allocation-no-clobber-") as directory:
            out = Path(directory) / "existing"
            out.mkdir()
            sentinel = out / "rules.json"
            sentinel.write_bytes(b"keep existing")
            result = subprocess.run([sys.executable, str(ROOT / "scripts/export-owned-allocations.py"), "--manifest", str(BASE / "tree/source-manifest.json"), "--policy", str(BASE / "allocations/source-policy.json"), "--catalog", str(BASE / "tree/tree-catalog.json"), "--source-root", str(SOURCE), "--bundle", str(BASE / "current"), "--output-dir", str(out)], capture_output=True, text=True, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(sentinel.read_bytes(), b"keep existing")
            self.assertEqual(list(out.iterdir()), [sentinel])


if __name__ == "__main__":
    unittest.main()
