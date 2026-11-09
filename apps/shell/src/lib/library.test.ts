import { describe, expect, it } from "vitest";
import { fold, rank, search } from "./search";
import { ago, dayLabel, dayOffset, groupByDay, timeLabel } from "./time";

describe("search", () => {
  it("ignores case and accents", () => {
    expect(fold("Économie Générale")).toBe("economie generale");
    expect(rank("eco", ["Économie"])).toBe(0);
  });

  it("requires every word and prefers starts of names and words", () => {
    expect(rank("law public", ["Comparative Public Law"])).not.toBeNull();
    expect(rank("law physics", ["Comparative Public Law"])).toBeNull();
    const names = ["Applied Statistics", "Statistics 2", "Biostatistics"];
    expect(search(names, "stat", (name) => [name])).toEqual([
      "Statistics 2",
      "Applied Statistics",
      "Biostatistics",
    ]);
  });

  it("returns everything, in order, for an empty query", () => {
    expect(search(["b", "a"], "  ", (name) => [name])).toEqual(["b", "a"]);
  });

  it("ranks a title match above a subtitle match", () => {
    const items = [
      { title: "Workshop", subtitle: "Law faculty" },
      { title: "Law", subtitle: "" },
    ];
    expect(
      search(items, "law", (item) => [item.title, item.subtitle])[0].title,
    ).toBe("Law");
  });
});

describe("time", () => {
  const now = new Date(2026, 10, 9, 12, 0, 0);
  const seconds = (date: Date) => Math.floor(date.getTime() / 1000);

  it("counts calendar days, not 24-hour periods", () => {
    expect(dayOffset(new Date(2026, 10, 9, 23, 59), now)).toBe(0);
    expect(dayOffset(new Date(2026, 10, 10, 0, 1), now)).toBe(1);
    expect(dayOffset(new Date(2026, 10, 8, 23, 59), now)).toBe(-1);
    // Across the autumn clock change, a day is still one day.
    expect(
      dayOffset(new Date(2026, 9, 26, 9), new Date(2026, 9, 24, 23, 30)),
    ).toBe(2);
  });

  it("labels days in both languages", () => {
    expect(dayLabel("en", now, now)).toBe("Today");
    expect(dayLabel("fr", new Date(2026, 10, 10, 8), now)).toBe("Demain");
    expect(dayLabel("fr", new Date(2026, 10, 8, 8), now)).toBe("Hier");
    expect(dayLabel("en", new Date(2026, 10, 13, 8), now)).toBe(
      "Friday 13 November",
    );
    expect(dayLabel("fr", new Date(2027, 0, 4, 8), now)).toBe(
      "Lundi 4 janvier 2027",
    );
    expect(timeLabel("fr", new Date(2026, 10, 9, 9, 5))).toBe("09:05");
  });

  it("describes how long ago a refresh happened", () => {
    const at = seconds(now);
    expect(ago("en", at - 30, now.getTime())).toBeNull();
    expect(ago("en", at - 5 * 60, now.getTime())).toBe("5 minutes ago");
    expect(ago("fr", at - 2 * 3600, now.getTime())).toBe("il y a 2 heures");
    expect(ago("en", at - 3 * 86400, now.getTime())).toBe("3 days ago");
    // A clock that moved backwards never yields a future time.
    expect(ago("en", at + 600, now.getTime())).toBeNull();
  });

  it("groups consecutive items by calendar day", () => {
    const items = [
      new Date(2026, 10, 9, 8),
      new Date(2026, 10, 9, 18),
      new Date(2026, 10, 11, 9),
    ].map(seconds);
    const groups = groupByDay(items, (item) => item, now);
    expect(groups.map((group) => [group.offset, group.items.length])).toEqual([
      [0, 2],
      [2, 1],
    ]);
  });
});
