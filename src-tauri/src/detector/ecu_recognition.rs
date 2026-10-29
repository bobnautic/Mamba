// Universal ECU recognition: the identity card of ANY binary, whatever its
// manufacturer, presented the way StageX shows an ECU next to a project
// (detected manufacturer and family, hardware / software / calibration
// numbers, plus every other identification number found in the file).
//
// It complements and never replaces the strict `ECUIdentifier`. That one only
// says yes to the families Mamba has a map detector for, and its answer is
// what routes the detection engine. This module answers "what is this file?"
// for everything else too (EDC17, MED17, MD1/MG1, SIMOS, SID, DCM, Marelli…),
// so an ECU without a Mamba detector still opens as a properly labelled
// project instead of an anonymous "Unknown".
//
// Every field comes from positive evidence in the bytes: the ASCII
// identification blocks the ECU supplier and the vehicle manufacturer write
// into the flash. The file size alone never decides anything. The only
// estimate is the byte order of a file whose family is unknown, and it is
// flagged as such (`byte_order_source = "heuristic"`).

use regex::bytes::Regex;
use serde::Serialize;
use std::sync::OnceLock;

use super::ecu_identifier::{ECUIdentification, ECUIdentifier, ECUManufacturer, ECUType};

/// One piece of evidence the recognition is based on, with where it sits in
/// the file. `kind` is a stable key the frontend translates.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RecognitionEvidence {
    pub kind: String,
    pub value: String,
    pub offset: usize,
}

/// Everything the binary says about the ECU it was read from.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct EcuRecognition {
    /// True when at least the manufacturer or the family was recognised.
    pub detected: bool,
    /// ECU supplier: Bosch, Continental, Siemens, Delphi, Magneti Marelli, Denso…
    pub manufacturer: Option<String>,
    /// Exact family / variant as the supplier names it (EDC17C64, MED17.7.1, SIMOS18.10…).
    pub family: Option<String>,
    /// Generation without the variant (EDC17, MED17, SIMOS18…), handy for filtering.
    pub family_group: Option<String>,
    /// "Diesel" or "Petrol" when the family or the engine string tells.
    pub fuel: Option<String>,
    /// Supplier hardware number (Bosch 0281…/0261…, Siemens 5WS…, Continental A2C…).
    pub hardware_number: Option<String>,
    /// Supplier software number (Bosch 1037…/1039…).
    pub software_number: Option<String>,
    /// Calibration / data set identifier (Bosch descriptor data set).
    pub calibration_number: Option<String>,
    /// Bosch software project code (P866, P941…).
    pub project_code: Option<String>,
    /// Vehicle manufacturer software part number (VAG 03L906018HE, Mercedes A651…).
    pub part_number: Option<String>,
    /// Vehicle manufacturer hardware part number when it differs (VAG 04L907309).
    pub hardware_part_number: Option<String>,
    /// Vehicle manufacturer software version (VAG 4-digit index, e.g. 9978).
    pub software_version: Option<String>,
    /// VAG ODX / EXAM identifier (EV_ECM20TDI01103L906018HE).
    pub odx_identifier: Option<String>,
    /// Engine description as written in the identification block ("R4 2.0l TDI").
    pub engine: Option<String>,
    /// Engine code (CJCC, CNHA…).
    pub engine_code: Option<String>,
    /// Vehicle make the identification numbers belong to.
    pub vehicle_brand: Option<String>,
    /// Processor family of the ECU when the family pins it.
    pub processor: Option<String>,
    /// "hilo" (big-endian / Motorola) or "lohi" (little-endian / Intel).
    pub byte_order: Option<String>,
    /// "family" when the byte order follows from the family, "heuristic" when estimated.
    pub byte_order_source: Option<String>,
    pub flash_size: usize,
    /// Every identification number found, most relevant first (StageX "otherNumbers").
    pub other_numbers: Vec<String>,
    pub evidence: Vec<RecognitionEvidence>,
    /// 0..1, how sure the recognition is about manufacturer + family.
    pub confidence: f32,
    /// ECU type of the Mamba map detector that handles this file, when there is one.
    pub mamba_ecu_type: Option<String>,
    /// True when Mamba has a map detector for this file.
    pub detector_available: bool,
}

/// What a family name implies.
struct FamilyInfo {
    manufacturer: &'static str,
    fuel: Option<&'static str>,
    processor: Option<&'static str>,
    byte_order: Option<&'static str>,
}

const C167: &str = "Infineon C167";
const TRICORE: &str = "Infineon TriCore";
const AURIX: &str = "Infineon AURIX (TriCore)";
const MPC5XX: &str = "Freescale MPC5xx (PowerPC)";

fn family_info(family: &str) -> Option<FamilyInfo> {
    let f = family.to_ascii_uppercase();
    let info = |manufacturer, fuel, processor, byte_order| {
        Some(FamilyInfo { manufacturer, fuel, processor, byte_order })
    };
    // Longest prefixes first: MEDC17 before MED17 before ME7…
    if f.starts_with("EDC15") {
        info("Bosch", Some("Diesel"), Some(C167), Some("lohi"))
    } else if f.starts_with("EDC16") {
        info("Bosch", Some("Diesel"), Some(MPC5XX), Some("hilo"))
    } else if f.starts_with("EDC17") {
        info("Bosch", Some("Diesel"), Some(TRICORE), Some("lohi"))
    } else if f.starts_with("MEDC17") {
        info("Bosch", None, Some(TRICORE), Some("lohi"))
    } else if f.starts_with("MEVD17") || f.starts_with("MED17") || f.starts_with("ME17") || f.starts_with("MEV17") {
        info("Bosch", Some("Petrol"), Some(TRICORE), Some("lohi"))
    } else if f.starts_with("MED9") || f.starts_with("ME9") {
        info("Bosch", Some("Petrol"), Some(MPC5XX), Some("hilo"))
    } else if f.starts_with("ME7") {
        info("Bosch", Some("Petrol"), Some(C167), Some("lohi"))
    } else if f.starts_with("MD1") {
        info("Bosch", Some("Diesel"), Some(AURIX), Some("lohi"))
    } else if f.starts_with("MG1") {
        info("Bosch", Some("Petrol"), Some(AURIX), Some("lohi"))
    } else if f.starts_with("SIMOS18") || f.starts_with("SIMOS12") || f.starts_with("SIMOS16") {
        info("Continental", Some("Petrol"), Some(TRICORE), Some("lohi"))
    } else if f.starts_with("SIMOS") || f.starts_with("EMS3") || f.starts_with("SIM2K") {
        info("Continental", Some("Petrol"), None, None)
    } else if f.starts_with("PCR") {
        info("Continental", Some("Diesel"), None, None)
    } else if f.starts_with("SID") || f.starts_with("PPD") {
        info("Siemens", Some("Diesel"), None, None)
    } else if f.starts_with("MSD") || f.starts_with("MSV") {
        info("Siemens", Some("Petrol"), None, None)
    } else if f.starts_with("DCM") || f.starts_with("DDCR") || f.starts_with("CRD") {
        info("Delphi", Some("Diesel"), None, None)
    } else if f.starts_with("MJD") {
        info("Magneti Marelli", Some("Diesel"), None, None)
    } else if f.starts_with("IAW") || f.starts_with("8GM") {
        info("Magneti Marelli", Some("Petrol"), None, None)
    } else {
        None
    }
}

