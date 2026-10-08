[Setup]
AppId={{019EB36D-2D44-7A10-A0D3-1DA29AA7865C}}
AppName=weshtatistic
AppVersion={#AppVersion}
AppPublisher=weshford
AppPublisherURL=https://github.com/weshford/weshtatistic
AppSupportURL=https://github.com/weshford/weshtatistic/issues
AppUpdatesURL=https://github.com/weshford/weshtatistic/releases
DefaultDirName={autopf}\weshtatistic
DefaultGroupName=weshtatistic
DisableProgramGroupPage=yes
LicenseFile=LICENSE
; Output directory and name
OutputDir=staging
OutputBaseFilename=weshtatistic-setup-x86_64
SetupIconFile=assets\img\icon.ico
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "target\release\weshtatistic.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\weshtatistic"; Filename: "{app}\weshtatistic.exe"; IconFilename: "{app}\weshtatistic.exe"
Name: "{autodesktop}\weshtatistic"; Filename: "{app}\weshtatistic.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\weshtatistic.exe"; Description: "{cm:LaunchProgram,weshtatistic}"; Flags: nowait postinstall skipifsilent
