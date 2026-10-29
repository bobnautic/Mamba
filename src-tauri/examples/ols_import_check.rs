//! Manual validation/bench tool for `.ols` import, same convention as `dump_maps`/`expected_report`.
//! Inspects a `.ols` file and extracts every listed version to `<file>.vN.bin` next to it, so the
//! output can be byte-compared against a known-good extraction from another tool.
//!
//! Usage: cargo run --release --example ols_import_check -- <file.ols>

use mamba_lib::ols_import;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: ols_import_check <file.ols>");
        std::process::exit(2);
    });
    let data = std::fs::read(&path).expect("read input file");

    let Some(info) = ols_import::inspect(&data) else {
        println!("{path}: not recognised as a .ols container");
        return;
    };
    println!(
        "{path}: make={:?} model={:?} manufacturer={:?} ecu_name={:?} hw={:?} sw={:?}",
        info.make, info.model, info.manufacturer, info.ecu_name, info.hw_number, info.sw_number
    );
    println!("  format {} | {} map(s) in the project | byte order {:?}", info.format_version, info.maps_count, info.byte_order);
    let rom_len = info.versions.first().map(|v| v.size as u32).unwrap_or(0);
    for m in mamba_lib::ols_maps::parse_maps(&data, rom_len).iter().take(5) {
        println!(
            "    map {:?} {}x{} @0x{:X} f={} {:?} | X {:?} @0x{:X} f={} | Y {:?} @0x{:X} f={}",
            m.name, m.rows, m.cols, m.start, m.factor, m.unit, m.x.name, m.x.address, m.x.factor, m.y.name, m.y.address, m.y.factor
        );
    }
    for v in &info.versions {
        println!("  version {} ({} bytes) label={:?}", v.index, v.size, v.label);
        let bytes = ols_import::extract_version(&data, v.index).expect("extract");
        let out = format!("{path}.v{}.bin", v.index);
        std::fs::write(&out, &bytes).expect("write output");
        println!("    -> {out}");
    }
}
