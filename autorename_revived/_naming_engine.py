from __future__ import annotations

import re
from datetime import datetime
from pathlib import Path
from typing import List, Optional

from autorename_revived._path_safety import sanitize_filename
from autorename_revived._utils import GIBBERISH_PATTERNS

_DEFAULT_TEMPLATE = "{date}_{company}_{doctype}"
_SEQUENCE_ZEROFILL = 2
_TEMPLATE_PATTERN = re.compile(r"\{(\w+)\}")

_LEADING_TRAILING_GUNK = re.compile(r'^[\W_]+|[\W_]+$')
_MULTI_SEP = re.compile(r'[_\- ]{2,}')


def _strip_gibberish(text: str) -> str:
    for pat in GIBBERISH_PATTERNS:
        text = pat.sub('', text)
    text = _LEADING_TRAILING_GUNK.sub('', text)
    text = _MULTI_SEP.sub('_', text)
    return text.strip('_ ')


class NamingEngine:
    def __init__(self, config: dict):
        tc = config.get("naming", {})
        self.template = tc.get("template", _DEFAULT_TEMPLATE)
        self.fallback_template = tc.get("fallback", "{date}_Unknown_{doctype}")
        self.date_format = tc.get("date_format", "%Y%m%d")
        self.separator = tc.get("separator", "_")
        self.max_length = tc.get("max_length", 128)
        self.sequence_zerofill = tc.get("sequence_zerofill", _SEQUENCE_ZEROFILL)

    def generate(
        self,
        company: str = "",
        doctype: str = "",
        date_str: str = "",
        date_obj: Optional[datetime] = None,
        subject: str = "",
        original_filename: str = "",
        sequence: int = 0,
    ) -> str:
        suffix = Path(original_filename).suffix if original_filename else ".pdf"
        has_content = any([company, doctype, date_str, date_obj, original_filename])
        template = self.template if has_content else self.fallback_template
        fields = {
            "date": self._format_date(date_obj, date_str),
            "company": self._clean_field(company) or "Unknown",
            "doctype": self._clean_field(doctype) or "Doc",
            "category": self._clean_field(company) or "Unknown",
            "subject": self._clean_field(subject or company or doctype) or "Unknown",
            "original": self._clean_field(Path(original_filename).stem) if original_filename else "file",
            "sequence": str(sequence).zfill(self.sequence_zerofill),
        }
        result = _TEMPLATE_PATTERN.sub(lambda m: fields.get(m.group(1), m.group(0)), template)
        if not result or result == template or result == self.template:
            result = _TEMPLATE_PATTERN.sub(lambda m: fields.get(m.group(1), m.group(0)), self.fallback_template)
        avail = self.max_length - len(suffix)
        if avail < 4:
            avail = 4
        result = result[:avail] + suffix
        return result

    def generate_batch(
        self, docs: List[dict], base_company: str = "", base_doctype: str = ""
    ) -> List[str]:
        results = []
        seq = 1
        for doc in docs:
            c = doc.get("company", "") or base_company
            d = doc.get("doctype", "") or base_doctype
            ds = doc.get("date_str", "")
            dobj = doc.get("date_obj")
            name = self.generate(
                company=c,
                doctype=d,
                date_str=ds,
                date_obj=dobj,
                subject=doc.get("subject", ""),
                original_filename=doc.get("original_filename", ""),
                sequence=seq,
            )
            seq += 1
            results.append(name)
        return results

    def _format_date(self, date_obj: Optional[datetime], date_str: str = "") -> str:
        if date_obj:
            return date_obj.strftime(self.date_format)
        if date_str and len(date_str) == 8 and date_str.isdigit():
            return date_str
        cleaned = date_str.replace("-", "").replace("/", "").replace(".", "").strip()
        if cleaned.isdigit() and len(cleaned) >= 8:
            return cleaned[:8]
        return "00000000"

    def _clean_field(self, value: str) -> str:
        cleaned = _strip_gibberish(value)
        s = sanitize_filename(cleaned, max_length=48, replacement=self.separator)
        return s.strip(self.separator)
