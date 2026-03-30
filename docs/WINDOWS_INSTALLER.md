Windows installer

Build the installer from the workspace root with:

```powershell
powershell -ExecutionPolicy Bypass -File packaging\windows\build-installer.ps1
```

What the script does:

- builds `printcountpay-app` in release mode
- stages `PrintCountPay.exe` and the bundled `profiles\...` files under `dist\windows\staging`
- compiles `packaging\windows\PrintCountPay.iss` with Inno Setup 6

Requirements:

- Rust toolchain for building the app
- Inno Setup 6 (`ISCC.exe`) installed, or pass `-InnoSetupCompiler <path>`

Installer output:

- `dist\windows\installer\PrintCountPay-Setup-<version>.exe`

Silent install:

```powershell
.\PrintCountPay-Setup-<version>.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-
```

Useful script options:

```powershell
powershell -ExecutionPolicy Bypass -File packaging\windows\build-installer.ps1 -SkipBuild
powershell -ExecutionPolicy Bypass -File packaging\windows\build-installer.ps1 -SkipCompile
```

Installer layout:

- application files go to `%ProgramFiles%\PrintCountPay`
- runtime data goes to `%APPDATA%\PrintCountPay`

On first run, the app seeds `%APPDATA%\PrintCountPay\profiles` from the bundled installer files without overwriting existing profile files.
