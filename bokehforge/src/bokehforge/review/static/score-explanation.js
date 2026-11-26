const number = new Intl.NumberFormat(undefined, { maximumFractionDigits: 4 });
const points = new Intl.NumberFormat(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
const percent = new Intl.NumberFormat(undefined, { style: "percent", maximumFractionDigits: 1 });
const tileCache = new Map();
const tileStatus = {
  loading: "Preparing damage tiles…",
  pending: "Preparing damage tiles…",
  error: "Damage overlay unavailable. Toggle it off and on to try again.",
  stale: "This output changed. Reload the scene to see its current overlay.",
};

function element(tag, text, className) {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  if (className) node.className = className;
  return node;
}

function damage(value) {
  return value > 0 && value < 0.0001 ? value.toExponential(1) : number.format(value);
}

function familyRow(family) {
  const row = element("details", undefined, "damage-family");
  const summary = element("summary");
  const values = [
    [family.label, ""],
    [damage(family.burden), "Observed damage"],
    [percent.format(family.weight), "Weight"],
    [damage(family.weighted_damage), "Contribution"],
  ];
  for (const [value, label] of values) {
    const cell = element("span", value);
    if (label) cell.setAttribute("aria-label", label + " " + value);
    summary.append(cell);
  }
  const pool = element("dl", undefined, "damage-pool");
  for (const component of Object.values(family.pool)) {
    const line = element("div");
    line.append(element("dt", component.label),
      element("dd", damage(component.value) + " × " + percent.format(component.weight) +
        " = " + damage(component.contribution)));
    pool.append(line);
  }
  row.append(summary, element("p", family.description), pool);
  return row;
}

function drawGrid(canvas, payload) {
  const [width, height] = payload.candidate_size_wh;
  const { x_edges: xs, y_edges: ys, values, covered_pixels: coverage } = payload.grid;
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
      ctx.fillStyle = colors[Math.round(255 * Math.min(value, 1))];
      ctx.fillRect(xs[col], ys[row], xs[col + 1] - xs[col], ys[row + 1] - ys[row]);
    }
  }
}

export class ScoreExplanation {
  constructor(root, trigger, legendHost, requestTiles) {
    this.root = root;
    this.trigger = trigger;
    this.requestTiles = requestTiles;
    root.append(document.querySelector("#score-explanation-template").content.cloneNode(true));
    this.toggle = root.querySelector("[data-overlay]");
    this.family = root.querySelector("[data-family]");
    this.grid = root.querySelector("[data-grid]");
    this.status = root.querySelector("[data-damage-status]");
    this.canvas = element("canvas", undefined, "damage-canvas");
    this.canvas.hidden = true;
    this.canvas.setAttribute("aria-hidden", "true");
    this.legend = document.querySelector("#damage-legend-template").content.firstElementChild.cloneNode(true);
    legendHost.append(this.legend);
    this.key = "";
    this.open = false;
    this.overlayFor = "";
    this.tile = null;
    this.drawn = null;
    this.context = null;
    this.eventRevision = null;
    trigger.onclick = () => this.setOpen(!this.open);
    root.querySelector(".score-explanation-dismiss").onclick = () => {
      this.setOpen(false);
      trigger.focus();
    };
    this.toggle.onchange = () => {
      this.overlayFor = this.toggle.checked ? this.key : "";
      this.paintOverlay();
    };
    this.family.onchange = this.grid.onchange = () => this.paintOverlay();
  }

  setOpen(open) {
    this.open = open;
    this.root.hidden = !(open && this.key);
    this.trigger.setAttribute("aria-expanded", String(!this.root.hidden));
    this.trigger.textContent = open && this.key ? "Hide explanation" : "Explain score";
  }

  close() {
    this.overlayFor = "";
    this.setOpen(false);
    this.update(null);
  }

  update(context) {
    const benchmark = context?.benchmark;
    const explanation = benchmark?.explanation;
    const placement = explanation?.placements[0];
    const p = context?.params;
    const key = explanation && p?.rev && p.rev === benchmark.artifact_revision
      ? JSON.stringify([p.run, p.scene, p.stacker, p.rev, explanation.reference_id, benchmark.profile]) : "";
    this.context = key ? context : null;
    if (key !== this.key) {
      this.key = key;
      this.trigger.hidden = !key;
      this.setOpen(this.open);
      if (key) this.showSummary(explanation, placement, benchmark.alignment);
    }
    this.toggle.checked = Boolean(placement?.comparison && key && this.overlayFor === key);
    this.toggle.disabled = !context?.image || !placement?.comparison;
    this.paintOverlay();
  }

