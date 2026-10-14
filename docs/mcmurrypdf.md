# McMurryPDF

McMurry Software's office PDF workbench, based on PdfCraft. Required upstream copyrights and third-party notices remain in LICENSE-MIT, LICENSE-APACHE, NOTICE and ATTRIBUTION.md. The ampersand document logo was created for this fork from the user-owned McMurry identity.

## Document windows

In Preferences > Documents and view, turn **Open documents as tabs in the same window** on or off. New McMurryPDF desktop installations default to separate windows. Opening files through File > Open, drag-and-drop file paths, recent files, startup arguments and Windows file associations follows that choice. Existing tabs remain accessible until closed; Combine Files remains a dedicated workspace. Created PDFs and generated operation results can be transferred to a new window as unsaved documents, using a private temporary handoff under McMurryPDF's settings folder. Transfers are removed after successful import, and failed imports leave their file for diagnosis/recovery.

The browser build keeps its existing tab behavior. The desktop agent interface exposes `ui.set key=open-tabs value=off` and reports the preference through ui.state; ui.open reports new_window when a path is dispatched.

## Identity and updates

The Windows product and publisher are McMurryPDF / McMurry Software. Settings and recovery use McMurryPDF folders; portable mode uses McMurryPDFData. The fork does not migrate or modify upstream PdfCraft/PrintCraft settings. Help and manual update checks target https://github.com/rrmcmurry/pdfcraft. No automatic download/install or default agent listener is introduced. Rust package and internal CLI names stay unchanged to keep upstream merges manageable; the office staging folder exposes McMurryPDF.exe and McMurryPDF-cli.exe.

## Validation status

The branded Windows build was compiled and launched natively on 2026-10-14. Windows Sandbox verified CLI creation/inspection/rendering/extraction/editing, the real preference checkbox, two native windows with tabs off, two PDFs in one window with tabs on, preference persistence through restart, and McMurryPDF / McMurry Software in About and Windows version information. The final rebuilt executables were copied into the guest and checked against their host hashes before launch. Defender reported no detections for the staged build. The Desktop shortcut targets the stable office folder rather than Cargo's build directory.

All six focused window/preference/recovery tests passed. The recovery regression reproduced incorrect save-path assignment before the fix and passed after correction. Scoped Clippy with warnings denied, formatting, assets, layering, and parity checks passed. Recovery and specialized internal revision/destination views retain their existing session/tab behavior so their metadata is preserved. Existing tabs remain accessible until closed.

Large-file performance testing is reserved for the user's real documents; no result is claimed. Sandbox smoke checks do not prove complete PDF fidelity or the absence of malware. The baseline dependency advisory findings remain tracked outside the source checkout and have not yet been remediated by this branding/window change.

Reporting Services compatibility: stream headers may contain spaces or tabs before their required line ending. The parser preserves payload whitespace; regression and available parser tests pass. Windows release rebuilt and three-page report rendering verified locally (2026-10-14).
