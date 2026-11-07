// KeyToMusic Mood AI — content script.
// Finds the pages of the chapter being read, tells the KeyToMusic app where
// they are (the app downloads them and splits the chapter into scenes), and
// reports which page is on screen so the music follows the reading.
//
// Readers it knows:
// - "ts_reader" (most WordPress manga sites: sushiscan…): the page list is in
//   the page's script, one page or all of them shown at a time;
// - Madara (`chapter_preloaded_images`, `.reading-content`);
// - any reader showing the pages as a column of images.

(() => {
  const VERSION = '3.0.0';
  // A newer copy (extension reloaded) takes over; older ones go quiet.
  if (window.__ktmMoodAi === VERSION) return;
  window.__ktmMoodAi = VERSION;
  const ext = globalThis.browser ?? globalThis.chrome;

  const MIN_PAGES = 3;
  const MAX_WIDTH = 1600;
  const UPLOADS_AT_ONCE = 2;
  const RETRY_AFTER = 15000;
  const PLACEHOLDER = /^data:image\/(gif|svg)|\.svg(\?|$)|blank\.|loading|lazy[-_]?load|placeholder|spinner/i;

  let enabled = true;
  let siteChoice = undefined; // true / false / undefined (not asked yet)
  let reader = null; // { kind, urls, elements? }
  let chapter = null;
  let current = -1;
  let wanted = [];
  const uploading = new Set();
  const uploaded = new Set();
  const failedAt = new Map();
  let alive = true;
  let detectTimer = null;
  let scrollTimer = null;
  let prompt = null;

  // ── Messaging (an extension reload orphans this copy: it goes quiet) ──

  async function send(message) {
    if (!alive) return null;
    try {
      return await ext.runtime.sendMessage(message);
    } catch {
      alive = false;
      return null;
    }
  }

  function absolute(url) {
    try {
      return new URL(url, location.href).href;
    } catch {
      return '';
    }
  }

  // ── Readers ──

  // The JSON object passed to `name(` in the page's scripts.
  function scriptObject(name) {
    for (const script of document.scripts) {
      const text = script.textContent;
      const at = text.indexOf(name);
      if (at < 0) continue;
      const start = text.indexOf('{', at + name.length);
      if (start < 0) continue;
      let depth = 0;
      let inString = false;
      let escaped = false;
      for (let i = start; i < text.length; i++) {
        const c = text[i];
        if (inString) {
          if (escaped) escaped = false;
          else if (c === '\\') escaped = true;
          else if (c === '"') inString = false;
          continue;
        }
        if (c === '"') inString = true;
        else if (c === '{') depth++;
        else if (c === '}' && --depth === 0) {
          try {
            return JSON.parse(text.slice(start, i + 1));
          } catch {
            break;
          }
        }
      }
    }
    return null;
  }

  function scriptArray(name) {
    for (const script of document.scripts) {
      const text = script.textContent;
      const at = text.indexOf(name);
      if (at < 0) continue;
      const match = text.slice(at).match(/=\s*(\[[\s\S]*?\])\s*[;,\n]/);
      if (!match) continue;
      try {
        const list = JSON.parse(match[1]);
        if (Array.isArray(list)) return list;
      } catch {}
    }
    return null;
  }

  function tsReader() {
    if (!document.querySelector('#readerarea')) return null;
    const config = scriptObject('ts_reader.run(');
    const sources = config?.sources || [];
    const images = sources.find((s) => Array.isArray(s.images) && s.images.length)?.images;
    if (!images || images.length < 1) return null;
    return { kind: 'ts', urls: images.map(absolute) };
  }

  function madara() {
    const list = scriptArray('chapter_preloaded_images');
    if (list && list.length >= MIN_PAGES) {
      return { kind: 'madara', urls: list.map((u) => absolute(typeof u === 'string' ? u : u.src || '')) };
    }
    return null;
  }

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

  function pageSized(img) {
    const rect = img.getBoundingClientRect();
    const width = rect.width || img.naturalWidth;
    const height = rect.height || img.naturalHeight;
    if (width < 280 || height < 200) return false;
    return height >= width * 0.6; // not a banner
  }

  // The column holding the most page-sized images; every image in it with a
  // real source is a page (lazy ones included, so indices stay put).
  function column() {
    const known = document.querySelector('.reading-content, .reader-area, #reader, .chapter-content, .container-chapter-reader');
    let candidates = Array.from((known || document).querySelectorAll('img')).filter(pageSized);
    let best = known;
    if (!best) {
      const counts = new Map();
      for (const img of candidates) {
        let el = img.parentElement;
        for (let depth = 0; el && depth < 5; depth++, el = el.parentElement) {
          counts.set(el, (counts.get(el) || 0) + 1);
        }
      }
      let bestCount = 0;
      for (const [el, count] of counts) {
        if (count > bestCount || (count === bestCount && best && best.contains(el))) {
          best = el;
          bestCount = count;
        }
      }
    }
    if (!best) return null;
    const elements = Array.from(best.querySelectorAll('img')).filter((img) => {
      if (img.naturalWidth && img.naturalWidth < 250) return false; // icons, avatars
      const rect = img.getBoundingClientRect();
      if (rect.width && rect.width < 150) return false;
      return Boolean(lazySource(img)) || pageSized(img);
    });
    if (elements.filter(pageSized).length < MIN_PAGES) return null;
    return { kind: 'dom', urls: elements.map((img) => absolute(lazySource(img))), elements };
  }

  function findReader() {
    return tsReader() || madara() || column();
  }

  // ── Where the reader is ──

  function normalized(url) {
    return absolute(url).replace(/[?#].*$/, '');
  }

  // The image crossing 40% of the window, else the most visible one.
  function visible(elements) {
    const line = window.innerHeight * 0.4;
    let best = null;
    let bestArea = 0;
    for (const el of elements) {
      const rect = el.getBoundingClientRect();
      if (!rect.height) continue;
      if (rect.top <= line && rect.bottom >= line) return el;
      const area = Math.min(rect.bottom, window.innerHeight) - Math.max(rect.top, 0);
      if (area > bestArea) {
        best = el;
        bestArea = area;
      }
    }
    return best;
  }

  function indexOfImage(img) {
    const index = Number(img.getAttribute('data-index'));
    if (img.hasAttribute('data-index') && Number.isInteger(index)) return index;
    const src = normalized(lazySource(img) || img.currentSrc || img.src || '');
    return reader.urls.findIndex((u) => normalized(u) === src);
  }

  function visiblePage() {
    if (!reader) return -1;
    if (reader.kind === 'dom') {
      const el = visible(reader.elements);
      return el ? reader.elements.indexOf(el) : -1;
    }
    // ts_reader shows one page (single mode) or all of them (vertical mode).
    const shown = Array.from(
      document.querySelectorAll(reader.kind === 'ts' ? '#readerarea img' : '.reading-content img, #readerarea img'),
    ).filter((img) => img.getBoundingClientRect().height > 0);
    if (!shown.length) {
      const select = document.querySelector('#select-paged');
      return select ? Number(select.value) || 0 : -1;
    }
    const el = shown.length === 1 ? shown[0] : visible(shown);
    return el ? indexOfImage(el) : -1;
  }

  // ── Pages the app could not download ──

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
      return canvas.toDataURL('image/jpeg', 0.88).split(',')[1];
    } catch {
      return null; // cross-origin: the background fetches it instead
    }
  }

  function elementFor(index) {
    if (reader.elements) return reader.elements[index];
    const url = normalized(reader.urls[index] || '');
    return Array.from(document.images).find((img) => normalized(img.currentSrc || img.src) === url);
  }

  async function upload(index) {
    const key = chapter;
    const url = reader.urls[index];
    const img = elementFor(index);
    let data = null;
    if (img && img.complete && img.naturalWidth > 0) data = toJpeg(img);
    let response = null;
    if (data) {
      response = await send({ type: 'pages', chapter: key, pages: [{ page: index, data }] });
    } else if (url && /^https?:/.test(url)) {
      response = await send({ type: 'fetchPage', chapter: key, page: index, url, referer: location.href });
    }
    if (key !== chapter) return;
    if (response?.ok && !(response.rejected || []).includes(index)) uploaded.add(index);
    else failedAt.set(index, Date.now());
  }

  function pumpUploads() {
    if (!running()) return;
    const now = Date.now();
    for (const index of wanted) {
      if (uploading.size >= UPLOADS_AT_ONCE) return;
      if (uploading.has(index) || uploaded.has(index)) continue;
      if (now - (failedAt.get(index) || 0) < RETRY_AFTER) continue;
      uploading.add(index);
      upload(index).finally(() => {
        uploading.delete(index);
        pumpUploads();
      });
    }
  }

  function setWanted(list) {
    if (!Array.isArray(list)) return;
    // Closest to the page being read first.
    const focus = Math.max(0, current);
    wanted = list.slice().sort((a, b) => {
      const da = a >= focus ? a - focus : focus - a + 1000;
      const db = b >= focus ? b - focus : focus - b + 1000;
      return da - db;
    });
    pumpUploads();
  }

  // ── Talking to the app ──

  async function reportPosition(force = false) {
    // Only the tab being read drives the music.
    if (!running() || document.visibilityState !== 'visible') return;
    const page = visiblePage();
    if (page < 0 || (page === current && !force)) return;
    current = page;
    const response = await send({ type: 'position', chapter, page, totalPages: reader.urls.length });
    if (!response?.ok) return;
    // The app restarted and lost the chapter: announce it again.
    if (response.total === 0 || response.received === 0) {
      announce();
      return;
    }
    setWanted(response.wanted);
  }

  let announcing = false;
  async function announce() {
    if (announcing || !running()) return;
    announcing = true;
    try {
      const response = await send({
        type: 'chapter',
        chapter,
        title: document.title,
        url: location.href,
        pages: reader.urls,
        totalPages: reader.urls.length,
      });
      if (response?.ok) setWanted(response.wanted);
    } finally {
      announcing = false;
    }
  }

  // ── Chapter life cycle ──

  function chapterKey() {
    return location.origin + location.pathname.replace(/\/$/, '') + location.search;
  }

  function running() {
    return alive && enabled && siteChoice === true && chapter !== null && reader !== null;
  }

  function stop() {
    chapter = null;
    reader = null;
    send({ type: 'clearBadge' });
  }

  async function detect() {
    if (!alive || !enabled) return;
    const found = findReader();
    if (!found) {
      if (chapter && chapterKey() !== chapter) stop();
      return;
    }
    if (siteChoice === undefined) {
      askForSite();
      return;
    }
    if (siteChoice !== true) return;
    const key = chapterKey();
    const changed = key !== chapter || found.urls.join('\n') !== reader?.urls.join('\n');
    reader = found;
    if (changed) {
      chapter = key;
      current = -1;
      wanted = [];
      uploaded.clear();
      failedAt.clear();
      await announce();
      await reportPosition(true);
    } else {
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
        .box { position: fixed; right: 16px; bottom: 16px; z-index: 2147483647; max-width: 330px;
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
        Lancer la musique qui colle à ce que tu lis sur ${location.hostname} ? Les pages du chapitre sont analysées par KeyToMusic.
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
    attributeFilter: ['src', 'data-src', 'data-index', 'srcset'],
  });
  document.addEventListener('change', (event) => {
    if (event.target?.id === 'select-paged') setTimeout(() => reportPosition(), 50);
  });
  window.addEventListener('popstate', () => scheduleDetect(300));
  window.addEventListener('hashchange', () => setTimeout(() => reportPosition(), 50));
  setInterval(() => {
    if (chapter && chapterKey() !== chapter) scheduleDetect(0);
    else if (running()) reportPosition(true);
  }, 2000);
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') reportPosition(true);
  });

  loadSettings().then(() => scheduleDetect(300));
})();
