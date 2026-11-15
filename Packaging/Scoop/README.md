# Scoop distribution

`bucket/cwolf.json` installs the published Windows x64 release and exposes the
command `cwolf`. The complete release directory is retained, including Fox Media
Term, FFmpeg, compiler runtime DLLs, and license notices. No separate native
runtime package is required for this bundled release.

## Local verification

Install Scoop using its [official installer](https://github.com/ScoopInstaller/Install)
from a non-administrator PowerShell if it is not already available.

From the Commander Wolf repository root:

```powershell
scoop install .\Packaging\Scoop\bucket\cwolf.json
scoop info cwolf
```

Launch the Scoop-managed executable explicitly when testing alongside WinGet or
another manually installed copy:

```powershell
& "$(scoop prefix cwolf)\cwolf.exe" "D:\path\to\demo"
```

Check navigation, image preview, and audio/video playback. Verify removal with
`scoop uninstall cwolf`, then install again. `Get-Command cwolf -All` shows all
commands that could take precedence in the current shell. PowerShell aliases
also take precedence over the command.

## Publishing a bucket

The manifest is ready for a small standalone bucket repository, with this layout:

```text
bucket/
  cwolf.json
```

Keep the package name `cwolf` and retain the manifest filename. Publish the bucket
only after local installation, playback, and removal have been verified. A bucket
is a Git repository of manifests; application archives stay in Commander Wolf's
GitHub Releases. Users add its actual repository URL through `scoop bucket add`
and can then install `cwolf` by name.

The standalone bucket avoids cloning the application's source and documentation
just to obtain a package manifest. Official Main bucket submissions have separate
[eligibility criteria](https://github.com/ScoopInstaller/Scoop/wiki/Criteria-for-including-apps-in-the-main-bucket).

## Maintaining releases

`checkver` tracks stable GitHub releases. The `autoupdate` settings update the
versioned ZIP URL and extraction directory; the hash is obtained from the
published `SHA256SUMS.txt`, matched to the Windows ZIP filename by Scoop's built-in
checksum-file handling.

These settings support Scoop's maintainer tooling; they do not create a scheduled
job or publish manifest updates automatically. Run the appropriate bucket
`checkver` tooling, inspect and test the updated manifest, then commit and push
it to the bucket. Users receive that updated manifest with `scoop update`.

For version 0.1.0, the published ZIP and SHA256 were verified while preparing the
WinGet manifests. The Scoop manifest passed the official JSON Schema check;
local installation, navigation, media playback, and removal were verified on Windows.
Repeat these checks before publishing manifest changes for a new release.

References: [app manifests](https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests),
[autoupdate](https://github.com/ScoopInstaller/Scoop/wiki/App-Manifest-Autoupdate),
and [buckets](https://github.com/ScoopInstaller/Scoop/wiki/Buckets).
