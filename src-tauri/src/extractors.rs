use std::io::Read;
use zip::ZipArchive;

const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "tiff", "tif", "bmp", "gif", "webp",
];

const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "csv", "md", "rtf", "json", "xml", "html", "htm",
];

/// OOXML (`docx|xlsx|pptx|pptm`) plus the legacy binary trio. Legacy Office
/// files have no local extractor (`extract_text_from_bytes` returns an error
/// for them) and fall through to vision AI.
const OFFICE_EXTENSIONS: &[&str] = &["docx", "xlsx", "pptx", "pptm", "doc", "xls", "ppt"];

pub fn is_image_extension(path: &str) -> bool {
    let ext = std::path::Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    IMAGE_EXTENSIONS.contains(&ext.as_str())
}

pub fn is_text_extension(path: &str) -> bool {
    let ext = std::path::Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    TEXT_EXTENSIONS.contains(&ext.as_str())
}

pub fn is_office_extension(path: &str) -> bool {
    let ext = std::path::Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    OFFICE_EXTENSIONS.contains(&ext.as_str())
}

pub fn is_pdf_extension(path: &str) -> bool {
    let ext = std::path::Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    ext == "pdf"
}

/// Canonical list of handled extensions (no dot), derived from the sets the
/// extractors actually understand. The front end must stay in sync with this
/// list; `get_supported_extensions_list` exposes it over IPC.
pub fn supported_extensions() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    out.extend_from_slice(IMAGE_EXTENSIONS);
    out.extend_from_slice(TEXT_EXTENSIONS);
    out.extend_from_slice(OFFICE_EXTENSIONS);
    out.push("pdf");
    out
}

/// Whether a path's extension is one this app can process. Used to reject
/// files the extractors cannot read instead of silently sending them to
/// vision AI (the pipeline consults this before dispatch).
pub fn is_supported_extension(path: &str) -> bool {
    let ext = std::path::Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    supported_extensions().contains(&ext.as_str())
}

/// Assess text quality on a 0.0-1.0 scale.
///
/// Ratio is alphabetic *characters* over *characters* (not bytes) so
/// non-Latin scripts (Arabic, CJK, accented Latin) are not penalised for
/// their UTF-8 width, and the bonuses only apply to real documents.
pub fn assess_text_quality(text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }

    let char_count = text.chars().count() as f64;
    let alpha_count = text.chars().filter(|c| c.is_alphabetic()).count() as f64;
    let alpha_ratio = alpha_count / char_count;

    let word_count = text.split_whitespace().count() as f64;
    // Scale the word bonus with substance: tiny word counts don't max it out.
    let word_bonus = (word_count / 50.0).min(0.2);

    let newline_count = text.chars().filter(|c| *c == '\n').count() as f64;
    let newline_bonus = if newline_count > 2.0 { 0.1 } else { 0.0 };

    (alpha_ratio * 0.7 + word_bonus + newline_bonus).min(1.0)
}

// ---------------------------------------------------------------------------
// XML plumbing shared by the Office extractors
// ---------------------------------------------------------------------------

/// `(name, is_closing)` for an XML tag body such as `w:t xml:space="preserve"`
/// or `/w:p`. Also treats processing instructions and comments as unnamed.
fn tag_name(tag: &str) -> (&str, bool) {
    let closing = tag.starts_with('/');
    let body = tag
        .trim_start_matches('/')
        .trim_start_matches('?')
        .trim_start_matches('!');
    let name = body
        .split([' ', '\t', '\n', '\r', '/', '>'])
        .next()
        .unwrap_or("");
    (name, closing)
}

/// Read `attr="value"` from a tag body (`<c r="B2" t="s">` -> `Some("s")`).
fn attr_value(tag: &str, attr: &str) -> Option<String> {
    let mut rest = tag;
    while let Some(pos) = rest.find(attr) {
        let starts_name = pos == 0 || rest[..pos].ends_with([' ', '\t']);
        let after = &rest[pos + attr.len()..];
        if starts_name {
            if let Some(value) = after.strip_prefix('=').and_then(|r| r.strip_prefix('"')) {
                if let Some(end) = value.find('"') {
                    return Some(value[..end].to_string());
                }
            }
        }
        rest = &rest[pos + attr.len()..];
    }
    None
}

