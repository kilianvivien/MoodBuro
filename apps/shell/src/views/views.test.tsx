import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { AddSite } from "./AddSite";
import { Settings } from "./Settings";
import { idle, type Connection } from "../lib/connection";
import type { Site } from "../ipc";
import { I18nProvider } from "../i18n/context";
import { Toolbar } from "../components/Toolbar";
import { Sidebar } from "../components/Sidebar";
import type { Locale } from "../i18n/locale";
import type { ReactNode } from "react";

const inLocale = (locale: Locale, node: ReactNode) =>
  renderToStaticMarkup(<I18nProvider locale={locale}>{node}</I18nProvider>);

const site: Site = {
  id: "a",
  wwwroot: "https://moodle.example.edu/",
  name: "Example University",
  userId: 2,
  fullName: "Ada Learner",
};

function settings(
  props: Partial<Parameters<typeof Settings>[0]> = {},
  locale: Locale = "en",
) {
  return inLocale(
    locale,
    <Settings
      desktop
      sites={[site]}
      appearance="system"
      appearanceSaving={false}
      language="system"
      languageSaving={false}
      settingsError={null}
      connection={idle}
      accountMessage={null}
      onAppearance={vi.fn()}
      onLanguage={vi.fn()}
      onOpenSite={vi.fn()}
      onReauthenticate={vi.fn()}
      onRemove={vi.fn()}
      onAdd={vi.fn()}
      {...props}
    />,
  );
}

describe("Settings", () => {
  it("lists connected organisations with management actions", () => {
    const html = settings();
    expect(html).toContain("Example University");
    expect(html).toContain("Ada Learner · moodle.example.edu");
    expect(html).toContain('aria-label="Sign in again to Example University"');
    expect(html).toContain('aria-label="Remove Example University"');
    // Removal always asks first: no confirmation until the user requests it.
    expect(html).not.toContain("alertdialog");
  });

  it("tells browser-preview users what does not persist", () => {
    expect(settings({ desktop: false })).toContain("Browser preview");
    expect(settings()).not.toContain("Browser preview");
  });

  it("shows re-authentication progress on the right row and blocks others", () => {
    const html = settings({ connection: { phase: "signing-in", siteId: "a" } });
    expect(html).toContain("Continue in the sign-in window");
    expect(html).toMatch(
      /<button[^>]*disabled=""[^>]*aria-label="Sign in again/,
    );
  });
});

it("shows discovery before the sign-in window for re-authentication", () => {
  const html = settings({ connection: { phase: "checking", siteId: "a" } });
  expect(html).toContain("Connecting to the site…");
});

describe("AddSite", () => {
  const render = (connection: Connection, locale: Locale = "en") =>
    inLocale(
      locale,
      <AddSite connection={connection} firstRun onSubmit={vi.fn()} />,
    );

  it("shows honest progress for each phase", () => {
    expect(render({ phase: "checking", siteId: null })).toContain(
      'Finding your Moodle site<span class="visually-hidden"> — in progress',
    );
    const signingIn = render({ phase: "signing-in", siteId: null });
    expect(signingIn).toContain(
      'Finding your Moodle site<span class="visually-hidden"> — done',
    );
    expect(signingIn).toContain("Close it to cancel");
  });

  it("surfaces discovery errors on the field", () => {
    const html = render({
      phase: "idle",
      last: { siteId: null, error: "Not a Moodle site." },
    });
    expect(html).toContain('aria-invalid="true"');
    expect(html).toContain("Not a Moodle site.");
  });
});

describe("French", () => {
  it("renders Settings in French, including accessible labels", () => {
    const html = settings({ language: "fr" }, "fr");
    expect(html).toContain("Général");
    expect(html).toContain("Thème");
    expect(html).toContain("Automatique");
    expect(html).toContain("Sombre");
    expect(html).toContain('<span lang="fr">Français</span>');
    expect(html).toContain('<span lang="en">English</span>');
    expect(html).toContain("1 site");
    expect(html).toContain('aria-label="Se reconnecter à Example University"');
    expect(html).toContain('aria-label="Retirer Example University"');
    expect(html).toContain("Ajouter une organisation…");
    // The selected language is the checked radio.
    expect(html).toMatch(/checked="" value="fr"/);
    for (const english of ["Sign In Again", "Remove", "Theme", "Open<"])
      expect(html).not.toContain(english);
  });

  it("translates progress and account notices", () => {
    expect(
      settings({ connection: { phase: "checking", siteId: "a" } }, "fr"),
    ).toContain("Connexion au site…");
    const failed = settings(
      {
        accountMessage: {
          tone: "error",
          error: "Sign-in cancelled.",
          context: "signIn",
        },
      },
      "fr",
    );
    expect(failed).toContain("Connexion annulée.");
    expect(failed).not.toContain("Sign-in cancelled.");
  });

  it("explains untranslated backend errors in French, keeping the original", () => {
    const html = settings({ settingsError: "disk I/O error" }, "fr");
    expect(html).toContain("Impossible d’enregistrer ce réglage.");
    expect(html).toContain('<span lang="en">disk I/O error</span>');
  });

  it("tells browser-preview users in French", () => {
    expect(settings({ desktop: false }, "fr")).toContain(
      "Aperçu dans le navigateur",
    );
  });

  it("renders onboarding progress in French", () => {
    const html = inLocale(
      "fr",
      <AddSite
        connection={{ phase: "signing-in", siteId: null }}
        firstRun
        onSubmit={vi.fn()}
      />,
    );
    expect(html).toContain("Bienvenue dans MoodBuro");
    expect(html).toContain("Adresse Moodle");
    expect(html).toContain(
      'Recherche de votre site Moodle<span class="visually-hidden"> — terminé',
    );
    expect(html).toContain("Fermez-la pour annuler");
    expect(html).toContain("trousseau");
  });

  it("translates the toolbar and sidebar, including labels", () => {
    const toolbar = inLocale(
      "fr",
      <Toolbar
        view="site"
        site={site}
        error="Navigation failed."
        canLeave
        onNav={vi.fn()}
        onDismissError={vi.fn()}
        onLeave={vi.fn()}
      />,
    );
    expect(toolbar).toContain('aria-label="Précédent"');
    expect(toolbar).toContain('aria-label="Recharger la page"');
    expect(toolbar).toContain('aria-label="Fermer le message"');
    expect(toolbar).toContain("La navigation a échoué.");
    const settingsBar = inLocale(
      "fr",
      <Toolbar
        view="settings"
        site={undefined}
        error={null}
        canLeave
        onNav={vi.fn()}
        onDismissError={vi.fn()}
        onLeave={vi.fn()}
      />,
    );
    expect(settingsBar).toContain("Réglages");
    expect(settingsBar).toContain("Terminé");
    const sidebar = inLocale(
      "fr",
      <Sidebar
        sites={[site]}
        active="a"
        view="site"
        onSite={vi.fn()}
        onAdd={vi.fn()}
        onSettings={vi.fn()}
      />,
    );
    expect(sidebar).toContain('aria-label="Sites Moodle"');
    expect(sidebar).toContain('aria-label="Ajouter une organisation"');
    expect(sidebar).toContain('aria-label="Réglages"');
  });
});
