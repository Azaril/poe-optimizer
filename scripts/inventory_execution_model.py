#!/usr/bin/env python3
"""Inventory committed execution-model code without executing Cargo or source code.

Python 3.11+ and Git are required; all other dependencies are standard library.
Counts describe physical files, not semantic complexity, maintenance effort or speed.
"""
from __future__ import annotations

import argparse
from collections import defaultdict
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tomllib

SCHEMA_VERSION = 1


@dataclass(frozen=True)
class Rule:
    category: str
    kind: str
    pattern: str
    description: str


# First match wins. Exclusions and dedicated test files take precedence over src/.
# These are location/role labels, not assertions that every line has one concern.
RULES = (
    Rule('excluded_vendor', 'excluded', r'(^|/)vendor/', 'Vendored code and submodule contents are outside maintained project logic.'),
    Rule('excluded_build_or_run', 'excluded', r'(^|/)(target|runs|__pycache__|node_modules|dist|build)/', 'Build output, ignored evidence and dependency output.'),
    Rule('excluded_input_documents', 'excluded', r'^(example\.(build|import\.txt)$|examples/(?!.*\.rs$)|tests/fixtures/builds/(?!.*\.md$)|tests/fixtures/calibration/.*\.xml$)', 'Sample/user build documents, imported corpus metadata and example input configurations; not logic.'),
    Rule('test_rust', 'tests', r'^(tests/.*\.rs|crates/[^/]+/(tests/.*\.rs|src/(?:.*/)?tests(?:/.*)?\.rs|src/(?:.*/)?[^/]+_tests\.rs))$', 'Dedicated Rust integration/support/unit-test files, including tests below src/.'),
    Rule('test_tooling', 'tests', r'^scripts/test_[^/]+\.py$', 'Tests of maintenance/import tooling.'),
    Rule('source_fixtures', 'source_fixtures', r'(^|/)tests/.*\.lua$', 'Original-source semantic probes and Lua oracle/warming helpers; not shipping runtime logic.'),
    Rule('test_data', 'test_data', r'(^|/)tests/.*\.(json|sha256|txt)$', 'Reference outputs, corpora and expectation data outside excluded build documents.'),
    Rule('documentation', 'docs', r'(^docs/|\.md$)', 'Design, implementation history, notices and README files.'),
    Rule('authored_definition_policy', 'data_policy', r'^crates/poe-optimizer-data/data/class-tree-policy\.json$', 'Authored class/tree applicability policy; distinct from generated exports.'),
    Rule('generated_definition_payload', 'generated_data', r'^crates/poe-optimizer-data/data/', 'Generated definition/tree packages, identities and checksums; never counted as logic.'),
    Rule('acquisition_policy', 'data_policy', r'^crates/poe-optimizer-pob/(src/.*\.json|data/.*\.json)$', 'Source pin/provenance, source shapes and extraction/admission policy.'),
    Rule('prod_verifier', 'production_rust', r'^crates/poe-optimizer-data/src/(modifier_parser/programs/(validate|payload)\.rs|source_program/(classes|closure_creations|closures|constructors|context|iteration)\.rs|source_program/(constructors/walk|session/traversal)\.rs)$', 'Binding, coverage, closure/constructor and program validation modules; several also declare schema types.'),
    Rule('prod_ir_schema', 'production_rust', r'^crates/poe-optimizer-data/src/(modifier_parser/programs\.rs|source_program/(graph|session)\.rs)$', 'Typed source IR and neutral source/session graph representations, with local validation helpers.'),
    Rule('prod_ir_owner_binding', 'production_rust', r'^crates/poe-optimizer-data/src/source_program\.rs$', 'Shared source owner/catalog facade and domain compatibility/binding boundaries.'),
    Rule('prod_data', 'production_rust', r'^crates/poe-optimizer-data/src/.*\.rs$', 'Injected game/parser definitions, generated-package loading and domain schema validation.'),
    Rule('prod_source_runtime', 'production_rust', r'^crates/poe-optimizer-engine/src/(parser_program/runtime/.*\.rs|lua_(bits|number|pattern)(/.*)?\.rs)$', 'Source-compatible execution, heap, sessions, cells, table evidence, strings/numbers and diagnostics. This location bucket is not the total compatibility cost; parser/item/domain modules also retain source semantics.'),
    Rule('prod_ir_compiler', 'production_rust', r'^crates/poe-optimizer-engine/src/(parser_program|source_program)\.rs$', 'Compiled typed IR, shared traversal plans and legacy/source facade.'),
    Rule('prod_modifier_parser', 'production_rust', r'^crates/poe-optimizer-engine/src/(modifier_parser(/.*)?|modifier_scan)\.rs$', 'Native parser/scan algorithms over injected dictionaries and source-program adapters, including closed source-compatible byte-string/factory operations.'),
    Rule('prod_item_algorithms', 'production_rust', r'^crates/poe-optimizer-engine/src/item_.*\.rs$', 'Item formatting, runes and item algorithm helpers, including source-compatible numeric formatting; module role does not establish general mechanic coverage.'),
    Rule('prod_calculation', 'production_rust', r'^crates/poe-optimizer-engine/src/.*\.rs$', 'Domain calculation/query kernels and crate/data integration, including explicitly closed Mace/Spark profiles. Neither universal domain logic nor full build admission; modules may retain Lua-compatible scalar behavior.'),
    Rule('prod_observer', 'production_rust', r'^crates/poe-optimizer-pob/src/source_programs/capture(/.*)?\.rs$', 'Actual source graph/prototype/capture/layout observation and authentication.'),
    Rule('prod_lowerer', 'production_rust', r'^crates/poe-optimizer-pob/src/(source_programs(/.*)?|parser_programs)\.rs$', 'Tokenization/syntax/lowering plus source-to-IR evidence mapping and parser adapter.'),
    Rule('prod_parser_acquisition', 'production_rust', r'^crates/poe-optimizer-pob/src/modifier_parser_extract(/.*)?\.rs$', 'Domain parser dictionary/factory extraction, reconstruction and admission policy.'),
    Rule('prod_definition_acquisition', 'production_rust', r'^crates/poe-optimizer-pob/src/(.*_extract(/.*)?|game_data|game_data_worker|tree_data|tree_projection|tree_worker)\.rs$', 'Game/configuration/skill/item/tree extraction and offline worker orchestration.'),
    Rule('prod_reference_backend', 'production_rust', r'^crates/poe-optimizer-pob/src/.*\.rs$', 'Optional PoB reference host, import, metrics, mutation, supervision and source checks.'),
    Rule('prod_reference_lua', 'production_lua', r'^crates/poe-optimizer-pob/src/.*\.lua$', 'Maintained Lua acquisition/reference-host glue, not vendored game source or native evaluation.'),
    Rule('prod_preparation', 'production_rust', r'^crates/poe-optimizer-native/src/.*\.rs$', 'Native preparation, selected-view/profile admission, reusable evaluation and candidate integration, including closed legacy profile adapters; this bucket does not imply complete general preparation.'),
    Rule('prod_import_resolution', 'production_rust', r'^crates/poe-optimizer-import/src/.*\.rs$', 'Import, item/config/skill assembly and source/instance/view resolution.'),
    Rule('prod_core_contracts', 'production_rust', r'^crates/poe-optimizer-core/src/.*\.rs$', 'Build/candidate/data/evaluation contracts, metric and objective definitions.'),
    Rule('prod_search', 'production_rust', r'^crates/poe-optimizer-search/src/.*\.rs$', 'Search algorithm and public search contracts.'),
    Rule('prod_cli', 'production_rust', r'^src/.*\.rs$', 'CLI preparation/search/extraction orchestration and benchmark command implementation.'),
    Rule('prod_reference_ffi', 'production_rust', r'^crates/poe-optimizer-lua-utf8/src/.*\.rs$', 'Maintained optional Lua UTF-8 FFI integration, excluding vendor implementation.'),
    Rule('tooling', 'tooling', r'^(scripts/|\.github/|examples/.*\.rs$|crates/[^/]+/(build\.rs$|examples/.*\.rs$))', 'Maintenance, parity/calibration tools, CI, benchmark examples and build glue.'),
    Rule('package_manifests', 'configuration', r'(^|/)Cargo\.toml$', 'Declared package dependencies, features, targets and workspace configuration.'),
    Rule('dependency_lock', 'configuration', r'^Cargo\.lock$', 'Resolved package version lock; not implementation logic.'),
    Rule('repository_configuration', 'configuration', r'(^\.git|^rust-toolchain\.toml$|/\.gitattributes$)', 'Git and Rust toolchain configuration.'),
)

