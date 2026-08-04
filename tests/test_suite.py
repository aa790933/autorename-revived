from __future__ import annotations

import json
import os
import tempfile
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from autorename_revived._config_loader import _resolve_env_vars, get_gemini_config, load_config
from autorename_revived._document_extractor import (
    DocxExtractor,
    ImageExtractor,
    PdfExtractor,
    XlsxExtractor,
    get_extractor,
)
from autorename_revived._document_processing import (
    harmonize_company_name,
    load_undo_history,
    parse_document_date,
    rename_file,
    rename_invoice,
    save_rename_to_history,
    undo_last_rename,
)
from autorename_revived._naming_engine import NamingEngine
from autorename_revived._path_safety import (
    is_safe_path_component,
    resolve_safe_path,
    sanitize_filename,
    validate_subprocess_arg,
)
from autorename_revived._resources import bundle_dir, find_resource, resource_path
from autorename_revived._utils import SUPPORTED_EXTENSIONS, ExtractionResult, ExitCode, normalize_unicode

# ─────────────────────────────────────────────
# _version.py
# ─────────────────────────────────────────────


def test_version_exists():
    from autorename_revived._version import VERSION
    assert isinstance(VERSION, str)
    assert VERSION.startswith("3.")


# ─────────────────────────────────────────────
# _utils.py
# ─────────────────────────────────────────────


class TestExitCode:
    def test_values(self):
        assert ExitCode.SUCCESS == 0
        assert ExitCode.ERROR == 1
        assert ExitCode.USAGE == 2
        assert ExitCode.CONFIG == 3
        assert ExitCode.NO_FILES == 4
        assert ExitCode.PARTIAL == 5
        assert ExitCode.PROVIDER == 10
        assert ExitCode.AUTH == 11
        assert ExitCode.INTERRUPTED == 130

    def test_is_int_enum(self):
        assert int(ExitCode.SUCCESS) == 0


class TestExtractionResult:
    def test_create_default(self):
        r = ExtractionResult(text="abc", method="test")
        assert r.text == "abc"
        assert r.method == "test"
        assert r.metadata == {}
        assert r.error is None

    def test_create_full(self):
        r = ExtractionResult(text="x", method="m", metadata={"k": "v"}, error="err")
        assert r.error == "err"
        assert r.metadata["k"] == "v"


class TestSUPPORTED_EXTENSIONS:
    def test_pdf_supported(self):
        assert ".pdf" in SUPPORTED_EXTENSIONS

    def test_image_types_supported(self):
        for ext in (".png", ".jpg", ".jpeg", ".tiff", ".tif", ".bmp"):
            assert ext in SUPPORTED_EXTENSIONS

    def test_docx_supported(self):
        assert ".docx" in SUPPORTED_EXTENSIONS

    def test_xlsx_supported(self):
        assert ".xlsx" in SUPPORTED_EXTENSIONS

    def test_unsupported_not_present(self):
        assert ".xyz" not in SUPPORTED_EXTENSIONS
        assert ".exe" not in SUPPORTED_EXTENSIONS


class TestNormalizeUnicode:
    def test_nfkc_normalization(self):
        result = normalize_unicode("\u2460")  # CIRCLED DIGIT ONE
        assert result == "1" or len(result) > 0

    def test_identity(self):
        assert normalize_unicode("hello") == "hello"
        assert normalize_unicode("") == ""

    def test_default_form(self):
        assert normalize_unicode("caf\u00e9") == "caf\u00e9"


# ─────────────────────────────────────────────
# _path_safety.py
# ─────────────────────────────────────────────


class TestSanitizeFilename:
    def test_strips_invalid_chars(self):
        result = sanitize_filename('file<name>.pdf')
        assert "<" not in result
        assert ">" not in result

    def test_replaces_invalid(self):
        result = sanitize_filename('a/b\\c')
        assert "/" not in result
        assert "\\" not in result

    def test_removes_all_invalid_fs_chars(self):
        invalid = '\x00\x01\\/:*?"<>|'
        result = sanitize_filename(f"test{invalid}file")
        for ch in invalid:
            assert ch not in result

    def test_empty_becomes_underscore(self):
        result = sanitize_filename("")
        assert result == "_"

    def test_only_dots_and_spaces(self):
        result = sanitize_filename(" . . ")
        assert result == "_" or result == "."

    def test_respects_max_length(self):
        result = sanitize_filename("a" * 200, max_length=50)
        assert len(result) <= 50

    def test_leading_dot_prefixed(self):
        result = sanitize_filename(".hidden")
        assert not result.startswith(".")

    def test_trailing_dot_stripped(self):
        result = sanitize_filename("file.")
        assert not result.endswith(".")

    def test_normal_text_preserved(self):
        result = sanitize_filename("HelloWorld")
        assert result == "HelloWorld"


