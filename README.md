# MyTools

A compact desktop shelf for useful software you own. No subscription, ads, analytics, or document uploads.

[Download the latest release](https://github.com/ReaganCode23/my-tools-desktop/releases/latest)

- **Little Sign** — draw a signature with a touchpad or mouse; add printed names and dates; place, resize, and save on any page.
- **PDF Kit** — merge PDFs, reorder input documents, or extract page ranges.
- **Image Clean** — convert PNG/JPEG/WebP images and remove embedded metadata from the exported copy.
- **Password Studio** — generate passwords using the system's cryptographic random generator. Nothing is saved or uploaded.
- **Your shelf** — add website links or local apps, search the collection, and export/import web-tool bookmarks.

## Install

**Windows 10/11, x64:** Download **mytools.exe** from Releases and run it. This is the installer; the installed app is also named `mytools.exe`. It adds Start menu and desktop shortcuts. The small installer uses Microsoft's system WebView2. Windows usually already has WebView2; if absent, the installer downloads it once. This download is separate from the app's attachment size. Releases are update-signed, but do not currently carry a paid Windows Authenticode certificate.

**Linux, x64:** Download **mytools-linux-x86_64.tar.gz**, extract it, and run `./install.sh` from the extracted directory. The installer copies MyTools into `~/.local/share/mytools` and adds a desktop shortcut and app-menu entry, without sudo. It uses the system's WebKitGTK 4.1 and GTK3. Fedora packages: `webkit2gtk4.1 gtk3`; Ubuntu/Debian packages: `libwebkit2gtk-4.1-0 libgtk-3-0`. `.rpm` and `.deb` packages are also available; use your package manager to update system-installed copies.

The Linux portable installer is recommended for in-app updates: its executable is in a folder you own. All built-in tools work offline after installation. Website shortcuts and update checks require a connection.

## Updates

Open **Updates & about → Check for updates**. Downloads are verified against the app's embedded signing public key. Download first, save any unfinished documents, then choose **Restart & install**. The optional launch-time check starts off. No documents or passwords are transmitted to GitHub.

Windows uses the regular NSIS installer for updates. The user-installed Linux edition atomically replaces its executable after verifying the signed update archive, keeping one previous executable for recovery. Signed update metadata lives at `releases/latest/download/latest.json`.

## Build and release

Requires Node 22+, Rust stable, and [Tauri's platform prerequisites](https://v2.tauri.app/start/prerequisites/).

```
npm ci
npm run check
npm start
```

For Windows: `npm run build`. For Linux: `npm run build -- --config src-tauri/tauri.linux.conf.json`.

Push a tag such as `v1.0.1` after changing the version in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. GitHub Actions builds Windows and Linux, enforces the Windows installer size budget, signs update files using the repository secret `TAURI_SIGNING_PRIVATE_KEY`, and publishes one release with `latest.json`. No private signing keys belong in source control. The public key is in Tauri's config.

## Privacy and practical limits

Files are processed in memory and only saved where you choose. Added-tool bookmarks persist in the application's local configuration directory; passwords have no history. Clipboard contents may be retained by your operating system or clipboard manager. No password vault is included. Little Sign places visual signatures; it does not provide certificate-based digital signatures, audit trails, or a signing service. Password-protected PDFs must first be unlocked elsewhere. Image metadata removal does not hide information visible in the picture or its filename; animated images export as a still frame. PDF page copying may not preserve interactive document-level forms, outlines, or existing digital signatures.

PDF.js and pdf-lib are bundled locally with their license notices. System web engines supply security fixes through normal OS/WebView updates.
