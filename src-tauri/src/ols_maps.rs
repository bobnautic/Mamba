//! Map definitions stored in a WinOLS `.ols` project (the "Tableaux" of the
//! project): name, comment, unit, factor/offset, data address and cell type,
//! and the two axes with their own address, factor and unit.
//!
//! Companion of `ols_import.rs`, which only extracts the ROM bytes. Together
//! they let Mamba open a WinOLS project of an ECU it has no detector for:
//! the ROM comes from the container, the map list from the project itself.
//!
//! Layout: the WinOLS 5 record layout (container format version 800 and
//! above), established byte by byte on real projects saved by WinOLS 5.84 and
//! checked against 213 maps whose addresses, dimensions, factors and axis
//! addresses were known independently (a Mamba mappack imported in WinOLS
//! and saved back). The field walk follows the same order as the GPL-3.0
//! romHEX14 project's reader (<https://github.com/ctabuyo/romHEX14-community>,
//! `src/io/ols/OlsKennfeldParser.cpp`) for the parts that project documents;
//! this is an independent implementation with its own decomposition. Older
//! container formats (WinOLS 4.x) lay the records out differently and are not
//! read: `parse_maps` returns an empty list for them, and the ROM import still
//! works without a map list.
//!
//! Every record is validated before being kept (name, dimensions, data range
//! consistent with rows × cols × cell size, addresses inside the ROM), so a
//! project with no map, or a layout this module does not know, yields nothing
//! rather than garbage.

use crate::models::{DataType, DetectedMap, MapDimensions};

