(() => {
  // évite doubles scripts
  if (window.__KORNER_NOTIF_BOOT__) return;
  window.__KORNER_NOTIF_BOOT__ = true;

  const XANO_BASE = "https://xqgk-qz19-wyoe.p7.xano.io/api:2H_0O_Xw";
  const POLL_EVERY_MS = 10_000;

  const seenIds = new Set();

  // file d'URLs à ouvrir (on ouvre au focus)
  const pendingRedirects = [];

  // ==============================
  // USER ID (fourni par WeWeb)
  // ==============================
  let CURRENT_USER_ID = null;

  window.setUserId = (id) => {
    const n = Number(id);
    if (!Number.isFinite(n)) {
      console.warn("[tauri] invalid user id", id);
      return;
    }
    CURRENT_USER_ID = n;
    seenIds.clear(); // reset dedupe quand user change
    console.log("[tauri] user_id set:", n);
  };

  window.clearUserId = () => {
    CURRENT_USER_ID = null;
    seenIds.clear();
    console.log("[tauri] user_id cleared");
  };

  function getInvoke() {
    const invoke = window?.__TAURI__?.core?.invoke || window?.__TAURI_INTERNALS__?.invoke;
    if (!invoke) throw new Error("Tauri invoke indisponible (pas dans l'app desktop).");
    return invoke;
  }

  function isInTauri() {
    return !!(window?.__TAURI__?.core?.invoke || window?.__TAURI_INTERNALS__?.invoke);
  }

  async function sendNativeNotification(title, body, redirect_url = null) {
    if (!isInTauri()) return;

    const invoke = getInvoke();
    await invoke("notify_from_xano", { title, body, redirect_url });

    // On enregistre l'URL pour l'ouvrir quand l'app reprend le focus (clic notif)
    if (redirect_url) pendingRedirects.push(redirect_url);
  }

  function openLatestPendingRedirect() {
    const url = pendingRedirects.pop();
    if (!url) return;
    window.location.href = url;
  }

  // Quand l'app revient au premier plan (clic notif), on ouvre la page
  window.addEventListener("focus", openLatestPendingRedirect);
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") openLatestPendingRedirect();
  });

  async function pollNotifications() {
    try {
      const userId = CURRENT_USER_ID;
      if (!userId) return; // pas loggé / pas encore set

      const url = `${XANO_BASE}/notifications?user_id=${encodeURIComponent(userId)}`;
      const res = await fetch(url, { cache: "no-store" });
      if (!res.ok) {
        console.warn("[poll] http", res.status);
        return;
      }

      const notifications = await res.json();
      if (!Array.isArray(notifications) || notifications.length === 0) return;

      for (const n of notifications) {
        const id = n?.id;
        if (id != null && seenIds.has(id)) continue;

        await sendNativeNotification(
          n?.titre ?? "Notification",
          n?.description ?? "",
          n?.redirect_url ?? null
        );

        if (id != null) seenIds.add(id);
      }
    } catch (e) {
      console.warn("[poll] error", e);
    }
  }

  // Boot polling
  pollNotifications();
  setInterval(pollNotifications, POLL_EVERY_MS);

  // Optionnel: aide debug
  window.__kornerPollNotifications = pollNotifications;
})();