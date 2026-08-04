from __future__ import annotations

import io
import logging
from typing import List

from autorename_revived._utils import ExtractionResult

log = logging.getLogger(__name__)


def extract_text_from_pdf(path: str) -> ExtractionResult:
    try:
        import pdfplumber
    except ImportError:
        return ExtractionResult(text="", method="pdfplumber", error="pdfplumber not installed")
    try:
        text_parts: List[str] = []
        with pdfplumber.open(path) as pdf:
            for page in pdf.pages:
                t = page.extract_text() or ""
                if t.strip():
                    text_parts.append(t)
        result = "\n".join(text_parts).strip()
        if not result:
            return ExtractionResult(text="", method="pdfplumber", error="No text extracted")
        return ExtractionResult(text=result, method="pdfplumber")
    except Exception as e:
        return ExtractionResult(text="", method="pdfplumber", error=str(e))


def assess_text_quality(text: str) -> float:
    if not text or not text.strip():
        return 0.0
    total = len(text)
    if total < 10:
        return 0.0
    words = text.split()
    if len(words) < 3:
        return 0.0
    alpha = sum(1 for c in text if c.isalpha())
    ratio = alpha / total
    word_bonus = min(len(words) / 20.0, 0.3)
    return min(ratio + word_bonus, 1.0)


def render_pages_to_images(path: str, max_pages: int = 5, dpi: int = 200) -> List[bytes]:
    try:
        import pypdfium2 as pdfium
    except ImportError:
        log.warning("pypdfium2 not available for page rendering")
        return []
    try:
        pdf = pdfium.PdfDocument(path)
        n_pages = min(len(pdf), max_pages)
        images: List[bytes] = []
        for i in range(n_pages):
            page = pdf[i]
            bitmap = page.render(scale=dpi / 72)
            pil_image = bitmap.to_pil()
            buf = io.BytesIO()
            pil_image.save(buf, format="PNG")
            images.append(buf.getvalue())
        pdf.close()
        return images
    except Exception as e:
        log.warning(f"Page rendering failed: {e}")
        return []
