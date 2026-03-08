// KeyToMusic Manga Mood — Background Service Worker
// Proxies visible-page mood analysis requests from content scripts to the local KeyToMusic API.
// Streams chapter pages into the local cache, then resolves visible pages via lookup with a
// visible-window fallback when the cache is still cold.

const ext = globalThis.browser ?? globalThis.chrome;
const DEFAULT_PORT = 8765;
const STATS_STORAGE_KEY = "ktmMoodDebugStats";
const CHAPTER_FEED_TTL_MS = 10000;
const ANALYZE_WINDOW_TIMEOUT_MS = 180000;

function createEmptyStats(chapter = null) {
  return {
    analyzed: 0,
    lastMood: null,
    lastPage: null,
    visiblePage: null,
    totalPages: null,
    focusDirection: 1,
    chapter,
    analyzedPages: [],
    readyPages: [],
    warmingPages: [],
    cacheEntries: 0,
    currentAnalyzingPage: null,
    currentPhase: null,
    currentPhaseStartedAt: null,
    statusNote: null,
    lastStatusNote: null,
    queuePages: [],
    prefetchCurrentPage: null,
    prefetchQueuePages: [],
    recentEvents: [],
  };
}

function normalizePageList(pages) {
  if (!Array.isArray(pages)) return [];
  return [...new Set(
    pages
      .map((page) => Number(page))
      .filter((page) => Number.isFinite(page))
  )].sort((a, b) => a - b);
}

function normalizeStatsSnapshot(raw) {
  const snapshot = {
    ...createEmptyStats(raw?.chapter ?? null),
    ...(raw || {}),
  };
  snapshot.analyzedPages = normalizePageList(snapshot.analyzedPages);
  snapshot.readyPages = normalizePageList(snapshot.readyPages);
  snapshot.warmingPages = normalizePageList(snapshot.warmingPages);
  snapshot.queuePages = normalizePageList(snapshot.queuePages);
  snapshot.prefetchQueuePages = normalizePageList(snapshot.prefetchQueuePages);
  snapshot.cacheEntries = Number.isFinite(snapshot.cacheEntries)
    ? snapshot.cacheEntries
    : snapshot.readyPages.length;
  snapshot.totalPages = Number.isFinite(snapshot.totalPages) ? snapshot.totalPages : null;
  snapshot.focusDirection = Number.isFinite(snapshot.focusDirection) && Number(snapshot.focusDirection) !== 0
    ? Math.sign(Number(snapshot.focusDirection))
    : 1;
  if (!Number.isFinite(snapshot.prefetchCurrentPage)) {
    snapshot.prefetchCurrentPage = null;
  }
  return snapshot;
}

function hasUsefulStats(raw) {
  const snapshot = normalizeStatsSnapshot(raw);
  return Boolean(
    snapshot.chapter ||
    snapshot.visiblePage !== null ||
    snapshot.currentAnalyzingPage !== null ||
    snapshot.analyzed > 0 ||
    snapshot.readyPages.length > 0 ||
    snapshot.prefetchCurrentPage !== null ||
    snapshot.prefetchQueuePages.length > 0
  );
}

let queue = [];
let processing = false;
let chapterFeedQueue = [];
let chapterFeedProcessing = false;
let chapterFeedActiveController = null;
let chapterFeedActiveItem = null;
let stats = createEmptyStats();
let activeLiveRequest = null;
let liveRequestSeq = 0;
let activeLookupRequests = new Map();
let pendingDebugState = null;
let chapterFeedChapter = null;
let chapterFocusState = {
  observedChapter: null,
  observedPage: null,
  sentChapter: null,
  sentPage: null,
  sentDirection: 0,
};
const chapterFeedRecent = new Map();
const chapterFeedInFlight = new Set();

function getStatsStorageArea() {
  return ext.storage?.session ?? ext.storage?.local ?? null;
}

async function persistStatsSnapshot() {
  const storage = getStatsStorageArea();
  if (!storage) return;
  try {
    await storage.set({ [STATS_STORAGE_KEY]: stats });
  } catch {
    // Ignore storage failures in debug telemetry.
  }
}

async function loadPersistedStatsSnapshot() {
  const storage = getStatsStorageArea();
  if (!storage) return null;
  try {
    const data = await storage.get(STATS_STORAGE_KEY);
    return data?.[STATS_STORAGE_KEY] ?? null;
  } catch {
    return null;
  }
}

async function clearPersistedStatsSnapshot() {
  const storage = getStatsStorageArea();
  if (!storage) return;
  try {
    await storage.remove(STATS_STORAGE_KEY);
  } catch {
    // Ignore storage failures in debug telemetry.
  }
}

// --- Config ---

async function getConfig() {
  const { ktmPort = DEFAULT_PORT, ktmEnabled = true } = await ext.storage.local.get(["ktmPort", "ktmEnabled"]);
  return { port: ktmPort, enabled: ktmEnabled };
}

// --- API ---

function apiUrl(port, path) {
  return `http://127.0.0.1:${port}${path}`;
}

async function checkStatus(port) {
  try {
    const res = await fetch(apiUrl(port, '/api/status'), { signal: AbortSignal.timeout(3000) });
    return await res.json();
  } catch {
    return { server: 'unreachable', model: 'unknown', port: 0 };
  }
}

