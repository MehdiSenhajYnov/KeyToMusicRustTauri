// KeyToMusic Manga Mood — Popup UI

const ext = globalThis.browser ?? globalThis.chrome;
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

const MOOD_LABELS = {
  epic: 'Epic',
  tension: 'Tension',
  sadness: 'Sadness',
  comedy: 'Comedy',
  romance: 'Romance',
  horror: 'Horror',
  peaceful: 'Peaceful',
  mystery: 'Mystery',
};

const ANALYZED_AHEAD_TARGET = 20;
const ANALYZED_BEHIND_TARGET = 10;

const $ = (sel) => document.querySelector(sel);
let lastVisiblePage = null;

function renderMoodPill(container, color, label) {
  container.replaceChildren();
  const pill = document.createElement('span');
  pill.className = 'mood-pill';
  pill.style.background = `${color}20`;
  pill.style.color = color;

  const dot = document.createElement('span');
  dot.className = 'mood-dot';
  dot.style.background = color;
  pill.appendChild(dot);
  pill.appendChild(document.createTextNode(label));
  container.appendChild(pill);
}

function renderAnalyzing(container) {
  container.replaceChildren();
  const label = document.createElement('span');
  label.style.color = '#818cf8';
  label.style.fontSize = '11px';
  label.textContent = 'Analyzing...';
  container.appendChild(label);
}

function normalizePages(pages) {
  if (!Array.isArray(pages)) return [];
  return [...new Set(
    pages
      .map((page) => Number(page))
      .filter((page) => Number.isFinite(page))
  )].sort((a, b) => a - b);
}

function formatPageRanges(pages) {
  const normalized = normalizePages(pages);
  if (normalized.length === 0) return '--';

  const ranges = [];
  let start = normalized[0];
  let end = normalized[0];

  for (let i = 1; i < normalized.length; i++) {
    const page = normalized[i];
    if (page === end + 1) {
      end = page;
      continue;
    }
    ranges.push(start === end ? `${start + 1}` : `${start + 1}-${end + 1}`);
    start = end = page;
  }

  ranges.push(start === end ? `${start + 1}` : `${start + 1}-${end + 1}`);
  return ranges.join(', ');
}

function buildTargetPages(pageIndex, totalPages, direction = 1) {
  if (!Number.isFinite(pageIndex)) return [];

  const pages = [];
  const seen = new Set();
  const maxPage = Number.isFinite(totalPages) ? Math.max(0, Number(totalPages) - 1) : null;
  const pushPage = (targetIndex) => {
    if (targetIndex < 0) return;
    if (maxPage != null && targetIndex > maxPage) return;
    if (seen.has(targetIndex)) return;
    seen.add(targetIndex);
    pages.push(targetIndex);
  };

  pushPage(pageIndex);

  let aheadOffset = 1;
  let behindOffset = 1;
  const forwardBias = direction >= 0;

  while (aheadOffset <= ANALYZED_AHEAD_TARGET || behindOffset <= ANALYZED_BEHIND_TARGET) {
    if (forwardBias) {
      for (let i = 0; i < 2 && aheadOffset <= ANALYZED_AHEAD_TARGET; i += 1, aheadOffset += 1) {
        pushPage(pageIndex + aheadOffset);
      }
      if (behindOffset <= ANALYZED_BEHIND_TARGET) {
        pushPage(pageIndex - behindOffset);
        behindOffset += 1;
      }
    } else {
      for (let i = 0; i < 2 && behindOffset <= ANALYZED_BEHIND_TARGET; i += 1, behindOffset += 1) {
        pushPage(pageIndex - behindOffset);
      }
      if (aheadOffset <= ANALYZED_AHEAD_TARGET) {
        pushPage(pageIndex + aheadOffset);
        aheadOffset += 1;
      }
    }
  }

  return pages;
}

function formatPageList(pages, emptyLabel = '--') {
  const normalized = normalizePages(pages);
  return normalized.length > 0 ? formatPageRanges(normalized) : emptyLabel;
}

function formatPageLabel(page) {
  return Number.isFinite(page) ? `Page ${Number(page) + 1}` : null;
}

