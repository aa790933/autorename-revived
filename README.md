<div align="center">
  <h1>AutoRename-Revived v3.0.4</h1>
  <p><b>AI-powered batch document renamer — native Rust backend, multi-provider LLM, and modern Tauri GUI.</b></p>
  <p>
    <img src="https://img.shields.io/badge/rust-2021-orange?logo=rust" alt="Rust">
    <img src="https://img.shields.io/badge/platform-Windows-blue?logo=windows" alt="Windows">
    <img src="https://img.shields.io/github/license/aa790933/autorename-revived" alt="MIT">
  </p>
</div>

AutoRename-Revived extracts **company name**, **document date**, and **document type** from documents (PDF, images, DOCX, XLSX, PPTX) using AI, then renames them to a consistent `YYYYMMDD_COMPANY_DOCTYPE` format — batch processing hundreds of files in seconds.

---

## Features

- **Pure Rust Backend** — No Python runtime required; native AI provider routing via `reqwest` + `tokio`
- **5 AI Providers** — Google Gemini (default), OpenAI, Anthropic, xAI (Grok), and Ollama for offline use
- **Cloud-Native Vision** — Images and scanned PDFs analyzed via Vision LLMs; no local OCR dependency
- **Local Text Extraction** — DOCX, XLSX, PPTX, and PDF text extraction via `lopdf` + ZIP parsing
- **Async IPC** — Non-blocking background threads for file reading, Base64 encoding, and API calls
- **AI Sanitization** — Regex-based cleaning strips markdown fences, code blocks, and LLM gibberish
- **API Test Connection** — One-click provider verification in Settings (tests only the active provider)
- **Company Harmonization** — Fuzzy matching maps OCR typos to canonical names
- **Modern GUI** — Tauri v2 desktop app (TypeScript + Rust) with Catppuccin theme, drag-and-drop
- **Embedded State** — Settings stored in-memory via `tauri-plugin-store`; zero external config files
- **Portable & Installer** — Standalone EXE or MSI installer

## Quick Start

### GUI (Recommended)

Download the [latest release](https://github.com/aa790933/autorename-revived/releases) and run `AutoRename-Revived.exe` — no installation required.

- Drag & drop PDF files or folders onto the window
- Preview proposed names with the **Dry Run** button
- **Rename** with one click; **Undo** reverses the last batch
- **Test Connection** in Settings verifies your AI provider is reachable
- Theme: Catppuccin Macchiato (dark) / Latte (light)

### Development

```bash
# Prerequisites: Rust toolchain, Node.js 22+, pnpm
cd gui
pnpm install
pnpm tauri dev
```

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
│   ├── scripts/                    # Build scripts (colors, icons)
│   ├── package.json                # v3.0.4, pnpm dependencies
│   ├── tsconfig.json
│   ├── vite.config.ts
│   └── index.html
│
├── src-tauri/                      # Backend (Rust + Tauri v2)
│   ├── src/
│   │   ├── main.rs                 # Tauri entry point
│   │   ├── lib.rs                  # IPC command bindings (32 commands)
│   │   ├── ai.rs                   # AI provider routing (Gemini, OpenAI, Anthropic, Ollama, xAI, Custom)
│   │   ├── config.rs               # AppConfig model, tauri-plugin-store persistence, env var resolution
│   │   ├── document.rs             # Filename generation, undo history, path safety
│   │   ├── extractors.rs           # Local text extraction (DOCX, XLSX, PPTX, PDF), text quality scoring
│   │   └── file_utils.rs           # File system utilities
│   ├── capabilities/               # Tauri permissions
│   ├── icons/                      # App icons
│   ├── Cargo.toml                  # Rust dependencies
│   ├── tauri.conf.json             # Tauri build config
│   └── build.rs
│
├── .github/workflows/release.yml   # CI/CD: build → verify → release
├── .gitignore
├── LICENSE
├── README.md
└── metadata.json                   # Code signing config
```

## AI Providers

| Provider | Text Model | Vision Model | API Key |
|----------|-----------|-------------|---------|
| **Google Gemini** (default) | `gemini-3.1-flash-lite` | `gemini-3.1-flash-lite` | `ai.gemini_api_key` |
| OpenAI | `gpt-4o-mini` | `gpt-4o` | `ai.api_key` |
| Anthropic | `claude-3-5-haiku-latest` | `claude-sonnet-4-20250514` | `ai.api_key` |
| Ollama | `llama3.2` | `llama3.2` | None |
| xAI | `grok-3-beta` | `grok-3-beta` | `ai.api_key` |
| Custom | User-defined | User-defined | User-defined |

### Extraction Modes

| Setting | Values | Description |
|---------|--------|-------------|
| `pdf.vision` | `false` / `true` / `"auto"` | Send file images to Vision LLM |
| `pdf.text_quality_threshold` | `0.0`–`1.0` | Triggers vision in `"auto"` mode |

In `"auto"` mode, local text extraction runs first. If quality is above the threshold, the text AI is used (cheaper). If below, vision AI is used (more accurate for scanned documents).

### Environment Variables

Config values support `${VAR_NAME}` syntax for API keys:

```env
GEMINI_API_KEY=your-key-here
OPENAI_API_KEY=sk-your-key-here
```

## Building

### CI/CD

Push a `v*` tag to trigger `.github/workflows/release.yml`:

```bash
git tag v3.0.5
git push origin v3.0.5
```

This produces:
- `AutoRename-v3.0.5-Portable.zip` — Standalone EXE
- `*.msi` — Windows installer

### Local Build

```bash
# Prerequisites: Rust toolchain, Node.js 22+, pnpm
cd gui
pnpm install
pnpm tauri build
```

## IPC Commands

The frontend communicates with the Rust backend via 32 Tauri IPC commands:

| Command | Purpose |
|---------|---------|
| `load_app_config` / `save_app_config` | Config persistence via tauri-plugin-store |
| `save_app_config_batch` | Batch settings update from UI |
| `test_connection` | Test AI provider connectivity |
| `rename_pdfs` | Main rename pipeline (read → extract → AI → rename) |
| `undo_rename` | Undo last batch |
| `validate_config` | Config validation on startup |
| `get_config` / `get_config_path` / `save_config_cmd` | Config CRUD |
| `extract_metadata_from_text` / `extract_metadata_from_vision` | Direct AI extraction |
| `read_file_bytes` / `read_file_base64` | File I/O utilities |

## License

MIT — see [LICENSE](LICENSE)
