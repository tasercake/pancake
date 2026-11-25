const $ = (id) => document.getElementById(id);
const PAGE_SIZE = 10;
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
let requestId = 0, pollTimer, stopping = "";
let outputLoading = false;
let runOptions = "", outputState = "", candidateState = "";
let activeImage = null;

function url(path, params = {}) {
  return path + "?" + new URLSearchParams(params);
}

async function api(path, body) {
  const response = await fetch(path, body === undefined ? {} : {
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

function showImage(well, title, params) {
  const button = part("button");
  button.type = "button";
  button.setAttribute("aria-label", "Inspect " + title);
  const image = part("img");
  image.alt = title;
  image.decoding = "async";
  image.src = url("/api/image", params);
  image.onerror = () => {
    if (button.isConnected) placeholder(well, "Image unavailable", "BokehForge could not render this image.");
  };
  button.append(image, part("span", "Inspect", "expand"));
  button.onclick = () => {
    activeImage = { title, params };
    $("image-title").textContent = title;
    setPixelMode(false);
    $("image-dialog").showModal();
  };
  well.replaceChildren(button);
}

function setPixelMode(actual) {
  $("pixel-mode").setAttribute("aria-pressed", actual);
  const view = $("image-view");
  view.classList.toggle("actual", actual);
  const image = part("img");
  image.alt = activeImage.title;
  image.src = url("/api/image", { ...activeImage.params, size: actual ? "native" : "fit" });
  image.onerror = () => {
    if (image.isConnected) placeholder(view, "Image unavailable", "BokehForge could not render this image.");
  };
  view.replaceChildren(image);
  view.scrollTo(0, 0);
}

function filterScenes(requested) {
  const dataset = $("dataset").value;
  visible = scenes.filter((item) => !dataset || item.dataset === dataset);
  const current = visible.findIndex((item) => item.id === requested);
  goTo(Math.max(current, 0));
}

function goTo(position) {
  if (!visible.length || !Number.isFinite(position)) return;
  const index = Math.min(Math.max(Math.trunc(position), 0), visible.length - 1);
  const next = visible[index];
  $("scene-position").textContent = `Scene ${number.format(index + 1)} of ${number.format(visible.length)}`;
  $("scene-name").textContent = next.name;
  $("scene-meta").textContent = `${next.dataset} · ${number.format(next.frames)} source frames`;
  $("jump").value = index + 1;
  $("jump").max = visible.length;
  $("previous").disabled = visible.length <= 1;
  $("next").disabled = visible.length <= 1;
  if (scene?.id === next.id) {
    remember();
    return;
  }
  scene = next;
  details = null;
  frameIndex = 0;
  referenceIndex = 0;
  $("filmstrip").replaceChildren();
  $("film-range").textContent = "";
  $("film-actions").hidden = true;
  $("reference-control").hidden = true;
  $("reference-title").textContent = "Ground truth";
  for (const role of ["source", "reference"]) {
    $(`${role}-meta`).textContent = "";
    placeholder($(`${role}-image`), "Loading scene");
  }
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
  const token = ++requestId;
  try {
    const data = await api(url("/api/scene", { scene: scene.id, run: runId }));
    if (token !== requestId) return;
    const firstLoad = !details;
    details = data;
    outputLoading = false;
    showMessage($("scene-error"), "");
    if (firstLoad) {
      renderFilm();
      renderReference();
    }
    renderOutput();
  } catch (error) {
    if (token !== requestId) return;
    outputLoading = false;
    renderOutput();
    showMessage($("scene-error"), error.message);
    if (!details) {
      for (const role of ["source", "reference"]) placeholder($(`${role}-image`), "Scene unavailable");
    }
  }
}

function renderFilm() {
  const frames = details.frames;
  const start = Math.floor(frameIndex / PAGE_SIZE) * PAGE_SIZE;
  const end = Math.min(start + PAGE_SIZE, frames.length);
  const thumbnails = [];
  for (let index = start; index < end; index++) {
    const frame = frames[index];
    const thumb = part("button", undefined, "thumb");
    thumb.type = "button";
    thumb.title = `Frame ${index + 1} · ${frame.name}`;
    thumb.setAttribute("aria-label", thumb.title);
    thumb.dataset.index = index;
    thumb.onclick = () => selectFrame(index);
    const image = part("img");
    image.alt = "";
    image.loading = "lazy";
    image.decoding = "async";
    image.src = url("/api/image", { frame: frame.id, size: "thumb" });
    thumb.append(image, part("span", String(index + 1).padStart(2, "0")));
    thumbnails.push(thumb);
  }
  const strip = $("filmstrip");
  strip.replaceChildren(...thumbnails);
  strip.scrollLeft = 0;
  $("film-range").textContent = `${start + 1}–${end} of ${number.format(frames.length)}`;
  $("film-actions").hidden = frames.length <= PAGE_SIZE;
  $("frames-previous").disabled = start === 0;
  $("frames-next").disabled = end === frames.length;
  selectFrame(frameIndex);
}

function selectFrame(index) {
  frameIndex = index;
  const frame = details.frames[index];
  for (const thumb of $("filmstrip").children) thumb.setAttribute("aria-pressed", Number(thumb.dataset.index) === index);
  $("source-meta").textContent = frame.focus_distance_m === undefined
    ? `Frame ${index + 1} / ${details.frames.length}`
    : `${number.format(frame.focus_distance_m)} m · ƒ/${number.format(frame.aperture_f_number)}`;
  showImage($("source-image"), `source frame ${index + 1} · ${frame.name}`, { frame: frame.id });
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
    $("run-status").textContent = stopped ? "Stopping evaluation…" : runStates[banner.status];
    $("run-detail").textContent = banner.error ??
      `${banner.scope.label} · ${banner.ready + banner.failed} of ${banner.total} outputs processed${banner.failed ? ` · ${banner.failed} failed` : ""}`;
    $("run-progress").max = banner.total;
    $("run-progress").value = banner.ready + banner.failed;
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
  scope.replaceChildren(new Option("Current scene · " + scene.name, "scene"),
    new Option("Current dataset · " + scene.dataset, "dataset"),
    new Option("All indexed datasets", "all"));
  const choices = [...tools.values()].map((tool) => {
    const label = part("label", undefined, "tool-option");
    const checkbox = part("input");
    checkbox.type = "checkbox";
    checkbox.name = "stacker";
    checkbox.value = tool.id;
    checkbox.checked = tool.available;
    checkbox.disabled = !tool.available;
    label.append(checkbox, part("strong", tool.label), part("span", tool.available ? tool.method : tool.setup));
    return label;
  });
  $("available-tools").replaceChildren(...choices);
  $("start-run").disabled = ![...tools.values()].some((tool) => tool.available);
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

async function boot() {
  try {
    const catalog = await api("/api/catalog");
    if (catalog.status === "indexing") {
      setTimeout(boot, 1200);
      return;
    }
    if (catalog.status === "error" || !catalog.scenes.length) {
      $("catalog-state").querySelector("h1").textContent = "No scenes available";
      $("catalog-note").textContent = catalog.error ?? "The verified catalog contains no usable scenes.";
      return;
    }
    for (const tool of catalog.stackers) tools.set(tool.id, tool);
    scenes = catalog.scenes;
    for (const [id, name] of Object.entries(catalog.datasets)) {
      $("dataset").add(new Option(name === id ? id : `${name} (${id})`, id));
    }
    $("dataset").value = parameters.get("dataset") ?? "";
    if ($("dataset").selectedIndex < 0) $("dataset").value = "";
    runs = await api("/api/runs");
    runId = runs.find((run) => run.id === parameters.get("run"))?.id ?? runs[0]?.id ?? "";
    renderRuns();
    $("catalog-state").hidden = true;
    $("workspace").hidden = false;
    filterScenes(parameters.get("scene"));
    scheduleRuns();
  } catch (error) {
    $("catalog-state").querySelector("h1").textContent = "Could not connect to BokehForge";
    $("catalog-note").textContent = error.message;
  }
}

$("dataset").onchange = () => filterScenes(scene?.id);
$("previous").onclick = () => moveScene(-1);
$("next").onclick = () => moveScene(1);
$("jump").onchange = () => goTo($("jump").valueAsNumber - 1);
$("frames-previous").onclick = () => {
  frameIndex = Math.max(0, (Math.floor(frameIndex / PAGE_SIZE) - 1) * PAGE_SIZE);
  renderFilm();
};
$("frames-next").onclick = () => {
  frameIndex = (Math.floor(frameIndex / PAGE_SIZE) + 1) * PAGE_SIZE;
  renderFilm();
};
$("reference-select").onchange = () => { referenceIndex = Number($("reference-select").value); showReference(); };
$("run-select").onchange = () => changeRun($("run-select").value);
$("rating-form").oninput = captureDraft;
$("rating-form").onsubmit = saveRating;
$("open-run").onclick = openRunDialog;
$("stop-run").onclick = stopRun;
$("run-form").onsubmit = startRun;
$("pixel-mode").onclick = () => setPixelMode($("pixel-mode").getAttribute("aria-pressed") !== "true");
for (const button of document.querySelectorAll("[data-close]")) button.onclick = () => $(button.dataset.close).close();
document.addEventListener("keydown", (event) => {
  if (!scene || document.querySelector("dialog[open]") ||
      event.target.closest?.("input,textarea,select,[contenteditable]") ||
      event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
  if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
    event.preventDefault();
    moveScene(event.key === "ArrowLeft" ? -1 : 1);
  }
});
window.addEventListener("beforeunload", (event) => {
  if (drafts.size) event.preventDefault();
});
boot();
