// Ouvre une page GitHub de Mamba dans le navigateur par défaut (la webview
// n'a pas d'onglets) : commande Rust limitée au dépôt Mamba.
import { invoke } from "@tauri-apps/api/core";

export const MAMBA_RELEASES_URL = "https://github.com/bobnautic/Mamba/releases";
export const MAMBA_REPO_URL = "https://github.com/bobnautic/Mamba";
/** Texte de la licence (GPL-3.0), lié depuis les mentions de la fenêtre À propos. */
export const MAMBA_LICENSE_URL = "https://github.com/bobnautic/Mamba/blob/main/LICENSE";

export async function openExternal(url: string): Promise<void> {
  try {
    await invoke("open_external_url", { url });
  } catch {
    // hors Tauri (navigateur de dev) : ouvrir dans un onglet
    try {
      window.open(url, "_blank", "noopener");
    } catch {
      /* silencieux */
    }
  }
}
