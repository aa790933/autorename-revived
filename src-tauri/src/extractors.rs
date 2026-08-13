use std::io::Read;
use zip::ZipArchive;

const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "tiff", "tif", "bmp", "gif", "webp",
];

const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "csv", "md", "rtf", "json", "xml", "html", "htm",
];

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

/// Assess text quality on a 0.0-1.0 scale.
pub fn assess_text_quality(text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }

    let len = text.len() as f64;
    let alpha_count = text.chars().filter(|c| c.is_alphabetic()).count() as f64;
    let alpha_ratio = alpha_count / len;

    let word_count = text.split_whitespace().count() as f64;
    let word_bonus = if word_count > 5.0 { 0.2 } else { 0.0 };

    let newline_count = text.chars().filter(|c| *c == '\n').count() as f64;
    let newline_bonus = if newline_count > 2.0 { 0.1 } else { 0.0 };

    let quality = (alpha_ratio * 0.7 + word_bonus + newline_bonus).min(1.0);
    quality
}

/// Extract text from a DOCX file (ZIP containing word/document.xml).
pub fn extract_text_from_docx(bytes: &[u8]) -> Result<String, String> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| format!("Failed to open DOCX as ZIP: {}", e))?;

    let mut xml_content = String::new();

    if let Ok(mut file) = archive.by_name("word/document.xml") {
        file.read_to_string(&mut xml_content)
            .map_err(|e| format!("Failed to read document.xml: {}", e))?;
    } else {
        return Err("word/document.xml not found in DOCX".to_string());
    }

    let text = extract_xml_text(&xml_content);
    Ok(text)
}

/// Extract text from an XLSX file (ZIP containing xl/sharedStrings.xml and worksheets).
pub fn extract_text_from_xlsx(bytes: &[u8]) -> Result<String, String> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| format!("Failed to open XLSX as ZIP: {}", e))?;

    let mut shared_strings: Vec<String> = Vec::new();

    if let Ok(mut file) = archive.by_name("xl/sharedStrings.xml") {
        let mut xml = String::new();
        file.read_to_string(&mut xml)
            .map_err(|e| format!("Failed to read sharedStrings.xml: {}", e))?;
        shared_strings = extract_shared_strings(&xml);
    }

    let mut all_text = Vec::new();

    for i in 0..archive.len() {
        let name = archive.by_index(i).map_err(|e| e.to_string())?.name().to_string();
        if name.starts_with("xl/worksheets/sheet") && name.ends_with(".xml") {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
            let mut xml = String::new();
            file.read_to_string(&mut xml)
                .map_err(|e| format!("Failed to read {}: {}", name, e))?;
            let sheet_text = extract_sheet_text(&xml, &shared_strings);
            if !sheet_text.is_empty() {
                all_text.push(sheet_text);
            }
        }
    }

    Ok(all_text.join("\n"))
}

/// Extract text from a PPTX file (ZIP containing ppt/slides/slide*.xml).
pub fn extract_text_from_pptx(bytes: &[u8]) -> Result<String, String> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| format!("Failed to open PPTX as ZIP: {}", e))?;

    let mut all_text = Vec::new();

    for i in 0..archive.len() {
        let name = archive.by_index(i).map_err(|e| e.to_string())?.name().to_string();
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
            let mut xml = String::new();
            file.read_to_string(&mut xml)
                .map_err(|e| format!("Failed to read {}: {}", name, e))?;
            let slide_text = extract_xml_text(&xml);
            if !slide_text.is_empty() {
                all_text.push(slide_text);
            }
        }
    }

    Ok(all_text.join("\n"))
}

/// Extract text from a PDF using lopdf.
pub fn extract_text_from_pdf(bytes: &[u8]) -> Result<String, String> {
    let doc = lopdf::Document::load_mem(bytes)
        .map_err(|e| format!("Failed to parse PDF: {}", e))?;

    let mut all_text = Vec::new();

    let pages = doc.get_pages();
    for (_, page_id) in pages {
        if let Ok(text) = extract_pdf_page_text(&doc, page_id) {
            if !text.is_empty() {
                all_text.push(text);
            }
        }
    }

    Ok(all_text.join("\n"))
}

