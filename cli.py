from __future__ import annotations

import argparse
import json
import logging
import os
import signal
import sys
import uuid
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import Any, List, Optional, Tuple

from autorename_revived import VERSION, ExitCode, SUPPORTED_EXTENSIONS
from autorename_revived._config_loader import load_config, save_config
from autorename_revived._ai_processing import test_api_connection
from autorename_revived._document_extractor import get_extractor
from autorename_revived._document_processing import (
    load_undo_history,
    rename_file,
    rename_invoice,
    save_rename_to_history,
)

log = logging.getLogger("autorename-revived")


def _find_files(paths: List[str], config: dict, recursive: bool = True) -> List[str]:
    supported = SUPPORTED_EXTENSIONS
    files: List[str] = []
    for p in paths:
        p = os.path.abspath(p)
        if os.path.isfile(p):
            ext = os.path.splitext(p)[1].lower()
            if ext in supported:
                files.append(p)
            continue
        if os.path.isdir(p):
            if recursive:
                for root, _, filenames in os.walk(p):
                    for f in filenames:
                        ext = os.path.splitext(f)[1].lower()
                        if ext in supported:
                            files.append(os.path.join(root, f))
            else:
                for f in os.listdir(p):
                    ext = os.path.splitext(f)[1].lower()
                    if ext in supported:
                        files.append(os.path.join(p, f))
    return files


def _process_single(file_path: str, config: dict, dry_run: bool = False, batch_id: str = "") -> Tuple[str, Optional[str], Optional[str], dict]:
    try:
        extractor = get_extractor(file_path, config)
        metadata = extractor.extract_metadata(file_path)
        if not metadata:
            return file_path, None, "Skipped (no extractable metadata)", {}
        new_path, new_name = rename_invoice(file_path, metadata, config)
        ext = os.path.splitext(file_path)[1]
        if not new_name.endswith(ext):
            new_name += ext
            new_path += ext

        if os.path.normpath(file_path) == os.path.normpath(new_path):
            return file_path, None, "Already matches target name", metadata

        success = rename_file(file_path, new_path, dry_run=dry_run)
        if success and not dry_run:
            save_rename_to_history(file_path, new_path, config, batch_id=batch_id)
        status = new_name if success else None
        error = None if success else "Rename failed"
        return file_path, status, error, metadata
    except Exception as e:
        log.debug(f"Failed to process {file_path}: {e}")
        return file_path, None, str(e), {}


def _build_file_result(src: str, status: Optional[str], error: Optional[str], metadata: dict, dry_run: bool) -> dict:
    if error and not status:
        file_status = "failed"
    elif status is None:
        file_status = "skipped"
    else:
        file_status = "renamed"
    base = {
        "file": src,
        "status": file_status,
        "new_name": status,
        "new_path": None,
        "error": error,
        "warnings": [],
        "company": metadata.get("company_name", "") or None,
        "date": metadata.get("document_date", "") or None,
        "doc_type": metadata.get("document_type", "") or None,
        "provider": metadata.get("provider", "") or None,
        "model": metadata.get("model", "") or None,
    }
    if status and not dry_run:
        base["new_path"] = os.path.join(os.path.dirname(src), status)
    return base


def cmd_rename(args: argparse.Namespace, config: dict) -> int:
    paths = args.files
    dry_run = args.dry_run
    max_workers = config.get("max_workers", 4)
    recursive = getattr(args, "recursive", True)

    output_json = getattr(args, "output", "") == "json"

    files = _find_files(paths, config, recursive)
    if not files:
        if output_json:
            print(json.dumps({
                "success": True,
                "total": 0,
                "renamed": 0,
                "skipped": 0,
                "failed": 0,
                "files": [],
                "dry_run": dry_run,
                "batch_id": None,
            }))
        else:
            log.warning("No supported files found.")
        return ExitCode.NO_FILES

    batch_id = str(uuid.uuid4())[:8] if not dry_run else ""

    results: List[Tuple[str, Optional[str], Optional[str], dict]] = []
    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        futures = {executor.submit(_process_single, f, config, dry_run, batch_id): f for f in files}
        for future in as_completed(futures):
            results.append(future.result())

    renamed = 0
    errors = 0
    skipped = 0
    file_results = []

    for src, status, error, metadata in results:
        if error and not status:
            if not output_json:
                log.warning(f"  {os.path.basename(src)}: {error}")
            errors += 1
            file_results.append(_build_file_result(src, status, error, metadata, dry_run))
            continue
        if status is None:
            if not output_json:
                log.info(f"  {os.path.basename(src)}: Already correct")
            skipped += 1
            file_results.append(_build_file_result(src, status, error, metadata, dry_run))
            continue
        if not output_json:
            log.info(f"  {os.path.basename(src)} -> {status}")
        renamed += 1
        file_results.append(_build_file_result(src, status, error, metadata, dry_run))

    if not output_json:
        log.info(f"Results: {renamed} renamed, {skipped} skipped, {errors} errors")

    if output_json:
        print(json.dumps({
            "success": True,
            "total": len(files),
            "renamed": renamed,
            "skipped": skipped,
            "failed": errors,
            "files": file_results,
            "dry_run": dry_run,
            "batch_id": batch_id if batch_id else None,
        }))

    if dry_run:
        return ExitCode.SUCCESS
    if errors and renamed:
        return ExitCode.PARTIAL
    if errors:
        return ExitCode.ERROR
    return ExitCode.SUCCESS


