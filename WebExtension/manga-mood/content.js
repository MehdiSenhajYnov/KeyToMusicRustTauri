// KeyToMusic Manga Mood — Content Script
// Detects the currently visible manga page, asks the backend for a cached mood,
// streams nearby pages into the chapter cache, and falls back to a context-aware
// visible-page window analysis when needed.

(() => {
  // Guard against double injection
  if (window.__ktmMoodInjected) return;
  window.__ktmMoodInjected = true;
  const ext = globalThis.browser ?? globalThis.chrome;

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
      ext.runtime.sendMessage(msg).catch(handleInvalidContext);
    } catch {
      handleInvalidContext();
    }
  }

  async function safeRequestMessage(msg) {
    if (!contextValid) return null;
    try {
      return await ext.runtime.sendMessage(msg);
    } catch {
      handleInvalidContext();
      return null;
    }
  }

  function handleInvalidContext() {
    contextValid = false;
    // Extension was reloaded — stop all observers, listeners are now dead
    if (intersectionObserver) intersectionObserver.disconnect();
    if (mutationObserver) mutationObserver.disconnect();
    window.removeEventListener('scroll', onScroll);
    window.removeEventListener('popstate', onChapterChange);
    if (scrollTimer) clearTimeout(scrollTimer);
    if (pendingVisibleAnalysisTimer) clearTimeout(pendingVisibleAnalysisTimer);
    if (chapterReinitTimer) clearTimeout(chapterReinitTimer);
    if (initRetryTimer) clearTimeout(initRetryTimer);
    if (chapterCheckInterval) clearInterval(chapterCheckInterval);
    resetPreloadState();
    window.__ktmMoodInjected = false; // Allow re-injection
  }

  // --- State ---
  let activeProfile = null;
  let intersectionObserver = null;
  let mutationObserver = null;
  const analyzedImages = new WeakSet();
  const pendingImages = new WeakSet();
  const ANALYZE_COOLDOWN = 500; // ms between queuing analyses
  const VISIBLE_ANALYZE_DELAY = 700; // only run live analysis if the page stays visible a bit
  const LIVE_ERROR_COOLDOWN = 5000;
  const ANALYZED_AHEAD_TARGET = 20;
  const ANALYZED_BEHIND_TARGET = 10;
  const PIPELINE_CONTEXT_MARGIN = 4; // A published page needs raw pages up to +/-4 around it.
  const IN_PAGE_PRELOAD_CONCURRENCY = 8;
  const PRELOAD_RETRY_COOLDOWN = 15000;
  let lastAnalyzeTime = 0;
  let lastAnalyzeTarget = null;
  let lastTriggeredIndex = null; // Track current page to detect navigation
  let pendingVisibleAnalysisTimer = null;
  let pendingVisibleAnalysisKey = null;
  let lookupInFlightKey = null;
  let lastFocusState = null;
  let lastFocusedPageIndex = null;
  let lastFocusDirection = 1;
  let lastLookupKey = null;
  let lastLookupAt = 0;
  const LOOKUP_COOLDOWN = 800;
  const CHAPTER_FEED_COOLDOWN = 10000;
  const chapterFeedSentAt = new Map();
  const liveErrorSentAt = new Map();
  let currentChapter = null;            // current chapter pathname
  let chapterCheckInterval = null;      // polling for SPA chapter changes
  let chapterChangeBound = false;
  let scrollTrackerBound = false;
  let chapterReinitTimer = null;
  let initRetryTimer = null;
  let preloadPumpTimer = null;
  let preloadReadyFlushTimer = null;
  let preloadContainer = null;
  let preloadInFlight = 0;
  const preloadedPages = new Map();

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

  function getOrderedImages() {
    if (!activeProfile) return [];
    if (activeProfile.imageSelector) {
      return Array.from(document.querySelectorAll(activeProfile.imageSelector));
    }
    return autoDetectImages();
  }

  function ensurePreloadContainer() {
    if (preloadContainer?.isConnected) return preloadContainer;
    preloadContainer = document.createElement('div');
    preloadContainer.id = 'ktm-preload-buffer';
    preloadContainer.setAttribute('aria-hidden', 'true');
    preloadContainer.style.cssText = `
      position: fixed;
      left: -10000px;
      top: -10000px;
      width: 1px;
      height: 1px;
      overflow: hidden;
      opacity: 0;
      pointer-events: none;
      z-index: -1;
    `;
    (document.body || document.documentElement).appendChild(preloadContainer);
    return preloadContainer;
  }

  function cleanupPreloadedPageEntry(entry) {
    if (!entry) return;
    if (typeof entry.cleanup === 'function') {
      entry.cleanup();
      entry.cleanup = null;
    }
    if (entry.mode === 'source') {
      entry.img = null;
      return;
    }
    if (entry.img) {
      entry.img.onload = null;
      entry.img.onerror = null;
      entry.img.removeAttribute('srcset');
      entry.img.removeAttribute('sizes');
      entry.img.src = '';
      entry.img.remove();
      entry.img = null;
    }
  }

  function resetPreloadState() {
    if (preloadPumpTimer) {
      clearTimeout(preloadPumpTimer);
      preloadPumpTimer = null;
    }
    if (preloadReadyFlushTimer) {
      clearTimeout(preloadReadyFlushTimer);
      preloadReadyFlushTimer = null;
    }
    preloadInFlight = 0;
    for (const entry of preloadedPages.values()) {
      cleanupPreloadedPageEntry(entry);
    }
    preloadedPages.clear();
    if (preloadContainer) {
      preloadContainer.remove();
      preloadContainer = null;
    }
  }

  function collectAttributeValue(img, names) {
    for (const name of names) {
      const value = img.getAttribute(name);
      if (value && value.trim()) return value.trim();
    }
    return null;
  }

  function isPlaceholderSource(value) {
    if (!value || typeof value !== 'string') return true;
    const trimmed = value.trim();
    if (!trimmed) return true;
    const lower = trimmed.toLowerCase();
    return (
      lower === 'about:blank'
      || lower.startsWith('data:image/svg')
      || lower.includes('blank.gif')
      || lower.includes('/placeholder')
      || lower.endsWith('.svg')
    );
  }

  function getImageSourceDescriptor(img) {
    if (!img) return null;

    const src = [
      collectAttributeValue(img, ['data-src', 'data-lazy-src', 'data-original', 'data-pagespeed-lazy-src', 'data-cfsrc']),
      img.dataset?.src || null,
      img.dataset?.lazySrc || null,
      img.dataset?.original || null,
      img.currentSrc || null,
      collectAttributeValue(img, ['src']),
    ].find((candidate) => candidate && !isPlaceholderSource(candidate)) || null;

    const srcset = [
      collectAttributeValue(img, ['data-srcset', 'data-lazy-srcset']),
      img.dataset?.srcset || null,
      img.dataset?.lazySrcset || null,
      collectAttributeValue(img, ['srcset']),
    ].find((candidate) => candidate && candidate.trim()) || null;

    const sizes = [
      collectAttributeValue(img, ['data-sizes', 'sizes']),
      img.dataset?.sizes || null,
    ].find((candidate) => candidate && candidate.trim()) || null;

    if (!src && !srcset) return null;

    return {
      src,
      srcset,
      sizes,
      referrerPolicy: img.referrerPolicy || collectAttributeValue(img, ['referrerpolicy']) || '',
      crossOrigin: img.crossOrigin || collectAttributeValue(img, ['crossorigin']) || '',
    };
  }

  function isPreloadedCaptureReady(entry) {
    return !!entry?.img && entry.status === 'loaded' && entry.img.complete && entry.img.naturalWidth > 0 && entry.img.naturalHeight > 0;
  }

  function getCaptureImageForPageIndex(pageIndex) {
    const images = getOrderedImages();
    const sourceImg = images[pageIndex];
    if (!sourceImg) return null;
    if (activeProfile.isLoaded(sourceImg)) return sourceImg;

    const entry = preloadedPages.get(pageIndex);
    if (isPreloadedCaptureReady(entry)) {
      return entry.img;
    }
    return null;
  }

  function schedulePreloadPump() {
    if (preloadPumpTimer) return;
    preloadPumpTimer = setTimeout(() => {
      preloadPumpTimer = null;
      if (lastFocusedPageIndex !== null && lastFocusedPageIndex !== undefined) {
        pumpInPagePreload(lastFocusedPageIndex);
      }
    }, 50);
  }

  function schedulePreloadReadyFlush() {
    if (preloadReadyFlushTimer) return;
    preloadReadyFlushTimer = setTimeout(() => {
      preloadReadyFlushTimer = null;
      if (lastFocusedPageIndex === null || lastFocusedPageIndex === undefined) return;
      void queueChapterFeedAroundPage(lastFocusedPageIndex);
      const currentImg = getCurrentVisibleImage();
      if (!currentImg) return;
      const currentPageIndex = getPageIndex(currentImg);
      if (currentPageIndex === null || currentPageIndex === undefined) return;
      if (analyzedImages.has(currentImg) || pendingImages.has(currentImg)) return;

      const totalPages = getOrderedImages().length;
      const requiredPages = buildRequiredWindowPages(currentPageIndex, totalPages);
      if (requiredPages.every((page) => getCaptureImageForPageIndex(page))) {
        scheduleVisibleAnalysis(currentImg, currentPageIndex);
      }
    }, 120);
  }

  function finalizePreloadEntry(pageIndex, entry, status) {
    const current = preloadedPages.get(pageIndex);
    if (!current || current !== entry) return;
    current.status = status;
    if (status === 'failed') {
      current.failedAt = Date.now();
    }
    preloadInFlight = Math.max(0, preloadInFlight - 1);
    if (status === 'loaded') {
      schedulePreloadReadyFlush();
    }
    schedulePreloadPump();
  }

  function applySourceDescriptorToImage(img, descriptor) {
    if (!img || !descriptor) return;
    img.decoding = 'async';
    img.loading = 'eager';
    if ('fetchPriority' in img) {
      img.fetchPriority = 'high';
    }
    if (descriptor.referrerPolicy && !img.referrerPolicy) {
      img.referrerPolicy = descriptor.referrerPolicy;
    }
    if (descriptor.crossOrigin && !img.crossOrigin) {
      img.crossOrigin = descriptor.crossOrigin;
    }
    if (descriptor.sizes && img.getAttribute('sizes') !== descriptor.sizes) {
      img.setAttribute('sizes', descriptor.sizes);
    }
    if (descriptor.srcset && img.getAttribute('srcset') !== descriptor.srcset) {
      img.setAttribute('srcset', descriptor.srcset);
    }
    const currentSrc = img.getAttribute('src');
    if (descriptor.src && (isPlaceholderSource(currentSrc) || !currentSrc)) {
      img.setAttribute('src', descriptor.src);
    }
    img.classList.remove('lazyload', 'lazyloading');
    img.classList.add('ktm-preload-requested');
  }

  function startHiddenPreload(pageIndex, descriptor, descriptorKey) {
    const preloadRoot = ensurePreloadContainer();
    const preloadImg = new Image();
    preloadImg.decoding = 'async';
    preloadImg.loading = 'eager';
    if ('fetchPriority' in preloadImg) {
      preloadImg.fetchPriority = 'low';
    }
    if (descriptor.referrerPolicy) preloadImg.referrerPolicy = descriptor.referrerPolicy;
    if (descriptor.crossOrigin) preloadImg.crossOrigin = descriptor.crossOrigin;
    preloadImg.style.cssText = 'display:block;width:1px;height:1px;opacity:0;pointer-events:none;';

    const entry = {
      pageIndex,
      descriptorKey,
      status: 'loading',
      failedAt: 0,
      img: preloadImg,
      mode: 'hidden',
      cleanup: null,
    };
    preloadedPages.set(pageIndex, entry);
    preloadInFlight += 1;

    preloadImg.onload = () => finalizePreloadEntry(pageIndex, entry, 'loaded');
    preloadImg.onerror = () => finalizePreloadEntry(pageIndex, entry, 'failed');

    if (descriptor.sizes) preloadImg.sizes = descriptor.sizes;
    if (descriptor.srcset) preloadImg.srcset = descriptor.srcset;
    if (descriptor.src) preloadImg.src = descriptor.src;

    preloadRoot.appendChild(preloadImg);
    return true;
  }

  function startInPagePreload(pageIndex, sourceImg) {
    const descriptor = getImageSourceDescriptor(sourceImg);
    if (!descriptor) return false;

    const descriptorKey = JSON.stringify({
      src: descriptor.src || '',
      srcset: descriptor.srcset || '',
      sizes: descriptor.sizes || '',
    });

    const existing = preloadedPages.get(pageIndex);
    if (existing) {
      if (existing.descriptorKey === descriptorKey) {
        if (existing.status === 'loading' || existing.status === 'loaded') {
          return true;
        }
        if (existing.status === 'failed' && Date.now() - (existing.failedAt || 0) < PRELOAD_RETRY_COOLDOWN) {
          return false;
        }
      }
      cleanupPreloadedPageEntry(existing);
    }

    if (activeProfile.isLoaded(sourceImg)) {
      preloadedPages.set(pageIndex, {
        pageIndex,
        descriptorKey,
        status: 'loaded',
        failedAt: 0,
        img: sourceImg,
        mode: 'source',
        cleanup: null,
      });
      schedulePreloadReadyFlush();
      return true;
    }

    const entry = {
      pageIndex,
      descriptorKey,
      status: 'loading',
      failedAt: 0,
      img: sourceImg,
      mode: 'source',
      cleanup: null,
    };
    preloadedPages.set(pageIndex, entry);
    preloadInFlight += 1;

    const cleanupListeners = () => {
      sourceImg.removeEventListener('load', onLoad);
      sourceImg.removeEventListener('error', onError);
    };

    const onLoad = () => {
      cleanupListeners();
      finalizePreloadEntry(pageIndex, entry, 'loaded');
    };

    const onError = () => {
      cleanupListeners();
      preloadInFlight = Math.max(0, preloadInFlight - 1);
      preloadedPages.delete(pageIndex);
      startHiddenPreload(pageIndex, descriptor, descriptorKey);
    };

    entry.cleanup = cleanupListeners;
    sourceImg.addEventListener('load', onLoad, { once: true });
    sourceImg.addEventListener('error', onError, { once: true });
    applySourceDescriptorToImage(sourceImg, descriptor);

    if (activeProfile.isLoaded(sourceImg)) {
      cleanupListeners();
      finalizePreloadEntry(pageIndex, entry, 'loaded');
    }

    return true;
  }

  function pumpInPagePreload(pageIndex) {
    if (!activeProfile || !activeProfile.imageSelector) return;
    if (pageIndex === null || pageIndex === undefined) return;

    const images = getOrderedImages();
    if (images.length === 0) return;

    for (const targetIndex of buildLoadWindowPages(pageIndex, images.length)) {
      if (preloadInFlight >= IN_PAGE_PRELOAD_CONCURRENCY) break;

      const sourceImg = images[targetIndex];
      if (!sourceImg) continue;
      if (activeProfile.isLoaded(sourceImg)) continue;
      if (getCaptureImageForPageIndex(targetIndex)) continue;

      startInPagePreload(targetIndex, sourceImg);
    }
  }

  function buildRequiredWindowPages(pageIndex, totalPages) {
    const maxPage = Math.max(0, totalPages - 1);
    return [-2, -1, 0, 1, 2].map((offset) => (
      Math.min(Math.max(pageIndex + offset, 0), maxPage)
    ));
  }

  function buildPriorityOrderedPages(pageIndex, totalPages, aheadTarget, behindTarget) {
    const pages = [];
    const seen = new Set();
    const pushPage = (targetIndex) => {
      if (targetIndex < 0 || targetIndex >= totalPages || seen.has(targetIndex)) return;
      seen.add(targetIndex);
      pages.push(targetIndex);
    };

    pushPage(pageIndex);

    let aheadOffset = 1;
    let behindOffset = 1;
    const forwardBias = lastFocusDirection >= 0;

    while (aheadOffset <= aheadTarget || behindOffset <= behindTarget) {
      if (forwardBias) {
        for (let i = 0; i < 2 && aheadOffset <= aheadTarget; i += 1, aheadOffset += 1) {
          pushPage(pageIndex + aheadOffset);
        }
        if (behindOffset <= behindTarget) {
          pushPage(pageIndex - behindOffset);
          behindOffset += 1;
        }
      } else {
        for (let i = 0; i < 2 && behindOffset <= behindTarget; i += 1, behindOffset += 1) {
          pushPage(pageIndex - behindOffset);
        }
        if (aheadOffset <= aheadTarget) {
          pushPage(pageIndex + aheadOffset);
          aheadOffset += 1;
        }
      }
    }

    return pages;
  }

  function buildAnalyzedTargetPages(pageIndex, totalPages) {
    return buildPriorityOrderedPages(
      pageIndex,
      totalPages,
      ANALYZED_AHEAD_TARGET,
      ANALYZED_BEHIND_TARGET
    );
  }

  function buildLoadWindowPages(pageIndex, totalPages) {
    return buildPriorityOrderedPages(
      pageIndex,
      totalPages,
      ANALYZED_AHEAD_TARGET + PIPELINE_CONTEXT_MARGIN,
      ANALYZED_BEHIND_TARGET + PIPELINE_CONTEXT_MARGIN
    );
  }

  function buildVisibleWindowMembers(pageIndex) {
    const images = getOrderedImages();
    if (pageIndex === null || pageIndex === undefined || images.length === 0) return null;

    const totalPages = images.length;
    const members = [];
    const includedPages = new Set();
    for (const offset of [0, -1, 1, -2, 2, -3, 3, -4, 4]) {
      const targetIndex = Math.min(Math.max(pageIndex + offset, 0), totalPages - 1);
      if (includedPages.has(targetIndex)) continue;
      const img = getCaptureImageForPageIndex(targetIndex);
      if (!img) continue;

      const base64 = captureImageToBase64(img);
      if (!base64) {
        continue;
      }

      includedPages.add(targetIndex);
      members.push({
        page: targetIndex,
        base64,
      });
    }

    const requiredPages = buildRequiredWindowPages(pageIndex, totalPages);
    if (!requiredPages.every((page) => includedPages.has(page))) {
      return null;
    }

    return {
      totalPages,
      members,
    };
  }

  function buildChapterFeedPages(chapter, pageIndex) {
    const images = getOrderedImages();
    if (!chapter || pageIndex === null || pageIndex === undefined || images.length === 0) return null;

    const totalPages = images.length;
    const pages = [];
    for (const targetIndex of buildLoadWindowPages(pageIndex, totalPages)) {
      const pageKey = `${chapter}::${targetIndex}`;
      const lastSentAt = chapterFeedSentAt.get(pageKey) || 0;
      if (Date.now() - lastSentAt < CHAPTER_FEED_COOLDOWN) continue;

      const img = getCaptureImageForPageIndex(targetIndex);
      if (!img) continue;

      const image = captureImageToBase64(img);
      if (!image) continue;

      pages.push({
        page: targetIndex,
        image,
      });
    }

    if (pages.length === 0) return null;

    return {
      totalPages,
      pages,
    };
  }

  async function queueChapterFeedAroundPage(pageIndex) {
    const chapter = extractChapterInfo().chapter;
    const payload = buildChapterFeedPages(chapter, pageIndex);
    if (!chapter || !payload) return;
    const pages = payload.pages;

    const result = await safeRequestMessage({
      type: 'chapter_pages',
      chapter,
      totalPages: payload.totalPages,
      pages,
    });
    if (!result) return;

    const sentAt = Date.now();
    const acceptedPages = new Set(
      Array.isArray(result.pages) ? result.pages.map((page) => Number(page)).filter(Number.isFinite) : []
    );
    for (const entry of pages) {
      if (!acceptedPages.has(entry.page)) continue;
      chapterFeedSentAt.set(`${chapter}::${entry.page}`, sentAt);
    }
  }

  function updateChapterFocus(pageIndex, totalPages = null) {
    const chapter = extractChapterInfo().chapter;
    if (!chapter || pageIndex === null || pageIndex === undefined) return;
    if (lastFocusedPageIndex !== null && pageIndex !== lastFocusedPageIndex) {
      const delta = pageIndex - lastFocusedPageIndex;
      if (delta !== 0) {
        lastFocusDirection = Math.sign(delta);
      }
    }
    lastFocusedPageIndex = pageIndex;
    const focusState = {
      chapter,
      pageIndex,
      direction: lastFocusDirection,
      totalPages: totalPages ?? null,
    };
    if (
      lastFocusState &&
      lastFocusState.chapter === focusState.chapter &&
      lastFocusState.pageIndex === focusState.pageIndex &&
      lastFocusState.direction === focusState.direction &&
      lastFocusState.totalPages === focusState.totalPages
    ) {
      return;
    }
    lastFocusState = focusState;
    safeSendMessage({
      type: 'chapter_focus',
      chapter,
      pageIndex,
      direction: lastFocusDirection,
      totalPages,
    });
  }

  // --- Send to background for analysis ---

  function requestVisibleWindowAnalysis(img, pageIndex) {
    if (analyzedImages.has(img) || pendingImages.has(img)) return;
    const chapter = extractChapterInfo().chapter;
    const targetKey = `${chapter}::${pageIndex ?? img.dataset.index ?? img.currentSrc ?? img.src ?? "unknown"}`;
    const lastFailureAt = liveErrorSentAt.get(targetKey) || 0;
    if (Date.now() - lastFailureAt < LIVE_ERROR_COOLDOWN) {
      safeSendMessage({
        type: 'visible_analysis_blocked',
        pageIndex,
        note: 'Previous visible-window attempt failed recently. Waiting a bit before retrying.',
      });
      return;
    }

    // Cooldown
    const now = Date.now();
    if (now - lastAnalyzeTime < ANALYZE_COOLDOWN && lastAnalyzeTarget === targetKey) return;
    lastAnalyzeTime = now;
    lastAnalyzeTarget = targetKey;

    const windowPayload = buildVisibleWindowMembers(pageIndex);
    if (!windowPayload) {
      safeSendMessage({
        type: 'visible_analysis_blocked',
        pageIndex,
        note: 'Winner context not ready: one or more nearby pages in the 9-page local neighborhood are not loaded or cannot be captured in base64.',
      });
      return;
    }

    pendingImages.add(img);
    showSpinner(img);

    const message = {
      type: 'analyze_window',
      pageIndex: pageIndex ?? img.dataset.index ?? null,
      imgSelector: buildImgSelector(img),
      chapter,
      totalPages: windowPayload.totalPages,
      members: windowPayload.members,
    };

    safeSendMessage(message);
  }

  function scheduleVisibleAnalysis(img, pageIndex) {
    const chapter = extractChapterInfo().chapter;
    const targetKey = `${chapter}::${pageIndex}`;
    if (pendingVisibleAnalysisKey === targetKey && pendingVisibleAnalysisTimer) return;

    if (pendingVisibleAnalysisTimer) {
      clearTimeout(pendingVisibleAnalysisTimer);
      pendingVisibleAnalysisTimer = null;
    }

    pendingVisibleAnalysisKey = targetKey;
    safeSendMessage({
      type: 'visible_analysis_pending',
      pageIndex,
      note: `Waiting ${VISIBLE_ANALYZE_DELAY}ms to confirm that the visible page is stable.`,
    });
    pendingVisibleAnalysisTimer = setTimeout(() => {
      pendingVisibleAnalysisTimer = null;

      void (async () => {
        const currentImg = getCurrentVisibleImage();
        if (!currentImg) {
          pendingVisibleAnalysisKey = null;
          return;
        }

        const currentPageIndex = getPageIndex(currentImg);
        const currentChapter = extractChapterInfo().chapter;
        const stillCurrent = currentPageIndex === pageIndex && currentChapter === chapter;

        if (!stillCurrent || analyzedImages.has(currentImg) || pendingImages.has(currentImg)) {
          pendingVisibleAnalysisKey = null;
          return;
        }

        const lookup = await requestLookupForVisiblePage(currentImg, currentPageIndex, {
          force: true,
          allowAnalysisFallback: false,
        });
        if (lookup === undefined) {
          pendingVisibleAnalysisKey = null;
          return;
        }
        if (lookup?.hit) {
          pendingVisibleAnalysisKey = null;
          return;
        }

        const latestImg = getCurrentVisibleImage();
        const latestPageIndex = latestImg ? getPageIndex(latestImg) : null;
        const latestChapter = extractChapterInfo().chapter;
        const stillLatest = latestImg === currentImg
          && latestPageIndex === currentPageIndex
          && latestChapter === chapter;

        if (stillLatest && !analyzedImages.has(currentImg) && !pendingImages.has(currentImg)) {
          requestVisibleWindowAnalysis(currentImg, currentPageIndex);
        }

        pendingVisibleAnalysisKey = null;
      })();
    }, VISIBLE_ANALYZE_DELAY);
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

  function getPageIndex(img) {
    if (!activeProfile) return null;
    const idx = activeProfile.getIndex(img);
    if (idx !== null && idx !== undefined) {
      const numericIdx = Number(idx);
      if (Number.isFinite(numericIdx)) return numericIdx;
    }
    // Fallback: order-based index from all profile images
    const images = activeProfile.imageSelector
      ? Array.from(document.querySelectorAll(activeProfile.imageSelector))
      : autoDetectImages();
    const pos = images.indexOf(img);
    return pos >= 0 ? pos : null;
  }

  function getImageUrl(img) {
    return img.currentSrc || img.src || img.dataset.src || img.dataset.lazySrc || null;
  }

  async function requestLookupForVisiblePage(img, pageIndex, options = {}) {
    const { force = false, allowAnalysisFallback = true } = options;
    const chapter = extractChapterInfo().chapter;
    const key = `${chapter}::${pageIndex}`;
    const now = Date.now();

    if (lookupInFlightKey === key) return;
    if (!force && lastLookupKey === key && now - lastLookupAt < LOOKUP_COOLDOWN) return;

    lookupInFlightKey = key;
    lastLookupKey = key;
    lastLookupAt = now;

    try {
      const result = await safeRequestMessage({
        type: 'lookup',
        chapter,
        page: pageIndex,
      });

      if (!result || result.cancelled) return result ?? null;

      const currentImg = getCurrentVisibleImage();
      const currentPageIndex = currentImg ? getPageIndex(currentImg) : null;
      const currentChapter = extractChapterInfo().chapter;
      if (currentPageIndex !== pageIndex || currentChapter !== chapter) {
        return result;
      }

      if (result.hit) {
        analyzedImages.add(img);
        pendingImages.delete(img);
        img.__ktmMood = result.mood;
        lastTriggeredIndex = pageIndex;
        showMoodBadge(img, result.mood, false);
        return result;
      }

      if (allowAnalysisFallback && !analyzedImages.has(img) && !pendingImages.has(img)) {
        scheduleVisibleAnalysis(img, pageIndex);
      }
      return result;
    } finally {
      if (lookupInFlightKey === key) {
        lookupInFlightKey = null;
      }
    }
  }

  // --- Mood badge overlay ---

  const MOOD_COLORS = {
    epic: { bg: '#EF4444', label: 'Epic' },
    tension: { bg: '#F97316', label: 'Tension' },
    sadness: { bg: '#3B82F6', label: 'Sadness' },
    comedy: { bg: '#FACC15', label: 'Comedy' },
    romance: { bg: '#EC4899', label: 'Romance' },
    horror: { bg: '#7C3AED', label: 'Horror' },
    peaceful: { bg: '#22C55E', label: 'Peaceful' },
    mystery: { bg: '#6366F1', label: 'Mystery' },
  };

  function buildMoodBadgeContent(info) {
    const fragment = document.createDocumentFragment();
    const dot = document.createElement('span');
    dot.className = 'ktm-mood-dot';
    dot.style.background = info.bg;
    fragment.appendChild(dot);
    fragment.appendChild(document.createTextNode(info.label));
    return fragment;
  }

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
    badge.appendChild(buildMoodBadgeContent(info));
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

  function clearTrackedImageState(images = Array.from(document.querySelectorAll('img'))) {
    for (const img of images) {
      analyzedImages.delete(img);
      pendingImages.delete(img);
      removeSpinner(img);
      removeBadge(img);
      img.__ktmMood = null;
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
          checkCurrentPage();
        } else {
          // Wait for lazy load
          const handler = () => {
            img.removeEventListener('load', handler);
            if (!analyzedImages.has(img) && !pendingImages.has(img)) {
              checkCurrentPage();
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
        if (lastFocusedPageIndex !== null && lastFocusedPageIndex !== undefined) {
          schedulePreloadPump();
        }
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

  ext.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
    if (msg.type === 'mood_result') {
      const img = (msg.imgSelector ? document.querySelector(msg.imgSelector) : null) || findImageByPageIndex(msg.pageIndex);
      if (img) {
        const pageKey = `${extractChapterInfo().chapter}::${msg.pageIndex}`;
        liveErrorSentAt.delete(pageKey);
        analyzedImages.add(img);
        pendingImages.delete(img);
        img.__ktmMood = msg.mood;
        lastTriggeredIndex = getPageIndex(img);
        showMoodBadge(img, msg.mood);
      }
    }

    if (msg.type === 'mood_error' || msg.type === 'mood_cancelled') {
      const img = (msg.imgSelector ? document.querySelector(msg.imgSelector) : null) || findImageByPageIndex(msg.pageIndex);
      if (img) {
        const pageKey = `${extractChapterInfo().chapter}::${msg.pageIndex}`;
        pendingImages.delete(img);
        removeSpinner(img);
        if (msg.type === 'mood_error') {
          liveErrorSentAt.set(pageKey, Date.now());
        } else {
          liveErrorSentAt.delete(pageKey);
          setTimeout(() => {
            const currentImg = getCurrentVisibleImage();
            if (currentImg === img) {
              checkCurrentPage();
            }
          }, 300);
        }
      }
    }

    // Lookup result — confirmation that backend triggered the mood event
    if (msg.type === 'lookup_result') {
      const img = findImageByPageIndex(msg.page);
      if (!img) return;
      const currentImg = getCurrentVisibleImage();
      const currentPage = currentImg ? getPageIndex(currentImg) : null;
      const isStillCurrentPage = currentPage === msg.page;

      if (msg.hit) {
        analyzedImages.add(img);
        pendingImages.delete(img);
        img.__ktmMood = msg.mood;
        lastTriggeredIndex = msg.page;
        if (isStillCurrentPage) {
          showMoodBadge(img, msg.mood, false); // full opacity = triggered
        }
        return;
      }

      // If the cache does not have this visible page yet, analyze the current page
      // with its local reading window instead of falling back to a single-page prompt.
      if (isStillCurrentPage && !analyzedImages.has(img) && !pendingImages.has(img)) {
        scheduleVisibleAnalysis(img, getPageIndex(img));
      }
    }

    if (msg.type === 'get_current_image') {
      const img = getCurrentVisibleImage();
      if (!img) {
        sendResponse({ found: false });
        return;
      }

      const thumbnail = captureImageThumbnail(img, 200);
      const pageIndex = getPageIndex(img);
      const src = img.currentSrc || img.src || img.dataset?.src || null;

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
    chapterFeedSentAt.clear();
    liveErrorSentAt.clear();
    resetPreloadState();
    clearTrackedImageState();
    if (pendingVisibleAnalysisTimer) {
      clearTimeout(pendingVisibleAnalysisTimer);
      pendingVisibleAnalysisTimer = null;
      pendingVisibleAnalysisKey = null;
    }

    lastTriggeredIndex = null;
    lastFocusState = null;
    lastFocusedPageIndex = null;
    lastFocusDirection = 1;
    lookupInFlightKey = null;
    lastLookupKey = null;
    lastLookupAt = 0;

    // Re-init on new content (delay to let SPA render)
    if (chapterReinitTimer) {
      clearTimeout(chapterReinitTimer);
      chapterReinitTimer = null;
    }

    chapterReinitTimer = setTimeout(() => {
      chapterReinitTimer = null;
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
            setupScrollTracker();
            setTimeout(checkCurrentPage, 150);
          }
        }
        return;
      }

      const images = document.querySelectorAll(activeProfile.imageSelector);
      if (images.length === 0) return;

      const imgArray = Array.from(images);
      setupIntersectionObserver(imgArray);
      if (lastFocusedPageIndex !== null) {
        schedulePreloadPump();
      }
      setTimeout(checkCurrentPage, 150);

      const readerEl = document.querySelector(activeProfile.readerSelector);
      if (readerEl) setupMutationObserver(readerEl);

      console.log(`[KeyToMusic Mood] Re-initialized for new chapter — ${images.length} pages`);
    }, 500);
  }

  function setupChapterChangeDetection() {
    if (chapterCheckInterval) {
      clearInterval(chapterCheckInterval);
      chapterCheckInterval = null;
    }
    currentChapter = extractChapterInfo().chapter;

    // SPA navigation via History API
    if (!chapterChangeBound) {
      window.addEventListener('popstate', onChapterChange);
      chapterChangeBound = true;
    }

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
          setupScrollTracker();
          setTimeout(checkCurrentPage, 150);
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
    if (lastFocusedPageIndex !== null) {
      schedulePreloadPump();
    }
    setTimeout(checkCurrentPage, 150);

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
    if (scrollTrackerBound) return;
    scrollTrackerBound = true;
    window.addEventListener('scroll', onScroll, { passive: true });
  }

  function onScroll() {
    if (scrollTimer) clearTimeout(scrollTimer);
    scrollTimer = setTimeout(checkCurrentPage, 250);
  }

  function checkCurrentPage() {
    const img = getCurrentVisibleImage();
    if (!img) return;

    const pageIndex = getPageIndex(img);
    if (pageIndex !== null) {
      const totalPages = getOrderedImages().length || null;
      updateChapterFocus(pageIndex, totalPages);
      pumpInPagePreload(pageIndex);
      void queueChapterFeedAroundPage(pageIndex);
      if (img.__ktmMood) {
        if (lastTriggeredIndex !== pageIndex) {
          lastTriggeredIndex = pageIndex;
          safeSendMessage({
            type: 're_trigger',
            mood: img.__ktmMood,
            pageIndex,
          });
        }
        return;
      }

      if (pendingImages.has(img)) return;

      void requestLookupForVisiblePage(img, pageIndex);
      return;
    }

    // Fallback: if already analyzed with a cached mood, re-trigger it
    if (img.__ktmMood) {
      safeSendMessage({
        type: 're_trigger',
        mood: img.__ktmMood,
        pageIndex: pageIndex,
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
  initRetryTimer = setTimeout(() => {
    initRetryTimer = null;
    if (!activeProfile || !document.querySelectorAll(activeProfile?.imageSelector || 'nope').length) {
      init();
    }
  }, 2000);
})();
