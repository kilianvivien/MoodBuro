import { describe, expect, it } from "vitest";
import { formatDate, plural, resolveLocale } from "./locale";
import { describeError, knownErrors } from "./errors";
import { catalogues, type Messages } from "./messages";

describe("resolveLocale", () => {
  it("honours an explicit choice whatever the system says", () => {
    expect(resolveLocale("fr", ["en-US"])).toBe("fr");
    expect(resolveLocale("en", ["fr-FR"])).toBe("en");
  });

  it("follows the system's first supported language", () => {
    expect(resolveLocale("system", ["fr-FR", "en-US"])).toBe("fr");
    expect(resolveLocale("system", ["fr"])).toBe("fr");
    expect(resolveLocale("system", ["fr_CA"])).toBe("fr");
    expect(resolveLocale("system", ["de-DE", "fr-FR"])).toBe("fr");
    expect(resolveLocale("system", ["en-GB", "fr-FR"])).toBe("en");
  });

  it("falls back to English", () => {
    expect(resolveLocale("system", ["de-DE"])).toBe("en");
    expect(resolveLocale("system", [])).toBe("en");
  });
});

describe("formatting", () => {
  it("formats dates in the UI language", () => {
    const date = new Date(2026, 10, 9);
    expect(formatDate("fr", date, { dateStyle: "long" })).toBe(
      "9 novembre 2026",
    );
    expect(formatDate("en", date, { dateStyle: "long" })).toBe(
      "9 November 2026",
    );
  });

  it("uses each language's plural rules", () => {
    expect(plural("fr", 0)).toBe("one");
    expect(plural("en", 0)).toBe("other");
    expect(catalogues.fr.settings.siteCount(0)).toBe("0 site");
    expect(catalogues.en.settings.siteCount(0)).toBe("0 sites");
    expect(catalogues.fr.settings.siteCount(2)).toBe("2 sites");
    expect(catalogues.en.settings.siteCount(1)).toBe("1 site");
  });
});

/** Every leaf string, calling message functions with a sample argument. */
function strings(node: unknown): string[] {
  if (typeof node === "string") return [node];
  if (typeof node === "function") return [String(node(2))];
  return Object.values(node as object).flatMap(strings);
}

describe("catalogues", () => {
  it("translate every message", () => {
    const en = strings(catalogues.en);
    const fr = strings(catalogues.fr);
    expect(fr).toHaveLength(en.length);
    const untranslated = en.filter(
      (text, i) =>
        text === fr[i] &&
        // Identical in both languages on purpose.
        ![
          "Navigation",
          "Organisations",
          "Organisation",
          "Notifications",
          "Agenda",
          "Images",
          "Site",
          "Français",
          "English",
          "Sources",
          "Conversations",
        ].includes(text) &&
        !/^\d+ sites?$/u.test(text),
    );
    expect(untranslated).toEqual([]);
  });

  it("name no specific organisation", () => {
    for (const locale of ["en", "fr"] as const)
      for (const text of strings(catalogues[locale]))
        expect(text).not.toMatch(/\bINSP\b/iu);
  });

  it("use non-breaking spaces before French high punctuation", () => {
    const fr: Messages = catalogues.fr;
    expect(fr.settings.removeTitle("Exemple")).toBe("Retirer Exemple ?");
    for (const text of strings(fr)) expect(text).not.toMatch(/ [?:!;]/u);
  });
});

describe("describeError", () => {
  it("keeps backend English as-is in English", () => {
    expect(describeError("en", "Sign-in cancelled.")).toEqual({
      text: "Sign-in cancelled.",
      detail: null,
    });
  });

  it("translates known backend errors into French", () => {
    expect(describeError("fr", "Sign-in cancelled.").text).toBe(
      "Connexion annulée.",
    );
    expect(
      describeError(
        "fr",
        "Your Moodle session has expired. Reconnect this organisation in Settings.",
      ),
    ).toEqual({
      text: "Votre session Moodle a expiré. Reconnectez cette organisation dans les Réglages.",
      detail: null,
    });
    // Apostrophe style and stray whitespace do not defeat the match.
    expect(
      describeError(
        "fr",
        "  Could not remove this organisation's credentials. Please try again. ",
      ).detail,
    ).toBeNull();
  });

  it("translates every known error", () => {
    for (const english of knownErrors()) {
      const { text, detail } = describeError("fr", english);
      expect(detail).toBeNull();
      expect(text).not.toBe(english);
    }
  });

  it("says what failed in French and keeps unknown wording as detail", () => {
    expect(describeError("fr", "keyring: backend exploded", "add")).toEqual({
      text: "Impossible de connecter cette organisation.",
      detail: "keyring: backend exploded",
    });
    expect(describeError("fr", "Boom").text).toBe(
      "Une erreur inattendue s’est produite.",
    );
  });
});
