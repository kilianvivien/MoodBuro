/** Two-letter monogram for a site avatar: word initials, else the first letters. */
export function initials(name: string) {
  const words = name
    .trim()
    .split(/[\s\-_.·]+/u)
    .filter(Boolean);
  if (words.length === 0) return "?";
  const letters =
    words.length > 1
      ? words[0][0] + words[1][0]
      : Array.from(words[0]).slice(0, 2).join("");
  return letters.toUpperCase();
}

/** Host plus any subdirectory, without scheme or trailing slash. */
export function displayHost(wwwroot: string) {
  try {
    const url = new URL(wwwroot);
    return (url.host + url.pathname).replace(/\/+$/u, "");
  } catch {
    return wwwroot;
  }
}
