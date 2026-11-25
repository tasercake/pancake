import { VirtualRail } from "./rail.js";

const $ = (id) => document.getElementById(id);
const number = new Intl.NumberFormat();
const date = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" });
const parameters = new URLSearchParams(location.hash.slice(1));
const drafts = new Map();
const saving = new Set();
const tools = new Map();
const runStates = {
  running: "Running", completed: "Completed", completed_with_errors: "Completed with failures",
  cancelled: "Stopped", interrupted: "Interrupted", error: "Run failed",
};
const unavailable = {
  outside_scope: ["Scene outside this evaluation", "Choose another evaluation or run one for this scene."],
  pending: ["Output pending", "It will appear here when the stacker finishes."],
  failed: ["Stacker failed", ""],
  unfinished: ["Output unfinished", "This run ended before this output was available."],
  different_source: ["Dataset snapshot changed", "Run this scene again with the current source images."],
  missing_file: ["Output file missing", "Restore the saved TIFF or run this scene again."],
};

let scenes = [], visible = [], runs = [], scene = null, details = null;
let frameIndex = 0, referenceIndex = 0, runId = "", stackerId = "", formKey = "";
let sceneRequest, pollTimer, stopping = "";
let warmupTimer, warming = [];
let catalogRevision = -1, downloadTimer, startingDownload = false;
let outputLoading = false;
let runOptions = "", outputState = "", candidateState = "";
let activeImage = null, imageVersion = "";
const readyThumbs = new Map();
const panelImages = new WeakMap();

function url(path, params = {}) {
  return path + "?" + new URLSearchParams(params);
}

async function api(path, body, { signal } = {}) {
  const response = await fetch(path, body === undefined ? { signal } : {
    method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body),
  });
  const result = await response.json();
  if (!response.ok) throw new Error(result.error);
  return result;
}

function part(tag, text, className) {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  if (className) node.className = className;
  return node;
}

function showMessage(node, message) {
  node.textContent = message;
  node.hidden = !message;
}

function remember() {
  const params = new URLSearchParams();
  if ($("dataset").value) params.set("dataset", $("dataset").value);
  if (scene) params.set("scene", scene.id);
  if (runId) params.set("run", runId);
  history.replaceState(null, "", "#" + params);
}

function placeholder(well, title, description, log) {
  clearImageRequest(well);
  const content = part("div", undefined, "empty-image");
  content.append(part("strong", title));
  if (description) content.append(part("div", description));
  if (log) {
    const link = part("a", "View stacker log");
    link.href = log;
    link.target = "_blank";
    link.rel = "noopener";
    content.append(link);
  }
  well.replaceChildren(content);
}

function imageURL(params, size) {
  return url("/api/image", { ...params, size, v: imageVersion });
}

function previewElement(params, size, { title = "", priority = "auto", lazy = false } = {}) {
  const image = part("img");
  image.alt = title;
  image.decoding = "async";
  image.fetchPriority = priority;
  if (lazy) image.loading = "lazy";
  image.src = imageURL(params, size);
  return image;
}

function createThumbnail(params, alt = "") {
  const image = previewElement(params, "thumb", { title: alt, priority: "low", lazy: true });
  image.classList.add("preview-thumb");
  image.addEventListener("load", () => {
    image.classList.add("preview-loaded");
    const key = imageURL(params, "thumb");
    readyThumbs.delete(key);
    readyThumbs.set(key, image.currentSrc || image.src);
    if (readyThumbs.size > 120) readyThumbs.delete(readyThumbs.keys().next().value);
  }, { once: true });
  image.addEventListener("error", () => image.classList.add("preview-failed"), { once: true });
  return image;
}

function prefetchImage(params) {
  const pending = new AbortController();
  fetch(imageURL(params, "panel"), {
    signal: pending.signal, priority: "low", headers: { "X-BokehForge-Prefetch": "1" },
  }).then((response) => response.ok && response.arrayBuffer()).catch(() => {});
  return () => pending.abort();
}

function clearImageRequest(well) {
  const request = panelImages.get(well);
  if (request && !request.image.complete) request.image.removeAttribute("src");
  panelImages.delete(well);
}

