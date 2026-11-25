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
let frameIndex = 0, referenceIndex = 0, runId = parameters.get("run") ?? "", stackerId = "", formKey = "";
let sceneRequest, scoreRevision, stopping = "";
let warmupTimer, warming = [];
let catalogRevision = -1, lastDownload, rateTimeout, startingDownload = false;
let outputLoading = false;
let runOptions = "", outputState = "", candidateState = "";
let activeImage = null, imageVersion = "";
const readyThumbs = new Map();
const panelImages = new WeakMap();
const LIGHTBOX_HOVER_EXIT_DELAY = 250;

function stackerLabel(id) {
  return tools.get(id)?.label ?? (id === "zerene" ? "Zerene Stacker" : id);
}

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
  const key = imageURL(params, "thumb");
  const image = previewElement(params, "thumb", { title: alt, priority: "low", lazy: true });
  image.classList.add("preview-thumb");
  const failed = () => image.classList.add("preview-failed");
  image.addEventListener("load", () => {
    image.decode().then(() => {
      image.classList.add("preview-loaded");
      readyThumbs.delete(key);
      readyThumbs.set(key, image.currentSrc || image.src);
      if (readyThumbs.size > 120) readyThumbs.delete(readyThumbs.keys().next().value);
    }, failed);
  }, { once: true });
  image.addEventListener("error", failed, { once: true });
  return image;
}

function copyPreview(src) {
  const image = part("img");
  image.alt = "";
  image.src = src;
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
  panelImages.delete(well);
  if (request && !request.image.isConnected) request.image.removeAttribute("src");
}

function holdPanelImage(well) {
  clearImageRequest(well);
  const button = well.querySelector(".pane-image-button");
  if (button) {
    button.setAttribute("aria-disabled", "true");
    button.onclick = null;
  } else {
    well.replaceChildren();
  }
  return button;
}

function showImage(well, title, params) {
  const src = imageURL(params, "panel");
  const current = panelImages.get(well);
  if (current?.src === src && current.button.isConnected) {
    current.title = title;
    if (current.shown) {
      current.button.setAttribute("aria-label", "Inspect " + title);
      current.shown.alt = title;
    }
    return;
  }
  let button = holdPanelImage(well);
  if (!button) {
    button = part("button", undefined, "pane-image-button");
    button.type = "button";
    button.setAttribute("aria-label", "Inspect " + title);
    button.setAttribute("aria-disabled", "true");
    button.append(part("span", "Inspect", "expand"));
    well.append(button);
  }
  const image = previewElement(params, "panel", { title, priority: "high" });
  const request = { src, button, image, title, shown: null };
  panelImages.set(well, request);
  const reveal = (ready) => {
    if (panelImages.get(well) !== request) return;
    ready.classList.add("pane-preview");
    ready.alt = request.title;
    const old = button.querySelector("img");
    if (old) old.replaceWith(ready);
    else button.prepend(ready);
    request.shown = ready;
    button.classList.add("image-ready");
    button.removeAttribute("aria-disabled");
    button.setAttribute("aria-label", "Inspect " + request.title);
    button.onclick = () => openLightbox(request.title, params, request.shown);
  };
  const thumbnail = readyThumbs.get(imageURL(params, "thumb"));
  if (thumbnail) {
    const fallback = copyPreview(thumbnail);
    fallback.decode().then(() => {
      if (!request.shown) reveal(fallback);
    }, () => {});
  }
  image.decode().then(() => reveal(image), () => {
    if (panelImages.get(well) === request && !request.shown) {
      placeholder(well, "Image unavailable", "BokehForge could not render this image.");
    }
  });
}

function cancelLightboxRequests(state) {
  for (const image of state.requests) image.removeAttribute("src");
  state.requests.clear();
}

function cancelReferenceRequests(gallery) {
  for (const { image } of Object.values(gallery.referenceImages)) {
    if (!image.complete) image.removeAttribute("src");
  }
}

function lightboxImageKey({ params }) {
  return params.frame ?? params.stacker;
}