async function fetchCacheStatus(port) {
  try {
    const res = await fetch(apiUrl(port, '/api/cache/status'), { signal: AbortSignal.timeout(3000) });
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}`);
    }
    return await res.json();
  } catch {
    return null;
  }
}

function isAbortError(err) {
  return err?.name === "AbortError" || err?.message === "The operation was aborted.";
}

function isTimeoutError(err) {
  return err?.name === "TimeoutError" || err?.message === "The operation timed out.";
}

function createAbortableTimeout(parentSignal, timeoutMs) {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => {
    controller.abort(new DOMException("The operation timed out.", "TimeoutError"));
  }, timeoutMs);

  const onParentAbort = () => {
    controller.abort(parentSignal?.reason ?? new DOMException("The operation was aborted.", "AbortError"));
  };

  if (parentSignal) {
    if (parentSignal.aborted) {
      onParentAbort();
    } else {
      parentSignal.addEventListener("abort", onParentAbort, { once: true });
    }
  }

  return {
    signal: controller.signal,
    cleanup() {
      clearTimeout(timeoutId);
      if (parentSignal) {
        parentSignal.removeEventListener("abort", onParentAbort);
      }
    },
  };
}

async function analyzeWindowMood(port, item, requestId, signal = null) {
  const members = [];
  const sortedMembers = [...(item.members || [])].sort((a, b) => a.page - b.page);
  for (const member of sortedMembers) {
    const image = member?.base64 || null;
    if (!image) {
      throw new Error(`Missing base64 image data for page ${member.page + 1}`);
    }
    members.push({ page: member.page, image });
  }

  const abortable = createAbortableTimeout(signal, ANALYZE_WINDOW_TIMEOUT_MS);
  try {
    const res = await fetch(apiUrl(port, '/api/analyze-window'), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        request_id: requestId,
        chapter: item.chapter,
        page: item.pageIndex,
        total_pages: item.totalPages,
        members,
      }),
      signal: abortable.signal,
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({ error: res.statusText }));
      throw new Error(err.error || `HTTP ${res.status}`);
    }
    return await res.json();
  } finally {
    abortable.cleanup();
  }
}

async function lookupMood(port, chapter, page, signal = null) {
  const abortable = createAbortableTimeout(signal, 5000);
  try {
    const res = await fetch(apiUrl(port, '/api/lookup'), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ chapter, page }),
      signal: abortable.signal,
    });
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}`);
    }
    return await res.json();
  } finally {
    abortable.cleanup();
  }
}

async function triggerMood(port, mood) {
  const res = await fetch(apiUrl(port, '/api/trigger'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ mood }),
    signal: AbortSignal.timeout(5000),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error || `HTTP ${res.status}`);
  }
  return await res.json();
}

async function submitChapterPage(port, item, signal = null) {
  const abortable = createAbortableTimeout(signal, 10000);
  try {
    const res = await fetch(apiUrl(port, '/api/chapter/page'), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        chapter: item.chapter,
        page: item.page,
        image: item.image,
        total_pages: item.totalPages,
      }),
      signal: abortable.signal,
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({ error: res.statusText }));
      throw new Error(err.error || `HTTP ${res.status}`);
    }
    return await res.json();
  } finally {
    abortable.cleanup();
  }
}

async function sendChapterFocus(port, chapter, page, direction = 0, totalPages = null, signal = null) {
  const abortable = createAbortableTimeout(signal, 5000);
  try {
    const res = await fetch(apiUrl(port, '/api/chapter/focus'), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        chapter,
        page,
        direction,
        total_pages: totalPages,
      }),
      signal: abortable.signal,
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({ error: res.statusText }));
      throw new Error(err.error || `HTTP ${res.status}`);
    }
    return await res.json();
  } finally {
    abortable.cleanup();
  }
}

async function cancelServerLiveRequest(port, requestId) {
  const abortable = createAbortableTimeout(null, 3000);
  try {
    await fetch(apiUrl(port, '/api/live/cancel'), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ request_id: requestId }),
      signal: abortable.signal,
    });
  } finally {
    abortable.cleanup();
  }
}

// --- Image fetch (fallback when content script can't use canvas) ---

function arrayBufferToBase64(buffer) {
  const bytes = new Uint8Array(buffer);
  const chunks = [];
  // Process in 32KB chunks to avoid call stack overflow
  for (let i = 0; i < bytes.length; i += 32768) {
    chunks.push(String.fromCharCode.apply(null, bytes.subarray(i, i + 32768)));
  }
  return btoa(chunks.join(''));
}

async function fetchImageAsBase64(url, signal = null) {
  const abortable = createAbortableTimeout(signal, 10000);
  try {
    const res = await fetch(url, { signal: abortable.signal });
    if (!res.ok) {
      throw new Error(`Failed to fetch image URL: HTTP ${res.status}`);
    }
    const contentType = res.headers.get("content-type") || "";
    if (!contentType.startsWith("image/")) {
      throw new Error(`Fetched URL did not return an image (${contentType || "unknown content-type"})`);
    }
    const buf = await res.arrayBuffer();
    return arrayBufferToBase64(buf);
  } finally {
    abortable.cleanup();
  }
}

async function sendTabMessage(tabId, payload) {
  if (!tabId) return;
  try {
    await ext.tabs.sendMessage(tabId, payload);
  } catch {
    // Tab may have closed or content script may not be present.
  }
}