function showImage(well, title, params) {
  const src = imageURL(params, "panel");
  const current = panelImages.get(well);
  if (current?.src === src && current.button.isConnected) {
    current.title = title;
    current.button.setAttribute("aria-label", "Inspect " + title);
    current.image.alt = title;
    return;
  }
  clearImageRequest(well);
  const button = part("button", undefined, "pane-image-button");
  button.type = "button";
  button.setAttribute("aria-label", "Inspect " + title);
  const image = previewElement(params, "panel", { title, priority: "high" });
  image.classList.add("pane-preview");
  const request = { src, button, image, title };
  panelImages.set(well, request);
  const loading = part("span", "Loading image…", "image-loading");
  loading.setAttribute("role", "status");
  const thumbnail = readyThumbs.get(imageURL(params, "thumb"));
  let fallback;
  if (thumbnail) {
    fallback = part("img", undefined, "pane-fallback");
    fallback.alt = "";
    fallback.src = thumbnail;
    button.append(fallback);
    button.classList.add("has-preview");
  }
  image.addEventListener("load", () => {
    button.classList.add("image-ready");
    fallback?.remove();
    loading.remove();
  }, { once: true });
  image.addEventListener("error", () => {
    if (button.isConnected) placeholder(well, "Image unavailable", "BokehForge could not render this image.");
  }, { once: true });
  button.append(image, loading, part("span", "Inspect", "expand"));
  button.onclick = () => openLightbox(request.title, params, image.naturalWidth ? image : fallback);
  well.replaceChildren(button);
}

function cancelLightboxRequests(state) {
  for (const image of state.requests) image.removeAttribute("src");
  state.requests.clear();
}

function openLightbox(title, params, previewImage) {
  if (activeImage) cancelLightboxRequests(activeImage);
  const dialog = $("image-dialog");
  const view = $("image-view");
  const stage = part("div", undefined, "lightbox-stage");
  const state = {
    title, params, stage, image: null, requests: new Set(), scale: 1,
    actual: false, nativeRequested: false, nativeLoaded: false, pending: "",
  };
  activeImage = state;
  $("image-title").textContent = title;
  $("image-title").title = title;
  view.replaceChildren(stage);
  dialog.showModal();
  const fallback = (previewImage?.complete && previewImage.naturalWidth &&
                    (previewImage.currentSrc || previewImage.src)) || readyThumbs.get(imageURL(params, "thumb"));
  if (fallback) {
    const image = part("img");
    image.alt = title;
    image.src = fallback;
    if (image.complete && image.naturalWidth) {
      state.image = image;
      stage.replaceChildren(image);
    } else {
      image.addEventListener("load", () => {
        if (activeImage !== state || state.image) return;
        state.image = image;
        stage.replaceChildren(image);
        paintLightbox();
      }, { once: true });
    }
  }
  requestLightbox("fit");
}

function lightboxAnchor(x, y) {
  const state = activeImage;
  const view = $("image-view");
  const rect = view.getBoundingClientRect();
  x ??= rect.left + view.clientWidth / 2;
  y ??= rect.top + view.clientHeight / 2;
  const image = state.image?.getBoundingClientRect();
  return {
    x: x - rect.left, y: y - rect.top,
    u: image?.width ? (x - image.left) / image.width : 0.5,
    v: image?.height ? (y - image.top) / image.height : 0.5,
  };
}

function paintLightbox(anchor = lightboxAnchor()) {
  const state = activeImage;
  const view = $("image-view");
  const image = state.image;
  $("lightbox-status").textContent = state.pending;
  $("image-fit").disabled = state.scale === 1 && !state.actual;
  $("zoom-out").disabled = $("image-fit").disabled;
  $("zoom-in").disabled = state.scale >= 12;
  $("pixel-mode").setAttribute("aria-pressed", state.actual);
  if (!image?.naturalWidth) return;
  const wide = view.clientWidth, tall = view.clientHeight;
  const fit = Math.min(1, wide / image.naturalWidth, tall / image.naturalHeight);
  const scale = state.actual ? 1 : fit * state.scale;
  const width = Math.round(image.naturalWidth * scale);
  const height = Math.round(image.naturalHeight * scale);
  image.style.width = width + "px";
  image.style.height = height + "px";
  state.stage.style.width = Math.max(wide, width) + "px";
  state.stage.style.height = Math.max(tall, height) + "px";
  view.classList.toggle("pannable", width > wide || height > tall);
  view.scrollLeft = Math.max(0, wide - width) / 2 + width * anchor.u - anchor.x;
  view.scrollTop = Math.max(0, tall - height) / 2 + height * anchor.v - anchor.y;
}

