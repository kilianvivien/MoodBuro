import { describe, expect, it } from "vitest";
import type { Session } from "../../lib/timetable";
import { addDays, layoutDay, minutes, mondayOf } from "./WeekCalendar";

const session = (
  uid: string,
  start: string | null,
  end: string | null,
): Session => ({
  uid,
  title: uid,
  module: null,
  location: null,
  locationNote: null,
  speakers: [],
  cohortLabel: null,
  preparation: null,
  notes: [],
  remote: false,
  date: "2026-11-12",
  lastDate: "2026-11-12",
  start,
  end,
  startAt: null,
  endAt: null,
  allDay: start === null,
  tentative: false,
  links: [],
  cancelled: false,
});

describe("week calendar layout", () => {
  it("places every timed session, sharing width only when starting together", () => {
    const placed = layoutDay([
      session("seminar", "09:30", "12:30"),
      session("groupA", "10:00", "11:15"),
      session("groupB", "10:00", "11:15"),
      session("point", "10:30", "11:00"),
      session("later", "11:30", "12:15"),
      session("allday", null, null),
    ]);
    const at = (uid: string) => placed.find((p) => p.session.uid === uid)!;
    expect(placed).toHaveLength(5);
    expect(at("seminar")).toMatchObject({ left: 0, width: 1, depth: 0 });
    // Same start: side by side, both indented over the seminar.
    expect(at("groupA").left).toBeCloseTo(0.16);
    expect(at("groupA").width).toBeCloseTo(0.42);
    expect(at("groupB").left).toBeCloseTo(0.58);
    // Later still: indented further and drawn above.
    expect(at("point").depth).toBe(2);
    expect(at("point").left).toBeCloseTo(0.32);
    // After the groups end, only the seminar is under it.
    expect(at("later").depth).toBe(1);
    for (const p of placed) {
      expect(p.left).toBeGreaterThanOrEqual(0);
      expect(p.left + p.width).toBeLessThanOrEqual(1.0001);
    }
  });

  it("works on school wall dates and times without time zones", () => {
    expect(minutes("09:45")).toBe(585);
    expect(minutes("9h45")).toBeNull();
    expect(mondayOf("2026-11-15")).toBe("2026-11-09");
    expect(mondayOf("2026-11-09")).toBe("2026-11-09");
    expect(addDays("2026-03-28", 2)).toBe("2026-03-30");
  });
});
