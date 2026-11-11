import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { DownloadList } from "./Downloads";
import { AddSite } from "./AddSite";
import { Settings } from "./Settings";
import { idle, type Connection } from "../lib/connection";
import type { Site } from "../ipc";
import { I18nProvider } from "../i18n/context";
import { Toolbar } from "../components/Toolbar";
import { PopupBar } from "../components/PopupBar";
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
      globalShortcut={false}
      globalShortcutSaving={false}
      shortcutUnavailable={false}
      files={{ pdf: "inline", images: "inline" }}
      filesSaving={{ pdf: false, images: false }}
      settingsError={null}
      settingsErrorIn="general"
      connection={idle}
      accountMessage={null}
      onAppearance={vi.fn()}
      onLanguage={vi.fn()}
      onGlobalShortcut={vi.fn()}
      onFileHandling={vi.fn()}
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

  it("offers per-type file handling, with viewing in Moodle as the default", () => {
    const html = settings({ files: { pdf: "open", images: "inline" } });
    expect(html).toContain('name="handling-pdf" checked="" value="open"');
    expect(html).toContain('name="handling-images" checked="" value="inline"');
    expect(html).toContain("Save and Open");
    expect(settings({}, "fr")).toContain("Enregistrer et ouvrir");
  });

  it("keeps the global shortcut off by default and reports a failed registration", () => {
    const html = settings();
    expect(html).toContain('role="switch"');
    expect(html).not.toMatch(/role="switch"[^>]*checked=""/);
    expect(settings({ globalShortcut: true })).toMatch(
      /role="switch"[^>]*checked=""/,
    );
    expect(html).not.toContain("could not be registered");
    expect(
      settings({ globalShortcut: true, shortcutUnavailable: true }),
    ).toContain("could not be registered when MoodBuro started");
    // Off again: nothing to warn about.
    expect(
      settings({ globalShortcut: false, shortcutUnavailable: true }),
    ).not.toContain("could not be registered");
  });

  it("reports a failed save under the group it concerns", () => {
    const error = "Could not save settings.";
    const general = settings({ settingsError: error });
    const files = settings({ settingsError: error, settingsErrorIn: "files" });
    const before = (html: string) =>
      html.indexOf(error) < html.indexOf('id="files-heading"');
    expect(before(general)).toBe(true);
    expect(before(files)).toBe(false);
    expect(files.split(error)).toHaveLength(2);
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

it("uses a custom short label while retaining the full accessible site name", () => {
  const html = inLocale(
    "fr",
    <Sidebar
      sites={[{ ...site, shortLabel: "TEST" }]}
      active="a"
      view="site"
      onSite={vi.fn()}
      onAdd={vi.fn()}
      onSettings={vi.fn()}
    />,
  );
  expect(html).toContain('aria-hidden="true">TEST</span>');
  expect(html).toContain('aria-label="Example University"');
});

it("labels unread counts and reconnection state in both languages", () => {
  const render = (locale: Locale, connection: "online" | "reauth") =>
    inLocale(
      locale,
      <Sidebar
        sites={[site]}
        active="a"
        view="site"
        unread={[{ siteId: "a", unread: 123, connection }]}
        onSite={vi.fn()}
        onAdd={vi.fn()}
        onSettings={vi.fn()}
      />,
    );
  expect(render("en", "online")).toContain("123 unread");
  expect(render("en", "online")).toContain("99+");
  expect(render("fr", "reauth")).toContain("Reconnecter dans Réglages");
  expect(render("fr", "reauth")).toContain("123 non lus");
  expect(render("fr", "reauth")).toContain('data-warning="true"');
});

it("prefills an unknown organisation link without starting sign-in", () => {
  const submit = vi.fn();
  const html = inLocale(
    "en",
    <AddSite
      initialUrl="https://new.example.edu/"
      connection={idle}
      firstRun
      onSubmit={submit}
    />,
  );
  expect(html).toContain('value="https://new.example.edu/"');
  expect(submit).not.toHaveBeenCalled();
});

it("blocks incomplete downloads and shows explicit risky-file confirmation", () => {
  const props = {
    sites: [site],
    busy: false,
    onOpen: vi.fn(),
    onReveal: vi.fn(),
    onCancel: vi.fn(),
    onConfirm: vi.fn(),
  };
  const items = [
    {
      id: "one",
      siteId: site.id,
      filename: "installer.dmg",
      state: "complete" as const,
      startedAt: 1,
      needsConfirmation: true,
    },
  ];
  const normal = inLocale(
    "en",
    <DownloadList {...props} items={items} confirming={null} />,
  );
  expect(normal).toContain('aria-label="Open installer.dmg"');
  expect(normal).not.toContain("alertdialog");
  const confirmed = inLocale(
    "fr",
    <DownloadList {...props} items={items} confirming="one" />,
  );
  expect(confirmed).toContain('role="alertdialog"');
  expect(confirmed).toContain("Ouvrir le fichier");
  const pending = inLocale(
    "en",
    <DownloadList
      {...props}
      items={[{ ...items[0], state: "downloading" }]}
      confirming={null}
    />,
  );
  expect(pending.match(/disabled=""/g)).toHaveLength(2);
  // Copy Path follows the same availability, and the path itself never renders.
  const copyable = inLocale(
    "en",
    <DownloadList
      {...props}
      items={[{ ...items[0], state: "downloading" }]}
      confirming={null}
      onCopy={vi.fn()}
    />,
  );
  expect(copyable).toContain('aria-label="Copy the path of installer.dmg"');
  expect(copyable.match(/disabled=""/g)).toHaveLength(3);
  expect(
    inLocale(
      "fr",
      <DownloadList {...props} items={[]} confirming={null} filtered />,
    ),
  ).toContain("Aucun téléchargement ne correspond");
});

it("provides a Home button and a popup escape path independent of PDF JavaScript", () => {
  for (const locale of ["en", "fr"] as const) {
    const html = inLocale(
      locale,
      <Toolbar
        view="site"
        site={site}
        error={null}
        canLeave
        onNav={vi.fn()}
        onDismissError={vi.fn()}
        onLeave={vi.fn()}
      />,
    );
    expect(html).toContain(
      locale === "fr"
        ? 'aria-label="Accueil Moodle"'
        : 'aria-label="Moodle Home"',
    );
  }
  const popup = renderToStaticMarkup(<PopupBar />);
  expect(popup).toMatch(/Back to Moodle|Retour à Moodle/);
  expect(popup).toContain('disabled=""');
});

it("offers native previews separately from opening and labels unmatched courses", () => {
  const props = {
    sites: [site],
    busy: false,
    confirming: null,
    onOpen: vi.fn(),
    onReveal: vi.fn(),
    onCancel: vi.fn(),
    onConfirm: vi.fn(),
    onPreview: vi.fn(),
  };
  const item = {
    id: "office",
    siteId: site.id,
    filename: "lesson.pptx",
    state: "complete" as const,
    startedAt: 1,
    needsConfirmation: false,
    canPreview: true,
    courseName: "Public law",
  };
  const available = inLocale("en", <DownloadList {...props} items={[item]} />);
  expect(available).toContain('aria-label="Preview lesson.pptx"');
  expect(available).toContain('aria-label="Open lesson.pptx"');
  expect(available).toContain("Public law");
  const unavailable = inLocale(
    "fr",
    <DownloadList
      {...props}
      items={[{ ...item, state: "downloading", courseName: undefined }]}
    />,
  );
  expect(unavailable).toContain("Cours inconnu");
  expect(unavailable.match(/disabled=""/g)).toHaveLength(3);
});

it("offers failed downloads a return route without enabling file opening", () => {
  const props = {
    sites: [site],
    busy: false,
    confirming: null,
    onOpen: vi.fn(),
    onReveal: vi.fn(),
    onCancel: vi.fn(),
    onConfirm: vi.fn(),
    onReturn: vi.fn(),
  };
  const item = {
    id: "failed",
    siteId: site.id,
    filename: "notes.pdf",
    state: "failed" as const,
    startedAt: 1,
    needsConfirmation: false,
  };
  const html = inLocale("en", <DownloadList {...props} items={[item]} />);
  expect(html).toContain("Return to Moodle");
  expect(html).toContain(
    "To retry, return to Moodle and start a new download.",
  );
  expect(html).toMatch(/aria-label="Open notes.pdf" disabled=""/);
  expect(inLocale("fr", <DownloadList {...props} items={[item]} />)).toContain(
    "Retour à Moodle",
  );
  expect(
    inLocale(
      "en",
      <DownloadList {...props} items={[{ ...item, state: "complete" }]} />,
    ),
  ).not.toContain("Return to Moodle");
});

it("keeps transfer feedback visible over Moodle and translates its outcome", () => {
  const base = {
    view: "site" as const,
    site,
    error: null,
    canLeave: true,
    onNav: vi.fn(),
    onDismissError: vi.fn(),
    onLeave: vi.fn(),
    onDownloads: vi.fn(),
    onDismissDownload: vi.fn(),
  };
  for (const state of ["downloading", "complete", "failed"] as const) {
    const html = inLocale(
      "fr",
      <Toolbar
        {...base}
        downloadFeedback={{
          siteId: site.id,
          userId: site.userId,
          filename: "lesson.pdf",
          state,
        }}
      />,
    );
    expect(html).toContain('role="status"');
    expect(html).toContain(`data-state="${state}"`);
    expect(html).toContain("lesson.pdf");
    expect(html).toContain('class="download-feedback-link"');
    expect(html).not.toContain("/Users/");
  }
  const complete = inLocale(
    "fr",
    <Toolbar
      {...base}
      downloadFeedback={{
        siteId: site.id,
        userId: site.userId,
        filename: "lesson.pdf",
        state: "complete",
      }}
    />,
  );
  expect(complete).toContain("Téléchargé");
});

it("groups Settings into five accessible panels with General selected", () => {
  const html = settings();
  expect(html.match(/role="tab"/g)).toHaveLength(5);
  expect(html).toContain(
    'id="settings-general-tab" role="tab" aria-selected="true"',
  );
  expect(html).toContain('id="settings-files-panel" role="tabpanel" hidden=""');
  expect(html).toContain(
    'id="settings-assistant-panel" role="tabpanel" hidden=""',
  );
});
