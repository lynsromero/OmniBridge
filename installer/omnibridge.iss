[Setup]
AppId={{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}
AppName=OmniBridge
AppVersion=0.1.0
AppPublisher=Lyns Romero
AppPublisherURL=https://github.com/lynsromero/OmniBridge
DefaultDirName={autopf}\OmniBridge
DefaultGroupName=OmniBridge
OutputDir=installer
OutputBaseFilename=OmniBridge-Setup-0.1.0
Compression=lzma2/ultra64
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\omnibridge.exe
RestartApplications=no
CloseApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "..\target\release\omnibridge.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\*.dll"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\OmniBridge"; Filename: "{app}\omnibridge.exe"
Name: "{group}\Uninstall OmniBridge"; Filename: "{uninstallexe}"
Name: "{userdesktop}\OmniBridge"; Filename: "{app}\omnibridge.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop icon"; GroupDescription: "Additional icons:"
Name: "startup"; Description: "Start OmniBridge with Windows"; GroupDescription: "Startup:"; Flags: unchecked

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "OmniBridge"; ValueData: """{app}\omnibridge.exe"" gui"; Flags: uninsdeletevalue; Tasks: startup

[Run]
Filename: "{app}\omnibridge.exe"; Parameters: "gui"; Description: "Launch OmniBridge now"; Flags: nowait postinstall skipifsilent
