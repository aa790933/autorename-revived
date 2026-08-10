export const SUPPORTED_EXTENSIONS = [
  '.pdf', '.docx', '.doc', '.xlsx', '.xls', '.pptx', '.ppt', '.pptm',
  '.csv', '.txt', '.md', '.rtf', '.html', '.htm', '.json', '.xml',
  '.png', '.jpg', '.jpeg', '.webp', '.tiff', '.tif', '.bmp', '.gif',
];

export function isSupportedFile(path: string): boolean {
  const ext = path.toLowerCase().replace(/.*[.](\w+)$/, '.$1');
  return SUPPORTED_EXTENSIONS.includes(ext);
}

export function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}