function formatElapsed(startedAt) {
  const ts = Number(startedAt);
  if (!Number.isFinite(ts) || ts <= 0) return null;

  const elapsedSeconds = Math.max(0, Math.floor((Date.now() - ts) / 1000));
  if (elapsedSeconds < 60) return `${elapsedSeconds}s`;

  const minutes = Math.floor(elapsedSeconds / 60);
  const seconds = elapsedSeconds % 60;
  return `${minutes}m ${seconds.toString().padStart(2, '0')}s`;
}

function describeCurrentWork(stats, pendingPages, targetPage = null) {
  const phase = stats.currentPhase || null;
  const operationPage = Number.isFinite(stats.currentAnalyzingPage) ? Number(stats.currentAnalyzingPage) : null;
  const elapsed = formatElapsed(stats.currentPhaseStartedAt);
  const targetLabel = formatPageLabel(targetPage);
  const operationLabel = formatPageLabel(operationPage);

  const withElapsed = (label) => elapsed ? `${label} · ${elapsed}` : label;
  const primaryLabel = targetLabel || operationLabel;

  switch (phase) {
    case 'lookup':
      return {
        label: primaryLabel ? withElapsed(`Checking ${primaryLabel}`) : withElapsed('Checking cache'),
        note: stats.statusNote || 'Checking whether the current page already has a mood in cache.',
      };
    case 'lookup_miss':
      return {
        label: primaryLabel ? withElapsed(`Cache miss on ${primaryLabel}`) : withElapsed('Cache miss'),
        note: stats.statusNote || 'The current page is not ready in cache yet, so a direct analysis will start.',
      };
    case 'scheduled':
      return {
        label: primaryLabel ? withElapsed(`Scheduling ${primaryLabel}`) : withElapsed('Scheduling'),
        note: stats.statusNote || 'Preparing the next direct analysis for the visible page.',
      };
    case 'analyze_window':
      return {
        label: primaryLabel ? withElapsed(`Analyzing ${primaryLabel}`) : withElapsed('Analyzing visible page'),
        note: stats.statusNote || 'Running the direct visible-window analysis.',
      };
    case 'chapter_feed':
      return {
        label: targetLabel ? withElapsed(`Preparing ${targetLabel}`) : (operationLabel ? withElapsed(`Uploading ${operationLabel}`) : withElapsed('Uploading future pages')),
        note: targetLabel && operationLabel && targetPage !== operationPage
          ? `Uploading source ${operationLabel} so ${targetLabel} can be analyzed.`
          : (targetLabel
            ? `Uploading the pages needed to analyze ${targetLabel}.`
            : (stats.statusNote || 'Sending future pages into the backend cache pipeline.')),
      };
    case 'backend_window':
      return {
        label: targetLabel ? withElapsed(`Analyzing ${targetLabel}`) : (operationLabel ? withElapsed(`Analyzing ${operationLabel}`) : withElapsed('Analyzing next page')),
        note: targetLabel && operationLabel && targetPage !== operationPage
          ? `Computing the context window centered on ${operationLabel} to finish ${targetLabel}.`
          : (targetLabel
            ? `Computing the context needed to finish ${targetLabel}.`
            : (stats.statusNote || 'The backend chapter pipeline is actively computing the next page.')),
      };
    case 'backend_repair_single':
    case 'backend_repair_pair':
      return {
        label: targetLabel ? withElapsed(`Finalizing ${targetLabel}`) : (operationLabel ? withElapsed(`Finalizing ${operationLabel}`) : withElapsed('Finalizing next page')),
        note: targetLabel && operationLabel && targetPage !== operationPage
          ? `Running a repair step around ${operationLabel} to finish ${targetLabel}.`
          : (targetLabel
            ? `Running the final repair step for ${targetLabel}.`
            : (stats.statusNote || 'The backend chapter pipeline is refining the next page.')),
      };
    case 'backend_waiting':
      return {
        label: targetLabel ? `Waiting for ${targetLabel}` : 'Waiting for context',
        note: targetLabel
          ? `Waiting for enough nearby pages to finish ${targetLabel}.`
          : (stats.statusNote || 'The backend is active, but it is waiting for enough nearby pages to continue.'),
      };
    case 'backend_stalled':
      return {
        label: targetLabel ? `Blocked on ${targetLabel}` : 'Blocked',
        note: targetLabel
          ? `The backend pipeline hit an error while trying to finish ${targetLabel}.`
          : (stats.statusNote || 'The backend pipeline hit an error while pages are still waiting.'),
      };
    case 'blocked':
      return {
        label: primaryLabel ? `Blocked on ${primaryLabel}` : 'Blocked',
        note: stats.statusNote || 'The current page cannot be analyzed yet because the local context window is incomplete.',
      };
    default:
      if (pendingPages.length > 0) {
        return {
          label: targetLabel ? `Waiting for ${targetLabel}` : 'Waiting for context',
          note: targetLabel
            ? `Some future pages are queued, but the system still needs more nearby context to finish ${targetLabel}.`
            : 'Some future pages are queued, but the system is waiting for enough local context to continue.',
        };
      }
      return {
        label: 'Idle',
        note: stats.statusNote || 'Nothing is currently queued or running.',
      };
  }
}

