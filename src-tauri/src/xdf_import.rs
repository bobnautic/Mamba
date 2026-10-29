//! TunerPro `.xdf` definition files.
//!
//! An XDF describes where the maps of one binary live and how to read them:
//! address, grid, cell size, byte order, and a conversion equation. It carries
//! no ROM — the user opens their binary in Mamba, then imports the XDF that
//! goes with it, and the maps appear.
//!
//! Format: XML. `<XDFHEADER>` holds the defaults (`<DEFAULTS signed lsbfirst
//! datasizeinbits>`) and the categories, which become the folders of the map
//! list. Each `<XDFTABLE>` has a title, a description, one `<CATEGORYMEM>` per
//! folder it belongs to, and up to three `<XDFAXIS>`: `x` and `y` describe the
//! breakpoints (either an address in the ROM, or fixed labels written in the
//! file), `z` carries the values themselves — its `<EMBEDDEDDATA>` gives the
//! address, the cell size, the number of rows and columns, and flags for sign
//! and byte order. A `<XDFCONSTANT>` is the same thing for a single value.
//!
//! Read against a real VAG definition (Bosch M3.8.3, 06A906018CG) plus the
//! layouts TunerPro itself writes. Anything this module cannot make sense of
//! is skipped rather than guessed: a table with no `z` address, an unusable
//! cell size, an equation that is not linear, or a block that would fall
//! outside the binary.

use crate::models::{DataType, DetectedMap, MapDimensions};

/// A map definition is only accepted if its whole block fits in the binary.
const MAX_DIM: u32 = 4096;

#[derive(Debug, Default, Clone, Copy)]
struct Defaults {
    signed: bool,
    lsb_first: bool,
    size_bits: u32,
}

/// Values of the `mmedtypeflags` bits TunerPro writes: bit 0 = signed,
/// bit 1 = least significant byte first, bit 2 = the cell is a float.
#[derive(Debug, Default, Clone)]
struct Embedded {
    address: Option<u32>,
    size_bits: u32,
    rows: u32,
    cols: u32,
    flags: u32,
    has_flags: bool,
}

#[derive(Debug, Default, Clone)]
struct Axis {
    embedded: Embedded,
    units: String,
    index_count: u32,
    factor: f64,
    offset: f64,
    linear: bool,
    /// Points d'axe ecrits dans le fichier (balises `LABEL`) quand l'axe
    /// n'a pas d'adresse dans le binaire.
    labels: Vec<f64>,
}

/// One `<...>` tag: its name, its attributes, and whether it closes itself.
struct Tag {
    name: String,
    attrs: Vec<(String, String)>,
    self_closing: bool,
    closing: bool,
}

impl Tag {
    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
}

/// Minimal XML reader: enough for the flat, attribute-driven shape of an XDF,
/// with no external dependency. Returns the tags and the text between them.
struct XmlReader<'a> {
    data: &'a str,
    pos: usize,
}

enum Event {
    Tag(Tag),
    Text(String),
    End,
}

impl<'a> XmlReader<'a> {
    fn new(data: &'a str) -> Self {
        Self { data, pos: 0 }
    }

    fn next(&mut self) -> Event {
        let rest = &self.data[self.pos..];
        if rest.is_empty() {
            return Event::End;
        }
        match rest.find('<') {
            None => {
                self.pos = self.data.len();
                Event::Text(decode_entities(rest.trim()))
            }
            Some(0) => {
                // comment, declaration or CDATA: skipped whole
                if rest.starts_with("<!--") {
                    let end = rest.find("-->").map(|i| i + 3).unwrap_or(rest.len());
                    self.pos += end;
                    return self.next();
                }
                if rest.starts_with("<?") || rest.starts_with("<!") {
                    let end = rest.find('>').map(|i| i + 1).unwrap_or(rest.len());
                    self.pos += end;
                    return self.next();
                }
                let end = match rest.find('>') {
                    Some(i) => i,
                    None => {
                        self.pos = self.data.len();
                        return Event::End;
                    }
                };
                let inner = &rest[1..end];
                self.pos += end + 1;
                Event::Tag(parse_tag(inner))
            }
            Some(i) => {
                let text = &rest[..i];
                self.pos += i;
                Event::Text(decode_entities(text.trim()))
            }
        }
    }
}

