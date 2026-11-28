import { scoreExplanationKey } from "./score-explanation.js";

const number = new Intl.NumberFormat(undefined, { maximumFractionDigits: 4 });
const overlayCache = new Map();
const overlayStatus = {
  error: "Overlay unavailable. Turn the overlay off and on to try again.",
  stale: "This output changed. Reload the scene to see its current overlay.",
};

function element(tag, text, className) {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  if (className) node.className = className;
  return node;
}

function overlayContext(canvas, payload) {
  const [width, height] = payload.candidate_size_wh;
  const [[a, b, tx], [c, d, ty]] = payload.candidate_to_reference_xy;
  const determinant = a * d - b * c;
  const [ia, ib, ic, id] = [d, -b, -c, a].map((value) => value / determinant);
  // The returned affine uses pixel centers. Draw the half-open boxes on pixel edges.
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d");
  const [left, top, right, bottom] = payload.candidate_content_box_xyxy;
  ctx.beginPath();
  ctx.rect(left, top, right - left, bottom - top);
  ctx.clip();
  ctx.setTransform(ia, ic, ib, id, 0.5 - ia * (tx + 0.5) - ib * (ty + 0.5),
    0.5 - ic * (tx + 0.5) - id * (ty + 0.5));
  return ctx;
}

function drawGrid(canvas, payload) {
  const ctx = overlayContext(canvas, payload);
  const maximum = payload.damage_range?.[1] ?? 1;
  const { x_edges: xs, y_edges: ys, values, covered_pixels: coverage } = payload.grid;
  const colors = Array.from({ length: 256 }, (_, index) => {
    const t = index / 255;
    return `rgb(${Math.round(90 + 165 * t)}, ${Math.round(80 + 121 * t)}, ${Math.round(225 - 177 * t)})`;
  });
  let index = 0;
  for (let row = 0; row < ys.length - 1; row += 1) {
    for (let col = 0; col < xs.length - 1; col += 1, index += 1) {
      const value = values[index];
      if (value == null || coverage?.[index] === 0) continue;
      ctx.fillStyle = colors[Math.round(255 * Math.min(value / maximum, 1))];
      ctx.fillRect(xs[col], ys[row], xs[col + 1] - xs[col], ys[row + 1] - ys[row]);
    }
  }
}

function drawAttention(canvas, payload) {
  const [width, height] = payload.attention.size_wh;
  const values = atob(payload.attention.data);
  const source = element("canvas");
  source.width = width;
  source.height = height;
  const sourceContext = source.getContext("2d");
  const image = sourceContext.createImageData(width, height);
  for (let index = 0; index < values.length; index += 1) {
    const value = values.charCodeAt(index) / 255;
    const offset = index * 4;
    image.data[offset] = 29 + 79 * value;
    image.data[offset + 1] = 132 + 90 * value;
    image.data[offset + 2] = 165 + 80 * value;
    image.data[offset + 3] = 255;
  }
  sourceContext.putImageData(image, 0, 0);
  overlayContext(canvas, payload).drawImage(source, 0, 0, ...payload.reference_size_wh);
}

function heatmapSpec(output, { mode, family, side }) {
  const identity = mode === "off" ? "" : scoreExplanationKey(output);
  if (!identity) return null;
  const { params, benchmark } = output;
  const { explanation } = benchmark;
  const placement = explanation.placements[0];
  const comparison = placement?.comparison;
  const label = family === "all" && comparison ? "All damage" : comparison?.families[family]?.label;
  if (mode === "attention" ? !placement || !explanation.attention : !label) return null;
  const key = JSON.stringify([identity, mode, ...(mode === "attention" ? [] : [family, side])]);
  return { key, identity, params, benchmark, mode, family, side, label };
}

function loading(tile) {
  return tile?.state === "loading" || tile?.state === "pending";
}

export class ScoreOverlay {
  constructor(root, legendHost, settings, changeSettings, requestOverlay) {
    this.root = root;
    this.settings = settings;
    this.changeSettings = changeSettings;
    this.requestOverlay = requestOverlay;
    root.append(document.querySelector("#score-overlay-template").content.cloneNode(true));
    this.attention = root.querySelector("[data-attention]");
    this.damage = root.querySelector("[data-damage]");
    this.grid = root.querySelector("[data-overlay-grid]");
    this.gridSize = this.grid.querySelector("span");
    this.opacity = root.querySelector("[data-overlay-opacity]");
    this.picker = root.querySelector(".overlay-picker");
    this.status = root.querySelector(".overlay-status");
    this.legend = document.querySelector("#damage-legend-template").content.firstElementChild.cloneNode(true);
    legendHost.append(this.legend);
    this.canvas = element("canvas", undefined, "damage-canvas");
    this.canvas.hidden = true;
    this.canvas.setAttribute("aria-hidden", "true");
    this.families = null;
    this.requests = new Map();
    this.presentations = new Map();
    this.drawn = null;
    this.painting = false;

    this.picker.id = root.id + "-damage-types";
    this.damage.setAttribute("popovertarget", this.picker.id);
    this.attention.onclick = () => changeSettings({ mode: settings.mode === "attention" ? "off" : "attention" });
    this.damage.onclick = (event) => {
      if (settings.mode === "damage") return;
      event.preventDefault();
      changeSettings({ mode: "damage" });
    };
    this.grid.onclick = () => changeSettings({ side: settings.side === 8 ? 32 : 8 });
    this.opacity.oninput = () => changeSettings({ opacity: this.opacity.valueAsNumber });
    const needsPickerPosition = !CSS.supports("position-area", "bottom span-left");
    if (needsPickerPosition) {
      const reposition = () => {
        if (this.picker.matches(":popover-open")) this.positionPicker();
      };
      window.addEventListener("resize", reposition);
      window.addEventListener("scroll", reposition, { capture: true, passive: true });
    }
    this.picker.addEventListener("toggle", ({ newState }) => {
      if (newState === "open") {
        if (needsPickerPosition) this.positionPicker();
        (this.picker.querySelector('[aria-pressed="true"]') ?? this.picker.firstElementChild).focus();
      }
    });
  }

