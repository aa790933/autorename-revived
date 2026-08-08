<div align="center">
  <h1>AutoRename-Revived v3.0.5</h1>
  <p><b>AI-powered batch document renamer — native Rust + Tauri backend with Gemini 3.5 Flash Lite integration.</b></p>
  <p>
    <img src="https://img.shields.io/badge/rust-2021-orange?logo=rust" alt="Rust">
    <img src="https://img.shields.io/badge/tauri-v2-blue?logo=tauri" alt="Tauri">
    <img src="https://img.shields.io/badge/gemini-3.5--flash--lite-8E75F9?logo=google" alt="Gemini">
    <img src="https://img.shields.io/badge/platform-Windows-blue?logo=windows" alt="Windows">
    <img src="https://img.shields.io/github/license/aa790933/autorename-revived" alt="MIT">
  </p>
</div>

AutoRename-Revived extracts **company name**, **document date**, **document type**, **category**, and **subject** from documents (PDF, images, DOCX, XLSX, PPTX) using AI, then renames them to a consistent, customizable format — batch processing hundreds of files in seconds.

Built with **Tauri v2** (Rust backend + TypeScript frontend) for ultra-fast performance — no Python runtime required.

---

## Overview

| Feature | Details |
|---------|---------|
| **Backend** | Rust + Tauri v2 (native, zero-install) |
| **AI Engine** | Google Gemini `gemini-3.5-flash-lite` (vision + text) |
| **Providers** | Gemini (default), OpenAI, Anthropic, xAI, Ollama, Custom |
| **Local OCR-free** | Scanned PDFs & images analyzed via Vision LLMs |
| **Portable** | Standalone EXE with portable settings (`.portable` marker) |
| **Installer** | MSI installer with OS-global settings |

---

## Quick Start

### GUI (Recommended)

