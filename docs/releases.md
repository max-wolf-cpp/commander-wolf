# Release builds

[Back to the README](../README.md)

The [release workflow](../.github/workflows/release.yml) runs only through **Actions → Build release archives → Run workflow**. It has no push, tag, pull-request, or schedule trigger. Every job is skipped while the repository is private, before allocating a runner.

## Platforms

| Archive | Runner | Native toolchain | Runtime baseline |
|---|---|---|---|
| Windows x64 ZIP | `windows-2022` | Clang-CL with the Visual Studio x64 SDK environment | Windows 10+; compiler runtime DLLs bundled |
| Linux x64 tar.gz | `ubuntu-24.04` | LLVM 20 with libstdc++ | glibc 2.39+ and a compatible libstdc++ runtime |
| macOS arm64 tar.gz | `macos-15` | AppleClang/Xcode | Apple Silicon, macOS 14+; ad-hoc signed, not notarized |

The matrix builds natively. Passing another Rust `--target` is not enough to cross-compile the C++ dependencies, and the binding build script rejects mismatched host/target builds. An Intel macOS job is not part of the initial matrix.

The workflow pins Rust 1.99.1 and official actions by commit SHA. It bootstraps vcpkg at the manifest baseline, installs Linux compiler prerequisites, and exports the Windows developer environment between steps. Rust downloads and vcpkg binary packages are cached; CMake build trees and their machine-specific absolute paths are not cached. Temporary Actions artifacts expire after three days. Standard runner compute is free for public repositories; artifact/cache storage still has separate allowances.

## Local Windows verification

Use PowerShell 7 from the repository root. `clang-cl`, CMake, Ninja, Git, Python 3.12+, Rust, and a bootstrapped vcpkg checkout must be installed. Set `VCPKG_ROOT` to that checkout before running these commands.

Initialize the SDK/compiler environment and run the checks:

```powershell
& .\Packaging\SetupWindows.ps1
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Check each command's result before continuing. Native tests can also be run from `FoxMediaTerm/` using the presets in [Building and testing](building.md#native-library-and-tests).

Capture the release build and metadata, then package it:

```powershell
$taskBuildLog = Join-Path $env:TEMP 'cwolf-build.jsonl'
$taskMetadata = Join-Path $env:TEMP 'cwolf-metadata.json'

cargo build --release --locked --bin cwolf --message-format=json-render-diagnostics > $taskBuildLog
if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }

cargo metadata --locked --filter-platform x86_64-pc-windows-msvc --format-version 1 > $taskMetadata
if ($LASTEXITCODE -ne 0) { throw 'Dependency metadata failed.' }

python Packaging/package_release.py --build-log $taskBuildLog --cargo-metadata $taskMetadata
if ($LASTEXITCODE -ne 0) { throw 'Release packaging failed.' }
```

The archive and its `.sha256` file appear under `Dist/Releases/`. The packager uses the exact native install directory reported by this Cargo build rather than guessing between old build directories. Rebuild before packaging after source changes. Native build configuration/compiler/triplet details are installed by CMake and recorded in the archive; Debug native libraries and debug compiler runtimes are rejected. Local uncommitted changes are recorded as `dirty: true` in `BuildInfo.json`.

## What packaging checks

The archive contains the executable, installed native runtime libraries, the project license, original native/vendored dependency notices, Rust dependency notices and their inventory, `Cargo.lock`, build information, and per-file SHA256 checksums. If a crate ships no recognizable notice file, packaging reports its name and records the gap in `Licenses/Rust/dependencies.json`; review such cases before publishing.

CMake includes redistributable compiler runtime DLLs for optimized Windows configurations. The packager checks the actual PE imports, so a Visual C++ runtime already installed on the build machine cannot mask its absence from the archive. On macOS it rewrites non-system dependency paths, assigns local dylib identities, and applies ad-hoc signatures. On Linux it checks that dependencies resolve inside the package or through system runtime locations.

After creating an archive, packaging extracts it into a new temporary location, verifies its checksums, and starts the executable with arguments that exit before terminal initialization. The loader check removes library-path overrides and uses an unrelated working directory; on Windows it also removes developer tools from `PATH`. This verifies startup/loading without a TTY. It does not replace manual checks of the terminal UI, graphics protocols, media playback, or desktop integration.

## First GitHub run

1. Commit and push the prepared project and workflow to the default branch.
2. Make the repository public before starting Actions. There is no automatic build on push.
3. Open **Actions → Build release archives → Run workflow**.
4. Enter the release tag matching `v` plus the version in `Cargo.toml`, initially `v0.1.0`. If that tag exists, it must point to the selected workflow commit.
5. Leave **Create or update a draft release** enabled to prepare a release after all three platforms pass. Disable it to receive only short-lived Actions artifacts.
6. Inspect the jobs. Each platform runs Rust formatting, Rust/binding tests, Clippy, standalone native tests, the release build, packaging, and extracted-archive loader checks.
7. Download the archives, check the relevant terminal/media scenarios, and review dependency notices. Only then publish the draft manually.

The workflow never publishes a release. It refuses to replace assets of an already published release. A successful rerun may replace files in an existing draft for the same tag/commit; it does not move an existing tag. If the tag does not exist, the draft targets the workflow commit for the release tag. Updating that draft also updates its target commit and notes without moving an existing tag.

The final job combines and verifies the three archive checksum files and attaches the archives plus `SHA256SUMS.txt`. To publish a later version, update the Cargo package version/lockfile as appropriate, commit the changes, and dispatch the matching tag.