async function executeContentScript(tabId) {
  if (ext.scripting?.executeScript) {
    return await ext.scripting.executeScript({
      target: { tabId },
      files: ["content.js"],
    });
  }
  if (ext.tabs?.executeScript) {
    return await ext.tabs.executeScript(tabId, { file: "content.js" });
  }
  throw new Error("Content script injection is not supported on this browser");
}

async function resolveImageBase64(item, preferUrl = false, signal = null) {
  const candidates = preferUrl
    ? [
        { kind: "url", value: item.url },
        { kind: "base64", value: item.base64 },
      ]
    : [
        { kind: "base64", value: item.base64 },
        { kind: "url", value: item.url },
      ];

  let lastError = null;
  for (const candidate of candidates) {
    if (!candidate.value) continue;
    try {
      if (candidate.kind === "base64") {
        return candidate.value;
      }
      return await fetchImageAsBase64(candidate.value, signal);
    } catch (err) {
      lastError = err;
      console.warn(`[KTM Mood] Failed to resolve image from ${candidate.kind}:`, err?.message || err);
    }
  }

  if (lastError) {
    throw lastError;
  }
  return null;
}

function getLiveItemKey(item) {
  if (item.chapter && item.pageIndex !== null && item.pageIndex !== undefined) {
    return `${item.chapter}::${item.pageIndex}`;
  }
  if (item.imgSelector) return `selector::${item.imgSelector}`;
  if (item.url) return `url::${item.url}`;
  if (item.base64) return `base64::${item.base64.slice(0, 48)}`;
  return `anon::${Date.now()}`;
}

function isActiveRequestCurrent(requestId) {
  return activeLiveRequest?.requestId === requestId;
}

function lookupRequestKey(tabId, chapter, page) {
  return `${tabId ?? "no-tab"}::${chapter ?? "no-chapter"}::${page ?? "no-page"}`;
}

function chapterFeedKey(chapter, page) {
  return `${chapter ?? "no-chapter"}::${page ?? "no-page"}`;
}

function chapterFeedRecentlySubmitted(key) {
  const lastAt = chapterFeedRecent.get(key);
  if (!lastAt) return false;
  if (Date.now() - lastAt <= CHAPTER_FEED_TTL_MS) {
    return true;
  }
  chapterFeedRecent.delete(key);
  return false;
}

function resetFocusTracking() {
  chapterFocusState = {
    observedChapter: null,
    observedPage: null,
    sentChapter: null,
    sentPage: null,
    sentDirection: 0,
  };
}

function queueChapterFocus(chapter, page, totalPages = null, explicitDirection = null) {
  if (!chapter || page === null || page === undefined) return;

  let direction = Number.isFinite(explicitDirection) ? Math.sign(explicitDirection) : 0;
  if (
    direction === 0 &&
    chapterFocusState.observedChapter === chapter &&
    chapterFocusState.observedPage !== null &&
    chapterFocusState.observedPage !== undefined
  ) {
    direction = Math.sign(page - chapterFocusState.observedPage);
  }

  chapterFocusState.observedChapter = chapter;
  chapterFocusState.observedPage = page;

  if (
    chapterFocusState.sentChapter === chapter &&
    chapterFocusState.sentPage === page &&
    chapterFocusState.sentDirection === direction
  ) {
    return;
  }

  chapterFocusState.sentChapter = chapter;
  chapterFocusState.sentPage = page;
  chapterFocusState.sentDirection = direction;

  void getConfig()
    .then((config) => {
      if (!config.enabled) return null;
      return sendChapterFocus(config.port, chapter, page, direction, totalPages);
    })
    .catch((err) => {
      console.warn("[KTM Mood] Chapter focus update failed:", err?.message || err);
    });
}

function resetChapterFeedState(chapter = null) {
  chapterFeedActiveController?.abort(
    new DOMException("Chapter feed reset.", "AbortError")
  );
  chapterFeedActiveController = null;
  chapterFeedActiveItem = null;
  chapterFeedQueue = [];
  chapterFeedInFlight.clear();
  chapterFeedRecent.clear();
  chapterFeedChapter = chapter;
  syncStatsSnapshot();
}

function enqueueChapterPages(items) {
  const queuedItems = [];
  const acceptedPages = [];
  for (const item of items) {
    if (!item?.chapter || item.page === null || item.page === undefined || !item.image) {
      continue;
    }
    if (chapterFeedChapter !== item.chapter) {
      resetChapterFeedState(item.chapter);
    }
    const key = chapterFeedKey(item.chapter, item.page);
    if (chapterFeedInFlight.has(key) || chapterFeedRecentlySubmitted(key)) {
      continue;
    }
    chapterFeedInFlight.add(key);
    queuedItems.push({ ...item, key });
    acceptedPages.push(item.page);
  }
  if (queuedItems.length > 0) {
    chapterFeedQueue = [...queuedItems, ...chapterFeedQueue];
  }
  syncStatsSnapshot();
  return acceptedPages;
}

