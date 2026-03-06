// KeyToMusic Manga Mood — Popup UI

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

const MOOD_LABELS = {
  epic_battle: 'Epic Battle',
  tension: 'Tension',
  sadness: 'Sadness',
  comedy: 'Comedy',
  romance: 'Romance',
  horror: 'Horror',
  peaceful: 'Peaceful',
  emotional_climax: 'Climax',
  mystery: 'Mystery',
  chase_action: 'Chase',
};

const $ = (sel) => document.querySelector(sel);

async function refresh() {
  try {
    const res = await chrome.runtime.sendMessage({ type: 'get_status' });

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
    $('#pagesAnalyzed').textContent = stats.analyzed || 0;

    const moodEl = $('#lastMood');
    if (stats.lastMood) {
      const color = MOOD_COLORS[stats.lastMood] || '#6b7280';
      const label = MOOD_LABELS[stats.lastMood] || stats.lastMood;
      moodEl.innerHTML = `<span class="mood-pill" style="background:${color}20;color:${color}"><span class="mood-dot" style="background:${color}"></span>${label}</span>`;
    } else {
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
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (!tab?.id) throw new Error('no tab');

    const res = await chrome.tabs.sendMessage(tab.id, { type: 'get_current_image' });

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

    if (res.mood) {
      const color = MOOD_COLORS[res.mood] || '#6b7280';
      const label = MOOD_LABELS[res.mood] || res.mood;
      previewMood.innerHTML = `<span class="mood-pill" style="background:${color}20;color:${color}"><span class="mood-dot" style="background:${color}"></span>${label}</span>`;
    } else if (res.pending) {
      previewMood.innerHTML = `<span style="color:#818cf8;font-size:11px">Analyzing...</span>`;
    } else {
      previewMood.innerHTML = '';
    }

    // Open button
    currentImgSrc = res.src;
    openBtn.disabled = !res.src;
  } catch {
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
  await chrome.runtime.sendMessage({
    type: 'set_config',
    enabled: e.target.checked,
    port: parseInt($('#portInput').value) || 8765,
  });
  refresh();
});

$('#portSave').addEventListener('click', async () => {
  const port = parseInt($('#portInput').value);
  if (port < 1024 || port > 65535) return;
  await chrome.runtime.sendMessage({
    type: 'set_config',
    enabled: $('#enableToggle').checked,
    port,
  });
  refresh();
});

$('#injectBtn').addEventListener('click', async () => {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (!tab) return;

  const btn = $('#injectBtn');
  btn.textContent = 'Injecting...';
  btn.disabled = true;

  const res = await chrome.runtime.sendMessage({
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
    chrome.tabs.create({ url: currentImgSrc });
  }
});

// Init
refresh();
refreshPreview();