function requestLightbox(variant) {
  const state = activeImage;
  const native = variant === "native";
  if (native && (state.nativeRequested || state.nativeLoaded)) return;
  if (native) state.nativeRequested = true;
  state.pending = native ? "Loading full resolution…" : state.image ? "Loading detail…" : "Loading image…";
  paintLightbox();
  const image = previewElement(state.params, variant, { title: state.title, priority: "high" });
  state.requests.add(image);
  image.addEventListener("load", () => {
    state.requests.delete(image);
    if (activeImage !== state || (!native && state.nativeLoaded)) return;
    const anchor = lightboxAnchor();
    if (native) state.nativeLoaded = true;
    state.pending = !native && state.nativeRequested ? "Loading full resolution…" : "";
    state.image = image;
    state.stage.replaceChildren(image);
    paintLightbox(anchor);
  }, { once: true });
  image.addEventListener("error", () => {
    state.requests.delete(image);
    if (activeImage !== state || (!native && state.nativeLoaded)) return;
    if (native) state.nativeRequested = false;
    state.pending = native ? "Full resolution unavailable" : "Image unavailable";
    paintLightbox();
  }, { once: true });
}

function zoomLightbox(scale, x, y) {
  const state = activeImage;
  const anchor = lightboxAnchor(x, y);
  state.actual = false;
  state.scale = Math.max(1, Math.min(12, scale));
  if (state.scale > 1) requestLightbox("native");
  paintLightbox(anchor);
}

function lightboxScale() {
  const state = activeImage;
  if (!state.actual || !state.image) return state.scale;
  const view = $("image-view");
  return Math.max(1, state.image.naturalWidth / view.clientWidth, state.image.naturalHeight / view.clientHeight);
}

function setPixelMode(actual) {
  if (!actual) {
    zoomLightbox(1);
    return;
  }
  activeImage.actual = true;
  requestLightbox("native");
  paintLightbox();
}

function railItem(className) {
  const item = part("div", undefined, "rail-item " + className);
  item.setAttribute("role", "listitem");
  return item;
}

function inspectButton(title, params, image) {
  const button = part("button", undefined, "inspect-thumb");
  button.type = "button";
  button.title = "Inspect " + title;
  button.setAttribute("aria-label", button.title);
  button.onclick = () => openLightbox(title, params, image);
  return button;
}

function sceneEntry(item, index) {
  const row = railItem("scene-row");
  const button = part("button", undefined, "scene-entry");
  button.type = "button";
  button.title = item.name;
  button.onclick = () => goTo(index);
  const params = { frame: item.preview_frame };
  const image = createThumbnail(params);
  const preview = part("span", undefined, "scene-preview preview-thumbnail");
  preview.append(image);
  const text = part("span", undefined, "scene-entry-text");
  text.append(part("strong", item.name), part("small", `${item.dataset} · ${number.format(item.frames)} frames`));
  button.append(preview, text);
  const inspect = inspectButton(`scene preview · ${item.name}`, params, image);
  preview.ondblclick = inspect.onclick;
  row.append(button, inspect);
  return row;
}

function frameEntry(frame, index) {
  const tile = railItem("frame-tile");
  const title = `Frame ${index + 1} · ${frame.name}`;
  const button = part("button", undefined, "thumb preview-thumbnail");
  button.type = "button";
  button.title = title;
  button.setAttribute("aria-label", title);
  button.onclick = () => selectFrame(index);
  const params = { frame: frame.id };
  const image = createThumbnail(params);
  button.append(image, part("span", String(index + 1).padStart(2, "0")));
  const inspect = inspectButton(title, params, image);
  button.ondblclick = inspect.onclick;
  tile.append(button, inspect);
  return tile;
}

function filterScenes(requested) {
  const dataset = $("dataset").value;
  visible = dataset ? scenes.filter((item) => item.dataset === dataset) : scenes;
  sceneRail.setItems(visible);
  $("sidebar-count").textContent = number.format(visible.length);
  const current = visible.findIndex((item) => item.id === requested);
  goTo(Math.max(current, 0));
}

function goTo(position) {
  if (!visible.length || !Number.isFinite(position)) return;
  const index = Math.min(Math.max(Math.trunc(position), 0), visible.length - 1);
  const next = visible[index];
  sceneRail.select(index);
  $("scene-position").textContent = `Scene ${number.format(index + 1)} of ${number.format(visible.length)}`;
  $("scene-name").textContent = next.name;
  $("scene-meta").textContent = `${next.dataset} · ${number.format(next.frames)} source frames`;
  $("jump").value = index + 1;
  $("jump").max = visible.length;
  $("previous").disabled = visible.length <= 1;
  $("next").disabled = visible.length <= 1;
  if (scene?.id === next.id) {
    scene = next;
    remember();
    return;
  }
  scene = next;
  cancelWarmup();
  details = null;
  frameIndex = 0;
  referenceIndex = 0;
  filmRail.setItems([]);
  $("film-range").textContent = `${number.format(next.frames)} frames`;
  $("reference-control").hidden = true;
  $("reference-title").textContent = "Ground truth";
  $("source-meta").textContent = "";
  $("reference-meta").textContent = "";
  showImage($("source-image"), "source frame 1", { frame: next.first_frame });
  if (next.first_reference) showImage($("reference-image"), "reference", { frame: next.first_reference });
  else placeholder($("reference-image"), "Loading scene");
  renderOutput();
  remember();
  loadScene();
}