/// Decode the XML entities OOXML writers emit inside text nodes
/// (`&amp; &lt; &gt; &quot; &apos; &nbsp;` and numeric references).
fn decode_xml_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(pos) = rest.find('&') {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 1..];
        let decoded = match after.find(';') {
            Some(end) if end <= 10 => {
                let entity = &after[..end];
                match entity {
                    "amp" => Some("&".to_string()),
                    "lt" => Some("<".to_string()),
                    "gt" => Some(">".to_string()),
                    "quot" => Some("\"".to_string()),
                    "apos" => Some("'".to_string()),
                    "nbsp" => Some(" ".to_string()),
                    _ => entity
                        .strip_prefix('#')
                        .and_then(|code| {
                            code.strip_prefix(['x', 'X'])
                                .and_then(|h| u32::from_str_radix(h, 16).ok())
                                .or_else(|| code.parse::<u32>().ok())
                        })
                        .and_then(char::from_u32)
                        .map(|c| c.to_string()),
                }
            }
            _ => None,
        };
        match decoded {
            Some(text) => {
                out.push_str(&text);
                // Skip past the semicolon we consumed.
                let consumed = after
                    .find(';')
                    .filter(|end| *end <= 10)
                    .map(|end| end + 1)
                    .unwrap_or(0);
                rest = &after[consumed..];
            }
            None => {
                // Bare ampersand (or entity too long to be one): keep it.
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Collapse runs of 3+ newlines and trim the ends, so extracted text stays
/// compact enough for the AI prompt.
fn tidy_blank_lines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut newlines = 0usize;
    let mut started = false;
    for ch in text.chars() {
        if ch == '\n' {
            if !started {
                continue; // drop leading blank lines
            }
            newlines += 1;
            if newlines <= 2 {
                out.push(ch);
            }
        } else {
            newlines = 0;
            started = true;
            out.push(ch);
        }
    }
    while out.ends_with(['\n', ' ', '\t']) {
        out.pop();
    }
    out
}

/// Collect text from `<{text_tag}>` runs, emitting a newline after every
/// closing (or empty) `<{para_tag}>`. Shared by DOCX (`w:t`/`w:p`) and PPTX
/// (`a:t`/`a:p`).
///
/// This replaces a generic tag-stripper that joined *every* text node with a
/// single space: paragraph structure was lost, so the quality scorer saw one
/// giant line, and `&amp;` reached the AI still encoded.
fn extract_tag_runs(xml: &str, text_tag: &str, para_tag: &str) -> String {
    let mut out = String::new();
    let mut run = String::new();
    let mut tag_buf = String::new();
    let mut in_tag = false;
    let mut in_text = false;

    for ch in xml.chars() {
        match ch {
            '<' => {
                if in_text {
                    out.push_str(&decode_xml_entities(&run));
                    run.clear();
                    in_text = false;
                }
                in_tag = true;
                tag_buf.clear();
            }
            '>' => {
                in_tag = false;
                let tag = tag_buf.trim();
                let self_closing = tag.ends_with('/');
                let (name, closing) = tag_name(tag);
                if name == text_tag {
                    in_text = !closing && !self_closing;
                } else if name == para_tag && (closing || self_closing) {
                    out.push('\n');
                } else if !closing {
                    // Tab and explicit break elements sit between runs.
                    if name == "w:tab" || name == "a:tab" {
                        out.push('\t');
                    } else if name == "w:br" || name == "a:br" {
                        out.push('\n');
                    }
                }
                tag_buf.clear();
            }
            _ if in_tag => tag_buf.push(ch),
            _ if in_text => run.push(ch),
            _ => {}
        }
    }
    if in_text {
        out.push_str(&decode_xml_entities(&run));
    }
    tidy_blank_lines(&out)
}

// ---------------------------------------------------------------------------
// DOCX / PPTX
// ---------------------------------------------------------------------------

/// Extract text from a DOCX file (ZIP containing word/document.xml).
pub fn extract_text_from_docx(bytes: &[u8]) -> Result<String, String> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| format!("Failed to open DOCX as ZIP: {}", e))?;

    let xml = match archive.by_name("word/document.xml") {
        Ok(mut file) => {
            let mut raw = Vec::new();
            file.read_to_end(&mut raw)
                .map_err(|e| format!("Failed to read document.xml: {}", e))?;
            decode_bytes_to_string(&raw)
        }
        Err(_) => return Err("word/document.xml not found in DOCX".to_string()),
    };
    Ok(extract_tag_runs(&xml, "w:t", "w:p"))
}

