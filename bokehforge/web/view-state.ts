import {
  decodeQueryParams, encodeQueryParams, objectToSearchString, searchStringToObject,
  type DecodedValueMap, type QueryParamConfig,
} from "serialize-query-params";
import * as v from "valibot";

const id = v.pipe(v.string(), v.regex(/^[^\u0000-\u001f\u007f]{1,512}$/u), v.check(value => Boolean(value.trim())));
const mode = v.picklist(["off", "attention", "damage"]);
const family = v.picklist(["all", "tone", "color", "lost_detail", "added_structure", "boundary", "range"]);
const tileSize = v.picklist([8, 32]);
const opacity = v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(100));
const unit = v.pipe(v.number(), v.minValue(0), v.maxValue(1));
const details = v.pipe(v.array(v.pipe(v.string(), v.regex(/^(?:alignment|damage-method|family:[a-z][a-z0-9_-]{0,39})$/))), v.maxLength(32));
const image = v.variant("kind", [
  v.object({ kind: v.literal("frame"), id }),
  v.object({ kind: v.literal("output"), id }),
]);
const lightbox = v.object({
  scene: id,
  run: v.nullable(id),
  reference: v.nullable(id),
  image,
  zoom: v.pipe(v.number(), v.minValue(1), v.maxValue(12)),
  pixels: v.boolean(),
  u: unit,
  v: unit,
  swipe: v.boolean(),
  split: unit,
  explain: v.boolean(),
  details,
  benchmark: v.nullable(id),
});
const optionName = v.pipe(v.string(), v.regex(/^[a-z][a-z0-9_-]{0,47}$/));
const tool = v.object({
  id,
  selected: v.boolean(),
  expanded: v.boolean(),
  settings: v.record(optionName, v.union([v.pipe(v.string(), v.maxLength(128)), v.boolean()])),
});
const evaluation = v.object({
  scope: v.picklist(["scene", "dataset", "all"]),
  tools: v.pipe(v.array(tool), v.maxLength(24)),
});

function json(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    return undefined;
  }
}

function decimal(text: string): number {
  return /^\d+$/.test(text) ? Number(text) : NaN;
}

function flag(text: string): boolean | undefined {
  if (text === "1") return true;
  if (text === "0") return false;
  return undefined;
}

function param<T>(
  schema: v.GenericSchema<unknown, T>,
  fallback: T,
  parse: (text: string) => unknown = text => text,
  format: (value: T) => string = value => String(value),
): QueryParamConfig<T> {
  return {
    decode(encoded) {
      if (typeof encoded !== "string") return fallback;
      const result = v.safeParse(schema, parse(encoded));
      return result.success ? result.output : fallback;
    },
    encode(value) {
      const result = v.safeParse(schema, value);
      if (!result.success || JSON.stringify(result.output) === JSON.stringify(fallback)) return undefined;
      return format(result.output);
    },
  };
}

