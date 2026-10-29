# Mamba-Roadmap

Diese Seite zeigt, wohin die Reise für Mamba geht: vom heutigen VAG-EDC15/EDC16-Editor zu einem vollständigen Tuning-Editor für jedes Steuergeräte-Binary.

Für Funktionswünsche oder Fehlerberichte bitte ein Issue auf [GitHub](https://github.com/bobnautic/Mamba/issues) eröffnen.

## In 1.0

- Die Mamba-Identität: Name, Schlangen-Logo, grün-schwarzes Theme, Installer-Grafiken und ein eigener Update-Kanal.
- Der Editor: automatische Kennfelderkennung für Bosch EDC15P, EDC15VM+, EDC16U1, EDC16U31 und EDC16U34; jedes andere Binary öffnet sich mit einem WinOLS-`.ols`-Projekt, einer TunerPro-`.xdf` oder einem JSON-Mappack; Bearbeitung als Tabelle, 2D, 3D und im Hexdump; Versionen und Vergleich; virtueller Prüfstand; DTC an/aus; Prüfsummenkorrektur; WinOLS-Mappack-Export.

## Geplant

- Rückgängig und Wiederholen (Strg+Z / Strg+Y) im Editor.
- Ein Kennfeld-Finder für Binaries ohne Definitionen: Kandidaten werden aus der Struktur der Datei vorgeschlagen (Achsen, steigende Stützstellen, 8- und 16-Bit-Raster) und von Hand benannt und bestätigt.
- Export einer Kennfeldliste als TunerPro-`.xdf` sowie CSV-Import und -Export einzelner Kennfelder.
- Die Kennfelder zweier Versionen nebeneinander im Vergleichsfenster, Unterschiede hervorgehoben.
- 2D-Ansicht im WinOLS-Stil für volle Matrizen: eine Kurve pro Zeile.
- Prüfsummen über EDC15/EDC16 hinaus, Familie für Familie, sobald sie verifiziert sind.
- Favoriten-Kennfelder und eine andere Referenzversion als Ori.

## Nicht geplant

- Abgasrelevante Abschaltungen (AGR aus, DPF aus, AdBlue/SCR aus) und Ein-Klick-Tunings. Mamba ist ein Editor: Du siehst die Kennfelder und entscheidest selbst, was du änderst.
- Konten oder eine Cloud. Alles bleibt auf deinem Rechner.
