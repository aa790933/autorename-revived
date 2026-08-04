from __future__ import annotations

import re
import unicodedata
from pathlib import Path

from autorename_revived._utils import GIBBERISH_PATTERNS

_INVALID_FS_CHARS_RE = re.compile(r'[\x00-\x1f\\/:*?"<>|]')
_PATH_TRAVERSAL_RE = re.compile(r'(?:^|[/\\])\.\.(?:[/\\]|$)')

_UNICODE_CONTROL_RE = re.compile(
    r'[\u200b-\u200f\u2028-\u202f\u2060-\u2064\ufeff\u00ad]'
)

_RESERVED_WINDOWS_NAMES = {
    "con", "prn", "aux", "nul",
    "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9",
    "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
}

_WINDOWS_MAX_PATH = 260

_SANITIZE_REPLACEMENT = "_"


def sanitize_filename(name: str, max_length: int = 128, replacement: str = _SANITIZE_REPLACEMENT) -> str:
    if not name:
        return "_"

    cleaned = unicodedata.normalize("NFKC", name)
    cleaned = _UNICODE_CONTROL_RE.sub("", cleaned)

    for pat in GIBBERISH_PATTERNS:
        cleaned = pat.sub(replacement, cleaned)

    cleaned = _INVALID_FS_CHARS_RE.sub(replacement, cleaned).strip(" .")

    parts = [p for p in cleaned.split(replacement) if p]
    cleaned = replacement.join(parts) if parts else "_"

    if cleaned.startswith("."):
        cleaned = replacement + cleaned[1:]

    cleaned = cleaned[:max_length]

    stem_only = cleaned.rpartition(".")[0]
    if not stem_only:
        stem_only = cleaned
    if stem_only.lower() in _RESERVED_WINDOWS_NAMES:
        cleaned = f"{replacement}{cleaned}"

    return cleaned


def is_safe_path_component(component: str) -> bool:
    if not component or component in (".", ".."):
        return False
    return not bool(_INVALID_FS_CHARS_RE.search(component))


def resolve_safe_path(directory: str, filename: str) -> str:
    dir_path = Path(directory).resolve()
    safe_name = Path(sanitize_filename(filename))
    resolved = (dir_path / safe_name).resolve()
    resolved_str = str(resolved)
    if not resolved_str.startswith(str(dir_path)):
        raise ValueError(f"Path traversal blocked: {filename}")
    if len(resolved_str) > _WINDOWS_MAX_PATH:
        raise ValueError(
            f"Full path exceeds {_WINDOWS_MAX_PATH} characters: "
            f"{len(resolved_str)} chars"
        )
    return resolved_str


def validate_subprocess_arg(arg: str) -> str:
    if _PATH_TRAVERSAL_RE.search(arg):
        raise ValueError(f"Path traversal in subprocess arg: {arg}")
    return arg
