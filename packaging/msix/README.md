# MSIX Packaging

This folder contains a minimal MSIX packaging skeleton for Big Screen Launcher.

What it provides:

- A manifest template with `runFullTrust` and the `desktop:StartupTask` declaration.
- A PowerShell script that stages `big-screen-launcher.exe`, generates the package logo assets from `assets/app-store-logo-1080.png`, and writes `AppxManifest.xml`.
- An optional `makeappx.exe` step for producing a `.msix` file when the Windows SDK is installed.

Important manifest contract:

- The startup task `TaskId` is fixed to `BigScreenLauncherStartup`.
- That must stay aligned with the runtime code in `src/system/startup.rs`.

Before you run the script:

1. Reserve the app name in Microsoft Partner Center.
2. Collect the exact package identity values from the Store association flow.
3. Use the Store-provided package identity name and publisher subject in the script arguments.

Typical usage:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File ".\packaging\msix\build-msix.ps1" -IdentityName "undermoonn.BigScreenLauncher" -Publisher "CN=A18D8212-E6DF-4C8E-9912-7CDD880DD621" -PublisherDisplayName "undermoonn" -Version "1.0.0.0" -Pack
```

Notes:

- By default, the script runs `cargo build --release` first.
- If you omit `-Version`, the script reads the Cargo version and normalizes it to the 4-part MSIX format, for example `0.2.4` becomes `0.2.4.0`.
- Use `-SkipBuild` if you already built `target/release/big-screen-launcher.exe`.
- If `makeappx.exe` is not on the machine, omit `-Pack` to generate only the package layout.
- For local sideload testing, you still need a signing step after `.msix` creation.
- If you submit to Microsoft Store, keep any required non-integrated software disclosure in the first two lines of the Store description. For the current package, disclose the Microsoft Visual C++ Redistributable dependency there as well.