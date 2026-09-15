#!/usr/bin/env python3
"""Check the actual owned-only Cargo feature and compiled-source closures.

This is an architecture regression check, not a whole CLI distribution/parity claim.
Run separately from workspace builds: Cargo feature unification is intentional.
"""
from __future__ import annotations
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
PACKAGES = ("poe-optimizer-data", "poe-optimizer-engine")
FORBIDDEN = {"poe-optimizer-pob", "poe-optimizer-lua-utf8", "mlua", "mlua-sys",
             "lua-src", "luajit-src", "poe-optimizer-import", "poe-optimizer-native"}


def cargo(*args: str) -> str:
    result = subprocess.run(["cargo", *args], cwd=ROOT, text=True, encoding="utf-8",
                            stdout=subprocess.PIPE, check=True)
    return result.stdout


def allowed_source(package: str, path: str) -> bool:
    prefix = f"crates/{package}/src/"
    if not path.startswith(prefix):
        return False
    relative = path[len(prefix):]
    if relative == "lib.rs" or (relative.startswith("owned_") and relative.endswith(".rs")):
        return True
    return package == "poe-optimizer-engine" and relative in {
        "timing.rs", "timing/ordinary.rs", "resistance.rs", "resistance/ordinary.rs"}


def sources(depfile: Path) -> list[str]:
    # rustc emits an empty (phony) target for each dependency after its main rules.
    # Read those targets, retaining Windows drive colons and escaped spaces.
    result = []
    for line in depfile.read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("#") or not line.endswith(":"):
            continue
        raw = line[:-1].replace("\\ ", " ")
        path = Path(raw)
        if not path.is_absolute():
            path = ROOT / path
        result.append(path.resolve().relative_to(ROOT).as_posix())
    if not result:
        raise ValueError(f"no compiler dependency targets in {depfile}")
    return sorted(set(result))


def main() -> None:
    selected = [value for package in PACKAGES for value in ("-p", package)]
    flags = [*selected, "--no-default-features", "--locked"]
    tree = cargo("tree", *flags, "--edges", "normal,build", "--prefix", "none",
                 "--format", "{p}|{f}")
    for line in tree.splitlines():
        if not line.strip():
            continue
        package, features = line.split("|", 1)
        name = package.split()[0]
        if name in FORBIDDEN or (name in PACKAGES and "legacy" in features.removesuffix(" (*)").strip().split(",")):
            raise ValueError(f"owned-only dependency leak: {line}")
    output = cargo("check", *flags, "--lib", "--message-format=json")
    compiled = {}
    for line in output.splitlines():
        event = json.loads(line)
        if event.get("reason") != "compiler-artifact":
            continue
        package = event["target"]["name"].replace("_", "-")
        if package not in PACKAGES or "lib" not in event["target"]["kind"]:
            continue
        if "legacy" in event["features"]:
            raise ValueError(f"legacy feature compiled for {package}")
        files = [Path(f) for f in event["filenames"] if f.endswith(".rmeta")]
        if len(files) != 1:
            raise ValueError(f"expected one metadata artifact for {package}")
        file = files[0]
        stem = file.stem.removeprefix("lib")
        inputs = sources(file.with_name(stem + ".d"))
        unexpected = [f for f in inputs if not allowed_source(package, f)]
        if unexpected:
            raise ValueError(f"unexpected compiled inputs in {package}: {unexpected}")
        compiled[package] = inputs
    if set(compiled) != set(PACKAGES):
        raise ValueError("missing owned library compiler artifacts")
    print(json.dumps({"scope": "owned-data-and-engine-libraries", "features": "no-default-features",
                      "compiled_sources": compiled, "dependency_tree": tree.splitlines()}, indent=2))


if __name__ == "__main__":
    main()