  showSummary(explanation, placement, alignment) {
    const comparison = placement?.comparison;
    const families = Object.entries(comparison?.families ?? {});
    const previousFamily = this.family.value;
    this.root.querySelector("[data-score-heading]").textContent =
      "Benchmark score " + points.format(explanation.score) + " / 100";
    const summary = this.root.querySelector("[data-alignment-summary]");
    if (!placement) {
      summary.textContent = "The scorer could not place the delivery on the reference.";
    } else {
      const alternatives = explanation.placements.length;
      const coverage = "Reference area " + percent.format(placement.geometric_coverage);
      const resolution = "resolution cap " + percent.format(placement.resolution_cap);
      const quality = "overlap quality " + points.format(placement.overlap_score);
      const confidence = "match confidence " + percent.format(explanation.confidence);
      summary.textContent = [coverage, resolution, quality, confidence].join(" · ") +
        (alternatives > 1 ? " · Showing the leading of " + alternatives + " placements" : "") +
        (explanation.alignment_status === "uncertain" ? " · Native placement inferred from limited detail" : "");
    }
    this.root.querySelector("[data-alignment-details]").hidden = !alignment;
    this.root.querySelector("[data-alignment-evidence]").textContent = JSON.stringify(alignment, null, 2) ?? "";
    for (const id of ["controls", "columns", "method", "curve"]) {
      this.root.querySelector("[data-damage-" + id + "]").hidden = !comparison;
    }
    this.root.querySelector("[data-damage-curve]").textContent = comparison
      ? "Weighted damage " + damage(comparison.weighted_damage) +
        " · Overlap quality = 100 × exp(−" + number.format(explanation.damage_scale) + " × weighted damage)"
      : "";
    this.family.replaceChildren(...families.map(([id, info]) => new Option(info.label, id)));
    if (Object.hasOwn(comparison?.families ?? {}, previousFamily)) this.family.value = previousFamily;
    this.root.querySelector("[data-damage-families]").replaceChildren(...families.map(([, info]) => familyRow(info)));
  }

  scoresChanged(revision) {
    this.eventRevision = revision;
    if (this.tile?.state === "pending") this.request(this.tile.key);
  }

  request(key) {
    this.tile?.controller?.abort();
    const controller = new AbortController();
    const tile = { key, state: "loading", controller };
    this.tile = tile;
    const revision = this.eventRevision;
    const { params, benchmark } = this.context;
    const family = this.family.value;
    const side = this.grid.value;
    this.requestTiles({ ...params, family, side }, controller.signal).then((result) => {
      if (this.tile !== tile || controller.signal.aborted) return;
      if (result.status === "pending") {
        tile.state = "pending";
        if (this.eventRevision !== revision) this.request(key);
      } else if (result.status === "ok" && result.artifact_revision === params.rev &&
                 result.reference_id === benchmark.explanation.reference_id &&
                 result.family === family && String(result.grid?.tile_size) === side) {
        tileCache.set(key, result);
        if (tileCache.size > 12) tileCache.delete(tileCache.keys().next().value);
        tile.payload = result;
        tile.state = "ready";
      } else {
        tile.state = result.status === "stale" ? "stale" : "error";
      }
      this.paintOverlay();
    }, () => {
      if (this.tile !== tile || controller.signal.aborted) return;
      tile.state = "error";
      this.paintOverlay();
    });
  }

  paintOverlay() {
    const context = this.context;
    const enabled = this.toggle.checked && Boolean(context?.image);
    this.family.disabled = this.grid.disabled = !enabled;
    const key = enabled ? JSON.stringify([this.key, this.family.value, this.grid.value]) : "";
    if (this.tile?.key !== key) {
      this.tile?.controller?.abort();
      this.tile = null;
      if (key && !tileCache.has(key)) this.request(key);
    }
    const payload = key ? this.tile?.payload ?? tileCache.get(key) : null;
    const text = key && !payload ? tileStatus[this.tile?.state] ?? "" : "";
    if (text !== this.status.textContent) this.status.textContent = text;
    this.status.hidden = !text;
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
      drawGrid(this.canvas, payload);
      this.drawn = payload;
      this.legend.firstElementChild.textContent = this.family.selectedOptions[0].text +
        " · " + this.grid.value + " px tile damage";
    }
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
