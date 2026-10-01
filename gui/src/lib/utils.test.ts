import { describe, it, expect } from 'vitest';
import { extensionOf, isSupportedFile, escapeHtml, SUPPORTED_EXTENSIONS } from './utils';

describe('extensionOf', () => {
  it('lower-cases the extension and keeps the dot', () => {
    expect(extensionOf('Report.PDF')).toBe('.pdf');
    expect(extensionOf('sheet.xlsx')).toBe('.xlsx');
  });

  it('ignores dots in directory names', () => {
    // The old regex `replace(/.*[.](\w+)$/, '.$1')` returned '.2\\file' here.
    expect(extensionOf('C:\\release.v1.2\\file')).toBe('');
    expect(extensionOf('C:\\release.v1.2\\file.pdf')).toBe('.pdf');
    expect(extensionOf('/home/user.name/notes.txt')).toBe('.txt');
  });

  it('treats a leading dot as a dotfile, not an extension', () => {
    expect(extensionOf('.gitignore')).toBe('');
    expect(extensionOf('/home/.env')).toBe('');
  });

  it('returns an empty string when there is no extension', () => {
    expect(extensionOf('README')).toBe('');
    expect(extensionOf('C:\\folder\\name')).toBe('');
    expect(extensionOf('')).toBe('');
  });

  it('handles a trailing dot', () => {
    expect(extensionOf('weird.')).toBe('.');
    expect(isSupportedFile('weird.')).toBe(false);
  });
});

describe('isSupportedFile', () => {
  it('accepts every advertised extension', () => {
    for (const ext of SUPPORTED_EXTENSIONS) {
      expect(isSupportedFile(`file${ext}`)).toBe(true);
    }
  });

  it('is case-insensitive', () => {
    expect(isSupportedFile('SCAN.JPG')).toBe(true);
    expect(isSupportedFile('Doc.DOCX')).toBe(true);
  });

  it('rejects unsupported and extension-less paths', () => {
    expect(isSupportedFile('archive.zip')).toBe(false);
    expect(isSupportedFile('program.exe')).toBe(false);
    expect(isSupportedFile('noextension')).toBe(false);
    expect(isSupportedFile('C:\\folder.with.dot\\noextension')).toBe(false);
  });
});

describe('escapeHtml', () => {
  it('escapes every HTML-significant character', () => {
    expect(escapeHtml(`<img src=x onerror="alert('1')">`)).toBe(
      '&lt;img src=x onerror=&quot;alert(&#39;1&#39;)&quot;&gt;',
    );
  });

  it('escapes ampersands first so entities are not double-decoded', () => {
    expect(escapeHtml('A & B')).toBe('A &amp; B');
    expect(escapeHtml('&lt;')).toBe('&amp;lt;');
  });

  it('leaves plain text untouched', () => {
    expect(escapeHtml('3 files renamed')).toBe('3 files renamed');
  });
});
