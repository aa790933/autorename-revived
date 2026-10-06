/**
 * Shared utilities
 */

// Replaced by Vite at build time
export const __APP_VERSION__ = '__APP_VERSION__';

export const SUPPORTED_EXTENSIONS = [
  '.pdf', '.docx', '.doc', '.xlsx', '.xls', '.pptx', '.ppt', '.pptm',
  '.csv', '.txt', '.md', '.rtf', '.html', '.htm', '.json', '.xml',
  '.png', '.jpg', '.jpeg', '.webp', '.tiff', '.tif', '.bmp', '.gif',
] as const;

export type SupportedExtension = typeof SUPPORTED_EXTENSIONS[number];

/**
 * Get the file extension including the dot, lowercased
 */
export function extensionOf(path: string): string {
  const fileName = path.split(/[\\/]/).pop() ?? '';
  const dot = fileName.lastIndexOf('.');
  if (dot <= 0) return '';
  return fileName.slice(dot).toLowerCase();
}

/**
 * Check if a file is supported
 */
export function isSupportedFile(path: string): boolean {
  return SUPPORTED_EXTENSIONS.includes(extensionOf(path) as SupportedExtension);
}

/**
 * Escape HTML to prevent XSS
 */
export function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#x27;');
}

/**
 * Format bytes to human readable string
 */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

/**
 * Get file name from path
 */
export function getFileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/**
 * Debounce function
 */
export function debounce<T extends (...args: unknown[]) => unknown>(
  func: T,
  wait: number
): (...args: Parameters<T>) => void {
  let timeout: ReturnType<typeof setTimeout> | null = null;
  return (...args: Parameters<T>) => {
    if (timeout) clearTimeout(timeout);
    timeout = setTimeout(() => func(...args), wait);
  };
}

/**
 * Generate unique ID
 */
export function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

/**
 * Sleep/delay function
 */
export function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Capitalize first letter
 */
export function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1);
}

/**
 * Truncate string with ellipsis
 */
export function truncate(str: string, maxLength: number): string {
  if (str.length <= maxLength) return str;
  return str.slice(0, maxLength - 3) + '...';
}

/**
 * Class name utility for conditional classes
 */
export function classNames(...classes: (string | boolean | undefined | null)[]): string {
  return classes.filter(Boolean).join(' ');
}