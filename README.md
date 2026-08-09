<div align="center">
  <h1>AutoRename-Revived v3.0.4</h1>
   <p><b>AI-powered batch document renamer — native Rust + Tauri backend with multi-provider LLM support.</b></p>
  <p>
    <img src="https://img.shields.io/badge/rust-2021-orange?logo=rust" alt="Rust">
    <img src="https://img.shields.io/badge/tauri-v2-blue?logo=tauri" alt="Tauri">
    <img src="https://img.shields.io/badge/platform-Windows-blue?logo=windows" alt="Windows">
    <img src="https://img.shields.io/github/license/aa790933/autorename-revived" alt="MIT">
  </p>
</div>

AutoRename-Revived extracts **company name**, **document date**, **document type**, **category**, and **subject** from documents (PDF, images, DOCX, XLSX, PPTX) using AI, then renames them to a consistent, customizable format — batch processing hundreds of files in seconds.

Built with **Tauri v2** (Rust backend + TypeScript frontend) for ultra-fast performance — no Python runtime required.

---

## Overview

| Capability | Details |
|---|---|
| **Backend** | Rust + Tauri v2 (native, zero-install) |
| **AI Providers** | Gemini, OpenAI, Anthropic, Ollama, xAI, Custom |
| **Local Extraction** | DOCX, XLSX, PPTX, PDF text extraction (no external tools) |
| **Vision Mode** | Scanned PDFs & images analyzed via Vision LLMs |
| **Portable** | Standalone EXE with portable settings (`.portable` marker) |
| **Installer** | MSI installer with OS-global settings |

---

## Quick Start

### GUI (Recommended)