  positionPicker() {
    const anchor = this.damage.getBoundingClientRect();
    const { offsetWidth: width, offsetHeight: height } = this.picker;
    const { clientWidth, clientHeight } = document.documentElement;
    const gap = 5;
    const below = anchor.bottom + gap;
    const above = anchor.top - height - gap;
    const top = below + height <= clientHeight || above < 0 ? below : above;
    this.picker.style.left = Math.max(0, Math.min(clientWidth - width, anchor.right - width)) + "px";
    this.picker.style.top = Math.max(0, Math.min(clientHeight - height, top)) + "px";
  }

  update(output, context) {
    const selectionChanged = output?.params !== this.output?.params &&
      ["run", "scene", "stacker", "rev"].some((field) => output?.params[field] !== this.output?.params[field]);
    if (selectionChanged) this.picker.hidePopover();
    this.output = output;
    this.context = context;
    this.paint();
  }

  paintControls() {
    const { mode, family, side, opacity } = this.settings;
    const explanation = scoreExplanationKey(this.output) ? this.output.benchmark.explanation : null;
    const placement = explanation?.placements[0];
    const families = placement?.comparison?.families;
    const attentionAvailable = Boolean(placement && explanation.attention);
    const damageAvailable = Boolean(placement?.comparison);
    const attentionActive = mode === "attention";
    const damageActive = mode === "damage";
    this.root.hidden = !this.output;
    this.attention.disabled = !attentionAvailable && !attentionActive;
    this.damage.disabled = !damageAvailable && !damageActive;
    this.grid.disabled = !damageActive || !damageAvailable;
    this.opacity.disabled = !(attentionActive && attentionAvailable || damageActive && damageAvailable);
    this.attention.setAttribute("aria-pressed", String(attentionActive));
    this.damage.setAttribute("aria-pressed", String(damageActive));
    this.attention.title = attentionAvailable || attentionActive ? "Toggle attention heatmap" : "Attention heatmap unavailable";
    this.damage.title = damageActive ? "Choose damage type or turn off heatmap" :
      damageAvailable ? "Show damage heatmap" : "Damage heatmap unavailable";
    this.grid.title = damageActive
      ? `Damage tile size: ${side} px. Switch to ${side === 8 ? 32 : 8} px.`
      : "Damage tile size";
    this.grid.setAttribute("aria-label", this.grid.title);
    this.gridSize.textContent = side;
    this.opacity.value = opacity;
    this.opacity.setAttribute("aria-valuetext", opacity + "%");
    if (!damageActive || !this.output) this.picker.hidePopover();
    if (!this.output) return;

    if (families !== this.families) {
      this.families = families;
      const choices = [["all", "All damage"], ...Object.entries(families ?? {}).map(([id, info]) => [id, info.label]),
        ["off", "Turn off damage heatmap"]];
      this.picker.replaceChildren(...choices.map(([id, label]) => {
        const button = element("button", label);
        button.type = "button";
        button.dataset.family = id;
        button.onclick = () => {
          this.picker.hidePopover();
          this.changeSettings(id === "off" ? { mode: "off" } : { family: id });
          if (!this.damage.disabled) this.damage.focus();
        };
        return button;
      }));
    }
    for (const choice of this.picker.children) {
      if (choice.dataset.family !== "off") choice.setAttribute("aria-pressed", String(choice.dataset.family === family));
    }
  }

  stage(slot, output, reveal) {
    this.presentations.set(slot, { output, reveal });
    this.paintHeatmap();
  }

  cancelPresentation(slot) {
    this.presentations.delete(slot);
  }

  scoresChanged(revision) {
    this.eventRevision = revision;
    for (const tile of this.requests.values()) {
      if (tile.state === "pending") this.request(tile.spec);
    }
  }

