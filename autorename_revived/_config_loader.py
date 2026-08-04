from __future__ import annotations

import copy
import logging
import os
import re
from typing import Any, Dict, Optional

import yaml

from autorename_revived._resources import find_resource

log = logging.getLogger(__name__)

_CONFIG_FILENAMES = ["config.yaml", "config.yml"]
_ENV_VAR_RE = re.compile(r"\$\{(\w+)\}|\$(\w+)")

_GEMINI_DEFAULTS = {
    "api_key": "${GEMINI_API_KEY}",
    "model": "gemini-2.0-flash",
    "timeout": 30,
}

_VISION_DEFAULTS = {
    "provider": "gemini",
    "gemini": dict(_GEMINI_DEFAULTS),
}

_NAMED_DEFAULTS = {
    "provider": "openai",
    "model": "gpt-4o-mini",
    "temperature": 0.0,
    "timeout": 30,
}

_NAMING_DEFAULTS = {
    "template": "{date}_{company}_{doctype}",
    "fallback": "{date}_Unknown_{doctype}",
    "date_format": "%Y%m%d",
    "separator": "_",
    "max_length": 128,
    "sequence_zerofill": 2,
}

_PDF_DEFAULTS = {
    "vision": "auto",
    "vision_provider": "gemini",
    "ocr": False,
}

_UNDO_DEFAULTS = {
    "enabled": True,
    "log_path": "~/.autorename-revived/rename_history.json",
    "max_entries": 100,
}

_DEFAULTS = {
    "config_version": 2,
    "ai": dict(_NAMED_DEFAULTS),
    "vision": dict(_VISION_DEFAULTS),
    "naming": dict(_NAMING_DEFAULTS),
    "pdf": dict(_PDF_DEFAULTS),
    "undo": dict(_UNDO_DEFAULTS),
    "harmonized_companies": [],
    "debug": False,
    "max_workers": 4,
}


def _resolve_env_vars(value: str, env: Optional[Dict[str, str]] = None) -> str:
    if not isinstance(value, str):
        return value
    if env is None:
        env = os.environ

    def _replace(m: re.Match) -> str:
        var = m.group(1) or m.group(2)
        return env.get(var, m.group(0))

    return _ENV_VAR_RE.sub(_replace, value)


def _deep_resolve_env(obj: Any, env: Optional[Dict[str, str]] = None) -> Any:
    if isinstance(obj, dict):
        return {k: _deep_resolve_env(v, env) for k, v in obj.items()}
    if isinstance(obj, list):
        return [_deep_resolve_env(v, env) for v in obj]
    if isinstance(obj, str):
        return _resolve_env_vars(obj, env)
    return obj


def load_config(path: Optional[str] = None) -> Dict[str, Any]:
    if path:
        candidates = [path]
    else:
        candidates = [
            os.path.join(os.getcwd(), fname) for fname in _CONFIG_FILENAMES
        ] + [
            find_resource(_CONFIG_FILENAMES),
        ]

    cfg_path = None
    for c in candidates:
        if c and os.path.isfile(c):
            cfg_path = c
            break

    if not cfg_path:
        return copy.deepcopy(_DEFAULTS)

    with open(cfg_path, "r", encoding="utf-8") as f:
        try:
            raw = yaml.safe_load(f) or {}
        except yaml.YAMLError as e:
            log.warning(f"Failed to parse config {cfg_path}: {e}. Using defaults.")
            return copy.deepcopy(_DEFAULTS)

    config = copy.deepcopy(_DEFAULTS)
    _deep_merge(config, raw)
    config = _deep_resolve_env(config)

    config.setdefault("naming", copy.deepcopy(_NAMING_DEFAULTS))
    config.setdefault("vision", copy.deepcopy(_VISION_DEFAULTS))
    config.setdefault("undo", copy.deepcopy(_UNDO_DEFAULTS))
    config.setdefault("ai", copy.deepcopy(_NAMED_DEFAULTS))
    config.setdefault("pdf", copy.deepcopy(_PDF_DEFAULTS))

    if isinstance(config.get("harmonized_companies"), list):
        config["harmonized_companies"] = _resolve_harmonized_list(config["harmonized_companies"])

    return config


def _deep_merge(base: dict, override: dict) -> None:
    for key, value in override.items():
        if key in base and isinstance(base[key], dict) and isinstance(value, dict):
            _deep_merge(base[key], value)
        else:
            base[key] = value


def _resolve_harmonized_list(companies: list) -> list:
    resolved = []
    for entry in companies:
        if isinstance(entry, str):
            resolved.append({"name": entry, "variations": [entry]})
        elif isinstance(entry, dict):
            resolved.append(entry)
    return resolved


_CONFIG_PATH_CACHE: Optional[str] = None


def find_config_path(path: Optional[str] = None) -> Optional[str]:
    global _CONFIG_PATH_CACHE
    if path:
        _CONFIG_PATH_CACHE = path if os.path.isfile(path) else None
        return _CONFIG_PATH_CACHE
    if _CONFIG_PATH_CACHE and os.path.isfile(_CONFIG_PATH_CACHE):
        return _CONFIG_PATH_CACHE
    candidates = [
        os.path.join(os.getcwd(), fname) for fname in _CONFIG_FILENAMES
    ] + [
        find_resource(_CONFIG_FILENAMES),
    ]
    for c in candidates:
        if c and os.path.isfile(c):
            _CONFIG_PATH_CACHE = c
            return c
    return None


def save_config(config: dict, path: Optional[str] = None) -> str:
    cfg_path = path or find_config_path()
    if not cfg_path:
        cfg_path = os.path.join(os.getcwd(), "config.yaml")
    export = dict(config)
    export.pop("config_version", None)
    keys_top = ["ai", "vision", "naming", "pdf", "undo", "harmonized_companies", "debug", "max_workers"]
    order = {k: i for i, k in enumerate(keys_top)}
    sorted_keys = sorted(export.keys(), key=lambda k: (order.get(k, 99), k))
    ordered = {}
    for k in sorted_keys:
        ordered[k] = export[k]
    ordered["config_version"] = 2
    os.makedirs(os.path.dirname(os.path.abspath(cfg_path)), exist_ok=True)
    with open(cfg_path, "w", encoding="utf-8") as f:
        yaml.dump(ordered, f, default_flow_style=False, sort_keys=False, allow_unicode=True)
    _CONFIG_PATH_CACHE = cfg_path
    return cfg_path


def get_gemini_config(config: dict) -> dict:
    vision = config.get("vision", {})
    gemini = vision.get("gemini", {}) or {}
    return {
        "api_key": gemini.get("api_key") or config.get("ai", {}).get("gemini_api_key", ""),
        "model": gemini.get("model", "gemini-2.0-flash"),
        "timeout": gemini.get("timeout", 30),
    }
