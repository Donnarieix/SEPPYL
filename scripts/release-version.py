#!/usr/bin/env python3
"""Single source of truth and pre-deployment release consistency check."""
from pathlib import Path
import re
import sys
import tomllib

root = Path(__file__).resolve().parent.parent
def manifest(name):
    return tomllib.loads((root / name / "Cargo.toml").read_text(encoding="utf-8"))

api_version = manifest("api")["package"]["version"]
client_version = manifest("client")["package"]["version"]
if api_version != client_version or not re.fullmatch(r"\d+\.\d+\.\d+", api_version):
    sys.exit("Release blocked: API and Web must have the same valid SemVer version")

locked = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
for name in ("api", "client"):
    versions = [p["version"] for p in locked["package"] if p["name"] == name]
    if versions != [api_version]:
        sys.exit(f"Release blocked: lockfile version mismatch for {name}")

tag = "v" + api_version
notes = root / "docs" / "releases" / f"{tag}.md"
if not notes.is_file() or len(notes.read_text(encoding="utf-8").strip()) < 20:
    sys.exit(f"Release blocked: release notes missing for {tag}")

source = (root / "api/src/routes/miscellaneous/changelog.rs").read_text(encoding="utf-8")
if 'pub const CURRENT_VERSION:&str=concat!("v",env!("CARGO_PKG_VERSION"));' not in source:
    sys.exit("Release blocked: API changelog must derive its version from Cargo.toml")
if f'("{tag}",' not in source:
    sys.exit(f"Release blocked: {tag} is not seeded in the in-app changelog")
web_version = (root / "client/src/routes/legal.rs").read_text(encoding="utf-8")
if 'format!("v{}",env!("CARGO_PKG_VERSION"))' not in web_version:
    sys.exit("Release blocked: Web footer must display the full Cargo SemVer")

if "--check" in sys.argv:
    print(f"Release consistency verified: {tag}")
else:
    print(api_version)
