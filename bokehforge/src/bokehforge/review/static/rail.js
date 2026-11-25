// Keep the visible range and a small buffer in the DOM, even for long stacks
// and direct jumps into the middle of the catalog.
export class VirtualRail {
  constructor(view, axis, stride, render) {
    this.view = view;
    this.track = view.firstElementChild;
    this.axis = axis;
    this.stride = stride;
    this.render = render;
    this.offset = axis === "x" ? "scrollLeft" : "scrollTop";
    this.viewportSize = axis === "x" ? "clientWidth" : "clientHeight";
    this.items = [];
    this.nodes = new Map();
    this.selected = -1;
    this.raf = 0;
    view.addEventListener("scroll", () => this.schedule(), { passive: true });
    new ResizeObserver(() => this.schedule()).observe(view);
    view.addEventListener("focusout", () => this.schedule());
  }

  setItems(items) {
    this.items = items;
    this.selected = -1;
    for (const node of this.nodes.values()) this.remove(node);
    this.nodes.clear();
    this.track.style[this.axis === "x" ? "width" : "height"] = `${items.length * this.stride}px`;
    this.view[this.offset] = 0;
    this.schedule();
  }

  select(index) {
    this.selected = index;
    const offset = this.view[this.offset];
    const viewport = this.view[this.viewportSize];
    const start = index * this.stride;
    if (start < offset) this.view[this.offset] = start;
    else if (start + this.stride > offset + viewport) this.view[this.offset] = start + this.stride - viewport;
    this.schedule();
  }

  schedule() {
    if (!this.raf) this.raf = requestAnimationFrame(() => {
      this.raf = 0;
      this.update();
    });
  }

  remove(node) {
    for (const image of node.querySelectorAll("img")) {
      if (!image.complete) image.removeAttribute("src");
    }
    node.remove();
  }

  update() {
    const offset = this.view[this.offset];
    const viewport = this.view[this.viewportSize];
    if (!viewport) return;
    const start = Math.max(0, Math.floor(offset / this.stride) - 2);
    const end = Math.min(this.items.length, Math.ceil((offset + viewport) / this.stride) + 2);
    const indices = Array.from({ length: end - start }, (_, index) => start + index);
    for (const [index, node] of this.nodes) {
      if (index >= start && index < end) continue;
      if (node.contains(document.activeElement)) {
        indices.push(index);
      } else {
        this.remove(node);
        this.nodes.delete(index);
      }
    }
    indices.sort((a, b) => a - b);
    let previous = null;
    for (const index of indices) {
      let node = this.nodes.get(index);
      if (!node) {
        node = this.render(this.items[index], index);
        node.style[this.axis === "x" ? "left" : "top"] = `${index * this.stride}px`;
        node.setAttribute("aria-posinset", index + 1);
        node.setAttribute("aria-setsize", this.items.length);
        this.nodes.set(index, node);
      }
      const attribute = this.axis === "x" ? "aria-pressed" : "aria-current";
      node.firstElementChild.setAttribute(attribute, index === this.selected);
      const next = previous ? previous.nextElementSibling : this.track.firstElementChild;
      if (node !== next) this.track.insertBefore(node, next);
      previous = node;
    }
  }
}
