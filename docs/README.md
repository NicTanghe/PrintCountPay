PrintCountPay documentation

- USAGE.md: build/run instructions and UI logging controls
- WORKSPACE.md: crate layout and responsibilities
- SNMP.md: SNMP client usage and mocking
- WINDOWS_INSTALLER.md: Windows EXE installer build and silent install flow

Build the Windows EXE installer:

```powershell
powershell -ExecutionPolicy Bypass -File packaging\windows\build-installer.ps1
```

Installer output:

```text
dist\windows\installer\PrintCountPay-Setup-<version>.exe
```

For the full packaging flow and silent install flags, see `WINDOWS_INSTALLER.md`.
