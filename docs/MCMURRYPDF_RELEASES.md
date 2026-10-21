# McMurryPDF Windows distribution

## Install on another computer

Current testing release: [McMurryPDF 0.8.5](https://github.com/rrmcmurry/pdfcraft/releases/tag/mcmurrypdf-v0.8.5-testing). [Download the signed Windows x64 installer](https://github.com/rrmcmurry/pdfcraft/releases/download/mcmurrypdf-v0.8.5-testing/McMurrySoftware.McMurryPDF-testing-Setup.exe). Existing testing installations discover it through Help > Check for updates. Opening a second PDF in a separate window now brings the new document forward without reactivating the first. Saved zoom preferences retain the 0.8.4 behavior. Signed updates from published and locally patched 0.8.4, packaged two-document opening/rendering, zoom persistence, clean Setup, preserved PDFs/initials/settings/default association, and fresh independent thumbnails passed in Sandbox. The owner verified foreground activation on the real desktop before release. All six published assets match local SHA-256 digests and sizes; public-feed checks confirm 0.8.4 discovers 0.8.5 and 0.8.5 is current.

Version 0.8.2 adds persistent workpaper marking (typed references, dot/check shortcuts, font/size/color controls), personal initials with a date fixed when placed, capture of initials from a PDF page, and tray visibility controlled by the user. Marks and sign-offs remain portable PDF comments. Each user supplies their own initials; no personal signature is bundled in the installer.

Download **McMurrySoftware.McMurryPDF-testing-Setup.exe** from the McMurryPDF testing release in [this fork's releases](https://github.com/rrmcmurry/pdfcraft/releases). This is an x64 Windows pilot build, not a claim of Acrobat annotation parity. Check Properties > Digital Signatures: the publisher is **Robert McMurry** (McMurry Software's signing identity). Do not use the old unsigned prototype or the internal update-probe package.

Setup installs per user under `%LOCALAPPDATA%\McMurrySoftware.McMurryPDF`. The launch path stays `current\McMurryPDF.exe` across updates. No office share or mapped drive is needed. The native app does not require .NET; Setup installs the Microsoft Visual C++ 2022 x64 runtime if missing, which may require internet access and administrator approval.

On first launch, choose **Install or repair Explorer thumbnails** if you want PDF page previews. Windows requests administrator approval for this separate, signed Windows Installer component. It installs our isolated thumbnail provider and its own CLI renderer into Program Files. Acrobat is not required. The component can be removed separately in Windows Installed apps as **McMurryPDF Explorer Thumbnails**.

Choose McMurryPDF as the PDF default through Windows' Open with / Always or Default apps controls. Setup registers the stable executable but does not manufacture a Windows UserChoice hash. When moving from an office prototype, select the installed copy once; future package updates retain its path. The existing office shortcut is not automatically redirected by source builds.

Settings, recovery and custom assets belong in the user's McMurryPDF data folders, never in `current`. Uninstall preserves those folders and user documents. New installations default to separate document windows; Preferences > Documents and view can enable tabs.

## Updates

Use **Help > Check for updates**, then **Download update**, then **Install update and restart**. Checks and downloads run in the background. Startup does not contact the feed or silently apply a previously downloaded package. Save your work and close other McMurryPDF windows and CLI jobs before applying; the shared process lease prevents replacement while those jobs are active. The update helper waits for the requesting process to exit before replacing files.

Installed packages use their installed channel (`testing` for the pilot) and the public GitHub releases on `rrmcmurry/pdfcraft`. HTTPS retrieves release metadata and packages; packages are checked against the release feed's SHA-256 digest. The update service receives no PDF paths, contents or document attachments. This is not a complete audit of all optional PDF features or dependencies. GitHub release write access remains part of the distribution trust boundary.

The thumbnail component has an independent machine installation and version. After an application update, the updates dialog offers its administrator installer when the installed thumbnail version differs. It does not kill Explorer. Versioned native files and a machine renderer path let an already loaded provider find the current standalone renderer.

## Build and sign

The repository pins Velopack 1.2.0 and WiX 6.0.2 in `.config/dotnet-tools.json`. Building requires the Rust toolchain, MSVC/Windows SDK, .NET SDK, the `craft-fonts` build input, verified OCR models and Azure Artifact Signing access. None of these developer prerequisites are required on the destination computer.

Run `packaging/windows/Build-McMurryRelease.ps1` with `-Version`, `-Channel`, `-ModelsDirectory`, `-SigningMetadataPath` and `-SigningDlibPath`. Signing configuration is supplied from outside the repository; never commit credentials, account metadata or tokens. `Sign-McMurryArtifact.ps1` uses the Microsoft Artifact Signing client and verifies Authenticode, timestamp and publisher. The packager verifies the packaged GUI, CLI, thumbnail MSI and updater, not just raw build outputs. It writes an artifact hash manifest.

`-ReleaseDirectoryName` chooses a simple folder name under `dist/mcmurry`, keeping internal update probes separate from distributable packages. `-UpdateProbe` is for disposable sandbox tests only: never publish it. Stable builds require a clean `main` checkout. The first distribution is a clearly labeled testing prerelease from the feature branch while the PR is reviewed.

For the next release, increment the workspace version and independent thumbnail version when its files change. Build in the same channel output directory with the previous real release present for delta generation. Publish Setup, the full package, any generated delta packages, `releases.<channel>.json`, and the build hash manifest together on a GitHub release. Keep the previous release available. Do not upload internal probes, private test documents, sandbox logs or signing metadata.

## Validation and remaining work

`packaging/windows/Test-McMurrySandbox.ps1` exercises install, fresh thumbnails with Acrobat absent, corrupted-package rejection, update, uninstall and independent thumbnail removal in a dedicated offline Windows Sandbox. Its input folder is read-only; it creates its own synthetic PDF. Local-source overrides use `MCMURRYPDF_UPDATE_SOURCE` for a directory containing the channel feed and packages; normal installs use GitHub.

The earlier targeted security review remains applicable with limits: no application document-upload client was identified, and agent/MCP listeners stay opt-in. Known dependency advisories have not all been remediated. The installed update client uses Velopack/ureq's standard TLS configuration: no custom TLS session logging, configured ECH or rustls Brotli feature was found; the rustls key-share prediction advisory remains a residual limitation rather than an authentication bypass. RSA's recorded advisory concerns private-key operations; the inspected RSA use is verification. Continue dependency tracking before broad office deployment.

The 2026-10-15 OSV refresh checked 735 crates.io versions in the final lockfile. It returned the six previously recorded findings plus the updater's build-time `derivative` dependency, [RUSTSEC-2024-0388](https://rustsec.org/advisories/RUSTSEC-2024-0388.html), an unmaintained procedural macro. These are tracked limitations of the testing release, not a clean vulnerability-audit result. Git dependencies and vendored changes are outside that registry name/version scan.

Stamps, annotation save/reopen/interoperability, large real-world document performance, thumbnail size/time limits, protected documents, and broader rollback coverage remain follow-up work. A signed installer proves publisher identity and file integrity, not that every PDF feature is correct or safe.
