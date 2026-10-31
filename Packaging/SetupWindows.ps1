$ErrorActionPreference = 'Stop'

# Keep the discovered LLVM ahead of the developer shell's compiler search paths.
$taskLlvmDirectory = Split-Path (Get-Command clang-cl.exe -ErrorAction Stop).Source
$taskVswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$taskInstallation = & $taskVswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $taskInstallation) {
    throw 'Visual Studio C++ build tools were not found.'
}

$taskBefore = @{}
Get-ChildItem Env: | ForEach-Object { $taskBefore[$_.Name] = $_.Value }
& (Join-Path $taskInstallation 'Common7/Tools/Launch-VsDevShell.ps1') -Arch amd64 -HostArch amd64 -SkipAutomaticLocation
$env:PATH = "$taskLlvmDirectory;$env:PATH"

# A local invocation only sets this PowerShell process. CI exports changes to
# later steps as well; no developer-machine paths are stored in the repository.
if ($env:GITHUB_ENV) {
    Get-ChildItem Env: | Where-Object {
        $_.Value -ne $taskBefore[$_.Name] -and $_.Name -notmatch '^(GITHUB_|RUNNER_)'
    } | ForEach-Object {
        if ($_.Value -match "[\r\n]") { throw "Cannot export multiline environment variable $($_.Name)" }
        "$($_.Name)=$($_.Value)" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
    }
}
