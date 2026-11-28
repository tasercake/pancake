// Keep the visible range and a small buffer in the DOM, even for long stacks
// and direct jumps into the middle of the catalog.
export class VirtualRail {
  constructor(view, axis, stride, render, { itemPadding = 0 } = {}) {
    this.view = view;
    this.track = view.firstElementChild;
    this.axis = axis;
    this.stride = stride;
    this.itemPadding = itemPadding;
    this.render = render;
    this.offset = axis === "x" ? "scrollLeft" : "scrollTop";
    this.viewportSize = axis === "x" ? "clientWidth" : "clientHeight";
    this.items = [];
    this.nodes = new Map();
    this.selected = -1;
    this.raf = 0;
    if (itemPadding) view.style.setProperty("--rail-item-padding", `${itemPadding}px`);
    view.addEventListener("scroll", () => this.schedule(), { passive: true });
    new ResizeObserver(() => this.schedule()).observe(view);
    view.addEventListener("focusout", () => this.schedule());
  }

  setItems(items) {
    this.items = items;
    this.selected = -1;
    for (const node of this.nodes.values()) this.remove(node);
    this.nodes.clear();
    this.track.style[this.axis === "x" ? "width" : "height"] = `${this.itemStart(items.length)}px`;
    this.view[this.offset] = 0;
    this.schedule();
  }

  itemStart(index) {
    if (index === 0) return 0;
    // The first and last tiles each get one extra margin at the strip edge.
    const extra = index === this.items.length ? 2 * this.itemPadding : this.itemPadding;
    return index * this.stride + extra;
  }

  select(index) {
    this.selected = index;
    const offset = this.view[this.offset];
    const viewport = this.view[this.viewportSize];
    const start = this.itemStart(index);
    const end = this.itemStart(index + 1);
    if (start < offset) this.view[this.offset] = start;
    else if (end > offset + viewport) this.view[this.offset] = end - viewport;
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
    const offset = this.view[this.offset] - this.itemPadding;
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
        const start = this.itemStart(index);
        if (this.axis === "x") {
          node.style.left = `${start}px`;
          node.style.width = `${this.itemStart(index + 1) - start}px`;
          node.classList.toggle("rail-start", index === 0);
          node.classList.toggle("rail-end", index === this.items.length - 1);
        } else {
          node.style.top = `${start}px`;
        }
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
