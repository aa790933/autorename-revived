from __future__ import annotations

import base64
import json
import logging
import re
import time
from pathlib import Path
from typing import Any, Dict, List, Optional, Union

from pydantic import BaseModel, Field

log = logging.getLogger(__name__)


class DocumentMetadata(BaseModel):
    company_name: str = Field(default="", description="Extracted company/organization name")
    document_date: str = Field(default="", description="Document date as YYYY-MM-DD or empty")
    document_type: str = Field(default="", description="Document type (Invoice, Receipt, etc.)")
    confidence: float = Field(default=0.0, ge=0.0, le=1.0)
    invoice_number: str = Field(default="")
    total_amount: str = Field(default="")


_TEXT_SYSTEM_PROMPT = (
    "You extract structured metadata from document text. "
    "Return ONLY valid JSON with keys: company_name, document_date (YYYY-MM-DD or empty), "
    "document_type, invoice_number, total_amount, confidence (0.0-1.0). "
    "Always extract what you can see. For document_type, use one of: "
    "Invoice, Receipt, Contract, Report, Letter, Memo, Statement, Certificate, Form, Other. "
    "For company_name, use the sender or issuer name. "
    "Only set confidence < 0.5 if the document text is mostly unreadable."
)

_VISION_SYSTEM_PROMPT = (
    "You are an expert Document Classifier and File Renaming Engine. "
    "Analyze the provided document/image context, visual layout, header, dates, "
    "sender/recipient, and content summary.\n\n"
    "INSTRUCTIONS:\n"
    "1. Identify the document type (e.g., Invoice, Receipt, Passport, Contract, Photo, Diagram).\n"
    "2. Extract key metadata: Date (YYYY-MM-DD), Organization/Entity name, Document Title or Main Subject.\n"
    "3. Output ONLY a valid JSON response with keys: company_name, document_date (YYYY-MM-DD or empty), "
    "document_type, invoice_number, total_amount, confidence (0.0-1.0).\n"
    "4. If content is unreadable or ambiguous, fall back to a structured default like "
    "Document_YYYY-MM-DD rather than Unknown.\n"
    "5. Never alter the original file extension.\n\n"
    "Return ONLY valid JSON."
)


_AI_MARKDOWN_RE = re.compile(r'```(?:json|yaml)?\s*|\s*```')
_AI_COMMENT_LINE_RE = re.compile(r'^\s*(#|//).*$', re.MULTILINE)
_AI_COMMENT_BLOCK_RE = re.compile(r'<!--.*?-->', re.DOTALL)
_AI_CONTROL_RE = re.compile(r'[\x00-\x08\x0b\x0c\x0e-\x1f\x7f\u200b-\u200f\u2028-\u202f\u2060-\u2064\ufeff]')
_AI_GIBBERISH_RE = re.compile(
    r'\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b'
    r'|\b[0-9a-f]{16,}\b'
    r'|\b\d{12,}\b'
    r'|\*\*|\*|__|`'
)


def _strip_ai_gibberish(text: str) -> str:
    text = _AI_CONTROL_RE.sub('', text)
    text = _AI_COMMENT_LINE_RE.sub('', text)
    text = _AI_COMMENT_BLOCK_RE.sub('', text)
    text = _AI_MARKDOWN_RE.sub('', text)
    text = _AI_GIBBERISH_RE.sub('', text)
    return text.strip()


def _extract_json_braces(text: str) -> Optional[str]:
    start = text.find('{')
    if start == -1:
        return None
    depth = 0
    for i in range(start, len(text)):
        ch = text[i]
        if ch == '{':
            depth += 1
        elif ch == '}':
            depth -= 1
            if depth == 0:
                return text[start:i + 1]
    return None


def _parse_json_response(text: str) -> Dict[str, Any]:
    text = _strip_ai_gibberish(text)
    text = text.strip()
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        pass
    block = _extract_json_braces(text)
    if block:
        try:
            return json.loads(block)
        except json.JSONDecodeError:
            pass
    raise ValueError("Failed to parse AI response as JSON")


