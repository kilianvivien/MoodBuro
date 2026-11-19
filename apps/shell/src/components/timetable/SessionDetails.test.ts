import { describe, expect, it } from "vitest";
import type {
  QuestionSession,
  Questions,
  TimetableChoices,
} from "../../lib/timetable";
import { sharedGroupChoice } from "./SessionDetails";

const group = (
  uid: string,
  date: string,
  repeatKey: string | null = "atelier",
): QuestionSession => ({
  uid,
  title: "Atelier",
  date,
  start: "09:00",
  end: "12:00",
  location: null,
  cohortLabel: null,
  module: null,
  options: ["Groupe 1", "Groupe 2"].map((label) => ({
    label,
    location: null,
    start: "09:00",
    end: "12:00",
  })),
  repeatKey,
});

const questions = (...groups: QuestionSession[]): Questions => ({
  groups,
  participation: [],
  halfPromotion: [],
  times: [],
  sport: [],
  lv1: [],
  lv1Rooms: [],
});

const choices = (groups: Record<string, string>): TimetableChoices => ({
  groups,
  times: {},
  rooms: {},
  sport: {},
  halfPromotion: {},
});

const week = questions(
  group("mon", "2026-11-16"),
  group("tue", "2026-11-17"),
  group("thu", "2026-11-19"),
  group("other", "2026-11-18", "seminaire"),
  group("next", "2026-11-23"),
);

describe("shared group choice", () => {
  it("offers an answer to the unanswered sessions of the same course and week", () => {
    expect(
      sharedGroupChoice(week, choices({ mon: "Groupe 2" }), "mon"),
    ).toEqual({ group: "Groupe 2", targets: ["tue", "thu"], others: true });
  });

  it("never overwrites an answer already given", () => {
    expect(
      sharedGroupChoice(
        week,
        choices({ mon: "Groupe 2", tue: "Groupe 1" }),
        "mon",
      ),
    ).toEqual({ group: "Groupe 2", targets: ["thu"], others: true });
  });

  it("proposes the single answer of the matching sessions to an unanswered one", () => {
    expect(
      sharedGroupChoice(week, choices({ mon: "Groupe 1" }), "thu"),
    ).toEqual({ group: "Groupe 1", targets: ["tue", "thu"], others: false });
    expect(
      sharedGroupChoice(
        week,
        choices({ mon: "Groupe 1", tue: "Groupe 2" }),
        "thu",
      ),
    ).toBeNull();
  });

  it("offers nothing for a whole session, an exclusion or a lone session", () => {
    expect(sharedGroupChoice(week, choices({ mon: "" }), "mon")).toBeNull();
    expect(
      sharedGroupChoice(week, choices({ mon: "__exclude__" }), "mon"),
    ).toBeNull();
    expect(
      sharedGroupChoice(week, choices({ other: "Groupe 1" }), "other"),
    ).toBeNull();
    expect(
      sharedGroupChoice(week, choices({ next: "Groupe 1" }), "next"),
    ).toBeNull();
    const rotation = questions(
      group("a", "2026-11-20", null),
      group("b", "2026-11-20", null),
    );
    expect(
      sharedGroupChoice(rotation, choices({ a: "Groupe 1" }), "a"),
    ).toBeNull();
  });

  it("leaves out sessions whose participation is asked", () => {
    const optional = { ...week, participation: [group("tue", "2026-11-17")] };
    expect(
      sharedGroupChoice(optional, choices({ mon: "Groupe 2" }), "mon"),
    ).toEqual({ group: "Groupe 2", targets: ["thu"], others: true });
  });
});