function moveScene(step) {
  const current = visible.indexOf(scene);
  goTo((current + step + visible.length) % visible.length);
}

async function loadScene() {
  if (!scene) return;
  sceneRequest?.abort();
  sceneRequest = new AbortController();
  const { signal } = sceneRequest;
  try {
    const data = await api(url("/api/scene", { scene: scene.id, run: runId }), undefined, { signal });
    if (signal.aborted) return;
    const firstLoad = !details;
    details = data;
    outputLoading = false;
    showMessage($("scene-error"), "");
    if (firstLoad) {
      renderReference();
      filmRail.setItems(details.frames);
      selectFrame(frameIndex);
    }
    renderOutput();
  } catch (error) {
    if (signal.aborted) return;
    outputLoading = false;
    renderOutput();
    showMessage($("scene-error"), error.message);
    if (!details) {
      for (const role of ["source", "reference"]) placeholder($(`${role}-image`), "Scene unavailable");
    }
  }
}

function selectFrame(index) {
  const frame = details?.frames[index];
  if (!frame) return;
  const direction = index < frameIndex ? -1 : 1;
  frameIndex = index;
  filmRail.select(index);
  $("film-range").textContent = `${number.format(index + 1)} of ${number.format(details.frames.length)}`;
  $("source-meta").textContent = frame.focus_distance_m === undefined
    ? `Frame ${index + 1} / ${details.frames.length}`
    : `${number.format(frame.focus_distance_m)} m · ƒ/${number.format(frame.aperture_f_number)}`;
  showImage($("source-image"), `source frame ${index + 1} · ${frame.name}`, { frame: frame.id });
  cancelWarmup();
  warmupTimer = setTimeout(() => {
    if (document.hidden) return;
    const adjacentFrame = details.frames[frameIndex + direction] ?? details.frames[frameIndex - direction];
    const adjacentScene = visible[(visible.indexOf(scene) + 1) % visible.length];
    const frameIds = new Set([adjacentFrame?.id, adjacentScene !== scene && adjacentScene?.first_frame]);
    warming = [...frameIds].filter(Boolean).map((frame) => prefetchImage({ frame }));
  }, 240);
}

function cancelWarmup() {
  clearTimeout(warmupTimer);
  for (const cancel of warming) cancel();
  warming = [];
}

function showReference() {
  const frame = details.references[referenceIndex];
  $("reference-meta").textContent = frame ? frame.name : "";
  if (frame) showImage($("reference-image"), `reference · ${frame.name}`, { frame: frame.id });
  else placeholder($("reference-image"), "No ground truth available", details.reference_semantics);
}

function renderReference() {
  const refs = details.references;
  const isTruth = ["rendered_all_in_focus", "synthetic_source_image_reference"].includes(details.reference_type);
  $("reference-title").textContent = refs.length && !isTruth ? "Source reference" : "Ground truth";
  $("reference-title").title = details.reference_semantics;
  $("reference-control").hidden = refs.length < 2;
  $("reference-select").replaceChildren(...refs.map((ref, index) =>
    new Option(`${index + 1} · ${ref.name}`, index)));
  showReference();
}