/// Container format versions this record layout was established on.
const MIN_FORMAT_VERSION: u32 = 800;
const MAX_CSTRING_LEN: usize = 8192;
/// Largest ROM a record may point into (16 MB): rejects misread records.
const MAX_ROM_LEN: u32 = 0x100_0000;
const MAX_DIM: u32 = 512;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct OlsAxis {
    pub name: String,
    pub unit: String,
    pub factor: f64,
    pub offset: f64,
    /// Address of the axis points in the ROM; 0 or 0xFFFFFFFF = no axis data.
    pub address: u32,
    /// WinOLS data type: 1/8/9 = 8 bits, 2 = 16 bits high-low, 3 = 16 bits
    /// low-high, 4 / 5 = 32 bits high-low / low-high, 6 / 7 = float.
    pub data_type: u32,
    pub backwards: bool,
    pub signed: bool,
    /// Bytes preceding the axis points in the ROM (the axis identifier word
    /// on EDC15, e.g. 0xC048 for an IQ axis), as WinOLS records it.
    pub header: u32,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct OlsMap {
    pub name: String,
    pub id_name: String,
    pub comment: String,
    pub unit: String,
    pub folder: Option<String>,
    /// Nombre de lignes (= nombre de points de l'axe Y).
    pub rows: u32,
    /// Nombre de colonnes (= nombre de points de l'axe X).
    pub cols: u32,
    /// Same coding as `OlsAxis::data_type`.
    pub cell_type: u32,
    pub cell_signed: bool,
    pub factor: f64,
    pub offset: f64,
    /// First and one-past-last byte of the map values in the ROM.
    pub start: u32,
    pub end: u32,
    pub precision: u32,
    pub x: OlsAxis,
    pub y: OlsAxis,
}

impl OlsMap {
    pub fn cell_bytes(&self) -> u32 {
        cell_bytes(self.cell_type)
    }

    /// True when the values are stored high byte first.
    pub fn big_endian(&self) -> bool {
        matches!(self.cell_type, 2 | 4 | 6)
    }
}

fn cell_bytes(data_type: u32) -> u32 {
    match data_type {
        2 | 3 => 2,
        4 | 5 | 6 | 7 => 4,
        10 | 11 | 12 | 13 => 8,
        _ => 1,
    }
}

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8], pos: usize) -> Self {
        Self { data, pos }
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        if end > self.data.len() {
            return None;
        }
        let s = &self.data[self.pos..end];
        self.pos = end;
        Some(s)
    }

    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn u16(&mut self) -> Option<u16> {
        let b = self.take(2)?;
        Some(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Option<u32> {
        let b = self.take(4)?;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i32(&mut self) -> Option<i32> {
        self.u32().map(|v| v as i32)
    }

    fn f64(&mut self) -> Option<f64> {
        let b = self.take(8)?;
        let v = f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
        v.is_finite().then_some(v)
    }

    fn skip(&mut self, n: usize) -> Option<()> {
        self.take(n).map(|_| ())
    }

    /// Length-prefixed string, WinOLS 5 flavour: no trailing NUL, negative
    /// lengths are placeholders ("-", "?", "%").
    fn cstring(&mut self) -> Option<String> {
        let len = self.i32()?;
        match len {
            -1 => return Some("-".to_string()),
            -2 => return Some("?".to_string()),
            -3 => return Some("%".to_string()),
            n if n <= 0 => return Some(String::new()),
            _ => {}
        }
        let len = len as usize;
        if len > MAX_CSTRING_LEN {
            return None;
        }
        let raw = self.take(len)?;
        let text = decode_text(raw)?;
        Some(text)
    }

    /// A string carrying folder references behind it: the string, two words,
    /// then a count of (string, word) entries.
    fn folder_ref(&mut self) -> Option<(String, Vec<String>)> {
        let name = self.cstring()?;
        self.u32()?;
        self.u32()?;
        let count = self.u32()?;
        if count > 0x1000 {
            return None;
        }
        let mut entries = Vec::new();
        for _ in 0..count {
            entries.push(self.cstring()?);
            self.u32()?;
        }
        Some((name, entries))
    }
}

/// ASCII, UTF-8 or Windows-1252 text; anything with control characters is
/// not a string field.
fn decode_text(raw: &[u8]) -> Option<String> {
    let text = match std::str::from_utf8(raw) {
        Ok(s) => s.to_string(),
        Err(_) => raw.iter().map(|&b| b as char).collect(),
    };
    if text.chars().any(|c| (c as u32) < 0x20 && !matches!(c, '\t' | '\r' | '\n')) {
        return None;
    }
    Some(text)
}

fn read_axis(c: &mut Cursor) -> Option<OlsAxis> {
    let (name, _folders) = c.folder_ref()?;
    let unit = c.cstring()?;
    let factor = c.f64()?;
    let offset = c.f64()?;
    let _axis_type = c.u32()?;
    let address = c.u32()?;
    let data_type = c.u32()?;
    c.u32()?;
    let _cell_bits = c.u32()?;
    let backwards = c.u16()? != 0;
    c.skip(8)?;
    c.skip(4)?;
    c.skip(4)?;
    let signed = c.u8()? != 0;
    let inline = c.u32()?;
    if inline > 0x1_0000 {
        return None;
    }
    c.skip(inline as usize)?;
    c.u32()?;
    let header = c.u32()?;
    let _id_name = c.cstring()?;
    let folder_count = c.u32()?;
    if folder_count > 0x1000 {
        return None;
    }
    for _ in 0..folder_count {
        c.folder_ref()?;
    }
    c.u32()?;
    Some(OlsAxis { name, unit, factor, offset, address, data_type, backwards, signed, header })
}

/// One map record starting at its comment string. Returns the map and the
/// offset right after its second axis (the record tail is skipped by the
/// caller's resynchronisation).
fn read_map(data: &[u8], at: usize, rom_len: u32) -> Option<(OlsMap, usize)> {
    let mut c = Cursor::new(data, at);
    let (comment, folders) = c.folder_ref()?;
    c.u8()?;
    let name = c.cstring()?;
    if name.trim().is_empty() || name.len() > 120 {
        return None;
    }
    c.u32()?;
    c.u32()?;
    c.u32()?;
    let _kennfeld_type = c.u32()?;
    c.u32()?;
    let cell_type = c.u32()?;
    c.u32()?;
    let _cell_bits = c.u32()?;
    c.u32()?;
    let id_name = c.cstring()?;
    c.u32()?;
    c.u32()?;
    c.u32()?;
    c.u8()?;
    c.u32()?;
    c.skip(6 * 8)?;
    c.skip(6 * 8)?;
    let _cell_inverse = c.u8()?;
    let cell_signed = c.u8()? != 0;
    c.u8()?;
    c.u8()?;
    // Le record annonce les COLONNES puis les LIGNES, et plus bas l'axe des
    // colonnes (X) puis celui des lignes (Y). Établi sur un projet réel :
    // le mappack que Mamba avait exporté pour ce même fichier, réimporté
    // dans WinOLS, donne les dimensions et les adresses d'axes attendues de
    // ses 183 maps nommées — l'ordre inverse les contredit toutes.
    let cols = c.u32()?;
    let rows = c.u32()?;
    c.u32()?;
    c.u32()?;
    let precision = c.u32()?;
    let _label = c.cstring()?;
    let unit = c.cstring()?;
    let factor = c.f64()?;
    let offset = c.f64()?;
    let start = c.u32()?;
    let end = c.u32()?;
    let base = c.u32()?;
    c.f64()?;
    for _ in 0..5 {
        c.u32()?;
    }

    // Plausibility: a real record describes a block inside the ROM whose size
    // matches its grid. Anything else is a misaligned read.
    if !(1..=MAX_DIM).contains(&rows) || !(1..=MAX_DIM).contains(&cols) {
        return None;
    }
    if start >= end || end > base || base > MAX_ROM_LEN {
        return None;
    }
    if rom_len != 0 && end > rom_len {
        return None;
    }
    let cells = rows as u64 * cols as u64;
    if (end - start) as u64 != cells * cell_bytes(cell_type) as u64 {
        return None;
    }
    if !(1..=13).contains(&cell_type) {
        return None;
    }

    // Axe des COLONNES (X) d'abord, axe des LIGNES (Y) ensuite — même ordre
    // que les deux nombres ci-dessus.
    let x = read_axis(&mut c)?;
    let y = read_axis(&mut c)?;
    let folder = folders.into_iter().find(|f| !f.trim().is_empty());
    let map = OlsMap {
        name,
        id_name,
        comment,
        unit,
        folder,
        rows,
        cols,
        cell_type,
        cell_signed,
        factor,
        offset,
        start,
        end,
        precision,
        x,
        y,
    };
    Some((map, c.pos))
}

/// Reads the container header (24 bytes, "WinOLS File") and returns the
/// format version, or None when `data` is not a `.ols` container.
fn format_version(data: &[u8]) -> Option<u32> {
    if data.len() < 24 || &data[4..15] != b"WinOLS File" || data[15] != 0 {
        return None;
    }
    if u32::from_le_bytes([data[0], data[1], data[2], data[3]]) != 0x0000_000B {
        return None;
    }
    Some(u32::from_le_bytes([data[16], data[17], data[18], data[19]]))
}

/// All map records of a `.ols` project, in file order. `rom_len` (the size of
/// one saved version) bounds the addresses; pass 0 when unknown.
pub fn parse_maps(data: &[u8], rom_len: u32) -> Vec<OlsMap> {
    let Some(version) = format_version(data) else {
        return Vec::new();
    };
    if version < MIN_FORMAT_VERSION {
        return Vec::new();
    }
    // The records live between the header block and the saved versions. The
    // version directory (see ols_import) tells where the versions start;
    // without it, scan the whole file.
    let region_end = crate::ols_import::version_data_start(data).unwrap_or(data.len());
    let mut maps = Vec::new();
    let mut seen_starts = std::collections::HashSet::new();
    let mut pos = 24usize;
    while pos + 8 < region_end {
        // A record is introduced by 0xFFFFFFFF right before its comment string.
        if data[pos] == 0xFF && data[pos + 1] == 0xFF && data[pos + 2] == 0xFF && data[pos + 3] == 0xFF {
            let at = pos + 4;
            let len = i32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]);
            if (-3..=MAX_CSTRING_LEN as i32).contains(&len) {
                if let Some((map, after)) = read_map(data, at, rom_len) {
                    if seen_starts.insert((map.start, map.name.clone())) {
                        maps.push(map);
                    }
                    pos = after;
                    continue;
                }
            }
        }
        pos += 1;
    }
    maps
}

