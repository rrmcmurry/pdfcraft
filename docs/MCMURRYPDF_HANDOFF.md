# McMurryPDF handoff — 2026-10-14

## Resume here

Design and implement a signed Windows installer and update workflow based on the owner's existing Ledger Velopack distribution. Target office, home and laptop use without depending on mapped drives. Start with packaging and a tested install/update/uninstall cycle, then stamps and annotation interoperability. No installer or automatic update feed has been published yet.

## Completed and locally verified

- McMurryPDF / McMurry Software branding, original ampersand logo, independent settings and recovery namespace, fork-owned help/update URLs, and removal of restricted upstream trademark artwork while preserving copyright and license notices.
- Persistent “Open documents as tabs in the same window” preference; native desktop default is separate windows. New-document handoff preserves unsaved-document semantics. Recovery and special internal document views retain existing session handling. Six focused preference/window/recovery tests passed; recovery failure reproduced before its fix.
- Embedded PNG loading fixed; its regression reproduced NoMatchingImageLoader before correction and passed afterward.
- Reporting Services stream parser accepts horizontal padding before the required line ending without consuming leading payload whitespace. Synthetic regression failed before the change and passed afterward. 216 available vendored parser tests passed; two unrelated version tests need external fixture files absent from this checkout. Three-page office report now renders. No real report content is in Git.
- Stable Windows executable names and PDF association; preserve Windows UserChoice rather than fabricating its hash. Remove old versioned application IDs and OpenWithList entries. Earlier duplicate menu issue was confirmed fixed by the owner. Registry views seen by automated commands once disagreed with the visible editor; verify changes in the actual desktop session.
- Independent thumbnail provider source in packaging/windows/mcmurry-shell. It is a small standalone native COM adapter around the existing Rust CLI renderer, not a PDF parser. IInitializeWithStream / IThumbnailProvider, isolated Windows activation, hidden bounded child process, temporary stream copy, first-page PAM raster to HBITMAP. No Adobe dependency. GUI/CLI stay Rust. Existing native integration source is preserved separately from Cargo crates.
- User performed administrator COM class installation. Fresh-copy Windows Shell thumbnail tests passed for two reports with icon fallback disabled; Explorer visibly displayed both previews. Modern PDF registrations select our CLSID, prior Adobe fallback registrations removed, McMurryPDF remains default. Acrobat is still installed; no uninstall experiment was claimed.

## Packaging plan and unresolved design work

1. Reuse Ledger's signing/release practices and Velopack approach; Rust integration exists. Use a distinct package identity, stable launch registration, original branded installer resources and testing/stable channels. Do not publish or reuse Ledger package IDs.
2. Signed internet release feed for home/laptop access; optional office package cache. Avoid mapped-drive assumptions. Updates should check/download in the background and apply only after documents can be closed safely; preserve unsaved annotations and recovery state.
3. Keep settings, custom stamps and recovery outside replaceable application files. Uninstall preserves user PDFs. Let Windows' supported default-app selection flow establish PDF ownership.
4. Resolve thumbnail installation/update lifecycle: COM registration currently needs administrator installation for isolated activation on this machine. Do not set DisableProcessIsolation. The surrogate can retain the DLL; coordinate updates without killing unrelated host processes. Prefer trusted installed shell-component location, version/signature checks, and a stable renderer dependency. Current prototype uses the sibling CLI, so do not move the DLL alone.
5. The native adapter is x64, unsigned internal prototype, built with MSVC static runtime. Limits: 512 MiB document stream, 512 MiB renderer process, 12-second rendering timeout, one render per host, requested size up to 1024, first page at 96 dpi. Stream copy is size bounded, not included in rendering timeout. Thorough resource-limit, protected/encrypted PDF and update/rollback tests remain before broad deployment.
6. Current office deployment was staged outside the checkout; source-tree release builds are not installed update packages. Packaging scripts accept an installed AppDirectory; do not commit the office's absolute paths or private diagnostic images.
7. Then address stamps (appearance generation, save/reopen, custom assets) and annotations (appearance streams, editing, flattening, interoperability). Round-trip synthetic files between readers before claiming Acrobat parity. The user's real large-file performance testing remains outstanding.

## Security/network evidence and limits

The baseline source security review examined desktop startup, update networking, PDF JavaScript, MCP/control opt-in boundaries, process calls, signing identities, file operations, build scripts and pinned dependencies. Application-source searches found no document-upload client. The reviewed update checker fetched bounded GitHub release metadata only on explicit invocation and did not automatically install updates. PDF JavaScript exposed stubbed HTTP calls and restricted launchURL to web URLs; external link/shell behavior still needs clear user-facing policy.

Offline Windows Sandbox smoke tests exercised synthetic CLI and desktop flows with networking disabled and no office document/share exposure. Defender checks found no detections in the inspected builds. This is useful targeted evidence, not a complete dependency audit or guarantee that every feature stays offline. No personal documents or binaries were uploaded to public scanning services.

Dependency advisory findings in the earlier local review remain to be rechecked/remediated before adding network update distribution: rsa timing advisory (verification path / documented upstream exception), rustls advisories with differing feature applicability, and an unmaintained build-time proc-macro dependency. Do not copy signing credentials, local accounting files, security scan artifacts containing personal paths, or real test PDFs into the repository.

## Validation

Release GUI/CLI built successfully. Scoped Clippy with warnings denied and workspace formatting passed. Assets, dependency layering and parity validation passed. Final rerun: seven focused UI tests passed, 216 available parser tests passed (two missing-fixture version tests excluded), and the WebAssembly gate passed all 31 eligible crates. CLI Windows version-resource test passed. Packaging PowerShell syntax checks passed. Live Windows Shell checks are recorded above; this handoff does not imply every upstream test or standalone vendored Clippy warning is clean. The vendored crate has preexisting strict-lint issues and missing external version fixtures.

References: [Velopack Rust integration](https://docs.velopack.io/getting-started/rust), [Windows packaging](https://docs.velopack.io/packaging/operating-systems/windows), [Microsoft thumbnail handlers](https://learn.microsoft.com/en-us/windows/win32/shell/building-thumbnail-providers).