# Small manually inspected navigation set, not exhaustive ownership/purity claims.
MIXED_SEAMS = (
    ('crates/poe-optimizer-data/src/source_program.rs', ['source_semantics', 'domain_binding'], 'Neutral owners reuse parser graph/IR names and also expose domain admission adapters.'),
    ('crates/poe-optimizer-data/src/modifier_parser/programs.rs', ['source_semantics', 'legacy_parser_contract'], 'Standalone IR extensions and legacy parser wire/admission share representation.'),
    ('crates/poe-optimizer-data/src/modifier_parser.rs', ['domain_rules', 'source_graph'], 'Injected parser dictionaries and callback/table identities share the catalog.'),
    ('crates/poe-optimizer-engine/src/modifier_parser.rs', ['domain_algorithm', 'source_runtime_adapter'], 'Parser dictionary algorithms, shared pattern/program kernels and accounting meet here.'),
    ('crates/poe-optimizer-engine/src/modifier_parser/strings.rs', ['source_string_semantics', 'closed_factory_adapter'], 'Closed byte-string operations for proven factory expressions remain in the parser category; not all compatibility code is in prod_source_runtime.'),
    ('crates/poe-optimizer-engine/src/item_tools/numeric.rs', ['item_algorithm', 'source_numeric_semantics'], 'LuaJIT numeric tostring formatting coexists with item rounding helpers; the whole file is not exclusively domain or compatibility logic.'),
    ('crates/poe-optimizer-engine/src/modifier_scan.rs', ['domain_parser', 'source_iteration_and_value_semantics'], 'Original scan behavior depends on caller-supplied traversal order and source false/nil distinctions.'),
    ('crates/poe-optimizer-engine/src/mace.rs', ['closed_legacy_profile', 'injected_domain_calculation'], 'The header and literal profile/skill/weapon selectors restrict Mace evaluation; injected balance values do not make this a universal build algorithm.'),
    ('crates/poe-optimizer-engine/src/spark.rs', ['closed_legacy_profile', 'injected_domain_calculation'], 'The header limits Spark skill level, equipment/support scope and admitted inputs; this is not a general build evaluator.'),
    ('crates/poe-optimizer-engine/src/data.rs', ['injected_data_compilation', 'closed_profile_binding'], 'Shared dataset compilation retains Mace/Spark adapters, including exactly two Mace weapon capability slots.'),
    ('crates/poe-optimizer-native/src/profile.rs', ['closed_legacy_profile', 'prepared_state_admission'], 'Strict Spark/Mace document projection and shared prepared-skill consistency checks remain explicit; this is not complete general preparation.'),
    ('crates/poe-optimizer-engine/src/conditions/condition_program.rs', ['domain_conditions', 'source_scalar_semantics'], 'Typed condition evaluation retains source truthiness and unsupported-value distinctions.'),
    ('crates/poe-optimizer-engine/src/parser_program.rs', ['source_semantics', 'legacy_parser_facade'], 'Shared compiled source engine retains the parser facade and ownership constraints.'),
    ('crates/poe-optimizer-pob/src/source_programs.rs', ['source_authentication', 'lowering_orchestration'], 'Source hashes and lowering do not themselves establish game/numerical admission.'),
    ('crates/poe-optimizer-pob/src/game_data.rs', ['domain_acquisition', 'provenance_validation'], 'Definition extraction and implementation fingerprints cross source/domain boundaries.'),
    ('crates/poe-optimizer-native/src/preparation.rs', ['domain_lifecycle', 'admission_and_backend'], 'Shared preparation outcomes distinguish supported reusable evaluation from explicit incompleteness.'),
    ('crates/poe-optimizer-import/src/configuration.rs', ['domain_configuration', 'input_compatibility'], 'Imported configuration meaning is part of preparation, not just runtime compatibility.'),
)