function renderOutput() {
  const outputs = details?.outputs ?? [];
  if (outputs.length && !outputs.some((output) => output.stacker === stackerId)) {
    stackerId = (outputs.find((output) => output.status === "ready") ?? outputs[0]).stacker;
  }
  const row = outputs.find((output) => output.stacker === stackerId);
  const tabsKey = JSON.stringify([stackerId, outputs.map((output) => [output.stacker, output.status, output.rating?.grade])]);
  if (tabsKey !== outputState) {
    const tabStrip = $("output-tabs");
    const focused = tabStrip.contains(document.activeElement) ? document.activeElement.dataset.stacker : null;
    const tabs = outputs.map((output) => {
      const button = part("button", tools.get(output.stacker)?.label ?? output.stacker);
      button.type = "button";
      button.dataset.stacker = output.stacker;
      button.title = unavailable[output.status]?.[0] ?? "View stacker output";
      button.setAttribute("aria-pressed", output === row);
      if (output.rating) button.append(part("small", output.rating.grade + "/5"));
      button.onclick = () => { stackerId = output.stacker; renderOutput(); };
      return button;
    });
    tabStrip.replaceChildren(...tabs);
    tabs.find((tab) => tab.dataset.stacker === focused)?.focus();
    outputState = tabsKey;
  }

  const readyCount = outputs.filter((output) => output.status === "ready").length;
  const ratedCount = outputs.filter((output) => output.rating).length;
  $("rating-count").textContent = readyCount ? `${ratedCount} of ${readyCount} available outputs rated` : "";
  const params = { scene: scene.id, run: runId, stacker: stackerId };
  const currentImage = JSON.stringify([params, row?.status, row?.image, row?.error, !details || outputLoading]);
  if (candidateState !== currentImage) {
    const tool = row && tools.get(stackerId);
    $("candidate-meta").textContent = tool?.label ?? "";
    $("candidate-meta").title = tool?.method ?? "";
    const well = $("candidate-image");
    if (row?.status === "ready") {
      showImage(well, tool?.label ?? "stacker output", params);
    } else if (row) {
      const [title, description] = unavailable[row.status];
      placeholder(well, title, row.error || description, row.log ? url("/api/log", params) : "");
    } else if (!details || outputLoading) {
      placeholder(well, "Loading output");
    } else {
      placeholder(well, "No evaluation output", "Start an evaluation to compare a stacked image.");
    }
    candidateState = currentImage;
  }
  renderRating(row);
}

function ratingKey() {
  return JSON.stringify([runId, scene.id, stackerId]);
}

function renderRating(row) {
  const ready = row?.status === "ready";
  $("rating-title").textContent = ready ? "Rate " + (tools.get(row.stacker)?.label ?? row.stacker) : "Rate outputs";
  $("rating-form").hidden = !ready;
  $("rating-empty").hidden = ready;
  $("rating-empty").textContent = row || outputLoading || !details
    ? "Ratings open when this output is available."
    : "Run an evaluation to start rating outputs.";
  if (!ready) {
    formKey = "";
    return;
  }
  const key = ratingKey();
  if (key !== formKey) {
    const data = drafts.get(key) ?? row.rating;
    for (const input of $("rating-form").elements.grade) input.checked = Number(input.value) === data?.grade;
    $("comment").value = data?.comment ?? "";
    $("rating-status").textContent = drafts.has(key) ? "Unsaved changes" : row.rating ? "Saved locally" : "";
    formKey = key;
  }
  $("save-rating").disabled = saving.has(key);
}

function captureDraft() {
  const grade = Number($("rating-form").elements.grade.value);
  const comment = $("comment").value;
  const old = details.outputs.find((output) => output.stacker === stackerId).rating;
  if ((old?.grade ?? 0) === grade && (old?.comment ?? "") === comment) {
    drafts.delete(formKey);
    $("rating-status").textContent = old ? "Saved locally" : "";
  } else {
    drafts.set(formKey, { grade, comment });
    $("rating-status").textContent = "Unsaved changes";
  }
}

async function saveRating(event) {
  event.preventDefault();
  const key = formKey;
  const form = { run: runId, scene: scene.id, stacker: stackerId,
    grade: Number(event.target.elements.grade.value), comment: $("comment").value };
  saving.add(key);
  $("save-rating").disabled = true;
  $("rating-status").textContent = "Saving…";
  try {
    await api("/api/ratings", form);
    const draft = drafts.get(key);
    if (draft?.grade === form.grade && draft?.comment === form.comment) drafts.delete(key);
    if (key === formKey) $("rating-status").textContent = drafts.has(key) ? "Unsaved changes" : "Saved locally";
    await loadScene();
  } catch (error) {
    if (key === formKey) $("rating-status").textContent = error.message;
  } finally {
    saving.delete(key);
    if (key === formKey) $("save-rating").disabled = false;
  }
}

function changeRun(id) {
  if (id === runId) return;
  runId = id;
  outputLoading = true;
  if (details) details.outputs = [];
  if (scene) renderOutput();
  remember();
  loadScene();
}