/// "EDC17C64" -> "EDC17", "MED17.7.1" -> "MED17", "MD1CP001" -> "MD1",
/// "SIMOS18.10" -> "SIMOS18", "8GMF" -> "8GMF".
fn family_group(family: &str) -> String {
    let bytes = family.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        i += 1;
    }
    if i == 0 {
        return family.to_string();
    }
    let mut j = i;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    family[..j].to_string()
}

/// Supplier spelling -> one canonical name: upper case, no blanks,
/// "EDC17_C46" -> "EDC17C46", "MED17_7_1" -> "MED17.7.1".
fn normalize_family(raw: &str) -> String {
    let compact: Vec<u8> = raw
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .map(|b| b.to_ascii_uppercase())
        .collect();
    let mut out = String::with_capacity(compact.len());
    for (i, &b) in compact.iter().enumerate() {
        if b == b'_' {
            let prev_digit = i > 0 && compact[i - 1].is_ascii_digit();
            let next_digit = compact.get(i + 1).map_or(false, |c| c.is_ascii_digit());
            if prev_digit && next_digit {
                out.push('.');
            }
            continue;
        }
        out.push(b as char);
    }
    out.trim_matches('.').to_string()
}

struct Patterns {
    /// Bosch ASAP family descriptor: "34/1/EDC17C64/5/P866//C866DE7PO///"
    bosch_descriptor: Regex,
    bosch_sw: Regex,
    bosch_sw_full: Regex,
    bosch_hw: Regex,
    vag_part: Regex,
    vag_part_at_end: Regex,
    odx: Regex,
    engine_cyl_first: Regex,
    engine_disp_first: Regex,
    four_digits: Regex,
    siemens_hw: Regex,
    conti_hw: Regex,
    conti_sw: Regex,
    conti_hw_sw: Regex,
    makers: Regex,
    families: Vec<Regex>,
    oem: Vec<(Regex, &'static str)>,
}

impl Patterns {
    fn new() -> Self {
        // (?-u): byte-oriented matching, the input is binary, not UTF-8.
        let re = |p: &str| Regex::new(&format!("(?-u){p}")).expect("valid recognition regex");
        // Boundaries written as explicit classes: a part number may be glued
        // to a project code ("1037561849P941"), where \b would not hold.
        const B: &str = "(?:^|[^0-9A-Za-z])";
        const E: &str = "(?:[^0-9A-Za-z]|$)";
        Patterns {
            bosch_descriptor: re(
                r"(?:^|[^0-9])\d{1,6}/\d{1,2}/([A-Z][A-Z0-9_.]{2,20})/\d{1,3}/([A-Z0-9_.]{1,24})//([A-Z0-9_.]{0,32})///",
            ),
            bosch_sw: re(r"(?:^|[^0-9])(10(?:37|39)\d{6})(?:[^0-9]|$)"),
            bosch_sw_full: re(r"(?:^|[^0-9])(10(?:37|39)\d{6}[A-Z][A-Z0-9]{3,11})(?:[^0-9A-Z]|$)"),
            bosch_hw: re(r"(?:^|[^0-9A-Z])(02(?:61|81)[0-9S]\d{5})(?:[^0-9]|$)"),
            // 906 = engine ECU, 907 = ECU hardware, 909 = early Motronic, 997 = exchange part.
            // A one- or two-letter suffix may follow after a single blank ("038906019 NJ").
            vag_part: re(
                r"(?:^|[^0-9A-Z])(\d[0-9A-Z]{2}9(?:06|07|09|97)\d{3})(?:([A-Z]{1,3})|\x20([A-Z]{1,2}))?(?:[^0-9A-Z_]|$)",
            ),
            vag_part_at_end: re(r"(\d[0-9A-Z]{2}9(?:06|07|09|97)\d{3}[A-Z]{0,3})$"),
            odx: re(r"(?:^|[^0-9A-Za-z])(EV_[A-Z0-9]{4,40})"),
            engine_cyl_first: re(&format!(
                r"{B}([RVW]\d{{1,2}}\x20{{1,3}}\d[.,]\d\x20?[lL](?:\x20{{1,3}}[A-Z][A-Za-z]{{1,4}})?){E}"
            )),
            engine_disp_first: re(&format!(
                r"{B}(\d[.,]\d\x20?[lL]\x20{{1,3}}[RVW]\d{{1,2}}(?:/\d?[A-Z]{{1,3}})?(?:\x20{{1,3}}[A-Z]{{2,5}})?){E}"
            )),
            four_digits: re(r"(?:^|[^0-9A-Za-z.,])(\d{4})(?:[^0-9A-Za-z.,]|$)"),
            siemens_hw: re(&format!(r"{B}(5W[KPSY]\d{{4,5}}[A-Z]?){E}")),
            conti_hw: re(&format!(r"{B}(A2C\d{{8}}[A-Z0-9]{{0,4}}){E}")),
            conti_sw: re(&format!(r"{B}(\d{{8}}[A-Z]{{2}}){E}")),
            conti_hw_sw: re(r"(\d{3}[A-Z]{3}\d{10})"),
            makers: re(
                r"(?:^|[^A-Za-z])(BOSCH|Bosch|SIEMENS|Siemens|CONTINENTAL|Continental|DELPHI|Delphi|MARELLI|Marelli|DENSO|Denso|VISTEON|Visteon|HITACHI|Hitachi|KEFICO|Kefico)(?:[^A-Za-z]|$)",
            ),
            families: [
                r"(EDC1[5-7](?:[A-Z]{1,3}\d{0,2}(?:\.\d{1,2})?)?)",
                r"((?:MEVD|MEDC|MED|MEV|ME)(?:17|9|7)\.\d{1,2}(?:\.\d{1,2}){0,2})",
                r"(M[DG]1[A-Z]{1,2}\d{3})",
                r"(SIMOS\x20?\d{1,2}(?:\.\d{1,2})?)",
                r"(SID\x20?\d{3}[A-Z]?)",
                r"(PCR\x20?2\.1)",
                r"(PPD\x20?1\.\d)",
                r"(EMS\x20?3\d{3})",
                r"(SIM2K-?\d{2,3})",
                r"(MS[DV]\d{2}(?:\.\d)?)",
                r"(DCM\x20?\d\.\d{1,2}[A-Z]{0,2})",
                r"(DDCR)",
                r"(CRD\x20?[1-3](?:\.\d{1,2})?)",
                r"(MJD\x20?\d{1,2}[A-Z0-9]{0,4})",
                r"(IAW\x20?\d{1,2}[A-Z]{1,3}\d?)",
                r"(8GM[FKW])",
            ]
            .iter()
            .map(|f| re(&format!("{B}{f}{E}")))
            .collect(),
            oem: vec![
                (re(&format!(r"{B}(A\x20?\d{{3}}\x20?\d{{3}}\x20?\d{{2}}\x20?\d{{2}})(?:[^0-9]|$)")), "Mercedes-Benz"),
                (re(r"(?:^|[^0-9])(9[68]\d{6}80)(?:[^0-9]|$)"), "Peugeot / Citroën"),
                (re(&format!(r"{B}([0-9A-Z]{{4}}-12A650-[A-Z]{{1,3}}){E}")), "Ford"),
                (re(r"(?:^|[^0-9])(8200\d{6})(?:[^0-9]|$)"), "Renault"),
                (re(&format!(r"{B}(39\d{{3}}-[0-9A-Z]{{5}}){E}")), "Hyundai / Kia"),
                (re(&format!(r"{B}(8966[0-9]-[0-9A-Z]{{5}}){E}")), "Toyota"),
            ],
        }
    }
}

fn patterns() -> &'static Patterns {
    static P: OnceLock<Patterns> = OnceLock::new();
    P.get_or_init(Patterns::new)
}

