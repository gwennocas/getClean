param([switch] $RemoveBinary)

$ErrorActionPreference = 'Stop'
$installedBinary = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'getClean\getclean.exe'))
$task = Get-ScheduledTask -TaskName 'getClean' -ErrorAction SilentlyContinue
if ($task) {
    $taskBinary = $task.Actions.Execute.Trim('"')
    if ([IO.Path]::GetFullPath($taskBinary) -ine $installedBinary) {
        Write-Host 'The getClean task points to a different executable; leaving it in place.'
        exit 0
    }
    Stop-ScheduledTask -TaskName 'getClean' -ErrorAction SilentlyContinue
    Unregister-ScheduledTask -TaskName 'getClean' -Confirm:$false
    Write-Host 'getClean scheduled task removed.'
}

if ($RemoveBinary -and (Test-Path -LiteralPath $installedBinary)) {
    Remove-Item -LiteralPath $installedBinary -Force
    Write-Host 'getClean executable removed.'
}