async function processChapterFeedQueue() {
  if (chapterFeedProcessing || chapterFeedQueue.length === 0) return;
  chapterFeedProcessing = true;
  syncStatsSnapshot();

  try {
    while (chapterFeedQueue.length > 0) {
      const config = await getConfig();
      if (!config.enabled) {
        chapterFeedQueue = [];
        chapterFeedInFlight.clear();
        chapterFeedActiveItem = null;
        syncStatsSnapshot();
        return;
      }

      const item = chapterFeedQueue.shift();
      if (!item) continue;

      const controller = new AbortController();
      chapterFeedActiveController = controller;
      chapterFeedActiveItem = { ...item, startedAt: Date.now() };
      recordEvent("chapter-feed upload started", item.page ?? null);
      syncStatsSnapshot();
      try {
        await submitChapterPage(config.port, item, controller.signal);
        chapterFeedRecent.set(item.key, Date.now());
        rememberStatusNote(`Page ${Number(item.page) + 1} submitted to the backend chapter pipeline.`);
        recordEvent("chapter-feed upload queued", item.page ?? null);
      } catch (err) {
        if (isAbortError(err)) {
          rememberStatusNote(`Chapter prefetch upload cancelled for page ${Number(item.page) + 1}.`);
          continue;
        }
        rememberStatusNote(`Chapter prefetch failed for page ${Number(item.page) + 1}: ${err?.message || err}`);
        recordEvent(`chapter-feed error: ${err?.message || err}`, item.page ?? null);
        console.warn("[KTM Mood] Chapter feed failed:", err?.message || err);
      } finally {
        if (chapterFeedActiveController === controller) {
          chapterFeedActiveController = null;
        }
        if (chapterFeedActiveItem?.key === item.key) {
          chapterFeedActiveItem = null;
        }
        chapterFeedInFlight.delete(item.key);
        syncStatsSnapshot();
      }
    }
  } finally {
    chapterFeedProcessing = false;
    syncStatsSnapshot();
  }
}

async function notifyRequestCancelled(request) {
  if (!request || request.cancelNotified) return;
  request.cancelNotified = true;
  await sendTabMessage(request.tabId, {
    type: "mood_cancelled",
    pageIndex: request.pageIndex,
    imgSelector: request.imgSelector,
    reason: request.controller?.signal?.reason?.message || null,
  });
}

function cancelAllLookups(reason) {
  for (const entry of activeLookupRequests.values()) {
    entry.controller.abort(new DOMException(reason, "AbortError"));
  }
  activeLookupRequests.clear();
}

function cancelTabWork(tabId, reason) {
  if (tabId === null || tabId === undefined) return;

  const lookup = activeLookupRequests.get(tabId);
  if (lookup) {
    lookup.controller.abort(new DOMException(reason, "AbortError"));
    activeLookupRequests.delete(tabId);
  }

  if (queue.length > 0) {
    queue = queue.filter((item) => item.tabId !== tabId);
  }

  if (activeLiveRequest?.tabId === tabId) {
    cancelActiveVisibleRequest(reason);
  }

  syncStatsSnapshot();
}

function cancelActiveVisibleRequest(reason) {
  if (!activeLiveRequest) return;
  const requestId = activeLiveRequest.requestId;
  activeLiveRequest.superseded = true;
  activeLiveRequest.controller.abort(
    new DOMException(reason, "AbortError")
  );
  void getConfig()
    .then((config) => {
      if (!config.enabled) return null;
      return cancelServerLiveRequest(config.port, requestId);
    })
    .catch(() => null);
}

function ensureStatsChapter(chapter) {
  if (!chapter) return;
  if (stats.chapter === chapter) return;
  resetFocusTracking();
  stats = createEmptyStats(chapter);
  pendingDebugState = null;
  void persistStatsSnapshot();
}

function ensureChapterContext(chapter) {
  if (!chapter) return;
  ensureStatsChapter(chapter);
  if (chapterFeedChapter !== chapter) {
    resetChapterFeedState(chapter);
  }
}

function pushUniquePage(list, page) {
  if (page === null || page === undefined) return;
  if (!list.includes(page)) {
    list.push(page);
    list.sort((a, b) => a - b);
  }
}

function rememberStatusNote(note) {
  if (!note) return;
  stats.lastStatusNote = note;
  if (!activeLiveRequest && !pendingDebugState && !chapterFeedActiveItem) {
    stats.statusNote = note;
  }
}

function syncStatsSnapshot() {
  if (activeLiveRequest) {
    stats.currentAnalyzingPage = activeLiveRequest.pageIndex ?? null;
    stats.currentPhase = activeLiveRequest.phase ?? null;
    stats.currentPhaseStartedAt = activeLiveRequest.phaseStartedAt ?? null;
    stats.statusNote = activeLiveRequest.note ?? null;
  } else if (pendingDebugState) {
    stats.currentAnalyzingPage = pendingDebugState.pageIndex ?? null;
    stats.currentPhase = pendingDebugState.phase ?? null;
    stats.currentPhaseStartedAt = pendingDebugState.startedAt ?? null;
    stats.statusNote = pendingDebugState.note ?? null;
  } else if (chapterFeedActiveItem) {
    stats.currentAnalyzingPage = chapterFeedActiveItem.page ?? null;
    stats.currentPhase = "chapter_feed";
    stats.currentPhaseStartedAt = chapterFeedActiveItem.startedAt ?? Date.now();
    stats.statusNote = chapterFeedQueue.length > 0
      ? `Sending page ${Number(chapterFeedActiveItem.page) + 1} into the chapter cache. ${chapterFeedQueue.length} more page(s) queued behind it.`
      : `Sending page ${Number(chapterFeedActiveItem.page) + 1} into the chapter cache.`;
  } else {
    stats.currentAnalyzingPage = null;
    stats.currentPhase = null;
    stats.currentPhaseStartedAt = null;
    stats.statusNote = stats.lastStatusNote ?? null;
  }
  stats.queuePages = queue
    .map((item) => item.pageIndex)
    .filter((page) => page !== null && page !== undefined);
  stats.prefetchCurrentPage = chapterFeedActiveItem?.page ?? null;
  stats.prefetchQueuePages = chapterFeedQueue
    .map((item) => item.page)
    .filter((page) => page !== null && page !== undefined);
  void persistStatsSnapshot();
}