def cmd_undo(args: argparse.Namespace, config: dict) -> int:
    batch_id = getattr(args, "batch", None)
    output_json = getattr(args, "output", "") == "json"

    from autorename_revived._document_processing import load_undo_history, undo_last_rename
    history = load_undo_history(config)
    if not history:
        if output_json:
            print(json.dumps({
                "success": False,
                "error_type": "no_history",
                "message": "No undo history found.",
                "suggestion": "Nothing to undo.",
            }))
        else:
            log.info("No undo history found.")
        return ExitCode.ERROR

    if batch_id:
        entry = next((e for e in history if e.get("batch_id") == batch_id), None)
        if not entry:
            if output_json:
                print(json.dumps({
                    "success": False,
                    "error_type": "batch_not_found",
                    "message": f"Batch {batch_id} not found in history.",
                    "suggestion": "",
                }))
            else:
                log.warning(f"Batch {batch_id} not found in history.")
            return ExitCode.ERROR

    success = undo_last_rename(config, batch_id=batch_id or "")
    if success:
        if output_json:
            print(json.dumps({
                "success": True,
                "restored": 1,
                "failed": 0,
                "files": [{"status": "restored"}],
                "batch_id": batch_id,
            }))
        else:
            log.info("Undo completed successfully.")
        return ExitCode.SUCCESS
    else:
        if output_json:
            print(json.dumps({
                "success": False,
                "error_type": "undo_failed",
                "message": "Undo failed. File may no longer exist.",
                "suggestion": "",
            }))
        else:
            log.warning("Undo failed.")
        return ExitCode.ERROR


def cmd_config_show(args: argparse.Namespace, config: dict) -> int:
    output_json = getattr(args, "output", "") == "json"
    safe = {k: v for k, v in config.items() if k not in ("api_key",)}

    def _mask(obj: Any) -> Any:
        if isinstance(obj, dict):
            return {k: ("***" if "key" in k.lower() and v else v) for k, v in obj.items()}
        return obj

    safe = {k: _mask(v) if isinstance(v, dict) else v for k, v in safe.items()}

    if output_json:
        print(json.dumps(safe, default=str))
    else:
        print(json.dumps(safe, indent=2, default=str))
    return ExitCode.SUCCESS


def _resolve_provider_api_key(config: dict, provider: str) -> str:
    if provider == "ollama":
        return "noop"
    ai = config.get("ai", {})
    if provider == "gemini":
        vision = config.get("vision", {})
        gemini = vision.get("gemini", {})
        return gemini.get("api_key") or ai.get("gemini_api_key") or ai.get("api_key", "")
    if provider == "xai":
        return ai.get("xai_api_key") or ai.get("api_key", "")
    return ai.get("api_key", "")


def cmd_config_validate(args: argparse.Namespace, config: dict) -> int:
    output_json = getattr(args, "output", "") == "json"
    errors: List[str] = []
    warnings: List[str] = []

    if config.get("config_version") not in (1, 2):
        errors.append("Missing or invalid config_version")

    ai = config.get("ai", {})
    provider = ai.get("provider", "openai")
    api_key = _resolve_provider_api_key(config, provider)
    if not api_key and provider != "ollama":
        errors.append("No API key for AI provider (set via config or environment variable)")

    if not config.get("undo", {}).get("log_path"):
        warnings.append("Undo log path not configured")

    if output_json:
        issues = [{"field": "config", "level": "error", "message": e} for e in errors]
        issues += [{"field": "config", "level": "warning", "message": w} for w in warnings]
        print(json.dumps({"valid": len(errors) == 0, "issues": issues}))
    else:
        for e in errors:
            log.warning(f"Config error: {e}")
        for w in warnings:
            log.warning(f"Config warning: {w}")
        if errors:
            return ExitCode.CONFIG
        log.info("Configuration is valid.")
    return ExitCode.SUCCESS if not errors else ExitCode.CONFIG


