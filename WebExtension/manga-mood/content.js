// KeyToMusic Manga Mood — Content Script
// Detects manga pages as they enter the viewport, captures them,
// and sends to KeyToMusic for mood analysis.
// Handles lazy-loaded images and dynamically added pages (infinite scroll / chapter loading).
// Pre-calculates moods for all loaded images in background for instant playback on scroll.

(() => {
  // Guard against double injection
  if (window.__ktmMoodInjected) return;
  window.__ktmMoodInjected = true;

  // --- Site profiles ---
  // Each profile defines selectors for a manga reader site.
  // The content script tries each in order, uses the first match.
  const SITE_PROFILES = [
    {
      name: 'WordPress Flavor (Sushiscan, Lelmanga, etc.)',
      match: () => !!document.querySelector('#readerarea'),
      readerSelector: '#readerarea',
      imageSelector: '#readerarea img.ts-main-image',
      isLoaded: (img) => img.classList.contains('lazyloaded') || (img.complete && img.naturalWidth > 0 && img.src && !img.src.endsWith('.svg')),
      getIndex: (img) => img.dataset.index,
    },
    {
      name: 'Generic (large images in scrollable container)',
      match: () => true,
      readerSelector: null,
      imageSelector: null,
      isLoaded: (img) => img.complete && img.naturalWidth > 300,
      getIndex: (img) => null,
      autoDetect: true,
    },
  ];

  // --- Safe messaging (survives extension reload) ---

  let contextValid = true;

  function safeSendMessage(msg) {
    if (!contextValid) return;
    try {
      chrome.runtime.sendMessage(msg).catch(handleInvalidContext);
    } catch {
      handleInvalidContext();
    }
  }

  function handleInvalidContext() {
    contextValid = false;
    // Extension was reloaded — stop all observers, listeners are now dead
    if (intersectionObserver) intersectionObserver.disconnect();
    if (mutationObserver) mutationObserver.disconnect();
    window.removeEventListener('scroll', onScroll);
    if (chapterCheckInterval) clearInterval(chapterCheckInterval);
    window.__ktmMoodInjected = false; // Allow re-injection
  }

  // --- State ---
  let activeProfile = null;
  let intersectionObserver = null;
  let mutationObserver = null;
  const analyzedImages = new WeakSet();
  const pendingImages = new WeakSet();
  const ANALYZE_COOLDOWN = 500; // ms between queuing analyses
  let lastAnalyzeTime = 0;
  let lastTriggeredIndex = null; // Track current page to detect navigation

  // --- Pre-analysis state ---
  const precalculatedPages = new Set(); // page indices already sent for precalc
  const cachedMoods = new Map();        // pageIndex → mood (mirror of backend cache)
  let currentChapter = null;            // current chapter pathname
  let chapterCheckInterval = null;      // polling for SPA chapter changes

  // --- Chapter extraction ---

  function extractChapterInfo() {
    return { chapter: window.location.pathname.replace(/\/$/, '') };
  }

  // --- Detect site profile ---

  function detectProfile() {
    for (const profile of SITE_PROFILES) {
      if (profile.autoDetect) continue;
      if (profile.match()) return profile;
    }
    return null;
  }

  // For generic auto-detect: find large images that look like manga pages
  function autoDetectImages() {
    const imgs = document.querySelectorAll('img');
    return Array.from(imgs).filter(img =>
      img.complete &&
      img.naturalWidth > 300 &&
      img.naturalHeight > 400 &&
      img.naturalHeight > img.naturalWidth * 1.2 // portrait orientation = manga page
    );
  }

  // --- Image capture via canvas ---

  const MAX_SIZE = 672; // Match server resize

  function captureImageToBase64(img) {
    try {
      const { naturalWidth: w, naturalHeight: h } = img;
      if (!w || !h) return null;

      // Compute resize dimensions
      const scale = Math.min(MAX_SIZE / w, MAX_SIZE / h, 1);
      const dw = Math.round(w * scale);
      const dh = Math.round(h * scale);

      const canvas = document.createElement('canvas');
      canvas.width = dw;
      canvas.height = dh;
      const ctx = canvas.getContext('2d');
      ctx.drawImage(img, 0, 0, dw, dh);

      // toDataURL — may throw SecurityError on cross-origin images
      const dataUrl = canvas.toDataURL('image/jpeg', 0.80);
      // Strip prefix "data:image/jpeg;base64,"
      return dataUrl.split(',')[1];
    } catch (e) {
      // Cross-origin tainted canvas
      return null;
    }
  }

  // --- Send to background for analysis ---

  function requestAnalysis(img, pageIndex) {
    if (analyzedImages.has(img) || pendingImages.has(img)) return;

    // Cooldown
    const now = Date.now();
    if (now - lastAnalyzeTime < ANALYZE_COOLDOWN) return;
    lastAnalyzeTime = now;

    pendingImages.add(img);
    showSpinner(img);

    // Try canvas capture first
    const base64 = captureImageToBase64(img);

    const message = {
      type: 'analyze',
      pageIndex: pageIndex ?? img.dataset.index ?? null,
      imgSelector: buildImgSelector(img),
    };

    if (base64) {
      message.base64 = base64;
    } else {
      // Fallback: send URL for background to fetch
      message.url = img.src || img.dataset.src;
    }

    safeSendMessage(message);
  }

  // Build a selector to re-find this image later (for mood result callback)
  function buildImgSelector(img) {
    if (img.dataset.index !== undefined) {
      return `img[data-index="${img.dataset.index}"]`;
    }
    if (img.id) return `#${img.id}`;
    // Fallback: src-based
    const src = img.src || img.dataset.src;
    if (src) return `img[src="${CSS.escape(src)}"], img[data-src="${CSS.escape(src)}"]`;
    return null;
  }

  // --- Pre-analysis pipeline ---

  function getPageIndex(img) {
    if (!activeProfile) return null;
    const idx = activeProfile.getIndex(img);
    if (idx !== null && idx !== undefined) return Number(idx);
    // Fallback: order-based index from all profile images
    const images = activeProfile.imageSelector
      ? Array.from(document.querySelectorAll(activeProfile.imageSelector))
      : autoDetectImages();
    const pos = images.indexOf(img);
    return pos >= 0 ? pos : null;
  }

  function sendForPrecalculation(img, pageIndex, chapterInfo) {
    if (pageIndex === null || pageIndex === undefined) return;
    if (precalculatedPages.has(pageIndex)) return;

    const base64 = captureImageToBase64(img);
    if (!base64) return; // Cross-origin — can't precalculate

    precalculatedPages.add(pageIndex);

    safeSendMessage({
      type: 'precalculate',
      base64,
      chapter: chapterInfo.chapter,
      page: pageIndex,
    });
  }

  function setupPreAnalysis(images) {
    const chapterInfo = extractChapterInfo();

    for (const img of images) {
      const pageIndex = getPageIndex(img);
      if (pageIndex === null) continue;

      if (activeProfile.isLoaded(img)) {
        sendForPrecalculation(img, pageIndex, chapterInfo);
      } else {
        // Wait for lazy load
        const handler = () => {
          img.removeEventListener('load', handler);
          sendForPrecalculation(img, getPageIndex(img), chapterInfo);
        };
        img.addEventListener('load', handler);
      }
    }
  }

  // --- Mood badge overlay ---

  const MOOD_COLORS = {
    epic_battle: { bg: '#EF4444', label: 'Epic Battle' },
    tension: { bg: '#F97316', label: 'Tension' },
    sadness: { bg: '#3B82F6', label: 'Sadness' },
    comedy: { bg: '#FACC15', label: 'Comedy' },
    romance: { bg: '#EC4899', label: 'Romance' },
    horror: { bg: '#7C3AED', label: 'Horror' },
    peaceful: { bg: '#22C55E', label: 'Peaceful' },
    emotional_climax: { bg: '#F43F5E', label: 'Climax' },
    mystery: { bg: '#6366F1', label: 'Mystery' },
    chase_action: { bg: '#F59E0B', label: 'Chase' },
  };

  function showMoodBadge(img, mood, dimmed = false) {
    removeBadge(img);
    removeSpinner(img);

    const info = MOOD_COLORS[mood] || { bg: '#6B7280', label: mood };

    // Ensure parent is positioned for absolute overlay
    const parent = img.parentElement;
    if (parent && getComputedStyle(parent).position === 'static') {
      parent.style.position = 'relative';
    }

    const badge = document.createElement('div');
    badge.className = 'ktm-mood-badge';
    badge.setAttribute('data-ktm-badge', '');
    badge.innerHTML = `<span class="ktm-mood-dot" style="background:${info.bg}"></span>${info.label}`;
    badge.style.cssText = `
      position: absolute;
      top: 8px;
      right: 8px;
      display: flex;
      align-items: center;
      gap: 6px;
      padding: 4px 10px;
      border-radius: 9999px;
      background: rgba(0,0,0,${dimmed ? '0.4' : '0.7'});
      backdrop-filter: blur(4px);
      color: #fff;
      font: 600 12px/1 system-ui, sans-serif;
      z-index: 9999;
      pointer-events: none;
      opacity: 0;
      transform: translateY(-4px);
      transition: opacity 0.3s, transform 0.3s;
    `;

    const container = parent || img.parentElement;
    if (container) {
      container.appendChild(badge);
      // Trigger animation
      requestAnimationFrame(() => {
        badge.style.opacity = dimmed ? '0.5' : '1';
        badge.style.transform = 'translateY(0)';
      });
    }

    // Store reference on image
    img.__ktmBadge = badge;
  }

  function showSpinner(img) {
    removeSpinner(img);

    const parent = img.parentElement;
    if (parent && getComputedStyle(parent).position === 'static') {
      parent.style.position = 'relative';
    }

    const spinner = document.createElement('div');
    spinner.className = 'ktm-mood-spinner';
    spinner.setAttribute('data-ktm-spinner', '');
    spinner.style.cssText = `
      position: absolute;
      top: 8px;
      right: 8px;
      width: 20px;
      height: 20px;
      border: 2px solid rgba(255,255,255,0.3);
      border-top-color: #818CF8;
      border-radius: 50%;
      z-index: 9999;
      pointer-events: none;
      animation: ktm-spin 0.8s linear infinite;
    `;

    // Inject keyframe if not already done
    if (!document.querySelector('#ktm-styles')) {
      const style = document.createElement('style');
      style.id = 'ktm-styles';
      style.textContent = `
        @keyframes ktm-spin { to { transform: rotate(360deg); } }
        .ktm-mood-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
      `;
      document.head.appendChild(style);
    }

    const container = parent || img.parentElement;
    if (container) container.appendChild(spinner);
    img.__ktmSpinner = spinner;
  }

  function removeSpinner(img) {
    if (img.__ktmSpinner) {
      img.__ktmSpinner.remove();
      img.__ktmSpinner = null;
    }
  }

  function removeBadge(img) {
    if (img.__ktmBadge) {
      img.__ktmBadge.remove();
      img.__ktmBadge = null;
    }
  }

  // --- Find image by page index ---

  function findImageByPageIndex(pageIndex) {
    if (!activeProfile) return null;
    const images = activeProfile.imageSelector
      ? Array.from(document.querySelectorAll(activeProfile.imageSelector))
      : autoDetectImages();
    for (const img of images) {
      if (getPageIndex(img) === pageIndex) return img;
    }
    return null;
  }

  // --- IntersectionObserver: detect visible manga pages ---

  function setupIntersectionObserver(images) {
    if (intersectionObserver) intersectionObserver.disconnect();

    intersectionObserver = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;

        const img = entry.target;
        if (analyzedImages.has(img) || pendingImages.has(img)) continue;

        const profile = activeProfile;
        if (!profile) continue;

        if (profile.isLoaded(img)) {
          requestAnalysis(img, profile.getIndex(img));
        } else {
          // Wait for lazy load
          const handler = () => {
            img.removeEventListener('load', handler);
            if (!analyzedImages.has(img) && !pendingImages.has(img)) {
              requestAnalysis(img, profile.getIndex(img));
            }
          };
          img.addEventListener('load', handler);
        }
      }
    }, {
      // Pre-analyze: trigger 300px before image enters viewport
      rootMargin: '300px 0px 300px 0px',
      threshold: 0.1,
    });

    images.forEach(img => intersectionObserver.observe(img));
  }

  // --- MutationObserver: detect dynamically added images ---

  function setupMutationObserver(readerEl) {
    if (mutationObserver) mutationObserver.disconnect();

    mutationObserver = new MutationObserver((mutations) => {
      let newImages = [];

      for (const mutation of mutations) {
        for (const node of mutation.addedNodes) {
          if (node.nodeType !== Node.ELEMENT_NODE) continue;

          // Direct img additions
          if (node.tagName === 'IMG' && matchesProfileImage(node)) {
            newImages.push(node);
          }

          // Img inside added containers
          const imgs = node.querySelectorAll?.(activeProfile.imageSelector || 'img');
          if (imgs) {
            for (const img of imgs) {
              if (matchesProfileImage(img)) newImages.push(img);
            }
          }
        }
      }

      if (newImages.length > 0) {
        if (intersectionObserver) {
          newImages.forEach(img => intersectionObserver.observe(img));
        }
        // Also start pre-analysis for new images
        setupPreAnalysis(newImages);
      }
    });

    mutationObserver.observe(readerEl, { childList: true, subtree: true });
  }

  function matchesProfileImage(img) {
    if (!activeProfile) return false;
    if (activeProfile.imageSelector) {
      return img.matches(activeProfile.imageSelector.replace(/^.*\s/, ''));
    }
    return img.naturalWidth > 300;
  }

  // --- Find the most visible manga image ---

  function getCurrentVisibleImage() {
    if (!activeProfile) return null;

    let images;
    if (activeProfile.imageSelector) {
      images = Array.from(document.querySelectorAll(activeProfile.imageSelector));
    } else {
      images = autoDetectImages();
    }

    if (images.length === 0) return null;

    const vpCenter = window.innerHeight / 2;
    let best = null;
    let bestDist = Infinity;

    for (const img of images) {
      if (!activeProfile.isLoaded(img)) continue;
      const rect = img.getBoundingClientRect();
      // Skip images completely off-screen
      if (rect.bottom < 0 || rect.top > window.innerHeight) continue;
      const imgCenter = rect.top + rect.height / 2;
      const dist = Math.abs(imgCenter - vpCenter);
      if (dist < bestDist) {
        bestDist = dist;
        best = img;
      }
    }

    return best;
  }

  function captureImageThumbnail(img, maxSize) {
    try {
      const { naturalWidth: w, naturalHeight: h } = img;
      if (!w || !h) return null;
      const scale = Math.min(maxSize / w, maxSize / h, 1);
      const dw = Math.round(w * scale);
      const dh = Math.round(h * scale);
      const canvas = document.createElement('canvas');
      canvas.width = dw;
      canvas.height = dh;
      canvas.getContext('2d').drawImage(img, 0, 0, dw, dh);
      return canvas.toDataURL('image/jpeg', 0.75);
    } catch {
      return null;
    }
  }

  // --- Listen for messages from background / popup ---

  chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
    if (msg.type === 'mood_result' && msg.imgSelector) {
      const img = document.querySelector(msg.imgSelector);
      if (img) {
        analyzedImages.add(img);
        pendingImages.delete(img);
        img.__ktmMood = msg.mood;
        // Update lastTriggeredIndex so scroll tracker doesn't re-trigger immediately
        lastTriggeredIndex = activeProfile?.getIndex(img) ?? img.src ?? null;
        showMoodBadge(img, msg.mood);
      }
    }

    if (msg.type === 'mood_error' && msg.imgSelector) {
      const img = document.querySelector(msg.imgSelector);
      if (img) {
        pendingImages.delete(img);
        removeSpinner(img);
      }
    }

    // Pre-calculation result — store in local cache mirror, show dimmed badge
    if (msg.type === 'precalc_result') {
      cachedMoods.set(msg.page, msg.mood);
      const img = findImageByPageIndex(msg.page);
      if (img) {
        img.__ktmMood = msg.mood;
        showMoodBadge(img, msg.mood, true); // dimmed = pre-calculated, not yet triggered
      }
    }

    // Lookup result — confirmation that backend triggered the mood event
    if (msg.type === 'lookup_result' && msg.hit) {
      const img = findImageByPageIndex(msg.page);
      if (img) {
        analyzedImages.add(img);
        img.__ktmMood = msg.mood;
        showMoodBadge(img, msg.mood, false); // full opacity = triggered
      }
    }

    if (msg.type === 'get_current_image') {
      const img = getCurrentVisibleImage();
      if (!img) {
        sendResponse({ found: false });
        return;
      }

      const thumbnail = captureImageThumbnail(img, 200);
      const pageIndex = activeProfile?.getIndex(img) ?? null;
      const src = img.src || img.dataset?.src || null;

      const mood = img.__ktmMood || null;

      sendResponse({
        found: true,
        thumbnail,
        src,
        pageIndex,
        mood,
        analyzed: analyzedImages.has(img),
        pending: pendingImages.has(img),
      });
      return;
    }
  });

  // --- Chapter change detection ---

  function onChapterChange() {
    const newChapter = extractChapterInfo().chapter;
    if (newChapter === currentChapter) return;

    console.log(`[KeyToMusic Mood] Chapter changed: ${currentChapter} → ${newChapter}`);
    currentChapter = newChapter;

    // Reset pre-analysis state
    precalculatedPages.clear();
    cachedMoods.clear();
    lastTriggeredIndex = null;

    // Re-init on new content (delay to let SPA render)
    setTimeout(() => {
      // Clear old observers
      if (intersectionObserver) intersectionObserver.disconnect();
      if (mutationObserver) mutationObserver.disconnect();

      // Re-detect profile and images
      activeProfile = detectProfile();
      if (!activeProfile) {
        const imgs = autoDetectImages();
        if (imgs.length >= 3) {
          activeProfile = SITE_PROFILES.find(p => p.autoDetect);
          if (activeProfile) {
            setupIntersectionObserver(imgs);
            setupPreAnalysis(imgs);
          }
        }
        return;
      }

      const images = document.querySelectorAll(activeProfile.imageSelector);
      if (images.length === 0) return;

      const imgArray = Array.from(images);
      setupIntersectionObserver(imgArray);
      setupPreAnalysis(imgArray);

      const readerEl = document.querySelector(activeProfile.readerSelector);
      if (readerEl) setupMutationObserver(readerEl);

      console.log(`[KeyToMusic Mood] Re-initialized for new chapter — ${images.length} pages`);
    }, 500);
  }

  function setupChapterChangeDetection() {
    currentChapter = extractChapterInfo().chapter;

    // SPA navigation via History API
    window.addEventListener('popstate', onChapterChange);

    // Polling fallback for SPAs that don't fire popstate (pushState)
    chapterCheckInterval = setInterval(() => {
      const newChapter = extractChapterInfo().chapter;
      if (newChapter !== currentChapter) {
        onChapterChange();
      }
    }, 2000);
  }

  // --- Init ---

  function init() {
    activeProfile = detectProfile();
    if (!activeProfile) {
      // Try generic auto-detect
      const imgs = autoDetectImages();
      if (imgs.length >= 3) {
        activeProfile = SITE_PROFILES.find(p => p.autoDetect);
        if (activeProfile) {
          setupIntersectionObserver(imgs);
          setupPreAnalysis(imgs);
          // No mutation observer for generic — no known reader container
        }
      }
      setupChapterChangeDetection();
      return;
    }

    const images = document.querySelectorAll(activeProfile.imageSelector);
    if (images.length === 0) {
      setupChapterChangeDetection();
      return;
    }

    const imgArray = Array.from(images);
    setupIntersectionObserver(imgArray);
    setupPreAnalysis(imgArray);

    // Watch for dynamic additions
    const readerEl = document.querySelector(activeProfile.readerSelector);
    if (readerEl) {
      setupMutationObserver(readerEl);
    }

    setupScrollTracker();
    setupChapterChangeDetection();

    console.log(`[KeyToMusic Mood] Active on ${activeProfile.name} — ${images.length} pages found`);
  }

  // --- Scroll tracker: re-trigger mood when navigating back to an analyzed page ---

  let scrollTimer = null;

  function setupScrollTracker() {
    window.addEventListener('scroll', onScroll, { passive: true });
  }

  function onScroll() {
    if (scrollTimer) clearTimeout(scrollTimer);
    scrollTimer = setTimeout(checkCurrentPage, 250);
  }

  function checkCurrentPage() {
    const img = getCurrentVisibleImage();
    if (!img) return;

    const index = activeProfile?.getIndex(img) ?? img.src ?? null;
    if (index === null || index === lastTriggeredIndex) return;

    lastTriggeredIndex = index;

    // Check local cache mirror first — instant lookup via backend
    const pageIndex = getPageIndex(img);
    if (pageIndex !== null && cachedMoods.has(pageIndex)) {
      const chapterInfo = extractChapterInfo();
      safeSendMessage({
        type: 'lookup',
        chapter: chapterInfo.chapter,
        page: pageIndex,
      });
      return;
    }

    // Fallback: if already analyzed with a cached mood, re-trigger it
    if (img.__ktmMood) {
      safeSendMessage({
        type: 're_trigger',
        mood: img.__ktmMood,
        pageIndex: index,
      });
    }
  }

  // Run init — page may still be loading content
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }

  // Retry after a short delay for SPAs that render lazily
  setTimeout(() => {
    if (!activeProfile || !document.querySelectorAll(activeProfile?.imageSelector || 'nope').length) {
      init();
    }
  }, 2000);
})();
