; Inno Setup script for the Windows installer. CI builds it after `cargo build --release`:
;   iscc /DAppVersion=x.y.z installer\dialectic.iss
; Per-user install (no admin): puts dialectic.exe and dialectic.com on PATH, adds a Start menu
; shortcut, and upgrades in place when a newer installer runs.

#ifndef AppVersion
  #error Pass the version: iscc /DAppVersion=x.y.z installer\dialectic.iss
#endif

[Setup]
; Never change AppId: Windows uses it to recognise upgrades and the uninstaller.
AppId={{0E1B570C-241C-4223-ADB6-ED8EA6B42288}
AppName=Dialectic
AppVersion={#AppVersion}
AppPublisherURL=https://github.com/namzug16/dialectic
DefaultDirName={autopf}\Dialectic
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ChangesEnvironment=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
LicenseFile=..\LICENSE
UninstallDisplayIcon={app}\dialectic.exe
WizardStyle=modern
OutputDir=..\dist
OutputBaseFilename=dialectic-{#AppVersion}-windows-x86_64-setup

[Tasks]
Name: desktopicon; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\dialectic.exe"; DestDir: "{app}"; Flags: ignoreversion
; Terminals pick dialectic.com over dialectic.exe, so the console build serves the CLI.
Source: "..\target\release\dialectic-console.exe"; DestDir: "{app}"; DestName: "dialectic.com"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Dialectic"; Filename: "{app}\dialectic.exe"
Name: "{autodesktop}\Dialectic"; Filename: "{app}\dialectic.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Check: not UserPathContains(ExpandConstant('{app}'))

[Run]
Filename: "{app}\dialectic.exe"; Description: "{cm:LaunchProgram,Dialectic}"; Flags: nowait postinstall skipifsilent

[Code]
function UserPathContains(Directory: string): Boolean;
var
  UserPath: string;
begin
  Result := RegQueryStringValue(HKCU, 'Environment', 'Path', UserPath)
    and (Pos(';' + Uppercase(Directory) + ';', ';' + Uppercase(UserPath) + ';') > 0);
end;

// Inno Setup does not undo a [Registry] edit to an existing value, so take {app} back out of PATH.
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  UserPath, Directory: string;
  Position: Integer;
begin
  if CurUninstallStep <> usPostUninstall then exit;
  if not RegQueryStringValue(HKCU, 'Environment', 'Path', UserPath) then exit;
  Directory := ExpandConstant('{app}');
  UserPath := ';' + UserPath + ';';
  Position := Pos(';' + Uppercase(Directory) + ';', Uppercase(UserPath));
  if Position = 0 then exit;
  Delete(UserPath, Position, Length(Directory) + 1);
  RegWriteExpandStringValue(HKCU, 'Environment', 'Path', Copy(UserPath, 2, Length(UserPath) - 2));
end;