def _file_to_base64(filepath: str) -> str:
    with open(filepath, "rb") as f:
        return base64.b64encode(f.read()).decode("ascii")


def _guess_mime(filepath: str) -> str:
    ext = Path(filepath).suffix.lower()
    return {
        ".jpg": "image/jpeg", ".jpeg": "image/jpeg",
        ".png": "image/png", ".gif": "image/gif",
        ".webp": "image/webp", ".bmp": "image/bmp",
        ".tiff": "image/tiff", ".tif": "image/tiff",
        ".pdf": "application/pdf",
    }.get(ext, "application/octet-stream")


def _openai_extract(text: str, config: dict) -> Dict[str, Any]:
    try:
        import openai
    except ImportError:
        raise ImportError("openai package required for OpenAI provider")
    model = config.get("model", "gpt-4o-mini")
    temperature = config.get("temperature", 0.0)
    timeout = config.get("timeout", 30)
    client = openai.OpenAI(api_key=config.get("api_key"), timeout=timeout)
    resp = client.chat.completions.create(
        model=model,
        temperature=temperature,
        messages=[
            {"role": "system", "content": _TEXT_SYSTEM_PROMPT},
            {"role": "user", "content": f"Document text:\n\n{text}\n\nExtract metadata JSON."},
        ],
        response_format={"type": "json_object"},
    )
    return _parse_json_response(resp.choices[0].message.content)


def _openai_vision_extract(filepaths: List[str], config: dict) -> Dict[str, Any]:
    try:
        import openai
    except ImportError:
        raise ImportError("openai package required for OpenAI vision")
    model = config.get("model", "gpt-4o")
    temperature = config.get("temperature", 0.0)
    timeout = config.get("timeout", 60)
    client = openai.OpenAI(api_key=config.get("api_key"), timeout=timeout)
    content: list = [{"type": "text", "text": _VISION_SYSTEM_PROMPT}]
    for fp in filepaths:
        b64 = _file_to_base64(fp)
        mime = _guess_mime(fp)
        content.append({
            "type": "image_url",
            "image_url": {"url": f"data:{mime};base64,{b64}", "detail": "auto"},
        })
    resp = client.chat.completions.create(
        model=model,
        temperature=temperature,
        messages=[{"role": "user", "content": content}],
        response_format={"type": "json_object"},
        max_tokens=1024,
    )
    return _parse_json_response(resp.choices[0].message.content)


def _anthropic_extract(text: str, config: dict) -> Dict[str, Any]:
    try:
        import anthropic
    except ImportError:
        raise ImportError("anthropic package required for Anthropic provider")
    model = config.get("anthropic_model", "claude-3-5-haiku-latest")
    base_url = config.get("anthropic_base_url", "https://api.anthropic.com")
    timeout = config.get("timeout", 30)
    client = anthropic.Anthropic(api_key=config.get("api_key"), base_url=base_url, timeout=timeout)
    resp = client.messages.create(
        model=model,
        max_tokens=1024,
        system=_TEXT_SYSTEM_PROMPT,
        messages=[{"role": "user", "content": f"Document text:\n\n{text}\n\nExtract metadata JSON."}],
    )
    return _parse_json_response(resp.content[0].text)


def _anthropic_vision_extract(filepaths: List[str], config: dict) -> Dict[str, Any]:
    try:
        import anthropic
    except ImportError:
        raise ImportError("anthropic package required for Anthropic vision")
    model = config.get("anthropic_model", "claude-sonnet-4-20250514")
    base_url = config.get("anthropic_base_url", "https://api.anthropic.com")
    timeout = config.get("timeout", 60)
    client = anthropic.Anthropic(api_key=config.get("api_key"), base_url=base_url, timeout=timeout)
    content: list = []
    for fp in filepaths:
        b64 = _file_to_base64(fp)
        mime = _guess_mime(fp)
        content.append({
            "type": "image",
            "source": {"type": "base64", "media_type": mime, "data": b64},
        })
    content.append({"type": "text", "text": _VISION_SYSTEM_PROMPT})
    resp = client.messages.create(
        model=model,
        max_tokens=1024,
        messages=[{"role": "user", "content": content}],
    )
    return _parse_json_response(resp.content[0].text)


