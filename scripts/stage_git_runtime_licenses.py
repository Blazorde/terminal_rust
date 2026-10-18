#!/usr/bin/env python3
"""Collect license texts and metadata for the conda Git runtime."""

from __future__ import annotations

import json
import re
import shutil
import sys
from pathlib import Path


def safe_name(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9._+-]+", "_", value)


def package_license_source(record: dict[str, object]) -> Path | None:
    link = record.get("link")
    if isinstance(link, dict):
        source = link.get("source")
        if isinstance(source, str):
            return Path(source) / "info" / "licenses"
    return None


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: stage_git_runtime_licenses.py OUTPUT_DIR", file=sys.stderr)
        return 2

    prefix = Path(sys.prefix).resolve()
    output = Path(sys.argv[1]).resolve()
    metadata_dir = prefix / "conda-meta"
    if not metadata_dir.is_dir():
        print(f"conda metadata directory not found: {metadata_dir}", file=sys.stderr)
        return 1

    licenses_root = output / "GIT-RUNTIME-LICENSES"
    if licenses_root.exists():
        shutil.rmtree(licenses_root)
    licenses_root.mkdir(parents=True)

    rows: list[tuple[str, str, str, str, str]] = []
    for metadata_path in sorted(metadata_dir.glob("*.json")):
        record = json.loads(metadata_path.read_text(encoding="utf-8"))
        name = str(record.get("name", metadata_path.stem))
        version = str(record.get("version", "unknown"))
        license_name = str(record.get("license", "Not specified"))
        package_url = str(record.get("url", record.get("channel", "")))
        source = package_license_source(record)
        copied_files: list[str] = []

        if source is not None and source.is_dir():
            destination = licenses_root / f"{safe_name(name)}-{safe_name(version)}"
            for license_file in sorted(path for path in source.rglob("*") if path.is_file()):
                relative = license_file.relative_to(source)
                target = destination / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(license_file, target)
                copied_files.append(target.relative_to(output).as_posix())

        rows.append(
            (
                name,
                version,
                license_name,
                package_url,
                ", ".join(f"`{path}`" for path in copied_files) or "No package license text found",
            )
        )

    lines = [
        "# Git runtime third-party notices",
        "",
        "This file is generated from the conda package records used to build the bundled Git runtime.",
        "The package license files found in those records are included under `GIT-RUNTIME-LICENSES/`.",
        "",
        "| Package | Version | Declared license | Package source | Included license files |",
        "| --- | --- | --- | --- | --- |",
    ]
    lines.extend(
        f"| {name} | {version} | {license_name} | {package_url or 'Not recorded'} | {license_files} |"
        for name, version, license_name, package_url, license_files in rows
    )
    lines.append("")
    (output / "GIT-RUNTIME-NOTICES.md").write_text("\n".join(lines), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