async function refresh() {
  try {
    const res = await ext.runtime.sendMessage({ type: 'get_status' });

    // Connection dot
    const dot = $('#connection');
    if (res.server === 'running') {
      dot.className = 'status-dot connected';
      dot.title = 'Connected to KeyToMusic';
    } else if (res.server === 'unreachable') {
      dot.className = 'status-dot disconnected';
      dot.title = 'Cannot reach KeyToMusic API';
    } else {
      dot.className = 'status-dot';
      dot.title = `Server: ${res.server}`;
    }

    // Toggle
    $('#enableToggle').checked = res.enabled;

    // Port
    $('#portInput').value = res.port;

    // Server status
    const statusEl = $('#serverStatus');
    if (res.server === 'running') {
      statusEl.textContent = 'Running';
      statusEl.style.color = '#22c55e';
    } else if (res.server === 'unreachable') {
      statusEl.textContent = 'Unreachable';
      statusEl.style.color = '#ef4444';
    } else {
      statusEl.textContent = res.server || '--';
      statusEl.style.color = '#f97316';
    }

    // Stats
    const stats = res.stats || {};
    const readyPages = Array.isArray(stats.readyPages) ? stats.readyPages : (stats.analyzedPages || []);
    const effectiveVisiblePage = stats.visiblePage != null ? Number(stats.visiblePage) : lastVisiblePage;
    const targetPages = buildTargetPages(
      effectiveVisiblePage,
      stats.totalPages,
      Number.isFinite(stats.focusDirection) ? Number(stats.focusDirection) : 1
    );
    const targetPageSet = new Set(targetPages);
    const localReadyPages = targetPages.length > 0
      ? normalizePages(readyPages).filter((page) => targetPageSet.has(page))
      : normalizePages(readyPages);
    const readySet = new Set(localReadyPages);
    const pendingPages = targetPages.length > 0
      ? targetPages.filter((page) => !readySet.has(page))
      : [];
    const targetPage = pendingPages.length > 0 ? pendingPages[0] : null;
    const currentWork = describeCurrentWork(stats, pendingPages, targetPage);
    $('#pagesAnalyzed').textContent = localReadyPages.length;
    $('#visiblePage').textContent = effectiveVisiblePage != null ? `Page ${effectiveVisiblePage + 1}` : '--';
    $('#currentAnalyzing').textContent = currentWork.label;
    $('#analysisNote').textContent = currentWork.note;
    $('#analyzedPagesList').textContent = formatPageList(localReadyPages, 'No page ready yet');
    $('#remainingPagesList').textContent = formatPageList(pendingPages, 'No page waiting');
    $('#recentEvents').textContent = Array.isArray(stats.recentEvents) && stats.recentEvents.length > 0
      ? stats.recentEvents.join('\n')
      : '--';

    const moodEl = $('#lastMood');
    if (stats.lastMood) {
      const color = MOOD_COLORS[stats.lastMood] || '#6b7280';
      const label = MOOD_LABELS[stats.lastMood] || stats.lastMood;
      renderMoodPill(moodEl, color, label);
    } else {
      moodEl.replaceChildren();
      moodEl.textContent = '--';
      moodEl.style.color = '';
    }
  } catch {
    $('#connection').className = 'status-dot disconnected';
  }
}

