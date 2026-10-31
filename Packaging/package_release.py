"""Package a native release build and verify the extracted archive (Python 3.12+)."""

import argparse
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import zipfile

from licenses import collect_licenses
from runtime_libraries import prepare_runtime, verify_runtime


ROOT = Path(__file__).resolve().parents[1]
TARGETS = {
    ("win32", "amd64"): ("windows-x64", "x86_64-pc-windows-msvc"),
    ("linux", "x86_64"): ("linux-x64", "x86_64-unknown-linux-gnu"),
    ("darwin", "arm64"): ("macos-arm64", "aarch64-apple-darwin"),
    ("darwin", "x86_64"): ("macos-x64", "x86_64-apple-darwin"),
}


@dataclass(frozen=True)
class BuildArtifacts:
    executable: Path
    native_install: Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-log", required=True, type=Path, help="Cargo release build JSON output")
    parser.add_argument("--cargo-metadata", required=True, type=Path, help="Target-filtered Cargo metadata JSON")
    parser.add_argument("--output", type=Path, default=ROOT / "Dist/Releases")
    arguments = parser.parse_args()

    host = (sys.platform, platform.machine().lower())
    if host not in TARGETS:
        raise RuntimeError(f"Unsupported release host: {host}")
    label, target = TARGETS[host]
    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?", version):
        raise RuntimeError(f"Invalid package version: {version}")
    artifacts = read_build_artifacts(arguments.build_log)
    verify_build_target(target)
    stem = f"commander-wolf-{version}-{label}"
    output = arguments.output.resolve()
    output.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(prefix="cwolf-package-") as temporary:
        directory = Path(temporary) / stem
        directory.mkdir()
        executable = directory / artifacts.executable.name
        shutil.copy2(artifacts.executable, executable)
        copy_runtime_libraries(artifacts.native_install, directory)
        shutil.copy2(ROOT / "LICENSE", directory / "LICENSE")
        notices = (ROOT / "THIRD_PARTY_NOTICES.md").read_text(encoding="utf-8")
        notices = notices.replace("Vendor/NvimWebDevicons/", "Licenses/NvimWebDevicons/")
        notices = notices.replace("docs/releases.md", "https://github.com/max-wolf-cpp/commander-wolf/blob/master/docs/releases.md")
        (directory / "THIRD_PARTY_NOTICES.md").write_text(notices, encoding="utf-8", newline="\n")
        collect_licenses(ROOT, artifacts.native_install, arguments.cargo_metadata, directory / "Licenses")
        write_package_readme(directory, target)
        write_build_info(directory, version, target, artifacts.native_install)
        prepare_runtime(directory, executable)
        write_checksums(directory)
        archive = create_archive(directory, Path(temporary))

        # Verify the archive itself in a new location, not the original build tree.
        with tempfile.TemporaryDirectory(prefix="cwolf-unpacked-") as unpacked:
            unpack_archive(archive, Path(unpacked))
            extracted = Path(unpacked) / stem
            verify_checksums(extracted)
            verify_runtime(extracted, extracted / executable.name)

        published = output / archive.name
        digest = sha256(archive)
        shutil.copy2(archive, published)
        published.with_name(published.name + ".sha256").write_text(
            f"{digest}  {published.name}\n", encoding="utf-8", newline="\n"
        )

    print(f"Verified release archive: {published}")


def read_build_artifacts(log: Path) -> BuildArtifacts:
    executable = None
    native_install = None
    finished = False
    for line in log.read_text(encoding="utf-8-sig").splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
        if message.get("reason") == "build-finished":
            finished = message["success"]
        if message.get("reason") == "build-script-executed" and "#fox-media-term@" in message["package_id"]:
            native_install = Path(message["out_dir"]) / "cmake-install"
        if message.get("reason") == "compiler-artifact" and message["target"]["name"] == "cwolf" and message.get("executable"):
            executable = Path(message["executable"])
            if message["profile"]["opt_level"] == "0" or message["profile"]["test"]:
                raise RuntimeError("Packaging requires an optimized application build, not a dev/test executable")

    if not finished or executable is None or native_install is None:
        raise RuntimeError("The log must describe a successful Cargo release build of cwolf and Fox Media Term")
    if not executable.is_file() or not native_install.is_dir():
        raise RuntimeError("Build outputs are missing; rebuild before packaging")
    native_info = json.loads((native_install / "share/fox_media_term/BuildInfo.json").read_text(encoding="utf-8"))
    if native_info["configuration"] not in {"Release", "RelWithDebInfo"}:
        raise RuntimeError("A release archive must use optimized native libraries, not Debug libraries")
    if executable.parent.name != "release":
        raise RuntimeError("Use cargo build --release before packaging")
    return BuildArtifacts(executable.resolve(), native_install.resolve())


