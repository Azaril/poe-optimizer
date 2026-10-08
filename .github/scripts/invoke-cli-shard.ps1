# Keep target selection explicit: --all-targets here would rerun the full suite.
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateRange(0, 3)]
    [int] $Shard,

    [switch] $PlanOnly,

    # Injectable metadata/executable let ordinary Rust tests verify planning and
    # failure orchestration without recursively compiling or running Cargo.
    [string] $MetadataPath,
    [string] $Cargo = 'cargo'
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$shardCount = 4
if ($MetadataPath) {
    $metadata = Get-Content -LiteralPath $MetadataPath -Raw | ConvertFrom-Json
} else {
    $metadataJson = & $Cargo metadata --no-deps --format-version 1 --locked
    if ($LASTEXITCODE -ne 0) { throw "Cargo metadata exited with code $LASTEXITCODE" }
    $metadata = ($metadataJson -join "`n") | ConvertFrom-Json
}
$packages = @($metadata.packages | Where-Object {
    $_.name -eq 'poe-optimizer-cli' -and $_.id -in $metadata.workspace_members
})
if ($packages.Count -ne 1) { throw 'Expected exactly one workspace CLI package' }
$package = $packages[0]
$targets = @($package.targets)
if ($targets.Count -eq 0) { throw 'CLI metadata contains no targets' }
$byKey = [System.Collections.Generic.Dictionary[string, object]]::new([StringComparer]::Ordinal)
$integrationNames = [System.Collections.Generic.List[string]]::new()
$buildDependencies = [System.Collections.Generic.List[object]]::new()
foreach ($target in $targets) {
    $kinds = @($target.kind)
    # Cargo runs the package's build script as a dependency of selected targets;
    # custom-build is never a test selector. Preserve its separate census.
    if ($kinds.Count -eq 1 -and $kinds[0] -eq 'custom-build') {
        if ($buildDependencies.Count -ne 0) { throw 'Multiple CLI custom-build targets' }
        if ($target.name -ne 'build-script-build' -or
            @($target.crate_types).Count -ne 1 -or $target.crate_types[0] -ne 'bin' -or
            $target.test -isnot [bool] -or $target.test -or
            $target.doc -isnot [bool] -or $target.doc -or
            $target.doctest -isnot [bool] -or $target.doctest -or
            [string]::IsNullOrWhiteSpace($target.src_path) -or
            @($target.'required-features').Where({ $null -ne $_ }).Count -ne 0) {
            throw 'Malformed CLI custom-build target'
        }
        $buildDependencies.Add($target)
        continue
    }
    # Adding another test target kind requires an explicit selector policy,
    # never silent omission.
    if ($kinds.Count -ne 1 -or $kinds[0] -notin @('bin', 'test')) {
        throw "Unsupported CLI target kind: $($target.name) [$($kinds -join ',')]"
    }
    if ($target.test -isnot [bool] -or -not $target.test) {
        throw "Unsupported CLI target test setting: $($target.name)"
    }
    if (@($target.'required-features').Where({ $null -ne $_ }).Count -ne 0) {
        throw "Unsupported CLI target required-features: $($target.name)"
    }
    if ([string]::IsNullOrWhiteSpace($target.name)) { throw 'CLI target has no name' }
    $identity = "$($kinds[0]):$($target.name)"
    if ($byKey.ContainsKey($identity)) { throw "Duplicate CLI target: $identity" }
    $byKey.Add($identity, $target)
    if ($kinds[0] -eq 'test') { $integrationNames.Add($target.name) }
}
$integrationNames.Sort([StringComparer]::Ordinal)
if ($integrationNames.Count -lt $shardCount) { throw 'Every CLI shard must contain an integration target' }

$assignments = [System.Collections.Generic.List[object]]::new()
$binaryNames = [System.Collections.Generic.List[string]]::new()
foreach ($target in $targets) {
    if ($target.kind[0] -eq 'bin') { $binaryNames.Add($target.name) }
}
$binaryNames.Sort([StringComparer]::Ordinal)
foreach ($name in $binaryNames) {
    $assignments.Add([pscustomobject]@{ kind = 'bin'; name = $name; shard = 0; selector = '--bin' })
}
for ($index = 0; $index -lt $integrationNames.Count; $index++) {
    $assignments.Add([pscustomobject]@{
        kind = 'test'; name = $integrationNames[$index]; shard = $index % $shardCount; selector = '--test'
    })
}
$assignedKeys = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach ($target in $assignments) {
    $identity = "$($target.kind):$($target.name)"
    if (-not $byKey.ContainsKey($identity) -or -not $assignedKeys.Add($identity)) {
        throw "CLI partition is not disjoint: $identity"
    }
}
if ($assignedKeys.Count -ne $byKey.Count) { throw 'CLI partition omitted metadata targets' }
if ($assignedKeys.Count + $buildDependencies.Count -ne $targets.Count) {
    throw 'CLI partition and build dependencies omitted metadata targets'
}
$selected = @($assignments | Where-Object { $_.shard -eq $Shard })
$selectors = @($selected | ForEach-Object { $_.selector; $_.name })
$profiles = @(
    [pscustomobject]@{ name = 'all-features'; flag = '--all-features' },
    [pscustomobject]@{ name = 'native-only'; flag = '--no-default-features' }
)
$commands = @($profiles | ForEach-Object {
    $arguments = @('test', '-p', $package.name, $_.flag, '--locked', '--no-fail-fast') + $selectors
    if (($arguments -join ' ').Length -gt 24000) { throw 'CLI shard exceeds the bounded command length' }
    [pscustomobject]@{ profile = $_.name; arguments = $arguments }
})
$plan = [pscustomobject]@{
    schema_version = 1; package = $package.name; shard_count = $shardCount; shard = $Shard
    targets = @($assignments); build_dependencies = @($buildDependencies)
    selected_targets = $selected; commands = $commands
}
if ($PlanOnly) {
    $plan | ConvertTo-Json -Depth 8
    exit 0
}
Write-Host "CLI shard $Shard/${shardCount}: $($selected.Count) of $($assignments.Count) targets"
foreach ($dependency in $buildDependencies) {
    Write-Host "  Cargo-managed build dependency: $($dependency.name) [$($dependency.src_path)]"
}
foreach ($target in $selected) { Write-Host "  $($target.kind):$($target.name)" }
$firstFailure = 0
foreach ($command in $commands) {
    $elapsed = [System.Diagnostics.Stopwatch]::StartNew()
    Write-Host "Starting CLI profile $($command.profile)"
    try {
        $code = & (Join-Path $PSScriptRoot 'invoke-with-failure-annotation.ps1') `
            -FilePath $Cargo -ArgumentList $command.arguments -ReturnExitCode
        if ($code -isnot [int]) { throw 'Annotation wrapper did not return one exit code' }
    } catch {
        Write-Host "::error title=CLI invocation failed::$($_.Exception.Message.Replace('%', '%25').Replace("`r", '%0D').Replace("`n", '%0A'))"
        $code = 1
    }
    Write-Host "CLI profile $($command.profile) exited $code after $($elapsed.Elapsed.TotalSeconds.ToString('F2', [Globalization.CultureInfo]::InvariantCulture))s"
    if ($code -ne 0 -and $firstFailure -eq 0) { $firstFailure = $code }
}
exit $firstFailure
