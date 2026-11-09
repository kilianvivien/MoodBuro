/** Lower-case, accent-free form used to compare names typed without diacritics. */
export function fold(text: string) {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
}

/**
 * Local metadata search: every typed word must appear in one of the fields.
 * Returns a rank (lower is better) or null when the item does not match.
 * A word that starts the first field, or starts one of its words, ranks first.
 */
export function rank(query: string, fields: readonly string[]) {
  const words = fold(query).split(/\s+/u).filter(Boolean);
  if (words.length === 0) return 0;
  const folded = fields.map(fold);
  let score = 0;
  for (const word of words) {
    const best = Math.min(
      ...folded.map((field, index) => {
        const at = field.indexOf(word);
        if (at < 0) return Infinity;
        const boundary = at === 0 || /[\s\-_.·/(]/u.test(field[at - 1]);
        return (at === 0 ? 0 : boundary ? 1 : 2) + index * 3;
      }),
    );
    if (best === Infinity) return null;
    score += best;
  }
  return score;
}

/** Items matching `query`, best first; ties keep their original order. */
export function search<T>(
  items: readonly T[],
  query: string,
  fields: (item: T) => readonly string[],
) {
  return items
    .map((item, index) => ({ item, index, score: rank(query, fields(item)) }))
    .filter(
      (entry): entry is { item: T; index: number; score: number } =>
        entry.score !== null,
    )
    .sort((a, b) => a.score - b.score || a.index - b.index)
    .map((entry) => entry.item);
}
