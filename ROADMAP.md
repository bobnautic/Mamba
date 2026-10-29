# Mamba roadmap

This page lists where Mamba is heading: from today's VAG EDC15/EDC16 editor to a complete tuning editor for any ECU binary.

To ask for a feature or report a bug, open an issue on [GitHub](https://github.com/bobnautic/Mamba/issues).

## In 1.0

- The Mamba identity: name, snake logo, green and black theme, installer artwork and its own update channel.
- The editor: automatic map detection on Bosch EDC15P, EDC15VM+, EDC16U1, EDC16U31 and EDC16U34; any other binary opens with a WinOLS `.ols` project, a TunerPro `.xdf` or a JSON mappack; table, 2D, 3D and hexdump editing; versions and compare; virtual dyno; DTC on/off; checksum correction; WinOLS mappack export.

## Planned

- Undo and redo (Ctrl+Z / Ctrl+Y) in the editor.
- A map finder for binaries without definitions: candidate maps suggested from the structure of the file (axes, rising breakpoints, 8 and 16-bit grids), named and confirmed by hand.
- Writing a map list back out as a TunerPro `.xdf`, and CSV import and export of a map.
- The maps of two versions side by side in the Compare window, differences highlighted.
- WinOLS-style 2D view for full matrices: one curve per row.
- Checksums beyond EDC15/EDC16, family by family as each one is verified.
- Favourite maps, and a reference version other than Ori.

## Not planned

- Emissions-related removals (EGR off, DPF off, AdBlue/SCR off) and one-click tunes. Mamba is an editor: you see the maps and you decide what you change.
- Accounts or a cloud. Everything stays on your computer.