function setLightboxGallery(gallery, data) {
  const images = data.frames.map((frame, index) => ({
    title: sourceFrameTitle(frame, index), label: String(index + 1), params: { frame: frame.id },
  }));
  for (const output of data.outputs) {
    if (output.status === "ready") images.push({
      title: stackerLabel(output.stacker),
      params: { scene: gallery.scene, run: gallery.run, stacker: output.stacker },
    });
  }
  const reference = data.references[gallery.reference];
  for (const [index, frame] of data.references.entries()) {
    if (frame !== reference) images.push({
      title: referenceTitle(frame), label: `Ref. ${index + 1}`, params: { frame: frame.id },
    });
  }
  if (reference) images.push({ title: referenceTitle(reference), label: "Reference", params: { frame: reference.id } });
  const changed = images.length !== gallery.images.length || images.some((entry, index) =>
    lightboxImageKey(entry) !== lightboxImageKey(gallery.images[index]) || entry.title !== gallery.images[index].title);
  if (changed) gallery.images = images;
  gallery.benchmarks = new Map(data.outputs.map(({ stacker, benchmark }) => [stacker, benchmark]));
  if (gallery.referenceFrame?.id !== reference?.id) {
    cancelReferenceRequests(gallery);
    gallery.referenceImages = {};
    gallery.referenceLayer.replaceChildren();
    gallery.referenceFrame = reference;
  }
  if (activeImage?.gallery !== gallery) return;
  if (changed) {
    clearTimeout(gallery.hoverTimer);
    clearLightboxHover(gallery);
    fillLightboxStrip(gallery);
  }
  paintLightboxOverlays();
}

function openLightbox(title, params, previewImage, sceneId = scene?.id) {
  const previous = activeImage;
  if (previous) closeLightboxRequests(previous);
  const stage = part("div", undefined, "lightbox-stage");
  const surface = part("div", undefined, "lightbox-surface");
  const hoverLayer = part("div", undefined, "lightbox-hover");
  const referenceLayer = part("div", undefined, "lightbox-reference");
  for (const layer of [hoverLayer, referenceLayer]) {
    layer.hidden = true;
    layer.setAttribute("aria-hidden", "true");
  }
  surface.append(hoverLayer, referenceLayer);
  stage.append(surface);
  $("image-view").replaceChildren(stage);
  const entry = { title, label: title, params };
  const currentScene = sceneId === scene?.id;
  const gallery = {
    scene: sceneId, run: runId, reference: currentScene ? referenceIndex : 0, images: [entry],
    surface, hoverLayer, hover: null, referenceLayer, referenceImages: {},
    baseEntry: entry, benchmarks: new Map(), swipe: false, split: 0.5, holdKey: "",
  };
  if (currentScene && details) {
    setLightboxGallery(gallery, details);
  } else if (!currentScene && sceneId) {
    loadLightboxGallery(gallery);
  }
  const dialog = $("image-dialog");
  if (!dialog.open) dialog.showModal();
  showLightboxImage(entry, gallery, stage, previewImage);
  fillLightboxStrip(gallery);
  $("image-view").focus();
}

function lightboxScoresPending(gallery) {
  return [...gallery.benchmarks.values()].some((benchmark) => benchmark?.status === "pending");
}

async function loadLightboxGallery(gallery) {
  const revision = scoreRevision;
  const request = new AbortController();
  gallery.request = request;
  const { signal } = request;
  try {
    const data = await api(url("/api/scene", { scene: gallery.scene, run: gallery.run }), undefined, { signal });
    if (signal.aborted) return;
    setLightboxGallery(gallery, data);
    if (revision !== scoreRevision && lightboxScoresPending(gallery)) loadLightboxGallery(gallery);
  } catch {
    // Keep the current preview available if its scene cannot be loaded.
  } finally {
    if (gallery.request === request) gallery.request = null;
  }
}

function lightboxIndex() {
  return activeImage.gallery.images.findIndex((entry) => lightboxImageKey(entry) === lightboxImageKey(activeImage));
}

function fillLightboxStrip(gallery) {
  lightboxRail.setItems(gallery.images);
  lightboxRail.select(lightboxIndex());
}

function moveLightbox(step) {
  const { images } = activeImage.gallery;
  if (images.length < 2) return;
  const index = lightboxIndex();
  const next = index < 0 ? (step > 0 ? 0 : images.length - 1) : (index + step + images.length) % images.length;
  selectLightboxEntry(images[next]);
}

function selectLightboxEntry(entry) {
  const state = activeImage;
  const { gallery, stage } = state;
  clearTimeout(gallery.hoverTimer);
  if (gallery.hover?.entry !== entry) clearLightboxHover(gallery);
  if (lightboxImageKey(state) === lightboxImageKey(entry)) {
    paintLightboxOverlays();
    return;
  }
  const { u, v } = lightboxAnchor();
  cancelLightboxRequests(state);
  // Keep the last rendered image and its label together until the next image loads.
  showLightboxImage(entry, gallery, stage, undefined, {
    image: state.image, scale: state.scale, actual: state.actual, position: { u, v },
  });
  lightboxRail.select(lightboxIndex());
}

function scheduleLightboxHoverReset(gallery) {
  clearTimeout(gallery.hoverTimer);
  if (!gallery.hover) return;
  gallery.hoverTimer = setTimeout(() => {
    if (activeImage?.gallery !== gallery || !$("image-dialog").open) return;
    clearLightboxHover(gallery);
    paintLightboxOverlays();
  }, LIGHTBOX_HOVER_EXIT_DELAY);
}

