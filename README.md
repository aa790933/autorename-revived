<div align="center">
  <h1>AutoRename-Revived v3.0.4</h1>
  <p><b>AI-powered batch document renamer — OCR, multi-provider LLM, modern GUI, and zero-latency threading.</b></p>
  <p>
    <img src="https://img.shields.io/badge/python-3.11%2B-blue?logo=python" alt="Python 3.11+">
    <img src="https://img.shields.io/badge/platform-Windows-blue?logo=windows" alt="Windows">
    <img src="https://img.shields.io/github/license/aa790933/autorename-revived" alt="MIT">
  </p>
</div>

AutoRename-Revived extracts **company name**, **document date**, and **document type** from documents (PDF, images, DOCX, XLSX) using AI, then renames them to a consistent `YYYYMMDD_COMPANY_DOCTYPE` format — batch processing hundreds of files in seconds.

---

## Features

- **5 AI Providers** — OpenAI, Anthropic (Claude), Google Gemini, xAI (Grok), and Ollama for fully offline use
- **Cloud-Native Vision** — Images and scanned PDFs analyzed via Vision LLMs (OpenAI, Gemini, Anthropic, Custom); no local OCR dependency
- **Multithreaded Processing** — Parallel batch via `ThreadPoolExecutor` with zero UI freezing; drag-and-drop runs on daemon threads
- **AI Sanitization** — Regex-based cleaning strips markdown fences, code blocks, and LLM gibberish before filenames reach the filesystem
- **API Test Connection** — One-click provider verification in both GUI and CLI (`config test-connection`)
- **Company Harmonization** — Fuzzy matching (Jaro-Winkler via rapidfuzz) maps OCR typos to canonical names
- **Modern GUI** — Tauri v2 desktop app (TypeScript + Rust) with Catppuccin theme, drag-and-drop, and one-click rename
- **Portable & Installer** — Standalone `--onefile` EXE or Inno Setup installer; context menu integration via `setup.ps1`
- **Privacy-First** — Run fully offline with Ollama; no data leaves your machine

## Quick Start

### GUI (Recommended)

Download the [latest release](https://github.com/aa790933/autorename-revived/releases) and run `AutoRename-v3.0.4-Portable.exe` — no installation required.

- Drag & drop PDF files or folders onto the window
- Preview proposed names with the **Dry Run** button
- **Rename** with one click; **Undo** reverses the last batch
- **Test Connection** in Settings verifies your AI provider is reachable
- Theme: Catppuccin Macchiato (dark) / Latte (light)

### CLI

```bash
# Set up
python -m venv venv
venv\Scripts\activate
pip install -r requirements.txt

# Rename files
python cli.py rename "C:\path\to\file.pdf"

# Preview only
python cli.py rename --dry-run "C:\path\to\folder"

# Undo last rename
python cli.py undo

# Test API connection
python cli.py config test-connection

# Validate config
python cli.py config save ai.model "gpt-4o-mini"
python cli.py config save vision.gemini.api_key "your-key-here"
python cli.py config validate
python cli.py config show
```

## Configuration

See [`config.yaml.example`](config.yaml.example) for the complete reference.

### Extraction Modes

| Setting | Values | Description |
|---------|--------|-------------|
| `pdf.vision` | `false` / `true` / `"auto"` | Send page images to Vision LLM (~$0.0001/page) |
| `pdf.text_quality_threshold` | `0.0`–`1.0` | Triggers vision in `"auto"` mode |
| `pdf.max_pages` | integer | Max pages processed per PDF |

In `"auto"` mode, vision activates only when pdfplumber's text quality drops below the threshold.

### Environment Variables

Config values support `${VAR_NAME}` syntax. A `.env` file next to `config.yaml` is loaded automatically via `python-dotenv`.

```env
OPENAI_API_KEY=sk-your-key-here
ANTHROPIC_API_KEY=sk-ant-your-key-here
```

## Project Structure

```
autorename-revived/
├── cli.py                          # CLI entry point (argparse: rename, undo, config save/validate/test-connection)
├── autorename_revived/
│   ├── __init__.py                 # Version + public API exports
│   ├── _ai_processing.py           # Multi-provider AI extraction (test_api_connection with model param)
│   ├── _config_loader.py           # YAML config with ${VAR} substitution + save_config
│   ├── _document_extractor.py      # PdfExtractor / ImageExtractor / DocxExtractor / XlsxExtractor / PptxExtractor / CsvExtractor / TxtExtractor
│   ├── _document_processing.py     # Harmonization, rename, undo history
│   ├── _naming_engine.py           # Template-based filename generation
│   ├── _path_safety.py             # Sanitization, traversal guards, Windows reserved names
│   ├── _pdf_utils.py               # PDF text + image extraction (pdfplumber, pypdfium2 page rendering)
│   ├── _resources.py               # PyInstaller resource resolution
│   └── _utils.py                   # Exit codes, validation constants
├── gui/                            # Tauri v2 desktop app
│   ├── src/                        # TypeScript frontend (settings.ts, files.ts, renderer.ts)
│   │   ├── lib/                    # sidecar.ts (Tauri bridge), dnd.ts, filepicker.ts, renderer.ts
│   │   ├── views/                  # settings.ts (editable form), about.ts, files.ts
│   │   └── css/                    # Catppuccin theme, component styles
│   ├── src-tauri/                  # Rust backend + CLI sidecar integration
│   │   ├── src/lib.rs              # Tauri entry (--version argv handler, get_version invoke)
│   │   └── binaries/               # Bundled CLI sidecar (autorename-revived-cli-x86_64-pc-windows-msvc.exe)
│   └── package.json                # Version 3.0.4, Node dependencies (pnpm)
├── tests/
│   └── test_suite.py               # 141 tests (pytest, --run-live for integration)
├── build.py                        # Triple-target build orchestrator (CLI sidecar, onedir, Tauri)
├── setup.ps1                       # Context menu installer
└── .github/workflows/release.yml   # CI/CD: test → build → verify → release (3 artifacts)
```

## Building

Releases are **CI-built only** — push a `v*` tag to trigger `.github/workflows/release.yml`:

```bash
python build.py    # Builds all three targets: CLI sidecar, onedir, Tauri GUI
```

> **Note**: Local builds require Rust toolchain (for Tauri), pnpm/Node.js (for GUI assets), and all Python dependencies installed in `venv/`. CI handles full builds automatically.

## Testing

```bash
pytest tests/ -v --cov
```

Live integration tests (require API keys in `.env`):

```bash
pytest tests/ --run-live -v
pytest tests/ --run-live --provider ollama -v
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error |
| 2 | Usage error |
| 3 | Configuration error |
| 4 | No files found |
| 5 | Partial failure |
| 10 | AI provider error |
| 11 | Authentication error |

## License

MIT — see [LICENSE](LICENSE)
