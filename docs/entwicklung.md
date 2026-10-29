# Entwicklung

Mamba ist eine [Tauri](https://tauri.app)-App: ein Next.js-Frontend (statischer Export) in der Webview, dahinter eine Rust-Shell mit der Erkennungs-Engine. Wie neue Steuergerätefamilien dazukommen und welche Qualität ein Detektor erreichen muss, steht in [CONTRIBUTING.md](../CONTRIBUTING.md).

## Voraussetzungen

- [Node.js](https://nodejs.org) ≥ 18
- [Rust](https://rustup.rs) (stable)
- je nach System:
  - **Windows** 10/11
  - **macOS** 12+ mit den Xcode Command Line Tools
  - **Linux** mit den [Tauri-Voraussetzungen](https://tauri.app/start/prerequisites/): webkit2gtk 4.1, gtk3, libayatana-appindicator, librsvg

## Bauen und testen

```bash
npm install
npm run app:dev     # Desktop-App mit Hot Reload
npm run app:build   # Installer unter src-tauri/target/release/bundle/

cd src-tauri && cargo test   # Tests der Erkennungs-Engine
```

Das Frontend allein (`npm run dev`) läuft auch im Browser, findet dort aber kein Backend: Projektablage und Erkennung gehen über Tauri-IPC.

## Architektur

```
src/                        Next.js-Frontend (statischer Export, in der Tauri-Webview)
  app/dashboard/            Projektbibliothek (Startseite)
  app/editor/               Kennfeld-Editor
  app/settings/             Einstellungen
  components/               Fenster, Dialoge, Kennfeldansicht, Hexdump
  lib/local/                lokales Backend: Projektablage auf der Platte + API-Bridge
  lib/ecu/                  Steuergeräte-Helfer (DTC-Listen, Prüfsummen, Lösungen)
  i18n/                     Übersetzungen (7 Sprachen)
src-tauri/                  Rust-Desktop-Shell
  src/detector/             Erkennungs-Engine (ein Ordner pro Hersteller und Familie)
  src/detector/ecu_recognition.rs   ECU-Erkennung für jedes Binary (Identifikationsblöcke)
  src/project_archive.rs    Projektarchive (.mamba): Export, Import, Duplizieren
  src/ols_import.rs, xdf_import.rs, mappack_import.rs   Fremdformate
  src/commands.rs           IPC-Befehle für das Frontend
  examples/                 Werkzeuge für den Prüfstand (dump_maps, expected_report …)
  icons/                    App-Icons und Installer-Grafiken
```

Projekte liegen als Dateien im App-Datenordner, ein Ordner pro Projekt:

```
projects/<id>/
  project.json            Metadaten und Erkennungsergebnis
  original.bin            das importierte Binary, Byte für Byte
  versions.json           „Ori“ und alle weiteren Versionen
  edits-<version>.json    Kennfeldänderungen einer Version
  version-<id>.bin        Binary einer importierten Version
```

Die Pfade je System stehen in der [Benutzung](benutzung.md#11-daten-und-sicherung).

## Release

Releases baut und veröffentlicht der Workflow [`.github/workflows/release.yml`](../.github/workflows/release.yml). So entsteht Version `X.Y.Z`:

1. **Version setzen.** Die Versionsnummer steht in fünf Dateien (`package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`). Ein Befehl setzt sie überall:

   ```bash
   npm run version:set -- 1.1.0
   ```

2. **Release-Notizen (optional)** in `docs/releases/v1.1.0.md` schreiben: eine Titelzeile `Mamba 1.1.0`, Abschnitte in Fett (`**Editor**`) und Punkte mit `- `. So zeigt sie auch das Update-Fenster der App an. Fehlt die Datei, nimmt der Workflow die Liste der seit dem letzten Release gemergten Pull Requests.
3. **Mergen** nach `main`. Die CI prüft, dass die Version in allen Dateien gleich ist.
4. **Veröffentlichen**, auf einem der beiden Wege:
   - auf GitHub unter *Actions › Release › Run workflow* mit Branch `main` und Haken bei **publish**; der Tag `v1.1.0` wird dabei angelegt, oder
   - per Tag:

     ```bash
     git tag v1.1.0 && git push origin v1.1.0
     ```

Der Workflow prüft zuerst, dass Tag und Version zusammenpassen und der Tag neu ist, baut dann parallel auf Windows, macOS und Linux und veröffentlicht das Release mit diesen Dateien:

| Datei | Wofür |
|---|---|
| `Mamba_X.Y.Z_x64-setup.exe` | Windows-Installer (NSIS) |
| `Mamba_X.Y.Z_macos-universal.dmg` | macOS, Apple Silicon und Intel, Erstinstallation |
| `Mamba_X.Y.Z_macos-universal.app.tar.gz` | macOS-Update in der App und `install-macos.sh` |
| `Mamba_X.Y.Z_linux-x86_64.AppImage`, `Mamba_X.Y.Z_linux-amd64.deb` | Linux |
| `SHA256SUMS.txt` | Prüfsummen aller Dateien |

Jede Version bleibt unter [Releases](https://github.com/bobnautic/Mamba/releases) mit ihren eigenen Dateien abrufbar. Die App findet das neueste Release über die automatische Update-Prüfung; die Dateinamen sind die, die der Updater (`src-tauri/src/update.rs`) erwartet.

Ein Testbuild ohne Veröffentlichung: *Run workflow* ohne Haken bei **publish**. Pull Requests, die den Workflow, die Tauri-Konfiguration, Icons oder Installer-Dateien ändern, bauen ebenfalls alle Installer. Die Dateien liegen dann als Artefakte am Workflow-Lauf.

Die Installer sind nicht signiert: Windows zeigt beim ersten Start die SmartScreen-Warnung, macOS verlangt *Trotzdem öffnen*. Für signierte Builds braucht es ein Windows-Code-Signing-Zertifikat bzw. eine Apple Developer ID.

## Design

Mamba trägt den Look der **Black Mamba**: Anthrazit als Grundton, Giftgrün als Akzent.

| Token | Farbe | Verwendung |
|---|---|---|
| `mamba-500` | ![#2fcb46](https://img.shields.io/badge/-%232fcb46-2fcb46) | Giftgrün, Primärakzent |
| `venom-400` | ![#c6f135](https://img.shields.io/badge/-%23c6f135-c6f135) | leuchtendes Auge, Verlaufsende |
| `mamba-700` | ![#18863a](https://img.shields.io/badge/-%2318863a-18863a) | Buttons (weiße Schrift lesbar) |
| Black Mamba | ![#262b29](https://img.shields.io/badge/-%23262b29-262b29) ![#070908](https://img.shields.io/badge/-%23070908-070908) | Schuppen, Hintergrund des dunklen Themes |

- Die Palette liegt in `tailwind.config.ts` (`mamba-*`, `venom-*`).
- Das Logo liegt in `public/mamba-icon.svg` (App-Icon mit Kachel) und `public/mamba-mark.svg` (freigestellte Schlange für kleine Größen). Alle App-Icons, Installer-Grafiken und das DMG-Hintergrundbild sind daraus abgeleitet.
- Warn- und Fehlerfarben (rot/orange) und die WinOLS-Konvention für geänderte Werte (rot = über dem Original, blau = darunter) bleiben bewusst erhalten.