function previewLightboxEntry(entry, thumbnail) {
  const { gallery } = activeImage;
  const shown = gallery.hover?.shown;
  clearLightboxHover(gallery);
  if (lightboxImageKey(entry) === lightboxImageKey(activeImage)) {
    paintLightboxOverlays();
    return;
  }
  const hover = { entry, shown, request: null, events: new AbortController(), unavailable: false };
  gallery.hover = hover;
  const { signal } = hover.events;
  const useThumbnail = () => {
    if (hover.shown?.entry === entry || !thumbnail.naturalWidth) return;
    const image = copyPreview(thumbnail.currentSrc || thumbnail.src);
    image.decode().then(() => {
      if (signal.aborted || hover.shown?.entry === entry) return;
      hover.shown = { entry, image };
      paintLightboxOverlays();
    }, () => {});
  };
  if (thumbnail.complete) useThumbnail();
  else thumbnail.addEventListener("load", useThumbnail, { once: true, signal });
  requestLightboxHover();
  paintLightboxOverlays();
}

function requestLightboxHover() {
  const state = activeImage;
  const hover = state.gallery.hover;
  if (!hover) return;
  const variant = state.actual || state.scale > 1 ? "native" : "fit";
  if (hover.variant === variant || hover.variant === "native") return;
  hover.variant = variant;
  hover.unavailable = false;
  if (hover.request && !hover.request.complete) hover.request.removeAttribute("src");
  const image = previewElement(hover.entry.params, variant);
  hover.request = image;
  const { signal } = hover.events;
  image.decode().then(() => {
    if (signal.aborted || hover.request !== image) return;
    hover.shown = { entry: hover.entry, image };
    paintLightboxOverlays();
  }, () => {
    if (signal.aborted || hover.request !== image) return;
    hover.unavailable = true;
    paintLightboxOverlays();
  });
}

function clearLightboxHover(gallery) {
  const hover = gallery.hover;
  if (!hover) return;
  hover.events.abort();
  if (hover.request && !hover.request.complete) hover.request.removeAttribute("src");
  gallery.hover = null;
}

function setLightboxTitle(title) {
  const heading = $("image-title");
  if (heading.textContent !== title) {
    heading.textContent = title;
    heading.title = title;
  }
}

function setLightboxImage(state, image) {
  if (state.image) state.image.replaceWith(image);
  else state.gallery.surface.prepend(image);
  state.image = image;
  state.gallery.baseEntry = state.entry;
  const hover = state.gallery.hover;
  if (hover && lightboxImageKey(hover.entry) === lightboxImageKey(state)) clearLightboxHover(state.gallery);
}

function showLightboxImage(entry, gallery, stage, previewImage, viewport = {}) {
  const { title, params } = entry;
  const state = {
    entry, title, params, gallery, stage, image: null, requests: new Set(), scale: 1,
    actual: false, position: null, nativeRequested: false, nativeLoaded: false, error: "",
    ...viewport,
  };
  activeImage = state;
  if (!state.image) gallery.baseEntry = entry;
  const fallback = !state.image && ((previewImage?.complete && previewImage.naturalWidth &&
                    (previewImage.currentSrc || previewImage.src)) ||
    readyThumbs.get(imageURL(params, "thumb")));
  if (fallback) {
    const image = copyPreview(fallback);
    image.alt = title;
    state.requests.add(image);
    image.decode().then(() => {
      state.requests.delete(image);
      if (activeImage !== state || state.image) return;
      setLightboxImage(state, image);
      paintLightbox();
    }, () => state.requests.delete(image));
  }
  requestLightbox(state.actual || state.scale > 1 ? "native" : "fit");
}

function lightboxAnchor(x, y) {
  const state = activeImage;
  const view = $("image-view");
  const rect = view.getBoundingClientRect();
  const position = x === undefined && y === undefined ? state.position : null;
  x ??= rect.left + view.clientWidth / 2;
  y ??= rect.top + view.clientHeight / 2;
  const image = state.image?.getBoundingClientRect();
  return {
    x: x - rect.left, y: y - rect.top,
    u: position?.u ?? (image?.width ? (x - image.left) / image.width : 0.5),
    v: position?.v ?? (image?.height ? (y - image.top) / image.height : 0.5),
  };
}

function paintLightbox(anchor = lightboxAnchor()) {
  const state = activeImage;
  const view = $("image-view");
  const image = state.image;
  $("image-fit").disabled = state.scale === 1 && !state.actual;
  $("zoom-out").disabled = $("image-fit").disabled;
  $("zoom-in").disabled = state.scale >= 12;
  $("pixel-mode").setAttribute("aria-pressed", state.actual);
  requestLightboxHover();
  if (!image?.naturalWidth) {
    paintLightboxOverlays();
    return;
  }
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
  paintLightboxOverlays();
}