def git(repo: Path, *args: str) -> bytes:
    return subprocess.check_output(['git', '-C', str(repo), *args])


def classify(path: str, mode: str) -> tuple[str, str]:
    if mode == '160000':
        return 'excluded_submodule', 'excluded'
    if mode != '100644' and mode != '100755':
        return 'excluded_nonregular', 'excluded'
    for rule in RULES:
        if re.search(rule.pattern, path):
            return rule.category, rule.kind
    return 'unclassified', 'unclassified'


class Blobs:
    """Read one committed blob at a time; do not traverse vendor/user inputs."""
    def __init__(self, repo: Path):
        self.process = subprocess.Popen(
            ['git', '-C', str(repo), 'cat-file', '--batch'],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        )

    def read(self, oid: str) -> bytes:
        assert self.process.stdin is not None and self.process.stdout is not None
        self.process.stdin.write(oid.encode('ascii') + b'\n')
        self.process.stdin.flush()
        header = self.process.stdout.readline().split()
        if len(header) != 3 or header[1] != b'blob':
            raise ValueError(f'Expected blob: {oid}')
        size = int(header[2])
        raw = self.process.stdout.read(size)
        if len(raw) != size or self.process.stdout.read(1) != b'\n':
            raise ValueError(f'Truncated Git blob: {oid}')
        return raw

    def close(self) -> None:
        assert self.process.stdin is not None and self.process.stdout is not None
        self.process.stdin.close()
        self.process.stdout.close()
        if self.process.wait() != 0:
            raise ValueError('Git blob reader failed')


