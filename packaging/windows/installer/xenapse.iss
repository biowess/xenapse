
#define AppName "Xenapse"
#define AppVersion GetEnv("XENAPSE_VERSION")
#define AppPublisher "Biowess"
#define AppURL "https://github.com/biowess/xenapse"
#define ArchiveName "xenapse-" + AppVersion + "-windows-x86_64"
#define StageDir "..\..\..\dist\" + ArchiveName
[Setup]
AppId={{B8E4D2A1-7C63-4F90-A512-9E6D3B8C2F41}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
AppUpdatesURL={#AppURL}

DefaultDirName={localappdata}\Programs\Xenapse
DefaultGroupName=Xenapse
DisableProgramGroupPage=yes
UsePreviousAppDir=yes
UsePreviousTasks=yes

PrivilegesRequired=lowest
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64

SetupIconFile=..\..\..\assets\icons\xenapse.ico
UninstallDisplayIcon={app}\xenapse.ico

LicenseFile={#StageDir}\LICENSE

OutputDir=..\..\..\dist
OutputBaseFilename=xenapse-{#AppVersion}-windows-x86_64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

Uninstallable=yes
CloseApplications=no

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"
Name: "fileassoc"; Description: "Associate Xenapse with .xnp and .xcss files"; GroupDescription: "File associations:"; Flags: unchecked

[Files]
Source: "{#StageDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autodesktop}\Xenapse"; Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoExit -NoProfile -ExecutionPolicy Bypass -File ""{app}\welcome.ps1"""; WorkingDir: "{app}"; IconFilename: "{app}\xenapse.ico"; Tasks: desktopicon

[Run]
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\integration.ps1"" -Action Install -InstallDir ""{app}"""; Flags: runhidden waituntilterminated; StatusMsg: "Registering Xenapse in your user environment..."

Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\packaging\windows\install-filetypes.ps1"""; Flags: runhidden waituntilterminated; StatusMsg: "Registering Xenapse file types..."; Check: WizardIsTaskSelected('fileassoc')

Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\packaging\windows\uninstall-filetypes.ps1"""; Flags: runhidden waituntilterminated; StatusMsg: "Removing previous Xenapse file associations..."; Check: not WizardIsTaskSelected('fileassoc')

[UninstallRun]
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\integration.ps1"" -Action Uninstall -InstallDir ""{app}"""; Flags: runhidden waituntilterminated; RunOnceId: "XenapseIntegration"

Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\packaging\windows\uninstall-filetypes.ps1"""; Flags: runhidden waituntilterminated; RunOnceId: "XenapseFileTypes"
