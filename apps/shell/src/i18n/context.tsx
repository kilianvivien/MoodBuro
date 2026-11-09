import { createContext, useContext, useMemo, type ReactNode } from "react";
import { describeError, type ErrorContext } from "./errors";
import { formatDate, intlTag, type Locale } from "./locale";
import { catalogues } from "./messages";

function translator(locale: Locale) {
  return {
    locale,
    lang: intlTag[locale],
    t: catalogues[locale],
    error: (raw: string, context?: ErrorContext) =>
      describeError(locale, raw, context),
    date: (date: Date | number, options?: Intl.DateTimeFormatOptions) =>
      formatDate(locale, date, options),
  };
}

export type I18n = ReturnType<typeof translator>;

const I18nContext = createContext<I18n>(translator("en"));

export function I18nProvider({
  locale,
  children,
}: {
  locale: Locale;
  children: ReactNode;
}) {
  const value = useMemo(() => translator(locale), [locale]);
  return <I18nContext value={value}>{children}</I18nContext>;
}

export function useI18n() {
  return useContext(I18nContext);
}