Download the [latest release](https://github.com/aa790933/autorename-revived/releases) and run `AutoRename-Revived.exe` — no installation required.

1. **Configure AI** — Open Settings, enter your Gemini API key
2. **Test Connection** — Click "Test Connection" to verify your key works
3. **Drag & Drop** — Drop PDF files or folders onto the window
4. **Preview** — Click **Dry Run** to preview proposed names
5. **Rename** — Click **Rename** to apply; **Undo** reverses the last batch

---

## Custom Format Settings

The renamer supports fully customizable naming templates using `{field}` placeholders. Fields are replaced with extracted AI metadata.

### Available Placeholders

| Placeholder | Description | Example |
|-------------|-------------|---------|
| `{date}` | Document date in `YYYYMMDD` format | `20240115` |
| `{company}` | Extracted company name | `AcmeCorp` |
| `{doctype}` | Document type (Invoice, Contract, etc.) | `Invoice` |
| `{category}` | Document category | `Finance` |
| `{subject}` | Document subject / title | `Q3_Report` |
| `{original}` | Original filename stem (no extension) | `scan_001` |
| `{sequence}` | Zero-filled sequence number | `_01`, `_02` |

### Default Template

```
{date}_{company}_{doctype}
```

Example output: `20240115_AcmeCorp_Invoice_01.pdf`

### Custom Template Examples

**Comma-separated with category:**
```
{date},{company},{doctype}{category},{subject},{original}{sequence}
```

**Subject-first format:**
```
{subject}_{date}_{company}
```

**Simple date-prefixed:**
```
{date}_{subject}{sequence}
```

### Fallback Template

If the AI fails to extract any metadata, the **fallback template** is used instead. By default:
```
{date}_Unknown_{doctype}
```

All fields that cannot be determined are replaced with `"Unknown"` (instead of `00000000`), ensuring human-readable filenames even when AI extraction fails.

### Field-Specific Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `naming.date_format` | `%Y%m%d` | Chrono format for parsed dates |
| `naming.sequence_zerofill` | `2` | Padding width for `{sequence}` |
| `naming.max_length` | `128` | Maximum filename length |
| `naming.separator` | `_` | Separator between fields |

---

## AI Providers

| Provider | Text Model | Vision Model | API Key Field |
|----------|-----------|-------------|---------------|
| **Google Gemini** (default) | `gemini-3.5-flash-lite` | `gemini-3.5-flash-lite` | `ai.gemini_api_key` |
| OpenAI | `gpt-4o-mini` | `gpt-4o` | `ai.api_key` |
| Anthropic | `claude-3-5-haiku-latest` | `claude-sonnet-4-20250514` | `ai.api_key` |
| Ollama | `llama3.2` | `llama3.2` | None (local) |
| xAI | `grok-3-beta` | `grok-3-beta` | `ai.api_key` |
| Custom | User-defined | User-defined | User-defined |

### Extraction Modes

| Setting | Values | Description |
|---------|--------|-------------|
| `pdf.vision` | `false` / `true` / `"auto"` | Whether to send images to Vision LLM |
| `pdf.text_quality_threshold` | `0.0`–`1.0` | Triggers vision in `"auto"` mode when local text quality is below threshold |

In `"auto"` mode, local text extraction runs first. If quality is above the threshold, the text AI is used (cheaper). If below, vision AI is used (more accurate for scanned documents).

### Structured AI Extraction (responseSchema)

The Gemini integration uses **Structured Outputs** via `responseSchema` in the `generationConfig`. This forces the API to return a strictly-typed JSON object with all required fields (`date`, `company`, `doctype`, `category`, `subject`). Key safeguards:

1. **API-level schema enforcement** — The Gemini API validates the response against the schema before returning, guaranteeing well-formed JSON with all required fields.
2. **Aggressive extraction prompt** — The system prompt instructs the AI to never return null/empty, and to deduce missing information from visual context.
3. **Multi-layered Rust fallback parsing** — If the strict JSON parse fails, a regex-based fallback extracts key-value pairs from the raw text.
4. **Safe field mapping** — Empty, null, or whitespace-only AI values are replaced with `"Unknown"` (for text fields) or `"Unknown"` (for dates), ensuring files are never named `00000000_` or `Unknown_Unknown`.

### Environment Variables

Config values support `${VAR_NAME}` syntax for API keys:

```env
GEMINI_API_KEY=your-gemini-key-here
OPENAI_API_KEY=sk-your-openai-key-here
```

---

## Portable vs. Installer

AutoRename-Revived is distributed in two editions:

### Portable Edition

- **File**: `AutoRename-v{version}-Portable.zip` → `AutoRename-Revived.exe`
- **Settings location**: Stored **alongside the EXE** in the same directory as `settings.json`
- **Portability**: Copy the entire folder to any machine — your settings travel with you
- **Marker**: A `.portable` file next to the EXE enables portable mode

```
USB Drive / Folder:
├── AutoRename-Revived.exe
├── .portable          ← marker file (enables portable mode)
├── settings.json      ← your settings live here
└── renamed-files/
```

### Installer Edition (MSI)

- **File**: `AutoRename-v{version}.msi`
- **Settings location**: Stored in the OS standard AppData directory:
  - `%APPDATA%\AutoRename-Revived\settings.json`
- **Shared**: Settings are shared across reinstallations on the same machine
- **No `.portable` marker**: Uses standard OS app data

---

## Building

### CI/CD

Push a `v*` tag to trigger `.github/workflows/release.yml`:

```bash
git tag v3.0.5
git push origin v3.0.5
```

This produces:
- `AutoRename-v3.0.5-Portable.zip` — Standalone EXE with `.portable` marker
- `*.msi` — Windows installer

### Local Build

```bash
# Prerequisites: Rust toolchain, Node.js 24+, pnpm 10
cd gui
pnpm install
pnpm tauri build
```

---

## Project Structure

```
autorename-revived/
├── gui/                            # Frontend (TypeScript + Vite)
│   ├── src/
│   │   ├── main.ts                 # App entry point
│   │   ├── renderer.ts             # View router + status bar
│   │   ├── lib/
│   │   │   ├── config-store.ts     # In-memory config CRUD
│   │   │   ├── sidecar.ts          # IPC wrappers (invoke Rust commands)
│   │   │   ├── state.ts            # Pub/sub app state
│   │   │   ├── dnd.ts              # Drag-and-drop
│   │   │   ├── filepicker.ts       # File/folder picker dialogs
│   │   │   ├── rename-cache.ts     # Dry-run cache apply via Tauri FS
│   │   │   ├── theme.ts            # Dark/light toggle
│   │   │   ├── titlebar.ts         # Custom window controls
│   │   │   ├── toast.ts            # Toast notifications
│   │   │   └── utils.ts            # Supported extensions, escapeHtml
│   │   ├── views/
│   │   │   ├── files.ts            # Main file list + rename pipeline
│   │   │   ├── settings.ts         # Settings form + provider switcher
│   │   │   └── about.ts            # About page
│   │   └── css/                    # Catppuccin theme, component styles
│   ├── public/                     # Static assets
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   └── index.html
│
├── src-tauri/                      # Backend (Rust + Tauri v2)
│   ├── src/
│   │   ├── main.rs                 # Tauri entry point
│   │   ├── lib.rs                  # IPC command bindings
│   │   ├── ai.rs                   # AI provider routing (Gemini, OpenAI, etc.)
│   │   ├── config.rs               # AppConfig model, persistence, env var resolution
│   │   ├── document.rs             # Filename generation, undo history, path safety
│   │   ├── extractors.rs           # Local text extraction (DOCX, XLSX, PPTX, PDF)
│   │   ├── file_utils.rs           # File system utilities
│   │   └── portable.rs             # Portable vs installer detection
│   ├── dependencies/               # Tauri permissions
│   ├── icons/                      # App icons
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── build.rs
│
├── .github/workflows/release.yml   # CI/CD: build → verify → release
├── .gitignore
├── LICENSE
├── README.md
└── metadata.json                   # Code signing config
```

---

## IPC Commands

The frontend communicates with the Rust backend via Tauri IPC commands:

| Command | Purpose |
|---------|---------|
| `load_app_config` / `save_app_config` | Config persistence |
| `save_app_config_batch` | Batch settings update from UI |
| `test_connection` | Test AI provider connectivity |
| `rename_pdfs` | Main rename pipeline (read → extract → AI → rename) |
| `undo_rename` | Undo last batch |
| `get_config` / `get_config_path` / `save_config_cmd` | Config CRUD |
| `extract_metadata_from_text` / `extract_metadata_from_vision` | Direct AI extraction |
| `is_portable_app` / `get_settings_path` | Portability info |
| `read_file_bytes` / `read_file_base64` | File I/O utilities |

---

## License

MIT — see [LICENSE](LICENSE)
