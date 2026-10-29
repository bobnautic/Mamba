"use client";

import { useState, useRef } from "react";
import { PROJECT_NAME_MAX_LENGTH } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Upload, X, FileText, Check, Cpu, AlertCircle, AlertTriangle } from "lucide-react";
import { useToast } from "@/hooks/use-toast";
import { useI18n } from "@/contexts/i18n-context";
import { useSettings } from "@/contexts/settings-context";
import { useRouter } from "next/navigation";
import axios from "axios";
import { MODAL_GLASS, MODAL_GLASS_LIGHT } from "@/lib/modal-glass";
import { useThemeOptional } from "@/contexts/theme-context";
import { identifyEcu, detectMaps, inspectOlsContainer, extractOlsVersion, extractOlsMaps, type OlsVersionInfo, type OlsInspection } from "@/lib/local/detector";
import * as localStore from "@/lib/local/store";
import MambaGradientDefs, { MambaFileIcon } from "@/components/mamba-gradient-defs";
// Listes déroulantes au style de l'app (même composant que la langue des paramètres)
import { StyledSelect } from "@/components/styled-select";
import { lookupEcuBrand } from "@/lib/ecu-brand-db";

interface ProjectCreatorProps {
  onProjectCreated?: (projectId: string) => void;
}

interface ECUIdentification {
  manufacturer: string;
  ecu_type: string;
  variant?: string;
  software_version?: string;
  hardware_version?: string;
  part_number?: string;
  confidence: number;
  /** Projet WinOLS d'un calculateur sans détecteur : les maps viennent du projet */
  ols_maps?: boolean;
}

// Helper function to get stage icon styles based on stage value
function getStageIconStyles(stage?: string, isLight?: boolean) {
  switch (stage) {
    case 'Stage 1':
      return { background: 'bg-green-500/20', border: 'border-green-500/30', numberColor: 'text-green-500', stageNumber: '1' };
    case 'Stage 2':
      return { background: 'bg-yellow-500/20', border: 'border-yellow-500/30', numberColor: 'text-yellow-500', stageNumber: '2' };
    case 'Stage 3':
      return { background: 'bg-red-500/20', border: 'border-red-500/30', numberColor: 'text-red-500', stageNumber: '3' };
    default:
      return { background: isLight ? 'bg-black/[0.04]' : 'bg-slate-500/20', border: isLight ? 'border-black/10' : 'border-slate-500/30', numberColor: null, stageNumber: null };
  }
}

