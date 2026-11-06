// KeyToMusic Mood AI — content script.
// Finds the pages of the manga being read (any reader that shows them as a
// column of images), sends them to the KeyToMusic app, and reports which page
// is on screen. The app splits the chapter into scenes and plays the music.

(() => {
  if (window.__ktmMoodAi) return;
  window.__ktmMoodAi = true;
  const ext = globalThis.browser ?? globalThis.chrome;

  const MIN_PAGES = 3;
  const MAX_WIDTH = 900;
  const CAPTURE_CONCURRENCY = 2;
  const AHEAD = 30;
  const RETRY_AFTER = 20000;
  const PLACEHOLDER = /^data:image\/(gif|svg)|\.svg(\?|$)|blank\.|loading|lazy[-_]?load|placeholder|spinner/i;

  let enabled = true;
  let siteChoice = undefined; // true / false / undefined (not asked yet)
  let chapter = null;
  let pages = [];
  let sent = new Set();
  let pending = new Set();
  const failedAt = new Map();
  let current = -1;
  let active = 0;
  let alive = true;
  let detectTimer = null;
  let scrollTimer = null;
  let prompt = null;

  // ── Messaging (survives an extension reload by going quiet) ──

  async function send(message) {
    if (!alive) return null;
    try {
      return await ext.runtime.sendMessage(message);
    } catch {
      alive = false;
      return null;
    }
  }

  // ── Finding the pages ──

  function lazySource(img) {
    for (const name of ['data-src', 'data-lazy-src', 'data-original', 'data-url', 'data-srcset', 'srcset']) {
      const value = img.getAttribute(name);
      if (value && value.trim()) {
        const first = value.trim().split(',')[0].trim().split(/\s+/)[0];
        if (first && !PLACEHOLDER.test(first)) return first;
      }
    }
    const src = img.currentSrc || img.getAttribute('src') || '';
    return src && !PLACEHOLDER.test(src) ? src : '';
  }

  function absolute(url) {
    try {
      return new URL(url, location.href).href;
    } catch {
      return '';
    }
  }

  function pageSized(img) {
    const rect = img.getBoundingClientRect();
    const width = rect.width || img.naturalWidth;
    const height = rect.height || img.naturalHeight;
    if (width < 280 || height < 200) return false;
    return height >= width * 0.6; // not a banner
  }

  // The reader is the element holding the most page-sized images; every image
  // in it with a real source is a page (lazy ones included, so indices stay put).
  function findPages() {
    const reader = document.querySelector('#readerarea, .reading-content, .reader-area, #reader');
    let candidates = Array.from((reader || document).querySelectorAll('img')).filter(pageSized);
    let column = reader;
    if (!column) {
      const counts = new Map();
      for (const img of candidates) {
        let el = img.parentElement;
        for (let depth = 0; el && depth < 5; depth++, el = el.parentElement) {
          counts.set(el, (counts.get(el) || 0) + 1);
        }
      }
      let bestCount = 0;
      for (const [el, count] of counts) {
        if (count > bestCount || (count === bestCount && column && column.contains(el))) {
          column = el;
          bestCount = count;
        }
      }
    }
    if (!column) return [];
    const all = Array.from(column.querySelectorAll('img')).filter((img) => {
      if (img.naturalWidth && img.naturalWidth < 250) return false; // icons, avatars
      const rect = img.getBoundingClientRect();
      if (rect.width && rect.width < 150) return false;
      return Boolean(lazySource(img)) || pageSized(img);
    });
    const sized = all.filter(pageSized).length;
    return sized >= MIN_PAGES ? all : [];
  }

  function chapterKey() {
    return location.origin + location.pathname.replace(/\/$/, '') + location.search;
  }

  // ── Capturing a page ──

  function toJpeg(img) {
    const width = img.naturalWidth;
    const height = img.naturalHeight;
    if (!width || !height) return null;
    const scale = Math.min(1, MAX_WIDTH / width);
    const canvas = document.createElement('canvas');
    canvas.width = Math.round(width * scale);
    canvas.height = Math.round(height * scale);
    try {
      canvas.getContext('2d').drawImage(img, 0, 0, canvas.width, canvas.height);
      return canvas.toDataURL('image/jpeg', 0.85).split(',')[1];
    } catch {
      return null; // cross-origin: the background fetches it instead
    }
  }

  function loadImage(url) {
    return new Promise((resolve) => {
      const img = new Image();
      img.decoding = 'async';
      img.onload = () => resolve(img);
      img.onerror = () => resolve(null);
      img.src = url;
      setTimeout(() => resolve(null), 30000);
    });
  }

  async function capture(index) {
    const key = chapter;
    const img = pages[index];
    if (!img) return;
    const url = absolute(lazySource(img));
    let data = null;
    if (img.complete && img.naturalWidth > 0 && !PLACEHOLDER.test(img.currentSrc || '')) {
      data = toJpeg(img);
    }
    if (!data && url) {
      const loaded = await loadImage(url);
      if (loaded) data = toJpeg(loaded);
    }
    let response = null;
    if (data) {
      response = await send({ type: 'pages', chapter: key, pages: [{ page: index, data }] });
    } else if (url && !url.startsWith('blob:')) {
      response = await send({ type: 'fetchPage', chapter: key, page: index, url });
    }
    if (key !== chapter) return;
    if (response?.ok && !(response.rejected || []).includes(index)) {
      sent.add(index);
    } else {
      failedAt.set(index, Date.now());
    }
  }

  // The page on screen first, then ahead, then behind.
  function nextToCapture() {
    const start = Math.max(0, current);
    const order = [];
    for (let i = start; i < pages.length && i <= start + AHEAD; i++) order.push(i);
    for (let i = start - 1; i >= 0 && i >= start - 10; i--) order.push(i);
    for (let i = start + AHEAD + 1; i < pages.length; i++) order.push(i);
    for (let i = start - 11; i >= 0; i--) order.push(i);
    const now = Date.now();
    return order.find(
      (i) => !sent.has(i) && !pending.has(i) && !(now - (failedAt.get(i) || 0) < RETRY_AFTER),
    );
  }

  function pump() {
    if (!running()) return;
    while (active < CAPTURE_CONCURRENCY) {
      const index = nextToCapture();
      if (index === undefined) return;
      pending.add(index);
      active++;
      capture(index).finally(() => {
        pending.delete(index);
        active--;
        pump();
      });
    }
  }

  // ── Position ──

  function visiblePage() {
    const line = window.innerHeight * 0.4;
    let best = -1;
    let bestArea = 0;
    pages.forEach((img, index) => {
      const rect = img.getBoundingClientRect();
      if (rect.top <= line && rect.bottom >= line) {
        best = index;
        bestArea = Infinity;
      } else if (bestArea !== Infinity) {
        const visible = Math.min(rect.bottom, window.innerHeight) - Math.max(rect.top, 0);
        if (visible > bestArea) {
          best = index;
          bestArea = visible;
        }
      }
    });
    return best;
  }

  async function reportPosition(force = false) {
    if (!running() || !pages.length) return;
    const page = visiblePage();
    if (page < 0 || (page === current && !force)) return;
    current = page;
    pump();
    const response = await send({
      type: 'position',
      chapter,
      page,
      totalPages: pages.length,
    });
    // The app restarted and lost the pages: send them again.
    if (response?.ok && response.received < sent.size) {
      sent = new Set();
      pump();
    }
  }

  // ── Chapter life cycle ──

  function running() {
    return alive && enabled && siteChoice === true && chapter !== null;
  }

  async function openChapter() {
    chapter = chapterKey();
    sent = new Set();
    pending = new Set();
    failedAt.clear();
    current = -1;
    const response = await send({
      type: 'chapter',
      chapter,
      title: document.title,
      totalPages: pages.length,
    });
    if (response?.ok) sent = new Set(response.received || []);
    await reportPosition(true);
    pump();
  }

  function stop() {
    chapter = null;
    pages = [];
    send({ type: 'clearBadge' });
  }

  async function detect() {
    if (!alive || !enabled) return;
    const found = findPages();
    if (!found.length) {
      if (chapter && chapterKey() !== chapter) stop();
      return;
    }
    if (siteChoice === undefined) {
      askForSite();
      return;
    }
    if (siteChoice !== true) return;
    const changed = chapterKey() !== chapter;
    pages = found;
    if (changed) {
      await openChapter();
    } else {
      pump();
      reportPosition();
    }
  }

  // Throttled, not debounced: readers that mutate constantly must still be seen.
  function scheduleDetect(delay = 800) {
    if (detectTimer && delay > 0) return;
    clearTimeout(detectTimer);
    detectTimer = setTimeout(() => {
      detectTimer = null;
      detect();
    }, delay);
  }

  // ── Asking once per site ──

  function askForSite() {
    if (prompt) return;
    prompt = document.createElement('div');
    const root = prompt.attachShadow({ mode: 'closed' });
    root.innerHTML = `
      <style>
        .box { position: fixed; right: 16px; bottom: 16px; z-index: 2147483647; max-width: 320px;
          font: 13px/1.4 system-ui, sans-serif; color: #1c1c1e; background: #fff; border-radius: 12px;
          box-shadow: 0 8px 28px rgba(0,0,0,.25); padding: 14px 16px; }
        b { display: block; margin-bottom: 4px; }
        .row { display: flex; gap: 8px; justify-content: flex-end; margin-top: 10px; }
        button { font: inherit; border: 0; border-radius: 8px; padding: 6px 12px; cursor: pointer; }
        .yes { background: #3f3fa8; color: #fff; }
        .no { background: #ececf0; color: #1c1c1e; }
      </style>
      <div class="box">
        <b>KeyToMusic Mood AI</b>
        Ce site ressemble à un lecteur de manga. Envoyer ses pages à KeyToMusic pour adapter la musique ?
        <div class="row"><button class="no">Jamais ici</button><button class="yes">Oui</button></div>
      </div>`;
    const answer = async (value) => {
      const { sites } = await ext.storage.local.get({ sites: {} });
      sites[location.hostname] = value;
      await ext.storage.local.set({ sites });
      prompt?.remove();
      prompt = null;
    };
    root.querySelector('.yes').addEventListener('click', () => answer(true));
    root.querySelector('.no').addEventListener('click', () => answer(false));
    document.documentElement.appendChild(prompt);
  }

  // ── Wiring ──

  async function loadSettings() {
    const { enabled: on, sites } = await ext.storage.local.get({ enabled: true, sites: {} });
    enabled = on;
    siteChoice = sites[location.hostname];
  }

  ext.storage.onChanged.addListener(async (changes, area) => {
    if (area !== 'local' || !(changes.enabled || changes.sites)) return;
    await loadSettings();
    if (!enabled || siteChoice === false) {
      stop();
    } else {
      chapter = null;
      scheduleDetect(0);
    }
  });

  window.addEventListener(
    'scroll',
    () => {
      if (scrollTimer) return;
      scrollTimer = setTimeout(() => {
        scrollTimer = null;
        reportPosition();
      }, 250);
    },
    { passive: true },
  );
  // Readers that turn pages without scrolling (one page at a time, SPA navigation).
  new MutationObserver(() => scheduleDetect()).observe(document.documentElement, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ['src', 'data-src', 'srcset'],
  });
  window.addEventListener('popstate', () => scheduleDetect(300));
  setInterval(() => {
    if (chapter && chapterKey() !== chapter) scheduleDetect(0);
    else if (running()) reportPosition();
  }, 2000);
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') reportPosition(true);
  });

  loadSettings().then(() => scheduleDetect(500));
})();
