#!/usr/bin/env python3
"""Project an integration commit into a standalone web-only Git tree.

Writes Git objects using a temporary index, never modifies the working tree,
branches, or history. Keep exclusions conservative: shared desktop modules
are still dependencies of the headless runtime.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

DROP_DIRS = (".github/actions/", ".github/signing/", "packaging/", "web-export/")
DROP_FILES = {
    ".github/workflows/aur.yml",
    ".github/workflows/build.yml",
    ".github/workflows/release.yml",
    "docs/LINUX.md",
    "scripts/check-linux-packages.sh",
    "scripts/flatpak-sources.sh",
    "scripts/postprocess-appimage.sh",
    "scripts/sign-updater-files.sh",
    "scripts/smoke-test-linux.sh",
}
OVERLAY = "web-export/overlay/"
NATIVE_ROOT = "src-tauri/"
VENDOR_ROOT = "vendor/flow-desktop/"
NATIVE_DROP_DIRS = ("icons/android/", "icons/ios/", "linux/")

def git(*args, data=None, env=None):
    return subprocess.check_output(["git", *args], input=data, env=env)

def tree_entries(commit):
    result = {}
    for entry in git("ls-tree", "-r", "-z", commit).split(b"\0"):
        if not entry:
            continue
        meta, name = entry.split(b"\t", 1)
        mode, kind, oid = meta.split()
        if kind != b"blob":
            raise ValueError(f"Unsupported Git entry: {name!r} ({kind!r})")
        result[os.fsdecode(name)] = (mode, oid)
    return result

def project(commit):
    source = tree_entries(commit)
    output = {
        name: value for name, value in source.items()
        if name not in DROP_FILES and not name.startswith(DROP_DIRS)
        and not name.startswith(NATIVE_ROOT) and name != "UPSTREAM.lock"
    }
    # Keep upstream files byte-identical and preserve their relative layout.
    # A legacy symlink avoids rewriting every cross-runtime import and SQLx path.
    if any(name.startswith(VENDOR_ROOT) or name == "src-tauri" for name in source):
        raise ValueError("Integration source already contains the projected layout")
    for name, value in source.items():
        if name.startswith(NATIVE_ROOT):
            relative = name[len(NATIVE_ROOT):]
            if not relative.startswith(NATIVE_DROP_DIRS):
                output[VENDOR_ROOT + relative] = value
    alias_oid = git("hash-object", "-w", "--stdin", data=VENDOR_ROOT.rstrip("/").encode()).strip()
    output["src-tauri"] = (b"120000", alias_oid)
    for name, value in source.items():
        if name.startswith(OVERLAY):
            target = name[len(OVERLAY):]
            if (not target or target in ("UPSTREAM.lock", "src-tauri")
                    or target.startswith(("web-export/", NATIVE_ROOT, VENDOR_ROOT))):
                raise ValueError(f"Invalid overlay destination: {target}")
            output[target] = value
    required = ("LICENSE", "README.md", "package.json", "pnpm-lock.yaml",
                "Containerfile", "compose.web.yml", "web-server/Cargo.toml",
                "web-server/migrations", "vendor/flow-desktop/src/api/mod.rs",
                "src-tauri",
                "scripts/export-web.py", ".github/workflows/web-check.yml")
    missing = [name for name in required if name not in output]
    if not any(name.startswith(VENDOR_ROOT + "migrations/") for name in output):
        missing.append(VENDOR_ROOT + "migrations/")
    if missing:
        raise ValueError(f"Missing required web paths: {', '.join(missing)}")
    if not any(name.startswith(OVERLAY) for name in source):
        raise ValueError("Source must be a full integration commit, not a web export")
    lock = {
        "format": 2,
        "upstream_repository": "https://github.com/Flow-Tube/Flow-Desktop",
        "integration_commit": commit,
    }
    if "FLOW_DESKTOP_REVISION" in source:
        revision = git("cat-file", "blob", source["FLOW_DESKTOP_REVISION"][1].decode()).decode().strip()
        if len(revision) != 40 or any(c not in "0123456789abcdef" for c in revision):
            raise ValueError("Invalid FLOW_DESKTOP_REVISION")
        lock["upstream_commit"] = revision
    lock_data = (json.dumps(lock, indent=2) + "\n").encode()
    oid = git("hash-object", "-w", "--stdin", data=lock_data).strip()
    output["UPSTREAM.lock"] = (b"100644", oid)
    with tempfile.TemporaryDirectory(prefix="flow-web-export-") as tmp:
        env = dict(os.environ, GIT_INDEX_FILE=str(Path(tmp) / "index"))
        git("read-tree", "--empty", env=env)
        payload = b"".join(mode + b" " + oid + b"\t" + os.fsencode(name) + b"\0"
                           for name, (mode, oid) in sorted(output.items()))
        git("update-index", "-z", "--index-info", data=payload, env=env)
        return git("write-tree", env=env).decode().strip()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", help="Integration commit or ref")
    parser.add_argument("--verify", metavar="REF", help="Require exact tree equality with this ref")
    args = parser.parse_args()
    commit = git("rev-parse", "--verify", "--end-of-options", args.source + "^{commit}").decode().strip()
    tree = project(commit)
    if args.verify:
        expected = git("rev-parse", "--verify", "--end-of-options", args.verify + "^{tree}").decode().strip()
        if tree != expected:
            subprocess.run(["git", "diff", "--stat", expected, tree], check=True)
            parser.exit(1, f"Web export mismatch: expected {expected}, generated {tree}\n")
    print(tree)

if __name__ == "__main__":
    main()
