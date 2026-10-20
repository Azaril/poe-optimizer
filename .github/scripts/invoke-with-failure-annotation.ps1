# Stream a native command unchanged, preserving its exit code. On failure, expose
# bounded early diagnostics and a tail in the public check-run annotations.
# Large --no-fail-fast batches can finish thousands of lines after the first
# failure; the final tail alone then contains only successful tests and totals.
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateNotNullOrEmpty()]
    [string] $FilePath,

    [Parameter(Mandatory)]
    [string[]] $ArgumentList,

    # Dispatchers can run independent checks before returning an aggregate
    # failure. Existing callers retain the original process-exit behavior.
    [switch] $ReturnExitCode
)

$ErrorActionPreference = 'Stop'
# A nonzero native exit must reach the annotation and exit below, even when the
# caller enables PowerShell's native-command error preference.
$PSNativeCommandUseErrorActionPreference = $false
$maximumLines = 120
$maximumCharacters = 16000
$tail = [System.Collections.Generic.Queue[string]]::new()
$tailCharacters = 0
$diagnostics = [System.Collections.Generic.List[string]]::new()
$diagnosticCharacters = 0
$maximumDiagnosticCharacters = 12000
$maximumDiagnosticLineCharacters = 1500
$maximumDiagnosticWindows = 8
$diagnosticWindowCount = 0
$diagnosticRemainingLines = 0
$diagnosticsTruncated = $false
$lastTestTarget = ''

& $FilePath @ArgumentList 2>&1 | ForEach-Object {
    $line = $_.ToString()
    Write-Host $line
    if ($line -cmatch '^\s*Running ') {
        $lastTestTarget = $line.Substring(0, [Math]::Min($line.Length, $maximumDiagnosticLineCharacters))
    }
    $diagnosticStart = $line -cmatch '^(thread .+ panicked at|error(?:\[[^\]]+\])?:|fatal:)'
    $diagnosticPrefix = @()
    if ($diagnosticStart -and $diagnosticRemainingLines -eq 0) {
        if ($diagnosticWindowCount -lt $maximumDiagnosticWindows -and -not $diagnosticsTruncated) {
            $diagnosticWindowCount++
            $diagnosticRemainingLines = 12
            $diagnosticPrefix = @("", "Failure context $diagnosticWindowCount")
            if ($lastTestTarget) {
                $diagnosticPrefix += $lastTestTarget
            }
        } else {
            $diagnosticsTruncated = $true
        }
    }
    if ($diagnosticRemainingLines -gt 0) {
        $diagnosticLine = $line
        if ($diagnosticLine.Length -gt $maximumDiagnosticLineCharacters) {
            $diagnosticLine = $diagnosticLine.Substring(0, $maximumDiagnosticLineCharacters - 14) + '...[truncated]'
        }
        foreach ($entry in ($diagnosticPrefix + @($diagnosticLine))) {
            if ($diagnosticCharacters + $entry.Length + 1 -gt $maximumDiagnosticCharacters) {
                $diagnosticsTruncated = $true
                $diagnosticRemainingLines = 0
                break
            }
            $diagnostics.Add($entry)
            $diagnosticCharacters += $entry.Length + 1
        }
        if ($diagnosticRemainingLines -gt 0) {
            $diagnosticRemainingLines--
        }
    }
    if ($line.Length -gt $maximumCharacters) {
        $line = $line.Substring($line.Length - $maximumCharacters)
    }
    $tail.Enqueue($line)
    $tailCharacters += $line.Length
    while ($tail.Count -gt $maximumLines -or
           $tailCharacters + $tail.Count - 1 -gt $maximumCharacters) {
        $tailCharacters -= $tail.Dequeue().Length
    }
}
$commandExitCode = $LASTEXITCODE

function Write-FailureAnnotation {
    param(
        [string] $Title,
        [string] $Label,
        [string] $Body,
        [string] $CommandSummary,
        [int] $CommandExitCode
    )
    # Public check-run messages are truncated at 4096 characters. Keep each
    # decoded body below that bound, including a bounded command/header. Emit
    # the newest chunk first so even the first annotation contains the failure.
    $chunkSize = 3500
    $chunkCount = [Math]::Max(1, [int][Math]::Ceiling($Body.Length / $chunkSize))
    for ($chunkIndex = $chunkCount - 1; $chunkIndex -ge 0; $chunkIndex--) {
        $start = $chunkIndex * $chunkSize
        $length = [Math]::Min($chunkSize, $Body.Length - $start)
        $chunk = $Body.Substring($start, $length)
        $message = "Command exited with code ${CommandExitCode}: $CommandSummary`n" +
            "$Label chunk $($chunkIndex + 1)/$chunkCount (newest first):`n" +
            $chunk
        # Escape percent first so literal strings such as %0A remain literal.
        $message = $message.Replace('%', '%25').Replace("`r", '%0D').Replace("`n", '%0A')
        Write-Host "::error title=${Title}::$message"
    }
}

if ($commandExitCode -ne 0) {
    $commandSummary = "$FilePath $($ArgumentList -join ' ')"
    if ($commandSummary.Length -gt 300) {
        $commandSummary = $commandSummary.Substring(0, 297) + '...'
    }
    if ($diagnostics.Count -gt 0) {
        $label = "Early failure contexts (at most $maximumDiagnosticWindows windows and $maximumDiagnosticCharacters characters"
        if ($diagnosticsTruncated) {
            $label += '; further contexts omitted'
        }
        $label += ')'
        Write-FailureAnnotation -Title 'Command diagnostics' -Label $label `
            -Body ([string]::Join("`n", $diagnostics.ToArray())) `
            -CommandSummary $commandSummary -CommandExitCode $commandExitCode
    }
    Write-FailureAnnotation -Title 'Command failed' `
        -Label "Output tail (last $($tail.Count) lines, at most $maximumCharacters characters)" `
        -Body ([string]::Join("`n", $tail.ToArray())) `
        -CommandSummary $commandSummary -CommandExitCode $commandExitCode
}
if ($ReturnExitCode) {
    return [int] $commandExitCode
}
exit $commandExitCode
