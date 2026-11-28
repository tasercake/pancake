import { encodeView, type ViewState } from "./view-state";

type Equal<A, B> = [A] extends [B] ? [B] extends [A] ? true : false : false;
type Expect<T extends true> = T;

type Overlay = Expect<Equal<ViewState["overlay"], "off" | "attention" | "damage">>;
type Tiles = Expect<Equal<ViewState["tiles"], 8 | 32>>;
type Image = Expect<Equal<NonNullable<ViewState["lightbox"]>["image"],
  { kind: "frame"; id: string } | { kind: "output"; id: string }>>;

declare const state: ViewState;
encodeView(state);
// @ts-expect-error Opacity is a number at the typed boundary.
encodeView({ ...state, opacity: "75" });
// @ts-expect-error Ratings are not shareable URL state.
encodeView({ ...state, rating: 5 });
