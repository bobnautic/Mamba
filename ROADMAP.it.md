# Roadmap di Mamba

Questa pagina indica la direzione di Mamba: dall'attuale editor VAG EDC15/EDC16 a un editor di rimappatura completo per qualsiasi binario di centralina.

Per chiedere una funzione o segnalare un bug, apri una issue su [GitHub](https://github.com/bobnautic/Mamba/issues).

## Nella 1.0

- L'identità di Mamba: nome, logo a serpente, tema verde e nero, grafica dell'installer e canale di aggiornamento proprio.
- L'editor: rilevamento automatico delle mappe su Bosch EDC15P, EDC15VM+, EDC16U1, EDC16U31 ed EDC16U34; qualsiasi altro binario si apre con un progetto WinOLS `.ols`, un `.xdf` di TunerPro o un mappack JSON; modifica in tabella, 2D, 3D e nell'hexdump; versioni e confronto; banco virtuale; correzione del checksum; esportazione di mappack WinOLS.

## Previsto

- Annulla e ripeti (Ctrl+Z / Ctrl+Y) nell'editor.
- Un cercatore di mappe per i binari senza definizioni: mappe candidate proposte dalla struttura del file (assi, punti crescenti, griglie a 8 e 16 bit), nominate e confermate a mano.
- Esportazione di una lista di mappe come `.xdf` di TunerPro, e importazione ed esportazione CSV di una mappa.
- Le mappe di due versioni affiancate nella finestra di confronto, con le differenze evidenziate.
- Vista 2D in stile WinOLS per le matrici complete: una curva per riga.
- Checksum oltre EDC15/EDC16, famiglia per famiglia man mano che vengono verificati.
- Mappe preferite e una versione di riferimento diversa da Ori.

## Non previsto

- Rimozioni legate alle emissioni (EGR off, DPF off, AdBlue/SCR off) e rimappature con un clic. Mamba è un editor: vedi le mappe e decidi tu cosa cambiare.
- Account o cloud. Tutto resta sul tuo computer.