def metrics(raw: bytes) -> dict:
    try:
        text = raw.decode('utf-8')
    except UnicodeDecodeError:
        text = None
    if text is None or '\0' in text:
        return {'sha256': hashlib.sha256(raw).hexdigest(), 'encoding': 'binary_or_non_utf8',
                'physical_lines': None, 'nonblank_lines': None}
    # Literal LF records: no platform newline conversion or Unicode separator splitting.
    lines = text.split('\n')
    if lines[-1] == '':
        lines.pop()
    return {'sha256': hashlib.sha256(raw).hexdigest(), 'encoding': 'utf-8',
            'physical_lines': len(lines),
            'nonblank_lines': sum(bool(line.strip()) for line in lines)}


def summarize(records: list[dict]) -> dict:
    return {'files': len(records), 'bytes': sum(r.get('bytes') or 0 for r in records),
            'physical_lines': sum(r.get('physical_lines') or 0 for r in records),
            'nonblank_lines': sum(r.get('nonblank_lines') or 0 for r in records),
            'files_without_line_measurement': sum(r.get('physical_lines') is None for r in records)}


def dependency_report(manifests: dict[str, dict], lock: dict) -> dict:
    root = manifests['Cargo.toml']
    workspace_deps = root.get('workspace', {}).get('dependencies', {})
    packages = []
    edges = []
    for path, manifest in sorted(manifests.items()):
        package = manifest.get('package')
        if not package:
            continue
        name = package['name']
        packages.append({'package': name, 'manifest': path,
                         'features': manifest.get('features', {}),
                         'rust_version': package.get('rust-version')})
        sections = [(None, manifest)] + sorted(manifest.get('target', {}).items())
        for target, section in sections:
            for kind in ('dependencies', 'dev-dependencies', 'build-dependencies'):
                for alias, value in sorted(section.get(kind, {}).items()):
                    spec = {'version': value} if isinstance(value, str) else dict(value)
                    if spec.get('workspace'):
                        shared = workspace_deps[alias]
                        inherited = {'version': shared} if isinstance(shared, str) else dict(shared)
                        features = list(inherited.get('features', [])) + list(spec.get('features', []))
                        spec = {**inherited, **spec}
                        if features:
                            spec['features'] = sorted(set(features))
                    edges.append({'from': name, 'dependency': spec.get('package', alias),
                                  'alias': alias, 'kind': kind, 'target': target,
                                  'optional': spec.get('optional', False),
                                  'version': spec.get('version'), 'path': spec.get('path'),
                                  'features': sorted(spec.get('features', [])),
                                  'default_features': spec.get('default-features', True)})
    internal_names = {p['package'] for p in packages}
    adjacency = defaultdict(set)
    for e in edges:
        if e['kind'] == 'dependencies' and not e['optional'] and e['dependency'] in internal_names:
            adjacency[e['from']].add(e['dependency'])

    def closure(name: str) -> list[str]:
        seen = set()
        pending = [name]
        while pending:
            item = pending.pop()
            if item not in seen:
                seen.add(item)
                pending.extend(adjacency[item] - seen)
        return sorted(seen)

    roots = ['poe-optimizer-cli', 'poe-optimizer-native', 'poe-optimizer-engine']
    closures = {name: closure(name) for name in roots if name in internal_names}
    watched = {'mlua', 'mlua-sys', 'luajit-src', 'lua-src', 'cc', 'rayon'}
    native = set(closures.get('poe-optimizer-native', []))
    native_lua_edges = [e for e in edges if e['from'] in native
                        and e['kind'] == 'dependencies' and not e['optional']
                        and e['dependency'] in {'mlua', 'mlua-sys', 'poe-optimizer-pob', 'poe-optimizer-lua-utf8'}]
    return {
        'method': 'Committed Cargo.toml declarations parsed with tomllib, without Cargo resolution/build. Target predicates are retained, not evaluated; optional dependencies and feature activation are not inferred.',
        'packages': packages, 'edges': edges,
        'internal_normal_nonoptional_closure_including_root': closures,
        'native_closure_declared_lua_edges': native_lua_edges,
        'selected_locked_external_packages': [p for p in lock.get('package', []) if p['name'] in watched],
        'important_seams': {
            'cli_default_features': root.get('features', {}).get('default', []),
            'cli_pob_feature_members': root.get('features', {}).get('pob', []),
            'declared_lua_and_reference_edges': [e for e in edges if e['dependency'] in {'mlua', 'mlua-sys', 'luajit-src', 'poe-optimizer-pob', 'poe-optimizer-lua-utf8'}],
            'pob_to_engine_or_native_edges': [e for e in edges if e['from'] == 'poe-optimizer-pob' and e['dependency'] in {'poe-optimizer-engine', 'poe-optimizer-native'}],
            'rayon_edges': [e for e in edges if e['dependency'] == 'rayon'],
            'limits': [
                'Feature definitions and optional edges remain explicit; disabling CLI defaults is a separate invocation/configuration.',
                'Normal/dev/build and target predicates are separate evidence; tests can depend on Lua without implying a normal native runtime dependency.',
                'Rayon placement does not measure scaling or prove a WASM build.',
                'The static internal closure is not a resolved Cargo tree or proof about every external transitive dependency.',
            ],
        },
    }


