# McMurryPDF Windows thumbnail provider

Original McMurry Software shell integration, licensed MIT OR Apache-2.0 (see the repository LICENSE-MIT and LICENSE-APACHE files). This standalone Windows COM adapter is outside the Rust PDF crates. It contains no Adobe code, assets or dependencies. PDF rendering uses the stable McMurryPDF-cli.exe beside the DLL.

The provider implements IInitializeWithStream and IThumbnailProvider. Windows receives a first-page HBITMAP with preserved page aspect ratio; Explorer adds the associated app icon. Documents remain unchanged. Temporary inputs and PAM rasters are deleted after rendering. The DLL does not perform network operations or invoke a command shell.

Limits: 512 MiB document stream, 512 MiB child process memory, one rendering process per host, 12 seconds renderer timeout, requested thumbnail size up to 1024 pixels, first-page raster at 96 dpi. Encrypted, malformed, oversized or slow files return a failure so Explorer can show a file icon. Stream copying is size bounded; the timeout applies to rendering. This is an internal unsigned build, not yet an office-wide installer.

Build: run Build.ps1. Copy McMurryPDFThumbnail.dll to the stable McMurryPDF folder. MSVC builds x64 with the static runtime, warnings as errors and Windows mitigation linker flags. The existing GUI and CLI need no rebuild.

Install: run Install-Admin.ps1 -AppDirectory <installed-app-folder> in administrator PowerShell as the same office user. Machine-wide registration is limited to our unique COM class and its standard surrogate AppID; PDF handler selection remains per user and default UserChoice is preserved. Process isolation stays enabled. Register.ps1 alone provides per-user registration; direct COM tests succeed, but Windows Shell activation on this machine returned REGDB_E_CLASSNOTREG until machine-wide installation can be verified.

Verification: Test.ps1 directly invokes our provider on document streams. Test-Shell.ps1 makes new copies outside the source tree in the user's temporary folder and asks Windows for thumbnails with icon fallback disabled. These fresh copies defeat preexisting thumbnail cache entries. Diagnostic private documents and rendered images must never be committed or published. Direct COM tests passed for both user reports at 197 x 256 pixels. The isolated Shell test is pending administrator installation.

Update only when the provider DLL is no longer loaded in the Windows surrogate. Keep the GUI, CLI and DLL at their stable names. Do not rename production executables or alter default UserChoice hashes. Registry values before registration are backed up in McMurryPDF/association-backup.

References: Microsoft Windows SDK propsys.h and thumbcache.h, and [Building Thumbnail Handlers](https://learn.microsoft.com/en-us/windows/win32/shell/building-thumbnail-providers).

Administrator installation verification (2026-10-14): machine-wide class installed; fresh-copy Windows Shell tests passed for both office test PDFs with icon fallback disabled. Explorer visibly displays both generated previews. Earlier per-user-only Shell activation failure is resolved. Our prior Adobe fallback registrations were removed; McMurryPDF remains default. Acrobat is still installed, but no Adobe thumbnail handler is selected for this PDF association.