/// ECU types the Mamba map detector handles (enabled entries of ecus.json).
fn detector_families() -> &'static [String] {
    static F: OnceLock<Vec<String>> = OnceLock::new();
    F.get_or_init(|| {
        let list: serde_json::Value =
            serde_json::from_str(include_str!("../../ecus.json")).unwrap_or_default();
        list.as_array()
            .map(|ecus| {
                ecus.iter()
                    .filter(|e| e["enabled"].as_bool().unwrap_or(false))
                    .filter_map(|e| e["name"].as_str().map(|s| s.to_ascii_uppercase()))
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// True when Mamba has a map detector for this ECU type.
pub fn has_detector(ecu_type: &str) -> bool {
    let t = ecu_type.trim().to_ascii_uppercase();
    detector_families().iter().any(|f| *f == t)
}

fn ascii(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim().to_string()
}

/// Insertion-ordered set of identification numbers.
#[derive(Default)]
struct Numbers(Vec<String>);

impl Numbers {
    fn push(&mut self, value: Option<&String>) {
        if let Some(v) = value {
            let v = v.trim();
            if !v.is_empty() && !self.0.iter().any(|x| x == v) {
                self.0.push(v.to_string());
            }
        }
    }
}

/// A file made of printable text (a log, an A2L, a CSV…) is not an ECU dump.
fn looks_like_text(data: &[u8]) -> bool {
    let sample = &data[..data.len().min(64 * 1024)];
    if sample.len() < 64 {
        return false;
    }
    let printable = sample
        .iter()
        .filter(|&&b| b == b'\n' || b == b'\r' || b == b'\t' || (0x20..0x7F).contains(&b))
        .count();
    printable * 100 >= sample.len() * 95
}

/// Number of smooth, strictly increasing 16-bit sequences (the axes every
/// calibration is full of) when the file is read with one byte order. Read
/// with the wrong order, an axis jumps around and does not count. A run must
/// move both bytes (so a series of small values that only looks increasing
/// once swapped is not counted) and climb by small steps (random data almost
/// never does).
fn smooth_axis_runs(data: &[u8], big_endian: bool) -> usize {
    const MIN_LEN: usize = 6;
    const MAX_STEP: u16 = 0x0800;
    let words = data.len() / 2;
    let read = |k: usize| -> u16 {
        let b = [data[2 * k], data[2 * k + 1]];
        if big_endian {
            u16::from_be_bytes(b)
        } else {
            u16::from_le_bytes(b)
        }
    };
    let mut runs = 0;
    let mut k = 0;
    while k + 1 < words {
        let first = read(k);
        let mut prev = first;
        let (mut hi_moved, mut lo_moved) = (false, false);
        let mut j = k + 1;
        while j < words {
            let v = read(j);
            if v <= prev || v - prev > MAX_STEP {
                break;
            }
            hi_moved |= (v >> 8) != (prev >> 8);
            lo_moved |= (v & 0xFF) != (prev & 0xFF);
            prev = v;
            j += 1;
        }
        if j - k >= MIN_LEN && hi_moved && lo_moved && prev - first >= 0x100 && prev != 0xFFFF {
            runs += 1;
        }
        k = j;
    }
    runs
}

/// Byte order estimated from the calibration axes, for files whose family
/// does not pin it. None when the evidence is thin or ambiguous.
pub fn guess_byte_order(data: &[u8]) -> Option<&'static str> {
    byte_order_from_axes(data).filter(|a| a.best >= 12 && a.best >= 2 * a.other.max(1)).map(|a| a.order)
}

/// What the calibration axes say about the byte order.
struct AxisVote {
    order: &'static str,
    best: usize,
    other: usize,
}

impl AxisVote {
    /// So lopsided that it overrules what the family implies (some
    /// manufacturers store the calibration of a little-endian family in
    /// big-endian, e.g. Mercedes MD1 files).
    fn decisive(&self) -> bool {
        self.best >= 100 && self.best >= 10 * self.other.max(1)
    }
}

fn byte_order_from_axes(data: &[u8]) -> Option<AxisVote> {
    let be = smooth_axis_runs(data, true);
    let le = smooth_axis_runs(data, false);
    if be == le {
        return None;
    }
    Some(if be > le {
        AxisVote { order: "hilo", best: be, other: le }
    } else {
        AxisVote { order: "lohi", best: le, other: be }
    })
}

/// Engine code written right after the engine description, repeated
/// ("R4 2.0l TDI    CJCC CJCC CJCC CJCC").
fn engine_code_after(window: &[u8]) -> Option<String> {
    let text: String = window
        .iter()
        .take_while(|&&b| (0x20..0x7F).contains(&b))
        .map(|&b| b as char)
        .collect();
    let tokens: Vec<&str> = text.split_whitespace().collect();
    for (i, tok) in tokens.iter().enumerate() {
        let is_code = (3..=4).contains(&tok.len()) && tok.bytes().all(|b| b.is_ascii_uppercase());
        if is_code && tokens.iter().skip(i).take(3).filter(|t| *t == tok).count() == 3 {
            return Some(tok.to_string());
        }
    }
    None
}

fn evidence(kind: &str, value: &str, offset: usize) -> RecognitionEvidence {
    RecognitionEvidence { kind: kind.to_string(), value: value.to_string(), offset }
}

/// Recognise the ECU a binary was read from. Never fails: an unrecognised
/// file comes back with `detected = false` and whatever numbers were found.
pub fn recognize(data: &[u8]) -> EcuRecognition {
    recognize_with(data, &ECUIdentifier::identify(data))
}

/// Same as `recognize`, reusing the strict identification the caller already
/// ran on these bytes (it decides whether a Mamba detector applies).
pub fn recognize_with(data: &[u8], strict: &ECUIdentification) -> EcuRecognition {
    let mut r = EcuRecognition { flash_size: data.len(), ..Default::default() };
    if data.len() < 1024 || looks_like_text(data) {
        return r;
    }
    let p = patterns();

    let strict_type = format!("{:?}", strict.ecu_type);
    let strict_supported = strict.ecu_type != ECUType::Unknown && has_detector(&strict_type);
    if strict_supported {
        r.mamba_ecu_type = Some(strict_type.clone());
        r.detector_available = true;
        r.evidence.push(evidence("mamba_detector", &strict_type, 0));
    }

    // ── Bosch family descriptor (EDC17 / MED17 / MD1 / MG1 generation) ──
    let mut descriptor_family: Option<String> = None;
    if let Some(c) = p.bosch_descriptor.captures(data) {
        let whole = c.get(0).unwrap();
        let family = normalize_family(&ascii(&c[1]));
        if family_info(&family).is_some() {
            descriptor_family = Some(family);
            let project = ascii(&c[2]);
            let dataset = ascii(&c[3]);
            if !project.is_empty() {
                r.project_code = Some(project);
            }
            if !dataset.is_empty() {
                r.calibration_number = Some(dataset);
            }
            let text = ascii(whole.as_bytes());
            let text = text.trim_start_matches(|ch: char| !ch.is_ascii_digit());
            r.evidence.push(evidence("family_descriptor", text, whole.start()));
        }
    }

    // ── Family names written in clear ──
    let mut tokens: Vec<(String, usize, usize)> = Vec::new(); // (family, count, first offset)
    for fam_re in &p.families {
        for c in fam_re.captures_iter(data) {
            let m = c.get(1).unwrap();
            let family = normalize_family(&ascii(m.as_bytes()));
            match tokens.iter_mut().find(|t| t.0 == family) {
                Some(t) => t.1 += 1,
                None => tokens.push((family, 1, m.start())),
            }
        }
    }
    // A specific name inherits the votes of its generic prefix
    // ("EDC17" ×3 + "EDC17C46" ×1 -> EDC17C46 with 4).
    let best_token = tokens
        .iter()
        .map(|(f, n, off)| {
            let inherited: usize = tokens
                .iter()
                .filter(|(g, _, _)| g != f && f.starts_with(g.as_str()))
                .map(|(_, m, _)| *m)
                .sum();
            (f.clone(), n + inherited, *off)
        })
        .max_by(|a, b| a.1.cmp(&b.1).then(a.0.len().cmp(&b.0.len())));
    for (family, count, offset) in &tokens {
        r.evidence.push(evidence("family_string", &format!("{family} ×{count}"), *offset));
    }

    // ── Supplier numbers ──
    let mut numbers = Numbers::default();
    let mut bosch_sw: Vec<(String, usize)> = Vec::new();
    for c in p.bosch_sw.captures_iter(data) {
        let m = c.get(1).unwrap();
        let v = ascii(m.as_bytes());
        if !bosch_sw.iter().any(|(x, _)| *x == v) {
            bosch_sw.push((v, m.start()));
        }
    }
    let bosch_sw_full: Vec<String> =
        p.bosch_sw_full.captures_iter(data).map(|c| ascii(&c[1])).collect();
    let mut bosch_hw: Vec<(String, usize)> = Vec::new();
    for c in p.bosch_hw.captures_iter(data) {
        let m = c.get(1).unwrap();
        let v = ascii(m.as_bytes());
        if !bosch_hw.iter().any(|(x, _)| *x == v) {
            bosch_hw.push((v, m.start()));
        }
    }
    if let Some((v, off)) = bosch_sw.first() {
        r.evidence.push(evidence("bosch_sw", v, *off));
    }
    if let Some((v, off)) = bosch_hw.first() {
        r.evidence.push(evidence("bosch_hw", v, *off));
    }
    let siemens_hw: Option<String> = p.siemens_hw.captures(data).map(|c| ascii(&c[1]));
    let conti_hw: Option<String> = p.conti_hw.captures(data).map(|c| ascii(&c[1]));

    // ── VAG ODX identifier and part numbers ──
    let odx = p.odx.captures(data).map(|c| {
        let m = c.get(1).unwrap();
        (ascii(m.as_bytes()), m.start())
    });
    let odx_part = odx.as_ref().and_then(|(token, _)| {
        p.vag_part_at_end.captures(token.as_bytes()).map(|c| ascii(&c[1]))
    });
    if let Some((token, off)) = &odx {
        r.odx_identifier = Some(token.clone());
        r.evidence.push(evidence("odx", token, *off));
    }
    let mut vag_parts: Vec<(String, usize, usize)> = Vec::new(); // (number, start, end)
    for c in p.vag_part.captures_iter(data) {
        let base = c.get(1).unwrap();
        let suffix = c.get(2).or_else(|| c.get(3));
        let mut number = ascii(base.as_bytes());
        let mut end = base.end();
        if let Some(s) = suffix {
            number.push_str(&ascii(s.as_bytes()));
            end = s.end();
        }
        if !vag_parts.iter().any(|(n, _, _)| *n == number) {
            vag_parts.push((number, base.start(), end));
        }
    }
    let sw_part = odx_part.clone().or_else(|| {
        vag_parts
            .iter()
            .find(|(n, _, _)| matches!(&n[3..6], "906" | "909"))
            .or_else(|| vag_parts.first())
            .map(|(n, _, _)| n.clone())
    });
    let hw_part = sw_part.as_ref().and_then(|sw| {
        vag_parts
            .iter()
            .filter(|(n, _, _)| n != sw)
            .min_by_key(|(n, _, _)| if &n[3..6] == "907" { 0 } else { 1 })
            .map(|(n, _, _)| n.clone())
    });
    if let Some(sw) = &sw_part {
        let off = vag_parts.iter().find(|(n, _, _)| n == sw).map_or(0, |v| v.1);
        r.evidence.push(evidence("oem_part", sw, off));
        // VAG software index: first standalone 4-digit group after a
        // standalone copy of the software part number.
        for (_, _, end) in vag_parts.iter().filter(|(n, _, _)| n == sw) {
            let window = &data[*end..(*end + 64).min(data.len())];
            if let Some(c) = p.four_digits.captures(window) {
                r.software_version = Some(ascii(&c[1]));
                break;
            }
        }
        if r.software_version.is_none() {
            // Part number only met inside the ODX token: look after its next copy.
            let needle = sw.as_bytes();
            let mut from = 0;
            while let Some(pos) = find(&data[from..], needle) {
                let start = from + pos;
                let end = start + needle.len();
                let standalone = start > 0 && !data[start - 1].is_ascii_alphanumeric();
                if standalone {
                    let window = &data[end..(end + 64).min(data.len())];
                    if let Some(c) = p.four_digits.captures(window) {
                        r.software_version = Some(ascii(&c[1]));
                        break;
                    }
                }
                from = end;
            }
        }
    }

    // ── Engine ──
    let engine_match = p
        .engine_cyl_first
        .captures(data)
        .or_else(|| p.engine_disp_first.captures(data))
        .map(|c| c.get(1).unwrap());
    if let Some(m) = engine_match {
        let text = ascii(m.as_bytes()).split_whitespace().collect::<Vec<_>>().join(" ");
        r.evidence.push(evidence("engine", &text, m.start()));
        r.engine = Some(text);
        let window = &data[m.end()..(m.end() + 160).min(data.len())];
        r.engine_code = engine_code_after(window);
    }

    // ── Continental SIMOS signature: ODX identifier + Continental part numbers
    // next to it (the layout simos18_dtc.py of StageX-A2L reads) ──
    let mut conti_signature: Option<(String, Option<String>)> = None;
    if let Some((_, off)) = &odx {
        let lo = off.saturating_sub(0x120);
        let hi = (off + 0x120).min(data.len());
        let window = &data[lo..hi];
        let manufacturer_sw = p.conti_sw.captures(window).map(|c| ascii(&c[1]));
        let hw_sw = p.conti_hw_sw.captures(window).map(|c| ascii(&c[1]));
        if let Some(sw) = manufacturer_sw.filter(|s| bosch_sw.is_empty() && s.starts_with("1037")) {
            r.evidence.push(evidence("continental_signature", &sw, *off));
            conti_signature = Some((sw, hw_sw));
        }
    }

    // ── Family ──
    let strict_fallback = match strict.ecu_type {
        ECUType::Unknown => None,
        ECUType::EDC17C | ECUType::EDC17CP | ECUType::EDC17U => Some("EDC17".to_string()),
        ECUType::MED17_1 | ECUType::MED17_5 => Some("MED17".to_string()),
        _ => Some(normalize_family(&strict_type)),
    };
    let (family, family_confidence) = if let Some(f) = descriptor_family.clone() {
        (Some(f), 0.95)
    } else if strict_supported {
        (Some(strict_type.clone()), strict.confidence.max(0.8))
    } else if let Some((f, count, _)) = best_token.clone().filter(|t| family_info(&t.0).is_some()) {
        (Some(f), if count >= 2 { 0.85 } else { 0.75 })
    } else if conti_signature.is_some() {
        (Some("SIMOS18".to_string()), 0.7)
    } else if let Some(f) = strict_fallback {
        (Some(f), strict.confidence.min(0.7))
    } else {
        (None, 0.0)
    };
    let info = family.as_deref().and_then(family_info);

    // ── Manufacturer ──
    let maker_literal = p.makers.captures(data).map(|c| {
        let name = ascii(&c[1]).to_ascii_uppercase();
        match name.as_str() {
            "BOSCH" => "Bosch",
            "SIEMENS" => "Siemens",
            "CONTINENTAL" => "Continental",
            "DELPHI" => "Delphi",
            "MARELLI" => "Magneti Marelli",
            "DENSO" => "Denso",
            "VISTEON" => "Visteon",
            "HITACHI" => "Hitachi",
            _ => "Kefico",
        }
    });
    r.manufacturer = info
        .as_ref()
        .map(|i| i.manufacturer.to_string())
        .or_else(|| (!bosch_sw.is_empty() || !bosch_hw.is_empty()).then(|| "Bosch".to_string()))
        .or_else(|| conti_signature.as_ref().map(|_| "Continental".to_string()))
        .or_else(|| (siemens_hw.is_some()).then(|| "Siemens".to_string()))
        .or_else(|| (conti_hw.is_some()).then(|| "Continental".to_string()))
        .or_else(|| maker_literal.map(str::to_string))
        .or_else(|| {
            (strict.manufacturer != ECUManufacturer::Unknown)
                .then(|| strict.manufacturer.display_name().to_string())
        });
    if let (Some(name), None) = (maker_literal, &info) {
        r.evidence.push(evidence("manufacturer_name", name, 0));
    }

    // ── Numbers ──
    let strict_numbers = |v: &Option<String>| if strict_supported { v.clone() } else { None };
    r.hardware_number = strict_numbers(&strict.hardware_version)
        .or_else(|| bosch_hw.first().map(|(v, _)| v.clone()))
        .or_else(|| siemens_hw.clone())
        .or_else(|| conti_hw.clone())
        .or_else(|| conti_signature.as_ref().and_then(|(_, hw)| hw.clone()));
    r.software_number = bosch_sw
        .first()
        .map(|(v, _)| v.clone())
        .or_else(|| conti_signature.as_ref().map(|(sw, _)| sw.clone()))
        .or_else(|| strict_numbers(&strict.software_version));
    r.part_number = sw_part.clone().or_else(|| strict_numbers(&strict.part_number));
    r.hardware_part_number = hw_part;

    // ── Vehicle make ──
    let mut oem_numbers: Vec<String> = Vec::new();
    // (make, how many numbers, its first number) of the make with the most numbers
    let mut oem_brand: Option<(&'static str, usize, String)> = None;
    for (oem_re, brand) in &p.oem {
        let found: Vec<String> = oem_re
            .captures_iter(data)
            .map(|c| ascii(&c[1]).replace(' ', ""))
            .collect();
        if !found.is_empty() && oem_brand.as_ref().map_or(true, |(_, n, _)| found.len() > *n) {
            oem_brand = Some((brand, found.len(), found[0].clone()));
        }
        oem_numbers.extend(found);
    }
    r.vehicle_brand = if !vag_parts.is_empty() || odx.is_some() {
        Some("Volkswagen Group".to_string())
    } else if let Some((brand, _, number)) = &oem_brand {
        if r.part_number.is_none() {
            r.part_number = Some(number.clone());
        }
        Some(brand.to_string())
    } else if family.as_deref().map_or(false, |f| f.starts_with("MSD") || f.starts_with("MSV")) {
        Some("BMW".to_string())
    } else {
        None
    };

    // ── Fuel, processor, byte order ──
    r.fuel = info.as_ref().and_then(|i| i.fuel.map(str::to_string)).or_else(|| {
        let engine = r.engine.as_deref().unwrap_or("").to_ascii_uppercase();
        if ["TDI", "EDC", "CDI", "HDI", "DCI", "CRDI", "JTD"].iter().any(|k| engine.contains(k)) {
            Some("Diesel".to_string())
        } else if ["TFSI", "TSI", "FSI", "MPI", "5VT"].iter().any(|k| engine.contains(k)) {
            Some("Petrol".to_string())
        } else {
            None
        }
    });
    r.processor = info.as_ref().and_then(|i| i.processor.map(str::to_string));
    let axes = byte_order_from_axes(data);
    let family_order = info.as_ref().and_then(|i| i.byte_order);
    let (order, source) = match (family_order, &axes) {
        (Some(f), Some(a)) if a.order != f && a.decisive() => (Some(a.order), "heuristic"),
        (Some(f), _) => (Some(f), "family"),
        (None, Some(a)) if a.best >= 12 && a.best >= 2 * a.other.max(1) => (Some(a.order), "heuristic"),
        _ => (None, ""),
    };
    if let Some(order) = order {
        r.byte_order = Some(order.to_string());
        r.byte_order_source = Some(source.to_string());
        if let Some(a) = &axes {
            let votes = format!("{order}: {} / {}", a.best, a.other);
            r.evidence.push(evidence("byte_order_axes", &votes, 0));
        }
    }

    r.family_group = family.as_deref().map(family_group);
    r.family = family;
    r.detected = r.family.is_some() || r.manufacturer.is_some();
    r.confidence = if r.family.is_some() {
        family_confidence
    } else if r.manufacturer.is_some() {
        0.5
    } else {
        0.0
    };

    // ── Every identification number, most relevant first ──
    numbers.push(r.software_number.as_ref());
    numbers.push(r.hardware_number.as_ref());
    numbers.push(r.part_number.as_ref());
    numbers.push(r.hardware_part_number.as_ref());
    numbers.push(r.software_version.as_ref());
    numbers.push(r.project_code.as_ref());
    numbers.push(r.calibration_number.as_ref());
    for v in &bosch_sw_full {
        numbers.push(Some(v));
    }
    for (v, _) in bosch_sw.iter().chain(bosch_hw.iter()) {
        numbers.push(Some(v));
    }
    for (v, _, _) in &vag_parts {
        numbers.push(Some(v));
    }
    for v in oem_numbers.iter().chain(siemens_hw.iter()).chain(conti_hw.iter()) {
        numbers.push(Some(v));
    }
    if let Some((sw, hw)) = &conti_signature {
        numbers.push(Some(sw));
        numbers.push(hw.as_ref());
    }
    numbers.0.truncate(24);
    r.other_numbers = numbers.0;
    r.evidence.truncate(24);
    r
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic pseudo-random filler (xorshift), so tests do not depend on a crate.
    fn noise(len: usize, seed: u32) -> Vec<u8> {
        let mut x = seed.max(1);
        (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                (x & 0xFF) as u8
            })
            .collect()
    }

    fn put(buf: &mut [u8], offset: usize, text: &[u8]) {
        buf[offset..offset + text.len()].copy_from_slice(text);
    }

    /// Smooth increasing axes, the way calibration data looks.
    fn put_axes(buf: &mut [u8], offset: usize, big_endian: bool, count: usize) {
        let mut pos = offset;
        for a in 0..count {
            let start = 500 + (a as u16 * 37) % 900;
            for i in 0..12u16 {
                let v = start + i * (250 + (a as u16 % 5) * 40);
                let b = if big_endian { v.to_be_bytes() } else { v.to_le_bytes() };
                buf[pos..pos + 2].copy_from_slice(&b);
                pos += 2;
            }
            // a few calibration words between the axes
            buf[pos..pos + 6].copy_from_slice(&[0x12, 0x07, 0x00, 0x00, 0x9C, 0x40]);
            pos += 6;
        }
    }

    #[test]
    fn vag_edc17c46_identity_block() {
        let mut data = noise(2 * 1024 * 1024, 7);
        put(&mut data, 0x40020, b"\x001037561849P941D5WW\x00");
        put(&mut data, 0x40100, b"\x00000635/1/EDC17_C46/5/P941//C941DD5WW///\x00");
        put(
            &mut data,
            0x1E0000,
            b"\x0003L906018JL EV_ECM20TDI01103L906018HE  003007 03L906018HE  9978                      R4 2.0l TDI          CJCC CJCC CJCC CJCC CJCC \x00",
        );
        let r = recognize(&data);
        assert!(r.detected);
        assert_eq!(r.manufacturer.as_deref(), Some("Bosch"));
        assert_eq!(r.family.as_deref(), Some("EDC17C46"));
        assert_eq!(r.family_group.as_deref(), Some("EDC17"));
        assert_eq!(r.software_number.as_deref(), Some("1037561849"));
        assert_eq!(r.project_code.as_deref(), Some("P941"));
        assert_eq!(r.calibration_number.as_deref(), Some("C941DD5WW"));
        assert_eq!(r.part_number.as_deref(), Some("03L906018HE"));
        assert_eq!(r.hardware_part_number.as_deref(), Some("03L906018JL"));
        assert_eq!(r.software_version.as_deref(), Some("9978"));
        assert_eq!(r.odx_identifier.as_deref(), Some("EV_ECM20TDI01103L906018HE"));
        assert_eq!(r.engine.as_deref(), Some("R4 2.0l TDI"));
        assert_eq!(r.engine_code.as_deref(), Some("CJCC"));
        assert_eq!(r.vehicle_brand.as_deref(), Some("Volkswagen Group"));
        assert_eq!(r.fuel.as_deref(), Some("Diesel"));
        assert_eq!(r.processor.as_deref(), Some(TRICORE));
        assert_eq!(r.byte_order.as_deref(), Some("lohi"));
        assert_eq!(r.byte_order_source.as_deref(), Some("family"));
        assert!(!r.detector_available);
        assert!(r.mamba_ecu_type.is_none());
        assert!(r.confidence >= 0.9);
        assert!(r.other_numbers.iter().any(|n| n == "1037561849P941D5WW"));
        assert_eq!(r.other_numbers.first().map(String::as_str), Some("1037561849"));
    }

    #[test]
    fn edc17c64_with_hardware_part_before_odx() {
        let mut data = noise(4 * 1024 * 1024, 11);
        put(&mut data, 0x10000, b"\x001037555198P866E7PO\x00");
        put(&mut data, 0x10040, b"\x0034/1/EDC17C64/5/P866//C866DE7PO///\x00");
        put(
            &mut data,
            0x3F0000,
            b"\x0004L907309   EV_ECM20TDI01104L906021BD  002006 04L906021BD  8284                      R4 2,0L EDC          CNHA CNHA CNHA CNHA \x00",
        );
        let r = recognize(&data);
        assert_eq!(r.family.as_deref(), Some("EDC17C64"));
        assert_eq!(r.part_number.as_deref(), Some("04L906021BD"));
        assert_eq!(r.hardware_part_number.as_deref(), Some("04L907309"));
        assert_eq!(r.software_version.as_deref(), Some("8284"));
        assert_eq!(r.engine.as_deref(), Some("R4 2,0L EDC"));
        assert_eq!(r.engine_code.as_deref(), Some("CNHA"));
    }

    #[test]
    fn md1_descriptor() {
        let mut data = noise(1024 * 1024, 3);
        put(&mut data, 0x2000, b"\x0046/1/MD1CP001/1/DA_MDG1D//DA_MDG1D_0M_B070D///\x00");
        let r = recognize(&data);
        assert_eq!(r.family.as_deref(), Some("MD1CP001"));
        assert_eq!(r.family_group.as_deref(), Some("MD1"));
        assert_eq!(r.manufacturer.as_deref(), Some("Bosch"));
        assert_eq!(r.fuel.as_deref(), Some("Diesel"));
        assert_eq!(r.calibration_number.as_deref(), Some("DA_MDG1D_0M_B070D"));
    }

    #[test]
    fn simos18_identity_block() {
        let mut data = noise(5632 * 1024, 5);
        put(
            &mut data,
            0x220000,
            b"\x0010375561AA------111SCG0054000000\x00\x005G0906259N .2.0l R4 TFSI .   0003\x00\x00EV_ECM20TFS0205G0906259N..002004J623\x00",
        );
        let r = recognize(&data);
        assert_eq!(r.manufacturer.as_deref(), Some("Continental"));
        assert_eq!(r.family.as_deref(), Some("SIMOS18"));
        assert_eq!(r.software_number.as_deref(), Some("10375561AA"));
        assert_eq!(r.hardware_number.as_deref(), Some("111SCG0054000000"));
        assert_eq!(r.part_number.as_deref(), Some("5G0906259N"));
        assert_eq!(r.engine.as_deref(), Some("2.0l R4 TFSI"));
        assert_eq!(r.fuel.as_deref(), Some("Petrol"));
        assert_eq!(r.software_version.as_deref(), Some("0003"));
    }

    #[test]
    fn me7_family_string_and_bosch_numbers() {
        let mut data = noise(1024 * 1024, 13);
        put(&mut data, 0x18000, b"\x000261208228 1037368072 ME7.5   \x00");
        put(&mut data, 0x18100, b"\x0006A906032HN 1.8L R4/5VT G 0002\x00");
        put(&mut data, 0x30000, b"\x00ME7.5\x00");
        let r = recognize(&data);
        assert_eq!(r.family.as_deref(), Some("ME7.5"));
        assert_eq!(r.family_group.as_deref(), Some("ME7"));
        assert_eq!(r.hardware_number.as_deref(), Some("0261208228"));
        assert_eq!(r.software_number.as_deref(), Some("1037368072"));
        assert_eq!(r.part_number.as_deref(), Some("06A906032HN"));
        assert_eq!(r.engine.as_deref(), Some("1.8L R4/5VT"));
        assert_eq!(r.fuel.as_deref(), Some("Petrol"));
        assert_eq!(r.processor.as_deref(), Some(C167));
    }

    #[test]
    fn delphi_and_marelli_families() {
        let mut data = noise(1024 * 1024, 17);
        put(&mut data, 0x500, b"\x00DCM3.5 \x00");
        put(&mut data, 0x900, b"\x00DCM3.5\x00");
        let r = recognize(&data);
        assert_eq!(r.family.as_deref(), Some("DCM3.5"));
        assert_eq!(r.manufacturer.as_deref(), Some("Delphi"));
        assert!(r.confidence >= 0.85);

        let mut data = noise(1024 * 1024, 19);
        put(&mut data, 0x700, b"\x00MJD 6F3 \x00");
        let r = recognize(&data);
        assert_eq!(r.family.as_deref(), Some("MJD6F3"));
        assert_eq!(r.manufacturer.as_deref(), Some("Magneti Marelli"));
    }

    #[test]
    fn oem_part_numbers_name_the_vehicle_make() {
        // 4 MB: a size no Mamba detector handles, so only the recognition speaks
        let mut data = noise(4 * 1024 * 1024, 23);
        put(&mut data, 0x1000, b"\x00A6519003900\x00");
        put(&mut data, 0x1100, b"\x000281017569\x00");
        let r = recognize(&data);
        assert_eq!(r.vehicle_brand.as_deref(), Some("Mercedes-Benz"));
        assert_eq!(r.part_number.as_deref(), Some("A6519003900"));
        assert_eq!(r.manufacturer.as_deref(), Some("Bosch"));
        assert!(r.family.is_none());
        assert!(r.detected);

        let mut data = noise(1024 * 1024, 29);
        put(&mut data, 0x1000, b"\x009663548180\x00");
        assert_eq!(recognize(&data).vehicle_brand.as_deref(), Some("Peugeot / Citroën"));

        let mut data = noise(1024 * 1024, 31);
        put(&mut data, 0x1000, b"\x006G91-12A650-EG\x00");
        assert_eq!(recognize(&data).vehicle_brand.as_deref(), Some("Ford"));
    }

    #[test]
    fn random_and_text_files_are_not_recognised() {
        let r = recognize(&noise(2 * 1024 * 1024, 37));
        assert!(!r.detected);
        assert!(r.family.is_none());
        assert!(r.manufacturer.is_none());
        assert_eq!(r.flash_size, 2 * 1024 * 1024);

        let text = "ASAP2_VERSION 1 61\n/begin PROJECT EDC17C46 \"\"\n".repeat(4000);
        let r = recognize(text.as_bytes());
        assert!(!r.detected);
    }

    #[test]
    fn byte_order_follows_the_calibration_axes() {
        let mut be = noise(512 * 1024, 41);
        put_axes(&mut be, 0x10000, true, 120);
        assert_eq!(guess_byte_order(&be), Some("hilo"));

        let mut le = noise(512 * 1024, 43);
        put_axes(&mut le, 0x10000, false, 120);
        assert_eq!(guess_byte_order(&le), Some("lohi"));

        assert_eq!(guess_byte_order(&noise(512 * 1024, 47)), None);
    }

    #[test]
    fn heuristic_byte_order_is_flagged() {
        let mut data = noise(512 * 1024, 53);
        put_axes(&mut data, 0x8000, true, 120);
        put(&mut data, 0x100, b"\x00DELPHI\x00");
        let r = recognize(&data);
        assert_eq!(r.manufacturer.as_deref(), Some("Delphi"));
        assert_eq!(r.byte_order.as_deref(), Some("hilo"));
        assert_eq!(r.byte_order_source.as_deref(), Some("heuristic"));
    }

    #[test]
    fn decisive_axes_overrule_the_family_byte_order() {
        let mut data = noise(1024 * 1024, 59);
        put(&mut data, 0x2000, b"\x0046/1/MD1CP001/1/DA_MDG1D//DA_MDG1D_0M_B070D///\x00");
        put_axes(&mut data, 0x40000, true, 400);
        let r = recognize(&data);
        assert_eq!(r.family.as_deref(), Some("MD1CP001"));
        assert_eq!(r.byte_order.as_deref(), Some("hilo"));
        assert_eq!(r.byte_order_source.as_deref(), Some("heuristic"));

        // a weak contrary vote does not
        let mut data = noise(1024 * 1024, 61);
        put(&mut data, 0x2000, b"\x0046/1/MD1CP001/1/DA_MDG1D//DA_MDG1D_0M_B070D///\x00");
        put_axes(&mut data, 0x40000, true, 20);
        let r = recognize(&data);
        assert_eq!(r.byte_order.as_deref(), Some("lohi"));
        assert_eq!(r.byte_order_source.as_deref(), Some("family"));
    }

    #[test]
    fn family_names_are_normalised() {
        assert_eq!(normalize_family("EDC17_C46"), "EDC17C46");
        assert_eq!(normalize_family("MED17_7_1"), "MED17.7.1");
        assert_eq!(normalize_family("SIMOS 18.10"), "SIMOS18.10");
        assert_eq!(family_group("MEVD17.2.8"), "MEVD17");
        assert_eq!(family_group("SID803A"), "SID803");
        assert_eq!(family_group("8GMF"), "8GMF");
    }

    #[test]
    fn detector_families_come_from_ecus_json() {
        assert!(has_detector("EDC16U34"));
        assert!(has_detector("edc15p"));
        assert!(!has_detector("EDC17C46"));
        assert!(!has_detector("Unknown"));
    }
}
