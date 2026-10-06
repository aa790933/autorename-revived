<div align="center">
  <h1>AutoRename-Revived v4.0.0</h1>
   <p><b>AI-powered batch document renamer , native Rust + Tauri v2 backend with React + TypeScript frontend and multi-provider LLM support.</b></p>
  <p>
    <img src="https://img.shields.io/badge/rust-2021-orange?logo=rust" alt="Rust">
    <img src="https://img.shields.io/badge/tauri-v2-blue?logo=tauri" alt="Tauri">
    <img src="https://img.shields.io/badge/react-18-61dafb?logo=react" alt="React">
    <img src="https://img.shields.io/badge/typescript-5-3178c6?logo=typescript" alt="TypeScript">
    <img src="https://img.shields.io/badge/tailwindcss-3-38bdf8?logo=tailwindcss" alt="Tailwind CSS">
    <img src="https://img.shields.io/badge/platform-Windows-blue?logo=windows" alt="Windows">
    <img src="https://img.shields.io/github/license/aa790933/autorename-revived" alt="MIT">
  </p>
</div>

AutoRename-Revived extracts **company name**, **document date**, **document type**, **category**, and **subject** from documents (PDF, images, DOCX, XLSX, PPTX) using AI, then renames them to a consistent, customizable format , batch processing hundreds of files in seconds.

Built with **Tauri v2** (Rust backend + React/TypeScript frontend) for ultra-fast performance , no Python runtime required.

---

## Overview

| Capability | Details |
|---|---|
| **Backend** | Rust + Tauri v2 (native, zero-install) |
| **Frontend** | React 18 + TypeScript + Tailwind CSS + Zustand |
| **AI Providers** | Gemini, OpenAI, Anthropic, Ollama, xAI, Custom |
| **Local Extraction** | DOCX, XLSX, PPTX, PDF text extraction (no external tools) |
| **Vision Mode** | Scanned PDFs & images analyzed via Vision LLMs |
| **Internationalization** | English, French, Arabic (RTL support) |
| **Portable** | Standalone EXE with portable settings (`.portable` marker) |
| **Installer** | MSI installer with OS-global settings |

---

## Quick Start

### GUI (Recommended)