def _gemini_metadata_from_text(text: str, config: dict) -> Dict[str, Any]:
    try:
        import google.generativeai as genai
    except ImportError:
        raise ImportError("google-generativeai package required for Gemini provider")
    api_key = config.get("api_key")
    if not api_key:
        raise ValueError("Gemini API key is required")
    genai.configure(api_key=api_key)
    model_name = config.get("model", "gemini-2.0-flash")
    model = genai.GenerativeModel(
        model_name,
        generation_config={"response_mime_type": "application/json", "temperature": 0.0},
    )
    resp = model.generate_content(
        f"{_TEXT_SYSTEM_PROMPT}\n\nDocument text:\n{text}\n\nExtract metadata JSON."
    )
    return _parse_json_response(resp.text)


def _gemini_vision_extract(filepaths: List[str], config: dict) -> Dict[str, Any]:
    try:
        import google.generativeai as genai
    except ImportError:
        raise ImportError("google-generativeai package required for Gemini vision")
    api_key = config.get("api_key")
    if not api_key:
        raise ValueError("Gemini API key is required for vision extraction")
    genai.configure(api_key=api_key)
    model_name = config.get("model", "gemini-2.0-flash")
    model = genai.GenerativeModel(
        model_name,
        generation_config={"response_mime_type": "application/json", "temperature": 0.0},
    )
    content: list = [_VISION_SYSTEM_PROMPT]
    for fp in filepaths:
        mime = _guess_mime(fp)
        with open(fp, "rb") as f:
            data = f.read()
        content.append({"inline_data": {"mime_type": mime, "data": data}})
    resp = model.generate_content(content)
    return _parse_json_response(resp.text)


def _openai_compat_extract(text: str, config: dict) -> Dict[str, Any]:
    try:
        import openai
    except ImportError:
        raise ImportError("openai package required for OpenAI-compatible provider")
    base_url = config.get("base_url", config.get("ollama_base_url", "http://localhost:11434/v1"))
    model = config.get("model", config.get("ollama_model", "llama3.2"))
    api_key = config.get("api_key", "noop")
    temperature = config.get("temperature", 0.0)
    timeout = config.get("timeout", 30)
    client = openai.OpenAI(base_url=base_url, api_key=api_key, timeout=timeout)
    resp = client.chat.completions.create(
        model=model,
        temperature=temperature,
        messages=[
            {"role": "system", "content": _TEXT_SYSTEM_PROMPT},
            {"role": "user", "content": f"Document text:\n\n{text}\n\nExtract metadata JSON."},
        ],
        response_format={"type": "json_object"},
    )
    return _parse_json_response(resp.choices[0].message.content)


def _openai_compat_vision_extract(filepaths: List[str], config: dict) -> Dict[str, Any]:
    try:
        import openai
    except ImportError:
        raise ImportError("openai package required for OpenAI-compatible vision")
    base_url = config.get("base_url", config.get("ollama_base_url", "http://localhost:11434/v1"))
    model = config.get("model", config.get("ollama_model", "llama3.2"))
    api_key = config.get("api_key", "noop")
    temperature = config.get("temperature", 0.0)
    timeout = config.get("timeout", 60)
    client = openai.OpenAI(base_url=base_url, api_key=api_key, timeout=timeout)
    content: list = [{"type": "text", "text": _VISION_SYSTEM_PROMPT}]
    for fp in filepaths:
        b64 = _file_to_base64(fp)
        mime = _guess_mime(fp)
        content.append({
            "type": "image_url",
            "image_url": {"url": f"data:{mime};base64,{b64}", "detail": "auto"},
        })
    resp = client.chat.completions.create(
        model=model,
        temperature=temperature,
        messages=[{"role": "user", "content": content}],
        max_tokens=1024,
    )
    return _parse_json_response(resp.choices[0].message.content)


_RETRY_DELAYS = [1.0, 2.0, 4.0]


def _with_retry(fn, max_retries=3):
    last_exc = None
    for attempt in range(max_retries):
        try:
            return fn()
        except Exception as e:
            last_exc = e
            if attempt < max_retries - 1:
                time.sleep(_RETRY_DELAYS[attempt])
    raise last_exc


