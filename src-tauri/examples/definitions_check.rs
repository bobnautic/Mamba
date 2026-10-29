//! Reads a map definition file (TunerPro `.xdf` or JSON mappack) and prints
//! what Mamba makes of it, so a real file can be checked outside the app.
//!
//!   cargo run --example definitions_check -- <file> [rom size in bytes]

use mamba_lib::{mappack_import, models::MapDimensions, xdf_import};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: definitions_check <file.xdf|file.json> [rom_size]");
            std::process::exit(2);
        }
    };
    let rom_len: u32 = args.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let data = std::fs::read(&path).expect("cannot read the definition file");
    let text = String::from_utf8_lossy(&data);

    let (format, maps) = if xdf_import::looks_like_xdf(&data) {
        ("XDF", xdf_import::parse_xdf(&text, rom_len))
    } else if mappack_import::looks_like_json(&data) {
        ("JSON", mappack_import::parse_mappack(&text, rom_len).expect("invalid mappack"))
    } else {
        eprintln!("unrecognised definition file");
        std::process::exit(1);
    };

    println!(
        "{path}: {format}, {} map(s), byte order {}",
        maps.len(),
        mappack_import::dominant_byte_order(&maps).unwrap_or("unknown")
    );
    for m in &maps {
        let (rows, cols) = match m.dimensions {
            MapDimensions::TwoDimensional { rows, cols } => (rows, cols),
            MapDimensions::OneDimensional { length } => (1, length),
            MapDimensions::ThreeDimensional { x, y, .. } => (y, x),
        };
        println!(
            "  ${:06X} {:>3}x{:<3} {:?}{} f={} o={} [{}] {} | X ${:06X} f={} {} | Y ${:06X} f={} {}",
            m.address,
            rows,
            cols,
            m.data_type,
            if m.is_little_endian == Some(true) { " LoHi" } else { "" },
            m.correction_factor.unwrap_or(1.0),
            m.offset.unwrap_or(0.0),
            m.category.clone().unwrap_or_default(),
            m.name.clone().unwrap_or_default(),
            m.x_axis_address.unwrap_or(0),
            m.x_axis_correction.unwrap_or(1.0),
            m.x_label.clone().unwrap_or_default(),
            m.y_axis_address.unwrap_or(0),
            m.y_axis_correction.unwrap_or(1.0),
            m.y_label.clone().unwrap_or_default(),
        );
        for (side, values) in [("X", &m.x_axis_values), ("Y", &m.y_axis_values)] {
            if let Some(v) = values {
                let head: Vec<String> = v.iter().take(4).map(|x| format!("{x}")).collect();
                println!("      axe {side} fixe : {} points, {} …", v.len(), head.join(", "));
            }
        }
    }
}