function markPendingDebug(pageIndex, phase, note = null) {
  pendingDebugState = {
    pageIndex,
    phase,
    startedAt: Date.now(),
    note,
  };
  syncStatsSnapshot();
}

function clearPendingDebug(pageIndex = null) {
  if (
    pendingDebugState &&
    (pageIndex === null || pendingDebugState.pageIndex === pageIndex)
  ) {
    pendingDebugState = null;
    syncStatsSnapshot();
  }
}

function setActivePhase(phase, note = null) {
  if (!activeLiveRequest) return;
  activeLiveRequest.phase = phase;
  activeLiveRequest.phaseStartedAt = Date.now();
  activeLiveRequest.note = note;
  syncStatsSnapshot();
}

function recordEvent(message, pageIndex = null) {
  const stamp = new Date().toLocaleTimeString("fr-FR", { hour12: false });
  const prefix = pageIndex !== null && pageIndex !== undefined ? `P${Number(pageIndex) + 1}` : "--";
  stats.recentEvents = Array.isArray(stats.recentEvents) ? stats.recentEvents : [];
  stats.recentEvents.unshift(`${stamp} · ${prefix} · ${message}`);
  stats.recentEvents = stats.recentEvents.slice(0, 12);
  void persistStatsSnapshot();
}

// --- Live analysis queue ---

async function processQueue() {
  if (processing || queue.length === 0) return;
  processing = true;

  const config = await getConfig();
  if (!config.enabled) {
    queue = [];
    processing = false;
    return;
  }

  // Take the most recent item (user has scrolled to it most recently)
  const item = queue.pop();
  // Drop older items — user has scrolled past them
  queue = [];
  ensureChapterContext(item.chapter ?? null);
  recordEvent("processing visible page request", item.pageIndex ?? null);
  const requestId = ++liveRequestSeq;
  const controller = new AbortController();
  const request = {
    requestId,
    controller,
    key: getLiveItemKey(item),
    pageIndex: item.pageIndex ?? null,
    chapter: item.chapter ?? null,
    tabId: item.tabId ?? null,
    imgSelector: item.imgSelector ?? null,
    priority: item.priority ?? "visible",
    phase: "resolve",
    phaseStartedAt: Date.now(),
    superseded: false,
    cancelNotified: false,
    note: null,
  };
  activeLiveRequest = request;
  clearPendingDebug(item.pageIndex ?? null);
  syncStatsSnapshot();

  try {
    try {
      if (item.chapter && item.pageIndex !== null && item.pageIndex !== undefined) {
        try {
          if (isActiveRequestCurrent(requestId)) {
            setActivePhase("lookup");
          }
          const lookup = await lookupMood(config.port, item.chapter, item.pageIndex, controller.signal);
          if (lookup.hit) {
            if (
              !isActiveRequestCurrent(requestId) ||
              controller.signal.aborted ||
              request.superseded
            ) {
              return;
            }
            await triggerMood(config.port, lookup.mood);
            stats.lastMood = lookup.mood;
            stats.lastPage = item.pageIndex;
            pushUniquePage(stats.analyzedPages, item.pageIndex);
            rememberStatusNote(`Page ${Number(item.pageIndex) + 1} resolved from cache -> ${lookup.mood}.`);
            recordEvent(`visible request resolved from cache -> ${lookup.mood}`, item.pageIndex ?? null);
            ext.action.setBadgeBackgroundColor({ color: getMoodColor(lookup.mood) });
            await sendTabMessage(item.tabId, {
              type: "mood_result",
              mood: lookup.mood,
              pageIndex: item.pageIndex,
              imgSelector: item.imgSelector,
            });
            return;
          }
        } catch (err) {
          if (isAbortError(err)) {
            throw err;
          }
          // Fallback to direct analysis below
        }
      }

      if (isActiveRequestCurrent(requestId)) {
        setActivePhase("analyze_window");
      }
      recordEvent("visible-window analysis started", item.pageIndex ?? null);
      const result = await analyzeWindowMood(config.port, item, requestId, controller.signal);
      if (
        !isActiveRequestCurrent(requestId) ||
        controller.signal.aborted ||
        request.superseded
      ) {
        return;
      }

      const committedMood = result.committed_mood || result.mood;
      stats.analyzed++;
      stats.lastMood = committedMood;
      stats.lastPage = item.pageIndex;
      pushUniquePage(stats.analyzedPages, item.pageIndex);
      rememberStatusNote(`Visible-window analysis completed for page ${Number(item.pageIndex) + 1} -> ${committedMood}.`);
      recordEvent(`visible-window result -> ${committedMood}`, item.pageIndex ?? null);

      await triggerMood(config.port, committedMood);

      // Update badge
      ext.action.setBadgeText({ text: String(stats.analyzed) });
      ext.action.setBadgeBackgroundColor({ color: getMoodColor(committedMood) });

      // Notify content script
      await sendTabMessage(item.tabId, {
        type: "mood_result",
        mood: committedMood,
        pageIndex: item.pageIndex,
        imgSelector: item.imgSelector,
      });
    } catch (err) {
      if (isTimeoutError(err)) {
        rememberStatusNote(`Visible-window analysis timed out for page ${Number(item.pageIndex) + 1}.`);
        recordEvent("analysis timed out", item.pageIndex ?? null);
        await sendTabMessage(item.tabId, {
          type: "mood_error",
          pageIndex: item.pageIndex,
          imgSelector: item.imgSelector,
          error: err.message,
        });
      } else if (isAbortError(err)) {
        rememberStatusNote(`Visible-window analysis cancelled for page ${Number(item.pageIndex) + 1}.`);
        recordEvent("analysis cancelled", item.pageIndex ?? null);
        await notifyRequestCancelled(request);
      } else {
        rememberStatusNote(`Visible-window analysis failed for page ${Number(item.pageIndex) + 1}: ${err?.message || err}`);
        recordEvent(`analysis error: ${err?.message || err}`, item.pageIndex ?? null);
        // Notify content script of error (for removing spinner)
        await sendTabMessage(item.tabId, {
          type: "mood_error",
          pageIndex: item.pageIndex,
          imgSelector: item.imgSelector,
          error: err.message,
        });
      }
    }
  } catch (err) {
    recordEvent(`queue setup error: ${err?.message || err}`, item.pageIndex ?? null);
  } finally {
    if (isActiveRequestCurrent(requestId)) {
      activeLiveRequest = null;
    }
    syncStatsSnapshot();
    processing = false;
    if (queue.length > 0) {
      void processQueue();
    }
  }
}