def inventory(repo: Path, revision: str) -> dict:
    commit = git(repo, 'rev-parse', '--verify', '--end-of-options', revision + '^{commit}').decode().strip()
    tree = git(repo, 'ls-tree', '-r', '-l', '-z', commit)
    entries = []
    for entry in tree.split(b'\0'):
        if not entry:
            continue
        header, name = entry.split(b'\t', 1)
        mode, object_kind, oid, size = header.split()
        path = name.decode('utf-8')
        category, kind = classify(path, mode.decode())
        entries.append({'path': path, 'mode': mode.decode(), 'git_object_type': object_kind.decode(),
                        'git_object_id': oid.decode(), 'bytes': None if size == b'-' else int(size),
                        'category': category, 'kind': kind})
    entries.sort(key=lambda e: e['path'])
    manifests = {}
    lock = {}
    reader = Blobs(repo)
    try:
        for entry in entries:
            if entry['kind'] == 'excluded':
                continue
            raw = reader.read(entry['git_object_id'])
            if len(raw) != entry['bytes']:
                raise ValueError(f"Git size disagreement: {entry['path']}")
            entry.update(metrics(raw))
            path = entry['path']
            if path.endswith('.rs'):
                # Diagnostic lexical markers only: embedded raw strings may contain them.
                entry['lexical_test_attribute_markers'] = raw.count(b'#[test]')
                entry['lexical_cfg_test_markers'] = raw.count(b'#[cfg(test)]')
            if PurePosixPath(path).name == 'Cargo.toml':
                manifests[path] = tomllib.loads(raw.decode('utf-8'))
            if path == 'Cargo.lock':
                lock = tomllib.loads(raw.decode('utf-8'))
    finally:
        reader.close()
    included = [e for e in entries if e['kind'] != 'excluded']
    excluded = [e for e in entries if e['kind'] == 'excluded']
    categories = sorted({r.category for r in RULES} | {e['category'] for e in entries}
                        | {'excluded_submodule', 'excluded_nonregular', 'unclassified'})
    by_category = {name: summarize([e for e in entries if e['category'] == name]) for name in categories}
    by_kind = {kind: summarize([e for e in entries if e['kind'] == kind])
               for kind in sorted({e['kind'] for e in entries})}
    by_package = {}
    for e in included:
        parts = e['path'].split('/')
        name = parts[1] if parts[0] == 'crates' else 'workspace_root'
        by_package.setdefault(name, []).append(e)
    indexed = {e['path']: e for e in entries}
    seams = [{'path': path, 'labels': labels, 'reason': reason,
              'present': path in indexed, 'sha256': indexed.get(path, {}).get('sha256')}
             for path, labels, reason in MIXED_SEAMS]
    assert len(indexed) == len(entries)
    assert sum(v['files'] for v in by_category.values()) == len(entries)
    assert sum(v['bytes'] for v in by_category.values()) == sum(e.get('bytes') or 0 for e in entries)
    counted_rust = [e for e in included if e['kind'] == 'production_rust']
    inline_markers = [e for e in counted_rust if e.get('lexical_test_attribute_markers', 0)
                      or e.get('lexical_cfg_test_markers', 0)]
    return {
        'schema_version': SCHEMA_VERSION,
        'snapshot': {'revision_requested': revision, 'commit': commit,
                     'commit_time': git(repo, 'show', '-s', '--format=%cI', commit).decode().strip(),
                     'tree': git(repo, 'rev-parse', commit + '^{tree}').decode().strip(),
                     'content_basis': 'Committed Git blobs only; ignores working-tree edits, untracked files and nested submodule content.',
                     'inventory_script_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()},
        'methodology': [
            'Each tracked tree entry receives exactly one category using first-match rules; excluded entries remain in scope reconciliation.',
            'Included file bytes and SHA256 are measured on committed bytes. Physical lines are LF records, including a final unterminated line; nonblank lines use Unicode whitespace stripping.',
            'Binary/non-UTF8 included files retain byte/hash measures but no line counts. Excluded content is not loaded or line-counted; Git blob sizes/IDs remain visible, and gitlinks have unknown size.',
            'Production Rust categories are module-location inventories, not parsed non-test executable LOC. Dedicated test files are separate; inline tests, comments, raw strings and generated fragments remain in their parent file.',
            'Lexical test/cfg markers identify possible mixed files; they are not parsed test counts and may occur inside strings.',
            'LOC/bytes are navigation and size measures, not proxies for maintenance cost, correctness, semantic complexity, performance, upstream effort or the value of an execution model.',
            'Generated exports, authored policy, source fixtures and test data are separate; the large generated JSON package is never counted as logic.',
            'Mixed-seam labels reflect inspected examples and do not claim whole-file semantic purity. An unclassified bucket preserves new/unrecognized file roles.',
            'No wall-clock timestamp, absolute checkout path or working-tree status is embedded: rerunning identical script/revision yields identical JSON bytes.',
        ],
        'rules': [{'priority': i, 'category': r.category, 'kind': r.kind,
                   'regex': r.pattern, 'description': r.description} for i, r in enumerate(RULES)],
        'special_rules': {'gitlink': 'excluded_submodule', 'nonregular_mode': 'excluded_nonregular',
                          'no_match': 'unclassified'},
        'reconciliation': {'tracked_entries': len(entries), 'included_entries': len(included),
                           'excluded_entries': len(excluded), 'included_plus_excluded_equals_tracked': True,
                           'category_partition_is_unique': True, 'unclassified_paths': [e['path'] for e in entries if e['category'] == 'unclassified']},
        'totals': {'tracked': summarize(entries), 'included': summarize(included), 'excluded': summarize(excluded)},
        'by_category': by_category, 'by_kind': by_kind,
        'by_package_all_included_roles': {name: summarize(rows) for name, rows in sorted(by_package.items())},
        'production_rust_files_with_lexical_test_markers': [
            {key: e[key] for key in ('path', 'category', 'lexical_test_attribute_markers', 'lexical_cfg_test_markers')}
            for e in inline_markers],
        'mixed_seams': seams,
        'dependencies': dependency_report(manifests, lock),
        'files': entries,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, default=Path(__file__).resolve().parents[1],
                        help='Git repository root (default: parent of this script directory).')
    parser.add_argument('--revision', default='HEAD', help='Committed revision to inventory (default: HEAD).')
    parser.add_argument('--output', type=Path, required=True, help='JSON output; must not overwrite a tracked file.')
    args = parser.parse_args()
    repo = args.repo.resolve()
    output = args.output.resolve()
    try:
        relative_output = output.relative_to(repo).as_posix()
    except ValueError:
        relative_output = None
    if relative_output and git(repo, 'ls-files', '--', relative_output).strip():
        parser.error('Refusing to overwrite tracked repository content with inventory output.')
    report = inventory(repo, args.revision)
    output.parent.mkdir(parents=True, exist_ok=True)
    encoded = (json.dumps(report, indent=2, ensure_ascii=False, sort_keys=True) + '\n').encode('utf-8')
    output.write_bytes(encoded)
    print(json.dumps({'commit': report['snapshot']['commit'], **report['reconciliation'],
                      'output_sha256': hashlib.sha256(encoded).hexdigest()}, sort_keys=True))
    return 0


if __name__ == '__main__':
    sys.exit(main())