/// "hilo" when most maps store their values high byte first, "lohi"
/// otherwise; None without maps.
pub fn byte_order(maps: &[OlsMap]) -> Option<&'static str> {
    if maps.is_empty() {
        return None;
    }
    let big = maps.iter().filter(|m| m.big_endian()).count();
    Some(if big * 2 > maps.len() { "hilo" } else { "lohi" })
}

/// Folder a WinOLS map goes into, using the same names as the app's own
/// mappack so both trees read alike (EGR, Injection system, Turbo boost
/// pressure…). The rules match the EDC15P detector's own naming pass; a name
/// it does not recognise falls back to the folder the WinOLS project itself
/// put the map in, then to "Other".
fn category_for(name: &str, project_folder: Option<&str>) -> String {
    let l = name.to_lowercase();
    let by_name = if l.contains("start of injection") || l.contains("soi") {
        Some("Start of injection")
    } else if l.contains("injector duration") || l.contains("selector for injector") || l.contains("injection") {
        Some("Injection system")
    } else if l.contains("smoke") || l.contains("iq by maf") || l.contains("iq by map")
        || l.contains("iq limiter") || l.contains("iq by air int") || l.contains("map/maf switch")
    {
        Some("Smoke limitation")
    } else if l.contains("launch control") {
        Some("Launch control")
    } else if l.contains("torque limiter") || l.contains("torque to iq") {
        Some("Engine torque limiters")
    } else if l.contains("driver wish") || l.contains("drivers wish") || l.contains("start iq") {
        Some("Engine fuel request")
    } else if l.contains("n75") || l.contains("boost actuator") || l.contains("pid map") {
        Some("Turbo boost pressure control")
    } else if l.contains("boost") || l.contains("turbo") || l.contains("svbl") {
        Some("Turbo boost pressure")
    } else if l.contains("egr") {
        Some("EGR")
    } else if l.contains("idle rpm") || l.contains("idle speed") {
        Some("Idle speed RPM")
    } else if l.contains("svrl") || l.contains("rpm limiter") || l.contains("speed limiter") {
        Some("Maximum RPM limiter")
    } else if l.contains("vcds") || l.contains("measurement") || l.contains("diagnostic") {
        Some("VCDS diagnostic")
    } else if l.contains("map linearisation") || l.contains("map linearization") || l.contains("map sensor") {
        Some("MAP sensor")
    } else {
        None
    };
    by_name
        .map(|c| c.to_string())
        .or_else(|| project_folder.map(|f| f.trim().to_string()).filter(|f| !f.is_empty()))
        .unwrap_or_else(|| "Other".to_string())
}

