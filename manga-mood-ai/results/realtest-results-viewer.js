"use strict";

(function () {
  const API_BASE = "/api";

  const STATUS_SCORE = {
    fail: 0,
    relaxed: 1,
    strict: 2,
  };

  const MOOD_FAMILY = {
    epic: 1,
    tension: 1,
    sadness: 2,
    peaceful: 2,
    comedy: 3,
    romance: 3,
    horror: 4,
    mystery: 4,
  };

  const DELTA_SORT = {
    b_better: 0,
    a_better: 1,
    changed: 2,
    same: 3,
  };
  const REFERENCE_METHOD_ID = "ground_truth_reference";
  const REFERENCE_SOURCE_PATH = "ground_truth_reference.json";
  const REFERENCE_SOUNDTRACK_METHOD_ID = "ground_truth_reference_soundtrack_commit";
  const REFERENCE_SOUNDTRACK_SOURCE_PATH = "ground_truth_reference_soundtrack_commit.json";
  const SOUNDTRACK_COMMIT_SUFFIX = "_soundtrack_commit.json";
  const ALL_MOODS = [
    "epic",
    "tension",
    "sadness",
    "comedy",
    "romance",
    "horror",
    "peaceful",
    "mystery",
  ];
  const SOUNDTRACK_FAMILY_BY_MOOD = {
    epic: "action",
    tension: "action",
    sadness: "emotion",
    romance: "emotion",
    comedy: "light",
    peaceful: "light",
    horror: "suspense",
    mystery: "suspense",
  };
  const SOUNDTRACK_FAMILY_MOODS = {
    action: ["epic", "tension"],
    emotion: ["sadness", "romance"],
    light: ["comedy", "peaceful"],
    suspense: ["horror", "mystery"],
  };
  const SOUNDTRACK_TARGET_MOODS = {
    action: ["tension", "epic"],
    emotion: ["sadness", "romance"],
    light: ["peaceful", "comedy"],
    suspense: ["mystery", "horror"],
  };
  const SOUNDTRACK_CONTEXT_RADIUS = 2;
  const SOUNDTRACK_MAX_RECENT_OBSERVATIONS = 6;
  const SOUNDTRACK_MAX_CONTIGUOUS_PAGE_GAP = 3;
  const SOUNDTRACK_MIN_FUTURE_PAGES_FOR_FORCED_FAMILY_EXIT = 2;
  const SOUNDTRACK_MIN_FAMILY_STREAK_TO_SWITCH = 2;
  const SOUNDTRACK_MIN_MOOD_STREAK_TO_SWITCH = 2;
  const SOUNDTRACK_FAMILY_SUPPORT_THRESHOLD = 0.55;
  const SOUNDTRACK_CONTEXT_FAMILY_SUPPORT_THRESHOLD = 0.50;
  const SOUNDTRACK_CONTEXT_FAMILY_GAP_THRESHOLD = 0.02;
  const SOUNDTRACK_CONTEXT_MOOD_SUPPORT_THRESHOLD = 0.50;
  const SOUNDTRACK_CONTEXT_MOOD_GAP_THRESHOLD = 0.02;
  const SOUNDTRACK_SAME_FAMILY_MOOD_SCORE_THRESHOLD = 0.50;
  const SOUNDTRACK_SAME_FAMILY_MOOD_GAP_THRESHOLD = 0.08;
  const METHOD_ALIASES = {
    [REFERENCE_METHOD_ID]: "Reference",
    [REFERENCE_SOURCE_PATH]: "Reference",
    [REFERENCE_SOUNDTRACK_SOURCE_PATH]: "Reference + OST commit",
    "realtests/realtest_suite_t4b_wide5_selective_hold.json": "Production",
    "realtests/realtest_suite_t4b_wide5_selective.json": "Baseline",
  };
  const INFO_ICON = [
    "<svg width=\"16\" height=\"16\" viewBox=\"0 0 16 16\" fill=\"none\" xmlns=\"http://www.w3.org/2000/svg\" aria-hidden=\"true\">",
    "<circle cx=\"8\" cy=\"8\" r=\"6.25\" stroke=\"currentColor\" stroke-width=\"1.5\"/>",
    "<circle cx=\"8\" cy=\"4.6\" r=\"0.9\" fill=\"currentColor\"/>",
    "<path d=\"M8 7.1V11.3\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linecap=\"round\"/>",
    "</svg>",
  ].join("");
  const DEFAULT_INSPECTOR_WIDTH = 360;
  const NARROW_INSPECTOR_WIDTH = 320;
  const MIN_INSPECTOR_WIDTH = 280;
  const MIN_RESULTS_WIDTH = 360;
  const WORKSPACE_RESIZER_SIZE = 20;
  const WORKSPACE_STACK_BREAKPOINT = 960;
  const NARROW_INSPECTOR_BREAKPOINT = 1180;
  const RESIZER_KEYBOARD_STEP = 32;
  const RESULTS_TABLE_OVERSCAN = 8;
  const ESTIMATED_RESULTS_ROW_HEIGHT = 96;
  const INSPECTOR_WIDTH_STORAGE_KEY = "realtestResultsViewer.inspectorWidth";
  const VIEWER_PREFERENCES_STORAGE_KEY = "realtestResultsViewer.preferences";

  const state = {
    groundTruth: null,
    methods: [],
    methodSearch: "",
    activeMethodAId: "",
    activeMethodBId: "",
    selectedPath: "",
    imageRoot: "../test-images/RealTest/",
    generatedAt: "",
    serverMode: false,
    pendingLoads: Object.create(null),
    inspectorWidth: null,
    pendingInspectorWidth: null,
    resizeSession: null,
    resizeAnimationFrame: 0,
    cachedResultsRows: [],
    resultsRowHeight: ESTIMATED_RESULTS_ROW_HEIGHT,
    resultsMeasureFrame: 0,
    resultsScrollFrame: 0,
  };

  const filters = {
    series: "all",
    chapter: "all",
    status: "all",
    compare: "all",
    bench: "all",
    sort: "page",
    pageSearch: "",
  };

  const elements = {};

  document.addEventListener("DOMContentLoaded", function () {
    void init();
  });

  async function init() {
    cacheElements();
    restoreViewerPreferences();
    bindEvents();
    restoreInspectorWidth();
    syncWorkspaceLayout();
    elements.imageRootInput.value = state.imageRoot;
    renderAll();

    if (window.location.protocol === "file:") {
      setCatalogStatus(
        "Mode auto indisponible en file://. Lance `python scripts/serve-realtest-viewer.py` puis ouvre l'URL locale.",
        "warn"
      );
      renderAll();
      return;
    }

    await reloadServerCatalog(false);
  }

  function cacheElements() {
    elements.workspace = document.getElementById("workspace");
    elements.results = document.querySelector(".results");
    elements.workspaceResizer = document.getElementById("workspaceResizer");
    elements.inspector = document.getElementById("inspectorPanel");
    elements.catalogStatus = document.getElementById("catalogStatus");
    elements.catalogSummary = document.getElementById("catalogSummary");
    elements.focusCompareButton = document.getElementById("focusCompareButton");
    elements.toggleSetupButton = document.getElementById("toggleSetupButton");
    elements.toggleFiltersButton = document.getElementById("toggleFiltersButton");
    elements.toggleCatalogButton = document.getElementById("toggleCatalogButton");
    elements.toggleImportButton = document.getElementById("toggleImportButton");
    elements.reloadCatalogButton = document.getElementById("reloadCatalogButton");
    elements.setupDrawer = document.getElementById("setupDrawer");
    elements.filtersDrawer = document.getElementById("filtersDrawer");
    elements.catalogDrawer = document.getElementById("catalogDrawer");
    elements.importDrawer = document.getElementById("importDrawer");
    elements.methodSearchInput = document.getElementById("methodSearchInput");
    elements.methodASelect = document.getElementById("methodASelect");
    elements.methodBSelect = document.getElementById("methodBSelect");
    elements.swapMethodsButton = document.getElementById("swapMethodsButton");
    elements.clearCompareButton = document.getElementById("clearCompareButton");
    elements.seriesFilter = document.getElementById("seriesFilter");
    elements.chapterFilter = document.getElementById("chapterFilter");
    elements.statusFilter = document.getElementById("statusFilter");
    elements.compareFilter = document.getElementById("compareFilter");
    elements.benchFilter = document.getElementById("benchFilter");
    elements.sortFilter = document.getElementById("sortFilter");
    elements.pageSearchInput = document.getElementById("pageSearchInput");
    elements.resetFiltersButton = document.getElementById("resetFiltersButton");
    elements.firstDiffButton = document.getElementById("firstDiffButton");
    elements.groundTruthInput = document.getElementById("groundTruthInput");
    elements.methodFilesInput = document.getElementById("methodFilesInput");
    elements.imageRootInput = document.getElementById("imageRootInput");
    elements.methodsTableBody = document.getElementById("methodsTableBody");
    elements.heroTitle = document.getElementById("heroTitle");
    elements.heroSubtitle = document.getElementById("heroSubtitle");
    elements.heroChip = document.getElementById("heroChip");
    elements.summaryCards = document.getElementById("summaryCards");
    elements.previewTitle = document.getElementById("previewTitle");
    elements.previewCounter = document.getElementById("previewCounter");
    elements.previewFigure = document.getElementById("previewFigure");
    elements.pageMethodTableBody = document.getElementById("pageMethodTableBody");
    elements.resultsTitle = document.getElementById("resultsTitle");
    elements.resultsCountLine = document.getElementById("resultsCountLine");
    elements.resultsTableWrap = document.getElementById("resultsTableWrap");
    elements.exportCsvButton = document.getElementById("exportCsvButton");
    elements.firstFailButton = document.getElementById("firstFailButton");
    elements.resultsTableHead = document.getElementById("resultsTableHead");
    elements.resultsTableBody = document.getElementById("resultsTableBody");
  }

  function bindEvents() {
    elements.focusCompareButton.addEventListener("click", focusCompareView);
    elements.toggleSetupButton.addEventListener("click", function () {
      toggleDrawer(elements.setupDrawer);
    });
    elements.toggleFiltersButton.addEventListener("click", function () {
      toggleDrawer(elements.filtersDrawer);
    });
    elements.toggleCatalogButton.addEventListener("click", function () {
      toggleDrawer(elements.catalogDrawer);
    });
    elements.toggleImportButton.addEventListener("click", function () {
      toggleDrawer(elements.importDrawer);
    });
    elements.reloadCatalogButton.addEventListener("click", function () {
      void reloadServerCatalog(true);
    });
    elements.setupDrawer.addEventListener("toggle", syncToolbarState);
    elements.filtersDrawer.addEventListener("toggle", syncToolbarState);
    elements.catalogDrawer.addEventListener("toggle", syncToolbarState);
    elements.importDrawer.addEventListener("toggle", syncToolbarState);

    elements.methodSearchInput.addEventListener("input", function (event) {
      state.methodSearch = normalizeSearchInput(event.target.value);
      persistViewerPreferences();
      renderMethodCatalog();
    });

    elements.methodASelect.addEventListener("change", function (event) {
      void activateMethodA(event.target.value);
    });

    elements.methodBSelect.addEventListener("change", function (event) {
      void activateMethodB(event.target.value);
    });

    elements.swapMethodsButton.addEventListener("click", function () {
      void swapMethods();
    });

    elements.clearCompareButton.addEventListener("click", function () {
      void clearCompare();
    });

    elements.seriesFilter.addEventListener("change", function (event) {
      filters.series = event.target.value;
      filters.chapter = "all";
      persistViewerPreferences();
      syncChapterOptions();
      renderAll();
    });

    elements.chapterFilter.addEventListener("change", function (event) {
      filters.chapter = event.target.value;
      persistViewerPreferences();
      renderAll();
    });

    elements.statusFilter.addEventListener("change", function (event) {
      filters.status = event.target.value;
      persistViewerPreferences();
      renderAll();
    });

    elements.compareFilter.addEventListener("change", function (event) {
      filters.compare = event.target.value;
      persistViewerPreferences();
      renderAll();
    });

    elements.benchFilter.addEventListener("change", function (event) {
      filters.bench = event.target.value;
      persistViewerPreferences();
      renderAll();
    });

    elements.sortFilter.addEventListener("change", function (event) {
      filters.sort = event.target.value;
      persistViewerPreferences();
      renderAll();
    });

    elements.pageSearchInput.addEventListener("input", function (event) {
      filters.pageSearch = normalizeSearchInput(event.target.value);
      persistViewerPreferences();
      renderAll();
    });

    elements.resetFiltersButton.addEventListener("click", resetFilters);
    elements.firstDiffButton.addEventListener("click", selectFirstDiff);
    elements.firstFailButton.addEventListener("click", selectFirstFail);
    elements.exportCsvButton.addEventListener("click", exportCsv);
    elements.resultsTableWrap.addEventListener("scroll", handleResultsTableScroll);
    elements.resultsTableBody.addEventListener("click", handleResultsTableClick);
    elements.methodsTableBody.addEventListener("click", handleMethodCatalogClick);
    elements.imageRootInput.addEventListener("input", function (event) {
      state.imageRoot = event.target.value.trim() || "../test-images/RealTest/";
      renderAll();
    });
    elements.groundTruthInput.addEventListener("change", function (event) {
      void handleGroundTruthImport(event);
    });
    elements.methodFilesInput.addEventListener("change", function (event) {
      void handleMethodImport(event);
    });
    elements.workspaceResizer.addEventListener("pointerdown", handleWorkspaceResizeStart);
    elements.workspaceResizer.addEventListener("pointermove", handleWorkspaceResizeMove);
    elements.workspaceResizer.addEventListener("pointerup", handleWorkspaceResizeEnd);
    elements.workspaceResizer.addEventListener("pointercancel", handleWorkspaceResizeEnd);
    elements.workspaceResizer.addEventListener("lostpointercapture", handleWorkspaceResizeEnd);
    elements.workspaceResizer.addEventListener("keydown", handleWorkspaceResizerKeyDown);
    window.addEventListener("resize", handleViewportResize);
  }

  async function reloadServerCatalog(forceRefresh) {
    if (window.location.protocol === "file:") {
      return;
    }

    setCatalogStatus("Chargement du catalogue live...", "neutral");

    const previousMethods = new Map(
      state.methods.map(function (method) {
        return [method.id, method];
      })
    );
    const previousManualMethods = state.methods.filter(function (method) {
      return isManualMethod(method);
    });

    try {
      const [catalogPayload, groundTruthPayload] = await Promise.all([
        fetchJson(apiUrl("/catalog", forceRefresh)),
        fetchJson(apiUrl("/ground-truth", forceRefresh)),
      ]);

      state.serverMode = true;
      state.generatedAt = String(catalogPayload.generatedAt || "");
      state.imageRoot = String(catalogPayload.imageRoot || state.imageRoot);
      state.groundTruth = normalizeGroundTruth(groundTruthPayload);
      elements.imageRootInput.value = state.imageRoot;

      const nextMethods = normalizeCatalogMethods(catalogPayload.methods || []).map(function (method) {
        const previous = previousMethods.get(method.id);
        if (previous && previous.predictions) {
          method.predictions = previous.predictions;
        }
        return method;
      });

      state.methods = nextMethods;
      for (const method of previousManualMethods) {
        upsertMethod(method);
      }

      upsertReferenceMethod();
      pickDefaultMethods();
      persistViewerPreferences();
      renderAll();
      await hydrateActiveMethods();
      setCatalogStatus("Catalogue live charge depuis les JSON du dossier results.", "ok");
    } catch (error) {
      setCatalogStatus("Chargement catalogue impossible: " + error.message, "warn");
    }

    renderAll();
  }

  async function activateMethodA(methodId) {
    state.activeMethodAId = methodId;
    if (state.activeMethodAId && state.activeMethodAId === state.activeMethodBId) {
      state.activeMethodBId = "";
    }
    persistViewerPreferences();
    renderAll();
    await hydrateActiveMethods();
    renderAll();
  }

  async function activateMethodB(methodId) {
    state.activeMethodBId = methodId === state.activeMethodAId ? "" : methodId;
    persistViewerPreferences();
    renderAll();
    await hydrateActiveMethods();
    renderAll();
  }

  async function swapMethods() {
    if (!state.activeMethodBId) {
      return;
    }
    const currentA = state.activeMethodAId;
    state.activeMethodAId = state.activeMethodBId;
    state.activeMethodBId = currentA;
    persistViewerPreferences();
    renderAll();
    await hydrateActiveMethods();
    renderAll();
  }

  async function clearCompare() {
    state.activeMethodBId = "";
    persistViewerPreferences();
    renderAll();
  }

  async function hydrateActiveMethods() {
    const methodIds = uniqueDefined([state.activeMethodAId, state.activeMethodBId]).filter(function (methodId) {
      const method = getMethodById(methodId);
      return method && !method.predictions;
    });

    if (!methodIds.length) {
      return;
    }

    const labels = methodIds.map(function (methodId) {
      const method = getMethodById(methodId);
      return method ? method.label : methodId;
    });

    setCatalogStatus("Chargement des resultats: " + labels.join(", "), "neutral");

    try {
      await Promise.all(
        methodIds.map(function (methodId) {
          return ensureMethodLoaded(methodId);
        })
      );
    } catch (error) {
      setCatalogStatus("Chargement d'une methode impossible: " + error.message, "warn");
    }
  }

  async function ensureMethodLoaded(methodId) {
    const method = getMethodById(methodId);
    if (!method || method.predictions) {
      return method;
    }

    if (state.pendingLoads[methodId]) {
      await state.pendingLoads[methodId];
      return getMethodById(methodId);
    }

    state.pendingLoads[methodId] = (async function () {
      const payload = await fetchJson(apiUrl("/method", false, { source: method.sourcePath }));
      const methodPayload = isResultMap(payload)
        ? { id: method.id, label: method.label, sourcePath: method.sourcePath, predictions: payload }
        : payload;
      const normalized = normalizeMethodPayload(methodPayload, method);
      method.predictions = normalized.predictions;
      method.pageCount = normalized.pageCount;
      method.counted = normalized.counted;
      method.strict = normalized.strict;
      method.relaxedPass = normalized.relaxedPass;
      return method;
    })();

    try {
      await state.pendingLoads[methodId];
    } finally {
      delete state.pendingLoads[methodId];
    }

    return getMethodById(methodId);
  }

  async function handleGroundTruthImport(event) {
    const file = event.target.files && event.target.files[0];
    if (!file) {
      return;
    }

    try {
      const payload = JSON.parse(await file.text());
      state.groundTruth = normalizeGroundTruth(payload);
      state.selectedPath = "";
      upsertReferenceMethod();
      pickDefaultMethods();
      persistViewerPreferences();
      setCatalogStatus("Ground truth importe manuellement: " + file.name, "ok");
      renderAll();
    } catch (error) {
      setCatalogStatus("Import ground truth impossible: " + error.message, "warn");
    } finally {
      event.target.value = "";
    }
  }

  async function handleMethodImport(event) {
    const files = Array.from(event.target.files || []);
    if (!files.length) {
      return;
    }

    const imported = [];
    const skipped = [];

    for (const file of files) {
      try {
        const payload = JSON.parse(await file.text());
        const parsed = parseImportedMethod(file.name, payload);
        if (!parsed) {
          skipped.push(file.name);
          continue;
        }
        upsertMethod(parsed);
        imported.push(parsed.label);
      } catch (error) {
        skipped.push(file.name + ": " + error.message);
      }
    }

    if (!state.activeMethodAId && state.methods.length) {
      state.activeMethodAId = state.methods[0].id;
    }

    const messages = [];
    if (imported.length) {
      messages.push("Methodes importees: " + imported.join(", "));
    }
    if (skipped.length) {
      messages.push("Ignorees: " + skipped.join(" | "));
    }

    setCatalogStatus(messages.join(" || ") || "Aucune methode importee.", imported.length ? "ok" : "warn");
    persistViewerPreferences();
    renderAll();
    event.target.value = "";
  }

  function parseImportedMethod(fileName, payload) {
    if (!isResultMap(payload)) {
      return null;
    }

    return normalizeMethodPayload(
      {
        id: "manual/" + fileName,
        label: fileName.replace(/\.json$/i, ""),
        sourcePath: "manual/" + fileName,
        predictions: payload,
      },
      null
    );
  }

  function normalizeGroundTruth(rawGroundTruth) {
    if (!rawGroundTruth || typeof rawGroundTruth !== "object" || Array.isArray(rawGroundTruth)) {
      throw new Error("ground truth inattendu");
    }

    const groundTruth = {};
    for (const relPath of Object.keys(rawGroundTruth)) {
      const entry = rawGroundTruth[relPath];
      if (!entry || typeof entry !== "object") {
        continue;
      }
      if (typeof entry.mood !== "string" || !entry.mood) {
        continue;
      }
      groundTruth[relPath] = {
        relPath: relPath,
        mood: entry.mood,
        intensity: toNumber(entry.intensity),
        confidence: toNumber(entry.confidence),
        reasoning: typeof entry.reasoning === "string" ? entry.reasoning : "",
      };
    }

    if (!Object.keys(groundTruth).length) {
      throw new Error("aucune page exploitable");
    }

    return groundTruth;
  }

  function normalizeCatalogMethods(rawMethods) {
    return rawMethods
      .map(normalizeCatalogMethod)
      .filter(Boolean)
      .sort(compareMethods);
  }

  function normalizeCatalogMethod(rawMethod) {
    if (!rawMethod || typeof rawMethod !== "object") {
      return null;
    }

    const sourcePath = String(rawMethod.sourcePath || rawMethod.label || rawMethod.id);
    const predictions = isResultMap(rawMethod.predictions)
      ? normalizePredictionMap(rawMethod.predictions)
      : null;

    return {
      id: String(rawMethod.id || rawMethod.sourcePath || rawMethod.label),
      label: methodDisplayLabel(sourcePath, rawMethod.label || rawMethod.sourcePath || rawMethod.id),
      sourcePath: sourcePath,
      pageCount: toNumber(rawMethod.pageCount),
      counted: toNumber(rawMethod.counted),
      strict: toNumber(rawMethod.strict),
      relaxedPass: toNumber(rawMethod.relaxedPass),
      predictions: predictions,
    };
  }

  function normalizeMethodPayload(rawMethod, baseMethod) {
    if (!rawMethod || typeof rawMethod !== "object") {
      throw new Error("methode inattendue");
    }

    const predictions = normalizePredictionMap(rawMethod.predictions || rawMethod);
    if (!Object.keys(predictions).length) {
      throw new Error("aucune prediction exploitable");
    }

    const sourcePath = String(rawMethod.sourcePath || (baseMethod && baseMethod.sourcePath) || rawMethod.label || rawMethod.id);

    const ownStats = computeMethodOwnStatsFromPredictions(predictions);

    return {
      id: String(rawMethod.id || (baseMethod && baseMethod.id) || rawMethod.sourcePath || rawMethod.label),
      label: methodDisplayLabel(
        sourcePath,
        rawMethod.label || (baseMethod && baseMethod.label) || rawMethod.sourcePath || rawMethod.id
      ),
      sourcePath: sourcePath,
      pageCount: Object.keys(predictions).length,
      counted: ownStats.counted,
      strict: ownStats.strict,
      relaxedPass: ownStats.relaxedPass,
      predictions: predictions,
    };
  }

  function normalizePredictionMap(rawPredictions) {
    if (!isResultMap(rawPredictions)) {
      return {};
    }

    const predictions = {};
    for (const relPath of Object.keys(rawPredictions)) {
      const parsedTag = parseTag(rawPredictions[relPath]);
      if (!parsedTag) {
        continue;
      }
      predictions[relPath] = parsedTag;
    }
    return predictions;
  }

  function isResultMap(payload) {
    return (
      payload &&
      typeof payload === "object" &&
      !Array.isArray(payload) &&
      Object.keys(payload).length > 0 &&
      Object.values(payload).every(function (value) {
        return typeof value === "string";
      })
    );
  }

  function parseTag(rawTag) {
    if (typeof rawTag !== "string") {
      return null;
    }

    const parts = rawTag.split(":");
    const mood = parts[0] ? parts[0].trim() : "";
    if (!mood) {
      return null;
    }

    return {
      mood: mood,
      intensity: toNumber(parts[1]),
      raw: mood + ":" + toNumber(parts[1]),
    };
  }

  function methodDisplayLabel(sourcePath, fallbackLabel) {
    if (isSoundtrackCommitSourcePath(sourcePath)) {
      const baseSourcePath = soundtrackCommitBaseSourcePath(sourcePath);
      const baseLabel = methodDisplayLabel(baseSourcePath, baseSourcePath);
      return String(baseLabel + " + OST commit");
    }
    return String(METHOD_ALIASES[sourcePath] || fallbackLabel || sourcePath);
  }

  function upsertReferenceMethod() {
    if (!state.groundTruth) {
      return;
    }

    const predictions = {};
    for (const relPath of Object.keys(state.groundTruth)) {
      const entry = state.groundTruth[relPath];
      predictions[relPath] = {
        mood: entry.mood,
        intensity: toNumber(entry.intensity),
        raw: formatTag(entry.mood, entry.intensity),
      };
    }

    const ownStats = computeMethodOwnStatsFromPredictions(predictions);
    upsertMethod({
      id: REFERENCE_METHOD_ID,
      label: methodDisplayLabel(REFERENCE_SOURCE_PATH, "Reference"),
      sourcePath: REFERENCE_SOURCE_PATH,
      pageCount: Object.keys(predictions).length,
      counted: ownStats.counted,
      strict: ownStats.strict,
      relaxedPass: ownStats.relaxedPass,
      predictions: predictions,
    });

    const soundtrackPredictions = buildReferenceSoundtrackCommitPredictions(state.groundTruth);
    const soundtrackStats = computeMethodOwnStatsFromPredictions(soundtrackPredictions);
    upsertMethod({
      id: REFERENCE_SOUNDTRACK_METHOD_ID,
      label: methodDisplayLabel(REFERENCE_SOUNDTRACK_SOURCE_PATH, "Reference + OST commit"),
      sourcePath: REFERENCE_SOUNDTRACK_SOURCE_PATH,
      pageCount: Object.keys(soundtrackPredictions).length,
      counted: soundtrackStats.counted,
      strict: soundtrackStats.strict,
      relaxedPass: soundtrackStats.relaxedPass,
      predictions: soundtrackPredictions,
    });
  }

  function upsertMethod(method) {
    const index = state.methods.findIndex(function (entry) {
      return entry.id === method.id;
    });

    if (index >= 0) {
      state.methods[index] = method;
    } else {
      state.methods.push(method);
    }

    state.methods.sort(compareMethods);
  }

  function compareMethods(left, right) {
    const priorityDiff = methodPriority(left) - methodPriority(right);
    if (priorityDiff !== 0) {
      return priorityDiff;
    }
    return left.label.localeCompare(right.label);
  }

  function methodPriority(method) {
    const source = method.sourcePath || "";
    if (isSoundtrackCommitSourcePath(source)) {
      return methodPriority({ sourcePath: soundtrackCommitBaseSourcePath(source) }) + 0.5;
    }
    if (source === REFERENCE_SOURCE_PATH) {
      return 1;
    }
    if (source === REFERENCE_METHOD_ID) {
      return 1;
    }
    if (source === "realtest_suite_t4b_wide5_selective_hold.json") {
      return 4;
    }
    if (source === "realtests/realtest_suite_t4b_wide5_selective_hold.json") {
      return 0;
    }
    if (source === "realtests/realtest_suite_t4b_wide5_selective.json") {
      return 2;
    }
    if (source === "realtest_suite_t4b_wide5_selective.json") {
      return 5;
    }
    if (source.startsWith("realtests/realtest_suite_")) {
      return 3;
    }
    if (source.startsWith("realtest_suite_")) {
      return 6;
    }
    if (source.startsWith("realtests/realtest_v12_")) {
      return 7;
    }
    if (source.startsWith("passes/")) {
      return 8;
    }
    if (source.startsWith("manual/")) {
      return 9;
    }
    return 10;
  }

  function pickDefaultMethods() {
    if (!state.methods.length) {
      state.activeMethodAId = "";
      state.activeMethodBId = "";
      return;
    }

    const preferredA = [
      "realtests/realtest_suite_t4b_wide5_selective_hold.json",
      "realtests/realtest_suite_t4b_wide5_selective.json",
      "realtests/realtest_suite_t4b_hist_live.json",
      "realtest_suite_t4b_wide5_selective_hold.json",
    ];
    const preferredB = [
      soundtrackCommitCompanionSourcePath("realtests/realtest_suite_t4b_wide5_selective_hold.json"),
      REFERENCE_METHOD_ID,
      "realtests/realtest_suite_t4b_wide5_selective.json",
      "realtests/realtest_suite_t4b_hist_live.json",
      "realtests/realtest_suite_t4b_viterbi.json",
    ];

    const chosenA = firstExistingMethodId(preferredA) || state.methods[0].id;
    state.activeMethodAId = state.activeMethodAId && hasMethod(state.activeMethodAId)
      ? state.activeMethodAId
      : chosenA;

    const chosenB = firstExistingMethodId(preferredB);
    if (state.activeMethodBId && hasMethod(state.activeMethodBId) && state.activeMethodBId !== state.activeMethodAId) {
      return;
    }
    state.activeMethodBId = chosenB && chosenB !== state.activeMethodAId ? chosenB : "";
  }

  function firstExistingMethodId(candidates) {
    for (const id of candidates) {
      if (hasMethod(id)) {
        return id;
      }
    }
    return "";
  }

  function hasMethod(methodId) {
    return state.methods.some(function (method) {
      return method.id === methodId;
    });
  }

  function isSoundtrackCommitSourcePath(sourcePath) {
    return typeof sourcePath === "string" && sourcePath.endsWith(SOUNDTRACK_COMMIT_SUFFIX);
  }

  function soundtrackCommitBaseSourcePath(sourcePath) {
    if (!isSoundtrackCommitSourcePath(sourcePath)) {
      return "";
    }
    return sourcePath.slice(0, -SOUNDTRACK_COMMIT_SUFFIX.length) + ".json";
  }

  function soundtrackCommitCompanionSourcePath(sourcePath) {
    if (typeof sourcePath !== "string" || !sourcePath.endsWith(".json") || isSoundtrackCommitSourcePath(sourcePath)) {
      return "";
    }
    return sourcePath.slice(0, -5) + SOUNDTRACK_COMMIT_SUFFIX;
  }

  function findMethodIdBySourcePath(sourcePath) {
    const match = state.methods.find(function (method) {
      return method.sourcePath === sourcePath;
    });
    return match ? match.id : "";
  }

  function getSoundtrackPairMethodId(methodId) {
    const method = getMethodById(methodId);
    if (!method) {
      return "";
    }

    if (isSoundtrackCommitSourcePath(method.sourcePath)) {
      return findMethodIdBySourcePath(soundtrackCommitBaseSourcePath(method.sourcePath));
    }

    return findMethodIdBySourcePath(soundtrackCommitCompanionSourcePath(method.sourcePath));
  }

  async function compareMethodWithSoundtrackPair(methodId) {
    const method = getMethodById(methodId);
    const pairMethodId = getSoundtrackPairMethodId(methodId);
    if (!method || !pairMethodId) {
      return;
    }

    if (isSoundtrackCommitSourcePath(method.sourcePath)) {
      state.activeMethodAId = pairMethodId;
      state.activeMethodBId = methodId;
    } else {
      state.activeMethodAId = methodId;
      state.activeMethodBId = pairMethodId;
    }
    persistViewerPreferences();
    renderAll();
    await hydrateActiveMethods();
    renderAll();
  }

  function focusCompareView() {
    closeDrawers();
    syncToolbarState();
    const resultsPanel = elements.resultsTableBody.closest(".table-wrap");
    if (resultsPanel) {
      resultsPanel.scrollIntoView({ behavior: "smooth", block: "nearest" });
    }
  }

  function toggleDrawer(targetDrawer) {
    const shouldOpen = !targetDrawer.open;
    closeDrawers();
    targetDrawer.open = shouldOpen;
    syncToolbarState();
  }

  function syncToolbarState() {
    const compareActive =
      !elements.setupDrawer.open &&
      !elements.filtersDrawer.open &&
      !elements.catalogDrawer.open &&
      !elements.importDrawer.open;
    elements.focusCompareButton.classList.toggle("is-active", compareActive);
    elements.toggleSetupButton.classList.toggle("is-active", elements.setupDrawer.open);
    elements.toggleFiltersButton.classList.toggle("is-active", elements.filtersDrawer.open);
    elements.toggleCatalogButton.classList.toggle("is-active", elements.catalogDrawer.open);
    elements.toggleImportButton.classList.toggle("is-active", elements.importDrawer.open);
  }

  function closeDrawers() {
    elements.setupDrawer.open = false;
    elements.filtersDrawer.open = false;
    elements.catalogDrawer.open = false;
    elements.importDrawer.open = false;
  }

  function restoreInspectorWidth() {
    try {
      const rawWidth = window.localStorage.getItem(INSPECTOR_WIDTH_STORAGE_KEY);
      const parsedWidth = toNumber(rawWidth);
      state.inspectorWidth = parsedWidth > 0 ? parsedWidth : null;
    } catch (error) {
      state.inspectorWidth = null;
    }
  }

  function restoreViewerPreferences() {
    let rawPreferences = null;

    try {
      rawPreferences = window.localStorage.getItem(VIEWER_PREFERENCES_STORAGE_KEY);
    } catch (error) {
      return;
    }

    if (!rawPreferences) {
      return;
    }

    let preferences = null;
    try {
      preferences = JSON.parse(rawPreferences);
    } catch (error) {
      return;
    }

    if (!preferences || typeof preferences !== "object" || Array.isArray(preferences)) {
      return;
    }

    state.activeMethodAId = readStoredString(preferences.activeMethodAId);
    state.activeMethodBId = readStoredString(preferences.activeMethodBId);
    state.methodSearch = normalizeSearchInput(preferences.methodSearch);

    const storedFilters = preferences.filters;
    if (storedFilters && typeof storedFilters === "object" && !Array.isArray(storedFilters)) {
      filters.series = readStoredDynamicOption(storedFilters.series, "all");
      filters.chapter = readStoredDynamicOption(storedFilters.chapter, "all");
      filters.status = readStoredEnum(storedFilters.status, ["all", "strict", "relaxed", "fail"], "all");
      filters.compare = readStoredEnum(
        storedFilters.compare,
        ["all", "changed", "b_better", "a_better", "same"],
        "all"
      );
      filters.bench = readStoredEnum(storedFilters.bench, ["all", "counted", "skipped"], "all");
      filters.sort = readStoredEnum(storedFilters.sort, ["page", "delta", "status", "confidence"], "page");
      filters.pageSearch = normalizeSearchInput(storedFilters.pageSearch);
    }

    syncRestoredControls();
  }

  function syncRestoredControls() {
    elements.methodSearchInput.value = state.methodSearch;
    elements.statusFilter.value = filters.status;
    elements.compareFilter.value = filters.compare;
    elements.benchFilter.value = filters.bench;
    elements.sortFilter.value = filters.sort;
    elements.pageSearchInput.value = filters.pageSearch;
  }

  function persistViewerPreferences() {
    const preferences = {
      activeMethodAId: state.activeMethodAId || "",
      activeMethodBId: state.activeMethodBId || "",
      methodSearch: state.methodSearch || "",
      filters: {
        series: filters.series,
        chapter: filters.chapter,
        status: filters.status,
        compare: filters.compare,
        bench: filters.bench,
        sort: filters.sort,
        pageSearch: filters.pageSearch,
      },
    };

    try {
      window.localStorage.setItem(VIEWER_PREFERENCES_STORAGE_KEY, JSON.stringify(preferences));
    } catch (error) {
      return;
    }
  }

  function persistInspectorWidth(width) {
    try {
      window.localStorage.setItem(INSPECTOR_WIDTH_STORAGE_KEY, String(Math.round(width)));
    } catch (error) {
      return;
    }
  }

  function isStackedWorkspace() {
    return window.innerWidth <= WORKSPACE_STACK_BREAKPOINT;
  }

  function getDefaultInspectorWidth() {
    return window.innerWidth <= NARROW_INSPECTOR_BREAKPOINT
      ? NARROW_INSPECTOR_WIDTH
      : DEFAULT_INSPECTOR_WIDTH;
  }

  function getInspectorWidthLimits() {
    const workspaceWidth = elements.workspace.clientWidth || elements.workspace.getBoundingClientRect().width || window.innerWidth;
    return {
      minWidth: MIN_INSPECTOR_WIDTH,
      maxWidth: Math.max(MIN_INSPECTOR_WIDTH, workspaceWidth - MIN_RESULTS_WIDTH - WORKSPACE_RESIZER_SIZE),
    };
  }

  function clampInspectorWidth(width, limits) {
    const activeLimits = limits || getInspectorWidthLimits();
    const numericWidth = toNumber(width);
    const safeWidth = numericWidth > 0 ? numericWidth : activeLimits.minWidth;
    return Math.min(Math.max(safeWidth, activeLimits.minWidth), activeLimits.maxWidth);
  }

  function setInspectorWidthStyle(width) {
    elements.workspace.style.setProperty("--inspector-width", width + "px");
  }

  function setDragInspectorWidthStyle(width) {
    elements.workspace.style.setProperty("--drag-inspector-width", width + "px");
  }

  function clearDragInspectorWidthStyle() {
    elements.workspace.style.removeProperty("--drag-inspector-width");
  }

  function freezeResultsWidth(width) {
    elements.results.style.flex = "0 0 " + width + "px";
    elements.results.style.width = width + "px";
  }

  function releaseResultsWidth() {
    elements.results.style.removeProperty("flex");
    elements.results.style.removeProperty("width");
  }

  function getCurrentInspectorWidth(limits) {
    return clampInspectorWidth(
      state.pendingInspectorWidth || state.inspectorWidth || getDefaultInspectorWidth(),
      limits
    );
  }

  function cancelWorkspaceResizeFrame() {
    if (!state.resizeAnimationFrame) {
      return;
    }
    window.cancelAnimationFrame(state.resizeAnimationFrame);
    state.resizeAnimationFrame = 0;
  }

  function flushWorkspaceResizeFrame() {
    state.resizeAnimationFrame = 0;
    if (!state.resizeSession) {
      return;
    }

    const nextWidth = clampInspectorWidth(
      state.resizeSession.pendingWidth,
      state.resizeSession.limits
    );
    state.pendingInspectorWidth = nextWidth;
    setDragInspectorWidthStyle(nextWidth);
    updateWorkspaceResizerAria(nextWidth, state.resizeSession.limits);
  }

  function scheduleWorkspaceResize(width) {
    if (!state.resizeSession) {
      return;
    }

    state.resizeSession.pendingWidth = clampInspectorWidth(width, state.resizeSession.limits);
    if (state.resizeAnimationFrame) {
      return;
    }

    state.resizeAnimationFrame = window.requestAnimationFrame(flushWorkspaceResizeFrame);
  }

  function updateWorkspaceResizerAria(currentWidth, limits) {
    const activeLimits = limits || getInspectorWidthLimits();
    elements.workspaceResizer.setAttribute("aria-valuemin", String(Math.round(activeLimits.minWidth)));
    elements.workspaceResizer.setAttribute("aria-valuemax", String(Math.round(activeLimits.maxWidth)));
    elements.workspaceResizer.setAttribute("aria-valuenow", String(Math.round(currentWidth)));
    if (isStackedWorkspace()) {
      elements.workspaceResizer.setAttribute("aria-hidden", "true");
      elements.workspaceResizer.tabIndex = -1;
      return;
    }
    elements.workspaceResizer.removeAttribute("aria-hidden");
    elements.workspaceResizer.tabIndex = 0;
  }

  function syncWorkspaceLayout() {
    const limits = getInspectorWidthLimits();
    const preferredWidth = state.inspectorWidth || getDefaultInspectorWidth();
    const nextWidth = clampInspectorWidth(preferredWidth, limits);
    state.pendingInspectorWidth = nextWidth;
    clearDragInspectorWidthStyle();
    setInspectorWidthStyle(nextWidth);
    updateWorkspaceResizerAria(nextWidth, limits);
  }

  function applyInspectorWidth(width, persist, limits) {
    const nextWidth = clampInspectorWidth(width, limits);
    state.inspectorWidth = nextWidth;
    state.pendingInspectorWidth = nextWidth;
    setInspectorWidthStyle(nextWidth);
    updateWorkspaceResizerAria(nextWidth, limits);
    if (persist) {
      persistInspectorWidth(nextWidth);
    }
    return nextWidth;
  }

  function handleViewportResize() {
    if (state.resizeSession) {
      handleWorkspaceResizeEnd();
    }
    syncWorkspaceLayout();
  }

  function handleWorkspaceResizeStart(event) {
    if (event.button !== 0 || !event.isPrimary || isStackedWorkspace()) {
      return;
    }

    const limits = getInspectorWidthLimits();
    const startWidth = clampInspectorWidth(
      elements.inspector.clientWidth || state.inspectorWidth || getDefaultInspectorWidth(),
      limits
    );

    state.resizeSession = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startWidth: startWidth,
      pendingWidth: startWidth,
      limits: limits,
      resultsWidth: elements.results.getBoundingClientRect().width,
    };

    state.pendingInspectorWidth = startWidth;
    freezeResultsWidth(state.resizeSession.resultsWidth);
    setDragInspectorWidthStyle(startWidth);
    elements.workspace.classList.add("is-resizing");
    document.body.classList.add("is-resizing");

    if (typeof elements.workspaceResizer.setPointerCapture === "function") {
      elements.workspaceResizer.setPointerCapture(event.pointerId);
    }

    event.preventDefault();
  }

  function handleWorkspaceResizeMove(event) {
    if (!state.resizeSession || event.pointerId !== state.resizeSession.pointerId) {
      return;
    }

    const deltaX = event.clientX - state.resizeSession.startX;
    scheduleWorkspaceResize(state.resizeSession.startWidth - deltaX);
    event.preventDefault();
  }

  function handleWorkspaceResizeEnd(event) {
    if (!state.resizeSession) {
      return;
    }

    if (event && event.pointerId !== undefined && event.pointerId !== state.resizeSession.pointerId) {
      return;
    }

    if (state.resizeAnimationFrame) {
      flushWorkspaceResizeFrame();
    }

    const pointerId = state.resizeSession.pointerId;
    const limits = state.resizeSession.limits;
    const finalWidth = clampInspectorWidth(
      state.pendingInspectorWidth || state.resizeSession.pendingWidth || state.resizeSession.startWidth,
      limits
    );

    setDragInspectorWidthStyle(finalWidth);
    state.resizeSession = null;
    cancelWorkspaceResizeFrame();

    if (
      typeof elements.workspaceResizer.releasePointerCapture === "function" &&
      typeof elements.workspaceResizer.hasPointerCapture === "function" &&
      elements.workspaceResizer.hasPointerCapture(pointerId)
    ) {
      elements.workspaceResizer.releasePointerCapture(pointerId);
    }

    if (!isStackedWorkspace()) {
      applyInspectorWidth(finalWidth, true, limits);
    } else {
      updateWorkspaceResizerAria(getCurrentInspectorWidth(limits), limits);
    }

    releaseResultsWidth();
    clearDragInspectorWidthStyle();
    elements.workspace.classList.remove("is-resizing");
    document.body.classList.remove("is-resizing");
  }

  function handleWorkspaceResizerKeyDown(event) {
    if (isStackedWorkspace()) {
      return;
    }

    const limits = getInspectorWidthLimits();
    const currentWidth = clampInspectorWidth(
      elements.inspector.clientWidth || state.inspectorWidth || getDefaultInspectorWidth(),
      limits
    );
    let nextWidth = null;

    if (event.key === "ArrowLeft") {
      nextWidth = currentWidth + RESIZER_KEYBOARD_STEP;
    } else if (event.key === "ArrowRight") {
      nextWidth = currentWidth - RESIZER_KEYBOARD_STEP;
    } else if (event.key === "Home") {
      nextWidth = limits.minWidth;
    } else if (event.key === "End") {
      nextWidth = limits.maxWidth;
    }

    if (nextWidth === null) {
      return;
    }

    applyInspectorWidth(nextWidth, true, limits);
    event.preventDefault();
  }

  function resetFilters() {
    filters.series = "all";
    filters.chapter = "all";
    filters.status = "all";
    filters.compare = "all";
    filters.bench = "all";
    filters.sort = "page";
    filters.pageSearch = "";
    elements.seriesFilter.value = "all";
    elements.chapterFilter.value = "all";
    elements.statusFilter.value = "all";
    elements.compareFilter.value = "all";
    elements.benchFilter.value = "all";
    elements.sortFilter.value = "page";
    elements.pageSearchInput.value = "";
    persistViewerPreferences();
    renderAll();
  }

  function selectFirstDiff() {
    const rows = buildVisibleRows();
    const target = rows.find(function (row) {
      return row.deltaKind !== "same";
    });
    if (!target) {
      setCatalogStatus("Aucun delta visible.", "warn");
      return;
    }
    state.selectedPath = target.relPath;
    renderAll();
  }

  function selectFirstFail() {
    const rows = buildVisibleRows();
    const target = rows.find(function (row) {
      return row.statusA === "fail";
    });
    if (!target) {
      setCatalogStatus("Aucun fail visible pour A.", "warn");
      return;
    }
    state.selectedPath = target.relPath;
    renderAll();
  }

  function handleResultsTableClick(event) {
    const row = event.target.closest("tr[data-path]");
    if (!row) {
      return;
    }
    state.selectedPath = row.dataset.path;
    renderAll();
  }

  function handleMethodCatalogClick(event) {
    const action = event.target.closest("button[data-action]");
    if (!action) {
      return;
    }

    const methodId = action.dataset.methodId;
    if (action.dataset.action === "set-a") {
      void activateMethodA(methodId);
      return;
    }

    if (action.dataset.action === "set-b") {
      void activateMethodB(methodId);
      return;
    }

    if (action.dataset.action === "compare-pair") {
      void compareMethodWithSoundtrackPair(methodId);
    }
  }

  function renderAll() {
    syncToolbarState();
    syncSelectOptions();
    syncSeriesOptions();
    syncChapterOptions();
    renderCatalogSummary();
    renderMethodCatalog();
    renderHero();
    renderPreview();
    renderResultsTable();
  }

  function syncSelectOptions() {
    const previousMethodAId = state.activeMethodAId;
    const previousMethodBId = state.activeMethodBId;
    const options = ["<option value=\"\">Aucune</option>"].concat(
      state.methods.map(function (method) {
        const suffix = isMethodLoaded(method) ? "" : " (lazy)";
        return "<option value=\"" + escapeAttribute(method.id) + "\">" +
          escapeHtml(method.label + suffix) +
        "</option>";
      })
    );

    elements.methodASelect.innerHTML = options.join("");
    elements.methodBSelect.innerHTML = options.join("");

    if (!state.methods.length) {
      elements.methodASelect.value = "";
      elements.methodBSelect.value = "";
      return;
    }

    if (!hasMethod(state.activeMethodAId)) {
      state.activeMethodAId = state.methods[0].id;
    }
    if (!hasMethod(state.activeMethodBId) || state.activeMethodBId === state.activeMethodAId) {
      state.activeMethodBId = "";
    }

    elements.methodASelect.value = state.activeMethodAId || "";
    elements.methodBSelect.value = state.activeMethodBId || "";

    if (state.activeMethodAId !== previousMethodAId || state.activeMethodBId !== previousMethodBId) {
      persistViewerPreferences();
    }
  }

  function syncSeriesOptions() {
    const series = getAvailableSeries();
    const previousSeries = filters.series;
    const options = ["<option value=\"all\">Tous</option>"].concat(
      series.map(function (entry) {
        return "<option value=\"" + escapeAttribute(entry) + "\">" + escapeHtml(entry) + "</option>";
      })
    );

    elements.seriesFilter.innerHTML = options.join("");
    if (!state.groundTruth) {
      elements.seriesFilter.value = "all";
      return;
    }
    elements.seriesFilter.value = series.includes(filters.series) ? filters.series : "all";
    filters.series = elements.seriesFilter.value;

    if (filters.series !== previousSeries) {
      persistViewerPreferences();
    }
  }

  function syncChapterOptions() {
    const chapters = getAvailableChapters(filters.series);
    const previousChapter = filters.chapter;
    const options = ["<option value=\"all\">Tous</option>"].concat(
      chapters.map(function (chapter) {
        return "<option value=\"" + escapeAttribute(chapter) + "\">" + escapeHtml(chapter) + "</option>";
      })
    );

    elements.chapterFilter.innerHTML = options.join("");
    if (!state.groundTruth) {
      elements.chapterFilter.value = "all";
      elements.chapterFilter.disabled = true;
      return;
    }
    elements.chapterFilter.value = chapters.includes(filters.chapter) ? filters.chapter : "all";
    filters.chapter = elements.chapterFilter.value;
    elements.chapterFilter.disabled = filters.series === "all";

    if (filters.chapter !== previousChapter) {
      persistViewerPreferences();
    }
  }

  function renderCatalogSummary() {
    const gtCount = state.groundTruth ? Object.keys(state.groundTruth).length : 0;
    const methodCount = state.methods.length;
    const activeA = getMethodById(state.activeMethodAId);
    const activeB = getMethodById(state.activeMethodBId);

    const parts = [];
    parts.push(gtCount + " pages");
    parts.push(methodCount + " methodes");
    if (activeA) {
      parts.push("A: " + activeA.label);
    }
    if (activeB) {
      parts.push("B: " + activeB.label);
    }

    elements.catalogSummary.textContent = parts.join(" · ");
  }

  function renderMethodCatalog() {
    const methods = getFilteredMethods();
    if (!methods.length) {
      elements.methodsTableBody.innerHTML =
        "<tr><td colspan=\"3\" class=\"empty\">Aucune methode ne correspond a la recherche.</td></tr>";
      return;
    }

    const html = methods.map(function (method) {
      const ownStats = computeMethodOwnStats(method);
      const pairMethodId = getSoundtrackPairMethodId(method.id);
      const pairLabel = isSoundtrackCommitSourcePath(method.sourcePath)
        ? "Compare Base"
        : "Compare OST";
      const rowClass = method.id === state.activeMethodAId
        ? "method-a"
        : method.id === state.activeMethodBId
          ? "method-b"
          : "";
      const lazySuffix = isMethodLoaded(method) ? "" : " | lazy";
      return (
        "<tr class=\"" + rowClass + "\">" +
          "<td><strong>" + escapeHtml(method.label) + "</strong>" +
            "<div class=\"note\">strict " + ownStats.strict + "/" + ownStats.counted +
              " | relax " + ownStats.relaxedPass + "/" + ownStats.counted + lazySuffix + "</div>" +
            "<div class=\"mono\">" + escapeHtml(method.sourcePath) + "</div></td>" +
          "<td>" + escapeHtml(String(ownStats.coverage)) + "</td>" +
          "<td><div class=\"method-actions\">" +
            "<button type=\"button\" data-action=\"set-a\" data-method-id=\"" + escapeAttribute(method.id) + "\">Set A</button>" +
            "<button type=\"button\" data-action=\"set-b\" data-method-id=\"" + escapeAttribute(method.id) + "\">Set B</button>" +
            (pairMethodId
              ? "<button type=\"button\" data-action=\"compare-pair\" data-method-id=\"" + escapeAttribute(method.id) + "\">" + escapeHtml(pairLabel) + "</button>"
              : "") +
          "</div></td>" +
        "</tr>"
      );
    });

    elements.methodsTableBody.innerHTML = html.join("");
  }

  function renderHero() {
    const methodA = getMethodById(state.activeMethodAId);
    const methodB = getMethodById(state.activeMethodBId);
    const rows = buildVisibleRows();

    if (!methodA) {
      elements.heroTitle.textContent = "Aucune methode A selectionnee";
      elements.heroSubtitle.textContent = "Choisis une methode A dans Configurer.";
      elements.heroChip.textContent = "Comparer";
      elements.summaryCards.innerHTML = "";
      return;
    }

    elements.heroTitle.textContent = methodB
      ? methodA.label + " vs " + methodB.label
      : methodA.label;

    if (!isMethodLoaded(methodA)) {
      elements.heroSubtitle.textContent = "Chargement des predictions de A...";
      elements.heroChip.textContent = "Scope A · " + methodAScopeCount(methodA);
      elements.summaryCards.innerHTML = "";
      return;
    }

    const stats = computeVisibleStats(rows);
    if (methodB && isMethodLoaded(methodB)) {
      elements.heroSubtitle.textContent = [
        rows.length + " pages visibles",
        "A relax " + formatFraction(stats.aRelaxed, stats.counted),
        "B relax " + formatFraction(stats.bRelaxed, stats.counted),
        "delta B+ " + stats.bBetter + " / A+ " + stats.aBetter,
      ].join(" · ");
    } else {
      elements.heroSubtitle.textContent = [
        rows.length + " pages visibles",
        "bench " + stats.counted,
        "A strict " + formatFraction(stats.aStrict, stats.counted),
        "A fail " + formatFraction(stats.aFail, stats.counted),
      ].join(" · ");
    }
    elements.heroChip.textContent = "Scope A · " + methodAScopeCount(methodA);
    elements.summaryCards.innerHTML = "";
  }

  function renderPreview() {
    const rows = buildVisibleRows();
    const methodA = getMethodById(state.activeMethodAId);

    if (methodA && !isMethodLoaded(methodA)) {
      elements.previewTitle.textContent = "Chargement";
      elements.previewCounter.textContent = "0 / 0";
      elements.previewFigure.innerHTML =
        "<div class=\"placeholder\">Le viewer charge la methode A selectionnee.</div>";
      elements.pageMethodTableBody.innerHTML =
        "<tr><td colspan=\"4\" class=\"empty\">Les methodes chargees apparaitront ici.</td></tr>";
      return;
    }

    const selectedRow = resolveSelectedRow(rows);
    if (!selectedRow) {
      elements.previewTitle.textContent = "Aucune page selectionnee";
      elements.previewCounter.textContent = "0 / 0";
      elements.previewFigure.innerHTML =
        "<div class=\"placeholder\">Aucune page visible avec les filtres courants.</div>";
      elements.pageMethodTableBody.innerHTML =
        "<tr><td colspan=\"4\" class=\"empty\">Aucune methode chargee.</td></tr>";
      return;
    }

    const rowIndex = rows.findIndex(function (row) {
      return row.relPath === selectedRow.relPath;
    });
    elements.previewTitle.textContent = selectedRow.relPath;
    elements.previewCounter.textContent = rowIndex + 1 + " / " + rows.length;
    const methodB = getMethodById(state.activeMethodBId);
    const pills = [
      renderPreviewPill("GT " + selectedRow.expectedTag, ""),
      renderPreviewPill("A " + selectedRow.actualTagA, "status-" + selectedRow.statusA),
    ];
    if (selectedRow.compareEnabled) {
      pills.push(renderPreviewPill(selectedRow.deltaLabel, deltaClass(selectedRow.deltaKind)));
    } else if (state.activeMethodBId) {
      pills.push(renderPreviewPill("B loading", ""));
    }
    elements.previewFigure.innerHTML =
      "<div class=\"preview-shell\">" +
        "<div class=\"preview-top\">" +
          "<div class=\"preview-pills\">" + pills.join("") + "</div>" +
          "<div class=\"preview-actions\">" +
            "<div class=\"info-wrap\">" +
              "<button class=\"info-button\" type=\"button\" aria-label=\"Voir les informations\">" + INFO_ICON + "</button>" +
              "<div class=\"info-popover\">" + buildPreviewInfoHtml(selectedRow, methodA, methodB, rows.length, rowIndex + 1) + "</div>" +
            "</div>" +
          "</div>" +
        "</div>" +
        "<div class=\"preview-media\">" +
          "<img src=\"" + escapeAttribute(selectedRow.imageUrl) + "\" alt=\"" + escapeAttribute(selectedRow.relPath) + "\">" +
        "</div>" +
      "</div>";
    renderPageMethods(selectedRow.relPath);
  }

  function renderPill(text, extraClass) {
    return "<span class=\"pill " + extraClass + "\">" + escapeHtml(text) + "</span>";
  }

  function renderPreviewPill(text, extraClass) {
    return "<span class=\"preview-pill " + extraClass + "\">" + escapeHtml(text) + "</span>";
  }

  function buildPreviewInfoHtml(selectedRow, methodA, methodB, totalRows, currentIndex) {
    return [
      "<div class=\"info-title\">Informations</div>",
      "<div class=\"info-grid\">",
      renderPreviewInfoRow("Serie", selectedRow.series),
      renderPreviewInfoRow("Chapitre", selectedRow.chapterKey),
      renderPreviewInfoRow("Fichier", "<span class=\"mono\">" + escapeHtml(selectedRow.pageName) + "</span>", true),
      renderPreviewInfoRow("Chemin", "<span class=\"mono\">" + escapeHtml(selectedRow.relPath) + "</span>", true),
      renderPreviewInfoRow("Page", currentIndex + " / " + totalRows),
      renderPreviewInfoRow("Bench", selectedRow.counted ? "counted" : "skipped"),
      renderPreviewInfoRow("Confidence", formatConfidence(selectedRow.confidence)),
      renderPreviewInfoRow("GT", selectedRow.expectedTag),
      renderPreviewInfoRow(methodA ? "A" : "A", selectedRow.actualTagA + " · " + selectedRow.statusA),
      renderPreviewInfoRow("A method", methodA ? methodA.label : "aucune"),
      selectedRow.compareEnabled
        ? renderPreviewInfoRow("B", selectedRow.actualTagB + " · " + selectedRow.statusB)
        : "",
      methodB
        ? renderPreviewInfoRow("B method", methodB.label + (selectedRow.compareEnabled ? "" : " · loading"))
        : "",
      selectedRow.compareEnabled
        ? renderPreviewInfoRow("Delta", selectedRow.deltaLabel)
        : "",
      "</div>",
      "<div class=\"info-reasoning\">",
      "<div class=\"info-title\">Reasoning</div>",
      "<div class=\"info-value\">" + escapeHtml(selectedRow.reasoning || "Aucun reasoning dans le ground truth.") + "</div>",
      "</div>",
    ].join("");
  }

  function renderPreviewInfoRow(label, value, rawHtml) {
    const content = rawHtml ? value : escapeHtml(value);
    return (
      "<div class=\"info-row\">" +
        "<div class=\"info-key\">" + escapeHtml(label) + "</div>" +
        "<div class=\"info-value\">" + content + "</div>" +
      "</div>"
    );
  }

  function hasActiveComparison() {
    return Boolean(state.activeMethodBId);
  }

  function resultsTableColumnCount(showComparisonColumns) {
    return showComparisonColumns ? 6 : 4;
  }

  function renderResultsTableHead(showComparisonColumns) {
    const headers = ["Img", "Page", "GT", "A"];
    if (showComparisonColumns) {
      headers.push("B", "Delta");
    }

    elements.resultsTableHead.innerHTML =
      "<tr>" + headers.map(function (label) {
        return "<th>" + escapeHtml(label) + "</th>";
      }).join("") + "</tr>";
  }

  function renderResultsTableEmpty(message, showComparisonColumns) {
    elements.resultsTableBody.innerHTML =
      "<tr><td colspan=\"" + resultsTableColumnCount(showComparisonColumns) + "\" class=\"empty\">" +
        escapeHtml(message) +
      "</td></tr>";
  }

  function handleResultsTableScroll() {
    if (state.resultsScrollFrame) {
      return;
    }

    state.resultsScrollFrame = window.requestAnimationFrame(function () {
      state.resultsScrollFrame = 0;
      renderResultsTable(state.cachedResultsRows);
    });
  }

  function renderResultsSpacer(height, showComparisonColumns) {
    if (height <= 0) {
      return "";
    }

    return (
      "<tr class=\"spacer-row\" aria-hidden=\"true\">" +
        "<td colspan=\"" + resultsTableColumnCount(showComparisonColumns) + "\" style=\"height:" + Math.round(height) + "px\"></td>" +
      "</tr>"
    );
  }

  function renderResultsTableRow(row, showComparisonColumns) {
    const isSelected = row.relPath === state.selectedPath;
    const pageMeta = row.chapterKey;
    const cells = [
      "<td><img class=\"thumb\" loading=\"lazy\" src=\"" + escapeAttribute(row.imageUrl) + "\" alt=\"" + escapeAttribute(row.pageName) + "\"></td>",
      "<td><div class=\"row-page\"><div class=\"row-page-title\">" + escapeHtml(row.pageName) + "</div><div class=\"row-page-meta\">" + escapeHtml(pageMeta) + "</div></div></td>",
      "<td><span class=\"tag\">" + escapeHtml(row.expectedTag) + "</span></td>",
      "<td><span class=\"tag status-" + row.statusA + "\">" + escapeHtml(row.actualTagA) + "</span></td>",
    ];

    if (showComparisonColumns) {
      cells.push(
        "<td>" + (row.compareEnabled
          ? "<span class=\"tag status-" + row.statusB + "\">" + escapeHtml(row.actualTagB) + "</span>"
          : "<span class=\"mono\">-</span>") + "</td>"
      );
      cells.push(
        "<td>" + (row.compareEnabled
          ? "<span class=\"badge " + deltaClass(row.deltaKind) + "\">" + escapeHtml(row.deltaLabel) + "</span>"
          : "<span class=\"mono\">-</span>") + "</td>"
      );
    }

    return (
      "<tr data-path=\"" + escapeAttribute(row.relPath) + "\" data-selected=\"" + (isSelected ? "true" : "false") + "\">" +
        cells.join("") +
      "</tr>"
    );
  }

  function scheduleResultsRowHeightMeasure(showComparisonColumns) {
    if (state.resultsMeasureFrame) {
      return;
    }

    state.resultsMeasureFrame = window.requestAnimationFrame(function () {
      state.resultsMeasureFrame = 0;
      const measuredRow = elements.resultsTableBody.querySelector("tr[data-path]");
      if (!measuredRow) {
        return;
      }

      const nextHeight = Math.ceil(measuredRow.getBoundingClientRect().height);
      if (nextHeight > 0 && Math.abs(nextHeight - state.resultsRowHeight) > 1) {
        state.resultsRowHeight = nextHeight;
        renderResultsTable(state.cachedResultsRows);
      }
    });
  }

  function renderPageMethods(relPath) {
    const expected = state.groundTruth && state.groundTruth[relPath];
    if (!expected || !state.methods.length) {
      elements.pageMethodTableBody.innerHTML =
        "<tr><td colspan=\"4\" class=\"empty\">Aucune methode chargee.</td></tr>";
      return;
    }

    const loadedMethods = state.methods.filter(isMethodLoaded);
    if (!loadedMethods.length) {
      elements.pageMethodTableBody.innerHTML =
        "<tr><td colspan=\"4\" class=\"empty\">Aucune methode chargee pour l'instant.</td></tr>";
      return;
    }

    const orderedMethods = loadedMethods.slice().sort(function (left, right) {
      if (left.id === state.activeMethodAId) {
        return -1;
      }
      if (right.id === state.activeMethodAId) {
        return 1;
      }
      if (left.id === state.activeMethodBId) {
        return -1;
      }
      if (right.id === state.activeMethodBId) {
        return 1;
      }
      return compareMethods(left, right);
    });

    const rows = orderedMethods.map(function (method) {
      const prediction = method.predictions[relPath] || null;
      const status = classifyStatus(expected, prediction);
      const intensityMatch = prediction ? prediction.intensity === expected.intensity : false;
      return (
        "<tr>" +
          "<td><strong>" + escapeHtml(method.label) + "</strong></td>" +
          "<td><span class=\"tag\">" + escapeHtml(prediction ? prediction.raw : "missing") + "</span></td>" +
          "<td><span class=\"badge status-" + status + "\">" + escapeHtml(status) + "</span></td>" +
          "<td>" + escapeHtml(intensityMatch ? "match" : "miss") + "</td>" +
        "</tr>"
      );
    });

    const unloadedCount = state.methods.length - loadedMethods.length;
    if (unloadedCount > 0) {
      rows.push(
        "<tr><td colspan=\"4\" class=\"empty\">" +
          escapeHtml(String(unloadedCount)) +
          " autre(s) methode(s) non chargee(s) pour l'instant. Choisis-les en A/B pour les charger." +
        "</td></tr>"
      );
    }

    elements.pageMethodTableBody.innerHTML = rows.join("");
  }

  function renderResultsTable(cachedRows) {
    const methodA = getMethodById(state.activeMethodAId);
    const rows = cachedRows !== undefined ? cachedRows : buildVisibleRows();
    const showComparisonColumns = hasActiveComparison();
    state.cachedResultsRows = rows;

    elements.resultsTitle.textContent = buildScopeTitle();
    elements.resultsCountLine.textContent =
      rows.length + " page(s) visibles";
    renderResultsTableHead(showComparisonColumns);

    if (!methodA) {
      renderResultsTableEmpty("Choisis une methode A.", showComparisonColumns);
      return;
    }

    if (!isMethodLoaded(methodA)) {
      renderResultsTableEmpty("Chargement de la methode A...", showComparisonColumns);
      return;
    }

    if (!rows.length) {
      renderResultsTableEmpty("Aucune ligne visible.", showComparisonColumns);
      return;
    }

    const viewportHeight = elements.resultsTableWrap.clientHeight || 0;
    const rowHeight = state.resultsRowHeight || ESTIMATED_RESULTS_ROW_HEIGHT;
    const maxScrollTop = Math.max(0, rows.length * rowHeight - viewportHeight);
    if (elements.resultsTableWrap.scrollTop > maxScrollTop) {
      elements.resultsTableWrap.scrollTop = maxScrollTop;
    }

    const scrollTop = elements.resultsTableWrap.scrollTop;
    const visibleCount = viewportHeight > 0
      ? Math.ceil(viewportHeight / rowHeight) + (RESULTS_TABLE_OVERSCAN * 2)
      : Math.min(rows.length, 40);
    const startIndex = viewportHeight > 0
      ? Math.max(0, Math.floor(scrollTop / rowHeight) - RESULTS_TABLE_OVERSCAN)
      : 0;
    const endIndex = Math.min(rows.length, startIndex + visibleCount);
    const topHeight = startIndex * rowHeight;
    const bottomHeight = Math.max(0, (rows.length - endIndex) * rowHeight);
    const visibleRows = rows.slice(startIndex, endIndex);

    const html = [];
    if (topHeight > 0) {
      html.push(renderResultsSpacer(topHeight, showComparisonColumns));
    }
    html.push(
      visibleRows.map(function (row) {
        return renderResultsTableRow(row, showComparisonColumns);
      }).join("")
    );
    if (bottomHeight > 0) {
      html.push(renderResultsSpacer(bottomHeight, showComparisonColumns));
    }

    elements.resultsTableBody.innerHTML = html.join("");
    scheduleResultsRowHeightMeasure(showComparisonColumns);
  }

  function buildVisibleRows() {
    const methodA = getMethodById(state.activeMethodAId);
    if (!methodA || !state.groundTruth || !isMethodLoaded(methodA)) {
      return [];
    }

    const methodB = getMethodById(state.activeMethodBId);
    const baseRows = buildRowsOnActiveScope(methodA, methodB && isMethodLoaded(methodB) ? methodB : null);
    return filterAndSortRows(baseRows);
  }

  function buildRowsOnActiveScope(methodA, methodB) {
    return Object.keys(state.groundTruth)
      .map(function (relPath) {
        const expected = state.groundTruth[relPath];
        const predictionA = methodA.predictions[relPath] || null;
        const predictionB = methodB ? methodB.predictions[relPath] || null : null;
        const parts = splitRelPath(relPath);
        const statusA = classifyStatus(expected, predictionA);
        const statusB = methodB ? classifyStatus(expected, predictionB) : "";
        const deltaKind = methodB ? compareStatus(statusA, statusB, predictionA, predictionB) : "same";
        return {
          relPath: relPath,
          series: parts.series,
          chapter: parts.chapter,
          chapterKey: parts.series && parts.chapter ? parts.series + "/" + parts.chapter : relPath,
          pageName: parts.page,
          expectedTag: formatTag(expected.mood, expected.intensity),
          actualTagA: predictionA ? predictionA.raw : "missing",
          actualTagB: methodB ? (predictionB ? predictionB.raw : "missing") : "-",
          statusA: statusA,
          statusB: statusB,
          deltaKind: deltaKind,
          deltaLabel: deltaLabel(deltaKind),
          counted: expected.confidence > 0,
          confidence: expected.confidence,
          reasoning: expected.reasoning,
          compareEnabled: Boolean(methodB),
          intensityMatchA: predictionA ? predictionA.intensity === expected.intensity : false,
          imageUrl: buildImageUrl(relPath),
        };
      });
  }

  function filterAndSortRows(rows) {
    const filtered = rows.filter(function (row) {
      if (filters.series !== "all" && row.series !== filters.series) {
        return false;
      }
      if (filters.chapter !== "all" && row.chapter !== filters.chapter) {
        return false;
      }
      if (filters.status !== "all" && row.statusA !== filters.status) {
        return false;
      }
      if (filters.bench === "counted" && !row.counted) {
        return false;
      }
      if (filters.bench === "skipped" && row.counted) {
        return false;
      }
      if (row.compareEnabled && filters.compare !== "all") {
        if (filters.compare === "changed" && row.deltaKind === "same") {
          return false;
        }
        if (filters.compare === "b_better" && row.deltaKind !== "b_better") {
          return false;
        }
        if (filters.compare === "a_better" && row.deltaKind !== "a_better") {
          return false;
        }
        if (filters.compare === "same" && row.deltaKind !== "same") {
          return false;
        }
      }
      if (filters.pageSearch) {
        const haystack = [
          row.relPath,
          row.expectedTag,
          row.actualTagA,
          row.actualTagB,
          row.statusA,
          row.statusB,
          row.reasoning,
        ].join(" ").toLowerCase();
        if (!haystack.includes(filters.pageSearch)) {
          return false;
        }
      }
      return true;
    });

    filtered.sort(sortVisibleRows);
    return filtered;
  }

  function buildScopeTitle() {
    if (filters.series === "all") {
      return "Toutes les pages";
    }
    if (filters.chapter === "all") {
      return filters.series;
    }
    return filters.series + " · chapitre " + filters.chapter;
  }

  function sortVisibleRows(left, right) {
    if (filters.sort === "delta") {
      if (DELTA_SORT[left.deltaKind] !== DELTA_SORT[right.deltaKind]) {
        return DELTA_SORT[left.deltaKind] - DELTA_SORT[right.deltaKind];
      }
      return compareRelPath(left.relPath, right.relPath);
    }

    if (filters.sort === "status") {
      if (STATUS_SCORE[left.statusA] !== STATUS_SCORE[right.statusA]) {
        return STATUS_SCORE[left.statusA] - STATUS_SCORE[right.statusA];
      }
      return compareRelPath(left.relPath, right.relPath);
    }

    if (filters.sort === "confidence") {
      if (right.confidence !== left.confidence) {
        return right.confidence - left.confidence;
      }
      return compareRelPath(left.relPath, right.relPath);
    }

    return compareRelPath(left.relPath, right.relPath);
  }

  function computeVisibleStats(rows) {
    const countedRows = rows.filter(function (row) {
      return row.counted;
    });

    const stats = {
      counted: countedRows.length,
      aStrict: 0,
      aRelaxed: 0,
      aFail: 0,
      aIntensity: 0,
      bStrict: 0,
      bRelaxed: 0,
      bFail: 0,
      bBetter: 0,
      aBetter: 0,
      changed: 0,
      same: 0,
    };

    for (const row of countedRows) {
      if (row.statusA === "strict") {
        stats.aStrict += 1;
      }
      if (row.statusA === "strict" || row.statusA === "relaxed") {
        stats.aRelaxed += 1;
      } else {
        stats.aFail += 1;
      }
      if (row.intensityMatchA) {
        stats.aIntensity += 1;
      }
      if (row.compareEnabled) {
        if (row.statusB === "strict") {
          stats.bStrict += 1;
        }
        if (row.statusB === "strict" || row.statusB === "relaxed") {
          stats.bRelaxed += 1;
        } else {
          stats.bFail += 1;
        }
        if (row.deltaKind === "b_better") {
          stats.bBetter += 1;
        } else if (row.deltaKind === "a_better") {
          stats.aBetter += 1;
        }
        if (row.deltaKind === "same") {
          stats.same += 1;
        } else {
          stats.changed += 1;
        }
      }
    }

    return stats;
  }

  function computeMethodOwnStats(method) {
    if (isMethodLoaded(method)) {
      return computeMethodOwnStatsFromPredictions(method.predictions);
    }

    return {
      coverage: toNumber(method.pageCount),
      counted: toNumber(method.counted),
      strict: toNumber(method.strict),
      relaxedPass: toNumber(method.relaxedPass),
    };
  }

  function computeMethodOwnStatsFromPredictions(predictions) {
    if (!state.groundTruth) {
      return { coverage: Object.keys(predictions).length, counted: 0, strict: 0, relaxedPass: 0 };
    }

    const rows = Object.keys(predictions)
      .filter(function (relPath) {
        return Object.prototype.hasOwnProperty.call(state.groundTruth, relPath);
      })
      .map(function (relPath) {
        const expected = state.groundTruth[relPath];
        const prediction = predictions[relPath];
        return {
          counted: expected.confidence > 0,
          status: classifyStatus(expected, prediction),
        };
      });

    const countedRows = rows.filter(function (row) {
      return row.counted;
    });
    const strict = countedRows.filter(function (row) {
      return row.status === "strict";
    }).length;
    const relaxedPass = countedRows.filter(function (row) {
      return row.status === "strict" || row.status === "relaxed";
    }).length;

    return {
      coverage: rows.length,
      counted: countedRows.length,
      strict: strict,
      relaxedPass: relaxedPass,
    };
  }

  function classifyStatus(expected, prediction) {
    if (!prediction || !prediction.mood) {
      return "fail";
    }
    if (prediction.mood === expected.mood) {
      return "strict";
    }
    if (moodFamily(prediction.mood) === moodFamily(expected.mood)) {
      return "relaxed";
    }
    return "fail";
  }

  function compareStatus(statusA, statusB, predictionA, predictionB) {
    const scoreA = STATUS_SCORE[statusA];
    const scoreB = STATUS_SCORE[statusB];

    if (scoreB > scoreA) {
      return "b_better";
    }
    if (scoreA > scoreB) {
      return "a_better";
    }
    const rawA = predictionA ? predictionA.raw : "missing";
    const rawB = predictionB ? predictionB.raw : "missing";
    return rawA === rawB ? "same" : "changed";
  }

  function deltaLabel(kind) {
    if (kind === "b_better") {
      return "B > A";
    }
    if (kind === "a_better") {
      return "A > B";
    }
    if (kind === "changed") {
      return "Changed";
    }
    return "Same";
  }

  function deltaClass(kind) {
    if (kind === "b_better") {
      return "delta-better";
    }
    if (kind === "a_better") {
      return "delta-worse";
    }
    return "delta-same";
  }

  function moodFamily(mood) {
    return Object.prototype.hasOwnProperty.call(MOOD_FAMILY, mood) ? MOOD_FAMILY[mood] : -1;
  }

  function resolveSelectedRow(rows) {
    if (!rows.length) {
      state.selectedPath = "";
      return null;
    }

    const selected = rows.find(function (row) {
      return row.relPath === state.selectedPath;
    });
    if (selected) {
      return selected;
    }

    state.selectedPath = rows[0].relPath;
    return rows[0];
  }

  function getFilteredMethods() {
    if (!state.methodSearch) {
      return state.methods;
    }
    return state.methods.filter(function (method) {
      const haystack = [method.label, method.sourcePath].join(" ").toLowerCase();
      return haystack.includes(state.methodSearch);
    });
  }

  function getAvailableChapters(seriesId) {
    const chapters = new Set();
    for (const relPath of getActiveScopePaths()) {
      const parts = splitRelPath(relPath);
      if (seriesId !== "all" && parts.series !== seriesId) {
        continue;
      }
      if (parts.chapter) {
        chapters.add(parts.chapter);
      }
    }

    return Array.from(chapters).sort(function (left, right) {
      return left.localeCompare(right, undefined, { numeric: true, sensitivity: "base" });
    });
  }

  function getAvailableSeries() {
    const series = new Set();
    for (const relPath of getActiveScopePaths()) {
      const parts = splitRelPath(relPath);
      if (parts.series) {
        series.add(parts.series);
      }
    }

    return Array.from(series).sort();
  }

  function getActiveScopePaths() {
    if (!state.groundTruth) {
      return [];
    }

    return Object.keys(state.groundTruth);
  }

  function getMethodById(methodId) {
    return state.methods.find(function (method) {
      return method.id === methodId;
    }) || null;
  }

  function methodAScopeCount(method) {
    if (!method || !state.groundTruth) {
      return 0;
    }
    if (!isMethodLoaded(method)) {
      return toNumber(method.pageCount);
    }
    return Object.keys(method.predictions).filter(function (relPath) {
      return Object.prototype.hasOwnProperty.call(state.groundTruth, relPath);
    }).length;
  }

  function readStoredString(value) {
    return typeof value === "string" ? value.trim() : "";
  }

  function readStoredDynamicOption(value, fallback) {
    const normalized = readStoredString(value);
    return normalized || fallback;
  }

  function readStoredEnum(value, allowedValues, fallback) {
    return allowedValues.includes(value) ? value : fallback;
  }

  function normalizeSearchInput(value) {
    return typeof value === "string" ? value.trim().toLowerCase() : "";
  }

  function splitRelPath(relPath) {
    const parts = String(relPath).split("/");
    return {
      series: parts[0] || "",
      chapter: parts[1] || "",
      page: parts.slice(2).join("/") || "",
    };
  }

  function compareRelPath(left, right) {
    const a = splitRelPath(left);
    const b = splitRelPath(right);

    if (a.series !== b.series) {
      return a.series.localeCompare(b.series);
    }
    if (compareMaybeNumeric(a.chapter, b.chapter) !== 0) {
      return compareMaybeNumeric(a.chapter, b.chapter);
    }
    return compareMaybeNumeric(a.page, b.page);
  }

  function compareMaybeNumeric(left, right) {
    const leftNum = leadingNumber(left);
    const rightNum = leadingNumber(right);
    if (leftNum !== null && rightNum !== null && leftNum !== rightNum) {
      return leftNum - rightNum;
    }
    return String(left).localeCompare(String(right));
  }

  function leadingNumber(value) {
    const match = String(value).match(/^(\d+)/);
    return match ? Number(match[1]) : null;
  }

  function buildImageUrl(relPath) {
    let root = state.imageRoot || "../test-images/RealTest/";
    if (!root.endsWith("/")) {
      root += "/";
    }
    return new URL(root + relPath, window.location.href).href;
  }

  function buildReferenceSoundtrackCommitPredictions(groundTruth) {
    const predictions = {};
    const chapters = buildGroundTruthChapterBuckets(groundTruth);
    const committer = createSoundtrackCommitter();

    for (const chapter of chapters) {
      for (let index = 0; index < chapter.pages.length; index += 1) {
        const current = chapter.pages[index];
        const futureContext = chapter.pages
          .slice(index + 1, index + 3)
          .map(createSoundtrackObservationFromGroundTruthPage);
        const decision = updateSoundtrackCommitter(
          committer,
          chapter.chapterKey,
          createSoundtrackObservationFromGroundTruthPage(current),
          futureContext
        );
        predictions[current.relPath] = {
          mood: decision.committedMood,
          intensity: decision.committedIntensity,
          raw: formatTag(decision.committedMood, decision.committedIntensity),
        };
      }
    }

    return predictions;
  }

  function buildGroundTruthChapterBuckets(groundTruth) {
    const chapterMap = new Map();
    const relPaths = Object.keys(groundTruth).sort(compareRelPath);
    for (const relPath of relPaths) {
      const entry = groundTruth[relPath];
      const parts = splitRelPath(relPath);
      const chapterKey = parts.series && parts.chapter ? parts.series + "/" + parts.chapter : relPath;
      if (!chapterMap.has(chapterKey)) {
        chapterMap.set(chapterKey, []);
      }
      chapterMap.get(chapterKey).push({
        relPath: relPath,
        pageNumber: leadingNumber(parts.page),
        mood: entry.mood,
        intensity: toNumber(entry.intensity),
      });
    }

    return Array.from(chapterMap.entries())
      .map(function (entry) {
        return { chapterKey: entry[0], pages: entry[1] };
      })
      .sort(function (left, right) {
        return left.chapterKey.localeCompare(right.chapterKey);
      });
  }

  function createSoundtrackObservationFromGroundTruthPage(page) {
    return {
      page: Number.isFinite(page.pageNumber) ? page.pageNumber : null,
      mood: page.mood,
      intensity: toNumber(page.intensity),
      scores: createOneHotScores(page.mood),
      narrativeRole: inferNarrativeRole(page.mood, page.intensity),
    };
  }

  function createOneHotScores(mood) {
    const scores = {};
    for (const entry of ALL_MOODS) {
      scores[entry] = entry === mood ? 1 : 0;
    }
    return scores;
  }

  function createSoundtrackCommitter() {
    return {
      currentChapter: "",
      committedMood: "",
      committedIntensity: 0,
      dwellCount: 0,
      recent: [],
      lastPage: null,
    };
  }

  function updateSoundtrackCommitter(committer, chapterKey, observation, futureContext) {
    syncSoundtrackChapter(committer, chapterKey);

    const previousMood = committer.committedMood || "";
    const previousIntensity = committer.committedIntensity || 0;

    if (observation.page != null && observation.page === committer.lastPage) {
      return {
        committedMood: committer.committedMood || observation.mood,
        committedIntensity: committer.committedIntensity || observation.intensity,
        moodChanged: false,
        intensityChanged: false,
        previousMood: previousMood || null,
        previousIntensity: previousIntensity || null,
        dwellCount: Math.max(committer.dwellCount, 1),
      };
    }

    clearSoundtrackRecentIfDiscontinuous(committer, observation.page);
    pushSoundtrackRecent(committer, soundtrackRecentObservation(observation));
    committer.lastPage = observation.page;

    const futureRecent = collectSoundtrackFutureContext(observation.page, futureContext);
    const contextScores = computeSoundtrackContextScores(committer.recent, observation.page, futureRecent);
    const hasFutureContext = futureRecent.length > 0;

    if (!committer.committedMood || !committer.committedIntensity) {
      committer.committedMood = observation.mood;
      committer.committedIntensity = observation.intensity;
      committer.dwellCount = 1;
      return {
        committedMood: observation.mood,
        committedIntensity: observation.intensity,
        moodChanged: previousMood !== observation.mood,
        intensityChanged: previousIntensity !== observation.intensity,
        previousMood: previousMood || null,
        previousIntensity: previousIntensity || null,
        dwellCount: committer.dwellCount,
      };
    }

    const currentMood = committer.committedMood;
    const currentIntensity = committer.committedIntensity;
    const committedFamily = soundtrackFamilyOf(currentMood);
    const candidateFamily = dominantSoundtrackFamily(observation.scores, observation.mood);

    let nextMood = currentMood;
    let nextIntensity = currentIntensity;

    if (candidateFamily !== committedFamily) {
      const targetFamily = resolveSoundtrackFamilySwitchTarget(
        committer,
        committedFamily,
        candidateFamily,
        observation,
        contextScores,
        futureRecent,
        hasFutureContext
      );
      if (targetFamily) {
        nextMood = selectCrossFamilySoundtrackMood(targetFamily, contextScores);
        nextIntensity = selectCrossFamilySoundtrackIntensity(
          committer,
          nextMood,
          targetFamily,
          observation,
          futureRecent
        );
      }
    } else if (
      observation.mood !== currentMood &&
      shouldSwitchSoundtrackMood(committer, currentMood, observation, contextScores, hasFutureContext)
    ) {
      nextMood = observation.mood;
      nextIntensity = observation.intensity;
    }

    if (nextMood === currentMood) {
      nextIntensity = resolveSoundtrackIntensity(
        committer,
        currentMood,
        currentIntensity,
        observation,
        futureRecent
      );
    }

    const moodChanged = nextMood !== currentMood;
    const intensityChanged = nextIntensity !== currentIntensity;

    committer.committedMood = nextMood;
    committer.committedIntensity = nextIntensity;
    committer.dwellCount = moodChanged ? 1 : committer.dwellCount + 1;

    return {
      committedMood: nextMood,
      committedIntensity: nextIntensity,
      moodChanged: moodChanged,
      intensityChanged: intensityChanged,
      previousMood: previousMood || null,
      previousIntensity: previousIntensity || null,
      dwellCount: committer.dwellCount,
    };
  }

  function syncSoundtrackChapter(committer, chapterKey) {
    if (!chapterKey || committer.currentChapter === chapterKey) {
      return;
    }
    committer.currentChapter = chapterKey;
    committer.committedMood = "";
    committer.committedIntensity = 0;
    committer.dwellCount = 0;
    committer.recent = [];
    committer.lastPage = null;
  }

  function soundtrackRecentObservation(observation) {
    return {
      page: observation.page,
      mood: observation.mood,
      intensity: observation.intensity,
      scores: observation.scores,
      dominantFamily: dominantSoundtrackFamily(observation.scores, observation.mood),
    };
  }

  function clearSoundtrackRecentIfDiscontinuous(committer, currentPage) {
    if (committer.lastPage == null || currentPage == null) {
      return;
    }
    if (
      currentPage <= committer.lastPage ||
      currentPage > committer.lastPage + SOUNDTRACK_MAX_CONTIGUOUS_PAGE_GAP
    ) {
      committer.recent = [];
    }
  }

  function pushSoundtrackRecent(committer, observation) {
    committer.recent.push(observation);
    while (committer.recent.length > SOUNDTRACK_MAX_RECENT_OBSERVATIONS) {
      committer.recent.shift();
    }
  }

  function collectSoundtrackFutureContext(currentPage, futureContext) {
    if (currentPage == null) {
      return [];
    }

    const collected = [];
    let expectedPage = currentPage + 1;
    for (const observation of futureContext) {
      if (observation.page == null || observation.page !== expectedPage) {
        break;
      }
      if (Math.abs(observation.page - currentPage) > SOUNDTRACK_CONTEXT_RADIUS) {
        break;
      }
      collected.push(soundtrackRecentObservation(observation));
      expectedPage = observation.page + 1;
    }
    return collected;
  }

  function computeSoundtrackContextScores(recent, currentPage, futureRecent) {
    const scores = createOneHotScores("epic");
    for (const mood of ALL_MOODS) {
      scores[mood] = 0;
    }

    if (currentPage == null) {
      const fallback = recent.length ? recent[recent.length - 1].scores : scores;
      return cloneScores(fallback);
    }

    let totalWeight = 0;
    for (const observation of recent.concat(futureRecent)) {
      if (observation.page == null) {
        continue;
      }
      const distance = Math.abs(observation.page - currentPage);
      if (distance > SOUNDTRACK_CONTEXT_RADIUS) {
        continue;
      }
      const weight = soundtrackContextWeight(distance);
      totalWeight += weight;
      for (const mood of ALL_MOODS) {
        scores[mood] += (observation.scores[mood] || 0) * weight;
      }
    }

    if (totalWeight <= Number.EPSILON) {
      const fallback = recent.length ? recent[recent.length - 1].scores : scores;
      return cloneScores(fallback);
    }

    for (const mood of ALL_MOODS) {
      scores[mood] = scores[mood] / totalWeight;
    }
    return scores;
  }

  function soundtrackContextWeight(distance) {
    if (distance === 0) {
      return 1.0;
    }
    if (distance === 1) {
      return 0.8;
    }
    return 0.45;
  }

  function resolveSoundtrackFamilySwitchTarget(
    committer,
    committedFamily,
    candidateFamily,
    observation,
    contextScores,
    futureRecent,
    hasFutureContext
  ) {
    if (
      shouldSwitchSoundtrackFamily(
        committer,
        committedFamily,
        candidateFamily,
        observation,
        contextScores,
        hasFutureContext
      )
    ) {
      return candidateFamily;
    }

    return forcedSoundtrackFamilyExitTarget(
      committedFamily,
      candidateFamily,
      observation,
      contextScores,
      futureRecent
    );
  }

  function shouldSwitchSoundtrackFamily(committer, committedFamily, candidateFamily, observation, contextScores, hasFutureContext) {
    if (dominantSoundtrackFamily(contextScores, observation.mood) !== candidateFamily) {
      return false;
    }

    const contextCandidateSupport = soundtrackFamilyScore(contextScores, candidateFamily);
    const contextCommittedSupport = soundtrackFamilyScore(contextScores, committedFamily);
    const contextGap = contextCandidateSupport - contextCommittedSupport;
    const currentCandidateSupport = soundtrackFamilyScore(observation.scores, candidateFamily);
    const currentCommittedSupport = soundtrackFamilyScore(observation.scores, committedFamily);
    const currentGap = currentCandidateSupport - currentCommittedSupport;

    if (
      contextCandidateSupport >= SOUNDTRACK_CONTEXT_FAMILY_SUPPORT_THRESHOLD &&
      contextGap >= SOUNDTRACK_CONTEXT_FAMILY_GAP_THRESHOLD &&
      (currentCandidateSupport >= SOUNDTRACK_FAMILY_SUPPORT_THRESHOLD || currentGap >= 0)
    ) {
      return true;
    }

    if (hasFutureContext) {
      return false;
    }

    const familyStreak = trailingSoundtrackFamilyStreak(committer, candidateFamily);
    return familyStreak >= SOUNDTRACK_MIN_FAMILY_STREAK_TO_SWITCH &&
      currentCandidateSupport >= SOUNDTRACK_FAMILY_SUPPORT_THRESHOLD;
  }

  function forcedSoundtrackFamilyExitTarget(committedFamily, candidateFamily, observation, contextScores, futureRecent) {
    if (committedFamily !== "light") {
      return "";
    }

    if (futureRecent.length < SOUNDTRACK_MIN_FUTURE_PAGES_FOR_FORCED_FAMILY_EXIT) {
      return "";
    }

    for (let index = 0; index < SOUNDTRACK_MIN_FUTURE_PAGES_FOR_FORCED_FAMILY_EXIT; index += 1) {
      if (futureRecent[index].dominantFamily === committedFamily) {
        return "";
      }
    }

    return bestAlternativeSoundtrackFamily(contextScores, committedFamily, candidateFamily, observation.mood);
  }

  function shouldSwitchSoundtrackMood(committer, currentMood, observation, contextScores, hasFutureContext) {
    const family = soundtrackFamilyOf(observation.mood);
    if (dominantSoundtrackMoodInFamily(contextScores, family, currentMood) !== observation.mood) {
      return false;
    }

    const contextCandidateScore = contextScores[observation.mood] || 0;
    const contextCurrentScore = contextScores[currentMood] || 0;
    const currentCandidateScore = observation.scores[observation.mood] || 0;
    const currentScore = observation.scores[currentMood] || 0;

    if (
      contextCandidateScore >= SOUNDTRACK_CONTEXT_MOOD_SUPPORT_THRESHOLD &&
      contextCandidateScore - contextCurrentScore >= SOUNDTRACK_CONTEXT_MOOD_GAP_THRESHOLD
    ) {
      return true;
    }

    if (hasFutureContext) {
      return false;
    }

    return (
      currentCandidateScore >= SOUNDTRACK_SAME_FAMILY_MOOD_SCORE_THRESHOLD &&
      currentCandidateScore - currentScore >= SOUNDTRACK_SAME_FAMILY_MOOD_GAP_THRESHOLD &&
      trailingSoundtrackMoodStreak(committer, observation.mood) >= SOUNDTRACK_MIN_MOOD_STREAK_TO_SWITCH
    );
  }

  function resolveSoundtrackIntensity(committer, committedMood, committedIntensity, observation, futureRecent) {
    if (observation.intensity === committedIntensity) {
      return committedIntensity;
    }

    const sameMoodStreak = trailingSoundtrackMoodStreak(committer, committedMood);
    const sameFamilyStreak = trailingSoundtrackFamilyStreak(committer, soundtrackFamilyOf(committedMood));
    const contextConfirms = futureRecent.some(function (next) {
      return next.mood === committedMood && next.intensity === observation.intensity;
    });

    if (observation.intensity > committedIntensity) {
      if (
        sameMoodStreak >= SOUNDTRACK_MIN_MOOD_STREAK_TO_SWITCH ||
        contextConfirms ||
        (
          sameFamilyStreak >= SOUNDTRACK_MIN_FAMILY_STREAK_TO_SWITCH &&
          (observation.narrativeRole === "escalation" || observation.narrativeRole === "climax") &&
          observation.intensity >= 3
        )
      ) {
        return observation.intensity;
      }
      return committedIntensity;
    }

    if (
      sameMoodStreak >= SOUNDTRACK_MIN_MOOD_STREAK_TO_SWITCH ||
      contextConfirms ||
      (
        sameFamilyStreak >= SOUNDTRACK_MIN_FAMILY_STREAK_TO_SWITCH &&
        (observation.narrativeRole === "de_escalation" || observation.narrativeRole === "transition")
      )
    ) {
      return observation.intensity;
    }

    return committedIntensity;
  }

  function selectCrossFamilySoundtrackMood(targetFamily, contextScores) {
    return bestTargetSoundtrackMoodInFamily(contextScores, targetFamily);
  }

  function selectCrossFamilySoundtrackIntensity(committer, targetMood, targetFamily, observation, futureRecent) {
    if (observation.mood === targetMood || soundtrackFamilyOf(observation.mood) === targetFamily) {
      return observation.intensity;
    }

    const recentChain = committer.recent.slice().reverse().concat(futureRecent);
    for (const candidate of recentChain) {
      if (candidate.mood === targetMood) {
        return candidate.intensity;
      }
    }

    for (const candidate of recentChain) {
      if (soundtrackFamilyOf(candidate.mood) === targetFamily) {
        return candidate.intensity;
      }
    }

    return observation.intensity;
  }

  function trailingSoundtrackFamilyStreak(committer, family) {
    let streak = 0;
    for (let index = committer.recent.length - 1; index >= 0; index -= 1) {
      if (committer.recent[index].dominantFamily !== family) {
        break;
      }
      streak += 1;
    }
    return streak;
  }

  function trailingSoundtrackMoodStreak(committer, mood) {
    let streak = 0;
    for (let index = committer.recent.length - 1; index >= 0; index -= 1) {
      if (committer.recent[index].mood !== mood) {
        break;
      }
      streak += 1;
    }
    return streak;
  }

  function dominantSoundtrackMoodInFamily(scores, family, fallbackMood) {
    const moods = SOUNDTRACK_FAMILY_MOODS[family] || [fallbackMood, fallbackMood];
    const firstScore = scores[moods[0]] || 0;
    const secondScore = scores[moods[1]] || 0;
    if (firstScore > secondScore + Number.EPSILON) {
      return moods[0];
    }
    if (secondScore > firstScore + Number.EPSILON) {
      return moods[1];
    }
    return soundtrackFamilyOf(fallbackMood) === family ? fallbackMood : moods[0];
  }

  function bestAlternativeSoundtrackFamily(scores, committedFamily, fallbackFamily, fallbackMood) {
    let bestFamily = fallbackFamily;
    let bestScore = soundtrackFamilyScore(scores, fallbackFamily);
    for (const family of Object.keys(SOUNDTRACK_FAMILY_MOODS)) {
      if (family === committedFamily) {
        continue;
      }
      const score = soundtrackFamilyScore(scores, family);
      if (
        score > bestScore + Number.EPSILON ||
        (Math.abs(score - bestScore) <= Number.EPSILON &&
          betterSoundtrackFamilyTiebreak(family, bestFamily, fallbackFamily, fallbackMood))
      ) {
        bestScore = score;
        bestFamily = family;
      }
    }
    return bestFamily;
  }

  function betterSoundtrackFamilyTiebreak(candidate, currentBest, fallbackFamily, fallbackMood) {
    if (candidate === currentBest) {
      return false;
    }
    if (candidate === fallbackFamily && currentBest !== fallbackFamily) {
      return true;
    }
    if (currentBest === fallbackFamily && candidate !== fallbackFamily) {
      return false;
    }

    const fallbackMoodFamily = soundtrackFamilyOf(fallbackMood);
    if (fallbackMoodFamily === candidate && fallbackMoodFamily !== currentBest) {
      return true;
    }
    if (fallbackMoodFamily === currentBest && fallbackMoodFamily !== candidate) {
      return false;
    }

    return soundtrackFamilyPriority(candidate) < soundtrackFamilyPriority(currentBest);
  }

  function soundtrackFamilyPriority(family) {
    switch (family) {
      case "action":
        return 0;
      case "suspense":
        return 1;
      case "emotion":
        return 2;
      case "light":
      default:
        return 3;
    }
  }

  function bestTargetSoundtrackMoodInFamily(scores, family) {
    const moods = SOUNDTRACK_TARGET_MOODS[family] || [ALL_MOODS[0], ALL_MOODS[0]];
    const firstScore = scores[moods[0]] || 0;
    const secondScore = scores[moods[1]] || 0;
    if (firstScore > secondScore + Number.EPSILON) {
      return moods[0];
    }
    if (secondScore > firstScore + Number.EPSILON) {
      return moods[1];
    }
    return moods[0];
  }

  function dominantSoundtrackFamily(scores, fallbackMood) {
    const fallbackFamily = soundtrackFamilyOf(fallbackMood);
    let bestFamily = fallbackFamily;
    let bestScore = soundtrackFamilyScore(scores, fallbackFamily);
    for (const family of Object.keys(SOUNDTRACK_FAMILY_MOODS)) {
      const score = soundtrackFamilyScore(scores, family);
      if (score > bestScore + Number.EPSILON) {
        bestScore = score;
        bestFamily = family;
      }
    }
    return bestFamily;
  }

  function soundtrackFamilyScore(scores, family) {
    return (SOUNDTRACK_FAMILY_MOODS[family] || []).reduce(function (sum, mood) {
      return sum + (scores[mood] || 0);
    }, 0);
  }

  function soundtrackFamilyOf(mood) {
    return SOUNDTRACK_FAMILY_BY_MOOD[mood] || "action";
  }

  function cloneScores(scores) {
    const copy = {};
    for (const mood of ALL_MOODS) {
      copy[mood] = scores[mood] || 0;
    }
    return copy;
  }

  function inferNarrativeRole(mood, intensity) {
    if (mood === "epic" && intensity >= 2) {
      return "climax";
    }
    if ((mood === "tension" || mood === "mystery" || mood === "horror") && intensity >= 2) {
      return "escalation";
    }
    if ((mood === "peaceful" && intensity <= 1) || (mood === "sadness" && intensity <= 2)) {
      return "de_escalation";
    }
    return "continuation";
  }

  function exportCsv() {
    const methodA = getMethodById(state.activeMethodAId);
    const methodB = getMethodById(state.activeMethodBId);
    const rows = buildVisibleRows();
    if (!methodA || !rows.length) {
      setCatalogStatus("Aucune ligne a exporter.", "warn");
      return;
    }

    const header = [
      "page",
      "chapter",
      "bench_counted",
      "expected",
      "method_a",
      "prediction_a",
      "status_a",
      "method_b",
      "prediction_b",
      "status_b",
      "delta",
      "confidence",
      "reasoning",
    ];
    const lines = [header.join(",")];

    for (const row of rows) {
      lines.push([
        csvCell(row.pageName),
        csvCell(row.chapterKey),
        csvCell(row.counted ? "yes" : "no"),
        csvCell(row.expectedTag),
        csvCell(methodA.label),
        csvCell(row.actualTagA),
        csvCell(row.statusA),
        csvCell(methodB ? methodB.label : ""),
        csvCell(row.actualTagB),
        csvCell(row.statusB),
        csvCell(row.deltaLabel),
        csvCell(String(row.confidence)),
        csvCell(row.reasoning || ""),
      ].join(","));
    }

    const blob = new Blob([lines.join("\n")], { type: "text/csv;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = methodA.label.replace(/[^\w.-]+/g, "_") + "_comparison.csv";
    anchor.click();
    URL.revokeObjectURL(url);
    setCatalogStatus("CSV exporte.", "ok");
  }

  function csvCell(value) {
    return "\"" + String(value).replace(/"/g, "\"\"") + "\"";
  }

  function formatTag(mood, intensity) {
    return mood + ":" + intensity;
  }

  function formatFraction(value, total) {
    return value + "/" + total;
  }

  function formatConfidence(value) {
    return Number(value).toFixed(2);
  }

  function isMethodLoaded(method) {
    return Boolean(method && method.predictions && Object.keys(method.predictions).length);
  }

  function isManualMethod(method) {
    return Boolean(method && method.sourcePath && method.sourcePath.startsWith("manual/"));
  }

  function uniqueDefined(values) {
    return Array.from(new Set(values.filter(Boolean)));
  }

  function apiUrl(path, forceRefresh, params) {
    const url = new URL(API_BASE + path, window.location.origin);
    if (forceRefresh) {
      url.searchParams.set("_ts", String(Date.now()));
    }
    if (params) {
      for (const key of Object.keys(params)) {
        if (params[key] !== undefined && params[key] !== null && params[key] !== "") {
          url.searchParams.set(key, String(params[key]));
        }
      }
    }
    return url;
  }

  async function fetchJson(url) {
    const response = await fetch(url.toString(), { cache: "no-store" });
    if (!response.ok) {
      throw new Error(response.status + " " + response.statusText);
    }
    return response.json();
  }

  function toNumber(value) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : 0;
  }

  function setCatalogStatus(text, tone) {
    elements.catalogStatus.textContent = text;
    elements.catalogStatus.dataset.tone = tone || "neutral";
  }

  function escapeHtml(value) {
    return String(value)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  function escapeAttribute(value) {
    return escapeHtml(value);
  }
})();
