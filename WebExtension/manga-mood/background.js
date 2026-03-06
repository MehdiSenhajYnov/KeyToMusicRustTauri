// KeyToMusic Manga Mood — Background Service Worker
// Proxies mood analysis requests from content scripts to the local KeyToMusic API.
// Handles queuing (sequential processing), pre-calculation queue, and config persistence.

const DEFAULT_PORT = 8765;
const MAX_QUEUE = 5;

let queue = [];
let processing = false;
let stats = { analyzed: 0, lastMood: null, lastPage: null };

// --- Pre-calculation state ---
let precalcQueue = [];
let precalcProcessing = false;
let lastViewedPage = 0; // for proximity-based priority sorting

// --- Config ---

async function getConfig() {
  const { ktmPort = DEFAULT_PORT, ktmEnabled = true } = await chrome.storage.local.get(['ktmPort', 'ktmEnabled']);
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

async function analyzeMood(port, base64Image) {
  const res = await fetch(apiUrl(port, '/api/analyze'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ image: base64Image }),
    signal: AbortSignal.timeout(30000), // VLM can be slow
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error || `HTTP ${res.status}`);
  }
  return await res.json();
}

async function analyzeMoodPrecalc(port, base64Image, chapter, page) {
  const res = await fetch(apiUrl(port, '/api/analyze'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      image: base64Image,
      precalculate: true,
      chapter,
      page,
    }),
    signal: AbortSignal.timeout(30000),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error || `HTTP ${res.status}`);
  }
  return await res.json();
}

async function lookupMood(port, chapter, page) {
  const res = await fetch(apiUrl(port, '/api/lookup'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ chapter, page }),
    signal: AbortSignal.timeout(5000),
  });
  if (!res.ok) {
    throw new Error(`HTTP ${res.status}`);
  }
  return await res.json();
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

async function fetchImageAsBase64(url) {
  const res = await fetch(url, { signal: AbortSignal.timeout(10000) });
  const buf = await res.arrayBuffer();
  return arrayBufferToBase64(buf);
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

  try {
    let base64;
    if (item.base64) {
      base64 = item.base64;
    } else if (item.url) {
      base64 = await fetchImageAsBase64(item.url);
    } else {
      processing = false;
      return;
    }

    const result = await analyzeMood(config.port, base64);

    stats.analyzed++;
    stats.lastMood = result.mood;
    stats.lastPage = item.pageIndex;

    // Update badge
    chrome.action.setBadgeText({ text: String(stats.analyzed) });
    chrome.action.setBadgeBackgroundColor({ color: getMoodColor(result.mood) });

    // Notify content script
    if (item.tabId) {
      chrome.tabs.sendMessage(item.tabId, {
        type: 'mood_result',
        mood: result.mood,
        pageIndex: item.pageIndex,
        imgSelector: item.imgSelector,
      }).catch(() => {}); // Tab may have closed
    }
  } catch (err) {
    // Notify content script of error (for removing spinner)
    if (item.tabId) {
      chrome.tabs.sendMessage(item.tabId, {
        type: 'mood_error',
        pageIndex: item.pageIndex,
        imgSelector: item.imgSelector,
        error: err.message,
      }).catch(() => {});
    }
  }

  processing = false;
  // Process next if any were added during processing
  if (queue.length > 0) {
    processQueue();
  } else {
    // Live queue empty — resume pre-calculation
    processPrecalcQueue();
  }
}

function enqueue(item) {
  // Limit queue size
  if (queue.length >= MAX_QUEUE) queue.shift();
  queue.push(item);
  processQueue();
}

// --- Pre-calculation queue ---

function sortPrecalcByProximity() {
  precalcQueue.sort((a, b) => {
    const distA = Math.abs(a.page - lastViewedPage);
    const distB = Math.abs(b.page - lastViewedPage);
    return distA - distB; // closest pages first
  });
}

