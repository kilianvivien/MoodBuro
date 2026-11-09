import type { Language } from "../ipc";

/** Languages the shell is translated into. */
export type Locale = "en" | "fr";

/** The browser/WebView's preferred languages, most preferred first. */
export function systemLanguages(): readonly string[] {
  if (typeof navigator === "undefined") return [];
  if (navigator.languages?.length) return navigator.languages;
  return navigator.language ? [navigator.language] : [];
}

/**
 * The saved preference wins; Automatic picks the first supported language in
 * the system's preference order (so `de-DE, fr-FR` gives French), else English.
 */
export function resolveLocale(
  language: Language,
  preferred: readonly string[] = systemLanguages(),
): Locale {
  if (language === "fr" || language === "en") return language;
  for (const tag of preferred) {
    const primary = tag.toLowerCase().split(/[-_]/u)[0];
    if (primary === "fr" || primary === "en") return primary;
  }
  return "en";
}

/** BCP 47 tag used for Intl formatting and `<html lang>`. */
export const intlTag: Record<Locale, string> = { en: "en-GB", fr: "fr-FR" };

export function formatDate(
  locale: Locale,
  date: Date | number,
  options: Intl.DateTimeFormatOptions = { dateStyle: "medium" },
) {
  return new Intl.DateTimeFormat(intlTag[locale], options).format(date);
}

export function formatNumber(locale: Locale, value: number) {
  return new Intl.NumberFormat(intlTag[locale]).format(value);
}

/** Intl plural category, e.g. French treats 0 as singular ("0 site"). */
export function plural(locale: Locale, count: number) {
  return new Intl.PluralRules(intlTag[locale]).select(count);
}