/// Extract text from a PPTX file (ZIP containing ppt/slides/slide*.xml).
pub fn extract_text_from_pptx(bytes: &[u8]) -> Result<String, String> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| format!("Failed to open PPTX as ZIP: {}", e))?;

    let mut all_text = Vec::new();
    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let name = file.name().to_string();
        if !(name.starts_with("ppt/slides/slide") && name.ends_with(".xml")) {
            continue;
        }
        let mut raw = Vec::new();
        if file.read_to_end(&mut raw).is_err() {
            return Err(format!("Failed to read {}", name));
        }
        let xml = decode_bytes_to_string(&raw);
        let slide_text = extract_tag_runs(&xml, "a:t", "a:p");
        if !slide_text.is_empty() {
            all_text.push(slide_text);
        }
    }

    Ok(all_text.join("\n"))
}

// ---------------------------------------------------------------------------
// XLSX
// ---------------------------------------------------------------------------

/// Parse `<si>` blocks from sharedStrings.xml into indexed strings.
///
/// Each `<si>` is exactly one cell string; rich-text entries split the string
/// across several `<t>` runs that must be concatenated. Empty entries are
/// kept (as `""`) because worksheet cells reference strings by *index* — a
/// skipped `<si>` would shift every later cell's text onto the wrong string.
fn extract_shared_strings(xml: &str) -> Vec<String> {
    let mut strings = Vec::new();
    let mut current = String::new();
    let mut run = String::new();
    let mut tag_buf = String::new();
    let mut in_tag = false;
    let mut in_si = false;
    let mut in_t = false;

    for ch in xml.chars() {
        match ch {
            '<' => {
                if in_t {
                    current.push_str(&decode_xml_entities(&run));
                    run.clear();
                    in_t = false;
                }
                in_tag = true;
                tag_buf.clear();
            }
            '>' => {
                in_tag = false;
                let tag = tag_buf.trim();
                let self_closing = tag.ends_with('/');
                let (name, closing) = tag_name(tag);
                match name {
                    "si" => {
                        if closing || self_closing {
                            strings.push(std::mem::take(&mut current));
                            in_si = false;
                        } else {
                            in_si = true;
                            current.clear();
                        }
                    }
                    "t" => in_t = !closing && !self_closing && in_si,
                    _ => {}
                }
                tag_buf.clear();
            }
            _ if in_tag => tag_buf.push(ch),
            _ if in_t => run.push(ch),
            _ => {}
        }
    }
    if in_t {
        current.push_str(&decode_xml_entities(&run));
    }
    if in_si && !current.is_empty() {
        strings.push(std::mem::take(&mut current));
    }
    strings
}

