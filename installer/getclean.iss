#ifndef MyAppVersion
  #define MyAppVersion "0.2.1"
#endif

[Setup]
AppId=gwennocas.getClean
AppName=getClean
AppVersion={#MyAppVersion}
AppPublisher=Gwen
AppPublisherURL=https://github.com/gwennocas/getClean
AppSupportURL=https://github.com/gwennocas/getClean/issues
AppUpdatesURL=https://github.com/gwennocas/getClean/releases
DefaultDirName={localappdata}\getClean
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..
OutputBaseFilename=getclean-v{#MyAppVersion}-windows-x86_64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\getclean.exe
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[Files]
Source: "..\target\release\getclean.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\scripts\install-windows.ps1"; DestDir: "{app}\scripts"; Flags: ignoreversion
Source: "..\scripts\uninstall-windows.ps1"; DestDir: "{app}\scripts"; Flags: ignoreversion

[Run]
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File ""{app}\scripts\install-windows.ps1"" -BinaryPath ""{app}\getclean.exe"""; Flags: runhidden logoutput; StatusMsg: "Configurando inicialização automática..."

[UninstallRun]
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File ""{app}\scripts\uninstall-windows.ps1"""; Flags: runhidden logoutput

[Code]
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
begin
  { Stop an older scheduled instance before Setup replaces the executable. }
  Exec(ExpandConstant('{sys}\schtasks.exe'), '/End /TN getClean', '',
    SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Result := '';
end;
