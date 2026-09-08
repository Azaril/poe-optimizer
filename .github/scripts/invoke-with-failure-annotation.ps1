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
    $message = "Command exited with code ${commandExitCode}: $FilePath $($ArgumentList -join ' ')`n" +
        "Last $($tail.Count) output lines (at most $maximumCharacters characters):`n" +
        [string]::Join("`n", $tail.ToArray())
    # Escape percent first so literal strings such as %0A remain literal.
    $message = $message.Replace('%', '%25').Replace("`r", '%0D').Replace("`n", '%0A')
    Write-Host "::error title=Test command failed::$message"
}
exit $commandExitCode
