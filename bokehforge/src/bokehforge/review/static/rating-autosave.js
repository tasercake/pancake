import { WorkspaceDisconnected } from "./workspace-socket.js";

const DRAFTS_KEY = "bokehforge.review.rating-drafts";
const DEBOUNCE = 450;
const same = (a, b) => (a?.grade ?? null) === (b?.grade ?? null) && (a?.comment ?? "") === (b?.comment ?? "");

export class RatingAutosave {
  constructor(socket, onChange) {
    this.socket = socket;
    this.onChange = onChange;
    this.pending = new Map();
    try {
      this.pending = new Map(JSON.parse(sessionStorage.getItem(DRAFTS_KEY) ?? "[]"));
    } catch {
      // In-memory drafts still work when browser storage is unavailable.
    }
  }

  setReady(ready) {
    if (this.ready === ready) return;
    this.ready = ready;
    this.pump();
    this.onChange();
  }

  receive(state) {
    this.state = state;
    this.refresh();
  }

  setScene(run, scene) {
    if (this.group?.run === run && this.group?.scene === scene) return;
    this.group = { run, scene, ratings: {} };
    this.refresh();
  }

  async refresh() {
    const group = this.group;
    const state = this.state;
    if (!group?.run || !state || group.reading ||
        (group.version === state.version && group.revision >= state.revision)) return;
    group.reading = true;
    let loaded = false;
    try {
      const snapshot = await this.socket.request("ratings.read", { run: group.run, scene: group.scene });
      if (this.group !== group) return;
      for (const rating of Object.values(snapshot.ratings)) this.record(group.run, group.scene, rating);
      group.version = snapshot.version;
      group.revision = snapshot.revision;
      loaded = true;
      this.onChange();
    } catch {
      // The workspace snapshot on reconnect triggers another read.
    } finally {
      group.reading = false;
      if (loaded) this.refresh();
    }
  }

  record(run, scene, rating) {
    const group = this.group;
    if (!rating || group?.run !== run || group.scene !== scene) return;
    if (!(group.ratings[rating.stacker]?.updated_at > rating.updated_at)) group.ratings[rating.stacker] = rating;
  }

  rating(stacker, fallback) {
    const current = this.group?.ratings[stacker];
    return current && !(fallback?.updated_at > current.updated_at) ? current : fallback;
  }

  edit(key, data, saved) {
    const draft = this.pending.get(key);
    if (draft && same(draft, data)) return;
    if (!draft?.sent && same(data, saved)) {
      this.pending.delete(key);
    } else {
      this.pending.set(key, {
        ...(draft ?? { expected: saved?.updated_at ?? null }),
        ...data,
        due: Date.now() + DEBOUNCE,
      });
    }
    this.changed();
  }

  retry(key) {
    const draft = this.pending.get(key);
    if (!draft?.error) return;
    if ("conflict" in draft) draft.expected = draft.conflict?.updated_at ?? null;
    delete draft.error;
    delete draft.conflict;
    draft.due = 0;
    this.changed();
  }

  changed() {
    try {
      if (this.pending.size) sessionStorage.setItem(DRAFTS_KEY, JSON.stringify([...this.pending]));
      else sessionStorage.removeItem(DRAFTS_KEY);
    } catch {
      // Keep the unsaved state visible even if the browser cannot retain it.
    }
    this.pump();
    this.onChange();
  }

  pump() {
    clearTimeout(this.timer);
    if (!this.ready || this.writing) return;
    let next, deadline = Infinity;
    for (const [key, draft] of this.pending) {
      const due = draft.sent ? 0 : draft.due;
      if (!draft.error && due < deadline) [next, deadline] = [key, due];
    }
    if (next === undefined) return;
    this.timer = setTimeout(() => this.save(next), Math.max(0, deadline - Date.now()));
  }

  async save(key) {
    const draft = this.pending.get(key);
    const [run, scene, stacker] = JSON.parse(key);
    const sent = draft.sent ?? { grade: draft.grade, comment: draft.comment, expected: draft.expected };
    draft.sent = sent;
    this.writing = true;
    this.changed();
    try {
      const rating = await this.socket.request("ratings.save", { run, scene, stacker, ...sent });
      this.record(run, scene, rating);
      const current = this.pending.get(key);
      if (same(current, rating)) this.pending.delete(key);
      else {
        current.expected = rating.updated_at;
        delete current.sent;
      }
    } catch (error) {
      const current = this.pending.get(key);
      if (!(error instanceof WorkspaceDisconnected)) {
        if (error.data && "conflict" in error.data) {
          current.conflict = error.data.conflict;
          this.record(run, scene, current.conflict);
        }
        current.error = error.message;
        delete current.sent;
      }
    } finally {
      this.writing = false;
      this.changed();
    }
  }
}