const params = {
  dataset: param(v.nullable(id), null),
  scene: param(v.nullable(id), null),
  run: param(v.nullable(id), null),
  stacker: param(v.nullable(id), null),
  source: param(v.nullable(id), null),
  reference: param(v.nullable(id), null),
  overlay: param(mode, "off"),
  family: param(family, "all"),
  tiles: param(tileSize, 8, decimal),
  opacity: param(opacity, 100, decimal),
  accent: param(v.nullable(v.pipe(v.string(), v.regex(/^#[a-f0-9]{6}$/i))), null),
  scenes: param(v.boolean(), false, flag, value => value ? "1" : "0"),
  details: param(details, [], json, JSON.stringify),
  picker: param(v.nullable(v.picklist(["output", "lightbox"])), null),
  appearance: param(v.boolean(), false, flag, value => value ? "1" : "0"),
  evaluation: param(v.nullable(evaluation), null, json, JSON.stringify),
  lightbox: param(v.nullable(lightbox), null, json, JSON.stringify),
};

export type ViewState = DecodedValueMap<typeof params>;
export type HistoryMode = "push" | "replace" | "continuous";
type Address = Pick<Location, "search" | "hash">;

function known<T>(values: Record<string, T>): Record<string, T> {
  return Object.fromEntries(Object.entries(values).filter(([key]) => Object.hasOwn(params, key)));
}

export function readView({ search, hash }: Address): ViewState {
  // Existing Stackroom links used a fragment with dataset, scene, and run.
  const legacy = searchStringToObject(hash.slice(1));
  return decodeQueryParams(params, known({ ...legacy, ...searchStringToObject(search) }));
}

export function encodeView(state: ViewState): string {
  return objectToSearchString(encodeQueryParams(params, known(state)));
}

export class ViewHistory {
  #page: Window;
  #timer: number | undefined;
  #pending: string | undefined;
  #lastWrite = -Infinity;

  constructor(page: Window) {
    this.#page = page;
  }

  read(): ViewState {
    this.#cancel();
    return readView(this.#page.location);
  }

  write(state: ViewState, mode: HistoryMode = "replace"): void {
    const query = encodeView(state);
    const target = this.#page.location.pathname + (query ? "?" + query : "");
    if (mode === "push") {
      this.flush();
      this.#write(target, "push");
    } else if (mode === "replace") {
      this.#cancel();
      this.#write(target, "replace");
    } else {
      this.#pending = target;
      if (this.#timer !== undefined) return;
      // Range and pan gestures can fire more often than browsers permit history writes.
      const wait = this.#lastWrite + 400 - this.#page.performance.now();
      if (wait <= 0) this.flush();
      else this.#timer = this.#page.setTimeout(() => this.flush(), wait);
    }
  }

  flush(): void {
    const target = this.#pending;
    this.#cancel();
    if (target !== undefined) this.#write(target, "replace");
  }

  #cancel(): void {
    if (this.#timer !== undefined) this.#page.clearTimeout(this.#timer);
    this.#timer = this.#pending = undefined;
  }

  #write(target: string, mode: "push" | "replace"): void {
    const { pathname, search, hash } = this.#page.location;
    if (target === pathname + search + hash) return;
    this.#page.history[mode === "push" ? "pushState" : "replaceState"](null, "", target);
    this.#lastWrite = this.#page.performance.now();
  }
}

export function readEvaluation(form: HTMLFormElement): ViewState["evaluation"] {
  const scope = form.querySelector<HTMLSelectElement>("#scope")?.value;
  const tools = Array.from(form.querySelectorAll(".tool-option"), card => {
    const checkbox = card.querySelector<HTMLInputElement>('input[name="stacker"]');
    const settings: Record<string, string | boolean> = {};
    for (const control of card.querySelectorAll<HTMLInputElement | HTMLSelectElement>("[data-setting]")) {
      const name = control.dataset.setting;
      if (!name) continue;
      settings[name] = control instanceof HTMLInputElement && control.type === "checkbox" ? control.checked : control.value;
    }
    return {
      id: checkbox?.value,
      selected: checkbox?.checked,
      expanded: card.querySelector("details")?.open,
      settings,
    };
  });
  const result = v.safeParse(evaluation, { scope, tools });
  return result.success ? result.output : null;
}

export function restoreEvaluation(form: HTMLFormElement, draft: NonNullable<ViewState["evaluation"]>): void {
  const scope = form.querySelector<HTMLSelectElement>("#scope");
  if (scope && Array.from(scope.options).some(option => option.value === draft.scope)) {
    scope.value = draft.scope;
    scope.dispatchEvent(new Event("change"));
  }
  const tools = new Map(draft.tools.map(tool => [tool.id, tool]));
  for (const card of form.querySelectorAll(".tool-option")) {
    const checkbox = card.querySelector<HTMLInputElement>('input[name="stacker"]');
    if (!checkbox) continue;
    const saved = tools.get(checkbox.value);
    if (!saved) continue;
    checkbox.checked = saved.selected;
    const panel = card.querySelector("details");
    if (panel) panel.open = saved.expanded;
    for (const control of card.querySelectorAll<HTMLInputElement | HTMLSelectElement>("[data-setting]")) {
      const value = saved.settings[control.dataset.setting ?? ""];
      if (control instanceof HTMLSelectElement) {
        if (typeof value === "string" && Array.from(control.options).some(option => option.value === value)) {
          control.value = value;
        }
      } else if (control.type === "checkbox") {
        if (typeof value === "boolean") control.checked = value;
      } else if (typeof value === "string") {
        control.value = value;
      }
    }
    panel?.dispatchEvent(new Event("change"));
  }
}
