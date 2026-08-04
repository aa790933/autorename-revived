from __future__ import annotations

import os
import sys


def bundle_dir() -> str:
    if getattr(sys, "frozen", False) and hasattr(sys, "_MEIPASS"):
        return str(sys._MEIPASS)
    return os.path.dirname(os.path.abspath(__file__))


def resource_path(relative: str) -> str:
    return os.path.join(bundle_dir(), relative)


def find_resource(names: list[str]) -> str | None:
    base = bundle_dir()
    for name in names:
        candidate = os.path.join(base, name)
        if os.path.isfile(candidate):
            return candidate
    return None