export function ProjectCreator({ onProjectCreated }: ProjectCreatorProps) {
  const router = useRouter();
  const { toast } = useToast();
  const { t } = useI18n();
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [isUploading, setIsUploading] = useState(false);
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [ecuIdentification, setEcuIdentification] = useState<ECUIdentification | null>(null);
  // Pourquoi le calculateur n'est pas pris en charge, quand il ne l'est pas :
  // « not-detected » = le détecteur ne reconnaît pas ce binaire — le projet
  // s'ouvre quand même et l'utilisateur y importe ses propres définitions de
  // maps ; « disabled » = calculateur reconnu mais désactivé dans la base,
  // et là rien ne s'ouvre.
  const [unsupportedReason, setUnsupportedReason] = useState<"not-detected" | "disabled" | null>(null);
  const [versioningInfo, setVersioningInfo] = useState<{
    fileId?: string;
    currentVersionId?: string;
    versions?: any[];
  }>({});
  
  // Cached base64 file data (avoid re-encoding for detect call). Once a .ols container has been
  // resolved to one chosen version, this holds the EXTRACTED raw ROM bytes, never the raw
  // container bytes -- identification, detection and project creation all read from here.
  const fileBase64Ref = useRef<string | null>(null);

  // Set only while `analyzeECUType` is waiting on the user to pick which saved version of a
  // multi-version `.ols` file to use; identification is deferred until then. `null` the rest of
  // the time, including for a raw dump or a single-version `.ols` file (auto-picked, see below).
  const [pendingOlsChoice, setPendingOlsChoice] = useState<{
    fileDataBase64: string;
    fileName: string;
    versions: OlsVersionInfo[];
  } | null>(null);
  const [isExtractingOlsVersion, setIsExtractingOlsVersion] = useState(false);

  // Projet WinOLS (.ols) derrière le fichier choisi : le conteneur, ce qu'il
  // dit de lui-même (calculateur, numéros HW/SW, maps, versions) et la version
  // retenue comme original.
  const [olsProject, setOlsProject] = useState<{
    containerBase64: string;
    info: OlsInspection;
    chosenIndex: number;
  } | null>(null);
  // Projet à plusieurs versions : importer aussi les autres comme versions du projet
  const [importAllVersions, setImportAllVersions] = useState(true);

  /** Marque du projet WinOLS ramenée à une entrée de la liste, sinon rien */
  const brandFromOls = (make: string): string => {
    const m = make.trim().toLowerCase();
    return ["Audi", "Seat", "Skoda", "Volkswagen"].find((b) => b.toLowerCase() === m) || "";
  };

  // Form state
  const [projectName, setProjectName] = useState("");
  const [vehicleBrand, setVehicleBrand] = useState("");
  const [vehicleModel, setVehicleModel] = useState("");
  const [engineType, setEngineType] = useState("");
  const [transmissionType, setTransmissionType] = useState("");
  const [year, setYear] = useState("");
  const [power, setPower] = useState("");
  const [customer, setCustomer] = useState("");
  const [stage, setStage] = useState("");
  const [date, setDate] = useState(new Date().toISOString().split('T')[0]);
  const [notes, setNotes] = useState("");

  const { platform } = useSettings();
  // Suit le thème de l'écran hôte
  const themeCtx = useThemeOptional();
  const L = (themeCtx?.theme ?? "default") === "light";
  const labelCls = `block text-sm font-medium mb-2 ${L ? 'text-slate-900' : 'text-white'}`;
  const inputCls = `w-full px-3 py-2 rounded-lg focus:outline-none focus:ring-0 ${L ? 'bg-black/[0.05] border border-black/20 text-slate-900 placeholder:text-black/40' : 'bg-black/15 border border-white/20 text-white placeholder:text-white/50'}`;
  const inputSmCls = `w-full px-2 py-2 rounded-lg text-sm focus:outline-none focus:ring-0 ${L ? 'bg-black/[0.05] border border-black/20 text-slate-900 placeholder:text-black/40' : 'bg-black/15 border border-white/20 text-white placeholder:text-white/50'}`;

  const maxFileSize = platform.maxFileSizeMB * 1024 * 1024;

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      if (file.size > maxFileSize) {
        toast({
          title: t.errors.fileTooLarge,
          description: t.errors.fileTooLargeDescription,
          variant: "destructive",
        });
        return;
      }
      setSelectedFile(file);
      setEcuIdentification(null);
      
      // Auto-fill project name from filename
      if (!projectName) {
        setProjectName(file.name.replace(/\.[^/.]+$/, "").slice(0, PROJECT_NAME_MAX_LENGTH));
      }

      // Automatically analyze the file to detect ECU type
      await analyzeECUType(file);
    }
  };

  const analyzeECUType = async (file: File) => {
    setIsAnalyzing(true);
    try {
      // Read file and encode as base64 using chunked approach (fast for large files)
      const arrayBuffer = await file.arrayBuffer();
      const uint8Array = new Uint8Array(arrayBuffer);
      const chunks: string[] = [];
      const chunkSize = 8192;
      for (let i = 0; i < uint8Array.length; i += chunkSize) {
        chunks.push(String.fromCharCode(...uint8Array.subarray(i, i + chunkSize)));
      }
      const fileDataBase64 = btoa(chunks.join(''));

      // A .ols file is a WinOLS PROJECT (metadata header + one or more saved ROM versions), not
      // a raw dump -- identification/detection need the actual ROM bytes of ONE chosen version,
      // never the container's own bytes. `inspectOlsContainer` returns null for a plain raw
      // dump (or any file it doesn't recognise), so that path is entirely unaffected below.
      const olsInfo = await inspectOlsContainer(fileDataBase64);
      setOlsProject(olsInfo ? { containerBase64: fileDataBase64, info: olsInfo, chosenIndex: 0 } : null);
      if (olsInfo && olsInfo.versions.length > 1) {
        // Ambiguous: defer identification until the user picks which saved version to use.
        setPendingOlsChoice({ fileDataBase64, fileName: file.name, versions: olsInfo.versions });
        setIsAnalyzing(false);
        return;
      }
      const resolvedBase64 = olsInfo
        ? await extractOlsVersion(fileDataBase64, olsInfo.versions[0].index)
        : fileDataBase64;

      await identifyAndSetState(resolvedBase64, file.name, olsInfo);
    } catch (error: any) {
      toast({
        title: t.errors.ecuIdentificationFailed,
        description: t.errors.ecuIdentificationFailedDescription,
        variant: "destructive",
      });
      setEcuIdentification({ manufacturer: "Unknown", ecu_type: "Unknown", confidence: 0 });
      setUnsupportedReason("not-detected");
      setIsAnalyzing(false);
    }
  };

  /** Once a .ols file's version ambiguity (if any) is resolved, this is the actual identification
   *  step -- shared by the direct (raw dump / single-version .ols) path above and by
   *  `chooseOlsVersion` below, so both run the exact same disabled-ECU check and error handling. */
  const identifyAndSetState = async (fileDataBase64: string, fileName: string, ols: OlsInspection | null = null) => {
    setIsAnalyzing(true);
    setUnsupportedReason(null);
    try {
      fileBase64Ref.current = fileDataBase64; // Cache for later use (detection + project creation)

      // Identify the ECU type via the embedded Rust detection engine
      const identResult = await identifyEcu(fileDataBase64, fileName);

      // Pris en charge = identifié ET activé dans la base des calculateurs
      const identified = !!(identResult?.ecu_type && identResult.ecu_type !== "Unknown");
      let supported = identified;
      if (identified) {
        try {
          const statusRes = await fetch(`/api/ecu-status?ecu_type=${encodeURIComponent(identResult.ecu_type)}`);
          if (statusRes.ok) {
            const { enabled } = await statusRes.json();
            if (!enabled) supported = false;
          }
        } catch {}
      }

      if (!supported && ols && ols.maps_count > 0) {
        // Projet WinOLS d'un calculateur sans détecteur : le projet s'ouvre sur
        // les maps que son auteur a définies (voir extractOlsMaps), avec les
        // numéros notés dans le projet.
        setEcuIdentification({
          manufacturer: ols.manufacturer || "WinOLS",
          ecu_type: ols.ecu_name || t.upload?.olsProject || "WinOLS project",
          hardware_version: ols.hw_number || undefined,
          software_version: ols.sw_number || undefined,
          confidence: 1,
          ols_maps: true,
        });
        if (ols.model) setVehicleModel((prev) => prev || ols.model);
        setVehicleBrand((prev) => prev || brandFromOls(ols.make));
        return;
      }
      if (!supported) {
        if (identified) {
          toast({
            title: t.errors.ecuIdentificationFailed,
            description: `ECU type "${identResult.ecu_type}" is currently disabled.`,
            variant: "destructive",
          });
        }
        // Le projet WinOLS porte souvent la marque, le modèle et les
        // numéros que son auteur a saisis, même quand il ne définit
        // aucune map : on les garde, c'est tout ce qu'on sait du fichier.
        setEcuIdentification({
          manufacturer: "Unknown",
          ecu_type: "Unknown",
          hardware_version: ols?.hw_number || undefined,
          software_version: ols?.sw_number || undefined,
          confidence: 0,
        });
        if (ols?.model) setVehicleModel((prev) => prev || ols.model);
        if (ols?.make) setVehicleBrand((prev) => prev || brandFromOls(ols.make));
        setUnsupportedReason(identified ? "disabled" : "not-detected");
        return;
      }

      // Le projet WinOLS peut porter les numéros que le détecteur n'a pas trouvés
      if (ols) {
        if (!identResult.hardware_version && ols.hw_number) identResult.hardware_version = ols.hw_number;
        if (!identResult.software_version && ols.sw_number) identResult.software_version = ols.sw_number;
        if (ols.model) setVehicleModel((prev) => prev || ols.model);
        setVehicleBrand((prev) => prev || brandFromOls(ols.make));
      }
      setEcuIdentification(identResult);

      // Auto-fill the vehicle brand from the embedded ECU reference database
      // (Bosch 0281xxx numbers, VAG 038906019xx / 03G906016xx refs). Never
      // overwrites a brand the user already picked.
      const brandInfo = lookupEcuBrand(
        identResult?.hardware_version,
        identResult?.software_version,
        identResult?.part_number
      );
      if (brandInfo) {
        setVehicleBrand((prev) => prev || brandInfo.b);
      }
    } catch (error: any) {
      toast({
        title: t.errors.ecuIdentificationFailed,
        description: t.errors.ecuIdentificationFailedDescription,
        variant: "destructive",
      });

      // Set unknown ECU type
      setEcuIdentification({
        manufacturer: "Unknown",
        ecu_type: "Unknown",
        confidence: 0,
      });
      setUnsupportedReason("not-detected");
    } finally {
      setIsAnalyzing(false);
    }
  };

  const handleRemoveFile = () => {
    setSelectedFile(null);
    setEcuIdentification(null);
    setUnsupportedReason(null);
    setPendingOlsChoice(null);
    setOlsProject(null);
  };

  /** User picked which saved version of a multi-version `.ols` file to use (see
   *  `pendingOlsChoice`) -- extract just that version's raw ROM bytes and run the normal
   *  identification flow on them. */
  const chooseOlsVersion = async (versionIndex: number) => {
    if (!pendingOlsChoice) return;
    setIsExtractingOlsVersion(true);
    try {
      const extractedBase64 = await extractOlsVersion(pendingOlsChoice.fileDataBase64, versionIndex);
      const fileName = pendingOlsChoice.fileName;
      setPendingOlsChoice(null);
      setOlsProject((p) => (p ? { ...p, chosenIndex: versionIndex } : p));
      await identifyAndSetState(extractedBase64, fileName, olsProject?.info ?? null);
    } catch (error: any) {
      toast({
        title: t.errors.ecuIdentificationFailed,
        description: t.errors.ecuIdentificationFailedDescription,
        variant: "destructive",
      });
      setPendingOlsChoice(null);
    } finally {
      setIsExtractingOlsVersion(false);
    }
  };

  /** Binaire que le détecteur ne reconnaît pas : le projet s'ouvre quand
   *  même, vide, et l'utilisateur y importe ensuite ses définitions de maps.
   *  Vaut aussi pour un projet WinOLS qui ne définit aucune map : il apporte
   *  au moins sa ROM et ses versions. Seul un calculateur reconnu mais
   *  désactivé dans la base reste bloqué. */
  const canCreateWithoutDetection = unsupportedReason === "not-detected";

  /** Projet WinOLS ouvert alors qu'il ne porte aucune définition de map :
   *  le message le dit, plutôt que de parler d'un fichier non détecté. */
  const olsWithoutMaps = !!olsProject && olsProject.info.maps_count === 0;

  /** Chemins encore en bêta : un projet WinOLS, ou un binaire que le
   *  détecteur ne reconnaît pas et qui attend un fichier de définitions.
   *  Un binaire reconnu suit le chemin habituel et n'affiche rien. */
  const betaPath = !!olsProject || ecuIdentification?.ecu_type === "Unknown";

  const handleCreateProject = async () => {
    if (!selectedFile) {
      toast({
        title: t.errors.noFileSelected,
        description: t.errors.noFileSelectedDescription,
      });
      return;
    }

    // Block if ECU is disabled (a WinOLS project of an unsupported ECU is allowed: its maps come from the project)
    if (!ecuIdentification?.ols_maps && ecuIdentification?.ecu_type && ecuIdentification.ecu_type !== "Unknown") {
      try {
        const statusRes = await fetch(`/api/ecu-status?ecu_type=${encodeURIComponent(ecuIdentification.ecu_type)}`);
        if (statusRes.ok) {
          const { enabled } = await statusRes.json();
          if (!enabled) {
            toast({
              title: "ECU Disabled",
              description: `ECU type "${ecuIdentification.ecu_type}" is currently disabled.`,
              variant: "destructive",
            });
            return;
          }
        }
      } catch {}
    }

    if (!projectName.trim()) {
      toast({
        title: t.errors.projectNameRequired,
        description: t.errors.projectNameRequiredDescription,
      });
      return;
    }

    setIsUploading(true);

    try {
      // Use cached base64 from identification step, or encode now
      let fileDataBase64 = fileBase64Ref.current;
      if (!fileDataBase64) {
        const arrayBuffer = await selectedFile.arrayBuffer();
        const uint8Array = new Uint8Array(arrayBuffer);
        const chunks: string[] = [];
        const chunkSize = 8192;
        for (let i = 0; i < uint8Array.length; i += chunkSize) {
          chunks.push(String.fromCharCode(...uint8Array.subarray(i, i + chunkSize)));
        }
        fileDataBase64 = btoa(chunks.join(''));
      }

      // Maps du projet : celles du détecteur Mamba, celles du projet WinOLS,
      // ou les deux. Un calculateur pris en charge garde TOUT son comportement
      // habituel (solutions, codes défaut, puissance, complétude) ; les maps du
      // projet WinOLS viennent en plus, dans leur propre mappack.
      const olsHasMaps = !!olsProject && olsProject.info.maps_count > 0;
      // Binaire que le détecteur ne reconnaît pas : il n'y a rien à
      // détecter. Le projet s'ouvre vide, et l'utilisateur y importe
      // ensuite ses définitions de maps (.xdf ou mappack .json).
      const notDetected = unsupportedReason === "not-detected" && !ecuIdentification?.ols_maps;
      let detectionResults;
      if (notDetected) {
        detectionResults = {
          success: true,
          maps: [],
          total_maps: 0,
          processing_time_ms: 0,
          file_size: selectedFile.size,
        };
      } else if (ecuIdentification?.ols_maps && olsProject) {
        detectionResults = await extractOlsMaps(olsProject.containerBase64);
      } else {
        detectionResults = await detectMaps({
          fileDataBase64,
          fileName: selectedFile.name,
          ecuType: ecuIdentification?.ecu_type || "unknown",
        });
        if (olsHasMaps && olsProject) {
          const fromOls = await extractOlsMaps(olsProject.containerBase64);
          detectionResults = {
            ...detectionResults,
            maps: [...(detectionResults.maps || []), ...(fromOls.maps || [])],
            total_maps: (detectionResults.maps?.length || 0) + (fromOls.maps?.length || 0),
          };
        }
      }
      const response = { data: detectionResults };
      // Clear any stale project data BEFORE attempting versioning
      // This prevents navigating to editor with stale data if versioning fails
      if (typeof window !== 'undefined') {
        sessionStorage.removeItem("currentProject");
        localStorage.removeItem("currentProject");
      }

      // Initialisation du versioning (création de l'Ori)
      let versioningData: { fileId?: string; currentVersionId?: string; versions?: any[] } = {};
      try {
        const versioning = await axios.post("/api/versioning/init", {
          projectName,
          fileName: selectedFile.name,
          fileData: fileDataBase64, // Base64 encoded binary data for PocketBase storage
          fileSize: selectedFile.size,
          ecuType: ecuIdentification?.ecu_type || "unknown",
          hardwareVersion: ecuIdentification?.hardware_version,
          softwareVersion: ecuIdentification?.software_version,
          detectionResults: response.data,
          mapsSource: notDetected ? "imported" : (ecuIdentification?.ols_maps ? "ols" : (olsHasMaps ? "both" : "detector")),
          byteOrder: olsProject?.info.byte_order || undefined,
          olsEcuName: olsProject?.info.ecu_name || undefined,
          vehicleBrand,
          vehicleModel,
          engineType,
          transmissionType,
          year,
          power,
          customer,
          stage,
          date,
          notes,
        }, { timeout: 120000 }); // 2 minutes timeout for large files
        versioningData = {
          fileId: versioning.data.fileId,
          currentVersionId: versioning.data.currentVersionId,
          versions: versioning.data.versions || [],
        };

        // Projet WinOLS à plusieurs versions : les autres versions enregistrées
        // deviennent des versions du projet (copie complète, comme un fichier
        // importé depuis l'éditeur), l'original restant la version choisie.
        if (olsProject && importAllVersions && olsProject.info.versions.length > 1 && versioningData.fileId) {
          const chosen = olsProject.info.versions.find((v) => v.index === olsProject.chosenIndex);
          for (const v of olsProject.info.versions) {
            if (v.index === olsProject.chosenIndex) continue;
            const name = (v.label || `${t.upload?.olsVersion || "Version"} ${v.index}`).slice(0, 100);
            if (chosen && v.size !== chosen.size) {
              toast({ title: t.errors.importError, description: (t.upload?.olsVersionSkipped || "Version {name} skipped").replace("{name}", name) });
              continue;
            }
            try {
              const b64 = await extractOlsVersion(olsProject.containerBase64, v.index);
              const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
              const created = await axios.post("/api/versioning/versions", {
                fileId: versioningData.fileId,
                name,
                baseVersionId: versioningData.currentVersionId,
                setCurrent: false,
              });
              const versionId = created.data?.version?.id;
              if (versionId) {
                await localStore.writeVersionBinary(versionId, bytes);
                versioningData.versions = [...(versioningData.versions || []), created.data.version];
              }
            } catch {
              toast({ title: t.errors.importError, description: (t.upload?.olsVersionSkipped || "Version {name} skipped").replace("{name}", name), variant: "destructive" });
            }
          }
        }
        setVersioningInfo(versioningData);
      } catch (err: any) {
        // Versioning failed - cannot proceed without stored file data.
        // Surface the underlying error: in a local app the real cause
        // (fs permission, disk full...) is actionable for the user.
        const detail = err?.response?.data?.error || err?.message || "";
        toast({
          title: t.errors.creationError,
          description: `${t.errors.creationErrorDescription}${detail ? ` — ${detail}` : ""}`,
          variant: "destructive",
        });
        setIsUploading(false);
        return; // Stop the process, don't navigate to editor without file data
      }

      // Navigate to editor page with project data
      // CRITICAL: Clear all viewMode storage when creating a new project
      // This ensures maps always open in "text" view by default
      if (typeof window !== 'undefined') {
        // Clear all viewMode entries from sessionStorage
        const keysToRemove: string[] = [];
        for (let i = 0; i < sessionStorage.length; i++) {
          const key = sessionStorage.key(i);
          if (key && key.startsWith('viewMode_')) {
            keysToRemove.push(key);
          }
        }
        keysToRemove.forEach(key => sessionStorage.removeItem(key));
      }
      
      // Store project metadata in sessionStorage (WITHOUT file_data to avoid quota exceeded)
      // The actual binary data is stored in PocketBase via versioning
      const projectMetadata = {
        file_name: selectedFile.name,
        original_name: selectedFile.name,
        file_size: selectedFile.size,
        ecu_type: ecuIdentification?.ecu_type || "unknown",
        hardware_version: ecuIdentification?.hardware_version,
        software_version: ecuIdentification?.software_version,
        maps_source: notDetected ? "imported" : (ecuIdentification?.ols_maps ? "ols" : (olsHasMaps ? "both" : "detector")),
        byte_order: olsProject?.info.byte_order || undefined,
        project_name: projectName,
        vehicle_brand: vehicleBrand,
        vehicle_model: vehicleModel,
        engine_type: engineType,
        transmission_type: transmissionType,
        year: year,
        power: power,
        customer: customer,
        stage: stage,
        date: date,
        notes: notes,
        created: new Date().toISOString(),
        detectionResults: response.data,
        ecuIdentification: ecuIdentification,
        fileId: versioningData.fileId,
        currentVersionId: versioningData.currentVersionId,
        versions: versioningData.versions || [],
        // Note: file_data is NOT stored here - it's in PocketBase
      };
      sessionStorage.setItem("currentProject", JSON.stringify(projectMetadata));

      // Navigate to editor - don't call onProjectCreated (it closes the modal
      // and shows the dashboard during the navigation transition)
      router.push(`/editor?project=${encodeURIComponent(projectName)}`);
    } catch (error: any) {
      const detail = error?.message || String(error ?? "");
      toast({
        title: t.errors.uploadFailed,
        description: `${t.errors.uploadFailedDescription}${detail ? ` — ${detail}` : ""}`,
        variant: "destructive",
      });
    } finally {
      setIsUploading(false);
    }
  };

  return (
    <div className="max-w-4xl mx-auto">
      <MambaGradientDefs />
      <div className="border rounded-lg p-8" style={L ? MODAL_GLASS_LIGHT : MODAL_GLASS}>
        <h2 className={`text-2xl font-bold mb-6 ${L ? "text-slate-900" : "text-white"}`}>{t.upload?.title || "Create New Project"}</h2>

        <div className="space-y-6">
          {/* File Upload Section */}
          <div>
            <label className={labelCls}>{t.upload?.ecuFile || "ECU File"} *</label>
            {!selectedFile ? (
              <label
                className="flex flex-col items-center justify-center w-full h-32 rounded-lg cursor-pointer upload-zone-hover transition-colors"
                style={{
                  // Pointillés en dégradé du logo (rect SVG — les bordures CSS
                  // n'acceptent pas de dégradé) + remplissage translucide du
                  // même dégradé, sur verre flouté
                  // Pointillés : noirs sur le thème clair, dégradé du logo en sombre
                  backgroundImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg'%3E%3Cdefs%3E%3ClinearGradient id='g' x1='0%25' y1='0%25' x2='100%25' y2='0%25'%3E%3Cstop offset='0%25' stop-color='%2318863a'/%3E%3Cstop offset='50%25' stop-color='%232fcb46'/%3E%3Cstop offset='100%25' stop-color='%23a6d516'/%3E%3C/linearGradient%3E%3C/defs%3E%3Crect width='100%25' height='100%25' fill='none' rx='8' ry='8' stroke='${L ? "%23000000" : "%23ffffff"}' stroke-width='4' stroke-dasharray='8 8'/%3E%3C/svg%3E"), linear-gradient(90deg, rgba(24, 134, 58, 0.12), rgba(47, 203, 70, 0.12), rgba(166, 213, 22, 0.12))`,
                  backdropFilter: 'blur(8px) saturate(130%)',
                  WebkitBackdropFilter: 'blur(8px) saturate(130%)',
                }}
              >
                <div className="flex flex-col items-center justify-center pt-5 pb-6">
                  <Upload className="w-10 h-10 mb-3 text-muted-foreground" />
                  <p className="text-sm text-muted-foreground">
                    <span className="font-semibold">{t.upload?.clickToUpload || "Click to upload"}</span>
                  </p>
                </div>
                {/* Pas d'attribut accept : la boîte de dialogue Windows s'ouvre
                    sur "Tous les fichiers" au lieu d'un filtre personnalisé */}
                <input
                  type="file"
                  className="hidden"
                  onChange={handleFileSelect}
                />
              </label>
            ) : (
              <div className="space-y-3">
                <div className="flex items-center justify-between p-2 border rounded-lg" style={{ backgroundColor: L ? 'rgba(0, 0, 0, 0.05)' : 'hsla(0, 0%, 55.7%, 0.12)' }}>
                  <div className="flex items-center gap-3 flex-1 min-w-0">
                    {(() => {
                      const iconStyles = getStageIconStyles(stage, L);
                      return (
                        <div className={`w-10 h-10 shrink-0 rounded-lg ${iconStyles.background} flex items-center justify-center border ${iconStyles.border}`}>
                          {iconStyles.stageNumber ? (
                            <span className="text-base font-bold" style={{ fontStyle: 'italic' }}>
                              <span className={L ? "text-slate-900" : "text-white"}>ST</span>
                              <span className={iconStyles.numberColor}>{iconStyles.stageNumber}</span>
                            </span>
                          ) : (
                            <MambaFileIcon className="w-5 h-5" barColor={L ? "#334155" : "#ffffff"} />
                          )}
                        </div>
                      );
                    })()}
                    <div className="min-w-0">
                      <p className={`font-medium truncate ${L ? "text-slate-900" : "text-white"}`} title={selectedFile.name}>{selectedFile.name}</p>
                      <p className="text-sm text-muted-foreground">
                        {(selectedFile.size / 1024).toFixed(2)} KB
                      </p>
                    </div>
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={handleRemoveFile}
                    disabled={isUploading || isAnalyzing}
                    className="shrink-0 ml-2 hover:bg-slate-500/20 text-slate-400 hover:text-white"
                  >
                    <X className="w-4 h-4" />
                  </Button>
                </div>

                {/* .ols version picker -- shown instead of the identification block while a
                    multi-version WinOLS project is waiting on the user to pick one saved
                    version (see `pendingOlsChoice`); identification only starts afterwards. */}
                {pendingOlsChoice ? (
                  <div className={`p-4 border rounded-lg ${L ? "bg-black/[0.03] border-black/10" : "bg-black/15 border-white/20"}`}>
                    <p className={`text-sm font-medium mb-3 ${L ? "text-slate-900" : "text-white"}`}>
                      {t.upload?.olsPickVersion || "This WinOLS project has several saved versions — pick one to import:"}
                    </p>
                    <div className="space-y-2">
                      {pendingOlsChoice.versions.map((v) => (
                        <button
                          key={v.index}
                          type="button"
                          disabled={isExtractingOlsVersion}
                          onClick={() => chooseOlsVersion(v.index)}
                          className={`w-full text-left px-3 py-2 rounded-lg border transition-colors disabled:opacity-50 ${
                            L
                              ? "bg-white/60 border-black/10 hover:bg-white text-slate-900"
                              : "bg-black/20 border-white/10 hover:bg-black/30 text-white"
                          }`}
                        >
                          <span className="font-medium">
                            {v.label || `${t.upload?.olsVersion || "Version"} ${v.index}`}
                          </span>
                          <span className={`ml-2 text-sm ${L ? "text-slate-500" : "text-slate-400"}`}>
                            ({(v.size / 1024).toFixed(0)} KB)
                          </span>
                        </button>
                      ))}
                    </div>
                    <label className={`flex items-center gap-2 mt-3 text-sm cursor-pointer ${L ? "text-slate-700" : "text-slate-300"}`}>
                      <input
                        type="checkbox"
                        checked={importAllVersions}
                        onChange={(e) => setImportAllVersions(e.target.checked)}
                        disabled={isExtractingOlsVersion}
                        className="accent-mamba-500"
                      />
                      <span>{t.upload?.olsImportAllVersions || "Also import the other saved versions as versions of the project"}</span>
                    </label>
                    {isExtractingOlsVersion && (
                      <div className="flex items-center gap-2 mt-3">
                        <div className={`loader loader-sm${L ? " loader-light" : ""}`} />
                        <p className={`text-sm ${L ? "text-slate-500" : "text-slate-400"}`}>
                          {t.upload?.analyzingEcu || "Analyzing ECU file..."}
                        </p>
                      </div>
                    )}
                  </div>
                ) : null}

                {/* ECU Identification Display */}
                {isAnalyzing ? (
                  <div className="flex items-center gap-3 p-4 border rounded-lg bg-red-500/10 border-red-500/30">
                    <div className={`loader loader-sm${L ? " loader-light" : ""}`} />
                    <p className="font-medium text-white">{t.upload?.analyzingEcu || "Analyzing ECU file..."}</p>
                  </div>
                ) : ecuIdentification ? (
                  ecuIdentification.ecu_type === "Unknown" ? (
                    canCreateWithoutDetection ? (
                      // Binaire non reconnu : le projet s'ouvre quand même, sans
                      // maps, et l'utilisateur y importe ses propres définitions.
                      <div className="p-4 border rounded-lg bg-amber-500/10 border-amber-500/30">
                        <div className="flex items-start gap-3">
                          <AlertCircle className={`w-6 h-6 flex-shrink-0 ${L ? "text-amber-600" : "text-amber-400"}`} />
                          <div className="flex-1">
                            {/* Titre et numéros sur la même ligne, à la même
                                place que sur un fichier reconnu : le
                                détecteur n'a rien trouvé, mais le projet
                                WinOLS porte souvent ces numéros. */}
                            <div className="flex items-center justify-between flex-1 gap-6">
                              <p className={`font-semibold leading-none ${L ? "text-slate-900" : "text-white"}`}>
                                {olsWithoutMaps
                                  ? (t.upload?.olsNoMaps || "ECU not supported and no map defined in the WinOLS project.")
                                  : (t.upload?.ecuNotDetected || "ECU not detected.")}
                              </p>
                              <div className="flex items-center gap-6 flex-1 justify-center">
                                {ecuIdentification.hardware_version && (
                                  <div className={`flex items-center gap-2 text-sm leading-none ${L ? "text-slate-500" : "text-slate-400"}`}>
                                    <span className="font-medium">HW:</span>
                                    <span className="font-medium">{ecuIdentification.hardware_version}</span>
                                  </div>
                                )}
                                {ecuIdentification.software_version && (
                                  <div className={`flex items-center gap-2 text-sm leading-none ${L ? "text-slate-500" : "text-slate-400"}`}>
                                    <span className="font-medium">SW:</span>
                                    <span className="font-medium">{ecuIdentification.software_version}</span>
                                  </div>
                                )}
                              </div>
                            </div>
                            <p className={`mt-2 text-sm ${L ? "text-amber-700" : "text-amber-300"}`}>
                              {t.upload?.ecuNotDetectedImport || "Mamba does not detect this file yet. You can still create the project, then import a map definition file (.xdf or .json) for this binary."}
                            </p>
                          </div>
                        </div>
                      </div>
                    ) : (
                    <div className="p-4 border rounded-lg bg-red-500/10 border-red-500/30">
                      <div className="flex items-center gap-3">
                        <AlertCircle className="w-6 h-6 flex-shrink-0 text-red-400" />
                        <p className="font-semibold leading-none text-white text-center flex-1">
                          {olsProject && olsProject.info.maps_count === 0
                            ? (t.upload?.olsNoMaps || "ECU not supported and no map defined in the WinOLS project.")
                            : (t.upload?.ecuNotDetected || "ECU not detected. Check that the file is the correct size and is not encrypted.")}
                        </p>
                      </div>
                    </div>
                    )
                  ) : (
                    <div className={`p-4 border rounded-lg ${L ? "bg-green-600/10 border-green-600/30" : "bg-green-500/10 border-white-500/30"}`}>
                      <div className="flex items-center gap-3">
                        <Cpu className={`w-6 h-6 flex-shrink-0 ${L ? "text-green-700" : "text-white"}`} />
                        <div className="flex items-center justify-between flex-1 gap-6">
                          <p className={`font-semibold leading-none ${L ? "text-slate-900" : "text-white"}`}>
                            {ecuIdentification.manufacturer} {ecuIdentification.ecu_type}
                          </p>
                          <div className="flex items-center gap-6 flex-1 justify-center">
                            {ecuIdentification.hardware_version && (
                              <div className={`flex items-center gap-2 text-sm leading-none ${L ? "text-slate-500" : "text-slate-400"}`}>
                                <span className="font-medium">HW:</span>
                                <span className="font-medium">{ecuIdentification.hardware_version}</span>
                              </div>
                            )}
                            {ecuIdentification.software_version && (
                              <div className={`flex items-center gap-2 text-sm leading-none ${L ? "text-slate-500" : "text-slate-400"}`}>
                                <span className="font-medium">SW:</span>
                                <span className="font-medium">{ecuIdentification.software_version}</span>
                              </div>
                            )}
                          </div>
                        </div>
                      </div>
                      {olsProject && (
                        <div className={`mt-3 pt-3 border-t text-sm ${L ? "border-black/10 text-slate-600" : "border-white/10 text-slate-300"}`}>
                          <p>
                            <span className="font-medium">{t.upload?.olsProject || "WinOLS project"}</span>
                            {" — "}
                            {(t.upload?.olsMapsCount || "{count} map(s) defined in the WinOLS project").replace("{count}", String(olsProject.info.maps_count))}
                          </p>
                          {ecuIdentification.ols_maps && (
                            <p className={`mt-1 ${L ? "text-amber-700" : "text-amber-300"}`}>
                              {t.upload?.olsUnsupportedEcu || "Mamba has no detector for this ECU: the maps shown are the ones defined in the WinOLS project. Solutions, fault codes and the power estimate are not available on it."}
                            </p>
                          )}
                        </div>
                      )}
                    </div>
                  )
                ) : null}

                {/* Import de projet WinOLS et import de définitions de maps :
                    encore en bêta, on le dit avant de créer le projet. */}
                {!isAnalyzing && ecuIdentification && betaPath && (
                  <div className={`mt-3 p-3 border rounded-lg flex items-start gap-3 ${
                    L ? "bg-amber-500/10 border-amber-500/40" : "bg-amber-500/10 border-amber-500/30"
                  }`}>
                    <AlertTriangle className={`w-5 h-5 flex-shrink-0 mt-0.5 ${L ? "text-amber-600" : "text-amber-400"}`} />
                    <p className={`text-sm ${L ? "text-amber-800" : "text-amber-200"}`}>
                      <span className="font-semibold">{t.upload?.betaTitle || "Beta"}</span>
                      {" — "}
                      {t.upload?.betaImportNotice ||
                        "Reading WinOLS projects and imported map definitions is still beta: expect files it does not read yet, maps or axes it reads differently, and bugs. A lot of work is left on it. Report what you hit, with the file if you can."}
                    </p>
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Project Information */}
          <div>
            <label className={labelCls}>{t.upload?.projectName || "Project Name"} *</label>
            <input
              type="text"
              value={projectName}
              maxLength={PROJECT_NAME_MAX_LENGTH}
              onChange={(e) => setProjectName(e.target.value.slice(0, PROJECT_NAME_MAX_LENGTH))}
              className={inputCls}
              placeholder=""
              disabled={isUploading}
              spellCheck={false}
            />
          </div>

          {/* Vehicle Information */}
          <div className="border-t pt-6">
            <h3 className={`text-lg font-semibold mb-4 ${L ? "text-slate-900" : "text-white"}`}>{t.upload?.vehicleInfo || "Vehicle Information (Optional)"}</h3>
            <div className="grid md:grid-cols-3 gap-3">
              {/* Première ligne: Brand - Model - Year */}
              <div>
                <label className={labelCls}>{t.upload?.brand || "Brand"}</label>
                <StyledSelect
                  value={vehicleBrand}
                  onChange={setVehicleBrand}
                  className="w-full"
                  disabled={isUploading}
                  options={[
                    { value: "", label: t.upload?.select || "Select..." },
                    { value: "Audi", label: "Audi" },
                    { value: "Seat", label: "Seat" },
                    { value: "Skoda", label: "Skoda" },
                    { value: "Volkswagen", label: "Volkswagen" },
                  ]}
                />
              </div>

              <div>
                <label className={labelCls}>{t.upload?.model || "Model"}</label>
                <input
                  type="text"
                  value={vehicleModel}
                  onChange={(e) => setVehicleModel(e.target.value)}
                  className={inputSmCls}
                  placeholder=""
                  disabled={isUploading}
                  spellCheck={false}
                />
              </div>

              <div>
                <label className={labelCls}>{t.upload?.year || "Year"}</label>
                <StyledSelect
                  value={year}
                  onChange={setYear}
                  className="w-full"
                  disabled={isUploading}
                  options={[
                    { value: "", label: t.upload?.select || "Select..." },
                    // Années des EDC15/EDC16 supportés : 1999 à 2009, la plus récente en tête
                    ...Array.from({ length: 11 }, (_, i) => 2009 - i).map(y => ({ value: String(y), label: String(y) })),
                  ]}
                />
              </div>

              {/* Deuxième ligne: Engine Type - Power (HP) - Transmission */}
              <div>
                <label className={labelCls}>{t.upload?.engineType || "Engine Type"}</label>
                <input
                  type="text"
                  value={engineType}
                  onChange={(e) => setEngineType(e.target.value)}
                  className={inputSmCls}
                  placeholder=""
                  disabled={isUploading}
                  spellCheck={false}
                />
              </div>

              <div>
                <label className={labelCls}>{t.upload?.powerHp || "Power (HP)"}</label>
                <input
                  type="text"
                  value={power}
                  onChange={(e) => setPower(e.target.value)}
                  className={inputSmCls}
                  placeholder=""
                  disabled={isUploading}
                  spellCheck={false}
                />
              </div>

              <div>
                <label className={labelCls}>{t.upload?.transmission || "Transmission"}</label>
                <StyledSelect
                  value={transmissionType}
                  onChange={setTransmissionType}
                  className="w-full"
                  disabled={isUploading}
                  options={[
                    { value: "", label: t.upload?.select || "Select..." },
                    { value: "Automatic", label: t.upload?.automatic || "Automatic" },
                    { value: "Manual", label: t.upload?.manual || "Manual" },
                  ]}
                />
              </div>

              {/* Troisième ligne: Customer - Stage - Date */}
              <div>
                <label className={labelCls}>{t.upload?.customer || "Customer"}</label>
                <input
                  type="text"
                  value={customer}
                  onChange={(e) => setCustomer(e.target.value)}
                  className={inputSmCls}
                  placeholder=""
                  disabled={isUploading}
                  spellCheck={false}
                />
              </div>

              <div>
                <label className={labelCls}>{t.upload?.stage || "Stage"}</label>
                <StyledSelect
                  value={stage}
                  onChange={setStage}
                  className="w-full"
                  disabled={isUploading}
                  options={[
                    { value: "", label: t.upload?.select || "Select..." },
                    { value: "Stage 1", label: "Stage 1" },
                    { value: "Stage 2", label: "Stage 2" },
                    { value: "Stage 3", label: "Stage 3" },
                  ]}
                />
              </div>

              <div>
                <label className={labelCls}>{t.upload?.date || "Date"}</label>
                <input
                  type="date"
                  value={date}
                  readOnly
                  className={`${inputSmCls} cursor-not-allowed opacity-75`}
                />
              </div>
            </div>
          </div>

          {/* Notes */}
          <div>
            <label className={labelCls}>{t.upload?.notes || "Notes"}</label>
            <textarea
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              className={`${inputCls} resize-none`}
              rows={3}
              placeholder=""
              disabled={isUploading}
              spellCheck={false}
            />
          </div>

          {/* Action Buttons */}
          <div className="flex gap-3 pt-4">
            <Button
              onClick={handleCreateProject}
              disabled={!selectedFile || !projectName.trim() || isUploading || isAnalyzing || !!pendingOlsChoice || (ecuIdentification?.ecu_type === "Unknown" && !canCreateWithoutDetection)}
              className="w-full bg-gradient-to-r from-mamba-700/90 via-mamba-600/90 to-venom-600/90 hover:from-mamba-600/90 hover:via-mamba-500/90 hover:to-venom-500/90 text-white shadow-lg shadow-mamba-500/20"
              size="lg"
            >
              {isUploading ? (
                <span className="inline-flex items-center">
                  {(t.common?.uploading || "Uploading...").replace(/\.{3}$/, '')}
                  <span className="inline-flex w-[18px]">
                    <span className="animate-[dotPulse_1.4s_infinite] [animation-delay:0s]">.</span>
                    <span className="animate-[dotPulse_1.4s_infinite] [animation-delay:0.2s]">.</span>
                    <span className="animate-[dotPulse_1.4s_infinite] [animation-delay:0.4s]">.</span>
                  </span>
                </span>
              ) : (
                <>
                  <Check className="w-4 h-4 mr-2" />
                  {t.upload?.createProject || "Create Project & Detect Maps"}
                </>
              )}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
