// Project data model of the local app.
// These shapes are inherited from the original web version (PocketBase
// collections) so the editor and dashboard code keeps working unchanged.

/** Longueur maximale d'un nom de projet : au-delà l'affichage du dashboard
 *  et de la barre latérale n'a plus la place, même tronqué. */
export const PROJECT_NAME_MAX_LENGTH = 64;

export interface FileRecord {
  id: string;
  file_name: string;
  original_name: string;
  project_name?: string;
  file_size: number;
  file_type: string;
  ecu_type?: string;
  hardware_version?: string;
  software_version?: string;
  status: 'pending' | 'processing' | 'completed' | 'failed';
  maps_detected: number;
  detection_data?: any;
  vehicle_brand?: string;
  vehicle_model?: string;
  engine_type?: string;
  transmission_type?: string;
  year?: string;
  power?: string;
  customer?: string;
  stage?: string;
  date?: string;
  notes?: string;
  map_display_settings?: any;
  /** Tri de la liste des maps choisi par l'utilisateur (mémorisé avec le projet) */
  map_sort_mode?: "address" | "name" | "name-desc";
  /** D'où vient la liste des maps : le détecteur Mamba, le projet WinOLS
   *  (.ols) d'un calculateur sans détecteur, ou un fichier de définitions que
   *  l'utilisateur importe (« imported » : .xdf TunerPro ou mappack JSON).
   *  Les trois derniers cas ne sont jamais re-détectés. */
  maps_source?: "detector" | "ols" | "both" | "imported";
  /** Ordre des octets déclaré par le projet WinOLS ("hilo" = poids fort d'abord) */
  byte_order?: "hilo" | "lohi";
  /** Nom du calculateur tel que noté dans le projet WinOLS */
  ols_ecu_name?: string;
  created: string;
  updated: string;
}

export interface Version {
  id: string;
  file: string; // file ID
  name: string;
  is_current: boolean;
  base_version?: string | null; // base version ID
  created: string;
}

export interface MapEdit {
  id: string;
  version: string; // version ID
  map_address: number;
  payload: any;
  created: string;
}
