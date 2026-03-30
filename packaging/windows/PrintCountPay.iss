#define MyAppName "PrintCountPay"
#define MyAppPublisher "PrintCountPay"
#define MyAppExeName "PrintCountPay.exe"
#define MyAppDataDir "PrintCountPay"
#define MyAppId "{{A1B8246D-5B95-4A09-B6B4-5E16A654B04E}}"
#define StageRoot "..\..\dist\windows\staging"
#define OutputRoot "..\..\dist\windows\installer"

#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif

[Setup]
AppId={#MyAppId}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
Compression=lzma
SolidCompression=yes
WizardStyle=modern
OutputDir={#OutputRoot}
#ifdef MyOutputBaseFilename
OutputBaseFilename={#MyOutputBaseFilename}
#else
OutputBaseFilename=PrintCountPay-Setup-{#MyAppVersion}
#endif
UninstallDisplayIcon={app}\{#MyAppExeName}
CloseApplications=yes
RestartApplications=no
SetupLogging=yes
UsedUserAreasWarning=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional icons:"; Flags: unchecked

[Dirs]
Name: "{userappdata}\{#MyAppDataDir}"

[Files]
Source: "{#StageRoot}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{userappdata}\{#MyAppDataDir}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{userappdata}\{#MyAppDataDir}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch {#MyAppName}"; WorkingDir: "{userappdata}\{#MyAppDataDir}"; Flags: nowait postinstall skipifsilent