Download the [latest release](https://github.com/aa790933/autorename-revived/releases) and run `AutoRename-Revived.exe`:

1. **Configure AI** — Open Settings, select a provider, and enter your API key
2. **Test Connection** — Click "Test Connection" to verify your key works
3. **Drag & Drop** — Drop PDF files or folders onto the window
4. **Preview** — Click **Dry Run** to preview proposed names without writing
5. **Rename** — Click **Rename** to apply changes
6. **Undo** — Click **Undo** to reverse the last batch
7. **Cancel** — Click **Cancel** during a long run to stop processing

---

## Naming

The renamer uses a customizable template with `{field}` placeholders. Each field is replaced with extracted AI metadata.

### Placeholders

| Placeholder | Description | Example |
|---|---|---|
| `{date}` | Document date in `YYYYMMDD` format | `20240115` |
| `{company}` | Extracted company name | `AcmeCorp` |
| `{doctype}` | Document type | `Invoice` |
| `{category}` | Document category | `Finance` |
| `{subject}` | Document subject / title | `Q3_Report` |
| `{original}` | Original filename stem | `scan_001` |
| `{sequence}` | Zero-filled sequence number | `_01`, `_02` |

### Default Template

```
{date}_{company}_{doctype}
```

Example output: `20240115_AcmeCorp_Invoice_01.pdf`

### Fallback Template

When AI extraction returns no metadata, the fallback template is used (default: `{date}_Unknown_{doctype}`). All undeterminable fields are replaced with `"Unknown"`.

### Settings

| Setting | Default | Description |
|---|---|---|
| `naming.date_format` | `%Y%m%d` | Chrono format for parsed dates |
| `naming.sequence_zerofill` | `2` | Padding width for `{sequence}` |
| `naming.max_length` | `128` | Truncation limit for generated filenames |
| `naming.separator` | `_` | Separator between fields (used when `{separator}` placeholder is in template) |

---

## AI Providers

Supported providers and their default models:

| Provider | Text Model | Vision Model | API Key Required |
|---|---|---|---|
| Gemini (default) | `gemini-3.5-flash-lite` | `gemini-3.5-flash-lite` | Yes |
| OpenAI | `gpt-4o-mini` | `gpt-4o` | Yes |
| Anthropic | `claude-3-5-haiku-latest` | `claude-sonnet-4-20250514` | Yes |
| Ollama | `llama3.2` | `llama3.2` | No (local) |
| xAI | `grok-3-beta` | `grok-3-beta` | Yes |
| Custom | User-defined | User-defined | User-defined |

### Vision Mode (PDF)

| Setting | Values | Description |
|---|---|---|
| `pdf.vision` | `auto`, `true`, `false` | Whether to use Vision LLM for PDF/images |
| `pdf.vision_provider` | Any provider | Separate provider for vision (e.g. Gemini for vision, OpenAI for text) |
| `pdf.text_quality_threshold` | `0.0` - `1.0` | Minimum local text quality before falling back to vision in `auto` mode |

In `auto` mode, local text extraction runs first. If quality meets the threshold, text AI is used (cheaper). Otherwise, vision AI is used.

### System Prompt

The AI system prompt is fully customizable via Settings → AI System Prompt. Leave it empty to use the built-in default, which instructs the AI to extract all five metadata fields as structured JSON.

---

## Environment Variables

Config values support `${VAR_NAME}` syntax for secrets:

```env
GEMINI_API_KEY=your-gemini-key-here
OPENAI_API_KEY=sk-your-openai-key-here
```

---

## Portable vs. Installer

### Portable Edition

- **Settings location**: Stored alongside the EXE as `settings.json`
- **Marker**: A `.portable` file next to the EXE enables portable mode
- **Portability**: Copy the folder to any machine — settings travel with you

```
Folder/
├── AutoRename-Revived.exe
├── .portable
├── settings.json
└── renamed-files/
```

### Installer Edition (MSI)

- **Settings location**: `%APPDATA%\AutoRename-Revived\settings.json`
- **Shared**: Settings persist across reinstallations on the same machine

---

## Building

### CI/CD

Push a `v*` tag to trigger `.github/workflows/release.yml`:

```bash
git tag v3.0.5
git push origin v3.0.5
```

Produces `AutoRename-v3.0.5-Portable.zip` and `AutoRename-v3.0.5.msi`.

### Local Build

```
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
│   │   ├── renderer.ts             # View routing + status bar
│   │   ├── lib/
│   │   │   ├── config-store.ts     # Config CRUD (in-memory + persistence)
│   │   │   ├── sidecar.ts          # IPC wrappers for Rust commands
│   │   │   ├── state.ts            # Pub/sub app state
│   │   │   ├── dnd.ts              # Drag-and-drop
│   │   │   ├── filepicker.ts       # File/folder dialogs
│   │   │   ├── rename-cache.ts     # Dry-run cache apply via Tauri FS
│   │   │   ├── theme.ts            # Dark/light toggle
│   │   │   ├── titlebar.ts         # Custom window controls
│   │   │   ├── toast.ts            # Toast notifications
│   │   │   └── utils.ts            # Extension helpers, escapeHtml
│   │   ├── views/
│   │   │   ├── files.ts            # Main file list + rename pipeline
│   │   │   ├── settings.ts         # Settings form + provider switcher
│   │   │   └── about.ts            # About page
│   │   └── css/                    # Catppuccin theme
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   └── index.html
│
├── src-tauri/                      # Backend (Rust + Tauri v2)
│   ├── src/
│   │   ├── main.rs                 # Tauri entry point
│   │   ├── lib.rs                  # IPC command bindings
│   │   ├── ai.rs                   # AI provider routing + model defaults
│   │   ├── config.rs               # AppConfig model, persistence, env resolution
│   │   ├── document.rs             # Filename generation, undo history, path safety
│   │   ├── extractors.rs           # Local text extraction (DOCX, XLSX, PPTX, PDF)
│   │   ├── file_utils.rs           # File system utilities
│   │   └── portable.rs             # Portable vs installer detection
│   ├── dependencies/               # Tauri capabilities
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── build.rs
│
├── .github/workflows/release.yml   # CI/CD
├── .gitignore
├── LICENSE
└── README.md
```

---

## IPC Commands

The frontend communicates with the Rust backend via Tauri IPC commands:

| Category | Commands |
|---|---|
| **Rename Pipeline** | `rename_pdfs`, `cancel_rename`, `undo_rename` |
| **Config** | `load_app_config`, `save_app_config`, `save_app_config_batch`, `get_config`, `get_config_path`, `save_config_cmd`, `validate_config` |
| **AI Extraction** | `extract_metadata_from_text`, `extract_metadata_from_vision`, `test_connection` |
| **File I/O** | `read_file_bytes`, `read_file_base64`, `preserve_file_extension`, `validate_extension`, `is_image_file`, `get_file_size_bytes`, `get_file_name_from_path`, `get_file_stem_from_path`, `get_file_ext`, `resolve_safe_path_cmd`, `ensure_directory_cmd`, `copy_file_cmd`, `file_exists_cmd`, `list_files`, `find_files_recursive` |
| **Rename Helpers** | `apply_rename_cmd`, `save_rename_to_history_cmd`, `undo_last_rename_cmd` |
| **Utility** | `get_version`, `get_supported_extensions_list`, `is_portable_app`, `get_settings_path` |

---

## License

MIT — see [LICENSE](LICENSE)
