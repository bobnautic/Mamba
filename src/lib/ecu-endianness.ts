/**
 * Single source of truth for ECU byte order in the web app.
 *
 * Big-endian ECUs: Bosch EDC16 family (MPC5xx) and Magneti Marelli MJD6
 * family (PowerPC) — 16-bit values are stored high byte first (eHiLo).
 * Everything else (EDC15 family, unknown) defaults to little-endian (eLoHi).
 *
 * Used by map-viewer.tsx (decode), page.tsx (write-back), compare-modal.tsx
 * and mappack-export.ts — they must all agree, otherwise the display and the
 * bytes written back to the binary diverge.
 */
let projectByteOrder: "hilo" | "lohi" | null = null;
let projectEcuType: string | null = null;

/**
 * Byte order declared by the open project itself (a WinOLS project records
 * how its maps store their values). Set by the editor when a project loads,
 * cleared when it closes; null = decide from the ECU type below.
 */
export function setProjectByteOrder(order: "hilo" | "lohi" | null | undefined): void {
  projectByteOrder = order === "hilo" || order === "lohi" ? order : null;
}

/**
 * ECU type of the open project, so code that has no access to it (the map
 * Properties defaults, for one) can still tell which way the bytes go.
 */
export function setProjectEcuType(ecuType: string | null | undefined): void {
  projectEcuType = ecuType || null;
}

/** Byte order of the open project: its own declaration, else its ECU family. */
export function projectIsBigEndian(): boolean {
  return isBigEndianEcu(projectEcuType);
}

export function isBigEndianEcu(ecuType: string | null | undefined): boolean {
  if (projectByteOrder) return projectByteOrder === "hilo";
  const t = (ecuType || "").toUpperCase();
  return t.includes("EDC16") || t.includes("MJD") || t.includes("MAREL");
}

/**
 * Marelli MJD6 axis values are UNSIGNED u16: RPM axes end with "infinity"
 * sentinels (18750/37500 raw, i.e. > 32767) and temperature axes are stored
 * as positive 0.25°C/bit values — interpreting them as signed i16 (the
 * Bosch behavior) turns the sentinels into bogus negative labels.
 */
export function hasUnsignedAxes(ecuType: string | null | undefined): boolean {
  const t = (ecuType || "").toUpperCase();
  return t.includes("MJD") || t.includes("MAREL");
}
