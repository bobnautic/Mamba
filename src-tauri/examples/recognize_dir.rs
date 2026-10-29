// Dev tool: run the universal ECU recognition on every binary in a directory
// (recursive) and print the identity card it reads from each file.
//   cargo run --example recognize_dir -- <dir>
// Used to validate the recognition against a corpus of real dumps, supported
// by a Mamba detector or not.

use std::path::Path;

fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                out.push(p);
            }
        }
    }
}

fn main() {
    let dir = std::env::args().nth(1).expect("usage: recognize_dir <dir>");
    let mut files = Vec::new();
    walk(Path::new(&dir), &mut files);
    files.sort();

    for path in files {
        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let r = mamba_lib::detector::recognize(&data);
        println!(
            "{} ({} KB)\n  {} {} ({:.0}%) group={} fuel={} cpu={} order={}/{}\n  hw={} sw={} cal={} project={} part={} hw_part={} version={}\n  engine={} code={} make={} odx={}\n  detector={} axes={} numbers={:?}\n",
            path.file_name().unwrap_or_default().to_string_lossy(),
            data.len() / 1024,
            r.manufacturer.as_deref().unwrap_or("?"),
            r.family.as_deref().unwrap_or("?"),
            r.confidence * 100.0,
            r.family_group.as_deref().unwrap_or("-"),
            r.fuel.as_deref().unwrap_or("-"),
            r.processor.as_deref().unwrap_or("-"),
            r.byte_order.as_deref().unwrap_or("-"),
            r.byte_order_source.as_deref().unwrap_or("-"),
            r.hardware_number.as_deref().unwrap_or("-"),
            r.software_number.as_deref().unwrap_or("-"),
            r.calibration_number.as_deref().unwrap_or("-"),
            r.project_code.as_deref().unwrap_or("-"),
            r.part_number.as_deref().unwrap_or("-"),
            r.hardware_part_number.as_deref().unwrap_or("-"),
            r.software_version.as_deref().unwrap_or("-"),
            r.engine.as_deref().unwrap_or("-"),
            r.engine_code.as_deref().unwrap_or("-"),
            r.vehicle_brand.as_deref().unwrap_or("-"),
            r.odx_identifier.as_deref().unwrap_or("-"),
            r.mamba_ecu_type.as_deref().unwrap_or("none"),
            mamba_lib::detector::ecu_recognition::guess_byte_order(&data).unwrap_or("?"),
            r.other_numbers,
        );
    }
}
