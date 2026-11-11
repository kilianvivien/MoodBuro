import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  ResourceFiles,
  SavedFiles,
  RemoveCopyConfirmation,
  CourseSyncControls,
} from "./CourseFiles";
import { I18nProvider } from "../i18n/context";
import { type ReactNode } from "react";
import { type Locale } from "../i18n/locale";
import type { CourseSync, OfflineCopy, OfflineResource } from "../ipc";
const render = (node: ReactNode, locale: Locale = "en") =>
  renderToStaticMarkup(<I18nProvider locale={locale}>{node}</I18nProvider>);
const resource: OfflineResource = {
  key: "a".repeat(64),
  moduleId: 42,
  sectionId: 1,
  sectionName: "Week 1",
  activityName: "Lecture",
  filename: "notes.pdf",
  bytes: null,
};
const copy: OfflineCopy = {
  id: "copy",
  siteId: "s",
  courseId: 2,
  resourceKey: resource.key,
  moduleId: 42,
  filename: "notes.pdf",
  sectionName: "Week 1",
  bytes: 128,
  savedAt: 1792888200,
  refreshedAt: 1792888200,
  available: true,
  needsConfirmation: false,
};
describe("offline course files", () => {
  it("requires explicit selection, states unknown sizes and disables oversized files", () => {
    const html = render(
      <ResourceFiles
        items={[
          resource,
          {
            ...resource,
            key: "large",
            filename: "video.mp4",
            bytes: 129 * 1024 * 1024,
          },
        ]}
        selected={[resource.key]}
        saved={[]}
        busy={false}
        onToggle={vi.fn()}
      />,
    );
    expect(html).toContain("Size unknown");
    expect(html).toContain('checked=""');
    expect(html).toContain('disabled=""');
    expect(html).toContain("Exceeds the 128 MB limit");
    expect(html).not.toContain("token");
  });
  it("offers offline Open and Finder separately from network refresh", () => {
    const html = render(
      <SavedFiles
        items={[copy]}
        busy={false}
        onOpen={vi.fn()}
        onRefresh={vi.fn()}
        onRemove={vi.fn()}
      />,
    );
    expect(html).toContain('aria-label="Open notes.pdf"');
    expect(html).toContain('aria-label="Show notes.pdf in Finder"');
    expect(html).toContain('aria-label="Refresh offline copy of notes.pdf"');
    expect(html).not.toContain('disabled=""');
  });
  it("keeps missing copies listed while allowing a fresh copy to be requested", () => {
    const html = render(
      <SavedFiles
        items={[{ ...copy, available: false }]}
        busy={false}
        onOpen={vi.fn()}
        onRefresh={vi.fn()}
        onRemove={vi.fn()}
      />,
    );
    expect(html.match(/disabled=""/g)).toHaveLength(2);
    expect(html).toContain("moved or is unavailable");
    const french = render(
      <SavedFiles
        items={[]}
        busy={false}
        onOpen={vi.fn()}
        onRefresh={vi.fn()}
        onRemove={vi.fn()}
      />,
      "fr",
    );
    expect(french).toContain("Aucune copie enregistrée");
  });
  it("distinguishes saved, missing and unsaved resources in both languages", () => {
    for (const locale of ["en", "fr"] as const) {
      const html = render(
        <ResourceFiles
          items={[
            resource,
            { ...resource, key: "missing" },
            { ...resource, key: "new" },
          ]}
          saved={[
            copy,
            {
              ...copy,
              id: "missing",
              resourceKey: "missing",
              available: false,
            },
          ]}
          selected={[]}
          busy={false}
          onToggle={vi.fn()}
        />,
        locale,
      );
      expect(html).toContain(
        locale === "en" ? "Available offline" : "Disponible hors connexion",
      );
      expect(html).toContain(
        locale === "en"
          ? "Saved copy unavailable"
          : "Copie enregistrée indisponible",
      );
      expect(html).toContain(
        locale === "en" ? "Not saved on this Mac" : "Non enregistré sur ce Mac",
      );
    }
  });
  it("keeps saved copies openable after refresh failure and labels the successful refresh", () => {
    for (const locale of ["en", "fr"] as const) {
      const html = render(
        <SavedFiles
          items={[copy]}
          busy={false}
          failedKeys={[copy.resourceKey]}
          onOpen={vi.fn()}
          onRefresh={vi.fn()}
          onRemove={vi.fn()}
        />,
        locale,
      );
      expect(html).toContain(
        locale === "en"
          ? "Last successful refresh"
          : "Dernière actualisation réussie",
      );
      expect(html).toContain(
        locale === "en"
          ? "You can still open this saved copy"
          : "Cette copie reste consultable",
      );
      expect(html).not.toContain('disabled=""');
    }
  });
  it("asks inline before removal, naming the file and organisation and explaining recovery", () => {
    for (const locale of ["en", "fr"] as const) {
      const html = render(
        <RemoveCopyConfirmation
          copy={copy}
          siteName="Example Campus"
          busy={false}
          onCancel={vi.fn()}
          onConfirm={vi.fn()}
        />,
        locale,
      );
      expect(html).toContain('role="alertdialog"');
      expect(html).toContain("notes.pdf");
      expect(html).toContain("Example Campus");
      expect(html).toContain("~/Moodle/.trash");
      expect(html).toContain(
        locale === "en"
          ? "locally edited file stays"
          : "fichier modifié localement reste",
      );
      expect(html).toContain(locale === "en" ? "Cancel" : "Annuler");
      const busy = render(
        <RemoveCopyConfirmation
          copy={copy}
          siteName="Example Campus"
          busy={true}
          onCancel={vi.fn()}
          onConfirm={vi.fn()}
        />,
        locale,
      );
      expect(busy.match(/disabled=""/g)).toHaveLength(2);
    }
  });
  it("requires a selected set before enabling sync and exposes pause, cancellation and failure recovery", () => {
    const job: CourseSync = {
      siteId: "s",
      userId: 7,
      courseId: 2,
      generation: 1,
      interval: 3600,
      state: "running",
      nextRun: 1792888200,
      refreshedAt: 0,
      failures: 0,
      error: null,
      keys: [resource.key],
      pending: 1,
      missing: 0,
    };
    for (const locale of ["en", "fr"] as const) {
      const off = render(
        <CourseSyncControls
          job={null}
          busy={false}
          selected={[]}
          onConfigure={vi.fn()}
          onControl={vi.fn()}
        />,
        locale,
      );
      expect(off).toContain('disabled=""');
      expect(off).toContain(
        locale === "en"
          ? "only while MoodBuro is open"
          : "uniquement lorsque MoodBuro est ouvert",
      );
      const running = render(
        <CourseSyncControls
          job={job}
          busy={false}
          selected={[]}
          onConfigure={vi.fn()}
          onControl={vi.fn()}
        />,
        locale,
      );
      expect(running).toContain(
        locale === "en"
          ? "Cancel Current Refresh"
          : "Annuler l’actualisation en cours",
      );
      const failed = render(
        <CourseSyncControls
          job={{
            ...job,
            state: "error",
            missing: 1,
            error: "Could not download this resource.",
          }}
          busy={false}
          selected={[resource.key]}
          onConfigure={vi.fn()}
          onControl={vi.fn()}
        />,
        locale,
      );
      expect(failed).toContain('role="alert"');
      expect(failed).toContain(
        locale === "en"
          ? "Saved copies are kept"
          : "Les copies sont conservées",
      );
      expect(failed).toContain(
        locale === "en" ? "Refresh Now" : "Actualiser maintenant",
      );
    }
  });
});
