import { createI18n } from 'vue-i18n';
import zh from './locales/zh.json';
import en from './locales/en.json';

export const i18n = createI18n({
  legacy: false,
  locale: 'zh',
  fallbackLocale: 'en',
  messages: { zh, en },
});

// Persistence callback - set by main.js during initialization
let persistCallback = null;

export function setPersistCallback(callback) {
  persistCallback = callback;
}

export function setLocale(locale) {
  // Validate locale
  if (!['zh', 'en'].includes(locale)) {
    console.warn(`[i18n] Invalid locale: ${locale}, falling back to 'zh'`);
    locale = 'zh';
  }
  i18n.global.locale.value = locale;
  document.querySelector('html').setAttribute('lang', locale);
  // Persist if callback is set
  if (persistCallback) {
    persistCallback(locale);
  }
}

export function getLocale() {
  return i18n.global.locale.value;
}
