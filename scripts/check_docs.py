#!/usr/bin/env python3
"""Check small, machine-readable documentation contracts."""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path
from urllib.parse import unquote, urlsplit


ROOT = Path(__file__).resolve().parents[1]
README_PATH = ROOT / "README.md"
QUICKSTART_PATH = ROOT / "examples" / "quickstart.rs"
QUICKSTART_RE = re.compile(
    r"<!-- BEGIN QUICKSTART -->\s*```rust\n(?P<code>.*?)\n```\s*"
    r"<!-- END QUICKSTART -->",
    re.DOTALL,
)
FEATURES_RE = re.compile(
    r"<!-- BEGIN CARGO FEATURES -->(?P<table>.*?)"
    r"<!-- END CARGO FEATURES -->",
    re.DOTALL,
)
FEATURE_ROW_RE = re.compile(r"^\|\s*`([^`]+)`\s*\|", re.MULTILINE)
LINK_RE = re.compile(r"!?\[[^\]\n]*\]\((?P<target><[^>]+>|[^\s)]+)")
BANNED_README_TEXT = (
    "complete implementation of all Anthropic API endpoints",
    "full API coverage",
    'threatflux-anthropic-sdk = "0.1',
    "CLAUDE.md#contributing",
)


def load_manifest() -> dict:
    with (ROOT / "Cargo.toml").open("rb") as manifest_file:
        return tomllib.load(manifest_file)


def extract(pattern: re.Pattern[str], text: str, name: str, errors: list[str]) -> str:
    match = pattern.search(text)
    if match is None:
        errors.append(f"README.md: missing {name} markers")
        return ""
    return match.group(1).strip()


def check_manifest_values(
    readme: str, package: dict, errors: list[str]
) -> None:
    version = package["version"]
    rust_version = package["rust-version"]
    expected_version = f"current crates.io release is `{version}`"
    expected_msrv = f"Rust {rust_version} or newer"
    if expected_version not in readme:
        errors.append(f"README.md: expected package version statement: {version}")
    if expected_msrv not in readme:
        errors.append(f"README.md: expected MSRV statement: {rust_version}")


def check_features(readme: str, manifest: dict, errors: list[str]) -> None:
    table = extract(FEATURES_RE, readme, "Cargo feature", errors)
    documented = set(FEATURE_ROW_RE.findall(table))
    expected = set(manifest.get("features", {}))
    if documented != expected:
        errors.append(
            "README.md: Cargo feature table differs from Cargo.toml "
            f"(documented={sorted(documented)}, expected={sorted(expected)})"
        )


def check_quickstart(readme: str, errors: list[str]) -> None:
    documented = extract(QUICKSTART_RE, readme, "quickstart", errors)
    compiled = QUICKSTART_PATH.read_text(encoding="utf-8").strip()
    if documented != compiled:
        errors.append(
            "README.md: quickstart code must exactly match examples/quickstart.rs"
        )


def documentation_files() -> list[Path]:
    root_files = [README_PATH, ROOT / "CONTRIBUTING.md", ROOT / "SECURITY.md"]
    return root_files + sorted((ROOT / "docs").glob("**/*.md"))


def local_target(raw_target: str) -> str | None:
    target = raw_target.strip("<>")
    parsed = urlsplit(target)
    if parsed.scheme or parsed.netloc or target.startswith("#"):
        return None
    return unquote(parsed.path)


def target_is_inside_root(target: Path) -> bool:
    return target == ROOT or ROOT in target.parents


def check_local_links(errors: list[str]) -> None:
    for markdown_path in documentation_files():
        text = markdown_path.read_text(encoding="utf-8")
        for match in LINK_RE.finditer(text):
            relative = local_target(match.group("target"))
            if not relative:
                continue
            resolved = (markdown_path.parent / relative).resolve()
            if not target_is_inside_root(resolved):
                errors.append(f"{markdown_path.relative_to(ROOT)}: link escapes repository")
            elif not resolved.exists():
                errors.append(
                    f"{markdown_path.relative_to(ROOT)}: missing local link target {relative}"
                )


def check_readme_language(readme: str, errors: list[str]) -> None:
    if "unofficial, community-maintained SDK" not in readme:
        errors.append("README.md: missing unofficial affiliation disclosure")
    for banned in BANNED_README_TEXT:
        if banned.lower() in readme.lower():
            errors.append(f"README.md: banned stale or absolute text: {banned}")


def print_errors(errors: list[str]) -> int:
    if not errors:
        print("Documentation contract passed.")
        return 0
    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    return 1


def main() -> int:
    manifest = load_manifest()
    if sys.argv[1:] == ["--print-msrv"]:
        print(manifest["package"]["rust-version"])
        return 0
    if sys.argv[1:]:
        print("usage: check_docs.py [--print-msrv]", file=sys.stderr)
        return 2

    readme = README_PATH.read_text(encoding="utf-8")
    errors: list[str] = []
    check_manifest_values(readme, manifest["package"], errors)
    check_features(readme, manifest, errors)
    check_quickstart(readme, errors)
    check_local_links(errors)
    check_readme_language(readme, errors)
    return print_errors(errors)


if __name__ == "__main__":
    raise SystemExit(main())