/// Extract cell text from an XLSX worksheet XML (rows tab-separated).
///
/// Cell type is read from the `<c>` element's `t` attribute: only `t="s"`
/// cells index into sharedStrings. The previous scanner matched
/// `tag.starts_with("v")`, which also opened on `<sheetView>` and friends,
/// and it sniffed `t="s"` anywhere in a tag (so a cell whose style attribute
/// contained that substring, or a leftover value from the previous cell,
/// produced wrong text).
fn extract_sheet_text(xml: &str, shared_strings: &[String]) -> String {
    let mut rows: Vec<String> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut value = String::new();
    let mut tag_buf = String::new();
    let mut in_tag = false;
    let mut in_value = false;
    let mut cell_shared = false;
    let mut cell_type = String::new();

    let push_value = |cell_shared: bool,
                      value: &mut String,
                      row: &mut Vec<String>,
                      shared_strings: &[String]| {
        let text = std::mem::take(value);
        if text.is_empty() {
            return;
        }
        if cell_shared {
            if let Ok(idx) = text.trim().parse::<usize>() {
                if let Some(s) = shared_strings.get(idx) {
                    row.push(decode_xml_entities(s));
                }
            }
        } else {
            row.push(text);
        }
    };

    for ch in xml.chars() {
        match ch {
            '<' => {
                if in_value {
                    push_value(cell_shared, &mut value, &mut row, shared_strings);
                    in_value = false;
                }
                in_tag = true;
                tag_buf.clear();
            }
            '>' => {
                in_tag = false;
                let tag = tag_buf.trim();
                let self_closing = tag.ends_with('/');
                let (name, closing) = tag_name(tag);
                match name {
                    "row" => {
                        if closing || self_closing {
                            if !row.is_empty() {
                                rows.push(row.join("\t"));
                                row.clear();
                            }
                        } else {
                            row.clear();
                        }
                    }
                    "c" => {
                        if closing || self_closing {
                            cell_type.clear();
                            cell_shared = false;
                        } else {
                            cell_type = attr_value(tag, "t").unwrap_or_default();
                            cell_shared = cell_type == "s";
                        }
                    }
                    "v" | "t" => {
                        // `<v>` holds numbers or shared-string indices;
                        // `<is><t>` holds inline strings (never indexed).
                        if closing || self_closing {
                            if in_value {
                                push_value(cell_shared, &mut value, &mut row, shared_strings);
                                in_value = false;
                                cell_shared = false;
                            }
                        } else {
                            value.clear();
                            in_value = true;
                            if name == "t" {
                                cell_shared = false;
                            }
                        }
                    }
                    _ => {}
                }
                tag_buf.clear();
            }
            _ if in_tag => tag_buf.push(ch),
            _ if in_value => value.push(ch),
            _ => {}
        }
    }
    if in_value {
        push_value(cell_shared, &mut value, &mut row, shared_strings);
    }
    if !row.is_empty() {
        rows.push(row.join("\t"));
    }
    rows.join("\n")
}

/// Extract text from an XLSX file (ZIP containing xl/sharedStrings.xml and
/// the worksheet parts).
pub fn extract_text_from_xlsx(bytes: &[u8]) -> Result<String, String> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| format!("Failed to open XLSX as ZIP: {}", e))?;

    let mut shared_strings: Vec<String> = Vec::new();
    if let Ok(mut file) = archive.by_name("xl/sharedStrings.xml") {
        let mut raw = Vec::new();
        file.read_to_end(&mut raw)
            .map_err(|e| format!("Failed to read sharedStrings.xml: {}", e))?;
        shared_strings = extract_shared_strings(&decode_bytes_to_string(&raw));
    }

    let mut all_text = Vec::new();
    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let name = file.name().to_string();
        if !(name.starts_with("xl/worksheets/sheet") && name.ends_with(".xml")) {
            continue;
        }
        let mut raw = Vec::new();
        if file.read_to_end(&mut raw).is_err() {
            return Err(format!("Failed to read {}", name));
        }
        let xml = decode_bytes_to_string(&raw);
        let sheet_text = extract_sheet_text(&xml, &shared_strings);
        if !sheet_text.is_empty() {
            all_text.push(sheet_text);
        }
    }

    Ok(all_text.join("\n"))
}

// ---------------------------------------------------------------------------
// PDF
// ---------------------------------------------------------------------------

/// Pages parsed before giving up. Long enough for any real letter/invoice
/// batch, short enough to keep a 500-page dump from stalling a low-end PC.
const MAX_PDF_PAGES: usize = 200;

