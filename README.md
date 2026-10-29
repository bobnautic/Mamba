<p align="center">
  <img src="public/mamba-icon.svg" alt="Mamba" width="128" height="128">
</p>

<h1 align="center">Mamba</h1>

<p align="center">
  <b>Der Open-Source-Tuning-Editor für Steuergeräte-Binaries — 100 % lokal, auf Windows, macOS und Linux.</b><br>
  <sub>Open Source · GPL-3.0</sub>
</p>

![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-2fcb46) ![Engine](https://img.shields.io/badge/detection%20engine-Rust-c6f135) ![License](https://img.shields.io/badge/license-GPL--3.0-18863a)

Binary rein, Kennfelder raus: Mamba öffnet den Flash-Dump eines Motorsteuergeräts, findet die Kennfelder (Fahrerwunsch, Ladedruck, N75, Einspritzbeginn, Drehmomentbegrenzer und viele mehr) und lässt sie dich als Tabelle, als 2D-Kurve, als 3D-Fläche oder direkt im Hexdump bearbeiten. Versionen anlegen und vergleichen, Fehlercodes abschalten und wieder einschalten, Prüfsumme korrigieren, Binary oder WinOLS-Mappack exportieren.

Jedes andere Steuergerät lässt sich ebenfalls öffnen — mit den Kennfelddefinitionen, die du mitbringst: ein WinOLS-`.ols`-Projekt, eine TunerPro-`.xdf` oder ein JSON-Mappack.

Kein Konto, keine Cloud, keine Limits: Alles läuft lokal, deine Dateien bleiben auf deinem Rechner.

## 🐍 Was Mamba kann

- **Automatische Kennfelderkennung** — eingebaute Rust-Engine mit Detektoren pro Steuergerätefamilie (VAG Bosch EDC15/EDC16)
- **ECU-Erkennung für jedes Binary** — liest die Identifikationsblöcke im Flash, egal von welchem Hersteller: Steuergerätehersteller und -familie (z. B. EDC17C46, MED17.7.1, MD1CP001, SIMOS18, SID, DCM, Marelli), Hardware-, Software- und Teilenummern, Datenstand, VAG-ODX-Kennung und SW-Index, Motor und Motorkennbuchstaben, Fahrzeughersteller, Prozessor und Byte-Reihenfolge — auch für Steuergeräte ohne Mamba-Kennfelddetektor
- **Projektverwaltung** — Bibliothek mit Suche über alle Felder (auch FIN, Kennzeichen, Tags und alle im Binary gefundenen Nummern), Filter nach Steuergerät, Marke, Stage und Kraftstoff, Sortierung, Duplikaterkennung per SHA-256 („diese Datei ist schon in deiner Bibliothek“), Projekt duplizieren, Export und Import als `.mamba`-Archiv (Original, alle Versionen und Kennfeldänderungen in einer Datei)
- **Eigene Kennfelddefinitionen (Beta)** — beliebiges Binary öffnen und ein WinOLS-`.ols`-Projekt, eine TunerPro-`.xdf` oder ein JSON-Mappack importieren
- **Vollständigkeitsprüfung** — ein Badge zeigt, ob alle für die Familie erwarteten Kennfelder gefunden wurden
- **Kennfeld-Editor** — Tabelle, 2D-Graph und 3D-Fläche, Tastaturnavigation, Kopieren/Einfügen zwischen Kennfeldern (auch mit Excel), absolute, additive und prozentuale Änderungen, Übertragen auf ähnliche Kennfelder, WinOLS-Tastenkürzel
- **Hexdump-Editor** — virtualisiert, mit Minimap und Markierung der Änderungen gegenüber dem Original
- **Versionen** — „Ori“ plus benannte Versionen pro Projekt, Vergleichsansicht
- **Virtueller Prüfstand** — Leistungs-/Drehmomentschätzung aus den Kennfeldern, druckbarer PDF-Bericht
- **DTC an/aus** — Fehlercodetabelle lesen, Codes abschalten und wieder aktivieren (EDC15 und EDC16), mit Beschreibung in allen App-Sprachen
- **Prüfsummenkorrektur** — EDC15-Familie und EDC16, nativ implementiert
- **Exporte** — modifizierte `.bin`, JSON-Mappack kompatibel zu WinOLS 5
- **Automatische Updates** — Mamba prüft einmal täglich die [Releases dieses Repos](https://github.com/bobnautic/Mamba/releases); Installation mit einem Klick
- **3 Themes** — Dunkel (Black Mamba), Hell und OLED
- **Sieben Sprachen** — Deutsch, Englisch, Französisch, Spanisch, Italienisch, Portugiesisch und Rumänisch

## 🚗 Automatisch erkannte Steuergeräte

| Steuergerät | Erkennung |
|-------------|-----------|
| Bosch EDC15P | Muster + Codeblock |
| Bosch EDC15VM+ | Muster + Codeblock |
| Bosch EDC16U1 | Signatur |
| Bosch EDC16U31 | Signatur |
| Bosch EDC16U34 | Signatur |

**Alle anderen Steuergeräte öffnen sich trotzdem.** Begrenzt ist nur die automatische Kennfelderkennung, nicht die App: Jedes Binary kann geöffnet werden, die Kennfeldliste kommt dann aus einem `.ols`-Projekt, einer `.xdf` oder einem JSON-Mappack.

Erkannt wird dabei trotzdem jedes Steuergerät, dessen Flash seine Kennung trägt: Die ECU-Erkennung liest die Identifikationsblöcke von Bosch (Familienkennung wie `34/1/EDC17C64/5/P866//…`, 1037-Softwarenummer, 0281/0261-Hardwarenummer), Continental/Siemens, Delphi, Magneti Marelli und der Fahrzeughersteller (VAG-ODX-Kennung und Teilenummern, Mercedes, PSA, Ford …). Das Projekt heißt dann „Bosch EDC17C46“ statt „Unbekannt“, Hersteller, Motor und Kraftstoff werden vorausgefüllt, und die Byte-Reihenfolge folgt der Familie (oder wird an den Kennfeldachsen geschätzt).

Die Zuordnung zu einem Kennfelddetektor bleibt bewusst streng: Eine Datei wird nur dann als eine der Familien oben behandelt, wenn es positive Belege gibt (Bosch-Hardwarenummern, Familienkennungen, strukturelle Signaturen). Ein 2-MB-Dump eines anderen Steuergeräts (EDC17, Marelli, Siemens …) wird nie als EDC16 fehlinterpretiert: Er öffnet sich als erkanntes Steuergerät ohne Kennfelddetektor.

### Tipp für stark modifizierte Dateien

Die Erkennung arbeitet auf der **Struktur, nicht auf den Daten** — getunte Stage-1/2/3-Dateien werden fast immer korrekt erkannt. Bei extrem veränderten Dateien gilt:

1. Projekt aus der **Originaldatei** anlegen — dort findet die Erkennung alle Kennfelder.
2. Die modifizierte Datei als **neue Version** des Projekts importieren.

Alle Versionen teilen sich die Kennfeldliste des Originals, und die Vergleichsansicht gibt es gratis dazu.

## 🎨 Look & Feel einer Mamba

Mamba trägt den Look ihres Namensgebers, der **Black Mamba**: Das Logo zeigt sie angriffsbereit, mit aufgerissenem, tintenschwarzem Maul, Giftzähnen, anthrazitfarbenen Schuppen und giftgrün leuchtendem Auge. Die App ist entsprechend in Black-Mamba-Anthrazit gehalten, Giftgrün ist der Akzent.

| Token | Farbe | Verwendung |
|-------|-------|------------|
| `mamba-500` | ![#2fcb46](https://img.shields.io/badge/-%232fcb46-2fcb46) | Giftgrün: Randlicht des Logos, Primärakzent |
| `venom-400` | ![#c6f135](https://img.shields.io/badge/-%23c6f135-c6f135) | leuchtendes Auge, Verlaufsende |
| `mamba-700` | ![#18863a](https://img.shields.io/badge/-%2318863a-18863a) | Buttons (weiße Schrift lesbar) |
| Black Mamba | ![#262b29](https://img.shields.io/badge/-%23262b29-262b29) ![#070908](https://img.shields.io/badge/-%23070908-070908) | Schuppen der Schlange, Hintergrund des dunklen Themes |

Die Palette liegt in `tailwind.config.ts` (`mamba-*`, `venom-*`), das Logo in `public/mamba-icon.svg` (App-Icon mit Kachel) und `public/mamba-mark.svg` (freigestellte Schlange für kleine Größen, z. B. neben dem Schriftzug). Alle App-Icons, die Installer-Grafiken und das DMG-Hintergrundbild sind daraus abgeleitet. Warn- und Fehlerfarben (rot/orange) sowie die WinOLS-Konvention für geänderte Werte (rot = über dem Original, blau = darunter) bleiben bewusst erhalten.

## ⬇️ Download

Installer erscheinen unter **[Releases](https://github.com/bobnautic/Mamba/releases)**:

- **Windows** — `Mamba_x.y.z_x64-setup.exe` (Windows 10/11)
- **macOS** — `Mamba_x.y.z_macos-universal.dmg` (Apple Silicon und Intel, macOS 12+). Nicht signiert: beim ersten Start *Systemeinstellungen › Datenschutz & Sicherheit › Trotzdem öffnen*, oder per Terminal installieren:
  ```bash
  curl -fsSL https://raw.githubusercontent.com/bobnautic/Mamba/main/install-macos.sh | sh
  ```
- **Linux** — `Mamba_x.y.z_linux-x86_64.AppImage` oder `.deb`

Ein Release entsteht, sobald ein Tag `vX.Y.Z` gepusht wird (siehe [`.github/workflows/release.yml`](.github/workflows/release.yml)).

## 🛠️ Entwicklung

Voraussetzungen:
- [Node.js](https://nodejs.org) ≥ 18
- [Rust](https://rustup.rs) (stable)
- Windows 10/11, macOS 12+ mit Xcode Command Line Tools, oder Linux mit den [Tauri-Voraussetzungen](https://tauri.app/start/prerequisites/) (webkit2gtk 4.1, gtk3, libayatana-appindicator, librsvg)

```bash
npm install
npm run app:dev     # startet die Desktop-App mit Hot Reload
npm run app:build   # baut den Installer unter src-tauri/target/release/bundle/
```

Tests der Erkennungs-Engine:

```bash
cd src-tauri && cargo test
```

## 🧱 Architektur

```
src/                  Next.js-Frontend (statischer Export, in der Tauri-Webview)
  app/dashboard/      Projektliste (Startseite)
  app/editor/         der Kennfeld-Editor
  lib/local/          lokales Backend: Projektablage auf der Platte + API-Bridge
  lib/ecu/            TypeScript-Helfer für Steuergeräte (DTC-Listen, Prüfsummen)
src-tauri/            Rust-Desktop-Shell
  src/detector/       die Erkennungs-Engine (ein Ordner pro Hersteller)
  src/detector/ecu_recognition.rs   ECU-Erkennung für jedes Binary (Identifikationsblöcke)
  src/project_archive.rs            Projektarchive (.mamba): Export, Import, Duplizieren
  src/commands.rs     IPC-Befehle für das Frontend
  icons/              App-Icons und Installer-Grafiken (aus public/mamba-icon.svg)
```

Projekte liegen unter `%APPDATA%/com.bobnautic.mamba/projects/` (Windows), `~/Library/Application Support/com.bobnautic.mamba/projects/` (macOS) bzw. `~/.local/share/com.bobnautic.mamba/projects/` (Linux) — ein Ordner pro Projekt mit Original-Binary, Metadaten und Versionen.

## 🗺️ Roadmap

Was geplant ist: [ROADMAP.de.md](ROADMAP.de.md) (auch auf [Englisch](ROADMAP.md) und in den übrigen App-Sprachen). Dieselbe Seite öffnet sich in der App.

## 🤝 Mitmachen

Beiträge sind willkommen, neue Steuergerätefamilien zuerst — nach den Standards in [CONTRIBUTING.md](CONTRIBUTING.md). Fehler oder nicht erkannte Kennfelder bitte als [Issue](https://github.com/bobnautic/Mamba/issues) melden, mit Steuergerätetyp und Softwarenummer der Datei.

## 🙏 Danksagung

Die Erkennungs-Engine steckt voller Reverse-Engineering-Arbeit an echten Dumps. Alle, die am Code mitgewirkt haben, stehen in [CONTRIBUTORS.md](CONTRIBUTORS.md).

Dank an **Dilemma** für [VAGEDCSuite](https://github.com/Blackfrosch/VAGEDCSuite) und **Skalda** für dessen [Weiterentwicklung](https://github.com/skaldamramra/VAGEDCSuite) — die Vorbilder für automatische Kennfelderkennung mit einfacher Oberfläche.

## ⚖️ Lizenz

[GPL-3.0](LICENSE) — du darfst Mamba nutzen, studieren, verändern und weitergeben; abgeleitete Werke müssen unter derselben Lizenz stehen. Die Copyright- und Lizenzhinweise stehen in [NOTICE](NOTICE).

## ⚠️ Haftungsausschluss

Mamba ist für Forschung, Ausbildung sowie Motorsport- und Offroad-Einsatz gedacht. Das Verändern des Steuergeräts eines Straßenfahrzeugs kann in deinem Land illegal sein, die Garantie und die Betriebserlaubnis erlöschen lassen, den Motor beschädigen oder das Fahrzeug abgasrechtlich unzulässig machen. Du allein bist für die Verwendung verantwortlich. Abgasrelevante Abschaltungen (AGR, DPF, SCR) sind ausdrücklich nicht Teil des Projekts.

**Sicherheit:** Tuning-Software ist ein beliebtes Ziel für Malware. Lade Mamba nur aus den [offiziellen Releases](https://github.com/bobnautic/Mamba/releases) herunter.
