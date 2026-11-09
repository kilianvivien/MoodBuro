/**
 * Sign-in progress shared by onboarding and Settings. Rust allows one pending
 * sign-in at a time, so a single state machine covers add and re-authenticate.
 *
 * checking   – Rust is discovering the site's public configuration
 * signing-in – the organisation sign-in window is open (until auth://finished)
 * idle       – `last` keeps the outcome of the previous attempt, if any
 *
 * `siteId` is null when adding a new organisation, or the id being re-authenticated.
 */
export interface Outcome {
  siteId: string | null;
  error: string | null;
}
export type Connection =
  | { phase: "idle"; last: Outcome | null }
  | { phase: "checking"; siteId: string | null }
  | { phase: "signing-in"; siteId: string | null };

export type ConnectionEvent =
  | { type: "add" }
  | { type: "reauthenticate"; siteId: string }
  | { type: "window-opened" }
  | { type: "finished"; error: string | null }
  | { type: "failed"; error: string }
  | { type: "dismiss" };

export const idle: Connection = { phase: "idle", last: null };

export function connection(
  state: Connection,
  event: ConnectionEvent,
): Connection {
  switch (event.type) {
    case "add":
      return state.phase === "idle"
        ? { phase: "checking", siteId: null }
        : state;
    case "reauthenticate":
      return state.phase === "idle"
        ? { phase: "checking", siteId: event.siteId }
        : state;
    case "window-opened":
      return state.phase === "checking"
        ? { phase: "signing-in", siteId: state.siteId }
        : state;
    case "finished":
    case "failed":
      // A late or duplicate event after the attempt ended is ignored.
      if (state.phase === "idle") return state;
      return {
        phase: "idle",
        last: { siteId: state.siteId, error: event.error },
      };
    case "dismiss":
      return state.phase === "idle" ? idle : state;
  }
}

/** The previous attempt's error, when it concerned `siteId` (null = adding). */
export function lastError(state: Connection, siteId: string | null) {
  return state.phase === "idle" && state.last?.siteId === siteId
    ? state.last.error
    : null;
}
