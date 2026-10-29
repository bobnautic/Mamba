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

Ein Release entsteht, sobald ein Tag `vX.Y.Z` gepusht wird ([`.github/workflows/release.yml`](../.github/workflows/release.yml)). Der Workflow baut die Installer für Windows, macOS (universal) und Linux. Die App findet neue Releases über die automatische Update-Prüfung.

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
