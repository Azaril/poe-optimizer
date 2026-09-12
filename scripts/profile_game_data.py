#!/usr/bin/env python3
"""Measure original loader statements in an isolated worktree; never patch the working project.

Prepare first, review the generated diff, then measure. Python 3.11+, Git and Cargo only.
All intervals are inclusive. Inner intervals must not be added to their parents.
"""
from __future__ import annotations
import argparse
import datetime as dt
import difflib
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

SOURCE = 'crates/poe-optimizer-data/src/game_data.rs'
LIB = 'crates/poe-optimizer-data/src/lib.rs'
COUNTERS = 'crates/poe-optimizer-data/src/load_profile.rs'
EXAMPLE = 'crates/poe-optimizer-data/examples/profile_game_data.rs'
CHECKS = [
    'embedded_and_external_bytes_share_one_validated_loader',
    'recursive_duplicate_keys_and_resource_limits_reject_before_snapshot',
    'unknown_nested_source_enum_fields_and_integer_key_aliases_do_not_disappear',
    'snapshot_owned_data_is_send_sync_and_remains_unchanged_by_package_edits',
]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_json(path, value):
    with Path(path).open('x', encoding='utf-8', newline='\n') as output:
        json.dump(value, output, indent=2)
        output.write('\n')


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()


def instrument(original):
    ranges = []

    def unique(text, label):
        if label.startswith(('load.', 'catalog.')):
            first, last = 'impl GameDataLoader {', 'pub fn bundled_package_bytes()'
        elif label.startswith('decode.'):
            first, last = 'fn decode_value(', 'fn reject_discarded_fields('
        elif label.startswith('validate.'):
            first, last = 'fn validate(package:', 'fn validate_requirement('
        elif label == 'cache.reviewed_passive_projection':
            first, last = 'fn parse_reviewed_passive_capability_keys(', 'fn reviewed_passive_capability_keys()'
        else:
            first, last = 'fn reviewed_passive_capability_keys()', 'fn validate_armour('
        start, end = original.index(first), original.index(last)
        region = original[start:end]
        if region.count(text) != 1:
            raise ValueError(f'Expected one unchanged source anchor in {label}: {text!r}')
        return start + region.index(text)

    def span(label, parent, first, following):
        begin, end = unique(first, label), unique(following, label)
        if begin >= end:
            raise ValueError(f'Invalid statement span for {label}')
        ranges.append((begin, end, label, parent))

    def statement(label, parent, text):
        begin = unique(text, label)
        ranges.append((begin, begin + len(text), label, parent))

    span('load.byte_limits_hash_trust', 'load', '        check_limits(bytes, limits)?;\n', '        let value = bounded_json(bytes, limits)?;\n')
    for label, text in [
        ('bounded_json', '        let value = bounded_json(bytes, limits)?;\n'),
        ('decode_and_scope_drop', '        let package = decode_value(value)?;\n'),
        ('validate_and_scope_drop', '        validate(&package, limits)?;\n'),
    ]:
        statement('load.' + label, 'load', text)
    span('load.identity', 'load', '        let identity = DataIdentity {\n', '        let configuration = ConfigDefinitionCatalog::new(package.configuration.clone())?;\n')
    catalogs = [
        ('configuration', '        let configuration = ConfigDefinitionCatalog::new(package.configuration.clone())?;\n'),
        ('skill_identities', '        let skill_identities = SkillIdentityCatalog::new(package.skill_identities.clone())?;\n'),
        ('skill_preparation', '        let skill_preparation = SkillPreparationCatalog::new(\n            package.skill_preparation.clone(),\n            &package.skill_identities,\n        )?;\n'),
        ('item_loading', '        let item_loading = ItemLoadingCatalog::new(package.item_loading.clone())?;\n'),
        ('item_scalability', '        let item_scalability = ItemScalabilityCatalog::new(package.item_scalability.clone())?;\n'),
        ('modifier_parser', '        let modifier_parser = ModifierParserCatalog::new(package.modifier_parser.clone())?;\n'),
        ('parser_programs', '        let parser_programs = ParserAdmittedProgramCatalog::new(&modifier_parser).map_err(error)?;\n'),
        ('item_assembly', '        let item_assembly = ItemAssemblyCatalog::new(package.item_assembly.clone())?;\n'),
        ('unique_requirements', '        let unique_requirements =\n            UniqueRequirementCatalog::new(package.unique_requirements.clone())?;\n'),
    ]
    for name, text in catalogs:
        statement('catalog.' + name, 'load', text)
    statement('decode.typed_records', 'load.decode_and_scope_drop', '    let package = GameDataPackage::deserialize(&value).map_err(error)?;\n')
    statement('decode.roundtrip_unknown_fields', 'load.decode_and_scope_drop', '    reject_discarded_fields(\n        "package",\n        &value,\n        &serde_json::to_value(&package).map_err(error)?,\n    )?;\n')
    validations = [
        ('skill_identities', '    package.skill_identities.validate()?;\n'),
        ('skill_preparation', '    package\n        .skill_preparation\n        .validate(&package.skill_identities)?;\n'),
        ('item_loading', '    package.item_loading.validate()?;\n'),
        ('item_scalability', '    package.item_scalability.validate()?;\n'),
        ('modifier_parser', '    package.modifier_parser.validate()?;\n'),
        ('item_assembly', '    package.item_assembly.validate()?;\n    package.item_assembly.validate_dependencies(\n        &package.item_loading,\n        &package.item_scalability,\n        &package.modifier_parser,\n    )?;\n'),
        ('unique_requirements', '    package\n        .unique_requirements\n        .validate_inputs(&package.item_loading, &package.tree)?;\n'),
        ('tree_authentication', '    crate::bundled::authenticate_bundle(&package.tree.canonical_bytes().map_err(error)?)\n        .map_err(error)?;\n'),
        ('numeric_value', '    let value = serde_json::to_value(package).map_err(error)?;\n'),
        ('passive_catalog', '    validate_passive_catalog(package, limits)?;\n'),
        ('jewellery', '    validate_jewellery(package)?;\n'),
        ('armour', '    validate_armour(package)?;\n'),
    ]
    for name, text in validations:
        statement('validate.' + name, 'load.validate_and_scope_drop', text)
    span('validate.manifest_and_section_digests', 'load.validate_and_scope_drop', '    let m = &package.manifest;\n', '    // Structural tree migration deliberately remains source-pinned.')
    span('validate.numeric_walk', 'load.validate_and_scope_drop', '    for section in [\n', '    for class in [\n')
    span('validate.profile_references_and_supports', 'load.validate_and_scope_drop', '    for class in [\n', '    package.receiving_defence.validate()?;\n')
    span('validate.receiving_action', 'load.validate_and_scope_drop', '    package.receiving_defence.validate()?;\n', '    let c = &package.character;\n')
    span('validate.remaining_records', 'load.validate_and_scope_drop', '    let c = &package.character;\n', '    validate_passive_catalog(package, limits)?;\n')
    if 'fn parse_reviewed_passive_capability_keys(' in original:
        statement('cache.reviewed_passive_projection', 'validate.passive_catalog', '    let package: PackageKeys = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;\n')
    else:
        statement('cache.reviewed_passive_json', 'validate.passive_catalog', '        let value: serde_json::Value =\n            serde_json::from_slice(PACKAGE_BYTES).map_err(|e| e.to_string())?;\n')
    ordered = sorted(ranges)
    assert all(a[1] <= b[0] for a, b in zip(ordered, ordered[1:])), 'Overlapping source spans'
    stages, chunks, cursor = [], [], 0
    for index, (begin, end, name, parent) in enumerate(ordered):
        text = original[begin:end]
        indent = text[:len(text) - len(text.lstrip(' '))]
        before = f'{indent}let __poe_profile_{index} = std::time::Instant::now(); // POE_LOAD_PROFILE\n'
        after = f'{indent}crate::load_profile::record({index}, __poe_profile_{index}); // POE_LOAD_PROFILE\n'
        chunks.extend([original[cursor:begin], before, text, after])
        stages.append({'index': index, 'name': name, 'parent': parent,
                       'first_line': original[:begin].count('\n') + 1,
                       'last_line': original[:end].count('\n'),
                       'original_statement_sha256': hashlib.sha256(text.encode()).hexdigest()})
        cursor = end
    chunks.append(original[cursor:])
    modified = ''.join(chunks)
    restored = ''.join(line for line in modified.splitlines(keepends=True) if not line.endswith('// POE_LOAD_PROFILE\n'))
    assert restored == original, 'Instrumentation changed an original byte/statement'
    return modified, stages


