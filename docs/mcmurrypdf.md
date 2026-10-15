# McMurryPDF

McMurry Software's office PDF workbench, based on PdfCraft. Required upstream copyrights and third-party notices remain in LICENSE-MIT, LICENSE-APACHE, NOTICE and ATTRIBUTION.md. The ampersand document logo was created for this fork from the user-owned McMurry identity.

## Tray behavior

Annotation tools, page edits, newly opened PDFs (including PDF navigation preferences), and completed signing preserve tray visibility. Fresh profiles start with trays closed; explicitly opened trays and saved workspace visibility are preserved. Open a tray through its button/menu, or request a workspace/chooser such as Stamps, Prepare a form, or Search.

## Workpaper marking

Choose **Mark workpaper** from the floating toolbar or command search. Its text, color and size controls extend the same vertical toolbar, using the existing annotation tools' property-menu behavior. The text entry opens focused: type a reference, press Enter, then click beside each amount; the tool stays active until Esc or another tool is chosen. Click the current-mark preview in the toolbar to reopen the text popup with its existing text selected for replacement. Enter `.` for a dot, `/` for a checkmark, or type references such as `B`, `4797`, `D` or `WTB`. The dot is half its original diameter at the same size setting. A slash within ordinary text, such as `A/B`, stays a slash. Hover over the entry for the symbol shortcuts. No side tray opens.

Text, font, size (6–36 points) and color are remembered. The size popout also offers Helvetica, Times and Courier in regular, bold, italic/oblique and bold italic/oblique variants (12 PDF font choices). These are the PDF standard fonts, not a picker for arbitrary installed system fonts. The initial choice is a red dot at 12 points. Short single-line WinAnsi text is supported, up to 32 characters; unsupported characters are refused rather than silently replaced. Changing a value affects subsequent marks only. Each placement is its own undo step. The checkmark uses a filled, tapered tick silhouette at 70% of its original size; font selection does not change dots or ticks. The pointer preview renders the same artwork as the saved comment.

Marks are normal PDF Stamp comments with their text/symbol, author and placement date, and self-contained vector appearances. They appear in Comments and can be selected, moved or deleted with the Select tool. Placing repeated marks leaves them unselected and respects the current Comments-panel visibility. They remain annotations when saving with the separate Fill & Sign flattening option enabled. No stamp-library asset, account or network connection is needed.

Automation uses `mark_add` with `doc`, `page`, `at` (the centre in displayed-page points), `text`, optional `font` (PDF font name, default `Helvetica`), `font_size`, `color` and `author`. The UI control channel supports `ui.command id=comment.mark`, `ui.set key=mark-text value=4797`, `ui.set key=mark-font value=Courier-Bold`, and `ui.set key=quick value=mark`.

## Personal sign-offs

Choose **Sign off (initials + date)** from the signature icon in the floating toolbar, Fill & Sign, or command search. Each user supplies their own saved initials using **Type**, **Draw**, **Image** or **Capture**. **Capture** dismisses the initials dialog and switches to a crop box on the open PDF; draw around the initials, review the transparent result, then **Apply**. Cancel keeps the previous initials. The active tool's pencil opens initials settings; Capture belongs in that dialog rather than a standalone crop shortcut. Saved initials live in that user's settings, never in the application or repository.

Click repeatedly to place one portable Stamp comment containing the initials and the current local date in `m/d/yy` format. The appearance and date are fixed for each placement; changing saved initials or the date affects future placements only. Existing sign-offs survive save/reopen and Fill & Sign flattening, with normal comment author information, move/resize/delete and per-placement undo. Placing them respects the Comments panel's current visibility.

Page capture respects the document's copy permissions, bounds rendering to four megapixels, removes near-white paper while preserving solid strokes, and trims transparent margins. It does not use the clipboard. The full-signature editor retains its separate photo Scan tab. Automation exposes `signoff_capture` (`doc`, `page`, displayed-point `rect`, output PNG `path`) and `signoff_add` (`doc`, `page`, centre `at`, `graphic` with `image`, `text` or `strokes`, optional `author`), honoring the configured filesystem root.

## Document window preference

In Preferences > Documents and view, turn **Open documents as tabs in the same window** on or off. New McMurryPDF desktop installations default to separate windows. Opening files through File > Open, drag-and-drop file paths, recent files, startup arguments and Windows file associations follows that choice. Existing tabs remain accessible until closed; Combine Files remains a dedicated workspace. Created PDFs and generated operation results can be transferred to a new window as unsaved documents, using a private temporary handoff under McMurryPDF's settings folder. Transfers are removed after successful import, and failed imports leave their file for diagnosis/recovery.

The browser build keeps its existing tab behavior. The desktop agent interface exposes `ui.set key=open-tabs value=off` and reports the preference through ui.state; ui.open reports new_window when a path is dispatched.

## Identity and updates

Signed Windows installer and explicit in-app updates: see [Windows distribution](MCMURRYPDF_RELEASES.md) for installation, the separate Explorer thumbnail component, release builds and remaining pilot limitations. Installed packages retain a stable `current\McMurryPDF.exe` path. The earlier office prototype remains a separate deployment until its shortcut/default is migrated deliberately.

The Windows product and publisher are McMurryPDF / McMurry Software. Settings and recovery use McMurryPDF folders; portable mode uses McMurryPDFData. The fork does not migrate or modify upstream PdfCraft/PrintCraft settings. Help and manual update checks target https://github.com/rrmcmurry/pdfcraft. No automatic download/install or default agent listener is introduced. Rust package and internal CLI names stay unchanged to keep upstream merges manageable; the office staging folder exposes McMurryPDF.exe and McMurryPDF-cli.exe.

## Validation status

The branded Windows build was compiled and launched natively on 2026-10-14. Windows Sandbox verified CLI creation/inspection/rendering/extraction/editing, the real preference checkbox, two native windows with tabs off, two PDFs in one window with tabs on, preference persistence through restart, and McMurryPDF / McMurry Software in About and Windows version information. The final rebuilt executables were copied into the guest and checked against their host hashes before launch. Defender reported no detections for the staged build. The Desktop shortcut targets the stable office folder rather than Cargo's build directory.

All six focused window/preference/recovery tests passed. The recovery regression reproduced incorrect save-path assignment before the fix and passed after correction. Scoped Clippy with warnings denied, formatting, assets, layering, and parity checks passed. Recovery and specialized internal revision/destination views retain their existing session/tab behavior so their metadata is preserved. Existing tabs remain accessible until closed.

Large-file performance testing is reserved for the user's real documents; no result is claimed. Sandbox smoke checks do not prove complete PDF fidelity or the absence of malware. The baseline dependency advisory findings remain tracked outside the source checkout and have not yet been remediated by this branding/window change.

Reporting Services compatibility: stream headers may contain spaces or tabs before their required line ending. The parser preserves payload whitespace; regression and available parser tests pass. Windows release rebuilt and three-page report rendering verified locally (2026-10-14).
