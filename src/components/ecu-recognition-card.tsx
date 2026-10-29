"use client";

// Identity card of the ECU read from a binary (universal recognition, see
// src-tauri/src/detector/ecu_recognition.rs): manufacturer and family, the
// supplier and vehicle manufacturer numbers, engine, processor, byte order
// and every identification number found, the way StageX lists them next to
// a project. Shared by the New Project screen and the project information
// dialog.

import { useState } from "react";
import { ChevronDown, ChevronUp, Cpu, RefreshCw } from "lucide-react";
import type { EcuRecognition } from "@/lib/types";
import { useI18n } from "@/contexts/i18n-context";
import { formatBytes } from "@/lib/format-bytes";

interface EcuRecognitionCardProps {
  recognition: EcuRecognition;
  sha256?: string;
  light: boolean;
  /** Details (numbers grid) open from the start */
  defaultOpen?: boolean;
  /** Re-read the identity from the stored binary */
  onReread?: () => void;
  rereading?: boolean;
}

export function EcuRecognitionCard({
  recognition: rec,
  sha256,
  light: L,
  defaultOpen = false,
  onReread,
  rereading = false,
}: EcuRecognitionCardProps) {
  const { t, language } = useI18n();
  const id = t.ecuId;
  const [open, setOpen] = useState(defaultOpen);

  const fuelLabel = (fuel?: string | null) =>
    fuel === "Diesel" ? t.library.fuelDiesel : fuel === "Petrol" ? t.library.fuelPetrol : fuel || "";
  const byteOrder = rec.byte_order
    ? `${rec.byte_order === "hilo" ? "HiLo (Motorola)" : "LoHi (Intel)"}${
        rec.byte_order_source === "heuristic" ? ` — ${id.byteOrderEstimated}` : ""
      }`
    : "";

  const rows: [string, string | null | undefined][] = [
    [id.hardwareNumber, rec.hardware_number],
    [id.softwareNumber, rec.software_number],
    [id.partNumber, rec.part_number],
    [id.hardwarePartNumber, rec.hardware_part_number],
    [id.softwareVersion, rec.software_version],
    [id.calibration, rec.calibration_number],
    [id.projectCode, rec.project_code],
    [id.engine, rec.engine],
    [id.engineCode, rec.engine_code],
    [id.vehicleBrand, rec.vehicle_brand],
    [id.fuel, fuelLabel(rec.fuel)],
    [id.processor, rec.processor],
    [id.byteOrder, byteOrder],
    [id.flashSize, rec.flash_size ? formatBytes(rec.flash_size, language) : ""],
    [id.odx, rec.odx_identifier],
    [id.sha256, sha256 ? `${sha256.slice(0, 16)}…` : ""],
  ];
  const filled = rows.filter(([, v]) => !!v);

  const evidenceLabel = (kind: string): string =>
    (id.evidenceKinds as Record<string, string>)[kind] || kind;

  const muted = L ? "text-slate-500" : "text-slate-400";
  const strong = L ? "text-slate-900" : "text-white";
  const pill = L ? "border-black/10 bg-black/[0.04]" : "border-white/10 bg-white/[0.05]";

  return (
    <div className={`rounded-lg border p-4 ${L ? "border-black/10 bg-black/[0.03]" : "border-white/15 bg-black/15"}`}>
      <div className="flex items-start gap-3">
        <Cpu className={`w-6 h-6 flex-shrink-0 mt-0.5 ${rec.detected ? (L ? "text-green-700" : "text-mamba-400") : muted}`} />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <p className={`text-xs uppercase tracking-wider ${muted}`}>{id.title}</p>
            {onReread && (
              <button
                type="button"
                onClick={onReread}
                disabled={rereading}
                title={id.reread}
                className={`ml-auto inline-flex items-center gap-1 text-xs px-2 py-0.5 rounded-md transition-colors disabled:opacity-50 ${
                  L ? "text-slate-600 hover:bg-black/5" : "text-slate-300 hover:bg-white/10"
                }`}
              >
                <RefreshCw className={`w-3 h-3 ${rereading ? "animate-spin" : ""}`} />
                {id.reread}
              </button>
            )}
          </div>
          {rec.detected ? (
            <div className="flex items-center gap-2 flex-wrap mt-1">
              <p className={`font-semibold ${strong}`}>
                {[rec.manufacturer, rec.family].filter(Boolean).join(" ")}
              </p>
              <span className={`text-xs px-2 py-0.5 rounded-full border tabular-nums ${pill} ${muted}`}>
                {id.confidence} {Math.round(rec.confidence * 100)} %
              </span>
              <span
                className={`text-xs px-2 py-0.5 rounded-full border ${
                  rec.detector_available
                    ? L
                      ? "border-green-600/30 bg-green-600/10 text-green-800"
                      : "border-mamba-500/40 bg-mamba-500/10 text-mamba-300"
                    : L
                      ? "border-amber-500/40 bg-amber-500/10 text-amber-800"
                      : "border-amber-500/30 bg-amber-500/10 text-amber-300"
                }`}
              >
                {id.detectorAvailable}: {rec.detector_available ? `${id.detectorYes} (${rec.mamba_ecu_type})` : id.detectorNo}
              </span>
            </div>
          ) : (
            <p className={`mt-1 text-sm ${muted}`}>{id.notRecognized}</p>
          )}

          {filled.length > 0 && (
            <button
              type="button"
              onClick={() => setOpen((o) => !o)}
              className={`mt-2 inline-flex items-center gap-1 text-xs ${L ? "text-slate-600 hover:text-black" : "text-slate-300 hover:text-white"}`}
            >
              {open ? <ChevronUp className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
              {open ? id.hideDetails : id.showDetails}
            </button>
          )}

          {open && filled.length > 0 && (
            <div className="mt-3 space-y-3">
              <dl className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-x-6 gap-y-2">
                {filled.map(([label, value]) => (
                  <div key={label} className="min-w-0">
                    <dt className={`text-[11px] uppercase tracking-wide ${muted}`}>{label}</dt>
                    <dd className={`text-sm font-mono truncate ${strong}`} title={value || ""}>
                      {value}
                    </dd>
                  </div>
                ))}
              </dl>

              {rec.other_numbers.length > 0 && (
                <div>
                  <p className={`text-[11px] uppercase tracking-wide mb-1 ${muted}`}>{id.otherNumbers}</p>
                  <div className="flex flex-wrap gap-1.5">
                    {rec.other_numbers.map((n) => (
                      <span key={n} className={`text-xs font-mono px-2 py-0.5 rounded border select-text ${pill} ${strong}`}>
                        {n}
                      </span>
                    ))}
                  </div>
                </div>
              )}

              {rec.evidence.length > 0 && (
                <div>
                  <p className={`text-[11px] uppercase tracking-wide mb-1 ${muted}`}>{id.evidence}</p>
                  <ul className={`text-xs space-y-0.5 ${muted}`}>
                    {rec.evidence.map((e, i) => (
                      <li key={`${e.kind}-${i}`} className="truncate" title={e.value}>
                        <span className={strong}>{evidenceLabel(e.kind)}</span>
                        {" — "}
                        <span className="font-mono">{e.value}</span>
                        {e.offset > 0 && <span className="font-mono"> @ 0x{e.offset.toString(16).toUpperCase()}</span>}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
