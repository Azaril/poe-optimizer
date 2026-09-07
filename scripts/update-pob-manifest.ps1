#requires -Version 7.2
<#
.SYNOPSIS
Regenerate the pinned PoB source manifest from a clean checkout.
.DESCRIPTION
Uses the revision declared by the evaluator. Optional ExpectedRevision must match
both evaluator constants and the checked-out source. Writes only the tracked
source-manifest JSON; does not fetch, update the submodule, or change Git state.
#>
[CmdletBinding()]
param(
    [ValidatePattern('^[0-9a-f]{40}$')]
    [string]$ExpectedRevision
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$workspaceRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$sourceRoot = Join-Path $workspaceRoot 'vendor/path-of-building-poe2'
$outputPath = Join-Path $workspaceRoot 'crates/poe-optimizer-pob/data/pob-source-manifest.json'
$utf8 = [Text.UTF8Encoding]::new($false, $true)
$absentPaths = @('src/first.run', 'src/installed.cfg', 'src/manifest.xml')

function Get-DeclaredRevision([string]$RelativePath) {
    $text = [IO.File]::ReadAllText((Join-Path $workspaceRoot $RelativePath), $utf8)
    $match = [regex]::Match($text, 'pub const UPSTREAM_REVISION: &str = "([0-9a-f]{40})";')
    if (-not $match.Success) { throw "Cannot read UPSTREAM_REVISION from $RelativePath" }
    return $match.Groups[1].Value
}

function Invoke-SourceGit([string[]]$GitArguments) {
    $output = & git -C $sourceRoot @GitArguments
    if ($LASTEXITCODE -ne 0) { throw "Git source check failed: $($GitArguments -join ' ')" }
    return $output
}

function Assert-SourcePin {
    $actualRevision = Invoke-SourceGit -GitArguments @('rev-parse', 'HEAD')
    if ($actualRevision -ne $ExpectedRevision) {
        throw "PoB is at $actualRevision; expected $ExpectedRevision"
    }
    if (Invoke-SourceGit -GitArguments @('status', '--porcelain', '--untracked-files=all')) {
        throw 'PoB source is dirty. Review or discard those changes before regenerating its manifest.'
    }
    foreach ($relative in $absentPaths) {
        if (Test-Path -LiteralPath (Join-Path $sourceRoot $relative)) {
            throw "Unexpected PoB startup override: $relative"
        }
    }
}

$declaredRevision = Get-DeclaredRevision 'crates/poe-optimizer-pob/src/source.rs'
$runtimeRevision = Get-DeclaredRevision 'crates/poe-optimizer-pob/src/runtime.rs'
if (-not $ExpectedRevision) { $ExpectedRevision = $declaredRevision }
if ($ExpectedRevision -ne $declaredRevision -or $ExpectedRevision -ne $runtimeRevision) {
    throw 'ExpectedRevision must match both evaluator UPSTREAM_REVISION constants.'
}
Assert-SourcePin

$paths = [string[]]@(Invoke-SourceGit -GitArguments @('ls-files', '--', 'src', 'runtime/lua', 'manifest.xml') |
    Where-Object { $_ -eq 'manifest.xml' -or $_.EndsWith('.lua', [StringComparison]::Ordinal) })
[Array]::Sort($paths, [StringComparer]::Ordinal)
if ($paths.Length -eq 0 -or $paths.Length -gt 4096 -or 'manifest.xml' -notin $paths) {
    throw 'Unexpected source inventory; review verifier limits and scope before regenerating.'
}
$knownPaths = [Collections.Generic.HashSet[string]]::new($paths, [StringComparer]::Ordinal)
foreach ($luaRoot in @('src', 'runtime/lua')) {
    foreach ($file in Get-ChildItem -LiteralPath (Join-Path $sourceRoot $luaRoot) -Recurse -File -Filter '*.lua') {
        $relative = [IO.Path]::GetRelativePath($sourceRoot, $file.FullName).Replace('\', '/')
        if (-not $knownPaths.Contains($relative)) {
            throw "Untracked or ignored Lua source can shadow a pinned module: $relative"
        }
    }
}

$entries = foreach ($relative in $paths) {
    if ($relative.Contains('\') -or $relative.Contains(':') -or
        @($relative.Split('/') | Where-Object { $_ -in @('', '.', '..') }).Count) {
        throw "Nonportable source path: $relative"
    }
    $path = Join-Path $sourceRoot $relative
    $info = Get-Item -LiteralPath $path
    if ($info.PSIsContainer -or $info.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint)) {
        throw "Source must be an ordinary file: $relative"
    }
    if ($info.Length -gt 32MB) { throw "Source file exceeds read limit: $relative" }
    $text = $utf8.GetString([IO.File]::ReadAllBytes($path)).Replace("`r`n", "`n")
    $bytes = $utf8.GetBytes($text)
    if ($bytes.Length -gt 16MB) { throw "Normalized source exceeds verifier limit: $relative" }
    [ordered]@{
        path = $relative
        bytes = $bytes.Length
        sha256 = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()
    }
}
Assert-SourcePin

$manifest = [ordered]@{
    schema_version = 1
    upstream_revision = $ExpectedRevision
    normalization = 'utf8_crlf_to_lf'
    lua_roots = @('src', 'runtime/lua')
    absent_paths = $absentPaths
    files = @($entries)
} | ConvertTo-Json -Depth 5
if ((Get-Item -LiteralPath $outputPath).Attributes.HasFlag([IO.FileAttributes]::ReparsePoint)) {
    throw 'Refusing to overwrite a linked source manifest.'
}
[IO.File]::WriteAllText($outputPath, $manifest.Replace("`r`n", "`n") + "`n", $utf8)
$manifestHash = [Convert]::ToHexString(
    [Security.Cryptography.SHA256]::HashData([IO.File]::ReadAllBytes($outputPath))
).ToLowerInvariant()
Write-Output "Wrote $($entries.Count) files at $ExpectedRevision"
Write-Output "Manifest SHA-256: $manifestHash"