// --- Current image preview ---

let currentImgSrc = null;

async function refreshPreview() {
  const previewImg = $('#previewImg');
  const previewEmpty = $('#previewEmpty');
  const previewOverlay = $('#previewOverlay');
  const previewPage = $('#previewPage');
  const previewMood = $('#previewMood');
  const openBtn = $('#openImgBtn');

  try {
    const [tab] = await ext.tabs.query({ active: true, currentWindow: true });
    if (!tab?.id) throw new Error('no tab');

    const res = await ext.tabs.sendMessage(tab.id, { type: 'get_current_image' });

    if (!res?.found) {
      previewImg.style.display = 'none';
      previewOverlay.style.display = 'none';
      previewEmpty.style.display = '';
      previewEmpty.textContent = 'No manga page detected';
      openBtn.disabled = true;
      currentImgSrc = null;
      return;
    }

    // Thumbnail
    if (res.thumbnail) {
      previewImg.src = res.thumbnail;
      previewImg.style.display = '';
      previewEmpty.style.display = 'none';
    } else {
      previewImg.style.display = 'none';
      previewEmpty.style.display = '';
      previewEmpty.textContent = 'Cannot capture (cross-origin)';
    }

    // Overlay info
    previewOverlay.style.display = '';
    previewPage.textContent = res.pageIndex != null ? `Page ${Number(res.pageIndex) + 1}` : '';
    lastVisiblePage = res.pageIndex != null ? Number(res.pageIndex) : null;

    if (res.mood) {
      const color = MOOD_COLORS[res.mood] || '#6b7280';
      const label = MOOD_LABELS[res.mood] || res.mood;
      renderMoodPill(previewMood, color, label);
    } else if (res.pending) {
      renderAnalyzing(previewMood);
    } else {
      previewMood.replaceChildren();
    }

    // Open button
    currentImgSrc = res.src;
    openBtn.disabled = !res.src;
  } catch {
    lastVisiblePage = null;
    previewImg.style.display = 'none';
    previewOverlay.style.display = 'none';
    previewEmpty.style.display = '';
    previewEmpty.textContent = 'Extension not active on this tab';
    openBtn.disabled = true;
    currentImgSrc = null;
  }
}

// --- Events ---

$('#enableToggle').addEventListener('change', async (e) => {
  await ext.runtime.sendMessage({
    type: 'set_config',
    enabled: e.target.checked,
    port: parseInt($('#portInput').value) || 8765,
  });
  refresh();
});

$('#portSave').addEventListener('click', async () => {
  const port = parseInt($('#portInput').value);
  if (port < 1024 || port > 65535) return;
  await ext.runtime.sendMessage({
    type: 'set_config',
    enabled: $('#enableToggle').checked,
    port,
  });
  refresh();
});

$('#injectBtn').addEventListener('click', async () => {
  const [tab] = await ext.tabs.query({ active: true, currentWindow: true });
  if (!tab) return;

  const btn = $('#injectBtn');
  btn.textContent = 'Injecting...';
  btn.disabled = true;

  const res = await ext.runtime.sendMessage({
    type: 'inject_content_script',
    tabId: tab.id,
  });

  if (res?.error) {
    btn.textContent = 'Failed';
    btn.style.borderColor = '#ef4444';
  } else {
    btn.textContent = 'Active!';
    btn.style.borderColor = '#22c55e';
  }

  setTimeout(() => {
    btn.textContent = 'Activate on this page';
    btn.disabled = false;
    btn.style.borderColor = '';
  }, 2000);
});

$('#openImgBtn').addEventListener('click', () => {
  if (currentImgSrc) {
    ext.tabs.create({ url: currentImgSrc });
  }
});

// Init
refresh();
refreshPreview();
const refreshTimer = setInterval(() => {
  refresh();
  refreshPreview();
}, 1000);

window.addEventListener('beforeunload', () => {
  clearInterval(refreshTimer);
});
