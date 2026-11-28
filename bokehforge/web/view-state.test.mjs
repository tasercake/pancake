import assert from "node:assert/strict";
import test from "node:test";
import { encodeView, readView, ViewHistory } from "../src/bokehforge/review/static/view-state.js";

const defaults = readView({ search: "", hash: "" });

function roundTrip(state) {
  return readView({ search: encodeView(state), hash: "" });
}

test("round-trips the comparison, lightbox and evaluation drafts", () => {
  const state = {
    dataset: "test dataset", scene: "demo/leaf-1", source: "demo/leaf-1/frame-a",
    reference: "demo/leaf-1/ref-b", run: "run:1", stacker: "pancake",
    overlay: "damage", family: "tone", tiles: 32, opacity: 0, accent: "#0088cc",
    scenes: false, appearance: false, details: ["alignment", "family:tone"], picker: "lightbox",
    evaluation: null,
    lightbox: {
      scene: "demo/leaf-2", run: "run:2", reference: "demo/leaf-2/ref-a",
      image: { kind: "output", id: "openfocus" }, zoom: 2.25, pixels: false,
      u: 0.3, v: 0.7, swipe: true, split: 0.22, explain: true,
      details: ["damage-method"], benchmark: "focus-stack",
    },
  };
  assert.deepEqual(roundTrip(state), state);

  const draft = { ...defaults, scene: "demo/leaf-1", evaluation: {
    scope: "dataset", tools: [{
      id: "pancake", selected: false, expanded: true,
      settings: { fusion: "hybrid", device: "cpu", align: false, threshold: "1.25" },
    }],
  } };
  assert.deepEqual(roundTrip(draft), draft);
});

test("applies defaults to malformed and duplicate values and discards unrelated fields", () => {
  const search = "?overlay=legacy&tiles=4&opacity=31no&accent=%23bad&family=tone&family=range" +
    "&lightbox=%7B%22zoom%22%3A0%7D&evaluation=null&rating=5&comment=private&__proto__=bad";
  assert.deepEqual(readView({ search, hash: "" }), defaults);
  assert.equal(encodeView({ ...defaults, rating: 5, comment: "private", opacity: "90" }), "");
  assert.equal(roundTrip({ ...defaults, opacity: 100 }).opacity, 100);
  assert.equal(roundTrip({ ...defaults, opacity: 0 }).opacity, 0);
});

test("legacy fragment links work and query parameters take precedence", () => {
  const state = readView({
    search: "?scene=new%2Fleaf&overlay=attention",
    hash: "#dataset=demo&scene=old%2Fleaf&run=evaluation",
  });
  assert.equal(state.scene, "new/leaf");
  assert.equal(state.dataset, "demo");
  assert.equal(state.run, "evaluation");
  assert.equal(state.overlay, "attention");
});

test("history coalesces range gestures, flushes before pushes, and cancels on back", () => {
  let now = 0;
  let nextTimer = 0;
  const timers = new Map();
  const writes = [];
  const page = {
    location: new URL("http://localhost/"),
    performance: { now: () => now },
    setTimeout(fn) { timers.set(++nextTimer, fn); return nextTimer; },
    clearTimeout(id) { timers.delete(id); },
    history: {
      pushState(_state, _title, path) { writes.push(["push", path]); page.location = new URL(path, page.location); },
      replaceState(_state, _title, path) { writes.push(["replace", path]); page.location = new URL(path, page.location); },
    },
  };
  const view = new ViewHistory(page);
  view.write({ ...defaults, scene: "one" }, "push");
  now += 40;
  view.write({ ...defaults, scene: "one", opacity: 20 }, "continuous");
  view.write({ ...defaults, scene: "one", opacity: 30 }, "continuous");
  assert.equal(timers.size, 1);
  view.write({ ...defaults, scene: "two", opacity: 30 }, "push");
  assert.deepEqual(writes, [
    ["push", "/?scene=one"], ["replace", "/?scene=one&opacity=30"], ["push", "/?scene=two&opacity=30"],
  ]);
  view.write({ ...defaults, scene: "two", opacity: 40 }, "continuous");
  assert.equal(timers.size, 1);
  page.location = new URL("http://localhost/?scene=one&opacity=30");
  assert.equal(view.read().opacity, 30);
  assert.equal(timers.size, 0);
});
