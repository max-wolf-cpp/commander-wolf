# WinGet distribution

These manifests install the published Windows x64 release as a per-user portable
package named `MaxWolf.CommanderWolf`. They are prepared for local verification;
the package is not yet published in the WinGet community repository.

The ZIP is installed as a complete directory tree, including Fox Media Term,
FFmpeg, compiler runtime DLLs, and license notices. `ArchiveBinariesDependOnPath`
adds the executable's install location to PATH so the adjacent native libraries
remain available. No separate FFmpeg or Visual C++ runtime package is required
for this bundled release.

## Local verification

From the Commander Wolf repository root:

```powershell
winget validate --manifest .\Packaging\WinGet\0.1.0
```

Local manifest installation requires the `LocalManifestFiles` administrator
setting. If it is disabled, run the following once from an elevated terminal:

```powershell
winget settings --enable LocalManifestFiles
```

Return to a regular terminal in the repository root and install:

```powershell
winget install --manifest .\Packaging\WinGet\0.1.0 --scope user
```

Verify the local installation by display name:

```powershell
winget list --name "Commander Wolf" --exact
```

Before the manifest is published in the community source, the installed entry
may appear under an `ARP` identifier ending in `MaxWolf.CommanderWolf__DefaultSource`.
Searching by the future catalog ID with `--id MaxWolf.CommanderWolf --exact`
can therefore return no match even when installation succeeded.

Restart the terminal to pick up PATH changes. Use `Get-Command cwolf -All` to
check whether an older manually installed copy appears before the WinGet copy.
Start `cwolf` and check directory navigation, image preview, and audio/video
playback. These checks exercise native DLL loading as well as the installed
command. Release 0.1.0 does not implement a `--version` command-line option.

The package can be removed with:

```powershell
winget uninstall --name "Commander Wolf" --exact
```

## Community repository submission

After local installation and removal have been verified, submit the three YAML
files to `microsoft/winget-pkgs` under:

```text
manifests/m/MaxWolf/CommanderWolf/0.1.0/
```

Only the YAML manifests belong in that submission; do not include this README
or application binaries. Do not advertise `winget install --id
MaxWolf.CommanderWolf --exact` as a public installation method until the
submission has been merged and the package is available in the source.

For each new published release, add a version directory and update the version,
release URL/date, ZIP SHA256, and nested executable path. Download the actual
published ZIP, verify its checksum and contents, then validate and test the
manifest before submission. Do not replace an existing published release ZIP
to update an installer hash.

References: [manifest format](https://learn.microsoft.com/en-us/windows/package-manager/package/manifest),
[submission process](https://learn.microsoft.com/en-us/windows/package-manager/package/repository),
and [installer schema](https://github.com/microsoft/winget-pkgs/blob/master/doc/manifest/schema/1.12.0/installer.md).