function renderRuns() {
  const active = runs.find((run) => run.status === "running");
  const banner = active ?? (runs[0]?.status === "error" ? runs[0] : null);
  $("run-banner").hidden = !banner;
  $("open-run").disabled = !scenes.length || Boolean(active);
  if (banner) {
    const stopped = stopping === banner.id;
    const setup = banner.setup;
    const settingUp = setup && banner.status === "running";
    const setupLabel = tools.get(setup?.stacker)?.label ?? setup?.stacker;
    $("run-status").textContent = stopped ? "Stopping evaluation…" :
      settingUp ? `Setting up ${setupLabel}` : runStates[banner.status];
    $("run-detail").textContent = (settingUp ? setup.message : banner.error) ??
      `${banner.scope.label} · ${banner.ready + banner.failed} of ${banner.total} outputs processed${banner.failed ? ` · ${banner.failed} failed` : ""}`;
    $("run-progress").max = banner.total;
    if (settingUp) $("run-progress").removeAttribute("value");
    else $("run-progress").value = banner.ready + banner.failed;
    $("stop-run").disabled = stopped;
    $("stop-run").hidden = banner !== active;
  } else {
    stopping = "";
  }
  const labels = runs.map((run) => [run.id, `${run.scope.label} · ${date.format(new Date(run.created_at))} · ${runStates[run.status]}`]);
  const optionsKey = JSON.stringify(labels);
  if (optionsKey !== runOptions) {
    $("run-select").replaceChildren(...(labels.length ? labels.map(([id, label]) => new Option(label, id))
      : [new Option("No evaluations yet", "")]));
    runOptions = optionsKey;
  }
  $("run-select").disabled = !runs.length;
  $("run-select").value = runId;
}

async function refreshRuns() {
  const old = runs.find((run) => run.id === runId);
  runs = await api("/api/runs");
  if (!runs.some((run) => run.id === runId)) changeRun(runs[0]?.id ?? "");
  renderRuns();
  const current = runs.find((run) => run.id === runId);
  if (current?.status !== old?.status || current?.ready !== old?.ready || current?.failed !== old?.failed) loadScene();
}

function scheduleRuns() {
  clearTimeout(pollTimer);
  pollTimer = setTimeout(async () => {
    try { await refreshRuns(); }
    catch (error) { showMessage($("scene-error"), error.message); }
    scheduleRuns();
  }, runs.some((run) => run.status === "running") ? 1500 : 4500);
}

function openRunDialog() {
  const scope = $("scope");
  scope.replaceChildren(new Option("Current scene", "scene"), new Option("Current dataset", "dataset"),
    new Option("All indexed datasets", "all"));
  const summary = $("scope-summary");
  const dataset = [...$("dataset").options].find((option) => option.value === scene.dataset)?.text ?? scene.dataset;
  scope.onchange = () => {
    const description = {
      scene: scene.name, dataset, all: `${number.format(scenes.length)} scenes`,
    }[scope.value];
    summary.textContent = description;
    summary.title = description;
  };
  scope.onchange();
  const prepared = new Set(runs.flatMap((run) => run.prepared ?? []));
  const choices = [...tools.values()].map((tool) => {
    const label = part("label", undefined, "tool-option");
    const checkbox = part("input");
    checkbox.type = "checkbox";
    checkbox.name = "stacker";
    checkbox.value = tool.id;
    checkbox.checked = tool.available || prepared.has(tool.id);
    const subtitle = part("span", checkbox.checked ? tool.method : `${tool.method} · Sets up on first run`);
    subtitle.title = subtitle.textContent;
    label.append(checkbox, part("strong", tool.label), subtitle);
    return label;
  });
  $("available-tools").replaceChildren(...choices);
  $("start-run").disabled = false;
  showMessage($("run-error"), "");
  $("new-run").showModal();
}

async function startRun(event) {
  event.preventDefault();
  const stackers = new FormData(event.target).getAll("stacker");
  if (!stackers.length) {
    showMessage($("run-error"), "Choose at least one stacking program.");
    return;
  }
  $("start-run").disabled = true;
  try {
    const result = await api("/api/runs", { scope: $("scope").value, scene: scene.id, dataset: scene.dataset, stackers });
    $("new-run").close();
    changeRun(result.id);
    await refreshRuns();
    scheduleRuns();
  } catch (error) {
    showMessage($("run-error"), error.message);
  } finally {
    $("start-run").disabled = false;
  }
}

async function stopRun() {
  const run = runs.find((item) => item.status === "running");
  if (!run) return;
  stopping = run.id;
  renderRuns();
  try { await api("/api/cancel", { run: run.id }); }
  catch (error) {
    stopping = "";
    showMessage($("scene-error"), error.message);
    renderRuns();
  }
}