Download the [latest release](https://github.com/aa790933/autorename-revived/releases) and run `AutoRename-Revived.exe`:

1. **Configure AI** , Open Settings, select a provider, and enter your API key
2. **Test Connection** , Click "Test Connection" to verify your key works
3. **Drag & Drop** , Drop PDF files or folders onto the window
4. **Preview** , Click **Dry Run** to preview proposed names without writing
5. **Rename** , Click **Rename** to apply changes
6. **Undo** , Click **Undo** to reverse the last batch
7. **Cancel** , Click **Cancel** during a long run to stop processing

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
| `{separator}` | Configured separator | `_` |

### Default Template

```
{date}_{doctype}_{company}_{subject}
```

Example output: `20240115_Invoice_AcmeCorp_Q3_Report_01.pdf`

### Fallback Template

When AI extraction returns no metadata, the fallback template is used (default: `{date}_{doctype}_{company}_Unknown`). All undeterminable fields are replaced with `"Unknown"`.

### Settings

| Setting | Default | Description |
|---|---|---|
| `naming.date_format` | `%Y-%m-%d` | Chrono format for parsed dates |
| `naming.sequence_zerofill` | `2` | Padding width for `{sequence}` |
| `naming.max_length` | `128` | Truncation limit for generated filenames |
| `naming.separator` | `_` | Separator between fields |

---

## AI Providers

Supported providers and their default models:

| Provider | Text Model | Vision Model | API Key Required |
|---|---|---|---|
| Gemini (default) | `gemini-2.5-flash` | `gemini-2.5-flash` | Yes |
| OpenAI | `gpt-4o-mini` | `gpt-4o` | Yes |
| Anthropic | `claude-3-5-haiku-latest` | `claude-sonnet-4-20250514` | Yes |
| Ollama | `llama3.2` | `llama3.2` | No (local) |
| xAI | `grok-3-beta` | `grok-3-beta` | Yes |
| Custom | User-defined | User-defined | User-defined |

### Vision Mode (PDF)

| Setting | Values | Description |
|---|---|---|
| `document.vision` | `auto`, `true`, `false` | Whether to use Vision LLM for PDF/images |
| `document.vision_provider` | Any provider | Separate provider for vision (e.g. Gemini for vision, OpenAI for text) |
| `document.text_quality_threshold` | `0.0` - `1.0` | Minimum local text quality before falling back to vision in `auto` mode |

In `auto` mode, local text extraction runs first. If quality meets the threshold, text AI is used (cheaper). Otherwise, vision AI is used.

### System Prompt

The AI system prompt is fully customizable via Settings → AI System Prompt. Leave it empty to use the built-in default, which instructs the AI to extract all five metadata fields as structured JSON.

---

## Internationalization

AutoRename-Revived v4.0.0 supports multiple languages with full RTL (Right-to-Left) support for Arabic:

| Language | Code | Direction | Status |
|---|---|---|---|
| English | `en` | LTR | ✅ Native |
| French | `fr` | LTR | ✅ Native |
| Arabic | `ar` | RTL | ✅ Native |

The UI automatically adapts to the selected language, including:
- All text translations
- RTL layout for Arabic
- Date/number formatting
- Placeholder hints in settings

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
- **Portability**: Copy the folder to any machine , settings travel with you

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
git tag v4.0.1
git push origin v4.0.1
```

Produces `AutoRename-v4.0.1-Portable.zip` and `AutoRename-v4.0.1.msi`.

### Local Build

```bash
# Prerequisites: Rust toolchain, Node.js 20+, pnpm 9
cd gui
pnpm install
pnpm tauri build
```

---

## Project Structure

```
autorename-revived/
├── gui/                            # Frontend (React + TypeScript + Vite)
│   ├── src/
│   │   ├── main.tsx                # App entry point
│   │   ├── App.tsx                 # Main app component
│   │   ├── components/
│   │   │   ├── ui/                 # Base UI components (Button, Input, Card, etc.)
│   │   │   ├── layout/             # Layout components (Sidebar, Header, Toaster)
│   │   │   ├── file-list/          # File list components (FileRow, DropZone, FileList)
│   │   │   ├── settings/           # Settings components (ProviderSelector, SettingsForm)
│   │   │   └── views/              # Page views (HistoryView, AboutView)
│   │   ├── hooks/                  # Custom React hooks
│   │   ├── store/                  # Zustand state management
│   │   ├── services/               # API services (Tauri IPC wrappers)
│   │   ├── types/                  # TypeScript type definitions
│   │   ├── i18n/                   # Internationalization (i18next)
│   │   ├── utils/                  # Utility functions
│   │   └── styles/                 # Global styles
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── tailwind.config.js
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
│   │   ├── portable.rs             # Portable vs installer detection
│   │   └── error.rs                # Unified error types
│   ├── capabilities/               # Tauri capabilities
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
| **Rename Pipeline** | `rename_files`, `cancel_rename`, `undo_rename` |
| **Config** | `load_app_config`, `save_app_config`, `save_app_config_batch`, `get_config`, `get_config_path`, `validate_config` |
| **AI Extraction** | `extract_metadata_from_text`, `extract_metadata_from_vision`, `test_connection` |
| **File I/O** | `read_file_bytes`, `read_file_base64`, `preserve_file_extension`, `validate_extension`, `is_image_file`, `get_file_size_bytes`, `get_file_name_from_path`, `get_file_stem_from_path`, `get_file_ext`, `resolve_safe_path_cmd`, `ensure_directory_cmd`, `copy_file_cmd`, `file_exists_cmd`, `list_files`, `find_files_recursive` |
| **Rename Helpers** | `apply_rename_cmd`, `save_rename_to_history_cmd`, `undo_last_rename_cmd` |
| **Utility** | `get_version`, `get_supported_extensions_list`, `is_portable_app`, `get_settings_path`, `get_undo_log_path` |

---

## Architecture Highlights

### Modern Frontend (v4.0.0)
- **React 18** with TypeScript for type-safe UI development
- **Zustand** for lightweight, scalable state management
- **i18next** for internationalization with RTL support
- **Tailwind CSS** for utility-first styling
- **Lucide React** for beautiful, consistent icons
- **Component-based architecture** for maintainability

### Robust Backend
- **Unified error handling** with `thiserror`
- **Async pipeline** with bounded concurrency
- **Multi-language AI extraction** with aligned results
- **Vision fallback** with quality assessment
- **Portable-aware** path resolution

### Performance Optimizations
- **Fat LTO** for smaller, faster binaries
- **Single codegen unit** for optimal Rust compilation
- **Lazy-loaded chunks** for frontend (React, i18n, Tauri API)
- **Bounded worker pool** for AI requests

---

## License

MIT , see [LICENSE](LICENSE)