  request(spec) {
    const { key, params, benchmark, mode, family, side } = spec;
    this.requests.get(key)?.controller.abort();
    const controller = new AbortController();
    const tile = { spec, state: "loading", controller };
    this.requests.set(key, tile);
    const revision = this.eventRevision;
    const query = mode === "attention" ? params : { ...params, family, side };
    this.requestOverlay(mode, query, controller.signal).then((result) => {
      if (controller.signal.aborted) return;
      if (result.status === "pending") {
        tile.state = "pending";
        if (this.eventRevision !== revision) this.request(spec);
      } else if (result.status === "ok" && result.artifact_revision === params.rev &&
                 result.reference_id === benchmark.explanation.reference_id &&
                 result.inspection_map_sha256 === benchmark.inspection_map_sha256 &&
                 (mode === "attention" ? result.attention?.encoding === "base64+uint8"
                   : result.family === family && result.grid?.tile_size === side)) {
        overlayCache.set(key, result);
        if (overlayCache.size > 12) overlayCache.delete(overlayCache.keys().next().value);
        tile.state = "ready";
      } else {
        tile.state = result.status === "stale" ? "stale" : "error";
      }
      this.paintHeatmap();
    }, () => {
      if (controller.signal.aborted) return;
      tile.state = "error";
      this.paintHeatmap();
    });
    return tile;
  }

  load(spec, needed) {
    if (!spec) return null;
    const { key } = spec;
    const cached = overlayCache.get(key) ?? (this.drawn?.key === key ? this.drawn.payload : null);
    if (cached) return { spec, state: "ready", payload: cached };
    needed.add(key);
    return this.requests.get(key) ?? this.request(spec);
  }

  paint() {
    this.paintControls();
    this.paintHeatmap();
  }

  paintHeatmap() {
    // Revealing a staged image updates the display context synchronously.
    if (this.painting) return;
    this.painting = true;
    try {
      const needed = new Set();
      let stagedTile = null;
      for (const [slot, { output, reveal }] of this.presentations) {
        const keepVisible = this.context?.image && this.drawn && !this.canvas.hidden && this.settings.opacity > 0;
        const tile = keepVisible ? this.load(heatmapSpec(output(), this.settings), needed) : null;
        if (loading(tile)) {
          stagedTile = tile;
        } else {
          this.presentations.delete(slot);
          reveal();
        }
      }
      const context = this.context;
      const spec = context?.image ? heatmapSpec(context, this.settings) : null;
      const tile = this.load(spec, needed);
      for (const [key, pending] of this.requests) {
        if (needed.has(key)) continue;
        pending.controller.abort();
        this.requests.delete(key);
      }
      const statusTile = stagedTile ?? tile;
      const preparing = statusTile?.spec.mode === "attention" ? "Preparing attention map…" : "Preparing damage tiles…";
      const text = statusTile && !statusTile.payload ? overlayStatus[statusTile.state] ?? preparing : "";
      if (text !== this.status.textContent) this.status.textContent = text;
      this.drawHeatmap(context, tile, stagedTile);
    } finally {
      this.painting = false;
    }
  }

  drawHeatmap(context, tile, stagedTile) {
    const { spec, payload } = tile ?? {};
    if (payload) {
      if (this.drawn?.key !== spec.key || this.drawn.payload !== payload) {
        const { mode, label, side } = spec;
        const attention = mode === "attention";
        if (attention) drawAttention(this.canvas, payload);
        else drawGrid(this.canvas, payload);
        this.drawn = { key: spec.key, identity: spec.identity, payload };
        this.legend.dataset.mode = mode;
        this.legend.firstElementChild.textContent = attention ? "Reference attention" : label + " · " + side + " px tiles";
        const maximum = payload.damage_range?.[1] ?? 1;
        Array.from(this.legend.querySelector(".damage-legend-values").children).forEach((tick, index) => {
          tick.textContent = number.format(maximum * index / 4);
        });
      }
    } else if ((!loading(tile) && !stagedTile) || this.drawn?.identity !== scoreExplanationKey(context)) {
      this.canvas.hidden = this.legend.hidden = true;
      if (this.drawn) {
        this.canvas.remove();
        this.canvas.width = this.canvas.height = 0;
        this.drawn = null;
      }
      return;
    }
    this.canvas.style.opacity = this.settings.opacity / 100;
    this.placeOverlay(context);
  }

  placeOverlay({ host, image, contain, clipLeft }) {
    if (this.canvas.parentElement !== host) host.append(this.canvas);
    const bounds = image.getBoundingClientRect();
    const origin = host.getBoundingClientRect();
    const fit = Math.min(bounds.width / image.naturalWidth, bounds.height / image.naturalHeight);
    const width = contain ? image.naturalWidth * fit : bounds.width;
    const height = contain ? image.naturalHeight * fit : bounds.height;
    const left = bounds.left + (bounds.width - width) / 2;
    const top = bounds.top + (bounds.height - height) / 2;
    const inset = Math.max(0, (clipLeft ?? left) - left);
    Object.assign(this.canvas.style, {
      left: (left - origin.left) + "px", top: (top - origin.top) + "px",
      width: width + "px", height: height + "px",
      clipPath: inset ? "inset(0 0 0 " + inset + "px)" : "none",
    });
    this.canvas.hidden = this.legend.hidden = width <= inset || height <= 0;
  }
}