function enqueue(item) {
  ensureChapterContext(item.chapter ?? null);
  const key = getLiveItemKey(item);
  const priority = item.priority ?? "visible";

  if (priority === "visible") {
    queue = [item];
    if (activeLiveRequest && activeLiveRequest.key !== key) {
      cancelActiveVisibleRequest("Superseded by a newer visible page.");
    }
  } else {
    queue = [item];
  }

  processQueue();
  syncStatsSnapshot();
}

// --- Mood colors ---

const MOOD_COLORS = {
  epic: '#EF4444',
  tension: '#F97316',
  sadness: '#3B82F6',
  comedy: '#FACC15',
  romance: '#EC4899',
  horror: '#7C3AED',
  peaceful: '#22C55E',
  mystery: '#6366F1',
};

function getMoodColor(mood) {
  return MOOD_COLORS[mood] || '#6B7280';
}

// --- Message handler ---

ext.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (msg.type === 'chapter_pages') {
    (async () => {
      const config = await getConfig();
      if (!config.enabled) {
        sendResponse({ queued: 0, pages: [] });
        return;
      }

      ensureChapterContext(msg.chapter || null);
      if (Number.isFinite(msg.totalPages)) {
        stats.totalPages = Number(msg.totalPages);
      }
      const acceptedPages = enqueueChapterPages(
        (msg.pages || []).map((entry) => ({
          chapter: msg.chapter || null,
          page: entry?.page ?? null,
          image: entry?.image || null,
          totalPages: msg.totalPages ?? null,
          tabId: sender.tab?.id ?? null,
        }))
      );
      if (acceptedPages.length > 0) {
        rememberStatusNote(`Queued ${acceptedPages.length} page(s) for chapter prefetch.`);
        recordEvent(`prefetch queued ${acceptedPages.length} page(s)`, acceptedPages[0]);
      }
      void processChapterFeedQueue();
      sendResponse({ queued: acceptedPages.length, pages: acceptedPages });
    })();
    return true;
  }

  if (msg.type === 'chapter_focus') {
    ensureChapterContext(msg.chapter || null);
    stats.visiblePage = msg.pageIndex ?? null;
    if (Number.isFinite(msg.totalPages)) {
      stats.totalPages = Number(msg.totalPages);
    }
    if (Number.isFinite(msg.direction) && Number(msg.direction) !== 0) {
      stats.focusDirection = Math.sign(Number(msg.direction));
    }
    void persistStatsSnapshot();
    queueChapterFocus(
      msg.chapter || null,
      msg.pageIndex ?? null,
      msg.totalPages ?? null,
      msg.direction ?? null
    );
    sendResponse({ ok: true });
    return false;
  }

  if (msg.type === 'analyze_window') {
    (async () => {
      clearPendingDebug(msg.pageIndex ?? null);
      recordEvent("visible-window request queued", msg.pageIndex ?? null);
      enqueue({
        members: msg.members || [],
        pageIndex: msg.pageIndex,
        imgSelector: msg.imgSelector,
        chapter: msg.chapter || null,
        totalPages: msg.totalPages ?? null,
        tabId: sender.tab?.id,
        priority: "visible",
      });
      await processQueue();
      sendResponse({ queued: true });
    })();
    return true;
  }

  if (msg.type === 'visible_analysis_pending') {
    markPendingDebug(
      msg.pageIndex ?? null,
      "scheduled",
      msg.note || "Waiting for the page to stay visible before launching analysis."
    );
    recordEvent("analysis scheduled", msg.pageIndex ?? null);
    sendResponse({ ok: true });
    return false;
  }

  if (msg.type === 'visible_analysis_blocked') {
    markPendingDebug(
      msg.pageIndex ?? null,
      "blocked",
      msg.note || "Local context window is not ready yet."
    );
    recordEvent("analysis blocked: local window not ready", msg.pageIndex ?? null);
    sendResponse({ ok: true });
    return false;
  }

  if (msg.type === 'lookup') {
    ensureChapterContext(msg.chapter || null);
    stats.visiblePage = msg.page ?? null;
    void persistStatsSnapshot();
    recordEvent("lookup requested", msg.page ?? null);
    markPendingDebug(
      msg.page ?? null,
      "lookup",
      "Checking cached mood for the current visible page."
    );
    const visibleLookupKey = msg.chapter && msg.page !== null && msg.page !== undefined
      ? `${msg.chapter}::${msg.page}`
      : null;
    if (
      activeLiveRequest &&
      visibleLookupKey &&
      activeLiveRequest.key !== visibleLookupKey
    ) {
      cancelActiveVisibleRequest("Superseded by current visible page lookup.");
    }

    (async () => {
      const tabId = sender.tab?.id ?? null;
      const requestKey = lookupRequestKey(tabId, msg.chapter, msg.page);
      const controller = new AbortController();
      const previousLookup = tabId !== null ? activeLookupRequests.get(tabId) : null;
      if (previousLookup && previousLookup.key !== requestKey) {
        previousLookup.controller.abort(
          new DOMException("Superseded by a newer visible page lookup.", "AbortError")
        );
      }
      if (tabId !== null) {
        activeLookupRequests.set(tabId, { key: requestKey, controller });
      }

      try {
        const config = await getConfig();
        if (!config.enabled) { sendResponse({ hit: false }); return; }

        const result = await lookupMood(config.port, msg.chapter, msg.page, controller.signal);
        if (tabId !== null) {
          const currentLookup = activeLookupRequests.get(tabId);
          if (!currentLookup || currentLookup.key !== requestKey || controller.signal.aborted) {
            sendResponse({ hit: false, cancelled: true });
            return;
          }
        }

        if (result.hit) {
          clearPendingDebug(msg.page ?? null);
          rememberStatusNote(`Lookup hit for page ${Number(msg.page) + 1} -> ${result.mood}.`);
          recordEvent(`lookup hit -> ${result.mood}`, msg.page ?? null);
          await triggerMood(config.port, result.mood);
          stats.lastMood = result.mood;
          stats.lastPage = msg.page;
          ext.action.setBadgeBackgroundColor({ color: getMoodColor(result.mood) });
          pushUniquePage(stats.analyzedPages, msg.page);
        }

        if (!result.hit) {
          rememberStatusNote(`No cached mood yet for page ${Number(msg.page) + 1}.`);
          recordEvent("lookup miss", msg.page ?? null);
          markPendingDebug(
            msg.page ?? null,
            "lookup_miss",
            "No cached mood yet for this page. Waiting for visible-window analysis."
          );
        }

        sendResponse(result);
      } catch (err) {
        if (isTimeoutError(err)) {
          recordEvent("lookup timed out", msg.page ?? null);
          sendResponse({ hit: false, timeout: true });
          return;
        }
        if (isAbortError(err)) {
          recordEvent("lookup cancelled", msg.page ?? null);
          sendResponse({ hit: false, cancelled: true });
          return;
        }
        recordEvent(`lookup error: ${err?.message || err}`, msg.page ?? null);
        sendResponse({ hit: false });
      } finally {
        if (tabId !== null) {
          const currentLookup = activeLookupRequests.get(tabId);
          if (currentLookup?.key === requestKey) {
            activeLookupRequests.delete(tabId);
          }
        }
      }
    })();
    return true; // async response
  }

  if (msg.type === 're_trigger') {
    (async () => {
      try {
        const config = await getConfig();
        if (!config.enabled) { sendResponse({ ok: false }); return; }
        const data = await triggerMood(config.port, msg.mood);
        recordEvent(`re-trigger -> ${data.mood || msg.mood}`, msg.pageIndex ?? null);

        stats.lastMood = msg.mood;
        stats.lastPage = msg.pageIndex;
        ext.action.setBadgeBackgroundColor({ color: getMoodColor(data.mood || msg.mood) });

        sendResponse({ ok: true, mood: data.mood });
      } catch {
        sendResponse({ ok: false });
      }
    })();
    return true; // async response
  }

  if (msg.type === 'get_status') {
    (async () => {
      const config = await getConfig();
      const status = await checkStatus(config.port);
      const persistedStats = await loadPersistedStatsSnapshot();
      let effectiveStats = hasUsefulStats(stats)
        ? normalizeStatsSnapshot(stats)
        : normalizeStatsSnapshot(persistedStats ?? stats);

      if (config.enabled && status.server === "running") {
        const cacheStatus = await fetchCacheStatus(config.port);
        if (cacheStatus) {
          const cacheChapter = typeof cacheStatus.chapter === "string" ? cacheStatus.chapter : null;
          const readyPages = normalizePageList(cacheStatus.pages);
          const pipelinePages = normalizePageList(cacheStatus.pipeline_pages);
          const readyPageSet = new Set(readyPages);
          const warmingPages = pipelinePages.filter((page) => !readyPageSet.has(page));
          const backendProcessing = Boolean(cacheStatus.pipeline_processing);
          const backendPhase = typeof cacheStatus.active_phase === "string" ? cacheStatus.active_phase : null;
          const backendPage = Number.isFinite(cacheStatus.active_page) ? Number(cacheStatus.active_page) : null;
          const backendStartedAt = Number.isFinite(cacheStatus.active_started_at)
            ? Number(cacheStatus.active_started_at)
            : null;
          const backendError = typeof cacheStatus.last_error === "string" ? cacheStatus.last_error : null;
          const backendFocusPage = Number.isFinite(cacheStatus.focus_page) ? Number(cacheStatus.focus_page) : null;
          const sameChapter =
            !effectiveStats.chapter ||
            !cacheChapter ||
            effectiveStats.chapter === cacheChapter;

          if (sameChapter) {
            effectiveStats.chapter = effectiveStats.chapter ?? cacheChapter;
            effectiveStats.readyPages = readyPages;
            effectiveStats.analyzedPages = readyPages;
            effectiveStats.warmingPages = warmingPages;
            effectiveStats.cacheEntries = Number.isFinite(cacheStatus.entries)
              ? Number(cacheStatus.entries)
              : readyPages.length;
            if (effectiveStats.prefetchCurrentPage == null && backendFocusPage !== null) {
              effectiveStats.prefetchCurrentPage = backendFocusPage;
            }

            if (effectiveStats.currentAnalyzingPage == null) {
              if (backendProcessing && backendPhase && backendPage !== null) {
                effectiveStats.currentAnalyzingPage = backendPage;
                effectiveStats.currentPhase = `backend_${backendPhase}`;
                effectiveStats.currentPhaseStartedAt = backendStartedAt;
                effectiveStats.statusNote = `Backend chapter pipeline is analyzing page ${backendPage + 1} (${backendPhase}).`;
              } else if (backendProcessing) {
                effectiveStats.currentAnalyzingPage = backendFocusPage;
                effectiveStats.currentPhase = "backend_waiting";
                effectiveStats.currentPhaseStartedAt = null;
                effectiveStats.statusNote = "Backend chapter pipeline is active and waiting for the next ready job.";
              } else if (warmingPages.length > 0 && backendError) {
                effectiveStats.currentAnalyzingPage = backendFocusPage;
                effectiveStats.currentPhase = "backend_stalled";
                effectiveStats.currentPhaseStartedAt = null;
                effectiveStats.statusNote = `Backend chapter pipeline stalled: ${backendError}`;
              }
            }

            stats.readyPages = readyPages;
            stats.analyzedPages = readyPages;
            stats.warmingPages = warmingPages;
            stats.cacheEntries = effectiveStats.cacheEntries;
            void persistStatsSnapshot();
          } else {
            effectiveStats.readyPages = [];
            effectiveStats.analyzedPages = [];
            effectiveStats.warmingPages = [];
            effectiveStats.cacheEntries = 0;
          }
        }
      }

      sendResponse({ ...status, enabled: config.enabled, port: config.port, stats: effectiveStats });
    })();
    return true; // async response
  }

  if (msg.type === 'set_config') {
    if (msg.enabled === false && activeLiveRequest) {
      cancelActiveVisibleRequest("Mood detection disabled.");
    }
    if (msg.enabled === false) {
      cancelAllLookups("Mood detection disabled.");
      resetChapterFeedState(null);
      resetFocusTracking();
    }
    ext.storage.local.set({
      ktmPort: msg.port ?? DEFAULT_PORT,
      ktmEnabled: msg.enabled ?? true,
    }).then(() => sendResponse({ ok: true }));
    return true;
  }

  if (msg.type === 'reset_stats') {
    if (activeLiveRequest) {
      cancelActiveVisibleRequest("Stats reset.");
      activeLiveRequest = null;
    }
    cancelAllLookups("Stats reset.");
    stats = createEmptyStats();
    queue = [];
    resetChapterFeedState(null);
    resetFocusTracking();
    pendingDebugState = null;
    void clearPersistedStatsSnapshot();
    ext.action.setBadgeText({ text: '' });
    sendResponse({ ok: true });
    return false;
  }

  if (msg.type === 'inject_content_script') {
    executeContentScript(msg.tabId)
      .then(() => sendResponse({ ok: true }))
      .catch(e => sendResponse({ error: e.message }));
    return true;
  }
});

// Init badge
ext.action.setBadgeText({ text: '' });

if (ext.tabs?.onRemoved) {
  ext.tabs.onRemoved.addListener((tabId) => {
    cancelTabWork(tabId, "Tab closed.");
  });
}

if (ext.tabs?.onUpdated) {
  ext.tabs.onUpdated.addListener((tabId, changeInfo) => {
    if (changeInfo?.status === "loading") {
      cancelTabWork(tabId, "Tab navigated.");
    }
  });
}