function requestLightbox(variant) {
  const state = activeImage;
  const native = variant === "native";
  if (native && (state.nativeRequested || state.nativeLoaded)) return;
  if (native) state.nativeRequested = true;
  state.error = "";
  paintLightbox();
  const image = previewElement(state.params, variant, { title: state.title, priority: "high" });
  state.requests.add(image);
  image.decode().then(() => {
    state.requests.delete(image);
    if (activeImage !== state || (!native && state.nativeLoaded)) return;
    const anchor = lightboxAnchor();
    if (native) state.nativeLoaded = true;
    state.error = "";
    setLightboxImage(state, image);
    paintLightbox(anchor);
  }, () => {
    state.requests.delete(image);
    if (activeImage !== state || (!native && state.nativeLoaded)) return;
    if (native) state.nativeRequested = false;
    state.error = native ? "Full resolution unavailable" : "Image unavailable";
    paintLightbox();
  });
}

function requestReference(gallery, variant, priority) {
  if (gallery.referenceImages[variant]) return;
  const image = previewElement({ frame: gallery.referenceFrame.id }, variant, { priority });
  const request = { image, ready: false, failed: false };
  gallery.referenceImages[variant] = request;
  const changed = () => {
    if (activeImage?.gallery === gallery && gallery.referenceImages[variant] === request) paintLightboxOverlays();
  };
  image.decode().then(() => {
    request.ready = true;
    changed();
  }, () => {
    request.failed = true;
    changed();
  });
}

function visibleLightboxImage(gallery) {
  const view = $("image-view").getBoundingClientRect();
  const image = gallery.surface.getBoundingClientRect();
  const left = Math.max(view.left, image.left);
  const right = Math.min(view.right, image.right);
  const top = Math.max(view.top, image.top);
  const bottom = Math.min(view.bottom, image.bottom);
  return { view, image, left, right, top, bottom };
}

function paintLightboxOverlays() {
  const state = activeImage;
  if (!state) return;
  const { gallery } = state;
  const held = Boolean(gallery.holdKey);
  const comparing = held || gallery.swipe;
  const hover = gallery.hover;
  const shown = hover?.shown;
  const previewing = Boolean(shown && state.image);
  gallery.hoverLayer.hidden = !previewing;
  if (previewing && gallery.hoverLayer.firstChild !== shown.image) gallery.hoverLayer.replaceChildren(shown.image);
  else if (!previewing && gallery.hoverLayer.firstChild) gallery.hoverLayer.replaceChildren();
  const reference = gallery.referenceFrame;
  const toggle = $("reference-swipe");
  toggle.disabled = !reference;
  toggle.setAttribute("aria-pressed", gallery.swipe);
  toggle.title = reference ? "Drag the divider or hold . to see the reference" : "This scene has no reference";
  if (reference) {
    requestReference(gallery, "fit", comparing ? "high" : "low");
    if (comparing && (state.actual || state.scale > 1)) requestReference(gallery, "native", "high");
  }
  const { fit, native } = gallery.referenceImages;
  const overlay = native?.ready ? native.image : fit?.ready ? fit.image : null;
  const showing = comparing && overlay && state.image;
  const layer = gallery.referenceLayer;
  const divider = $("reference-divider");
  layer.hidden = !showing;
  divider.hidden = !showing || held;
  const entry = previewing ? shown.entry : gallery.baseEntry;
  const viewingReference = held && showing;
  setLightboxTitle(viewingReference ? referenceTitle(reference) : entry.title);
  renderBenchmark($("lightbox-benchmark"), viewingReference ? null : gallery.benchmarks.get(entry.params.stacker), true);
  let status = state.error;
  if (hover) status = hover.unavailable && shown?.entry !== hover.entry ? "Preview unavailable" : "";
  if (comparing && !overlay && fit?.failed && (!native || native.failed)) status = "Reference unavailable";
  if ($("lightbox-status").textContent !== status) $("lightbox-status").textContent = status;
  if (!showing) return;
  if (layer.firstChild !== overlay) layer.replaceChildren(overlay);
  if (held) {
    layer.style.clipPath = "none";
    return;
  }

  const { view, image, left, right, top, bottom } = visibleLightboxImage(gallery);
  const edge = left + (right - left) * gallery.split;
  const cut = Math.min(1, Math.max(0, (edge - image.left) / image.width));
  layer.style.clipPath = `inset(0 ${100 * (1 - cut)}% 0 0)`;
  divider.style.left = `${edge - view.left}px`;
  divider.style.top = `${top - view.top}px`;
  divider.style.height = `${Math.max(0, bottom - top)}px`;
  $("reference-slider").setAttribute("aria-valuenow", Math.round(gallery.split * 100));
}

