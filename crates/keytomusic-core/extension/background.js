// KeyToMusic Mood AI — background.
// Relays the content script to the KeyToMusic app (http://127.0.0.1:<port>),
// fetches images the page cannot hand over, and shows the mood on the icon.

const ext = globalThis.browser ?? globalThis.chrome;

const DEFAULTS = { port: 8765, enabled: true, sites: {} };
const MAX_WIDTH = 900;

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

function toBase64(buffer) {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (let i = 0; i < bytes.length; i += 0x8000) {
    binary += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
  }
  return btoa(binary);
}

// Pages whose pixels the page cannot read (cross-origin): fetch them with the
// extension's host permission, shrink, and return base64 JPEG.
async function fetchImage(url) {
  const response = await fetch(url, { credentials: 'include', signal: AbortSignal.timeout(30000) });
  if (!response.ok) throw new Error(`Image refusée par le site (HTTP ${response.status})`);
  const blob = await response.blob();
  const bitmap = await createImageBitmap(blob);
  const scale = Math.min(1, MAX_WIDTH / bitmap.width);
  const width = Math.max(1, Math.round(bitmap.width * scale));
  const height = Math.max(1, Math.round(bitmap.height * scale));
  const canvas = new OffscreenCanvas(width, height);
  canvas.getContext('2d').drawImage(bitmap, 0, 0, width, height);
  bitmap.close?.();
  const jpeg = await canvas.convertToBlob({ type: 'image/jpeg', quality: 0.85 });
  return toBase64(await jpeg.arrayBuffer());
}

function showMood(tabId, current) {
  if (tabId === undefined) return;
  const style = current ? MOODS[current.mood] : null;
  ext.action.setBadgeText({ tabId, text: style ? style[0] : '' }).catch(() => {});
  if (style) ext.action.setBadgeBackgroundColor({ tabId, color: style[1] }).catch(() => {});
}

async function handle(message, sender) {
  const config = await settings();
  const tabId = sender.tab?.id;
  switch (message.type) {
    case 'settings':
      return config;
    case 'status':
      return api('/api/status');
    case 'chapter':
      return api('/api/chapter', {
        chapter: message.chapter,
        title: message.title,
        totalPages: message.totalPages,
      });
    case 'pages':
      return api('/api/pages', { chapter: message.chapter, pages: message.pages });
    case 'fetchPage': {
      const data = await fetchImage(message.url);
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
