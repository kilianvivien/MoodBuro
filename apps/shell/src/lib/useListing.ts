import { useCallback, useEffect, useRef, useState } from "react";
import { message, type Listing, type Site } from "../ipc";

/** Cached metadata older than this is refreshed when its view opens. */
export const STALE_SECONDS = 10 * 60;

/**
 * A cached, account-bound list (courses, agenda) with a refresh per organisation.
 * The cache shows at once; stale or missing organisations refresh in the background.
 * One organisation failing leaves the others, and the cached rows, in place.
 */
export function useListing<T>(
  sites: readonly Site[],
  load: () => Promise<Listing<T>>,
  refreshSite: (id: string) => Promise<void>,
) {
  const [listing, setListing] = useState<Listing<T> | null>(null);
  const [refreshing, setRefreshing] = useState<readonly string[]>([]);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [loadError, setLoadError] = useState<string | null>(null);
  const alive = useRef(true);
  const started = useRef(false);
  const ids = sites.map((site) => site.id).join("\n");

  const reload = useCallback(async () => {
    try {
      const next = await load();
      if (alive.current) {
        setListing(next);
        setLoadError(null);
      }
      return next;
    } catch (cause) {
      if (alive.current) setLoadError(message(cause));
      return null;
    }
  }, [load]);

  const refresh = useCallback(
    async (only?: readonly string[]) => {
      const targets = only ?? ids.split("\n").filter(Boolean);
      if (targets.length === 0) return;
      setRefreshing((current) => [...new Set([...current, ...targets])]);
      await Promise.all(
        targets.map(async (id) => {
          let failure: string | null = null;
          try {
            await refreshSite(id);
          } catch (cause) {
            failure = message(cause);
          }
          if (!alive.current) return;
          setErrors((current) => {
            const next = { ...current };
            if (failure) next[id] = failure;
            else delete next[id];
            return next;
          });
        }),
      );
      await reload();
      if (alive.current)
        setRefreshing((current) =>
          current.filter((id) => !targets.includes(id)),
        );
    },
    [ids, refreshSite, reload],
  );

  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    void reload().then((loaded) => {
      if (!loaded || !alive.current) return;
      const now = Math.floor(Date.now() / 1000);
      const stale = ids
        .split("\n")
        .filter(Boolean)
        .filter((id) => {
          const at = loaded.refreshed.find((r) => r.siteId === id)?.refreshedAt;
          return at === undefined || now - at > STALE_SECONDS;
        });
      if (stale.length > 0) void refresh(stale);
    });
  }, [ids, refresh, reload]);

  return {
    items: listing?.items ?? [],
    refreshed: listing?.refreshed ?? [],
    loaded: listing !== null,
    refreshing,
    errors,
    loadError,
    reload,
    refresh,
  };
}