fn parse_tag(inner: &str) -> Tag {
    let closing = inner.starts_with('/');
    let inner = inner.trim_start_matches('/');
    let self_closing = inner.ends_with('/');
    let inner = inner.trim_end_matches('/').trim();
    let mut parts = inner.splitn(2, char::is_whitespace);
    let name = parts.next().unwrap_or("").to_string();
    let mut attrs = Vec::new();
    if let Some(rest) = parts.next() {
        let bytes: Vec<char> = rest.chars().collect();
        let mut i = 0;
        while i < bytes.len() {
            while i < bytes.len() && bytes[i].is_whitespace() {
                i += 1;
            }
            let start = i;
            while i < bytes.len() && bytes[i] != '=' && !bytes[i].is_whitespace() {
                i += 1;
            }
            if start == i {
                break;
            }
            let key: String = bytes[start..i].iter().collect();
            while i < bytes.len() && (bytes[i].is_whitespace() || bytes[i] == '=') {
                i += 1;
            }
            if i >= bytes.len() {
                attrs.push((key, String::new()));
                break;
            }
            let quote = bytes[i];
            let value = if quote == '"' || quote == '\'' {
                i += 1;
                let vs = i;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                let v: String = bytes[vs..i].iter().collect();
                i += 1;
                v
            } else {
                let vs = i;
                while i < bytes.len() && !bytes[i].is_whitespace() {
                    i += 1;
                }
                bytes[vs..i].iter().collect()
            };
            attrs.push((key, decode_entities(&value)));
        }
    }
    Tag { name, attrs, self_closing, closing }
}

fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#176;", "°")
        .replace("&amp;", "&")
}

/// "0x1A2B", "1234" or "-32": the numbers an XDF writes.
fn parse_num(v: &str) -> Option<i64> {
    let v = v.trim();
    let (neg, v) = match v.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, v),
    };
    let n = if let Some(hex) = v.strip_prefix("0x").or_else(|| v.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16).ok()?
    } else {
        v.parse::<i64>().ok()?
    };
    Some(if neg { -n } else { n })
}

// ───────────────────────── conversion equations ─────────────────────────

/// Evaluates the arithmetic of a TunerPro `<MATH equation>` for one value of
/// X. Supports + - * / ^, parentheses and a leading minus — what conversion
/// equations actually use.
fn eval_equation(expr: &str, x: f64) -> Option<f64> {
    let chars: Vec<char> = expr.chars().filter(|c| !c.is_whitespace()).collect();
    let mut pos = 0usize;
    let v = parse_expr(&chars, &mut pos, x)?;
    if pos != chars.len() {
        return None;
    }
    v.is_finite().then_some(v)
}

fn parse_expr(c: &[char], pos: &mut usize, x: f64) -> Option<f64> {
    let mut acc = parse_term(c, pos, x)?;
    while *pos < c.len() && (c[*pos] == '+' || c[*pos] == '-') {
        let op = c[*pos];
        *pos += 1;
        let rhs = parse_term(c, pos, x)?;
        acc = if op == '+' { acc + rhs } else { acc - rhs };
    }
    Some(acc)
}

fn parse_term(c: &[char], pos: &mut usize, x: f64) -> Option<f64> {
    let mut acc = parse_factor(c, pos, x)?;
    while *pos < c.len() && (c[*pos] == '*' || c[*pos] == '/') {
        let op = c[*pos];
        *pos += 1;
        let rhs = parse_factor(c, pos, x)?;
        if op == '*' {
            acc *= rhs;
        } else {
            if rhs == 0.0 {
                return None;
            }
            acc /= rhs;
        }
    }
    Some(acc)
}

fn parse_factor(c: &[char], pos: &mut usize, x: f64) -> Option<f64> {
    if *pos >= c.len() {
        return None;
    }
    let base = match c[*pos] {
        '-' => {
            *pos += 1;
            -parse_factor(c, pos, x)?
        }
        '+' => {
            *pos += 1;
            parse_factor(c, pos, x)?
        }
        '(' => {
            *pos += 1;
            let v = parse_expr(c, pos, x)?;
            if *pos >= c.len() || c[*pos] != ')' {
                return None;
            }
            *pos += 1;
            v
        }
        ch if ch == 'X' || ch == 'x' => {
            *pos += 1;
            x
        }
        ch if ch.is_ascii_digit() || ch == '.' => {
            let start = *pos;
            while *pos < c.len() && (c[*pos].is_ascii_digit() || c[*pos] == '.' || c[*pos] == 'e' || c[*pos] == 'E') {
                *pos += 1;
            }
            let s: String = c[start..*pos].iter().collect();
            s.parse::<f64>().ok()?
        }
        _ => return None,
    };
    if *pos < c.len() && c[*pos] == '^' {
        *pos += 1;
        let e = parse_factor(c, pos, x)?;
        return Some(base.powf(e));
    }
    Some(base)
}

