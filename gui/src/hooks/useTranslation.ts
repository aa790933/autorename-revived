/**
 * Translation hook wrapper
 */
import { useTranslation as useI18nTranslation } from 'react-i18next';
import { useAppStore } from '@/store';

/**
 * Custom translation hook that syncs with app store language
 */
export function useTranslation(namespace?: string) {
  const { language } = useAppStore();
  const { t, i18n } = useI18nTranslation(namespace);

  // Sync language with store (if changed externally)
  // react-i18next handles this automatically via LanguageDetector,

  return { t, i18n, currentLanguage: language };
}

/**
 * Get translation function without hook (for non-component usage)
 */
import i18n from '@/i18n';

export const t = i18n.t.bind(i18n);

/**
 * Change language programmatically
 */
export function changeLanguage(lng: string): Promise<unknown> {
  return i18n.changeLanguage(lng);
}

/**
 * Get current language
 */
export function getCurrentLanguage(): string {
  return i18n.language;
}

/**
 * Check if current language is RTL
 */
export function isRTL(): boolean {
  const lng = i18n.language;
  return lng === 'ar';
}