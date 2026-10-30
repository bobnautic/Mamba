# Feuille de route de Mamba

Cette page indique la direction de Mamba : de l'éditeur VAG EDC15/EDC16 actuel à un éditeur de reprogrammation complet pour n'importe quel binaire de calculateur.

Pour demander une fonction ou signaler un bug, ouvrez une issue sur [GitHub](https://github.com/bobnautic/Mamba/issues).

## Dans la 1.0

- L'identité Mamba : nom, logo serpent, thème vert et noir, visuels de l'installeur et canal de mise à jour propre.
- L'éditeur : détection automatique des cartographies sur Bosch EDC15P, EDC15VM+, EDC16U1, EDC16U31 et EDC16U34 ; tout autre binaire s'ouvre avec un projet WinOLS `.ols`, un `.xdf` TunerPro ou un mappack JSON ; édition en tableau, 2D, 3D et dans l'hexdump ; versions et comparaison ; banc virtuel ; correction de checksum ; export de mappack WinOLS.

## Prévu

- Annuler et rétablir (Ctrl+Z / Ctrl+Y) dans l'éditeur.
- Un chercheur de cartographies pour les binaires sans définitions : des candidates proposées d'après la structure du fichier (axes, points croissants, grilles 8 et 16 bits), nommées et validées à la main.
- Export d'une liste de cartographies en `.xdf` TunerPro, et import/export CSV d'une cartographie.
- Les cartographies de deux versions côte à côte dans la fenêtre de comparaison, différences surlignées.
- Vue 2D façon WinOLS pour les matrices complètes : une courbe par ligne.
- Des checksums au-delà d'EDC15/EDC16, famille par famille une fois vérifiées.
- Cartographies favorites, et une version de référence autre qu'Ori.

## Pas prévu

- Les suppressions liées aux émissions (EGR off, FAP off, AdBlue/SCR off) et les reprogrammations en un clic. Mamba est un éditeur : vous voyez les cartographies et vous décidez de ce que vous modifiez.
- Comptes ou cloud. Tout reste sur votre ordinateur.
