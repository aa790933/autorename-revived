from __future__ import annotations

import re
import unicodedata
from dataclasses import dataclass, field
from enum import IntEnum
from typing import Optional


class ExitCode(IntEnum):
    SUCCESS = 0
    ERROR = 1
    USAGE = 2
    CONFIG = 3
    NO_FILES = 4
    PARTIAL = 5
    PROVIDER = 10
    AUTH = 11
    INTERRUPTED = 130


SUPPORTED_EXTENSIONS: set = {
    ".pdf", ".docx", ".xlsx", ".pptx", ".csv", ".txt", ".md", ".rtf",
    ".png", ".jpg", ".jpeg", ".tiff", ".tif", ".bmp", ".gif", ".webp",
}

GIBBERISH_PATTERNS = [
    re.compile(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}', re.I),
    re.compile(r'\b[0-9a-f]{8,}\b', re.I),
    re.compile(r'\b\d{6,}\b'),
    re.compile(r'\b\d{4,}[A-Za-z]{2,}\b'),
    re.compile(r'\b[A-Za-z]{2,}\d{4,}\b'),
    re.compile(r'\b[b-df-hj-np-tv-z0-9]{12,}\b', re.I),
]


@dataclass
class ExtractionResult:
    text: str
    method: str
    metadata: dict = field(default_factory=dict)
    error: Optional[str] = None


def normalize_unicode(s: str, form: str = "NFKC") -> str:
    return unicodedata.normalize(form, s)