fn extract_pdf_page_text(doc: &lopdf::Document, page_id: lopdf::ObjectId) -> Result<String, String> {
    let content = doc.get_page_content(page_id).map_err(|e| e.to_string())?;

    let mut text = String::new();

    if let Ok(ops) = lopdf::content::Content::decode(&content) {
        for op in &ops.operations {
            if op.operator == "Tj" || op.operator == "TJ" {
                if let Some(args) = op.operands.first() {
                    match args {
                        lopdf::Object::String(s, _) => {
                            text.push_str(&String::from_utf8_lossy(s.as_ref()));
                            text.push(' ');
                        }
                        lopdf::Object::Array(arr) => {
                            for item in arr {
                                if let lopdf::Object::String(s, _) = item {
                                    text.push_str(&String::from_utf8_lossy(s.as_ref()));
                                }
                            }
                            text.push(' ');
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(text.trim().to_string())
}

/// Extract text content from XML by stripping tags and collecting text nodes.
fn extract_xml_text(xml: &str) -> String {
    let mut text = Vec::new();
    let mut in_tag = false;
    let mut current = String::new();

    for ch in xml.chars() {
        match ch {
            '<' => {
                in_tag = true;
                if !current.is_empty() {
                    let trimmed = current.trim();
                    if !trimmed.is_empty() {
                        text.push(trimmed.to_string());
                    }
                    current.clear();
                }
            }
            '>' => {
                in_tag = false;
            }
            _ if !in_tag => {
                current.push(ch);
            }
            _ => {}
        }
    }

    if !current.is_empty() {
        let trimmed = current.trim();
        if !trimmed.is_empty() {
            text.push(trimmed.to_string());
        }
    }

    text.join(" ")
}

/// Extract shared strings from XLSX sharedStrings.xml.
fn extract_shared_strings(xml: &str) -> Vec<String> {
    let mut strings = Vec::new();
    let mut in_t = false;
    let mut current = String::new();

    for ch in xml.chars() {
        match ch {
            '<' => {
                if !current.is_empty() && in_t {
                    strings.push(current.clone());
                    current.clear();
                }
                in_t = false;
                current.clear();
            }
            '>' => {
                in_t = false;
                current.clear();
            }
            _ => {
                if in_t {
                    current.push(ch);
                }
            }
        }
    }

    if strings.is_empty() {
        let stripped = extract_xml_text(xml);
        return stripped
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
    }

    strings
}

/// Extract cell text from an XLSX worksheet XML.
fn extract_sheet_text(xml: &str, shared_strings: &[String]) -> String {
    let mut rows = Vec::new();
    let mut current_row = Vec::new();
    let mut in_v = false;
    let mut value_buf = String::new();
    let mut cell_type = String::new();

    let mut tag_buf = String::new();
    let mut in_tag = false;

    for ch in xml.chars() {
        match ch {
            '<' => {
                in_tag = true;
                tag_buf.clear();
            }
            '>' => {
                in_tag = false;
                let tag = tag_buf.trim();

                if tag == "/v" {
                    if !value_buf.is_empty() {
                        if cell_type == "s" {
                            if let Ok(idx) = value_buf.trim().parse::<usize>() {
                                if let Some(s) = shared_strings.get(idx) {
                                    current_row.push(s.clone());
                                }
                            }
                        } else {
                            current_row.push(value_buf.clone());
                        }
                        value_buf.clear();
                        cell_type.clear();
                    }
                } else if tag.starts_with("v") && !tag.starts_with("/") {
                    in_v = true;
                } else if tag.contains("t=\"s\"") {
                    cell_type = "s".to_string();
                } else if tag == "/row" || tag.starts_with("/row ") {
                    if !current_row.is_empty() {
                        rows.push(current_row.join("\t"));
                        current_row = Vec::new();
                    }
                } else if tag == "/c" || tag.starts_with("/c ") {
                }

                tag_buf.clear();
            }
            _ if in_tag => {
                tag_buf.push(ch);
            }
            _ if in_v => {
                value_buf.push(ch);
            }
            _ => {}
        }
    }

    if !current_row.is_empty() {
        rows.push(current_row.join("\t"));
    }

    rows.join("\n")
}

/// Auto-detect file type and extract text locally.
/// Returns (text, quality_score, method_used).
pub fn extract_text_from_file(path: &str) -> Result<(String, f64, String), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let ext = std::path::Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let (text, method) = match ext.as_str() {
        "txt" | "md" | "rtf" | "json" | "xml" | "html" | "htm" => {
            let text = decode_bytes_to_string(&bytes);
            (text, "text_read".to_string())
        }
        "csv" => {
            let text = decode_bytes_to_string(&bytes);
            (text, "csv_read".to_string())
        }
        "docx" => {
            let text = extract_text_from_docx(&bytes)?;
            (text, "docx_extract".to_string())
        }
        "xlsx" => {
            let text = extract_text_from_xlsx(&bytes)?;
            (text, "xlsx_extract".to_string())
        }
        "pptx" | "pptm" => {
            let text = extract_text_from_pptx(&bytes)?;
            (text, "pptx_extract".to_string())
        }
        "pdf" => {
            let text = extract_text_from_pdf(&bytes)?;
            (text, "pdf_extract".to_string())
        }
        "doc" | "xls" | "ppt" => {
            return Err(format!(
                "Legacy Office format '{}' requires vision AI for text extraction",
                ext
            ));
        }
        _ => {
            return Err(format!("Unsupported file type for local extraction: {}", ext));
        }
    };

    let quality = assess_text_quality(&text);
    Ok((text, quality, method))
}

/// Decode bytes to string using encoding detection.
fn decode_bytes_to_string(bytes: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_string();
    }

    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        if let Ok(text) = std::str::from_utf8(&bytes[3..]) {
            return text.to_string();
        }
    }

    encoding_rs::mem::decode_latin1(bytes).into_owned()
}

