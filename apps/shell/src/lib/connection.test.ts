import { describe, expect, it } from "vitest";
import { connection, idle, lastError, type Connection } from "./connection";

const run = (...events: Parameters<typeof connection>[1][]) =>
  events.reduce<Connection>(connection, idle);

describe("sign-in progress", () => {
  it("moves from site discovery to the sign-in window to success", () => {
    expect(run({ type: "add" })).toEqual({ phase: "checking", siteId: null });
    expect(run({ type: "add" }, { type: "window-opened" })).toEqual({
      phase: "signing-in",
      siteId: null,
    });
    expect(
      run(
        { type: "add" },
        { type: "window-opened" },
        { type: "finished", error: null },
      ),
    ).toEqual({ phase: "idle", last: { siteId: null, error: null } });
  });

  it("reports discovery failures against the add form only", () => {
    const state = run(
      { type: "add" },
      { type: "failed", error: "Not a Moodle site." },
    );
    expect(lastError(state, null)).toBe("Not a Moodle site.");
    expect(lastError(state, "site-1")).toBeNull();
  });

  it("allows only one sign-in at a time, as Rust does", () => {
    const state = run({ type: "reauthenticate", siteId: "a" }, { type: "add" });
    expect(state).toEqual({ phase: "checking", siteId: "a" });
  });

  it("attributes a cancelled re-authentication to its site", () => {
    const state = run(
      { type: "reauthenticate", siteId: "a" },
      { type: "finished", error: "Sign-in cancelled." },
    );
    expect(lastError(state, "a")).toBe("Sign-in cancelled.");
    expect(lastError(state, null)).toBeNull();
  });

  it("ignores late events and never dismisses an attempt in progress", () => {
    expect(run({ type: "finished", error: "late" })).toBe(idle);
    expect(run({ type: "add" }, { type: "dismiss" }).phase).toBe("checking");
  });
});
