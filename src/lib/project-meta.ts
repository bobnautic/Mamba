// Project library helpers shared by the dashboard, the New Project screen
// and the project information dialog: the choice lists of the metadata
// fields, and how a project is labelled, filtered and searched.

import type { EcuRecognition, FileRecord } from "@/lib/types";

/** Vehicle makes offered in the brand lists (values stored as written). */
export const VEHICLE_BRANDS = [
  "Alfa Romeo",
  "Audi",
  "BMW",
  "Citroën",
  "Cupra",
  "Dacia",
  "Fiat",
  "Ford",
  "Honda",
  "Hyundai",
  "Iveco",
  "Jaguar",
  "Jeep",
  "Kia",
  "Land Rover",
  "Mazda",
  "Mercedes-Benz",
  "Mini",
  "Mitsubishi",
  "Nissan",
  "Opel",
  "Peugeot",
  "Porsche",
  "Renault",
  "Seat",
  "Skoda",
  "Smart",
  "Subaru",
  "Suzuki",
  "Toyota",
  "Volkswagen",
  "Volvo",
];

/** Fuel values as stored; the labels are translated (library.fuel*). */
export const FUEL_VALUES = ["Diesel", "Petrol", "Hybrid", "Electric", "LPG/CNG"] as const;

export const STAGE_VALUES = ["Stage 1", "Stage 2", "Stage 3"];

/** Brand list holding the project's current value even when it is not a listed one. */
export function brandOptions(current?: string): string[] {
  return current && !VEHICLE_BRANDS.includes(current) ? [current, ...VEHICLE_BRANDS] : VEHICLE_BRANDS;
}

/**
 * Vehicle make to pre-fill from the recognition, only when the numbers name
 * exactly one make of the list ("Volkswagen Group" or "Peugeot / Citroën"
 * do not say which brand of the group).
 */
export function brandFromRecognition(rec?: EcuRecognition | null): string {
  const make = rec?.vehicle_brand || "";
  return VEHICLE_BRANDS.includes(make) ? make : "";
}

/** Engine field pre-fill: description and engine code ("R4 2.0l TDI CJCC"). */
export function engineFromRecognition(rec?: EcuRecognition | null): string {
  if (!rec?.engine) return rec?.engine_code || "";
  return rec.engine_code ? `${rec.engine} ${rec.engine_code}` : rec.engine;
}

/** "dpf, egr ,  customer" -> ["dpf", "egr", "customer"] */
export function parseTags(tags?: string | null): string[] {
  return (tags || "")
    .split(",")
    .map((t) => t.trim())
    .filter(Boolean);
}

function isKnownEcuType(ecuType?: string | null): ecuType is string {
  return !!ecuType && ecuType.toLowerCase() !== "unknown";
}

/**
 * ECU shown for a project: the type of its Mamba detector, else the family
 * the recognition read in the binary ("Bosch EDC17C46"), else nothing.
 */
export function projectEcuLabel(file: Pick<FileRecord, "ecu_type" | "ecu_recognition">): string | null {
  if (isKnownEcuType(file.ecu_type)) return file.ecu_type;
  const rec = file.ecu_recognition;
  if (rec?.family) return rec.manufacturer ? `${rec.manufacturer} ${rec.family}` : rec.family;
  return rec?.manufacturer || null;
}

/** True when the label above comes from the recognition, not from a Mamba detector. */
export function ecuLabelIsRecognised(file: Pick<FileRecord, "ecu_type" | "ecu_recognition">): boolean {
  return !isKnownEcuType(file.ecu_type) && !!(file.ecu_recognition?.family || file.ecu_recognition?.manufacturer);
}

/** ECU generation used by the library filter (EDC16, EDC17, MED17…). */
export function projectEcuGroup(file: Pick<FileRecord, "ecu_type" | "ecu_recognition">): string | null {
  const rec = file.ecu_recognition;
  if (rec?.family_group) return rec.family_group;
  if (!isKnownEcuType(file.ecu_type)) return null;
  const m = file.ecu_type.toUpperCase().match(/^[A-Z]+\d*/);
  return m ? m[0] : file.ecu_type;
}

/** Everything the library search looks into, lower-cased. */
export function projectSearchText(file: FileRecord): string {
  const rec = file.ecu_recognition;
  return [
    file.project_name,
    file.original_name,
    file.hardware_version,
    file.software_version,
    file.ecu_type,
    file.vehicle_brand,
    file.vehicle_model,
    file.engine_type,
    file.transmission_type,
    file.customer,
    file.stage,
    file.year,
    file.power,
    file.notes,
    file.fuel,
    file.vin,
    file.plate,
    file.tags,
    file.sha256,
    rec?.manufacturer,
    rec?.family,
    rec?.engine,
    rec?.engine_code,
    rec?.odx_identifier,
    rec?.vehicle_brand,
    ...(rec?.other_numbers || []),
  ]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();
}