function applyCatalog(catalog) {
  catalogRevision = catalog.revision;
  imageVersion = catalog.image_version;
  for (const tool of catalog.stackers) tools.set(tool.id, tool);
  scenes = catalog.scenes ?? [];
  const workspace = $("workspace");
  const notice = $("catalog-state");
  if (scenes.length) {
    const dataset = $("dataset");
    const selected = workspace.hidden ? parameters.get("dataset") : dataset.value;
    dataset.replaceChildren(new Option("All datasets", ""), ...Object.entries(catalog.datasets)
      .map(([id, name]) => new Option(name === id ? id : `${name} (${id})`, id)));
    dataset.value = selected ?? "";
    if (dataset.selectedIndex < 0) dataset.value = "";
    notice.hidden = true;
    workspace.hidden = false;
    filterScenes(scene?.id ?? parameters.get("scene"));
  } else {
    workspace.hidden = true;
    notice.hidden = false;
    const heading = notice.querySelector("h1");
    if (catalog.status === "indexing") {
      heading.textContent = "Indexing local datasets";
      $("catalog-note").textContent = "Verifying the downloaded originals before opening the scene catalog.";
    } else {
      heading.textContent = catalog.status === "missing" ? "Download a dataset to get started" : "No scenes available";
      $("catalog-note").textContent = catalog.error ?? (catalog.status === "missing"
        ? "Scenes appear here after their dataset has been downloaded and verified."
        : "The verified catalog contains no usable scenes.");
    }
  }
  renderRuns();
}

function byteSize(bytes) {
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  let unit = 0;
  while (bytes >= 1024 && unit < units.length - 1) {
    bytes /= 1024;
    unit++;
  }
  return `${bytes.toLocaleString(undefined, { maximumFractionDigits: unit ? 1 : 0 })} ${units[unit]}`;
}

function remainingTime(seconds) {
  if (seconds < 60) return `~${Math.max(1, Math.ceil(seconds))} sec left`;
  const minutes = Math.ceil(seconds / 60);
  if (minutes < 60) return `~${minutes} min left`;
  const hours = Math.floor(minutes / 60);
  return `~${hours} hr ${minutes % 60} min left`;
}

function renderDownload(status) {
  const names = Object.entries(status.missing).map(([id, name]) => name === id ? id : `${name} (${id})`);
  const active = status.status === "running";
  const indexing = status.status === "complete" && status.indexing;
  const banner = $("download-banner");
  banner.hidden = !names.length && !active && !indexing && !status.error && !status.catalog_error;
  const button = $("download-start");
  button.hidden = !names.length || active || indexing;
  button.disabled = startingDownload || status.indexing;
  button.textContent = status.status === "failed" ? "Retry download" : names.length === 1 ? "Download dataset" : "Download datasets";

  const phases = {
    preparing: "Resolving dataset files…",
    checking: "Checking existing files…",
    downloading: "Downloading dataset files…",
    retrying: "Retrying the transfer…",
    verifying: "Verifying downloaded files…",
  };
  const message = active ? phases[status.phase] : indexing ? "Download complete. Indexing scenes…"
    : names.length ? `Available to download · ${names.join(", ")}` : "Dataset download";
  if ($("download-state").textContent !== message) $("download-state").textContent = message;
  showMessage($("download-error"), status.error || status.catalog_error || "");

  const meter = $("download-meter");
  meter.hidden = !active && !indexing;
  const progress = $("download-progress");
  if (status.total > 0) {
    progress.max = status.total;
    progress.value = Math.min(status.received, status.total);
  } else {
    progress.removeAttribute("value");
  }
  const details = status.total > 0
    ? [`${byteSize(status.received)} of ${byteSize(status.total)}`,
       `${Math.floor(100 * progress.value / status.total)}%`]
    : status.received > 0 ? [byteSize(status.received) + " received"] : [];
  if (status.rate) details.push(byteSize(status.rate) + "/s");
  if (status.seconds_remaining > 0) details.push(remainingTime(status.seconds_remaining));
  $("download-detail").textContent = details.join(" · ");
}

function scheduleDownload(delay) {
  clearTimeout(downloadTimer);
  downloadTimer = setTimeout(pollDownload, delay);
}

async function pollDownload() {
  let delay = 3000;
  try {
    const status = await api("/api/download");
    if (status.catalog_revision !== catalogRevision) applyCatalog(await api("/api/catalog"));
    renderDownload(status);
    if (status.status === "running" || status.indexing) delay = 750;
  } catch (error) {
    $("download-banner").hidden = false;
    showMessage($("download-error"), error.message);
  }
  scheduleDownload(delay);
}

async function startDownload() {
  startingDownload = true;
  $("download-start").disabled = true;
  showMessage($("download-error"), "");
  try {
    renderDownload(await api("/api/download", {}));
    scheduleDownload(750);
  } catch (error) {
    showMessage($("download-error"), error.message);
  } finally {
    startingDownload = false;
    $("download-start").disabled = false;
  }
}