/// Factor and offset of a conversion, when it is linear: `f(x) = a·x + b`
/// read from f(0) and f(1), then confirmed on f(2) and f(10). A non-linear
/// equation (a table using a polynomial) returns `linear = false` and the map
/// keeps a factor of 1 rather than showing wrong numbers.
fn linear_of(equation: &str) -> (f64, f64, bool) {
    let f0 = eval_equation(equation, 0.0);
    let f1 = eval_equation(equation, 1.0);
    let (b, f1) = match (f0, f1) {
        (Some(b), Some(f1)) => (b, f1),
        _ => return (1.0, 0.0, false),
    };
    let a = f1 - b;
    for probe in [2.0_f64, 10.0, 100.0] {
        match eval_equation(equation, probe) {
            Some(v) if (v - (a * probe + b)).abs() <= 1e-6 * probe.abs().max(1.0) => {}
            _ => return (1.0, 0.0, false),
        }
    }
    (a, b, true)
}

// ───────────────────────────── XDF reading ──────────────────────────────

fn read_embedded(tag: &Tag, defaults: &Defaults) -> Embedded {
    let mut e = Embedded {
        size_bits: defaults.size_bits,
        rows: 0,
        cols: 0,
        ..Default::default()
    };
    if let Some(v) = tag.attr("mmedaddress").and_then(parse_num) {
        if v >= 0 {
            e.address = Some(v as u32);
        }
    }
    if let Some(v) = tag.attr("mmedelementsizebits").and_then(parse_num) {
        if v > 0 {
            e.size_bits = v as u32;
        }
    }
    if let Some(v) = tag.attr("mmedrowcount").and_then(parse_num) {
        if v > 0 {
            e.rows = v as u32;
        }
    }
    if let Some(v) = tag.attr("mmedcolcount").and_then(parse_num) {
        if v > 0 {
            e.cols = v as u32;
        }
    }
    if let Some(v) = tag.attr("mmedtypeflags").and_then(parse_num) {
        e.flags = v as u32;
        e.has_flags = true;
    }
    e
}

impl Embedded {
    fn signed(&self, defaults: &Defaults) -> bool {
        if self.has_flags {
            self.flags & 0x01 != 0
        } else {
            defaults.signed
        }
    }

    fn lsb_first(&self, defaults: &Defaults) -> bool {
        if self.has_flags {
            self.flags & 0x02 != 0
        } else {
            defaults.lsb_first
        }
    }

    fn float(&self) -> bool {
        self.flags & 0x04 != 0
    }

    fn cell_bytes(&self) -> u32 {
        (self.size_bits.max(1) + 7) / 8
    }
}

/// One table or constant being read.
#[derive(Default)]
struct Table {
    title: String,
    description: String,
    categories: Vec<u32>,
    x: Option<Axis>,
    y: Option<Axis>,
    z: Option<Axis>,
    constant: bool,
}

