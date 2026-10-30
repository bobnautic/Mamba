// Icônes de l'arbre des maps, dans l'esprit de la liste de WinOLS : dossiers
// pleins à deux tons (dos plus sombre, rabat clair) et une icône par type de
// map (champ, courbe, valeur seule) au lieu d'une icône de fichier unique.
// Dessinées sur une grille de 16 px, traits de 1 px sur des demi-pixels pour
// rester nettes aux tailles de la liste (14 et 16 px).

const FOLDER_DARK = {
  back: "#c98f22",
  front: "#f2c24b",
  edge: "#7a5410",
  shine: "#fbe29a",
};

const FOLDER_LIGHT = {
  back: "#d39a2c",
  front: "#f6cb5a",
  edge: "#8f6414",
  shine: "#fde9ad",
};

/** Dossier fermé ou ouvert (rabat basculé, feuille visible). */
export function TreeFolderIcon({
  open = false,
  light = false,
  className,
}: {
  open?: boolean;
  light?: boolean;
  className?: string;
}) {
  const c = light ? FOLDER_LIGHT : FOLDER_DARK;
  return (
    <svg className={className} viewBox="0 0 16 16" fill="none" strokeLinejoin="round" aria-hidden>
      {/* Dos du dossier avec son onglet */}
      <path
        d="M1.5 3.5a1 1 0 0 1 1-1h3.4l1.6 1.5h6a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1z"
        fill={c.back}
        stroke={c.edge}
      />
      {open ? (
        <>
          {/* Feuille qui dépasse du dossier ouvert */}
          <path d="M3.5 5.5h9v3h-9z" fill="#ffffff" fillOpacity="0.9" />
          {/* Rabat basculé vers l'avant */}
          <path
            d="M3.7 7.5h11.1a.5.5 0 0 1 .47.66l-1.9 5.34H1.5l1.73-5.66a.5.5 0 0 1 .47-.34z"
            fill={c.front}
            stroke={c.edge}
          />
          <path d="M4.2 8.5h10" stroke={c.shine} strokeLinecap="round" />
        </>
      ) : (
        <>
          <path d="M1.5 6.5h13v6.5a.5.5 0 0 1-.5.5H2a.5.5 0 0 1-.5-.5z" fill={c.front} stroke={c.edge} />
          <path d="M2.5 7.5h11" stroke={c.shine} strokeLinecap="round" />
        </>
      )}
    </svg>
  );
}

export type MapKind = "map" | "curve" | "value";

/** Type d'une map d'après ses dimensions : champ (lignes et colonnes),
 *  courbe (une seule ligne ou colonne) ou valeur seule (1×1). Sans
 *  dimensions, une ligne de `size / 2` cellules, comme la lit le MapViewer. */
export function mapKindOf(
  dimensions?: {
    TwoDimensional?: { rows: number; cols: number };
    OneDimensional?: { length: number };
  },
  size?: number,
): MapKind {
  const rows = dimensions?.TwoDimensional?.rows ?? 1;
  const cols =
    dimensions?.TwoDimensional?.cols ??
    dimensions?.OneDimensional?.length ??
    (dimensions ? 1 : Math.max(1, Math.floor((size ?? 0) / 2)));
  if (rows > 1 && cols > 1) return "map";
  if (rows * cols > 1) return "curve";
  return "value";
}

/** Icône du type de map, en currentColor : elle suit la couleur de la ligne
 *  (vert quand la map est modifiée). */
export function MapKindIcon({ kind, className }: { kind: MapKind; className?: string }) {
  return (
    <svg className={className} viewBox="0 0 16 16" fill="none" stroke="currentColor" aria-hidden>
      {kind === "map" && (
        <>
          {/* Tableau avec ses deux axes en tête */}
          <path d="M1.5 2.5h13v11h-13z" fill="currentColor" fillOpacity="0.12" />
          <path d="M1.5 2.5h13v3h-13zM1.5 5.5h3v8h-3z" fill="currentColor" fillOpacity="0.4" stroke="none" />
          <path d="M1.5 2.5h13v11h-13z" />
          <path d="M1.5 5.5h13M4.5 2.5v11" />
          <path d="M9.5 5.5v8M4.5 9.5h10" strokeOpacity="0.55" />
        </>
      )}
      {kind === "curve" && (
        <>
          {/* Axes et courbe */}
          <path d="M2.5 1.5v12h12" strokeLinecap="round" />
          <path d="M4 11.5c3 0 4.5-1.2 6-4.5s2.2-4 4.5-4" strokeWidth="1.5" strokeLinecap="round" />
        </>
      )}
      {kind === "value" && (
        <>
          {/* Une seule cellule */}
          <path d="M2.5 4.5h11v7h-11z" fill="currentColor" fillOpacity="0.12" />
          <path d="M2.5 4.5h11v7h-11z" />
          <path d="M5.5 8h5" strokeWidth="1.5" strokeLinecap="round" />
        </>
      )}
    </svg>
  );
}