def cmd_config_test_connection(args: argparse.Namespace, config: dict) -> int:
    output_json = getattr(args, "output", "") == "json"
    ai = config.get("ai", {})
    provider = args.provider or ai.get("provider", "openai")
    api_key = args.api_key or ai.get("api_key", "")
    model = args.model or ai.get("model", "")
    success, msg, latency_ms = test_api_connection(provider, api_key, config, model)
    out = {"success": success, "message": msg, "latency_ms": latency_ms, "provider": provider}
    if output_json:
        print(json.dumps(out))
    else:
        print(json.dumps(out, indent=2))
    return ExitCode.SUCCESS if success else ExitCode.AUTH


def cmd_config_save(args: argparse.Namespace, config: dict) -> int:
    output_json = getattr(args, "output", "") == "json"
    if not args.key or not args.value:
        if output_json:
            print(json.dumps({"success": False, "error_type": "usage", "message": "Both --key and --value are required"}))
        else:
            log.error("Both --key and --value are required for config save")
        return ExitCode.USAGE

    key_path = args.key.split(".")
    node = config
    for part in key_path[:-1]:
        node = node.setdefault(part, {})
    node[key_path[-1]] = _coerce_value(args.value)

    cfg_path = getattr(args, "config", "") or None
    try:
        path = save_config(config, cfg_path)
        if output_json:
            print(json.dumps({"success": True, "saved_path": path}))
        else:
            log.info(f"Saved {args.key}={args.value} to {path}")
        return ExitCode.SUCCESS
    except Exception as e:
        if output_json:
            print(json.dumps({"success": False, "error_type": "save_failed", "message": str(e)}))
        else:
            log.error(f"Failed to save config: {e}")
        return ExitCode.CONFIG


def cmd_config_save_batch(args: argparse.Namespace, config: dict) -> int:
    output_json = getattr(args, "output", "") == "json"
    pairs_raw = getattr(args, "pairs", "")
    if not pairs_raw:
        if output_json:
            print(json.dumps({"success": False, "error_type": "usage", "message": "--pairs is required (JSON array of {key, value})"}))
        else:
            log.error("--pairs is required for config save-batch")
        return ExitCode.USAGE

    try:
        pairs = json.loads(pairs_raw)
    except json.JSONDecodeError as e:
        if output_json:
            print(json.dumps({"success": False, "error_type": "invalid_json", "message": f"Invalid JSON: {e}"}))
        else:
            log.error(f"Invalid JSON in --pairs: {e}")
        return ExitCode.USAGE

    if not isinstance(pairs, list):
        if output_json:
            print(json.dumps({"success": False, "error_type": "invalid_json", "message": "--pairs must be a JSON array"}))
        else:
            log.error("--pairs must be a JSON array")
        return ExitCode.USAGE

    saved = 0
    failed = 0
    errors = []
    for entry in pairs:
        key = entry.get("key", "")
        value = entry.get("value", "")
        if not key:
            failed += 1
            errors.append("Missing key")
            continue
        key_path = key.split(".")
        node = config
        for part in key_path[:-1]:
            node = node.setdefault(part, {})
        node[key_path[-1]] = _coerce_value(str(value))
        saved += 1

    cfg_path = getattr(args, "config", "") or None
    try:
        path = save_config(config, cfg_path)
        if output_json:
            print(json.dumps({
                "success": failed == 0,
                "saved": saved,
                "failed": failed,
                "errors": errors,
                "saved_path": path,
            }))
        else:
            log.info(f"Batch saved {saved} settings to {path}")
        return ExitCode.SUCCESS if failed == 0 else ExitCode.PARTIAL
    except Exception as e:
        if output_json:
            print(json.dumps({"success": False, "error_type": "save_failed", "message": str(e)}))
        else:
            log.error(f"Failed to save config: {e}")
        return ExitCode.CONFIG


def _coerce_value(val: str) -> Any:
    if val.lower() in ("true", "yes"):
        return True
    if val.lower() in ("false", "no"):
        return False
    try:
        return int(val)
    except ValueError:
        pass
    try:
        return float(val)
    except ValueError:
        pass
    return val


def _signal_handler(signum, frame):
    log.warning("Interrupted by user")
    sys.exit(ExitCode.INTERRUPTED)