/// Reads every map definition of an XDF. `rom_len` is the size of the binary
/// the definitions apply to: a map that would not fit is left out. Pass 0 to
/// skip that check.
pub fn parse_xdf(xml: &str, rom_len: u32) -> Vec<DetectedMap> {
    let mut reader = XmlReader::new(xml);
    let mut defaults = Defaults { signed: false, lsb_first: false, size_bits: 16 };
    let mut categories: Vec<(u32, String)> = Vec::new();
    let mut tables: Vec<Table> = Vec::new();

    // path of open tags, so text lands in the right field
    let mut current: Option<Table> = None;
    let mut axis_id = String::new();
    let mut in_axis = false;
    let mut text_target: Option<&'static str> = None;
    let mut pending_equation: Option<String> = None;

    loop {
        match reader.next() {
            Event::End => break,
            Event::Text(t) => {
                if t.is_empty() {
                    continue;
                }
                if let (Some(target), Some(tbl)) = (text_target, current.as_mut()) {
                    match (target, in_axis) {
                        ("title", false) => tbl.title = t,
                        ("description", false) => tbl.description = t,
                        ("units", true) => {
                            let ax = axis_slot(tbl, &axis_id);
                            ax.units = t;
                        }
                        ("indexcount", true) => {
                            if let Some(v) = parse_num(&t) {
                                if v > 0 {
                                    axis_slot(tbl, &axis_id).index_count = v as u32;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                text_target = None;
            }
            Event::Tag(tag) => {
                let name = tag.name.to_uppercase();
                if tag.closing {
                    match name.as_str() {
                        "XDFAXIS" => {
                            if let (Some(tbl), Some(eq)) = (current.as_mut(), pending_equation.take()) {
                                let (a, b, linear) = linear_of(&eq);
                                let ax = axis_slot(tbl, &axis_id);
                                ax.factor = a;
                                ax.offset = b;
                                ax.linear = linear;
                            }
                            in_axis = false;
                            axis_id.clear();
                        }
                        "XDFTABLE" | "XDFCONSTANT" => {
                            if let Some(mut tbl) = current.take() {
                                if name == "XDFCONSTANT" {
                                    if let Some(eq) = pending_equation.take() {
                                        let (a, b, linear) = linear_of(&eq);
                                        if let Some(z) = tbl.z.as_mut() {
                                            z.factor = a;
                                            z.offset = b;
                                            z.linear = linear;
                                        }
                                    }
                                }
                                tables.push(tbl);
                            }
                        }
                        _ => {}
                    }
                    text_target = None;
                    continue;
                }

                match name.as_str() {
                    "DEFAULTS" => {
                        defaults.signed = tag.attr("signed").map(|v| v == "1").unwrap_or(false);
                        defaults.lsb_first = tag.attr("lsbfirst").map(|v| v == "1").unwrap_or(false);
                        if let Some(v) = tag.attr("datasizeinbits").and_then(parse_num) {
                            if v > 0 {
                                defaults.size_bits = v as u32;
                            }
                        }
                    }
                    "CATEGORY" => {
                        if let (Some(i), Some(n)) = (tag.attr("index").and_then(parse_num), tag.attr("name")) {
                            categories.push((i as u32, n.to_string()));
                        }
                    }
                    "XDFTABLE" => current = Some(Table::default()),
                    "XDFCONSTANT" => {
                        let mut t = Table::default();
                        t.constant = true;
                        // a constant carries its data directly, as a 1x1 "z"
                        t.z = Some(Axis { factor: 1.0, linear: true, ..Default::default() });
                        current = Some(t);
                    }
                    "CATEGORYMEM" => {
                        if let (Some(tbl), Some(c)) = (current.as_mut(), tag.attr("category").and_then(parse_num)) {
                            tbl.categories.push(c as u32);
                        }
                    }
                    "XDFAXIS" => {
                        in_axis = true;
                        axis_id = tag.attr("id").unwrap_or("z").to_lowercase();
                        if let Some(tbl) = current.as_mut() {
                            let slot = axis_slot(tbl, &axis_id);
                            slot.factor = 1.0;
                            slot.linear = true;
                        }
                    }
                    "EMBEDDEDDATA" => {
                        let e = read_embedded(&tag, &defaults);
                        if let Some(tbl) = current.as_mut() {
                            let id = if tbl.constant && !in_axis { "z".to_string() } else { axis_id.clone() };
                            let slot = axis_slot(tbl, &id);
                            slot.embedded = e;
                        }
                    }
                    "LABEL" => {
                        if let (Some(tbl), true) = (current.as_mut(), in_axis) {
                            let v = tag.attr("value").unwrap_or("").trim().replace(',', ".");
                            let slot = axis_slot(tbl, &axis_id);
                            match v.parse::<f64>() {
                                Ok(n) if n.is_finite() => slot.labels.push(n),
                                // Un point illisible rend tout l'axe inutilisable :
                                // mieux vaut numeroter que melanger vrai et faux.
                                _ => slot.labels.push(f64::NAN),
                            }
                        }
                    }
                    "MATH" => {
                        if let Some(eq) = tag.attr("equation") {
                            pending_equation = Some(eq.to_string());
                        }
                    }
                    "TITLE" | "DESCRIPTION" | "UNITS" | "INDEXCOUNT" => {
                        if !tag.self_closing {
                            text_target = Some(match name.as_str() {
                                "TITLE" => "title",
                                "DESCRIPTION" => "description",
                                "UNITS" => "units",
                                _ => "indexcount",
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let category_name = |idx: u32| -> Option<String> {
        categories
            .iter()
            .find(|(i, _)| *i == idx)
            .map(|(_, n)| n.clone())
            .filter(|n| !n.trim().is_empty())
    };

    tables
        .into_iter()
        .filter_map(|t| to_map(t, &defaults, rom_len, &category_name))
        .collect()
}

fn axis_slot<'t>(tbl: &'t mut Table, id: &str) -> &'t mut Axis {
    let slot = match id {
        "x" => &mut tbl.x,
        "y" => &mut tbl.y,
        _ => &mut tbl.z,
    };
    slot.get_or_insert_with(|| Axis { factor: 1.0, linear: true, ..Default::default() })
}

/// Points d'un axe fixe, prets a etre affiches : autant de valeurs que la
/// grille en attend, toutes lisibles, et l'equation de l'axe appliquee.
/// `None` des qu'une condition manque — l'editeur numerote alors l'axe.
fn static_values(axis: &Axis, expected: u32) -> Option<Vec<f64>> {
    if axis.embedded.address.is_some() || axis.labels.len() != expected as usize || expected == 0 {
        return None;
    }
    if axis.labels.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let (a, b) = if axis.linear { (axis.factor, axis.offset) } else { (1.0, 0.0) };
    Some(axis.labels.iter().map(|v| v * a + b).collect())
}
fn to_map(
    t: Table,
    defaults: &Defaults,
    rom_len: u32,
    category_name: &dyn Fn(u32) -> Option<String>,
) -> Option<DetectedMap> {
    let title = t.title.trim();
    if title.is_empty() {
        return None;
    }
    let z = t.z.as_ref()?;
    let address = z.embedded.address?;
    let cell = z.embedded.cell_bytes();
    if !(1..=4).contains(&cell) {
        return None;
    }

    // Grid: what the z block declares, else what the axes count, else 1x1.
    let rows = if z.embedded.rows > 0 {
        z.embedded.rows
    } else {
        t.y.as_ref().map(|a| a.index_count).filter(|v| *v > 0).unwrap_or(1)
    };
    let cols = if z.embedded.cols > 0 {
        z.embedded.cols
    } else {
        t.x.as_ref().map(|a| a.index_count).filter(|v| *v > 0).unwrap_or(1)
    };
    if rows == 0 || cols == 0 || rows > MAX_DIM || cols > MAX_DIM {
        return None;
    }
    let size = rows as u64 * cols as u64 * cell as u64;
    if size == 0 || size > u32::MAX as u64 {
        return None;
    }
    if rom_len > 0 && address as u64 + size > rom_len as u64 {
        return None;
    }

    let signed = z.embedded.signed(defaults);
    let data_type = if z.embedded.float() && cell == 4 {
        DataType::Float32
    } else {
        match (cell, signed) {
            (1, true) => DataType::Int8,
            (1, false) => DataType::UInt8,
            (2, true) => DataType::Int16,
            (2, false) => DataType::UInt16,
            (_, true) => DataType::Int32,
            (_, false) => DataType::UInt32,
        }
    };

    let mut d = DetectedMap::new(
        address,
        size as usize,
        MapDimensions::TwoDimensional { rows: rows as usize, cols: cols as usize },
        data_type,
    );
    d.external_source = Some("XDF".to_string());
    d.name = Some(title.to_string());
    let desc = t.description.trim();
    d.description = Some(if desc.is_empty() { format!("XDF definition {title}") } else { desc.to_string() });
    let folder = t
        .categories
        .iter()
        .find_map(|c| category_name(*c))
        .unwrap_or_else(|| if t.constant { "Constants".to_string() } else { "XDF".to_string() });
    d.category = Some(folder.clone());
    d.subcategory = Some(folder);
    d.unit = Some(z.units.clone());
    // A non-linear equation is left unapplied rather than shown wrong.
    d.correction_factor = Some(if z.linear { z.factor } else { 1.0 });
    d.offset = Some(if z.linear { z.offset } else { 0.0 });
    d.confidence = 1.0;
    // TunerPro stores little-endian as a flag; Mamba reads big-endian by
    // default on EDC16-class ECUs, so the flag travels with the map.
    if z.embedded.lsb_first(defaults) {
        d.is_little_endian = Some(true);
    }

    if let Some(x) = t.x.as_ref() {
        d.x_axis_values = static_values(x, cols);
        d.x_axis_address = x.embedded.address;
        d.x_axis_correction = Some(if x.linear { x.factor } else { 1.0 });
        d.x_axis_offset = (x.linear && x.offset != 0.0).then_some(x.offset);
        d.x_label = (!x.units.trim().is_empty()).then(|| x.units.trim().to_string());
    }
    if let Some(y) = t.y.as_ref() {
        d.y_axis_values = static_values(y, rows);
        d.y_axis_address = y.embedded.address;
        d.y_axis_correction = Some(if y.linear { y.factor } else { 1.0 });
        d.y_axis_offset = (y.linear && y.offset != 0.0).then_some(y.offset);
        d.y_label = (!y.units.trim().is_empty()).then(|| y.units.trim().to_string());
    }
    Some(d)
}

/// True when the bytes look like a TunerPro definition file.
pub fn looks_like_xdf(data: &[u8]) -> bool {
    let head = &data[..data.len().min(4096)];
    let text = String::from_utf8_lossy(head).to_uppercase();
    text.contains("<XDFFORMAT") || (text.contains("<XDFHEADER") && text.contains("<XDF"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<XDFFORMAT version="1.50">
  <XDFHEADER>
    <deftitle>Test</deftitle>
    <DEFAULTS datasizeinbits="16" signed="0" lsbfirst="1" float="0" />
    <CATEGORY index="0x0" name="Boost" />
    <CATEGORY index="0x1" name="Fueling" />
  </XDFHEADER>
  <XDFTABLE uniqueid="0x1" flags="0x0">
    <title>Boost target</title>
    <description>Requested boost</description>
    <CATEGORYMEM index="0" category="0" />
    <XDFAXIS id="x">
      <EMBEDDEDDATA mmedaddress="0x1000" mmedelementsizebits="16" mmedcolcount="8" />
      <units>rpm</units>
      <indexcount>8</indexcount>
      <MATH equation="X*1"><VAR id="X" /></MATH>
    </XDFAXIS>
    <XDFAXIS id="y">
      <EMBEDDEDDATA mmedaddress="0x1010" mmedelementsizebits="16" mmedrowcount="4" />
      <units>%</units>
      <indexcount>4</indexcount>
      <MATH equation="X/10"><VAR id="X" /></MATH>
    </XDFAXIS>
    <XDFAXIS id="z">
      <EMBEDDEDDATA mmedtypeflags="0x02" mmedaddress="0x2000" mmedelementsizebits="16" mmedrowcount="4" mmedcolcount="8" />
      <units>mbar</units>
      <decimalpl>1</decimalpl>
      <MATH equation="0.1 * X + 500"><VAR id="X" /></MATH>
    </XDFAXIS>
  </XDFTABLE>
  <XDFCONSTANT uniqueid="0x2">
    <title>Rev limiter</title>
    <CATEGORYMEM index="0" category="1" />
    <EMBEDDEDDATA mmedtypeflags="0x03" mmedaddress="0x3000" mmedelementsizebits="8" />
    <units>rpm</units>
    <MATH equation="X * 40"><VAR id="X" /></MATH>
  </XDFCONSTANT>
</XDFFORMAT>"#;

    #[test]
    fn reads_a_table_with_both_axes() {
        let maps = parse_xdf(SAMPLE, 0x10000);
        let m = maps.iter().find(|m| m.name.as_deref() == Some("Boost target")).expect("table");
        assert_eq!(m.address, 0x2000);
        assert!(matches!(m.dimensions, MapDimensions::TwoDimensional { rows: 4, cols: 8 }));
        assert_eq!(m.size, 4 * 8 * 2);
        assert!(matches!(m.data_type, DataType::UInt16));
        assert_eq!(m.is_little_endian, Some(true));
        assert_eq!(m.category.as_deref(), Some("Boost"));
        assert_eq!(m.unit.as_deref(), Some("mbar"));
        assert!((m.correction_factor.unwrap() - 0.1).abs() < 1e-12);
        assert!((m.offset.unwrap() - 500.0).abs() < 1e-12);
        assert_eq!(m.x_axis_address, Some(0x1000));
        assert_eq!(m.y_axis_address, Some(0x1010));
        assert!((m.y_axis_correction.unwrap() - 0.1).abs() < 1e-12);
        assert_eq!(m.x_label.as_deref(), Some("rpm"));
        assert_eq!(m.external_source.as_deref(), Some("XDF"));
    }

    #[test]
    fn reads_a_constant_as_a_single_cell_map() {
        let maps = parse_xdf(SAMPLE, 0x10000);
        let c = maps.iter().find(|m| m.name.as_deref() == Some("Rev limiter")).expect("constant");
        assert_eq!(c.address, 0x3000);
        assert!(matches!(c.dimensions, MapDimensions::TwoDimensional { rows: 1, cols: 1 }));
        assert!(matches!(c.data_type, DataType::Int8)); // flags 0x03 = signed + LSB first
        assert_eq!(c.category.as_deref(), Some("Fueling"));
        assert!((c.correction_factor.unwrap() - 40.0).abs() < 1e-12);
    }

    #[test]
    fn skips_a_table_that_would_not_fit_in_the_binary() {
        // the same file read against a 4 KB binary: every block lands outside
        assert!(parse_xdf(SAMPLE, 0x1000).is_empty());
    }

    #[test]
    fn equations_linear_and_not() {
        assert_eq!(linear_of("X*0.01").0, 0.01);
        assert_eq!(linear_of("0.125 * X").0, 0.125);
        let (a, b, lin) = linear_of("(X-128)*0.75");
        assert!(lin && (a - 0.75).abs() < 1e-12 && (b + 96.0).abs() < 1e-12);
        let (a, b, lin) = linear_of("X*X");
        assert!(!lin && a == 1.0 && b == 0.0);
        let (_, _, lin) = linear_of("sqrt(X)");
        assert!(!lin, "an equation this module cannot read stays unapplied");
    }

    #[test]
    fn a_fixed_axis_written_in_the_file_is_kept() {
        // Axe Y ecrit en clair (comme la MLHFM d'un ME7) : ses points
        // viennent du XDF, pas du binaire.
        let xml = r#"<XDFFORMAT><XDFTABLE><title>MLHFM</title>
          <XDFAXIS id="x"><EMBEDDEDDATA mmedelementsizebits="16" /><indexcount>1</indexcount>
            <LABEL index="0" value="" /><MATH equation="X" /></XDFAXIS>
          <XDFAXIS id="y"><EMBEDDEDDATA mmedelementsizebits="16" /><indexcount>3</indexcount>
            <LABEL index="0" value="0.0000" /><LABEL index="1" value="2.5000" />
            <LABEL index="2" value="5.0000" /><units>V</units><MATH equation="X" /></XDFAXIS>
          <XDFAXIS id="z"><EMBEDDEDDATA mmedtypeflags="0x02" mmedaddress="0x100"
            mmedelementsizebits="16" mmedrowcount="3" /><units>kg/h</units>
            <MATH equation="0.125 * X" /></XDFAXIS>
          </XDFTABLE></XDFFORMAT>"#;
        let maps = parse_xdf(xml, 0x10000);
        assert_eq!(maps.len(), 1);
        let m = &maps[0];
        assert_eq!(m.y_axis_values, Some(vec![0.0, 2.5, 5.0]));
        assert_eq!(m.y_axis_address, None);
        // Le point vide de l'axe X n'est pas un nombre : l'axe reste numerote
        assert_eq!(m.x_axis_values, None);
    }

    #[test]
    fn recognises_the_format() {
        assert!(looks_like_xdf(SAMPLE.as_bytes()));
        assert!(!looks_like_xdf(b"{\"maps\": []}"));
        assert!(!looks_like_xdf(&[0u8; 512]));
    }

    #[test]
    fn a_table_without_a_z_address_is_skipped() {
        let xml = r#"<XDFFORMAT><XDFTABLE><title>No data</title>
          <XDFAXIS id="z"><EMBEDDEDDATA mmedelementsizebits="16" mmedrowcount="4" /><MATH equation="X" /></XDFAXIS>
          </XDFTABLE></XDFFORMAT>"#;
        assert!(parse_xdf(xml, 0x10000).is_empty());
    }
}
