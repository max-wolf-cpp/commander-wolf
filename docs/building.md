# Building and testing

[Back to the README](../README.md)

Commander Wolf is a Cargo workspace containing the Rust application and the safe Rust bindings for Fox Media Term. Cargo invokes the native CMake project automatically. Build for the host platform; the shipped presets are not a cross-compilation setup.

## Requirements

- A current stable Rust toolchain, including Cargo and support for Rust 2024.
- Git and CMake 3.24 or newer on `PATH`.
- A bootstrapped [vcpkg](https://github.com/microsoft/vcpkg) checkout, with `VCPKG_ROOT` pointing to its root.
- A C++23 compiler and matching standard library, including `std::expected`.
- Network access on the first build for Cargo, vcpkg, and CMake downloads.

Use your platform's package manager or official installers for the native toolchain. vcpkg's [host requirements](https://learn.microsoft.com/en-us/vcpkg/concepts/supported-hosts) list the platform utilities needed by its ports.

## Windows

Use an x64 MSVC Rust toolchain. Install Visual Studio or Build Tools with the Desktop development with C++ workload and a Windows SDK. The default native preset additionally needs Clang-CL and Ninja. Open an **x64 Visual Studio developer shell**; `clang-cl`, `rc`, `cmake`, `ninja`, and `cargo` must be discoverable there.

If vcpkg is not installed, a possible location is outside the project under your home directory:

```powershell
git clone https://github.com/microsoft/vcpkg.git "$HOME/vcpkg"
& "$HOME/vcpkg/bootstrap-vcpkg.bat" -disableMetrics
$env:VCPKG_ROOT = "$HOME/vcpkg"
```

If it is already installed, set `VCPKG_ROOT` to that existing checkout. The assignment above applies to the current shell.

Clone and build:

```powershell
git clone https://github.com/max-wolf-cpp/commander-wolf.git
cd commander-wolf
cargo build --release --locked
.\target\release\cwolf.exe .
```

Cargo selects `ClangCL` for the native library. The alternate `MSVC` preset targets the Visual Studio 2022 generator and its matching toolset:

```powershell
$env:FOX_MEDIA_TERM_CMAKE_PRESET = "MSVC"
cargo build --release --locked
```

For another installed toolchain, make machine-specific adjustments in `FoxMediaTerm/CMakeUserPresets.json` rather than editing shared presets. A local preset can inherit from one of the shipped presets and be selected through `FOX_MEDIA_TERM_CMAKE_PRESET`.

## Linux

The supplied `ClangLinux` preset targets x64 Linux and uses Ninja Multi-Config. Install Clang/Clang++, a C++23-capable standard library, Ninja, CMake, and normal native build tools. vcpkg also needs Git, curl, tar, zip, unzip, and pkg-config; some ports use make, autoconf, automake, and libtool.

Bootstrap vcpkg outside the project if needed:

```sh
git clone https://github.com/microsoft/vcpkg.git "$HOME/vcpkg"
"$HOME/vcpkg/bootstrap-vcpkg.sh" -disableMetrics
export VCPKG_ROOT="$HOME/vcpkg"
```

Then build:

```sh
git clone https://github.com/max-wolf-cpp/commander-wolf.git
cd commander-wolf
cargo build --release --locked
./target/release/cwolf .
```

Linux binaries also depend on their platform C/C++ runtime and the native dependencies resolved by vcpkg. A desktop session is needed for clipboard and desktop file-opening integration, but not for ordinary panel navigation. Audio needs an available system audio backend.

## macOS

The `AppleClang` preset uses the Xcode generator, so install **full Xcode**, select it as the active developer directory, and complete its first-run setup. Install CMake and the utilities listed in vcpkg's host requirements, including pkg-config where required by a port. Both the compiler and standard library must support C++23.

Bootstrap vcpkg and build using the same shell commands as Linux above. Cargo selects `AppleClang` automatically. The output is `target/release/cwolf`.

## What the build downloads

| Component | Managed by | Purpose |
|---|---|---|
| Rust crates | Cargo, pinned by `Cargo.lock` | TUI, Lua host, file operations, syntax, documents, and image decoding |
| FFmpeg | vcpkg manifest | Audio/video decoding, resampling/scaling, and WebP preview |
| libsixel | vcpkg with the included overlay port | Sixel output |
| miniaudio | vcpkg manifest | Audio output |
| stb | vcpkg manifest | Native static-image loading/resizing |
| fpng and fpnge | CMake FetchContent, pinned commits | PNG encoding for graphics output |
| GoogleTest | vcpkg `tests` feature | Standalone native tests only |

No system Lua, FFmpeg executable, external archiver, or external PDF reader is required for the corresponding built-in functionality. vcpkg builds the native libraries from the declared manifest; a system `ffmpeg` command is not a substitute. Full-screen external editors are optional and must be configured separately.

The Rust build disables native tests. Its native build/install tree lives under `target/<profile>/build/fox-media-term-*/out/`. The vcpkg baseline, overlay ports, presets, and CMake dependency revisions are included in the repository.

## Debug builds and native configuration

```sh
cargo build --locked
cargo run --locked -- .
cargo run --locked -- --clean .
cargo run --locked -- --protocol hb-tc .
```

Cargo uses native `Debug` for its dev/test profile and native `Release` for its release profile. `FOX_MEDIA_TERM_CMAKE_CONFIG` can explicitly select a native configuration such as `RelWithDebInfo`. `FOX_MEDIA_TERM_CMAKE_PRESET` selects a configure preset. Both are environment variables, not user Lua settings.

## Runtime libraries and distribution

The executable is `cwolf` (`cwolf.exe` on Windows). Cargo stages the Fox Media Term shared library beside executables and test binaries. On Windows, the native install step also gathers its app-local runtime DLL dependencies, so the executable is accompanied by `fox_media_term.dll`, FFmpeg DLLs, and any needed runtime DLLs.

Keep these files together when moving or packaging a build. Embedded Lua defaults and vendored icon tables do not need to be shipped as separate data files. A user `init.lua` remains optional and lives in the per-user configuration directory.

Linux and macOS builds use an executable-relative library search path for the staged Fox Media Term library. Its other native dependencies may still be resolved through system or build-time locations: inspect them with `ldd` (Linux) or `otool -L` (macOS) before distributing binaries to another machine. Building locally does not by itself create a portable archive.

Binary packages must retain the project's license and the applicable notices for the actual dependencies shipped; see [Third-party notices](../THIRD_PARTY_NOTICES.md).

## Rust checks

Run from the repository root with the same toolchain environment used for building:

```sh
cargo fmt --all --check
cargo check --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

## Native library and tests

Fox Media Term can be built and tested independently. From `FoxMediaTerm/`, choose the commands for the host:

| Host | Configure | Build | Test |
|---|---|---|---|
| Windows, Clang-CL | `cmake --preset ClangCL` | `cmake --build --preset "Clang-CL Debug"` | `ctest --preset "Clang-CL Debug Tests"` |
| Windows, MSVC | `cmake --preset MSVC` | `cmake --build --preset "MSVC Debug"` | `ctest --preset "MSVC Debug Tests"` |
| Linux | `cmake --preset ClangLinux` | `cmake --build --preset "Clang Linux Debug"` | `ctest --preset "Clang Linux Debug Tests"` |
| macOS | `cmake --preset AppleClang` | `cmake --build --preset "Apple Clang Debug"` | `ctest --preset "Apple Clang Debug Tests"` |

Release and RelWithDebInfo build/test presets are available too. List available names with `cmake --list-presets=all` from `FoxMediaTerm/`.

Standalone presets enable `BUILD_TESTING` and the vcpkg `tests` feature. They create separate `FoxMediaTerm/Build/<preset>/` output; `FoxMediaTerm/Dist/<preset>/` is the install prefix. Tests cover image loading/resizing, rendering protocols, capability detection, the C ABI, Unicode paths, and audio/video behavior. Tests generate their temporary media fixtures during execution; no private sample-media folder is required.

## Troubleshooting

- **vcpkg toolchain not found:** verify `VCPKG_ROOT` points to a bootstrapped checkout containing `scripts/buildsystems/vcpkg.cmake` and that it is set in the same shell as Cargo.
- **`clang-cl`, `rc`, or Windows SDK missing:** use an x64 Visual Studio developer shell and confirm the Clang tools, Ninja, C++ workload, and SDK are installed.
- **`std::expected` missing:** update the C++ compiler and its standard library. On Linux, Clang also needs an appropriate C++ standard-library installation.
- **A DLL cannot be replaced:** close running applications using the build's DLLs and retry. Windows prevents overwriting a loaded DLL.
- **Native shared library missing at startup:** run the executable beside its staged library files, or rebuild with Cargo to stage them again. On Unix, also inspect the native library's dependencies.
- **Broken icon glyphs:** install a Nerd Font or use `:icons unicode` / `:icons none`.
- **Rendering problems:** try `cwolf --clean --protocol hb-tc .` or `--protocol ag` and report the terminal, OS, protocol, and reproduction steps.