def prepare(root, output, revision):
    commit = git(root, 'rev-parse', '--verify', revision + '^{commit}')
    original = subprocess.check_output(['git', '-C', str(root), 'show', f'{commit}:{SOURCE}']).decode('utf-8')
    modified, stages = instrument(original)  # Reject changed/ambiguous anchors before creating a worktree.
    output.mkdir(parents=True, exist_ok=False)
    worktree = output / 'worktree'
    subprocess.run(['git', '-C', str(root), 'worktree', 'add', '--detach', str(worktree), commit], check=True)
    assert (worktree / SOURCE).read_text(encoding='utf-8') == original
    (output / 'original-game-data.rs').write_text(original, encoding='utf-8', newline='\n')
    (output / 'instrumented-game-data.rs').write_text(modified, encoding='utf-8', newline='\n')
    (output / 'instrumentation.diff').write_text(''.join(difflib.unified_diff(original.splitlines(True), modified.splitlines(True), fromfile=SOURCE, tofile=SOURCE)), encoding='utf-8', newline='\n')
    module = '''// Isolated measurement artifact, never part of the production data library.
use std::{cell::RefCell, time::Instant};
thread_local! { static COUNTERS: RefCell<[(u64, u64); STAGE_COUNT]> = const { RefCell::new([(0, 0); STAGE_COUNT]) }; }
pub fn record(index: usize, start: Instant) {
    let elapsed: u64 = start.elapsed().as_nanos().try_into().unwrap();
    COUNTERS.with_borrow_mut(|rows| { rows[index].0 += 1; rows[index].1 += elapsed; });
}
pub fn take() -> Vec<(u64, u64)> {
    COUNTERS.with_borrow_mut(|rows| std::mem::replace(rows, [(0, 0); STAGE_COUNT]).to_vec())
}
'''.replace('STAGE_COUNT', str(len(stages)))
    (worktree / COUNTERS).write_text(module, encoding='utf-8', newline='\n')
    lib = (worktree / LIB).read_text(encoding='utf-8')
    (worktree / LIB).write_text(lib + '\n// Isolated profiling harness only.\npub mod load_profile;\n', encoding='utf-8', newline='\n')
    (worktree / EXAMPLE).parent.mkdir(exist_ok=True)
    shutil.copyfile(root / 'scripts/profile_game_data.rs', worktree / EXAMPLE)
    protected = [COUNTERS, LIB, EXAMPLE, 'Cargo.lock', 'Cargo.toml', 'crates/poe-optimizer-data/Cargo.toml', 'crates/poe-optimizer-data/data/game-data.json']
    write_json(output / 'prepared.json', {'schema_version': 1, 'commit': commit, 'worktree': str(worktree),
               'source': SOURCE, 'stages': stages, 'original_sha256': sha(output / 'original-game-data.rs'),
               'instrumented_sha256': sha(output / 'instrumented-game-data.rs'),
               'protected_sha256': {p: sha(worktree / p) for p in protected},
               'script_sha256': sha(__file__), 'tooling_harness_sha256': sha(root / 'scripts/profile_game_data.rs')})
    print(f'Prepared {len(stages)} intervals; review {output / "instrumentation.diff"}', flush=True)


