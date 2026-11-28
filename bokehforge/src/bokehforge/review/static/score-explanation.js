const number = new Intl.NumberFormat(undefined, { maximumFractionDigits: 4 });
const points = new Intl.NumberFormat(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
const percent = new Intl.NumberFormat(undefined, { style: "percent", maximumFractionDigits: 1 });

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

export function scoreExplanationKey(context) {
  const { params, benchmark } = context ?? {};
  const explanation = benchmark?.explanation;
  return explanation && params?.rev && params.rev === benchmark.artifact_revision
    ? JSON.stringify([params.run, params.scene, params.stacker, params.rev, explanation.reference_id,
      benchmark.profile, benchmark.inspection_map_sha256]) : "";
}

export class ScoreExplanation {
  constructor(root, trigger) {
    this.root = root;
    this.trigger = trigger;
    root.append(document.querySelector("#score-explanation-template").content.cloneNode(true));
    this.key = "";
    this.open = !trigger;
    const dismiss = root.querySelector(".score-explanation-dismiss");
    if (trigger) {
      trigger.onclick = () => this.setOpen(!this.open);
      dismiss.onclick = () => {
        this.setOpen(false);
        trigger.focus();
      };
    } else {
      dismiss.remove();
    }
  }

  setOpen(open) {
    this.open = open;
    this.root.hidden = !(open && this.key);
    if (!this.trigger) return;
    this.trigger.setAttribute("aria-expanded", String(!this.root.hidden));
    this.trigger.textContent = open && this.key ? "Hide explanation" : "Explain score";
  }

  close() {
    this.setOpen(false);
    this.update(null);
  }

  update(context) {
    const key = scoreExplanationKey(context);
    if (key === this.key) return;
    this.key = key;
    if (this.trigger) this.trigger.hidden = !key;
    this.setOpen(this.open);
    if (key) {
      const { explanation, alignment } = context.benchmark;
      this.showSummary(explanation, explanation.placements[0], alignment);
    }
  }

  showSummary(explanation, placement, alignment) {
    const comparison = placement?.comparison;
    const families = Object.values(comparison?.families ?? {});
    if (this.trigger) {
      this.root.querySelector("[data-score-heading]").textContent =
        "Benchmark score " + points.format(explanation.score) + " / 100";
    }
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
    for (const id of ["columns", "method", "curve"]) {
      this.root.querySelector("[data-damage-" + id + "]").hidden = !comparison;
    }
    this.root.querySelector("[data-damage-curve]").textContent = comparison
      ? "Weighted damage " + damage(comparison.weighted_damage) +
        " · Overlap quality = 100 × exp(−" + number.format(explanation.damage_scale) + " × weighted damage)" +
        (comparison.inspection_mix == null ? "" : " · Inspection mix " + percent.format(comparison.inspection_mix))
      : "";
    this.root.querySelector("[data-damage-families]").replaceChildren(...families.map(familyRow));
  }

}
