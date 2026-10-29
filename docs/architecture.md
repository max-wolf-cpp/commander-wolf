# Architecture

[Back to the README](../README.md)

- Application input follows `Event → CommandInvocation → Action → update() → Effect`. The reducer changes `AppState`; filesystem, terminal, Lua, and media work stays in the runtime/effect runner.
- File mutations are self-contained typed requests. Results distinguish success, failure, and changed-but-incomplete outcomes; two-panel refresh is attempted independently after any possible mutation.
- Incremental search is a pure `AppState` operation over loaded entries. Recursive finder work is isolated in runtime-owned cancellable threads that publish typed result batches tagged with a generation; stale generations are ignored by the reducer.
- `AppState` contains two independent panel states, generation-tagged preview/viewer presentation data, document page/status/text models, and no decoders, native resources, or raster buffers. Native audio/video handles, document decoders and caches, Lua, terminal output, filesystem access, and media buffers are runtime-owned.
- `DocumentRuntime` owns document decoding, reflow, rasterization, cancellation, and the bounded current/previous/next page cache. Request IDs and preview generations prevent late results from replacing a newer selection or viewport.
- `ui::draw()` receives immutable state and a typed Lua-derived theme. A pure derived layout is shared by the view and media runtime.
- Raw keys are normalized and resolved through context keymaps with multi-key sequences, leaders, counts, pending state, replacement, deletion, and context clearing.
- `FoxMediaTerm/` is the standalone native media project.
- `FoxMediaTerm/Include/FoxMediaTerm/FoxMediaTerm.h` is the only public native boundary. It is a C-compatible ABI with opaque handles, length-delimited owned buffers, a validated media probe, and a stride-aware RGBA8 entry point for already decoded Rust rasters.
- `FoxMediaTerm/Bindings/Rust/` owns all raw FFI declarations and exposes safe RAII Rust types to Commander Wolf.

Ratatui marks only an image, video, or raster-document region that owns an external terminal graphics layer—not preview metadata, reflowed ebook text, or the audio visualizer—with `CellDiffOption::Skip`. `MediaRuntime` records the exact preview or viewer region that actually received a payload so cleanup remains correct across resize and mode changes. `TerminalWriter` is the sole serialization boundary for both Ratatui frames and direct Fox Media Term payloads. Video and document frames remain on that data plane and do not trigger unrelated UI writes; ebook text and audio amplitude snapshots are ordinary generation-scoped view data rendered through Ratatui.
