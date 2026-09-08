# Stream a native command unchanged, preserving its exit code. On failure, expose
# a bounded tail in the public check-run annotation as well as the full job log.
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateNotNullOrEmpty()]
    [string] $FilePath,

    [Parameter(Mandatory)]
    [string[]] $ArgumentList
)

$ErrorActionPreference = 'Stop'
# A nonzero native exit must reach the annotation and exit below, even when the
# caller enables PowerShell's native-command error preference.
$PSNativeCommandUseErrorActionPreference = $false
$maximumLines = 120
$maximumCharacters = 16000
$tail = [System.Collections.Generic.Queue[string]]::new()
$tailCharacters = 0

& $FilePath @ArgumentList 2>&1 | ForEach-Object {
    $line = $_.ToString()
    Write-Host $line
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

if ($commandExitCode -ne 0) {
    $outputTail = [string]::Join("`n", $tail.ToArray())
    $commandSummary = "$FilePath $($ArgumentList -join ' ')"
    if ($commandSummary.Length -gt 300) {
        $commandSummary = $commandSummary.Substring(0, 297) + '...'
    }
    # Public check-run messages are truncated at 4096 characters. Keep each
    # decoded body below that bound, including a bounded command/header. Emit
    # the newest chunk first so even the first annotation contains the failure.
    $chunkSize = 3500
    $chunkCount = [Math]::Max(1, [int][Math]::Ceiling($outputTail.Length / $chunkSize))
    for ($chunkIndex = $chunkCount - 1; $chunkIndex -ge 0; $chunkIndex--) {
        $start = $chunkIndex * $chunkSize
        $length = [Math]::Min($chunkSize, $outputTail.Length - $start)
        $body = $outputTail.Substring($start, $length)
        $message = "Command exited with code ${commandExitCode}: $commandSummary`n" +
            "Output tail chunk $($chunkIndex + 1)/$chunkCount (newest first; last $($tail.Count) lines, at most $maximumCharacters characters):`n" +
            $body
        # Escape percent first so literal strings such as %0A remain literal.
        $message = $message.Replace('%', '%25').Replace("`r", '%0D').Replace("`n", '%0A')
        Write-Host "::error title=Test command failed::$message"
    }
}
exit $commandExitCode