def verify_build_target(target: str) -> None:
    compiler = subprocess.check_output(["rustc", "-vV"], text=True)
    if f"host: {target}" not in compiler:
        raise RuntimeError(f"The Rust host toolchain must match the native release target {target}")


def copy_runtime_libraries(install: Path, destination: Path) -> None:
    copied = []
    pattern = {"win32": "*.dll", "darwin": "*.dylib"}.get(sys.platform, "*.so*")
    for source_directory in (install / "bin", install / "lib"):
        if not source_directory.is_dir():
            continue
        for path in sorted(source_directory.glob(pattern)):
            if not path.is_file():
                continue
            target = destination / path.name
            if target.exists() and sha256(target) != sha256(path):
                raise RuntimeError(f"Conflicting runtime libraries named {path.name}")
            # Materialize aliases so ZIP extraction does not need symlink privileges.
            shutil.copy2(path, target)
            copied.append(path.name)
    if not any("fox_media_term" in name for name in copied):
        raise RuntimeError("The installed Fox Media Term shared library is missing")


def write_package_readme(directory: Path, target: str) -> None:
    if sys.platform.startswith("linux"):
        minimum = "This Linux build requires glibc 2.39+ and a compatible libstdc++ runtime.\n"
    elif sys.platform == "darwin":
        minimum = "This macOS build targets macOS 14 or newer and is ad-hoc signed, not notarized.\n"
    else:
        minimum = "This Windows build targets 64-bit Windows 10 or newer.\n"
    text = (
        "Commander Wolf\n\n"
        "Extract the entire archive and keep cwolf and its libraries together.\n"
        "Start cwolf from a terminal; use .\\cwolf.exe in PowerShell or ./cwolf on Unix.\n"
        "Press ? for help. Space then p toggles preview. q quits the main view.\n"
        "Use --clean to start with shipped settings. A Nerd Font is optional;\n"
        "use :icons unicode or :icons none when glyphs are missing.\n\n"
        + minimum + f"Target: {target}\n\n"
        "Documentation: https://github.com/max-wolf-cpp/commander-wolf\n"
        "Licenses: LICENSE, THIRD_PARTY_NOTICES.md, and Licenses/.\n"
    )
    (directory / "README.txt").write_text(text, encoding="utf-8", newline="\n")


def write_build_info(directory: Path, version: str, target: str, native_install: Path) -> None:
    def git(*arguments: str) -> str:
        return subprocess.check_output(["git", "-C", str(ROOT), *arguments], text=True).strip()
    info = {
        "version": version, "target": target, "commit": git("rev-parse", "HEAD"),
        "dirty": bool(git("status", "--porcelain", "--untracked-files=normal")),
        "native": json.loads((native_install / "share/fox_media_term/BuildInfo.json").read_text(encoding="utf-8")),
        "rustc": subprocess.check_output(["rustc", "-V"], text=True).strip(),
        "cargo_lock_sha256": sha256(ROOT / "Cargo.lock"),
        "vcpkg_baseline": json.loads((ROOT / "FoxMediaTerm/vcpkg-configuration.json").read_text())["default-registry"]["baseline"],
    }
    (directory / "BuildInfo.json").write_text(json.dumps(info, indent=2) + "\n", encoding="utf-8", newline="\n")
    shutil.copy2(ROOT / "Cargo.lock", directory / "Licenses/Cargo.lock")


def write_checksums(directory: Path) -> None:
    lines = [f"{sha256(path)}  {path.relative_to(directory).as_posix()}" for path in sorted(directory.rglob("*")) if path.is_file()]
    (directory / "SHA256SUMS.txt").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")


def verify_checksums(directory: Path) -> None:
    for line in (directory / "SHA256SUMS.txt").read_text(encoding="utf-8").splitlines():
        expected, name = line.split("  ", 1)
        if sha256(directory / name) != expected:
            raise RuntimeError(f"Archive checksum mismatch: {name}")


def create_archive(directory: Path, output: Path) -> Path:
    if sys.platform == "win32":
        archive = output / f"{directory.name}.zip"
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as stream:
            for path in sorted(directory.rglob("*")):
                if path.is_file():
                    stream.write(path, path.relative_to(directory.parent))
    else:
        archive = output / f"{directory.name}.tar.gz"
        with tarfile.open(archive, "w:gz") as stream:
            stream.add(directory, arcname=directory.name)
    return archive


def unpack_archive(archive: Path, destination: Path) -> None:
    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive) as stream:
            stream.extractall(destination)
    else:
        with tarfile.open(archive) as stream:
            stream.extractall(destination, filter="data")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"Release packaging failed: {error}", file=sys.stderr)
        sys.exit(1)
