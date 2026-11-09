import { intlTag, type Locale } from "../i18n/locale";

const DAY = 24 * 60 * 60 * 1000;
const startOfDay = (date: Date) =>
  new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();

/** Whole calendar days from `now` to `date` in the Mac's time zone (DST-safe). */
export function dayOffset(date: Date, now = new Date()) {
  return Math.round((startOfDay(date) - startOfDay(now)) / DAY);
}

/** "Today", "Tomorrow", "Yesterday", else a full date such as "Friday 13 November". */
export function dayLabel(locale: Locale, date: Date, now = new Date()) {
  const offset = dayOffset(date, now);
  if (Math.abs(offset) <= 1) {
    const text = new Intl.RelativeTimeFormat(intlTag[locale], {
      numeric: "auto",
    }).format(offset, "day");
    return text.charAt(0).toLocaleUpperCase(intlTag[locale]) + text.slice(1);
  }
  const text = new Intl.DateTimeFormat(intlTag[locale], {
    weekday: "long",
    day: "numeric",
    month: "long",
    ...(date.getFullYear() === now.getFullYear() ? {} : { year: "numeric" }),
  }).format(date);
  return text.charAt(0).toLocaleUpperCase(intlTag[locale]) + text.slice(1);
}

export function timeLabel(locale: Locale, date: Date) {
  return new Intl.DateTimeFormat(intlTag[locale], {
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

/** "5 minutes ago" / « il y a 5 minutes »; null under a minute ("just now"). */
export function ago(locale: Locale, seconds: number, now = Date.now()) {
  const elapsed = Math.max(0, Math.floor(now / 1000) - seconds);
  if (elapsed < 60) return null;
  const format = new Intl.RelativeTimeFormat(intlTag[locale], {
    numeric: "always",
  });
  if (elapsed < 3600) return format.format(-Math.floor(elapsed / 60), "minute");
  if (elapsed < 86400)
    return format.format(-Math.floor(elapsed / 3600), "hour");
  return format.format(-Math.floor(elapsed / 86400), "day");
}

/** Consecutive items sharing a calendar day, in their existing order. */
export function groupByDay<T>(
  items: readonly T[],
  seconds: (item: T) => number,
  now = new Date(),
) {
  const groups: { offset: number; date: Date; items: T[] }[] = [];
  for (const item of items) {
    const date = new Date(seconds(item) * 1000);
    const offset = dayOffset(date, now);
    const last = groups[groups.length - 1];
    if (last && last.offset === offset) last.items.push(item);
    else groups.push({ offset, date, items: [item] });
  }
  return groups;
}

/** The Mac's IANA time zone, e.g. "Europe/Paris". */
export function timeZone() {
  return Intl.DateTimeFormat().resolvedOptions().timeZone;
}
