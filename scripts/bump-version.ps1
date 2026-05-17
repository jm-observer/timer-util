param(
    [Parameter(Mandatory=$true, Position=0)]
    [string]$NewVersion
)

if ($NewVersion -notmatch '^\d+\.\d+\.\d+$') {
    Write-Error "Error: version must be in semver format (e.g. 1.2.3)"
    exit 1
}

$RepoRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$CargoToml = Join-Path $RepoRoot "Cargo.toml"

$content = Get-Content $CargoToml -Raw
if ($content -match 'version\s*=\s*"(\d+\.\d+\.\d+)"') {
    $CurrentVersion = $Matches[1]
} else {
    Write-Error "Error: could not find version in Cargo.toml"
    exit 1
}

Write-Host "Current version: $CurrentVersion"
Write-Host "New version:     $NewVersion"

if ($CurrentVersion -eq $NewVersion) {
    Write-Error "Error: new version is the same as current version"
    exit 1
}

$content = $content -replace "version = `"$CurrentVersion`"", "version = `"$NewVersion`""
Set-Content -Path $CargoToml -Value $content -NoNewline

Push-Location $RepoRoot
try {
    cargo check --workspace 2>&1 | Select-Object -Last 1
    Write-Host "Version bumped successfully."

    git add Cargo.toml
    git commit -m "chore: bump version to v$NewVersion"
    git tag "v$NewVersion"

    Write-Host ""
    Write-Host "Done! Created commit and tag v$NewVersion."
    Write-Host "To publish: git push && git push --tags"
} finally {
    Pop-Location
}
