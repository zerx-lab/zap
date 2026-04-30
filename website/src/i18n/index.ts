import { zhCN, type Dict } from './zh-CN';
import { en } from './en';

export const locales = ['zh-CN', 'en'] as const;
export type Locale = (typeof locales)[number];

export const defaultLocale: Locale = 'zh-CN';

const dicts: Record<Locale, Dict> = {
  'zh-CN': zhCN,
  en,
};

export function t(locale: Locale): Dict {
  return dicts[locale] ?? dicts[defaultLocale];
}

export function getLocaleFromUrl(url: URL): Locale {
  const seg = url.pathname.split('/').filter(Boolean)[0];
  if (seg === 'en') return 'en';
  return 'zh-CN';
}

export function localizedPath(locale: Locale, path = '/'): string {
  const clean = path.startsWith('/') ? path : `/${path}`;
  if (locale === defaultLocale) return clean === '/' ? '/' : clean;
  return `/${locale}${clean === '/' ? '' : clean}`;
}

export const localeLabels: Record<Locale, string> = {
  'zh-CN': '简体中文',
  en: 'English',
};
