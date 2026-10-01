/**
 * Extensions the app can process, with the leading dot.
 *
 * Mirrors `extractors::supported_extensions()` in the Rust backend; the
 * backend is authoritative and re-rejects anything it cannot read.
 */
export const SUPPORTED_EXTENSIONS = [
  '.pdf', '.docx', '.doc', '.xlsx', '.xls', '.pptx', '.ppt', '.pptm',
  '.csv', '.txt', '.md', '.rtf', '.html', '.htm', '.json', '.xml',
  '.png', '.jpg', '.jpeg', '.webp', '.tiff', '.tif', '.bmp', '.gif',
];

/**
 * Lower-cased extension of a path, including the dot, or `''`.
 *
 * The previous implementation used a `replace(/.*[.](\w+)$/, '.$1')` regex,
 * which mangles paths that contain a dot in a directory name (`C:\v1.2\file`)
 * and returns the whole path when there is no extension at all.
 */
export function extensionOf(path: string): string {
  const fileName = path.split(/[\\/]/).pop() ?? '';
  const dot = fileName.lastIndexOf('.');
  // A leading dot means a dotfile (".gitignore"), not an extension.
  if (dot <= 0) return '';
  return fileName.slice(dot).toLowerCase();
}

export function isSupportedFile(path: string): boolean {
  return SUPPORTED_EXTENSIONS.includes(extensionOf(path));
}

export function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}
