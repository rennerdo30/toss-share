"""Reject mismatched Rust, Dart and generator bridge versions before building."""

from pathlib import Path
import re
import tomllib

root = Path(__file__).resolve().parents[1]
cargo = tomllib.loads((root / "Cargo.toml").read_text())
expected = cargo["workspace"]["dependencies"]["flutter_rust_bridge"].removeprefix("=")
checks = [
    (root / "Makefile", r"cargo install flutter_rust_bridge_codegen --version ([0-9.]+)"),
    (root / "flutter_app/pubspec.yaml", r"(?m)^  flutter_rust_bridge: ([0-9.]+)$"),
    (root / "flutter_app/rust/src/frb_generated.rs", r'FLUTTER_RUST_BRIDGE_CODEGEN_VERSION: &str = "([0-9.]+)"'),
]
for workflow in (root / ".github/workflows").glob("*.yml"):
    if "cargo install flutter_rust_bridge_codegen" in workflow.read_text():
        checks.append((workflow, r"cargo install flutter_rust_bridge_codegen --version ([0-9.]+)"))

errors = []
lock = tomllib.loads((root / "Cargo.lock").read_text())
for name in ("flutter_rust_bridge", "flutter_rust_bridge_macros"):
    versions = [package["version"] for package in lock["package"] if package["name"] == name]
    if versions != [expected]:
        errors.append(f"Cargo.lock {name}: expected {expected}, found {versions}")
for path, pattern in checks:
    versions = re.findall(pattern, path.read_text())
    if not versions or any(version != expected for version in versions):
        errors.append(f"{path.relative_to(root)}: expected {expected}, found {versions}")
if errors:
    raise SystemExit("Bridge version mismatch:\n" + "\n".join(errors))
print(f"Rust, Dart and generator bridge versions agree: {expected}")