def test_api_connection(provider: str, api_key: str, config: Optional[dict] = None, model: str = "") -> tuple:
    start = time.time()
    if not api_key and provider != "ollama":
        return False, "API key is empty", 0
    try:
        if provider == "openai":
            import openai
            client = openai.OpenAI(api_key=api_key, timeout=10)
            models = client.models.list()
            count = len(list(models))
            ms = int((time.time() - start) * 1000)
            return True, f"Connected ({count} models, {ms}ms)", ms
        elif provider == "anthropic":
            import anthropic
            client = anthropic.Anthropic(api_key=api_key, timeout=10)
            client.models.list()
            ms = int((time.time() - start) * 1000)
            return True, f"Connected ({ms}ms)", ms
        elif provider == "gemini":
            import google.generativeai as genai
            genai.configure(api_key=api_key)
            model_name = model or (config or {}).get("ai", {}).get("model", "") or (config or {}).get("vision", {}).get("gemini", {}).get("model", "gemini-2.0-flash")
            m = genai.GenerativeModel(model_name)
            m.generate_content("OK", generation_config={"max_output_tokens": 1})
            ms = int((time.time() - start) * 1000)
            return True, f"Connected ({ms}ms)", ms
        elif provider == "ollama":
            import urllib.request
            base = (config or {}).get("ollama_base_url", "http://localhost:11434")
            base = base.rstrip("/")
            if base.endswith("/v1"):
                base = base[:-3]
            req = urllib.request.Request(f"{base}/api/tags", method="GET")
            with urllib.request.urlopen(req, timeout=5) as resp:
                data = json.loads(resp.read())
                models = data.get("models", [])
                ms = int((time.time() - start) * 1000)
                return True, f"Connected ({len(models)} models, {ms}ms)", ms
        elif provider == "xai":
            import openai
            client = openai.OpenAI(api_key=api_key, base_url="https://api.x.ai", timeout=10)
            models = client.models.list()
            count = len(list(models))
            ms = int((time.time() - start) * 1000)
            return True, f"Connected ({count} models, {ms}ms)", ms
        elif provider == "custom":
            import openai
            base_url = (config or {}).get("ai", {}).get("custom_base_url", "")
            if not base_url:
                return False, "Custom API base URL is required", 0
            client = openai.OpenAI(api_key=api_key or "noop", base_url=base_url, timeout=10)
            models = client.models.list()
            count = len(list(models))
            ms = int((time.time() - start) * 1000)
            return True, f"Connected ({count} models, {ms}ms)", ms
        else:
            return False, f"Unknown provider: {provider}", 0
    except Exception as e:
        msg = str(e).strip()
        ms = int((time.time() - start) * 1000)
        return False, f"Failed: {msg} ({ms}ms)", ms


_TEXT_PROVIDER_MAP = {
    "openai": _openai_extract,
    "anthropic": _anthropic_extract,
    "gemini": _gemini_metadata_from_text,
    "ollama": lambda text, cfg: _openai_compat_extract(text, {**cfg, "base_url": cfg.get("ollama_base_url", "http://localhost:11434/v1"), "model": cfg.get("ollama_model", "llama3.2")}),
    "xai": lambda text, cfg: _openai_compat_extract(text, {**cfg, "base_url": cfg.get("xai_base_url", "https://api.x.ai"), "model": cfg.get("xai_model", "grok-3-beta")}),
    "custom": lambda text, cfg: _openai_compat_extract(text, {**cfg, "base_url": cfg.get("custom_base_url", ""), "model": cfg.get("custom_model", "")}),
}

