// KeyToMusic Mood AI — popup: connection, chapter progress, site choice, port.

const ext = globalThis.browser ?? globalThis.chrome;

const MOODS = {
  epic: ['Épique', '#b42318'],
  tension: ['Tension', '#b54708'],
  sadness: ['Tristesse', '#1d5fb0'],
  comedy: ['Comédie', '#8a6a00'],
  romance: ['Romance', '#b4236e'],
  horror: ['Horreur', '#6e2bb0'],
  peaceful: ['Paisible', '#1f7a3d'],
  mystery: ['Mystère', '#3f3fa8'],
};
const INTENSITY = { 1: 'calme', 2: 'modéré', 3: 'intense' };

const $ = (id) => document.getElementById(id);
let host = '';

function setState(kind, text, detail = '') {
  $('dot').className = `dot ${kind}`;
  $('state').textContent = text;
  $('detail').textContent = detail;
}

async function refresh() {
  const response = await ext.runtime.sendMessage({ type: 'status' }).catch(() => null);
  if (!response?.ok) {
    setState('bad', 'App introuvable', response?.error || '');
    $('chapter').hidden = true;
    return;
  }
  if (!response.enabled) {
    setState('warn', 'Mood AI est désactivé dans KeyToMusic');
  } else if (!response.configured) {
    setState('warn', 'Modèle à configurer', 'Ajoute ta clé API dans KeyToMusic → Mood AI.');
  } else if (response.error) {
    setState('bad', 'Erreur du modèle', response.error);
  } else {
    setState('ok', 'Connecté', `Modèle : ${response.model}`);
  }

  const chapter = response.chapter;
  $('chapter').hidden = !chapter;
  if (!chapter) return;
  $('title').textContent = chapter.title || chapter.key;
  const total = chapter.totalPages || chapter.received || 0;
  $('progress').style.width = total ? `${Math.round((100 * chapter.analyzed) / total)}%` : '0';
  $('counts').textContent =
    `${chapter.analyzed}/${total} pages analysées · ${chapter.received} reçues` +
    (chapter.inFlight ? ` · ${chapter.inFlight} en cours` : '') +
    ` · page ${chapter.focus + 1}`;
  const current = chapter.current;
  $('mood').hidden = !current;
  $('summary').textContent = current?.summary || '';
  if (current) {
    const [name, color] = MOODS[current.mood] || [current.mood, '#555'];
    $('mood').textContent = `${name} · ${INTENSITY[current.intensity] || ''}`;
    $('mood').style.background = color;
  }
}

async function init() {
  const config = await ext.storage.local.get({ port: 8765, enabled: true, sites: {} });
  $('enabled').checked = config.enabled;
  $('port').value = config.port;

  const [tab] = await ext.tabs.query({ active: true, currentWindow: true }).catch(() => []);
  try {
    host = new URL(tab?.url || '').hostname;
  } catch {
    host = '';
  }
  $('host').textContent = host ? `(${host})` : '';
  const choice = config.sites[host];
  $('site').value = choice === true ? 'yes' : choice === false ? 'no' : 'ask';
  $('site').disabled = !host;

  $('enabled').addEventListener('change', () => ext.storage.local.set({ enabled: $('enabled').checked }));
  $('port').addEventListener('change', () => {
    const port = Number($('port').value);
    if (port >= 1024 && port <= 65535) ext.storage.local.set({ port }).then(refresh);
  });
  $('site').addEventListener('change', async () => {
    const { sites } = await ext.storage.local.get({ sites: {} });
    const value = $('site').value;
    if (value === 'ask') delete sites[host];
    else sites[host] = value === 'yes';
    await ext.storage.local.set({ sites });
  });

  await refresh();
  setInterval(refresh, 1500);
}

init();
