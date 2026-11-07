// KeyToMusic Mood AI — background.
// Relays the content script to the KeyToMusic app (http://127.0.0.1:<port>),
// fetches the rare pages the app cannot download itself (with the reader as
// Referer), shows the mood on the icon, and keeps itself up to date with the
// app.

const ext = globalThis.browser ?? globalThis.chrome;

const DEFAULTS = { port: 8765, enabled: true, sites: {} };
const MAX_WIDTH = 1600;

const MOODS = {
  epic: ['ÉPI', '#b42318'],
  tension: ['TEN', '#b54708'],
  sadness: ['TRI', '#1d5fb0'],
  comedy: ['COM', '#8a6a00'],
  romance: ['ROM', '#b4236e'],
  horror: ['HOR', '#6e2bb0'],
  peaceful: ['PAI', '#1f7a3d'],
  mystery: ['MYS', '#3f3fa8'],
};

async function settings() {
  const stored = await ext.storage.local.get(DEFAULTS);
  return { ...DEFAULTS, ...stored };
}

async function api(path, body) {
  const { port } = await settings();
  const init = body === undefined
    ? { signal: AbortSignal.timeout(4000) }
    : {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
        signal: AbortSignal.timeout(60000),
      };
  let response;
  try {
    response = await fetch(`http://127.0.0.1:${port}${path}`, init);
  } catch {
    throw new Error(`KeyToMusic ne répond pas sur le port ${port}. L'app est-elle ouverte, avec Mood AI activé ?`);
  }
  const data = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(data.error || `HTTP ${response.status}`);
  return data;
}

// ── Staying up to date ──
// The app rewrites the folder this extension was loaded from when it ships a
// newer version; reloading picks the new files up.

function newer(a, b) {
  const pa = String(a).split('.').map(Number);
  const pb = String(b).split('.').map(Number);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] || 0) - (pb[i] || 0);
    if (d) return d > 0;
  }
  return false;
}

let lastVersionCheck = 0;
async function checkVersion(status) {
  const now = Date.now();
  if (!status) {
    if (now - lastVersionCheck < 30000) return;
    lastVersionCheck = now;
    status = await api('/api/status').catch(() => null);
  }
  const shipped = status?.extensionVersion;
  if (!shipped || !newer(shipped, ext.runtime.getManifest().version)) return;
  const self = await ext.management?.getSelf?.().catch(() => null);
  if (self && self.installType !== 'development') return;
  console.info(`KeyToMusic Mood AI: updating to ${shipped}`);
  ext.runtime.reload();
}

// Pages already open when the extension (re)loads get the content script too.
async function injectIntoOpenTabs() {
  if (!ext.scripting) return;
  const tabs = await ext.tabs.query({ url: ['http://*/*', 'https://*/*'] }).catch(() => []);
  for (const tab of tabs) {
    ext.scripting.executeScript({ target: { tabId: tab.id }, files: ['content.js'] }).catch(() => {});
  }
}

ext.runtime.onInstalled.addListener(() => {
  injectIntoOpenTabs();
  ext.alarms?.create('version', { periodInMinutes: 1 });
});
ext.runtime.onStartup?.addListener(() => ext.alarms?.create('version', { periodInMinutes: 1 }));
ext.alarms?.onAlarm.addListener((alarm) => {
  if (alarm.name === 'version') checkVersion();
});

// ── Pages the app could not download ──

function toBase64(buffer) {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (let i = 0; i < bytes.length; i += 0x8000) {
    binary += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
  }
  return btoa(binary);
}

// Image hosts often refuse requests without the reader's Referer, which a
// fetch from here cannot set: a session rule adds it to this one request.
let nextRuleId = 1;
async function withReferer(url, referer, work) {
  const dnr = ext.declarativeNetRequest;
  if (!dnr || !referer) return work();
  const id = nextRuleId++;
  let added = false;
  try {
    await dnr.updateSessionRules({
      addRules: [{
        id,
        priority: 1,
        action: {
          type: 'modifyHeaders',
          requestHeaders: [{ header: 'referer', operation: 'set', value: referer }],
        },
        condition: {
          urlFilter: `|${url.replace(/[|^*]/g, '*')}|`,
          tabIds: [ext.tabs.TAB_ID_NONE],
          resourceTypes: ['xmlhttprequest', 'other', 'image'],
        },
      }],
    });
    added = true;
  } catch (error) {
    console.warn('KeyToMusic Mood AI: no Referer rule', error);
  }
  try {
    return await work();
  } finally {
    if (added) dnr.updateSessionRules({ removeRuleIds: [id] }).catch(() => {});
  }
}

async function fetchImage(url, referer) {
  const response = await withReferer(url, referer, () =>
    fetch(url, { credentials: 'include', signal: AbortSignal.timeout(30000) }),
  );
  if (!response.ok) throw new Error(`Image refusée par le site (HTTP ${response.status})`);
  const blob = await response.blob();
  const bitmap = await createImageBitmap(blob);
  const scale = Math.min(1, MAX_WIDTH / bitmap.width);
  const width = Math.max(1, Math.round(bitmap.width * scale));
  const height = Math.max(1, Math.round(bitmap.height * scale));
  const canvas = new OffscreenCanvas(width, height);
  canvas.getContext('2d').drawImage(bitmap, 0, 0, width, height);
  bitmap.close?.();
  const jpeg = await canvas.convertToBlob({ type: 'image/jpeg', quality: 0.88 });
  return toBase64(await jpeg.arrayBuffer());
}

// ── Icon ──

function showMood(tabId, current) {
  if (tabId === undefined) return;
  const style = current ? MOODS[current.mood] : null;
  ext.action.setBadgeText({ tabId, text: style ? style[0] : '' }).catch(() => {});
  if (style) ext.action.setBadgeBackgroundColor({ tabId, color: style[1] }).catch(() => {});
}

// ── Messages from the content script and the popup ──

async function handle(message, sender) {
  const config = await settings();
  const tabId = sender.tab?.id;
  switch (message.type) {
    case 'settings':
      return config;
    case 'status': {
      const status = await api('/api/status');
      checkVersion(status);
      return status;
    }
    case 'chapter':
      checkVersion();
      return api('/api/chapter', {
        chapter: message.chapter,
        title: message.title,
        url: message.url,
        pages: message.pages,
        totalPages: message.totalPages,
      });
    case 'pages':
      return api('/api/pages', { chapter: message.chapter, pages: message.pages });
    case 'fetchPage': {
      const data = await fetchImage(message.url, message.referer);
      return api('/api/pages', {
        chapter: message.chapter,
        pages: [{ page: message.page, data }],
      });
    }
    case 'position': {
      const result = await api('/api/position', {
        chapter: message.chapter,
        page: message.page,
        totalPages: message.totalPages,
      });
      showMood(tabId, result.current);
      return result;
    }
    case 'clearBadge':
      showMood(tabId, null);
      return {};
    default:
      throw new Error(`Message inconnu : ${message.type}`);
  }
}

ext.runtime.onMessage.addListener((message, sender, sendResponse) => {
  handle(message, sender).then(
    (result) => sendResponse({ ok: true, ...result }),
    (error) => sendResponse({ ok: false, error: String(error?.message || error) }),
  );
  return true;
});