async function boot() {
  try {
    const [catalog, history] = await Promise.all([api("/api/catalog"), api("/api/runs")]);
    runs = history;
    runId = runs.find((run) => run.id === parameters.get("run"))?.id ?? runs[0]?.id ?? "";
    applyCatalog(catalog);
    scheduleRuns();
    await pollDownload();
  } catch (error) {
    $("catalog-state").querySelector("h1").textContent = "Could not connect to BokehForge";
    $("catalog-note").textContent = error.message;
  }
}

const sceneRail = new VirtualRail($("scene-list"), "y", 74, sceneEntry);
const filmRail = new VirtualRail($("filmstrip"), "x", 112, frameEntry);

$("dataset").onchange = () => filterScenes(scene?.id);
$("previous").onclick = () => moveScene(-1);
$("next").onclick = () => moveScene(1);
$("jump").onchange = () => goTo($("jump").valueAsNumber - 1);
$("reference-select").onchange = () => { referenceIndex = Number($("reference-select").value); showReference(); };
$("run-select").onchange = () => changeRun($("run-select").value);
$("rating-form").oninput = captureDraft;
$("rating-form").onsubmit = saveRating;
$("open-run").onclick = openRunDialog;
$("download-start").onclick = startDownload;
$("stop-run").onclick = stopRun;
$("run-form").onsubmit = startRun;
$("pixel-mode").onclick = () => setPixelMode(!activeImage.actual);
$("image-fit").onclick = () => zoomLightbox(1);
$("zoom-in").onclick = () => zoomLightbox(lightboxScale() * 1.5);
$("zoom-out").onclick = () => zoomLightbox(lightboxScale() / 1.5);
const lightboxView = $("image-view");
lightboxView.addEventListener("wheel", (event) => {
  if (event.shiftKey) return;
  event.preventDefault();
  const units = event.deltaMode ? 20 : 1;
  const scale = Math.exp(Math.max(-1, Math.min(1, -event.deltaY * units / 300)));
  zoomLightbox(lightboxScale() * scale, event.clientX, event.clientY);
}, { passive: false });
lightboxView.addEventListener("dblclick", (event) => {
  zoomLightbox(lightboxScale() > 1 ? 1 : 2, event.clientX, event.clientY);
});
$("image-dialog").addEventListener("keydown", (event) => {
  const zoom = { "+": 1.5, "=": 1.5, "-": 1 / 1.5 }[event.key];
  if (event.ctrlKey || event.metaKey || event.altKey || (!zoom && event.key !== "0")) return;
  event.preventDefault();
  zoomLightbox(zoom ? lightboxScale() * zoom : 1);
});
let panStart = null;
lightboxView.addEventListener("pointerdown", (event) => {
  if (event.pointerType === "touch" || event.button !== 0 || !lightboxView.classList.contains("pannable")) return;
  lightboxView.setPointerCapture(event.pointerId);
  panStart = { x: event.clientX, y: event.clientY, left: lightboxView.scrollLeft, top: lightboxView.scrollTop };
  lightboxView.classList.add("dragging");
});
lightboxView.addEventListener("pointermove", (event) => {
  if (!panStart) return;
  lightboxView.scrollTo(panStart.left + panStart.x - event.clientX, panStart.top + panStart.y - event.clientY);
});
lightboxView.addEventListener("lostpointercapture", () => {
  panStart = null;
  lightboxView.classList.remove("dragging");
});
$("image-dialog").addEventListener("close", (event) => {
  if (event.currentTarget.open || !activeImage) return;
  cancelLightboxRequests(activeImage);
  activeImage = null;
  lightboxView.replaceChildren();
});
new ResizeObserver(() => {
  if (activeImage) paintLightbox();
}).observe(lightboxView);
for (const button of document.querySelectorAll("[data-close]")) button.onclick = () => $(button.dataset.close).close();
document.addEventListener("keydown", (event) => {
  if (!scene || document.querySelector("dialog[open]") ||
      event.target.closest?.("input,textarea,select,[contenteditable]") ||
      event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
  if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
    event.preventDefault();
    selectFrame(frameIndex + (event.key === "ArrowLeft" ? -1 : 1));
  } else if (event.key === "ArrowUp" || event.key === "ArrowDown") {
    event.preventDefault();
    moveScene(event.key === "ArrowUp" ? -1 : 1);
  }
});
window.addEventListener("beforeunload", (event) => {
  if (drafts.size) event.preventDefault();
});
boot();