_VISION_PROVIDER_MAP = {
    "openai": _openai_vision_extract,
    "anthropic": _anthropic_vision_extract,
    "gemini": _gemini_vision_extract,
    "ollama": lambda fps, cfg: _openai_compat_vision_extract(fps, {**cfg, "base_url": cfg.get("ollama_base_url", "http://localhost:11434/v1"), "model": cfg.get("ollama_model", "llama3.2")}),
    "xai": lambda fps, cfg: _openai_compat_vision_extract(fps, {**cfg, "base_url": cfg.get("xai_base_url", "https://api.x.ai"), "model": cfg.get("xai_model", "grok-3-beta")}),
    "custom": lambda fps, cfg: _openai_compat_vision_extract(fps, {**cfg, "base_url": cfg.get("custom_base_url", ""), "model": cfg.get("custom_model", "")}),
}


def _data_to_metadata(data: dict) -> DocumentMetadata:
    confidence = float(data.get("confidence", 0.0))
    return DocumentMetadata(
        company_name=str(data.get("company_name", "")),
        document_date=str(data.get("document_date", "")),
        document_type=str(data.get("document_type", "")),
        confidence=min(max(confidence, 0.0), 1.0),
        invoice_number=str(data.get("invoice_number", "")),
        total_amount=str(data.get("total_amount", "")),
    )


def _resolve_ai_config(config: dict, provider: str) -> dict:
    if provider == "gemini":
        ai = config.get("ai", {})
        return {
            "api_key": ai.get("gemini_api_key") or ai.get("api_key", ""),
            "model": ai.get("gemini_model") or ai.get("model", "gemini-2.0-flash"),
            "base_url": ai.get("gemini_base_url", ""),
            "timeout": ai.get("timeout", 30),
            "temperature": ai.get("temperature", 0.0),
        }
    if provider in ("openai", "custom"):
        return config.get("ai", {})
    if provider == "anthropic":
        ai = config.get("ai", {})
        return {
            "api_key": ai.get("api_key", ""),
            "model": ai.get("anthropic_model", "claude-3-5-haiku-latest"),
            "base_url": ai.get("anthropic_base_url", "https://api.anthropic.com"),
            "timeout": ai.get("timeout", 30),
            "temperature": ai.get("temperature", 0.0),
        }
    if provider == "ollama":
        ai = config.get("ai", {})
        return {
            "api_key": ai.get("api_key", "noop"),
            "base_url": ai.get("ollama_base_url", "http://localhost:11434/v1"),
            "model": ai.get("ollama_model", "llama3.2"),
            "timeout": ai.get("timeout", 30),
            "temperature": ai.get("temperature", 0.0),
        }
    if provider == "xai":
        ai = config.get("ai", {})
        return {
            "api_key": ai.get("api_key", ""),
            "base_url": ai.get("xai_base_url", "https://api.x.ai"),
            "model": ai.get("xai_model", "grok-3-beta"),
            "timeout": ai.get("timeout", 30),
            "temperature": ai.get("temperature", 0.0),
        }
    return config.get("ai", {})


def extract_metadata(text: str, config: dict, provider: Optional[str] = None) -> DocumentMetadata:
    p = provider or config.get("ai", {}).get("provider", "openai")
    extractor = _TEXT_PROVIDER_MAP.get(p)
    if not extractor:
        raise ValueError(f"Unknown provider: {p}. Supported: {list(_TEXT_PROVIDER_MAP.keys())}")
    ai_config = _resolve_ai_config(config, p)
    try:
        return _with_retry(lambda: _data_to_metadata(extractor(text, ai_config)))
    except Exception as e:
        log.warning(f"AI extraction failed with provider '{p}' after retries: {e}")
        raise


def extract_vision_metadata(
    filepaths: Union[str, List[str]],
    config: dict,
    provider: Optional[str] = None,
) -> DocumentMetadata:
    if isinstance(filepaths, str):
        filepaths = [filepaths]
    p = provider or config.get("ai", {}).get("provider", "openai")
    vision_extractor = _VISION_PROVIDER_MAP.get(p)
    if not vision_extractor:
        raise ValueError(f"Vision extraction not supported for provider: '{p}'. Supported: {list(_VISION_PROVIDER_MAP.keys())}")
    ai_config = _resolve_ai_config(config, p)
    try:
        return _with_retry(lambda: _data_to_metadata(vision_extractor(filepaths, ai_config)))
    except Exception as e:
        log.warning(f"Vision extraction failed with provider '{p}' after retries: {e}")
        raise
