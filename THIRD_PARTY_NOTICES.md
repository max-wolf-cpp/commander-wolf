# Third-party notices

Commander Wolf's original source code is covered by the root [MIT license](LICENSE). This does not replace the licenses of third-party data or dependencies.

## Vendored source data

`Vendor/NvimWebDevicons/` contains filename and extension icon tables from [nvim-tree/nvim-web-devicons](https://github.com/nvim-tree/nvim-web-devicons), at the revision recorded in its [README](Vendor/NvimWebDevicons/README.md). Its original [MIT license and copyright notice](Vendor/NvimWebDevicons/LICENSE) are included unchanged.

## Build-managed dependencies

Rust dependencies are declared in `Cargo.toml` and `FoxMediaTerm/Bindings/Rust/Cargo.toml`, with exact resolved versions in `Cargo.lock`. Their own manifests and distributed source packages contain their license information. They include Ratatui/Crossterm, mlua and Lua, syntect and its bundled syntax data, document decoders, image decoders, clipboard support, and archive libraries.

The native dependency set is declared in `FoxMediaTerm/vcpkg.json` and `FoxMediaTerm/CMakeLists.txt`:

- [FFmpeg](https://ffmpeg.org/legal.html)
- [libsixel](https://github.com/saitoha/libsixel)
- [miniaudio](https://github.com/mackron/miniaudio)
- [stb](https://github.com/nothings/stb)
- [fpng](https://github.com/richgel999/fpng)
- [fpnge](https://github.com/veluca93/fpnge)
- [GoogleTest](https://github.com/google/googletest), used by native tests

These dependencies are downloaded by the build tools rather than copied into this source repository. vcpkg records native port copyright/license files under each installed triplet's `share/<port>/copyright`; CMake-fetched projects include their upstream license files in their downloaded source trees.

## Binary redistribution

Include `LICENSE`, this notice, the vendored data license, and the applicable notices for the actual dependency versions included in a binary package. Rust dependencies compiled into the executable and native dependencies compiled into or shipped alongside Fox Media Term retain their respective license terms.

FFmpeg's license and redistribution requirements depend on its enabled build configuration; refer to its [licensing information](https://ffmpeg.org/legal.html) and the actual vcpkg build configuration. The project's MIT license does not relicense FFmpeg or other third-party libraries.

## Generated release packages

The release packager collects the installed native notices and Rust crate notice files into `Licenses/`. `Licenses/Rust/dependencies.json` records the resolved crates, license expressions, authors, source references, and collected files. Crates with no recognized distributed notice file are reported for review before publication. The inventory is a record of the build, not a replacement for the original licenses. See [Release builds](docs/releases.md) for packaging and review steps.
