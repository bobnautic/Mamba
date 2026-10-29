//! Écrit le résultat de détection d'un projet WinOLS (.ols) tel que l'app le
//! stocke, pour comparer hors interface : `<fichier>.maps.json`.
//!
//! Usage : cargo run --release --example ols_project_dump -- <fichier.ols>

use mamba_lib::{ols_import, ols_maps};

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: ols_project_dump <fichier.ols>");
        std::process::exit(2);
    });
    let data = std::fs::read(&path).expect("lecture du fichier");
    let Some(info) = ols_import::inspect(&data) else {
        println!("{path} : pas un conteneur .ols");
        return;
    };
    let rom_len = info.versions.first().map(|v| v.size as u32).unwrap_or(0);
    let maps = ols_maps::parse_maps(&data, rom_len);
    let big_endian = ols_maps::byte_order(&maps).unwrap_or("lohi") == "hilo";
    let detected = ols_maps::to_detected_maps(&maps, big_endian);

    println!(
        "{path} : format {}, {} version(s) de {} octets, {} map(s), ordre {:?}",
        info.format_version,
        info.versions.len(),
        rom_len,
        detected.len(),
        info.byte_order
    );
    println!(
        "  projet : marque={:?} modèle={:?} ECU={:?} HW={:?} SW={:?}",
        info.make, info.model, info.ecu_name, info.hw_number, info.sw_number
    );

    let json = serde_json::json!({
        "success": true,
        "maps": detected,
        "total_maps": detected.len(),
        "processing_time_ms": 0,
        "file_size": rom_len,
        "detector_version": mamba_lib::commands::detector_version(),
    });
    let out = format!("{path}.maps.json");
    std::fs::write(&out, serde_json::to_string_pretty(&json).unwrap()).expect("écriture");
    println!("  -> {out}");
}