/// Extract text from a PDF using lopdf.
pub fn extract_text_from_pdf(bytes: &[u8]) -> Result<String, String> {
    let doc = lopdf::Document::load_mem(bytes).map_err(|e| format!("Failed to parse PDF: {}", e))?;

    let mut all_text = Vec::new();
    let page_count = doc.get_pages().len();
    if page_count > MAX_PDF_PAGES {
        tracing::warn!(
            "PDF has {} pages — extracting text from the first {} only",
            page_count,
            MAX_PDF_PAGES
        );
    }

    for (_, page_id) in doc.get_pages().into_iter().take(MAX_PDF_PAGES) {
        if let Ok(text) = extract_pdf_page_text(&doc, page_id) {
            if !text.is_empty() {
                all_text.push(text);
            }
        }
    }

    Ok(all_text.join("\n"))
}

/// Show-text operators whose operand carries text: `Tj`, `TJ`, `'`, `"`.
fn is_text_show_operator(op: &str) -> bool {
    matches!(op, "Tj" | "TJ" | "'" | "\"")
}

/// Line-positioning operators that mean "start a new line of text".
fn is_line_break_operator(op: &str) -> bool {
    matches!(op, "Td" | "TD" | "T*" | "ET")
}

/// Pull text out of one page's content stream.
///
/// This walks `Tj`/`TJ`/`'`/`"` operators (the old version missed `'`/`"`
/// entirely) and turns the text-positioning operators into newlines so the
/// result keeps some line structure — the quality scorer and the AI both
/// depend on it. PDFs whose fonts encode text as CID glyphs with custom
/// mappings still yield mojibake here; that is why poor-quality text falls
/// back to vision AI in the pipeline.
fn extract_pdf_page_text(doc: &lopdf::Document, page_id: lopdf::ObjectId) -> Result<String, String> {
    let content = doc.get_page_content(page_id).map_err(|e| e.to_string())?;

    let mut text = String::new();

    if let Ok(ops) = lopdf::content::Content::decode(&content) {
        for op in &ops.operations {
            if is_line_break_operator(&op.operator) {
                if !text.ends_with('\n') && !text.is_empty() {
                    text.push('\n');
                }
                continue;
            }
            if !is_text_show_operator(&op.operator) {
                continue;
            }
            let mut wrote = false;
            for arg in &op.operands {
                match arg {
                    lopdf::Object::String(s, _) => {
                        text.push_str(&String::from_utf8_lossy(s.as_ref()));
                        wrote = true;
                    }
                    lopdf::Object::Array(arr) => {
                        for item in arr {
                            if let lopdf::Object::String(s, _) = item {
                                text.push_str(&String::from_utf8_lossy(s.as_ref()));
                                wrote = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
            if wrote {
                text.push(' ');
            }
            // `'` and `"` also imply a line break before showing the text.
            if op.operator == "'" || op.operator == "\"" {
                text.push('\n');
            }
        }
    }

    Ok(tidy_blank_lines(&text))
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Auto-detect file type from the path and extract text from `bytes`.
///
/// Takes the already-read buffer rather than a path: the pipeline reads each
/// file once (to enforce the size limit and to feed vision AI) and the old
/// signature re-read it from disk a second time for every document.
///
/// Returns (text, quality_score, method_used).
pub fn extract_text_from_bytes(
    path: &str,
    bytes: &[u8],
) -> Result<(String, f64, String), String> {
    let ext = std::path::Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let (text, method) = match ext.as_str() {
        "txt" | "md" | "rtf" | "json" | "xml" | "html" | "htm" => {
            (decode_bytes_to_string(bytes), "text_read".to_string())
        }
        "csv" => (decode_bytes_to_string(bytes), "csv_read".to_string()),
        "docx" => (extract_text_from_docx(bytes)?, "docx_extract".to_string()),
        "xlsx" => (extract_text_from_xlsx(bytes)?, "xlsx_extract".to_string()),
        "pptx" | "pptm" => (extract_text_from_pptx(bytes)?, "pptx_extract".to_string()),
        "pdf" => (extract_text_from_pdf(bytes)?, "pdf_extract".to_string()),
        "doc" | "xls" | "ppt" => {
            return Err(format!(
                "Legacy Office format '{}' requires vision AI for text extraction",
                ext
            ));
        }
        _ => {
            return Err(format!(
                "Unsupported file type for local extraction: {}",
                ext
            ));
        }
    };

    let quality = assess_text_quality(&text);
    Ok((text, quality, method))
}

/// Decode bytes to string using BOM detection and encoding sniffing.
///
/// BOM checks run *before* the UTF-8 validation: `from_utf8` happily accepts
/// a leading UTF-8 BOM (leaking U+FEFF into the text) and UTF-16 files pass
/// it as garbage, which previously fell through to the Latin-1 decoder and
/// produced mojibake for every UTF-16 `.txt`/`.csv` on Windows.
pub fn decode_bytes_to_string(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(&bytes[3..]).into_owned();
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        // UTF-16LE (or UTF-32LE, rare); encoding_rs strips the BOM.
        let (cow, _, _) = encoding_rs::UTF_16LE.decode(bytes);
        return cow.into_owned();
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let (cow, _, _) = encoding_rs::UTF_16BE.decode(bytes);
        return cow.into_owned();
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_string();
    }
    encoding_rs::mem::decode_latin1(bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_strings_concatenate_rich_runs_and_keep_empty_entries() {
        let xml = r#"<?xml version="1.0"?>
<sst><si><t>Hello</t></si><si><t>Par</t><t>tial</t></si><si/><si><t>A &amp; B</t></si></sst>"#;
        let got = extract_shared_strings(xml);
        assert_eq!(got, vec!["Hello", "Partial", "", "A & B"]);
    }

    #[test]
    fn sheet_text_resolves_shared_and_inline_cells_per_row() {
        let shared = vec!["Acme Corp".to_string(), "Invoice".to_string()];
        let xml = r#"<worksheet><sheetViews><sheetView/></sheetViews><sheetData>
<row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c><c r="C1"><v>120.50</v></c></row>
<row r="2"><c r="A2" t="inlineStr"><is><t>VAT 20%</t></is></c></row>
</sheetData></worksheet>"#;
        let got = extract_sheet_text(xml, &shared);
        assert_eq!(got, "Acme Corp\tInvoice\t120.50\nVAT 20%");
    }

    #[test]
    fn sheet_text_is_not_confused_by_view_tags() {
        // `<sheetView>`/`<view>` must not be read as a `<v>` value.
        let xml = r#"<worksheet><sheetViews><sheetView tabSelected="1"/></sheetViews>
<sheetData><row><c t="s"><v>0</v></c></row></sheetData></worksheet>"#;
        let got = extract_sheet_text(xml, &["Real".to_string()]);
        assert_eq!(got, "Real");
    }

    #[test]
    fn docx_runs_keep_paragraph_breaks_and_decode_entities() {
        let xml = r#"<w:document><w:body>
<w:p><w:r><w:t xml:space="preserve">To &amp; From</w:t></w:r></w:p>
<w:p><w:r><w:t>Line two</w:t></w:r></w:p>
</w:body></w:document>"#;
        let got = extract_tag_runs(xml, "w:t", "w:p");
        assert!(got.contains("To & From"), "got: {}", got);
        assert!(got.contains('\n'), "paragraphs must break: {}", got);
        assert!(got.contains("Line two"));
    }

    #[test]
    fn empty_text_tag_does_not_swallow_the_document() {
        let xml = r#"<w:document><w:p><w:t/><w:r><w:t>Visible</w:t></w:r></w:p></w:document>"#;
        assert_eq!(extract_tag_runs(xml, "w:t", "w:p"), "Visible");
    }

    #[test]
    fn entity_decoder_handles_numeric_and_unknown() {
        assert_eq!(decode_xml_entities("a&#65;b"), "aAb");
        assert_eq!(decode_xml_entities("a&#x417;b"), "aঅb");
        assert_eq!(decode_xml_entities("Tom & Jerry"), "Tom & Jerry");
        assert_eq!(decode_xml_entities("&notanentity;"), "&notanentity;");
    }

    #[test]
    fn quality_is_script_agnostic() {
        let latin = assess_text_quality("Invoice for services rendered today");
        let arabic = assess_text_quality("فاتورة عن الخدمات المقدمة اليوم");
        assert!(latin > 0.5, "latin quality {}", latin);
        assert!(arabic > 0.5, "arabic quality {}", arabic);
        assert_eq!(assess_text_quality(""), 0.0);
    }

    #[test]
    fn decode_boms_before_validating_utf8() {
        let utf8_bom = [0xEF, 0xBB, 0xBF];
        let text = decode_bytes_to_string(&[utf8_bom.as_slice(), "hello".as_bytes()].concat());
        assert_eq!(text, "hello");

        let mut utf16le: Vec<u8> = vec![0xFF, 0xFE];
        for u in "Hi".encode_utf16() {
            utf16le.extend_from_slice(&u.to_le_bytes());
        }
        assert_eq!(decode_bytes_to_string(&utf16le), "Hi");

        assert_eq!(decode_bytes_to_string(b"plain"), "plain");
    }

    // -----------------------------------------------------------------------
    // Extension classification (kept in sync with the frontend list)
    // -----------------------------------------------------------------------

    #[test]
    fn extension_classification_is_case_insensitive() {
        assert!(is_image_extension("A.PNG"));
        assert!(is_image_extension("photo.jpeg"));
        assert!(!is_image_extension("doc.pdf"));

        assert!(is_pdf_extension("Scan.PDF"));
        assert!(!is_pdf_extension("doc.docx"));

        assert!(is_office_extension("Sheet.XLSX"));
        assert!(is_office_extension("slides.pptm"));
        assert!(!is_office_extension("a.txt"));

        assert!(is_text_extension("notes.MD"));
        assert!(!is_text_extension("a.pdf"));
    }

    #[test]
    fn supported_extensions_are_unique_and_match_the_classifiers() {
        let list = supported_extensions();
        let mut sorted = list.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), list.len(), "duplicate extension in {list:?}");

        for ext in &list {
            let probe = format!("file.{}", ext);
            assert!(
                is_image_extension(&probe)
                    || is_text_extension(&probe)
                    || is_office_extension(&probe)
                    || is_pdf_extension(&probe),
                "{} is advertised but not classified",
                ext
            );
            assert!(is_supported_extension(&probe));
        }

        assert!(!is_supported_extension("archive.zip"));
        assert!(!is_supported_extension("noextension"));
    }

    // -----------------------------------------------------------------------
    // Local extraction entry point
    // -----------------------------------------------------------------------

    #[test]
    fn extract_text_from_bytes_handles_plain_text_and_reports_quality() {
        let text = "Invoice for consulting services rendered in March 2024\nAcme Corp\n";
        let (got, quality, method) =
            extract_text_from_bytes("notes.txt", text.as_bytes()).unwrap();
        assert_eq!(got, text);
        assert_eq!(method, "text_read");
        assert!(quality > 0.5, "quality {}", quality);
    }

    #[test]
    fn extract_text_from_bytes_rejects_legacy_office_and_unknown_types() {
        let legacy = extract_text_from_bytes("old.doc", b"\xd0\xcf\x11\xe0").unwrap_err();
        assert!(legacy.contains("Legacy Office"), "got: {}", legacy);

        let unknown = extract_text_from_bytes("archive.zip", b"PK\x03\x04").unwrap_err();
        assert!(unknown.contains("Unsupported"), "got: {}", unknown);
    }

    #[test]
    fn extract_text_from_bytes_reports_a_corrupt_zip_as_an_error() {
        // A `.docx` that is not actually a ZIP must fail loudly so the
        // pipeline can fall back to vision instead of renaming on empty text.
        assert!(extract_text_from_bytes("broken.docx", b"not a zip").is_err());
        assert!(extract_text_from_bytes("broken.xlsx", b"not a zip").is_err());
        assert!(extract_text_from_bytes("broken.pptx", b"not a zip").is_err());
    }

    #[test]
    fn quality_scoring_penalises_punctuation_only_text() {
        assert!(assess_text_quality("....----....") < 0.2);
        assert!(assess_text_quality("Lorem ipsum dolor sit amet") > 0.5);
    }
}
