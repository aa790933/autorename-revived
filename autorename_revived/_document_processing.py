from __future__ import annotations

import json
import logging
import os
from datetime import datetime
from typing import Any, Dict, List, Optional, Tuple

import dateparser
from rapidfuzz import fuzz, process as fuzz_process

from autorename_revived._naming_engine import NamingEngine
from autorename_revived._path_safety import sanitize_filename, resolve_safe_path

log = logging.getLogger(__name__)


def harmonize_company_name(name: str, harmonized_companies: List[Any]) -> str:
    if not name or not harmonized_companies:
        return name

    names = {e["name"]: e for e in harmonized_companies if isinstance(e, dict) and "name" in e}
    if not names:
        return name

    lookup = {}
    for entry in names.values():
        lookup[entry["name"].lower()] = entry["name"]
        for var in entry.get("variations", []):
            lookup[var.lower()] = entry["name"]

    if name.lower() in lookup:
        return lookup[name.lower()]

    candidates = list(names.keys())
    result = fuzz_process.extractOne(name, candidates, scorer=fuzz.token_sort_ratio, score_cutoff=80)
    if result:
        return names[result[0]]["name"]

    return name


def parse_document_date(date_str: str) -> Optional[str]:
    if not date_str:
        return None

    cleaned = date_str.replace("-", "").replace("/", "").replace(" ", "")
    if len(cleaned) == 8 and cleaned.isdigit():
        return cleaned

    try:
        parsed = dateparser.parse(date_str, settings={
            "DATE_ORDER": "DMY",
            "PREFER_DAY_OF_MONTH": "first",
            "STRICT_PARSING": False,
        })
        if parsed:
            return parsed.strftime("%Y%m%d")
    except Exception:
        pass

    return None


def rename_invoice(file_path: str, metadata: Dict[str, Any], config: dict) -> Tuple[str, str]:
    company = metadata.get("company_name", "").strip()
    doctype = metadata.get("document_type", "").strip()
    date_str = metadata.get("document_date", "").strip()
    confidence = metadata.get("confidence", 0.0)

    harmonized = config.get("harmonized_companies", [])
    company = harmonize_company_name(company, harmonized)
    parsed_date = parse_document_date(date_str) or ""

    engine = NamingEngine(config)
    new_name = engine.generate(company=company, doctype=doctype, date_str=parsed_date)

    directory = os.path.dirname(os.path.abspath(file_path))
    new_path = resolve_safe_path(directory, new_name)

    return new_path, new_name


def rename_file(src: str, dst: str, dry_run: bool = False) -> bool:
    if dry_run:
        return True
    try:
        os.rename(src, dst)
        return True
    except OSError as e:
        log.error(f"Rename failed: {src} -> {dst}: {e}")
        return False


def _get_undo_log_path(config: dict) -> str:
    log_path = config.get("undo", {}).get("log_path", "~/.autorename-revived/rename_history.json")
    return os.path.expanduser(log_path)


def _ensure_v2_format(data: Any) -> Dict[str, Any]:
    if isinstance(data, dict) and data.get("version") == 2:
        return data
    if isinstance(data, list):
        if not data:
            return {"version": 2, "batches": []}
        return {
            "version": 2,
            "batches": [{
                "batch_id": "migrated-v1",
                "timestamp": data[0].get("timestamp", "") if isinstance(data[0], dict) else "",
                "source": "cli",
                "undone": False,
                "files": data,
            }],
        }
    return {"version": 2, "batches": []}


def _read_undo_history(config: dict) -> Dict[str, Any]:
    path = _get_undo_log_path(config)
    if not os.path.isfile(path):
        return {"version": 2, "batches": []}
    try:
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
        return _ensure_v2_format(data)
    except (json.JSONDecodeError, OSError):
        return {"version": 2, "batches": []}


def load_undo_history(config: dict) -> List[Dict[str, str]]:
    log_data = _read_undo_history(config)
    all_files = []
    for batch in log_data.get("batches", []):
        if not batch.get("undone", False):
            for entry in batch.get("files", []):
                entry["batch_id"] = batch.get("batch_id", "")
            all_files.extend(batch.get("files", []))
    return all_files


def save_rename_to_history(old_path: str, new_path: str, config: dict, batch_id: str = "") -> None:
    if not config.get("undo", {}).get("enabled", True):
        return
    log_data = _read_undo_history(config)
    timestamp = datetime.now().isoformat()
    target_batch = None
    if batch_id:
        for batch in log_data.get("batches", []):
            if batch.get("batch_id") == batch_id and not batch.get("undone", False):
                target_batch = batch
                break
    if target_batch is None:
        target_batch = {
            "batch_id": batch_id or f"cli-{datetime.now().strftime('%Y%m%dT%H%M%S')}",
            "timestamp": timestamp,
            "source": "cli",
            "undone": False,
            "files": [],
        }
        log_data.setdefault("batches", []).append(target_batch)
    target_batch["files"].append({
        "old_path": os.path.normpath(old_path),
        "new_path": os.path.normpath(new_path),
        "timestamp": timestamp,
    })
    max_batches = config.get("undo", {}).get("max_entries", 100)
    batches = log_data.get("batches", [])
    if len(batches) > max_batches:
        log_data["batches"] = batches[-max_batches:]
    path = _get_undo_log_path(config)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(log_data, f, indent=2)


def undo_last_rename(config: dict, batch_id: str = "") -> bool:
    log_data = _read_undo_history(config)
    batches = log_data.get("batches", [])
    if not batches:
        log.info("No undo history found.")
        return False

    target_batch = None
    target_file_idx = -1
    if batch_id:
        for batch in reversed(batches):
            if batch.get("batch_id") == batch_id and not batch.get("undone", False):
                target_batch = batch
                target_file_idx = len(batch.get("files", [])) - 1
                break
    else:
        for batch in reversed(batches):
            if not batch.get("undone", False) and batch.get("files"):
                target_batch = batch
                target_file_idx = len(batch["files"]) - 1
                break

    if target_batch is None or target_file_idx < 0:
        log.info("No undo history found.")
        return False

    entry = target_batch["files"][target_file_idx]
    old_path = entry.get("old_path", "")
    new_path = entry.get("new_path", "")
    if not old_path or not new_path:
        log.warning("Invalid undo entry")
        return False

    if not os.path.isfile(new_path):
        log.warning(f"Renamed file no longer exists: {new_path}")
        return False

    try:
        os.rename(new_path, old_path)
        target_batch["files"].pop(target_file_idx)
        if not target_batch["files"]:
            target_batch["undone"] = True
        path = _get_undo_log_path(config)
        with open(path, "w", encoding="utf-8") as f:
            json.dump(log_data, f, indent=2)
        log.info(f"Undone: {new_path} -> {old_path}")
        return True
    except OSError as e:
        log.error(f"Undo rename failed: {e}")
        return False