def main() -> int:
    signal.signal(signal.SIGINT, _signal_handler)

    parser = argparse.ArgumentParser(prog="autorename-revived", description="Auto-rename documents using AI")
    parser.add_argument("--version", action="version", version=f"%(prog)s {VERSION}")
    sub = parser.add_subparsers(dest="command")

    rename_p = sub.add_parser("rename", help="Rename files based on AI-extracted metadata")
    rename_p.add_argument("files", nargs="+", help="Files or directories to process")
    rename_p.add_argument("--dry-run", action="store_true", help="Show what would be renamed without renaming")
    rename_p.add_argument("--provider", default="", help="AI provider override")
    rename_p.add_argument("--model", default="", help="Model override")
    rename_p.add_argument("--vision", action="store_true", help="Force vision mode")
    rename_p.add_argument("--text-only", action="store_true", help="Force text-only mode (disable vision)")
    rename_p.add_argument("--recursive", action="store_true", default=True, help="Recursively scan directories (default: true)")
    rename_p.add_argument("--no-recursive", action="store_false", dest="recursive", help="Do not scan directories recursively")
    rename_p.add_argument("--config", default="", help="Path to config file")
    rename_p.add_argument("--output", choices=["json"], help="Output format")

    undo_p = sub.add_parser("undo", help="Undo the last rename operation")
    undo_p.add_argument("--count", type=int, default=1, help="Number of operations to undo (not yet supported)")
    undo_p.add_argument("--batch", default="", help="Batch ID to undo")
    undo_p.add_argument("--config", default="")
    undo_p.add_argument("--output", choices=["json"], help="Output format")

    config_p = sub.add_parser("config", help="Configuration commands")
    config_sub = config_p.add_subparsers(dest="config_command")
    config_show = config_sub.add_parser("show", help="Show current configuration")
    config_show.add_argument("--config", default="")
    config_show.add_argument("--output", choices=["json"], help="Output format")
    config_validate = config_sub.add_parser("validate", help="Validate configuration")
    config_validate.add_argument("--config", default="")
    config_validate.add_argument("--output", choices=["json"], help="Output format")
    config_test = config_sub.add_parser("test-connection", help="Test AI provider API connection")
    config_test.add_argument("--config", default="")
    config_test.add_argument("--provider", default="", help="Provider to test (default: from config)")
    config_test.add_argument("--api-key", default="", help="API key to test with (default: from config)")
    config_test.add_argument("--model", default="", help="Model to test (default: from config)")
    config_test.add_argument("--output", choices=["json"], help="Output format")
    config_save = config_sub.add_parser("save", help="Save a config key-value pair to config.yaml")
    config_save.add_argument("--config", default="")
    config_save.add_argument("--key", default="", help="Dot-separated config key path (e.g. ai.api_key)")
    config_save.add_argument("--value", default="", help="Value to set")
    config_save.add_argument("--output", choices=["json"], help="Output format")

    config_save_batch = config_sub.add_parser("save-batch", help="Save multiple config key-value pairs at once")
    config_save_batch.add_argument("--config", default="")
    config_save_batch.add_argument("--pairs", default="", help='JSON array of {key, value} objects')
    config_save_batch.add_argument("--output", choices=["json"], help="Output format")

    args = parser.parse_args()
    if not args.command:
        parser.print_help()
        return ExitCode.USAGE

    cfg_path = getattr(args, "config", "") or None
    config = load_config(cfg_path)

    if args.command == "rename":
        if args.provider:
            config.setdefault("ai", {})["provider"] = args.provider
        if args.model:
            config.setdefault("ai", {})["model"] = args.model
        if args.vision:
            config.setdefault("pdf", {})["vision"] = True
        if args.text_only:
            config.setdefault("pdf", {})["vision"] = False

    level = logging.DEBUG if config.get("debug") else logging.INFO
    if getattr(args, "output", "") == "json":
        level = logging.WARNING
    logging.basicConfig(level=level, format="%(levelname)s: %(message)s", stream=sys.stderr)

    if args.command == "rename":
        return cmd_rename(args, config)
    if args.command == "undo":
        return cmd_undo(args, config)
    if args.command == "config":
        if args.config_command == "show":
            return cmd_config_show(args, config)
        if args.config_command == "validate":
            return cmd_config_validate(args, config)
        if args.config_command == "test-connection":
            return cmd_config_test_connection(args, config)
        if args.config_command == "save":
            return cmd_config_save(args, config)
        if args.config_command == "save-batch":
            return cmd_config_save_batch(args, config)
        log.warning("Unknown config command")
        return ExitCode.USAGE

    return ExitCode.USAGE


if __name__ == "__main__":
    sys.exit(main())