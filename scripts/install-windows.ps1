param(
    [string] $BinaryPath,
    [string] $Downloads
)

$ErrorActionPreference = 'Stop'
if (-not $BinaryPath) {
    $releaseBinary = Join-Path $PSScriptRoot '..\getclean.exe'
    if (Test-Path -LiteralPath $releaseBinary) {
        $BinaryPath = $releaseBinary
    } else {
        $BinaryPath = Join-Path $PSScriptRoot '..\target\release\getclean.exe'
    }
}
$binary = (Resolve-Path -LiteralPath $BinaryPath).Path
$installDir = Join-Path $env:LOCALAPPDATA 'getClean'
$installedBinary = Join-Path $installDir 'getclean.exe'
New-Item -ItemType Directory -Path $installDir -Force | Out-Null
if (Get-ScheduledTask -TaskName 'getClean' -ErrorAction SilentlyContinue) {
    Stop-ScheduledTask -TaskName 'getClean' -ErrorAction SilentlyContinue
}
if ($binary -ne $installedBinary) {
    Copy-Item -LiteralPath $binary -Destination $installedBinary -Force
}
if ($Downloads) {
    $folder = (Resolve-Path -LiteralPath $Downloads).Path
    $action = New-ScheduledTaskAction -Execute $installedBinary -Argument ('--downloads "{0}"' -f $folder)
} else {
    $action = New-ScheduledTaskAction -Execute $installedBinary
}
$user = [Security.Principal.WindowsIdentity]::GetCurrent().Name
$principal = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Limited
$triggers = @(
    (New-ScheduledTaskTrigger -AtLogOn -User $user),
    (New-ScheduledTaskTrigger -Daily -At '03:00')
)
$settings = New-ScheduledTaskSettingsSet `
    -ExecutionTimeLimit (New-TimeSpan -Seconds 0) `
    -RestartCount 255 `
    -RestartInterval (New-TimeSpan -Minutes 1) `
    -MultipleInstances IgnoreNew `
    -StartWhenAvailable `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries

Register-ScheduledTask -TaskName 'getClean' -Action $action -Trigger $triggers `
    -Principal $principal -Settings $settings -Description 'Organizes Downloads continuously' -Force | Out-Null
Start-ScheduledTask -TaskName 'getClean'
Write-Host 'getClean installed as a logon task.'
