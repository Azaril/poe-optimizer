#requires -Version 7.2
<#
.SYNOPSIS
Generate independent Windows calibration outputs using PoB's bundled Lua DLLs.
.DESCRIPTION
Does not execute the Rust evaluator. Compiles a separate C driver and runs the
independent Lua bootstrap/extractor. Outputs remain local until reviewed/copied.
#>
[CmdletBinding()]
param([string]$OutputDirectory)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'The bundled-DLL reference generator requires Windows x64.' }
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$pob = Join-Path $workspace 'vendor/path-of-building-poe2'
$pin = '3887ae68a6a6b8bb7b41d1b61998f1aa184201e4'
if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $workspace ("local/reference-calibration/run-" + [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss-fff'))
}
$output = [IO.Path]::GetFullPath($OutputDirectory)
if (($workspace + $output).ToCharArray() | Where-Object { [int]$_ -gt 127 }) {
    throw 'This small native reference harness currently requires ASCII paths.'
}
[IO.Directory]::CreateDirectory($output) | Out-Null
$utf8 = [Text.UTF8Encoding]::new($false, $true)

function Assert-Pin {
    $actual = & git -C $pob rev-parse HEAD
    if ($LASTEXITCODE -ne 0 -or $actual -ne $pin) { throw 'Unexpected PoB reference revision.' }
    $dirty = & git -C $pob status --porcelain --untracked-files=all
    if ($LASTEXITCODE -ne 0 -or $dirty) { throw 'PoB reference source must be clean.' }
}
function Hash-Bytes([string]$Path) {
    return [Convert]::ToHexString(
        [Security.Cryptography.SHA256]::HashData([IO.File]::ReadAllBytes($Path))
    ).ToLowerInvariant()
}
function Hash-TextLf([string]$Path) {
    $bytes = $utf8.GetBytes([IO.File]::ReadAllText($Path, $utf8).Replace("`r`n", "`n"))
    return [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()
}
function Run-Reference([string]$Harness, [string]$Scratch, [string]$InputFile, [string]$JsonFile, [string]$LogName) {
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $driver
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    foreach ($argument in @($luaDll, $Harness, (Join-Path $pob 'src'), $Scratch, $InputFile, $JsonFile)) {
        $start.ArgumentList.Add($argument)
    }
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $start
    try {
        [void]$process.Start()
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        if (-not $process.WaitForExit(45000)) {
            $process.Kill($true)
            $process.WaitForExit()
            throw 'Independent reference exceeded its 45-second process budget.'
        }
        [IO.File]::WriteAllText((Join-Path $output "$LogName.stdout.log"), $stdout.GetAwaiter().GetResult(), $utf8)
        [IO.File]::WriteAllText((Join-Path $output "$LogName.stderr.log"), $stderr.GetAwaiter().GetResult(), $utf8)
        if ($process.ExitCode -ne 0) { throw "Independent reference failed; inspect $LogName.stderr.log" }
    } finally { $process.Dispose() }
}
Assert-Pin
$vswhere = Join-Path ([Environment]::GetEnvironmentVariable('ProgramFiles(x86)')) 'Microsoft Visual Studio/Installer/vswhere.exe'
$installation = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if ($LASTEXITCODE -ne 0 -or -not $installation) { throw 'MSVC x64 build tools were not found.' }
$developerCommand = Join-Path $installation 'Common7/Tools/VsDevCmd.bat'
$compilerEnvironment = & cmd.exe /d /s /c "`"$developerCommand`" -no_logo -arch=x64 -host_arch=x64 >nul && set"
if ($LASTEXITCODE -ne 0) { throw 'MSVC environment setup failed.' }
foreach ($line in $compilerEnvironment) {
    if ($line -match '^(PATH|INCLUDE|LIB|LIBPATH)=(.*)$') {
        [Environment]::SetEnvironmentVariable($Matches[1], $Matches[2], 'Process')
    }
}
$driverSource = Join-Path $PSScriptRoot 'reference-driver.c'
$harness = Join-Path $PSScriptRoot 'reference-pob.lua'
$driver = Join-Path $output 'reference-driver.exe'
& cl.exe /nologo /W3 /O2 "/Fo:$output/reference-driver.obj" "/Fe:$driver" $driverSource
if ($LASTEXITCODE -ne 0) { throw 'Independent C driver compilation failed.' }
$luaDll = Join-Path $pob 'runtime/lua51.dll'
$utf8Dll = Join-Path $pob 'runtime/lua-utf8.dll'
$sourceManifest = Join-Path $workspace 'crates/poe-optimizer-pob/data/pob-source-manifest.json'
$compiler = (Get-Item -LiteralPath (Get-Command cl.exe).Source).VersionInfo.FileVersion
foreach ($name in @('spark-mapping', 'spark-bossing')) {
    $inputFile = Join-Path $workspace "tests/fixtures/calibration/$name.xml"
    $resultPath = Join-Path $output "$name.reference.json"
    $scratch = Join-Path $output ("scratch-" + $name + "-" + [Guid]::NewGuid().ToString('N'))
    [IO.Directory]::CreateDirectory((Join-Path $scratch 'Builds')) | Out-Null
    [IO.Directory]::CreateDirectory((Join-Path $scratch 'Path of Building (PoE2)/Builds')) | Out-Null
    Run-Reference -Harness $harness -Scratch $scratch -InputFile $inputFile -JsonFile $resultPath -LogName $name
    $report = [IO.File]::ReadAllText($resultPath, $utf8) | ConvertFrom-Json -AsHashtable
    $report['provenance'] = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
        upstream_revision = $pin
        source_manifest_sha256_lf = Hash-TextLf $sourceManifest
        fixture_sha256 = Hash-Bytes $inputFile
        lua51_dll_sha256 = Hash-Bytes $luaDll
        lua_utf8_dll_sha256 = Hash-Bytes $utf8Dll
        driver_source_sha256_lf = Hash-TextLf $driverSource
        driver_executable_sha256 = Hash-Bytes $driver
        harness_sha256_lf = Hash-TextLf $harness
        generator_sha256_lf = Hash-TextLf $PSCommandPath
        compiler = "MSVC $compiler /O2 x64"
        operating_system = [Environment]::OSVersion.VersionString
    }
    $report['comparison'] = [ordered]@{
        absolute_tolerance = 0.00000001
        relative_tolerance = 0.000000001
        scope = 'Independent host and extractor parity for shared PoB calculations; not independent game-mechanics certification.'
    }
    [IO.File]::WriteAllText($resultPath, ($report | ConvertTo-Json -Depth 8).Replace("`r`n", "`n") + "`n", $utf8)
    Write-Output $resultPath
}
Assert-Pin
Write-Output 'Independent values generated. Review numeric and provenance differences before replacing committed reference files.'