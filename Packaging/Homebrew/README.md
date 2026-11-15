# Homebrew packaging

This formula installs the published Commander Wolf macOS Apple Silicon archive.
It requires macOS 14 (Sonoma) or newer and provides the `cwolf` command.
Intel Macs and Linux are not supported by this formula.

The complete release stays in Homebrew's `libexec` directory, including its
bundled native libraries, licenses, and checksums. A small executable wrapper in
`bin` starts the packaged binary. No source build or separately installed FFmpeg
is required. The formula preserves library load paths and the release's ad-hoc
signatures; the application is not notarized. A license symlink at the formula
prefix prevents Homebrew's automatic metafile installation from moving the
original license out of the checksummed payload.

## Validate in GitHub Actions

Push the formula and workflow to the public project's default branch, then open
**Actions → Validate Homebrew package → Run workflow**. The workflow runs only
when manually requested and does not publish a package, change a release, or
rebuild the application.

The macOS ARM64 job creates a temporary local tap and checks:

- Homebrew formula style and archive SHA256 verification.
- Installation and the command wrapper through `brew test`.
- Every original payload checksum, library load paths, and code signatures.
- Native startup with library-path overrides removed and an unrelated working directory.
- A short terminal session that renders a fixture file and quits using `q`.
- Uninstallation followed by a second installation and the same runtime checks.

The terminal test uses an isolated home directory, `--clean`, and the half-block
graphics backend. It needs no personal configuration, real display, or audio
device. It does not establish that graphics protocols or audio/video playback
work in a real terminal. The runner uses macOS 15; it does not test the declared
macOS 14 minimum. Ad-hoc signing checks do not establish notarization or Gatekeeper
behavior on a user's Mac.

## Public tap

The validated formula is published in
[`max-wolf-cpp/homebrew-tap`](https://github.com/max-wolf-cpp/homebrew-tap) as
`Formula/cwolf.rb`. Install it with:

```sh
brew tap max-wolf-cpp/tap
brew trust --formula max-wolf-cpp/tap/cwolf
brew install max-wolf-cpp/tap/cwolf
```

Homebrew 6 requires explicit trust for third-party tap formulae. Trust only this
formula unless you intend to trust every package in the tap. A custom tap does
not require inclusion in Homebrew's central repositories.

For later releases, update the formula's archive URL and SHA256 from the
published release's `SHA256SUMS.txt`, run the workflow again, then synchronize
the validated formula to the tap. Keep this source copy and the tap in sync.
