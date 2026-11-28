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
    return "rgba(" + Math.round(90 + 165 * t) + "," + Math.round(80 + 121 * t) + "," +
      Math.round(225 - 177 * t) + "," + (0.26 + 0.57 * t) + ")";
  });
  let index = 0;
  for (let row = 0; row < ys.length - 1; row += 1) {
    for (let col = 0; col < xs.length - 1; col += 1, index += 1) {
      const value = values[index];
      if (!value || coverage?.[index] === 0) continue;
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
    image.data[offset + 3] = 210 * value;
  }
  sourceContext.putImageData(image, 0, 0);
  overlayContext(canvas, payload).drawImage(source, 0, 0, ...payload.reference_size_wh);
}

export class ScoreOverlay {
  constructor(root, settings, changeSettings, requestOverlay) {
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
    root.parentElement.append(this.legend);
    this.canvas = element("canvas", undefined, "damage-canvas");
    this.canvas.hidden = true;
    this.canvas.setAttribute("aria-hidden", "true");
    this.families = null;
    this.tile = null;
    this.drawn = null;

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
    this.picker.addEventListener("toggle", ({ newState }) => {
      if (newState === "open") {
        (this.picker.querySelector('[aria-pressed="true"]') ?? this.picker.firstElementChild).focus();
      }
    });
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

  scoresChanged(revision) {
    this.eventRevision = revision;
    if (this.tile?.state === "pending") this.request(this.tile.key);
  }

  request(key) {
    this.tile?.controller.abort();
    const controller = new AbortController();
    const tile = { key, state: "loading", controller };
    this.tile = tile;
    const revision = this.eventRevision;
    const { params, benchmark } = this.context;
    const { mode, family, side } = this.settings;
    const query = mode === "attention" ? params : { ...params, family, side };
    this.requestOverlay(mode, query, controller.signal).then((result) => {
      if (this.tile !== tile || controller.signal.aborted) return;
      if (result.status === "pending") {
        tile.state = "pending";
        if (this.eventRevision !== revision) this.request(key);
      } else if (result.status === "ok" && result.artifact_revision === params.rev &&
                 result.reference_id === benchmark.explanation.reference_id &&
                 result.inspection_map_sha256 === benchmark.inspection_map_sha256 &&
                 (mode === "attention" ? result.attention?.encoding === "base64+uint8"
                   : result.family === family && result.grid?.tile_size === side)) {
        overlayCache.set(key, result);
        if (overlayCache.size > 12) overlayCache.delete(overlayCache.keys().next().value);
        tile.payload = result;
        tile.state = "ready";
      } else {
        tile.state = result.status === "stale" ? "stale" : "error";
      }
      this.paintHeatmap();
    }, () => {
      if (this.tile !== tile || controller.signal.aborted) return;
      tile.state = "error";
      this.paintHeatmap();
    });
  }

  paint() {
    this.paintControls();
    this.paintHeatmap();
  }

  paintHeatmap() {
    const { context } = this;
    const { mode, family, side, opacity } = this.settings;
    const attention = mode === "attention";
    const identity = context?.image && mode !== "off" ? scoreExplanationKey(context) : "";
    const explanation = identity ? context.benchmark.explanation : null;
    const placement = explanation?.placements[0];
    const comparison = placement?.comparison;
    const familyLabel = family === "all" && comparison ? "All damage" : comparison?.families[family]?.label;
    const available = attention ? placement && explanation.attention : familyLabel;
    const key = available ? JSON.stringify([identity, mode, ...(attention ? [] : [family, side])]) : "";
    if (this.tile?.key !== key) {
      this.tile?.controller.abort();
      this.tile = null;
      if (key && !overlayCache.has(key)) this.request(key);
    }
    const payload = key ? this.tile?.payload ?? overlayCache.get(key) : null;
    const preparing = attention ? "Preparing attention map…" : "Preparing damage tiles…";
    const text = key && !payload ? overlayStatus[this.tile?.state] ?? preparing : "";
    if (text !== this.status.textContent) this.status.textContent = text;
    if (!payload) {
      this.canvas.hidden = this.legend.hidden = true;
      if (this.drawn) {
        this.canvas.remove();
        this.canvas.width = this.canvas.height = 0;
        this.drawn = null;
      }
      return;
    }
    if (this.drawn !== payload) {
      if (attention) drawAttention(this.canvas, payload);
      else drawGrid(this.canvas, payload);
      this.drawn = payload;
      this.legend.dataset.mode = mode;
      this.legend.firstElementChild.textContent = attention ? "Reference attention" : familyLabel + " · " + side + " px tiles";
      const maximum = payload.damage_range?.[1] ?? 1;
      Array.from(this.legend.querySelector(".damage-legend-values").children).forEach((tick, index) => {
        tick.textContent = number.format(maximum * index / 4);
      });
    }
    this.canvas.style.opacity = opacity / 100;
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
