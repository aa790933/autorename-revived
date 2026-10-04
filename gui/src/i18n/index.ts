/**
 * i18n configuration for AutoRename-Revived v4.0.0
 * Supports English, French, and Arabic with RTL support
 */
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';

import en from './locales/en.json';
import fr from './locales/fr.json';
import ar from './locales/ar.json';

const resources = {
  en: { translation: en },
  fr: { translation: fr },
  ar: { translation: ar },
};

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: 'en',
    supportedLngs: ['en', 'fr', 'ar'],
    nonExplicitSupportedLngs: true,
    debug: false,
    interpolation: {
      escapeValue: false,
    },
    react: {
      useSuspense: false,
    },
    detection: {
      order: ['localStorage', 'navigator'],
      caches: ['localStorage'],
    },
  });

export const SUPPORTED_LANGUAGES = [
  { code: 'en', name: 'English', nativeName: 'English', rtl: false },
  { code: 'fr', name: 'French', nativeName: 'Français', rtl: false },
  { code: 'ar', name: 'Arabic', nativeName: 'العربية', rtl: true },
] as const;

export type SupportedLanguage = typeof SUPPORTED_LANGUAGES[number]['code'];

export const getLanguageDirection = (lng: SupportedLanguage): 'ltr' | 'rtl' => {
  const lang = SUPPORTED_LANGUAGES.find(l => l.code === lng);
  return lang?.rtl ? 'rtl' : 'ltr';
};

export default i18n;