function closeLightboxRequests(state) {
  clearTimeout(state.gallery.hoverTimer);
  cancelLightboxRequests(state);
  clearLightboxHover(state.gallery);
  cancelReferenceRequests(state.gallery);
  state.gallery.request?.abort();
}

function releaseReference(event) {
  const gallery = activeImage?.gallery;
  if (!gallery?.holdKey || (event && gallery.holdKey !== (event.code || event.key))) return;
  gallery.holdKey = "";
  paintLightboxOverlays();
}

function zoomLightbox(scale, x, y) {
  const state = activeImage;
  const anchor = lightboxAnchor(x, y);
  state.position = null;
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
  activeImage.position = null;
  activeImage.actual = true;
  requestLightbox("native");
  paintLightbox();
}

function railItem(className) {
  const item = part("div", undefined, "rail-item " + className);
  item.setAttribute("role", "listitem");
  return item;
}

function inspectButton(title, params, image, sceneId) {
  const button = part("button", undefined, "inspect-thumb");
  button.type = "button";
  button.title = "Inspect " + title;
  button.setAttribute("aria-label", button.title);
  button.onclick = () => openLightbox(title, params, image, sceneId);
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
  const inspect = inspectButton(`scene preview · ${item.name}`, params, image, item.id);
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

function lightboxStripEntry(entry) {
  const gallery = activeImage.gallery;
  const tile = railItem("lightbox-tile");
  const button = part("button", undefined, "thumb preview-thumbnail");
  button.type = "button";
  button.title = entry.title;
  button.setAttribute("aria-label", entry.title);
  const thumbnail = createThumbnail(entry.params);
  button.append(thumbnail, part("span", entry.label ?? entry.title));
  button.addEventListener("pointerenter", (event) => {
    if (event.pointerType !== "touch" && !event.buttons && activeImage?.gallery === gallery) {
      clearTimeout(gallery.hoverTimer);
      if (gallery.hover?.entry !== entry) previewLightboxEntry(entry, thumbnail);
    }
  });
  button.addEventListener("pointerleave", (event) => {
    if (event.pointerType !== "touch" && activeImage?.gallery === gallery) {
      scheduleLightboxHoverReset(gallery);
    }
  });
  button.onclick = () => {
    if (activeImage?.gallery === gallery) selectLightboxEntry(entry);
  };
  tile.append(button);
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
  else holdPanelImage($("reference-image"));
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
  const revision = scoreRevision;
  let loaded = false;
  try {
    const data = await api(url("/api/scene", { scene: scene.id, run: runId }), undefined, { signal });
    if (signal.aborted) return;
    const firstLoad = !details;
    details = data;
    loaded = true;
    const gallery = activeImage?.gallery;
    if (gallery?.scene === scene.id && gallery.run === runId) setLightboxGallery(gallery, data);
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
  } finally {
    if (!signal.aborted) {
      sceneRequest = null;
      // A score may finish while the scene response is still in flight.
      if (loaded && revision !== scoreRevision && pendingScores()) loadScene();
    }
  }
}

function pendingScores() {
  return details?.outputs.some((output) => output.benchmark?.status === "pending");
}

function applyScores(revision) {
  if (revision === scoreRevision) return;
  scoreRevision = revision;
  if (!sceneRequest && pendingScores()) loadScene();
  const gallery = activeImage?.gallery;
  if (gallery && (gallery.scene !== scene?.id || gallery.run !== runId) &&
      !gallery.request && lightboxScoresPending(gallery)) loadLightboxGallery(gallery);
}

function sourceFrameTitle(frame, index) {
  return `source frame ${index + 1} · ${frame.name}`;
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
  showImage($("source-image"), sourceFrameTitle(frame, index), { frame: frame.id });
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

function referenceTitle(frame) {
  return `reference · ${frame.name}`;
}

function showReference() {
  const frame = details.references[referenceIndex];
  $("reference-meta").textContent = frame ? frame.name : "";
  if (frame) showImage($("reference-image"), referenceTitle(frame), { frame: frame.id });
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

const benchmarkNumber = new Intl.NumberFormat(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });

function renderBenchmark(label, benchmark, compact = false) {
  const line = label.parentElement;
  const info = label.nextElementSibling;
  const invalid = benchmark?.status === "invalid_output" && benchmark.score === 0;
  line.hidden = !benchmark;
  info.hidden = !invalid;
  const scored = benchmark?.score != null;
  let text = "";
  if (scored) {
    const value = benchmarkNumber.format(benchmark.score);
    text = compact ? `Score ${value} / 100` : `Benchmark score ${value} / 100`;
    if (invalid) text += " · Invalid delivery";
  } else if (benchmark) {
    const messages = compact
      ? { pending: "Scoring…", unscored: "Unscored" }
      : { pending: "Calculating benchmark score…", unscored: "No eligible benchmark reference" };
    text = messages[benchmark.status] ?? "Benchmark score unavailable";
  }
  if (label.textContent !== text) label.textContent = text;
  line.dataset.scored = scored;
  label.title = benchmark?.profile ?? "";
  info.dataset.reason = invalid ? benchmark.error || "The delivery failed the benchmark's image requirements." : "";
}

function showBenchmarkReason(button) {
  $("benchmark-reason").textContent = button.dataset.reason;
  $("benchmark-details").showModal();
}

function renderOutput() {
  const outputs = details?.outputs ?? [];
  if (outputs.length && !outputs.some((output) => output.stacker === stackerId)) {
    stackerId = (outputs.find((output) => output.status === "ready") ?? outputs[0]).stacker;
  }
  const row = outputs.find((output) => output.stacker === stackerId);
  const tabsKey = JSON.stringify([stackerId, outputs.map((output) =>
    [output.stacker, output.status, output.rating?.grade, output.benchmark?.score])]);
  if (tabsKey !== outputState) {
    const tabStrip = $("output-tabs");
    const focused = tabStrip.contains(document.activeElement) ? document.activeElement.dataset.stacker : null;
    const tabs = outputs.map((output) => {
      const button = part("button", stackerLabel(output.stacker));
      button.type = "button";
      button.dataset.stacker = output.stacker;
      button.title = unavailable[output.status]?.[0] ?? "View stacker output";
      button.setAttribute("aria-pressed", output === row);
      if (output.benchmark?.score != null) {
        const badge = part("small", benchmarkNumber.format(output.benchmark.score) + "/100", "benchmark-badge");
        badge.title = "Benchmark score";
        badge.setAttribute("aria-label", "Benchmark score " + badge.textContent);
        button.append(badge);
      }
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
    const method = runs.find((run) => run.id === runId)?.methods?.[stackerId];
    $("candidate-meta").textContent = row ? stackerLabel(stackerId) : "";
    $("candidate-meta").title = row ? method ?? tools.get(stackerId)?.method ?? "" : "";
    const well = $("candidate-image");
    if (row?.status === "ready") {
      showImage(well, stackerLabel(stackerId), params);
    } else if (row) {
      const [title, description] = unavailable[row.status];
      placeholder(well, title, row.error || description, row.log ? url("/api/log", params) : "");
    } else if (!details || outputLoading) {
      holdPanelImage(well);
    } else {
      placeholder(well, "No evaluation output", "Start an evaluation to compare a stacked image.");
    }
    candidateState = currentImage;
  }
  renderBenchmark($("candidate-benchmark"), row?.benchmark);
  renderRating(row);
}

function ratingKey() {
  return JSON.stringify([runId, scene.id, stackerId]);
}

function renderRating(row) {
  const ready = row?.status === "ready";
  $("rating-title").textContent = ready ? "Rate " + stackerLabel(row.stacker) : "Rate outputs";
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
  if (scene) {
    renderOutput();
    remember();
    loadScene();
  }
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
    const setupLabel = stackerLabel(setup?.stacker);
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

function applyRuns(history) {
  const old = runs.find((run) => run.id === runId);
  runs = history;
  if (!runs.some((run) => run.id === runId)) {
    changeRun(runs[0]?.id ?? "");
  } else {
    const current = runs.find((run) => run.id === runId);
    if (current.status !== old?.status || current.ready !== old?.ready || current.failed !== old?.failed) loadScene();
  }
  renderRuns();
}

function toolSetting(key, setting) {
  const label = part("label", undefined, "tool-setting");
  const control = part(setting.choices ? "select" : "input");
  control.dataset.setting = key;
  if (setting.choices) {
    control.append(...Object.entries(setting.choices).map(([value, text]) => new Option(text, value)));
    control.value = setting.default;
  } else if (typeof setting.default === "boolean") {
    control.type = "checkbox";
    control.checked = setting.default;
    label.classList.add("tool-flag");
  } else {
    control.type = "number";
    control.min = setting.minimum;
    control.max = setting.maximum;
    control.step = setting.step;
    control.value = setting.default;
    control.required = true;
  }
  const text = part("span", setting.label);
  label.append(...(control.type === "checkbox" ? [control, text] : [text, control]));
  return { label, control, when: setting.when };
}

function toolOption(tool, prepared) {
  const card = part("div", undefined, "tool-option");
  const label = part("label", undefined, "tool-option-label");
  const checkbox = part("input");
  checkbox.type = "checkbox";
  checkbox.name = "stacker";
  checkbox.value = tool.id;
  const ready = tool.available || (!tool.setup_hint && prepared.has(tool.id));
  checkbox.checked = ready;
  const subtitle = part("span");
  label.append(checkbox, part("strong", tool.label), subtitle);

  const details = part("details", undefined, "tool-settings");
  const summary = part("summary", "Settings");
  summary.setAttribute("aria-label", tool.label + " settings");
  const grid = part("div", undefined, "tool-settings-grid");
  const settings = new Map(Object.entries(tool.options).map(([key, setting]) => [key, toolSetting(key, setting)]));
  grid.append(...Array.from(settings.values(), ({ label }) => label));
  const update = () => {
    for (const { label, control, when } of settings.values()) {
      const hidden = Boolean(when && settings.get(when[0]).control.value !== when[1]);
      label.hidden = hidden;
      control.disabled = hidden;
    }
    const fusion = settings.get("method")?.control;
    const method = fusion?.selectedOptions[0]?.text ?? tool.method;
    const hint = ready ? "" :
      tool.setup_hint ? " · Local installation required" : " · Sets up on first run";
    subtitle.textContent = method + hint;
    subtitle.title = tool.setup_hint && !tool.available ? tool.setup_hint : subtitle.textContent;
  };
  details.addEventListener("change", update);
  details.append(summary, grid);
  if (tool.setup_hint && !tool.available) details.append(part("p", tool.setup_hint, "tool-setup-hint muted"));
  card.append(label, details);
  update();
  return card;
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
  $("available-tools").replaceChildren(...Array.from(tools.values(), (tool) => toolOption(tool, prepared)));
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
  const options = {};
  for (const checkbox of event.target.querySelectorAll('input[name="stacker"]:checked')) {
    const details = checkbox.closest(".tool-option").querySelector(".tool-settings");
    const settings = {};
    for (const input of details.querySelectorAll("[data-setting]:enabled")) {
      if (!input.validity.valid) {
        details.open = true;
        input.reportValidity();
        return;
      }
      settings[input.dataset.setting] = input.type === "checkbox" ? input.checked :
        input.type === "number" ? input.valueAsNumber : input.value;
    }
    options[checkbox.value] = settings;
  }
  $("start-run").disabled = true;
  try {
    const result = await api("/api/runs", {
      scope: $("scope").value, scene: scene.id, dataset: scene.dataset, stackers, options,
    });
    $("new-run").close();
    changeRun(result.id);
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
  if (catalog.revision === catalogRevision && catalog.image_version === imageVersion) return;
  const selectedScene = scene?.id ?? parameters.get("scene");
  if (imageVersion && catalog.image_version !== imageVersion) {
    scene = null;
    details = null;
  }
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
    filterScenes(selectedScene);
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

function applyDownload(status) {
  lastDownload = status;
  renderDownload(status);
  clearTimeout(rateTimeout);
  if (status.rate) {
    // The estimate expires once if transfer events go quiet.
    rateTimeout = setTimeout(() => renderDownload({ ...status, rate: null, seconds_remaining: null }), 4000);
  }
}

async function startDownload() {
  startingDownload = true;
  $("download-start").disabled = true;
  showMessage($("download-error"), "");
  try {
    await api("/api/download", {});
  } catch (error) {
    showMessage($("download-error"), error.message);
  } finally {
    startingDownload = false;
    $("download-start").disabled = Boolean(lastDownload?.indexing);
  }
}

function connect() {
  const events = new EventSource("/api/events");
  events.addEventListener("runs", (event) => applyRuns(JSON.parse(event.data)));
  events.addEventListener("scores", (event) => applyScores(JSON.parse(event.data)));
  events.addEventListener("catalog", (event) => applyCatalog(JSON.parse(event.data)));
  events.addEventListener("download", (event) => applyDownload(JSON.parse(event.data)));
  events.onopen = () => showMessage($("connection-status"), "");
  events.onerror = () => showMessage($("connection-status"), "Live updates disconnected. Reconnecting…");
}

const sceneRail = new VirtualRail($("scene-list"), "y", 74, sceneEntry);
const filmRail = new VirtualRail($("filmstrip"), "x", 112, frameEntry);
const lightboxStrip = $("lightbox-strip");
const lightboxRail = new VirtualRail(lightboxStrip, "x", 100, lightboxStripEntry);
lightboxStrip.addEventListener("pointerleave", (event) => {
  const gallery = activeImage?.gallery;
  if (event.pointerType !== "touch" && gallery) scheduleLightboxHoverReset(gallery);
});

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
$("reference-swipe").onclick = () => {
  const gallery = activeImage.gallery;
  gallery.swipe = !gallery.swipe;
  paintLightboxOverlays();
};
const referenceSlider = $("reference-slider");
function setReferenceSplit(split) {
  activeImage.gallery.split = Math.max(0, Math.min(1, split));
  paintLightboxOverlays();
}
function dragReference(event) {
  const { left, right } = visibleLightboxImage(activeImage.gallery);
  if (right > left) setReferenceSplit((event.clientX - left) / (right - left));
}
referenceSlider.addEventListener("pointerdown", (event) => {
  if (event.button !== 0) return;
  referenceSlider.setPointerCapture(event.pointerId);
  dragReference(event);
});
referenceSlider.addEventListener("pointermove", (event) => {
  if (referenceSlider.hasPointerCapture(event.pointerId) && activeImage?.gallery.swipe && !activeImage.gallery.holdKey) {
    dragReference(event);
  }
});
referenceSlider.addEventListener("pointerup", () => lightboxView.focus({ preventScroll: true }));
referenceSlider.addEventListener("keydown", (event) => {
  const { split } = activeImage.gallery;
  const next = { ArrowLeft: split - 0.05, ArrowRight: split + 0.05, Home: 0, End: 1 }[event.key];
  if (next === undefined || event.ctrlKey || event.metaKey || event.altKey) return;
  event.preventDefault();
  event.stopPropagation();
  setReferenceSplit(next);
});
lightboxView.addEventListener("scroll", paintLightboxOverlays, { passive: true });
lightboxView.addEventListener("wheel", (event) => {
  if (event.shiftKey) {
    activeImage.position = null;
    return;
  }
  event.preventDefault();
  const units = event.deltaMode ? 20 : 1;
  const scale = Math.exp(Math.max(-1, Math.min(1, -event.deltaY * units / 300)));
  zoomLightbox(lightboxScale() * scale, event.clientX, event.clientY);
}, { passive: false });
lightboxView.addEventListener("dblclick", (event) => {
  zoomLightbox(lightboxScale() > 1 ? 1 : 2, event.clientX, event.clientY);
});
document.addEventListener("keydown", (event) => {
  if (!activeImage || !$("image-dialog").open || $("benchmark-details").open ||
      event.ctrlKey || event.metaKey || event.altKey) return;
  if (event.key.toLowerCase() === "i") {
    const button = $("lightbox-benchmark").nextElementSibling;
    if (!button.hidden && !event.repeat) showBenchmarkReason(button);
    return;
  }
  if (event.key === ".") {
    if (activeImage.gallery.referenceFrame) {
      event.preventDefault();
      if (!event.repeat && !activeImage.gallery.holdKey) {
        activeImage.gallery.holdKey = event.code || event.key;
        paintLightboxOverlays();
      }
    }
    return;
  }
  if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
    event.preventDefault();
    moveLightbox(event.key === "ArrowLeft" ? -1 : 1);
    return;
  }
  if (event.target === lightboxView &&
      ["ArrowUp", "ArrowDown", "PageUp", "PageDown", "Home", "End", " "].includes(event.key)) {
    activeImage.position = null;
  }
  const zoom = { "+": 1.5, "=": 1.5, "-": 1 / 1.5 }[event.key];
  if (!zoom && event.key !== "0") return;
  event.preventDefault();
  zoomLightbox(zoom ? lightboxScale() * zoom : 1);
});
document.addEventListener("keyup", releaseReference);
window.addEventListener("blur", () => releaseReference());
document.addEventListener("visibilitychange", () => {
  if (document.hidden) releaseReference();
});
let panStart = null;
lightboxView.addEventListener("pointerdown", (event) => {
  if (!lightboxView.classList.contains("pannable")) return;
  if (event.pointerType === "touch") {
    activeImage.position = null;
    return;
  }
  if (event.button !== 0) return;
  lightboxView.setPointerCapture(event.pointerId);
  panStart = { x: event.clientX, y: event.clientY, left: lightboxView.scrollLeft, top: lightboxView.scrollTop };
  lightboxView.classList.add("dragging");
});
lightboxView.addEventListener("pointermove", (event) => {
  if (!panStart || !(event.buttons & 1)) return;
  activeImage.position = null;
  lightboxView.scrollTo(panStart.left + panStart.x - event.clientX, panStart.top + panStart.y - event.clientY);
});
function endLightboxPan() {
  panStart = null;
  lightboxView.classList.remove("dragging");
}
lightboxView.addEventListener("pointerup", endLightboxPan);
lightboxView.addEventListener("lostpointercapture", endLightboxPan);
$("image-dialog").addEventListener("close", (event) => {
  if (event.currentTarget.open || !activeImage) return;
  closeLightboxRequests(activeImage);
  activeImage = null;
  endLightboxPan();
  lightboxRail.setItems([]);
  lightboxView.replaceChildren();
});
new ResizeObserver(() => {
  if (activeImage) paintLightbox();
}).observe(lightboxView);
for (const button of document.querySelectorAll(".benchmark-info")) button.onclick = () => showBenchmarkReason(button);
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
connect();
