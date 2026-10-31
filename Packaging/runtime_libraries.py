"""Relocate native libraries and verify that release archives run independently."""

import os
import re
from pathlib import Path
import subprocess
import sys
import tempfile


def prepare_runtime(directory: Path, executable: Path) -> None:
    if sys.platform != "darwin":
        return

    libraries = sorted(directory.glob("*.dylib"))
    for library in libraries:
        run("install_name_tool", "-id", f"@rpath/{library.name}", str(library))

    for binary in [*libraries, executable]:
        for dependency in macos_dependencies(binary):
            if is_macos_system_library(dependency):
                continue
            local = directory / Path(dependency).name
            if not local.is_file():
                raise RuntimeError(f"Unbundled macOS dependency of {binary.name}: {dependency}")
            run("install_name_tool", "-change", dependency, f"@loader_path/{local.name}", str(binary))

    # Changing load commands invalidates the existing signature, including on arm64.
    for binary in [*libraries, executable]:
        run("codesign", "--force", "--sign", "-", str(binary))


def verify_runtime(directory: Path, executable: Path) -> None:
    environment = clean_environment()
    if sys.platform == "win32":
        verify_windows_runtime(directory, executable)
    elif sys.platform.startswith("linux"):
        dynamic = run("readelf", "-d", str(executable), env=environment)
        if "$ORIGIN" not in dynamic:
            raise RuntimeError("The executable has no executable-relative library search path")
        binaries = [executable, *sorted(p for p in directory.iterdir() if ".so" in p.name)]
        for binary in binaries:
            verify_linux_dependencies(binary, directory, environment)
    elif sys.platform == "darwin":
        for binary in [executable, *sorted(directory.glob("*.dylib"))]:
            for dependency in macos_dependencies(binary):
                if is_macos_system_library(dependency):
                    continue
                if not dependency.startswith("@loader_path/") or not (directory / Path(dependency).name).is_file():
                    raise RuntimeError(f"Non-relocatable dependency of {binary.name}: {dependency}")
            run("codesign", "--verify", "--strict", str(binary))

    # Extra positional arguments exit before terminal initialization. This checks the
    # actual loader without needing a TTY, display server, audio device, or config.
    with tempfile.TemporaryDirectory(prefix="cwolf-release-cwd-") as cwd:
        result = subprocess.run(
            [str(executable), "__package_probe_one__", "__package_probe_two__"],
            cwd=cwd, env=environment, capture_output=True, text=True, timeout=30,
        )
    expected = "cwolf: usage: cwolf [--clean] [--protocol PROTOCOL] [path]"
    if result.returncode != 1 or result.stderr.strip() != expected:
        raise RuntimeError(
            f"Packaged executable failed its loader check (exit {result.returncode}): "
            f"{result.stdout}{result.stderr}"
        )


def verify_windows_runtime(directory: Path, executable: Path) -> None:
    bundled = {path.name.lower() for path in directory.iterdir() if path.is_file()}
    runtime_name = re.compile(r"(?:msvcp|vcruntime|concrt|vccorlib|vcomp)[0-9]+(?:_[a-z0-9_]+)?d?[.]dll", re.IGNORECASE)
    for binary in [executable, *sorted(directory.glob("*.dll"))]:
        imports = run("dumpbin", "/nologo", "/dependents", str(binary))
        for line in imports.splitlines():
            name = line.strip()
            if runtime_name.fullmatch(name) and name.lower().endswith("d.dll"):
                raise RuntimeError(f"Debug compiler runtimes cannot be included in a release: {name}")
            if runtime_name.fullmatch(name) and name.lower() not in bundled:
                raise RuntimeError(f"The redistributable compiler runtime {name} required by {binary.name} is not bundled")


def verify_linux_dependencies(binary: Path, directory: Path, environment: dict[str, str]) -> None:
    output = run("ldd", str(binary), env=environment)
    if "not found" in output:
        raise RuntimeError(f"Missing dependency of {binary.name}:\n{output}")

    for line in output.splitlines():
        value = line.split("=>", 1)[-1].strip().split(" (", 1)[0]
        if not value.startswith("/"):
            continue
        dependency = Path(value).resolve()
        if dependency.is_relative_to(directory.resolve()):
            continue
        if any(value.startswith(prefix) for prefix in ("/lib/", "/lib64/", "/usr/lib/", "/usr/lib64/")):
            continue
        raise RuntimeError(f"{binary.name} still depends on a build-machine library: {value}")


def macos_dependencies(binary: Path) -> list[str]:
    names = [line.strip().split(" (", 1)[0] for line in run("otool", "-L", str(binary)).splitlines()[1:]]
    # otool prints a dylib's own LC_ID_DYLIB before its imported libraries.
    return names[1:] if binary.suffix == ".dylib" else names


def is_macos_system_library(name: str) -> bool:
    return name.startswith(("/usr/lib/", "/System/Library/"))


def clean_environment() -> dict[str, str]:
    environment = {
        key: value for key, value in os.environ.items()
        if not key.startswith(("LD_", "DYLD_")) and key not in {"LIBRARY_PATH", "LIB", "LIBPATH"}
    }
    if sys.platform == "win32":
        system_directory = environment.get("SystemRoot") or environment.get("WINDIR")
        if not system_directory:
            raise RuntimeError("SystemRoot is not set; cannot isolate the Windows loader check")
        system_root = Path(system_directory)
        environment["PATH"] = str(system_root / "System32") + os.pathsep + str(system_root)
    return environment


def run(*arguments: str, **kwargs) -> str:
    result = subprocess.run(arguments, capture_output=True, text=True, **kwargs)
    if result.returncode:
        raise RuntimeError(f"{arguments[0]} failed for {arguments[-1]}:\n{result.stdout}{result.stderr}")
    return result.stdout