fn axis_label(axis: &OlsAxis) -> Option<String> {
    let name = axis.name.trim();
    let unit = axis.unit.trim();
    match (name.is_empty(), unit.is_empty()) {
        (true, true) => None,
        (false, true) => Some(name.to_string()),
        (true, false) => Some(unit.to_string()),
        (false, false) => Some(format!("{name} ({unit})")),
    }
}

fn axis_address(axis: &OlsAxis) -> Option<u32> {
    (axis.address != 0 && axis.address != 0xFFFF_FFFF).then_some(axis.address)
}

/// Converts the project's maps into the shape the editor already displays,
/// so a WinOLS project opens like any detected file. `project_big_endian`
/// is the majority byte order of the project (see `byte_order`); a map that
/// departs from it carries its own flag.
pub fn to_detected_maps(maps: &[OlsMap], project_big_endian: bool) -> Vec<DetectedMap> {
    maps.iter()
        .map(|m| {
            let data_type = match (m.cell_bytes(), m.cell_type, m.cell_signed) {
                (_, 6 | 7, _) => DataType::Float32,
                (1, _, true) => DataType::Int8,
                (1, _, false) => DataType::UInt8,
                (2, _, true) => DataType::Int16,
                (2, _, false) => DataType::UInt16,
                (_, _, true) => DataType::Int32,
                (_, _, false) => DataType::UInt32,
            };
            let mut d = DetectedMap::new(
                m.start,
                (m.end - m.start) as usize,
                MapDimensions::TwoDimensional { rows: m.rows as usize, cols: m.cols as usize },
                data_type,
            );
            d.external_source = Some("OLS".to_string());
            d.name = Some(m.name.clone());
            let comment = m.comment.trim();
            d.description = Some(if comment.is_empty() { format!("WinOLS map {}", m.name) } else { comment.to_string() });
            let folder = category_for(&m.name, m.folder.as_deref());
            d.category = Some(folder.clone());
            d.subcategory = Some(folder);
            d.unit = Some(m.unit.clone());
            d.correction_factor = Some(if m.factor == 0.0 { 1.0 } else { m.factor });
            d.offset = Some(m.offset);
            d.confidence = 1.0;
            d.x_axis_address = axis_address(&m.x);
            d.y_axis_address = axis_address(&m.y);
            d.x_axis_correction = Some(if m.x.factor == 0.0 { 1.0 } else { m.x.factor });
            d.y_axis_correction = Some(if m.y.factor == 0.0 { 1.0 } else { m.y.factor });
            d.x_axis_offset = (m.x.offset != 0.0).then_some(m.x.offset);
            d.y_axis_offset = (m.y.offset != 0.0).then_some(m.y.offset);
            d.x_label = axis_label(&m.x);
            d.y_label = axis_label(&m.y);
            if project_big_endian && !m.big_endian() && m.cell_bytes() > 1 {
                d.is_little_endian = Some(true);
            }
            d
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cstr(s: &str) -> Vec<u8> {
        let mut v = (s.len() as i32).to_le_bytes().to_vec();
        v.extend_from_slice(s.as_bytes());
        v
    }

    fn folder_ref(s: &str) -> Vec<u8> {
        let mut v = cstr(s);
        v.extend_from_slice(&2u32.to_le_bytes());
        v.extend_from_slice(&2u32.to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes());
        v
    }

    fn axis(name: &str, unit: &str, factor: f64, address: u32, backwards: bool, header: u32) -> Vec<u8> {
        let mut v = folder_ref(name);
        v.extend(cstr(unit));
        v.extend_from_slice(&factor.to_le_bytes());
        v.extend_from_slice(&0f64.to_le_bytes());
        v.extend_from_slice(&1u32.to_le_bytes());
        v.extend_from_slice(&address.to_le_bytes());
        v.extend_from_slice(&3u32.to_le_bytes());
        v.extend_from_slice(&2u32.to_le_bytes());
        v.extend_from_slice(&10u32.to_le_bytes());
        v.extend_from_slice(&(backwards as u16).to_le_bytes());
        v.extend_from_slice(&[0u8; 16]);
        v.push(0);
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(&header.to_le_bytes());
        v.extend(cstr(""));
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes());
        v
    }

    /// A record laid out exactly like the WinOLS 5.84 one read on a real
    /// project (EGR map of an EDC15P, 16 rows × 13 columns at 0x4C116).
    fn record(name: &str, comment: &str, rows: u32, cols: u32, start: u32, cell_type: u32, signed: bool) -> Vec<u8> {
        let mut v = vec![0xFF, 0xFF, 0xFF, 0xFF];
        v.extend(folder_ref(comment));
        v.push(0);
        v.extend(cstr(name));
        for w in [2u32, 2, 0, 4, 2, cell_type, 2, 10, 4] {
            v.extend_from_slice(&w.to_le_bytes());
        }
        v.extend(cstr(name));
        for w in [0u32, 16, 0] {
            v.extend_from_slice(&w.to_le_bytes());
        }
        v.push(1);
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(&[0u8; 96]);
        v.extend_from_slice(&[0, signed as u8, 0, 0]);
        for w in [cols, rows, 0, 0, 2] {
            v.extend_from_slice(&w.to_le_bytes());
        }
        v.extend(cstr(name));
        v.extend(cstr("mg/st"));
        v.extend_from_slice(&0.1f64.to_le_bytes());
        v.extend_from_slice(&0f64.to_le_bytes());
        let end = start + rows * cols * cell_bytes(cell_type);
        v.extend_from_slice(&start.to_le_bytes());
        v.extend_from_slice(&end.to_le_bytes());
        v.extend_from_slice(&0x80000u32.to_le_bytes());
        v.extend_from_slice(&0f64.to_le_bytes());
        v.extend_from_slice(&[0u8; 20]);
        // axe des colonnes (X, le régime) d'abord, axe des lignes (Y, la
        // quantité) ensuite — l'ordre du vrai record
        v.extend(axis("Engine speed", "rpm", 1.0, 0x4C0FC, false, 0xEC38));
        v.extend(axis("IQ", "mg/st", 0.01, 0x4C0D8, true, 0xC048));
        // record tail as written by WinOLS: counters, then the 01 01 01 marker
        v.extend_from_slice(&[0u8; 60]);
        v.extend_from_slice(&[1, 1, 1]);
        v.extend_from_slice(&[0u8; 40]);
        v
    }

    fn container(records: &[Vec<u8>]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&0x0000_000Bu32.to_le_bytes());
        buf.extend_from_slice(b"WinOLS File\0");
        buf.extend_from_slice(&804u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&[0u8; 64]);
        for r in records {
            buf.extend_from_slice(r);
        }
        buf
    }

    #[test]
    fn reads_a_winols5_record() {
        let data = container(&[record("EGR [codeblock 2]", "EGR setpoint", 16, 13, 0x4C116, 3, false)]);
        let maps = parse_maps(&data, 0x80000);
        assert_eq!(maps.len(), 1);
        let m = &maps[0];
        assert_eq!(m.name, "EGR [codeblock 2]");
        assert_eq!(m.comment, "EGR setpoint");
        assert_eq!((m.rows, m.cols), (16, 13));
        assert_eq!((m.start, m.end), (0x4C116, 0x4C116 + 16 * 13 * 2));
        assert!((m.factor - 0.1).abs() < 1e-12);
        assert_eq!(m.unit, "mg/st");
        // X = les colonnes (régime), Y = les lignes (quantité)
        assert_eq!(m.x.address, 0x4C0FC);
        assert_eq!(m.x.header, 0xEC38);
        assert!((m.x.factor - 1.0).abs() < 1e-12);
        assert_eq!(m.y.address, 0x4C0D8);
        assert_eq!(m.y.header, 0xC048);
        assert!((m.y.factor - 0.01).abs() < 1e-12);
        assert!(m.y.backwards);
        assert!(!m.big_endian());
    }

    #[test]
    fn reads_several_records_and_skips_noise() {
        let mut noise = vec![0xFF, 0xFF, 0xFF, 0xFF];
        noise.extend(cstr("Hexdump"));
        noise.extend_from_slice(&[0x11u8; 40]);
        let data = container(&[
            noise,
            record("Driver wish", "", 16, 16, 0x50000, 3, true),
            record("Torque limiter", "Nm", 8, 12, 0x51000, 2, false),
        ]);
        let maps = parse_maps(&data, 0x80000);
        assert_eq!(maps.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), vec!["Driver wish", "Torque limiter"]);
        assert!(maps[0].cell_signed);
        assert!(maps[1].big_endian());
        assert_eq!(byte_order(&maps), Some("lohi"));
    }

    #[test]
    fn rejects_records_whose_size_does_not_match_the_grid() {
        let mut bad = record("Broken", "", 13, 16, 0x4C116, 3, false);
        // corrupt the end address: the block no longer covers rows × cols cells
        let pos = bad.windows(4).position(|w| w == (0x4C116u32 + 13 * 16 * 2).to_le_bytes()).unwrap();
        bad[pos..pos + 4].copy_from_slice(&(0x4C116u32 + 7).to_le_bytes());
        let data = container(&[bad]);
        assert!(parse_maps(&data, 0x80000).is_empty());
    }

    #[test]
    fn older_container_formats_yield_no_maps() {
        let mut data = container(&[record("EGR", "", 16, 13, 0x4C116, 3, false)]);
        data[16..20].copy_from_slice(&288u32.to_le_bytes());
        assert!(parse_maps(&data, 0x80000).is_empty());
    }

    #[test]
    fn sorts_maps_into_the_same_folders_as_the_app() {
        assert_eq!(category_for("EGR [codeblock 2]", None), "EGR");
        assert_eq!(category_for("Smoke limiter", None), "Smoke limitation");
        assert_eq!(category_for("N75 duty cycle", None), "Turbo boost pressure control");
        assert_eq!(category_for("Boost target map", None), "Turbo boost pressure");
        assert_eq!(category_for("Driver wish", None), "Engine fuel request");
        assert_eq!(category_for("Start of injection (SOI) 30°C", None), "Start of injection");
        assert_eq!(category_for("VCDS Diagnostic IQ Limit", None), "VCDS diagnostic");
        // nom inconnu : le dossier du projet WinOLS, sinon « Other »
        assert_eq!(category_for("Kennfeld 42", Some("Meine Karten")), "Meine Karten");
        assert_eq!(category_for("Kennfeld 42", None), "Other");
        assert_eq!(category_for("Kennfeld 42", Some("   ")), "Other");
    }

    #[test]
    fn converts_to_the_editor_shape() {
        let data = container(&[record("EGR [codeblock 2]", "EGR setpoint", 16, 13, 0x4C116, 3, false)]);
        let maps = parse_maps(&data, 0x80000);
        let detected = to_detected_maps(&maps, false);
        let d = &detected[0];
        assert_eq!(d.address, 0x4C116);
        assert_eq!(d.size, 16 * 13 * 2);
        assert!(matches!(d.dimensions, MapDimensions::TwoDimensional { rows: 16, cols: 13 }));
        assert!(matches!(d.data_type, DataType::UInt16));
        assert_eq!(d.x_axis_address, Some(0x4C0FC));
        assert_eq!(d.y_axis_address, Some(0x4C0D8));
        assert_eq!(d.x_label.as_deref(), Some("Engine speed (rpm)"));
        assert_eq!(d.y_label.as_deref(), Some("IQ (mg/st)"));
        assert_eq!(d.category.as_deref(), Some("EGR"));
        assert!(d.is_little_endian.is_none());
        // a little-endian map inside a big-endian project keeps its own flag
        let detected_be = to_detected_maps(&maps, true);
        assert_eq!(detected_be[0].is_little_endian, Some(true));
    }
}