async function processPrecalcQueue() {
  if (precalcProcessing || precalcQueue.length === 0) return;
  // Yield to live analysis — don't start if live queue has items
  if (processing || queue.length > 0) return;

  precalcProcessing = true;

  const config = await getConfig();
  if (!config.enabled) {
    precalcQueue = [];
    precalcProcessing = false;
    return;
  }

  // Re-sort by proximity to current page before picking
  sortPrecalcByProximity();
  const item = precalcQueue.shift();

  try {
    const result = await analyzeMoodPrecalc(config.port, item.base64, item.chapter, item.page);

    // Notify content script of precalc result
    if (item.tabId) {
      chrome.tabs.sendMessage(item.tabId, {
        type: 'precalc_result',
        mood: result.mood,
        page: item.page,
        cached: result.cached || false,
      }).catch(() => {});
    }
  } catch (err) {
    // Silent failure for precalc — not user-facing
    console.warn(`[KTM Precalc] Failed page ${item.page}:`, err.message);
  }

  precalcProcessing = false;

  // Yield to live queue if it got items while we were processing
  if (queue.length > 0 || processing) {
    return; // processQueue() will call us back when done
  }

  // Continue with next precalc item after a small delay
  if (precalcQueue.length > 0) {
    setTimeout(processPrecalcQueue, 100);
  }
}

function enqueuePrecalc(item) {
  // Dedup by page index
  if (precalcQueue.some(q => q.chapter === item.chapter && q.page === item.page)) return;
  precalcQueue.push(item);
  processPrecalcQueue();
}

// --- Mood colors ---

const MOOD_COLORS = {
  epic_battle: '#EF4444',
  tension: '#F97316',
  sadness: '#3B82F6',
  comedy: '#FACC15',
  romance: '#EC4899',
  horror: '#7C3AED',
  peaceful: '#22C55E',
  emotional_climax: '#F43F5E',
  mystery: '#6366F1',
  chase_action: '#F59E0B',
};

function getMoodColor(mood) {
  return MOOD_COLORS[mood] || '#6B7280';
}

// --- Message handler ---

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (msg.type === 'analyze') {
    enqueue({
      base64: msg.base64 || null,
      url: msg.url || null,
      pageIndex: msg.pageIndex,
      imgSelector: msg.imgSelector,
      tabId: sender.tab?.id,
    });
    sendResponse({ queued: true });
    return false;
  }

  if (msg.type === 'precalculate') {
    enqueuePrecalc({
      base64: msg.base64,
      chapter: msg.chapter,
      page: msg.page,
      tabId: sender.tab?.id,
    });
    sendResponse({ queued: true });
    return false;
  }

  if (msg.type === 'lookup') {
    lastViewedPage = msg.page; // Update for proximity sorting

    (async () => {
      try {
        const config = await getConfig();
        if (!config.enabled) { sendResponse({ hit: false }); return; }

        const result = await lookupMood(config.port, msg.chapter, msg.page);

        if (result.hit) {
          stats.lastMood = result.mood;
          stats.lastPage = msg.page;
          chrome.action.setBadgeBackgroundColor({ color: getMoodColor(result.mood) });
        }

        // Notify content script
        if (sender.tab?.id) {
          chrome.tabs.sendMessage(sender.tab.id, {
            type: 'lookup_result',
            hit: result.hit,
            mood: result.mood || null,
            page: msg.page,
          }).catch(() => {});
        }

        sendResponse(result);
      } catch {
        sendResponse({ hit: false });
      }
    })();
    return true; // async response
  }

  if (msg.type === 're_trigger') {
    (async () => {
      try {
        const config = await getConfig();
        if (!config.enabled) { sendResponse({ ok: false }); return; }

        const res = await fetch(apiUrl(config.port, '/api/trigger'), {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ mood: msg.mood }),
          signal: AbortSignal.timeout(5000),
        });
        const data = await res.json();

        stats.lastMood = msg.mood;
        stats.lastPage = msg.pageIndex;
        chrome.action.setBadgeBackgroundColor({ color: getMoodColor(msg.mood) });

        sendResponse({ ok: res.ok, mood: data.mood });
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
      sendResponse({ ...status, enabled: config.enabled, port: config.port, stats });
    })();
    return true; // async response
  }

  if (msg.type === 'set_config') {
    chrome.storage.local.set({
      ktmPort: msg.port ?? DEFAULT_PORT,
      ktmEnabled: msg.enabled ?? true,
    }).then(() => sendResponse({ ok: true }));
    return true;
  }

  if (msg.type === 'reset_stats') {
    stats = { analyzed: 0, lastMood: null, lastPage: null };
    precalcQueue = [];
    chrome.action.setBadgeText({ text: '' });
    sendResponse({ ok: true });
    return false;
  }

  if (msg.type === 'inject_content_script') {
    chrome.scripting.executeScript({
      target: { tabId: msg.tabId },
      files: ['content.js'],
    }).then(() => sendResponse({ ok: true }))
      .catch(e => sendResponse({ error: e.message }));
    return true;
  }
});

// Init badge
chrome.action.setBadgeText({ text: '' });