def measure(output, repeats, loads):
    plan = json.loads((output / 'prepared.json').read_text(encoding='utf-8'))
    worktree = Path(plan['worktree'])
    assert worktree == output / 'worktree' and git(worktree, 'rev-parse', 'HEAD') == plan['commit']
    assert sha(__file__) == plan['script_sha256'], 'Profiling script changed; prepare a new run'
    for file, digest in plan['protected_sha256'].items():
        assert sha(worktree / file) == digest, file
    for name in ['original', 'instrumented']:
        assert sha(output / f'{name}-game-data.rs') == plan[f'{name}_sha256']
    assert sha(worktree / SOURCE) == plan['original_sha256'], 'Unexpected loader edits in isolated worktree'
    assert set(git(worktree, 'diff', '--name-only').splitlines()) <= {LIB}, 'Unreviewed tracked worktree edits'
    env = dict(os.environ, CARGO_TARGET_DIR=str(output / 'target'))
    suffix = '.exe' if os.name == 'nt' else ''
    binaries = {}
    for mode in ['control', 'instrumented']:
        source = output / ('original-game-data.rs' if mode == 'control' else 'instrumented-game-data.rs')
        shutil.copyfile(source, worktree / SOURCE)
        command = ['cargo', 'build', '--release', '--locked', '-p', 'poe-optimizer-data', '--example', 'profile_game_data']
        print(f'Building {mode}', flush=True)
        with (output / f'{mode}-build.log').open('xb') as log:
            subprocess.run(command, cwd=worktree, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        binary = output / (mode + suffix)
        shutil.copyfile(output / 'target/release/examples' / ('profile_game_data' + suffix), binary)
        binaries[mode] = {'path': str(binary), 'sha256': sha(binary), 'loader_sha256': sha(worktree / SOURCE)}
        for check in CHECKS:
            print(f'Checking {mode}: {check}', flush=True)
            command = ['cargo', 'test', '--release', '--locked', '-p', 'poe-optimizer-data', '--test', 'game_data', '--', '--exact', check, '--test-threads=1']
            with (output / f'{mode}-check-{check}.log').open('xb') as log:
                subprocess.run(command, cwd=worktree, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
            check_log = (output / f'{mode}-check-{check}.log').read_text(encoding='utf-8')
            assert 'test result: ok. 1 passed; 0 failed' in check_log, check
    results = []
    for repeat in range(repeats):
        for mode in (['control', 'instrumented'] if repeat % 2 == 0 else ['instrumented', 'control']):
            print(f'Measuring {mode}, fresh process {repeat + 1}', flush=True)
            destination = output / f'{mode}-{repeat + 1}.json'
            with destination.open('xb') as out, destination.with_suffix('.log').open('xb') as err:
                subprocess.run([binaries[mode]['path'], str(loads)], cwd=worktree, stdout=out, stderr=err, check=True, timeout=600)
            report = json.loads(destination.read_text(encoding='utf-8'))
            results.append({'mode': mode, 'repeat': repeat + 1, 'artifact': str(destination), 'sha256': sha(destination), 'report': report})
    write_json(output / 'results.json', {'schema_version': 1, 'timestamp': dt.datetime.now(dt.timezone.utc).isoformat(),
               'scope': 'inclusive elapsed-time attribution, not allocations, heap ownership or full-build throughput',
               'plan': plan, 'binaries': binaries, 'results': results})
    print(f'Raw results: {output / "results.json"}', flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['prepare', 'measure'])
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--revision', default='HEAD')
    parser.add_argument('--repeats', type=int, default=3)
    parser.add_argument('--loads', type=int, default=2)
    args = parser.parse_args()
    root = Path(git(Path(__file__).resolve().parent, 'rev-parse', '--show-toplevel')).resolve()
    output = args.output.resolve()
    if not output.is_relative_to(root / 'runs') or output == root / 'runs':
        parser.error('--output must be a new child directory under this repository\'s runs/')
    if not 1 <= args.repeats <= 20 or not 1 <= args.loads <= 10:
        parser.error('repeats must be 1..20; loads must be 1..10')
    if args.phase == 'prepare':
        prepare(root, output, args.revision)
    else:
        measure(output, args.repeats, args.loads)


if __name__ == '__main__':
    main()