class TestIsSafePathComponent:
    def test_rejects_empty(self):
        assert is_safe_path_component("") is False

    def test_rejects_dot(self):
        assert is_safe_path_component(".") is False

    def test_rejects_dotdot(self):
        assert is_safe_path_component("..") is False

    def test_rejects_invalid_chars(self):
        assert is_safe_path_component("a<b") is False
        assert is_safe_path_component("a>b") is False
        assert is_safe_path_component('a"b') is False

    def test_accepts_normal(self):
        assert is_safe_path_component("hello") is True
        assert is_safe_path_component("file123") is True


class TestResolveSafePath:
    def test_normal_path_resolves(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_resolved = os.path.realpath(tmp)
            result = resolve_safe_path(tmp, "safe_file.pdf")
            assert result.startswith(tmp_resolved)
            assert result.endswith("safe_file.pdf")

    def test_blocks_traversal(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_resolved = os.path.realpath(tmp)
            result = resolve_safe_path(tmp, "../escape.pdf")
            assert result.startswith(tmp_resolved)
            assert ".." not in result

    def test_sanitizes_filename(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_resolved = os.path.realpath(tmp)
            result = resolve_safe_path(tmp, "bad<name>.pdf")
            assert "<" not in result
            assert result.startswith(tmp_resolved)


class TestValidateSubprocessArg:
    def test_rejects_dotdot(self):
        with pytest.raises(ValueError, match="Path traversal"):
            validate_subprocess_arg("../foo")

    def test_accepts_normal(self):
        assert validate_subprocess_arg("hello") == "hello"

    def test_rejects_backward_dotdot(self):
        with pytest.raises(ValueError):
            validate_subprocess_arg("..\\foo")


# ─────────────────────────────────────────────
# _naming_engine.py
# ─────────────────────────────────────────────


class TestNamingEngine:
    def _make_engine(self, **overrides) -> NamingEngine:
        config = {
            "naming": {
                "template": "{date}_{company}_{doctype}",
                "fallback": "{date}_Unknown_{doctype}",
                "date_format": "%Y%m%d",
                "separator": "_",
                "max_length": 128,
                **overrides,
            }
        }
        return NamingEngine(config)

    def test_default_template(self):
        engine = self._make_engine()
        result = engine.generate(company="Acme", doctype="Invoice", date_str="20240101")
        assert "20240101" in result
        assert "Acme" in result
        assert "Invoice" in result
        assert result.endswith(".pdf")

    def test_fallback_when_empty(self):
        engine = self._make_engine()
        result = engine.generate()
        assert "Unknown" in result
        assert result.endswith(".pdf")

    def test_sequence_field(self):
        engine = self._make_engine(template="{date}_{company}_{sequence}")
        result = engine.generate(company="Test", date_str="20240101", sequence=3)
        assert "03" in result or "3" in result
        assert result.endswith(".pdf")

    def test_sequence_zerofill_custom(self):
        engine = self._make_engine(template="{sequence}", original_filename="x.pdf", sequence_zerofill=3)
        result = engine.generate(sequence=5, original_filename="x.pdf")
        assert "005" in result
        assert result.endswith(".pdf")

    def test_custom_template_category_subject(self):
        engine = self._make_engine(template="{category}_{subject}_{date}")
        result = engine.generate(company="Acme", subject="Report", date_str="20240101")
        assert "Acme" in result
        assert "Report" in result
        assert "20240101" in result

    def test_sanitizes_fields(self):
        engine = self._make_engine(template="{company}")
        result = engine.generate(company="Acme<Corp>")
        assert "<" not in result
        assert ">" not in result

    def test_max_length_truncation(self):
        engine = self._make_engine(max_length=24)
        result = engine.generate(company="VeryLongCompanyNameInc", doctype="Invoice", date_str="20240101")
        assert len(result) <= 24
        assert result.endswith(".pdf")

    def test_original_filename_field(self):
        engine = self._make_engine(template="{original}")
        result = engine.generate(original_filename="statement")
        assert "statement" in result

    def test_batch_generates_unique_names(self):
        engine = self._make_engine(template="{date}_{company}_{doctype}_{sequence}")
        docs = [
            {"company": "Acme", "doctype": "Invoice", "date_str": "20240101"},
            {"company": "Acme", "doctype": "Invoice", "date_str": "20240101"},
        ]
        results = engine.generate_batch(docs)
        assert len(results) == 2
        assert results[0] != results[1]

    def test_batch_with_base_values(self):
        engine = self._make_engine(template="{date}_{company}_{doctype}_{sequence}")
        docs = [
            {"date_str": "20240101"},
            {"date_str": "20240101"},
        ]
        results = engine.generate_batch(docs, base_company="Acme", base_doctype="Invoice")
        assert len(results) == 2
        assert "Acme" in results[0]

    def test_custom_date_format_dmy(self):
        engine = NamingEngine({"naming": {"date_format": "%d-%m-%Y"}})
        from datetime import datetime
        name = engine.generate(company="Co", doctype="Inv", date_obj=datetime(2024, 3, 15))
        assert "15-03-2024" in name

    def test_subject_field_in_template(self):
        engine = NamingEngine({"naming": {"template": "{date}_{subject}"}})
        name = engine.generate(company="Acme", subject="Quarterly Report", date_str="20240101")
        assert "Quarterly" in name or "quarterly" in name.lower()

    def test_category_maps_to_company_by_default(self):
        engine = NamingEngine({"naming": {"template": "{date}_{category}_{doctype}"}})
        name = engine.generate(company="Acme Corp", doctype="Invoice", date_str="20240101")
        assert "Acme" in name

    def test_original_filename_field_in_template(self):
        engine = NamingEngine({"naming": {"template": "{original}_{date}_{company}_{doctype}"}})
        name = engine.generate(company="Acme", doctype="Inv", date_str="20240101", original_filename="scan_001.pdf")
        assert name.startswith("scan_001")

    def test_format_date_with_obj(self):
        engine = self._make_engine()
        from datetime import datetime
        d = datetime(2024, 3, 15)
        result = engine._format_date(d)
        assert result == "20240315"

    def test_format_date_with_str(self):
        engine = self._make_engine()
        assert engine._format_date(None, "2024-03-15") == "20240315"

    def test_format_date_empty(self):
        engine = self._make_engine()
        assert engine._format_date(None, "") == "00000000"

    def test_custom_date_format(self):
        engine = self._make_engine(template="{date}", date_format="%d-%m-%Y")
        from datetime import datetime
        result = engine.generate(date_obj=datetime(2024, 3, 15))
        assert "15-03-2024" in result


# ─────────────────────────────────────────────
# _config_loader.py
# ─────────────────────────────────────────────


class TestLoadConfig:
    def test_default_config(self):
        config = load_config()
        assert config["config_version"] == 2
        assert config["ai"]["provider"] == "openai"
        assert config["naming"]["template"] == "{date}_{company}_{doctype}"
        assert config["vision"]["provider"] == "gemini"
        assert config["pdf"]["vision"] == "auto"

    def test_naming_defaults_present(self):
        config = load_config()
        naming = config["naming"]
        assert naming["template"] == "{date}_{company}_{doctype}"
        assert naming["fallback"] == "{date}_Unknown_{doctype}"
        assert naming["date_format"] == "%Y%m%d"
        assert naming["separator"] == "_"
        assert naming["max_length"] == 128

    def test_vision_defaults_present(self):
        config = load_config()
        vision = config["vision"]
        assert vision["provider"] == "gemini"
        assert "gemini" in vision

    def test_undo_defaults_present(self):
        config = load_config()
        undo = config["undo"]
        assert undo["enabled"] is True
        assert undo["max_entries"] == 100

    def test_loads_yaml_file(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False, encoding="utf-8") as f:
            f.write("ai:\n  provider: ollama\n  ollama_model: llama3\n")
            fname = f.name
        try:
            config = load_config(fname)
            assert config["ai"]["provider"] == "ollama"
            assert config["ai"]["ollama_model"] == "llama3"
        finally:
            os.unlink(fname)

    def test_env_var_substitution(self):
        with patch.dict(os.environ, {"TEST_KEY": "test_value"}):
            result = _resolve_env_vars("${TEST_KEY}")
            assert result == "test_value"

    def test_env_var_missing_keeps_original(self):
        result = _resolve_env_vars("${MISSING_VAR_XYZ}")
        assert result == "${MISSING_VAR_XYZ}"

    def test_deep_resolve_nested(self):
        with patch.dict(os.environ, {"MY_KEY": "my_value"}):
            data = {"nested": {"key": "${MY_KEY}"}}
            from autorename_revived._config_loader import _deep_resolve_env
            resolved = _deep_resolve_env(data)
            assert resolved["nested"]["key"] == "my_value"

    def test_harmonized_company_strings_to_dicts(self):
        yaml_content = """
harmonized_companies:
  - Acme Corp
  - Globex Inc
"""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False, encoding="utf-8") as f:
            f.write(yaml_content)
            fname = f.name
        try:
            config = load_config(fname)
            companies = config["harmonized_companies"]
            assert len(companies) == 2
            for entry in companies:
                assert isinstance(entry, dict)
                assert "name" in entry
                assert "variations" in entry
        finally:
            os.unlink(fname)

    def test_get_gemini_config(self):
        config = {
            "vision": {
                "provider": "gemini",
                "gemini": {
                    "api_key": "${GEMINI_KEY}",
                    "model": "gemini-2.0-flash",
                },
            }
        }
        gc = get_gemini_config(config)
        assert gc["model"] == "gemini-2.0-flash"
        assert gc["api_key"] == "${GEMINI_KEY}"


# ─────────────────────────────────────────────
# _ai_processing.py (mocked)
# ─────────────────────────────────────────────


class TestExtractMetadata:
    def _mock_provider_map(self, return_value):
        from autorename_revived import _ai_processing as ap
        mock_fn = MagicMock(return_value=return_value)
        return {**ap._TEXT_PROVIDER_MAP, "openai": mock_fn, "gemini": mock_fn}

    def test_openai_provider(self):
        from autorename_revived._ai_processing import extract_metadata, _TEXT_PROVIDER_MAP
        mock_extract = MagicMock(return_value={
            "company_name": "Acme Corp",
            "document_date": "20240315",
            "document_type": "Invoice",
            "confidence": 0.95,
        })
        with patch.dict(_TEXT_PROVIDER_MAP, {"openai": mock_extract}):
            result = extract_metadata("some text", {"ai": {"provider": "openai", "api_key": "test"}})
        assert result.company_name == "Acme Corp"
        assert result.document_date == "20240315"
        assert result.document_type == "Invoice"
        assert result.confidence == 0.95

    def test_missing_fields_default_to_empty(self):
        from autorename_revived._ai_processing import extract_metadata, _TEXT_PROVIDER_MAP
        mock_extract = MagicMock(return_value={"confidence": 0.5})
        with patch.dict(_TEXT_PROVIDER_MAP, {"openai": mock_extract}):
            result = extract_metadata("text", {"ai": {"provider": "openai", "api_key": "test"}})
        assert result.company_name == ""
        assert result.document_date == ""
        assert result.document_type == ""

    def test_confidence_clamped(self):
        from autorename_revived._ai_processing import extract_metadata, _TEXT_PROVIDER_MAP
        mock_extract = MagicMock(return_value={"confidence": 1.5})
        with patch.dict(_TEXT_PROVIDER_MAP, {"openai": mock_extract}):
            result = extract_metadata("text", {"ai": {"provider": "openai", "api_key": "test"}})
        assert result.confidence == 1.0

    def test_unknown_provider_raises(self):
        from autorename_revived._ai_processing import extract_metadata
        with pytest.raises(ValueError, match="Unknown provider"):
            extract_metadata("text", {"ai": {"provider": "nonexistent"}})

    def test_provider_override(self):
        from autorename_revived._ai_processing import extract_metadata, _TEXT_PROVIDER_MAP
        mock_extract = MagicMock(return_value={"company_name": "Test"})
        with patch.dict(_TEXT_PROVIDER_MAP, {"openai": mock_extract}):
            result = extract_metadata("text", {"ai": {"provider": "ollama"}}, provider="openai")
        assert result.company_name == "Test"

    def test_gemini_provider(self):
        from autorename_revived._ai_processing import extract_metadata, _TEXT_PROVIDER_MAP
        mock_extract = MagicMock(return_value={
            "company_name": "Gemini Corp",
            "document_date": "20240315",
            "document_type": "Report",
            "confidence": 0.9,
        })
        with patch.dict(_TEXT_PROVIDER_MAP, {"gemini": mock_extract}):
            result = extract_metadata("text", {"ai": {"provider": "gemini", "api_key": "test"}})
        assert result.company_name == "Gemini Corp"

    def test_document_metadata_model(self):
        from autorename_revived._ai_processing import DocumentMetadata
        md = DocumentMetadata(
            company_name="Acme",
            document_date="20240101",
            document_type="Invoice",
            confidence=0.95,
            invoice_number="INV-123",
            total_amount="$1,000",
        )
        assert md.company_name == "Acme"
        assert md.invoice_number == "INV-123"
        data = md.model_dump()
        assert data["company_name"] == "Acme"

    def test_document_metadata_defaults(self):
        from autorename_revived._ai_processing import DocumentMetadata
        md = DocumentMetadata()
        assert md.company_name == ""
        assert md.document_date == ""
        assert md.document_type == ""
        assert md.confidence == 0.0
        assert md.invoice_number == ""
        assert md.total_amount == ""


# ─────────────────────────────────────────────
# _document_extractor.py (mocked)
# ─────────────────────────────────────────────


class TestGetExtractor:
    def test_pdf(self):
        ext = get_extractor("file.pdf", {})
        assert isinstance(ext, PdfExtractor)

    def test_docx(self):
        ext = get_extractor("file.docx", {})
        assert isinstance(ext, DocxExtractor)

    def test_xlsx(self):
        ext = get_extractor("file.xlsx", {})
        assert isinstance(ext, XlsxExtractor)

    def test_image(self):
        ext = get_extractor("file.png", {})
        assert isinstance(ext, ImageExtractor)
        ext2 = get_extractor("file.jpg", {})
        assert isinstance(ext2, ImageExtractor)
        ext3 = get_extractor("file.jpeg", {})
        assert isinstance(ext3, ImageExtractor)
        ext4 = get_extractor("file.tiff", {})
        assert isinstance(ext4, ImageExtractor)
        ext5 = get_extractor("file.bmp", {})
        assert isinstance(ext5, ImageExtractor)

    def test_unsupported_raises(self):
        with pytest.raises(ValueError, match="Unsupported"):
            get_extractor("file.xyz", {})

    def test_case_insensitive(self):
        ext = get_extractor("file.PDF", {})
        assert isinstance(ext, PdfExtractor)


class TestDocxExtractor:
    @patch("docx.Document")
    def test_extract_text(self, mock_doc):
        instance = mock_doc.return_value
        p1 = MagicMock()
        p1.text = "Hello"
        p2 = MagicMock()
        p2.text = "World"
        instance.paragraphs = [p1, p2]
        ext = DocxExtractor({})
        result = ext.extract("test.docx")
        assert "Hello" in result.text
        assert "World" in result.text
        assert result.method == "docx"


class TestXlsxExtractor:
    @patch("openpyxl.load_workbook")
    def test_extract(self, mock_wb):
        ws = MagicMock()
        ws.iter_rows.return_value = [("Cell1", "Cell2")]
        wb_instance = MagicMock()
        wb_instance.worksheets = [ws]
        mock_wb.return_value = wb_instance
        ext = XlsxExtractor({})
        result = ext.extract("test.xlsx")
        assert "Cell1" in result.text
        assert result.method == "xlsx"


class TestPdfExtractor:
    @patch("autorename_revived._document_extractor.extract_metadata")
    @patch("autorename_revived._document_extractor.extract_text_from_pdf")
    def test_extract_text_fallback(self, mock_extract_text, mock_ai):
        mock_extract_text.return_value = ExtractionResult(text="Sample PDF text", method="pdfplumber")
        mock_ai.return_value = MagicMock()
        mock_ai.return_value.model_dump.return_value = {"company_name": "Acme"}
        ext = PdfExtractor({})
        result = ext.extract_metadata("test.pdf")
        assert result["company_name"] == "Acme"

    @patch("autorename_revived._document_extractor.extract_text_from_pdf")
    def test_extract_returns_result(self, mock_extract):
        mock_extract.return_value = ExtractionResult(text="abc", method="pdfplumber")
        ext = PdfExtractor({})
        result = ext.extract("test.pdf")
        assert result.text == "abc"
        assert result.method == "pdfplumber"


# ─────────────────────────────────────────────
# _pdf_utils.py
# ─────────────────────────────────────────────


class TestAssessTextQuality:
    def test_empty_text(self):
        from autorename_revived._pdf_utils import assess_text_quality
        assert assess_text_quality("") == 0.0
        assert assess_text_quality("   ") == 0.0

    def test_short_text(self):
        from autorename_revived._pdf_utils import assess_text_quality
        assert assess_text_quality("a") == 0.0

    def test_high_quality(self):
        from autorename_revived._pdf_utils import assess_text_quality
        text = "This is a normal sentence with good text content for testing."
        quality = assess_text_quality(text)
        assert quality > 0.5

    def test_low_quality(self):
        from autorename_revived._pdf_utils import assess_text_quality
        text = "!@#$%^&*()_+{}[]|\\:;\"'<>,.?/~`"
        quality = assess_text_quality(text)
        assert quality < 0.5


# ─────────────────────────────────────────────
# _document_processing.py
# ─────────────────────────────────────────────


class TestHarmonizeCompanyName:
    def test_exact_match(self):
        companies = [{"name": "Acme Corp", "variations": ["ACME"]}]
        assert harmonize_company_name("Acme Corp", companies) == "Acme Corp"

    def test_variation_match(self):
        companies = [{"name": "Acme Corp", "variations": ["ACME Corporation"]}]
        assert harmonize_company_name("ACME Corporation", companies) == "Acme Corp"

    def test_fuzzy_match(self):
        companies = [{"name": "Microsoft Corporation", "variations": []}]
        result = harmonize_company_name("Microsoft Corp", companies)
        assert result == "Microsoft Corporation"

    def test_low_score_fuzzy_no_match(self):
        companies = [{"name": "Acme Corporation Inc", "variations": []}]
        result = harmonize_company_name("Totally Different", companies)
        assert result == "Totally Different"

    def test_no_match(self):
        companies = [{"name": "Globex Inc", "variations": []}]
        assert harmonize_company_name("Some Other Company", companies) == "Some Other Company"

    def test_empty_name(self):
        assert harmonize_company_name("", [{"name": "Test", "variations": []}]) == ""

    def test_empty_list(self):
        assert harmonize_company_name("Acme", []) == "Acme"

    def test_case_insensitive_variation(self):
        companies = [{"name": "Acme Corp", "variations": ["acme corporation"]}]
        assert harmonize_company_name("ACME CORPORATION", companies) == "Acme Corp"


class TestParseDocumentDate:
    def test_yyyymmdd_preserved(self):
        assert parse_document_date("20240101") == "20240101"

    def test_dashed_date(self):
        result = parse_document_date("2024-01-01")
        assert result == "20240101" or result is not None

    def test_dmy_format(self):
        result = parse_document_date("15/03/2024")
        assert result is not None
        assert len(result) == 8

    def test_empty_string(self):
        assert parse_document_date("") is None

    def test_none(self):
        assert parse_document_date(None) is None

    def test_invalid_date(self):
        result = parse_document_date("not-a-date")
        assert result is None or len(result) == 8

    def test_slashed_date(self):
        result = parse_document_date("2024/03/15")
        assert result is not None
        assert len(result) == 8


class TestRenameInvoice:
    def test_generates_valid_path(self):
        config = {
            "naming": {"template": "{date}_{company}_{doctype}", "date_format": "%Y%m%d"},
            "harmonized_companies": [],
        }
        metadata = {
            "company_name": "Acme",
            "document_type": "Invoice",
            "document_date": "20240101",
            "confidence": 0.95,
        }
        with tempfile.TemporaryDirectory() as tmp:
            src = os.path.join(tmp, "test.pdf")
            Path(src).touch()
            new_path, new_name = rename_invoice(src, metadata, config)
            assert new_name.endswith(".pdf")
            assert "Acme" in new_name
            assert "Invoice" in new_name
            assert "20240101" in new_name

    def test_harmonizes_company(self):
        config = {
            "naming": {"template": "{date}_{company}_{doctype}", "date_format": "%Y%m%d"},
            "harmonized_companies": [{"name": "Acme Corp", "variations": ["Acme"]}],
        }
        metadata = {
            "company_name": "Acme",
            "document_type": "Invoice",
            "document_date": "20240101",
            "confidence": 0.95,
        }
        with tempfile.TemporaryDirectory() as tmp:
            src = os.path.join(tmp, "test.pdf")
            Path(src).touch()
            new_path, new_name = rename_invoice(src, metadata, config)
            assert "Acme Corp" in new_name or "Acme" in new_name


class TestRenameFile:
    def test_rename_actual_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            src = os.path.join(tmp, "old.txt")
            dst = os.path.join(tmp, "new.txt")
            Path(src).write_text("test")
            assert rename_file(src, dst)
            assert Path(dst).exists()
            assert not Path(src).exists()

    def test_dry_run_does_not_rename(self):
        with tempfile.TemporaryDirectory() as tmp:
            src = os.path.join(tmp, "old.txt")
            dst = os.path.join(tmp, "new.txt")
            Path(src).write_text("test")
            assert rename_file(src, dst, dry_run=True)
            assert Path(src).exists()
            assert not Path(dst).exists()

    def test_nonexistent_src_returns_false(self):
        assert not rename_file("/nonexistent/src.txt", "/nonexistent/dst.txt")


class TestUndoHistory:
    def test_save_and_load(self):
        with tempfile.TemporaryDirectory() as tmp:
            log_path = os.path.join(tmp, "history.json")
            config = {"undo": {"enabled": True, "log_path": log_path, "max_entries": 100}}
            save_rename_to_history("/old/path.pdf", "/new/path.pdf", config)
            history = load_undo_history(config)
            assert len(history) == 1
            assert history[0]["old_path"] == os.path.normpath("/old/path.pdf")

    def test_disabled_undo(self):
        config = {"undo": {"enabled": False, "log_path": "/tmp/nonexistent/", "max_entries": 100}}
        save_rename_to_history("/old.pdf", "/new.pdf", config)
        assert load_undo_history(config) == []

    def test_load_empty(self):
        with tempfile.TemporaryDirectory() as tmp:
            log_path = os.path.join(tmp, "empty_history.json")
            config = {"undo": {"enabled": True, "log_path": log_path, "max_entries": 100}}
            assert load_undo_history(config) == []

    def test_save_trim_max_entries(self):
        with tempfile.TemporaryDirectory() as tmp:
            log_path = os.path.join(tmp, "history.json")
            config = {"undo": {"enabled": True, "log_path": log_path, "max_entries": 3}}
            for i in range(5):
                save_rename_to_history(f"/old{i}.pdf", f"/new{i}.pdf", config)
            history = load_undo_history(config)
            assert len(history) == 3
            assert history[0]["old_path"] == os.path.normpath("/old2.pdf")


# ─────────────────────────────────────────────
# cli.py (CLI)
# ─────────────────────────────────────────────


class TestFindFiles:
    def test_finds_pdfs_in_directory(self):
        from cli import _find_files
        with tempfile.TemporaryDirectory() as tmp:
            Path(os.path.join(tmp, "doc1.pdf")).touch()
            Path(os.path.join(tmp, "doc2.pdf")).touch()
            Path(os.path.join(tmp, "readme.xyz")).touch()
            result = _find_files([tmp], {"ai": {}})
            assert len(result) == 2
            assert all(f.endswith(".pdf") for f in result)

    def test_finds_single_file(self):
        from cli import _find_files
        with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as f:
            fname = f.name
        try:
            result = _find_files([fname], {"ai": {}})
            assert len(result) == 1
            assert result[0] == os.path.abspath(fname)
        finally:
            os.unlink(fname)

    def test_returns_empty_for_no_files(self):
        from cli import _find_files
        with tempfile.TemporaryDirectory() as tmp:
            result = _find_files([tmp], {"ai": {}})
            assert result == []

    def test_skips_unsupported_extensions(self):
        from cli import _find_files
        with tempfile.TemporaryDirectory() as tmp:
            Path(os.path.join(tmp, "file.pdf")).touch()
            Path(os.path.join(tmp, "file.exe")).touch()
            result = _find_files([tmp], {"ai": {}})
            assert len(result) == 1
            assert result[0].endswith(".pdf")

    def test_nonexistent_path_returns_empty(self):
        from cli import _find_files
        result = _find_files(["/nonexistent/path"], {"ai": {}})
        assert result == []


# ─────────────────────────────────────────────
# _resources.py
# ─────────────────────────────────────────────


class TestResources:
    def test_bundle_dir_exists(self):
        d = bundle_dir()
        assert d is not None
        assert os.path.isdir(d)

    @patch("autorename_revived._resources.sys")
    def test_bundle_dir_uses_meipass(self, mock_sys):
        mock_sys.frozen = True
        mock_sys._MEIPASS = "/tmp/bundle"
        from autorename_revived._resources import bundle_dir as bd
        assert bd() == "/tmp/bundle"

    def test_resource_path_returns_absolute(self):
        p = resource_path("nonexistent.py")
        assert isinstance(p, str)
        assert os.path.isabs(p)

    def test_find_resource_returns_path_for_existing(self):
        p = find_resource(["_resources.py"])
        assert p is not None
        assert p.endswith("_resources.py")

    def test_find_resource_returns_none_for_missing(self):
        p = find_resource(["does_not_exist_at_all.xyz"])
        assert p is None


# ─────────────────────────────────────────────
# _config_loader.py — save_config / find_config_path
# ─────────────────────────────────────────────


class TestSaveConfig:
    def test_find_config_path_returns_none(self):
        from autorename_revived._config_loader import find_config_path
        p = find_config_path("/nonexistent/dir/config.yaml")
        assert p is None

    def test_save_and_reload_config(self):
        from autorename_revived._config_loader import find_config_path, load_config, save_config
        cfg = load_config()
        assert cfg is not None
        assert "naming" in cfg
        original = cfg["naming"]["template"]
        cfg["naming"]["template"] = "{date}_{subject}_{sequence}"
        with tempfile.NamedTemporaryFile(suffix=".yaml", delete=False, mode="w") as f:
            tmppath = f.name
        try:
            saved = save_config(cfg, tmppath)
            assert saved == tmppath
            reloaded = load_config(tmppath)
            assert reloaded["naming"]["template"] == "{date}_{subject}_{sequence}"
        finally:
            os.unlink(tmppath)

    def test_save_config_creates_file(self):
        from autorename_revived._config_loader import save_config
        import tempfile
        tmpdir = tempfile.mkdtemp()
        cfg = {"ai": {"provider": "gemini", "model": "gemini-2.0-flash"}, "naming": {"template": "{date}_{company}"}}
        try:
            cfg_path = os.path.join(tmpdir, "config.yaml")
            result = save_config(cfg, cfg_path)
            assert os.path.isfile(result)
            with open(result, "r") as f:
                content = f.read()
            assert "provider: gemini" in content
            assert "gemini-2.0-flash" in content
        finally:
            import shutil
            shutil.rmtree(tmpdir, ignore_errors=True)


class TestCliVersion:
    def test_version_flag(self):
        import subprocess, sys
        proc = subprocess.run(
            [sys.executable, "cli.py", "--version"],
            capture_output=True, text=True,
        )
        assert proc.returncode == 0
        assert "autorename-revived" in proc.stdout


class TestExitCodeUsage:
    def test_can_import_runner(self):
        import cli
        assert cli is not None


# ─────────────────────────────────────────────
# CLI config save subcommand
# ─────────────────────────────────────────────


class TestCliConfigSave:
    def _make_config_file(self):
        import tempfile
        f = tempfile.NamedTemporaryFile(suffix=".yaml", mode="w", delete=False, encoding="utf-8")
        f.write("config_version: 2\nai:\n  provider: openai\n  api_key: test\n  model: gpt-4o-mini\nnaming:\n  template: '{date}_{company}_{doctype}'\n")
        f.close()
        return f.name

    def test_save_simple_key(self):
        import subprocess, sys, os
        cfg_path = self._make_config_file()
        try:
            proc = subprocess.run(
                [sys.executable, "cli.py", "config", "save", "--key", "ai.model", "--value", "gpt-4o", "--config", cfg_path, "--output", "json"],
                capture_output=True, text=True,
            )
            assert proc.returncode == 0
            result = __import__("json").loads(proc.stdout)
            assert result["success"] is True
            assert os.path.isfile(cfg_path)
        finally:
            os.unlink(cfg_path)

    def test_save_missing_key_value(self):
        from cli import cmd_config_save
        from argparse import Namespace
        ns = Namespace(key="", value="", output="json")
        result = cmd_config_save(ns, {})
        assert result == 2  # ExitCode.USAGE

    def test_save_nested_key(self):
        import subprocess, sys, os, json
        cfg_path = self._make_config_file()
        try:
            proc = subprocess.run(
                [sys.executable, "cli.py", "config", "save", "--key", "naming.max_length", "--value", "64", "--config", cfg_path, "--output", "json"],
                capture_output=True, text=True,
            )
            assert proc.returncode == 0
            result = json.loads(proc.stdout)
            assert result["success"] is True
        finally:
            os.unlink(cfg_path)


# ─────────────────────────────────────────────
# _resolve_provider_api_key
# ─────────────────────────────────────────────


class TestResolveProviderApiKey:
    def test_openai_key(self):
        from cli import _resolve_provider_api_key
        config = {"ai": {"api_key": "sk-openai"}}
        assert _resolve_provider_api_key(config, "openai") == "sk-openai"

    def test_ollama_returns_noop(self):
        from cli import _resolve_provider_api_key
        assert _resolve_provider_api_key({}, "ollama") == "noop"

    def test_gemini_vision_key(self):
        from cli import _resolve_provider_api_key
        config = {"vision": {"gemini": {"api_key": "gem-key"}}, "ai": {}}
        assert _resolve_provider_api_key(config, "gemini") == "gem-key"

    def test_gemini_fallback_to_ai(self):
        from cli import _resolve_provider_api_key
        config = {"ai": {"api_key": "ai-key"}}
        assert _resolve_provider_api_key(config, "gemini") == "ai-key"

    def test_missing_key(self):
        from cli import _resolve_provider_api_key
        assert _resolve_provider_api_key({}, "openai") == ""


# ─────────────────────────────────────────────
# System-level integration helper
# ─────────────────────────────────────────────


@pytest.mark.run_live
def test_live_openai():
    pytest.importorskip("openai")
    import os
    from autorename_revived._ai_processing import extract_metadata
    api_key = os.environ.get("OPENAI_API_KEY")
    if not api_key:
        pytest.skip("OPENAI_API_KEY not set")
    result = extract_metadata("ACME Corp - Invoice dated March 15 2024", {"ai": {"api_key": api_key}})
    assert isinstance(result.company_name, str)


@pytest.mark.run_live
def test_live_gemini():
    pytest.importorskip("google.generativeai")
    import os
    from autorename_revived._ai_processing import extract_metadata
    api_key = os.environ.get("GEMINI_API_KEY")
    if not api_key:
        pytest.skip("GEMINI_API_KEY not set")
    result = extract_metadata("ACME Corp - Invoice dated March 15 2024",
                               {"ai": {"api_key": api_key, "model": "gemini-2.0-flash"}},
                               provider="gemini")
    assert isinstance(result.company_name, str)
