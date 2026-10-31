"""Collect original dependency notices for the exact Cargo/native build."""

import json
from pathlib import Path
import shutil
import sys


NOTICE_PREFIXES = ("license", "licence", "copying", "copyright", "notice", "unlicense")


def collect_licenses(root: Path, native_install: Path, metadata_path: Path, destination: Path) -> None:
    destination.mkdir()
    shutil.copytree(root / "Vendor/NvimWebDevicons", destination / "NvimWebDevicons", ignore=shutil.ignore_patterns("*.lua", "Light"))

    native_notices = native_install / "share/fox_media_term/licenses"
    if not native_notices.is_dir():
        raise RuntimeError(f"Native license installation is missing: {native_notices}")
    shutil.copytree(native_notices, destination / "Native")
    collect_rust_licenses(metadata_path, destination / "Rust")


def collect_rust_licenses(metadata_path: Path, destination: Path) -> None:
    metadata = json.loads(metadata_path.read_text(encoding="utf-8-sig"))
    resolved = {node["id"] for node in metadata["resolve"]["nodes"]}
    packages = sorted(metadata["packages"], key=lambda package: (package["name"], package["version"]))
    inventory = []

    for package in packages:
        if package["id"] not in resolved or package["source"] is None:
            continue
        source = Path(package["manifest_path"]).parent.resolve()
        notices = notice_files(source, package.get("license_file"))
        package_directory = destination / f"{package['name']}-{package['version']}"
        for notice in notices:
            target = package_directory / notice.relative_to(source)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(notice, target)

        inventory.append({
            "name": package["name"], "version": package["version"],
            "license": package.get("license"), "authors": package.get("authors", []),
            "source": package["source"], "repository": package.get("repository"),
            "notice_files": [str(path.relative_to(source)).replace("\\", "/") for path in notices],
        })
        if not notices:
            print(f"Warning: no distributed notice file found for {package['name']} {package['version']}; see the license inventory.", file=sys.stderr)

    destination.mkdir(parents=True, exist_ok=True)
    (destination / "dependencies.json").write_text(json.dumps(inventory, indent=2) + "\n", encoding="utf-8", newline="\n")


def notice_files(source: Path, license_file: str | None) -> list[Path]:
    notices = set()
    for path in source.rglob("*"):
        if not path.is_file():
            continue
        name = path.name.lower()
        inside_license_directory = any(part.lower() in {"licenses", "licences"} for part in path.relative_to(source).parts[:-1])
        if name.startswith(NOTICE_PREFIXES) or (inside_license_directory and path.suffix.lower() in {".txt", ".md", ".rst"}):
            if not path.resolve().is_relative_to(source):
                raise RuntimeError(f"License symlink leaves the crate source: {path}")
            notices.add(path)

    if license_file:
        path = source / license_file
        if not path.is_file() or not path.resolve().is_relative_to(source):
            raise RuntimeError(f"Invalid crate license_file: {path}")
        notices.add(path)
    return sorted(notices)
