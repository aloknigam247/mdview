import { JSDOM } from "jsdom";

let cached: JSDOM | null = null;

function installSvgShims(win: Record<string, unknown>): void {
  const proto = (win.SVGElement as { prototype: Record<string, unknown> } | undefined)?.prototype;
  if (!proto) return;

  const approxTextWidth = (text: string, fontSize: number): number => {
    return Math.max(1, text.length * fontSize * 0.6);
  };

  const getTextContent = (node: unknown): string => {
    const n = node as { textContent?: string | null };
    return (n.textContent ?? "").toString();
  };

  const getFontSize = (node: unknown): number => {
    const n = node as { getAttribute?: (k: string) => string | null };
    const raw = n.getAttribute?.("font-size") ?? null;
    const parsed = raw ? parseFloat(raw) : NaN;
    return Number.isFinite(parsed) && parsed > 0 ? parsed : 12;
  };

  proto.getBBox = function (): { x: number; y: number; width: number; height: number } {
    const self = this as { tagName?: string };
    const tag = (self.tagName ?? "").toLowerCase();
    const txt = getTextContent(this);
    const fs = getFontSize(this);
    if (tag === "text" || tag === "tspan") {
      return { x: 0, y: 0, width: approxTextWidth(txt, fs), height: fs * 1.2 };
    }
    return { x: 0, y: 0, width: 10, height: 10 };
  };

  proto.getComputedTextLength = function (): number {
    const txt = getTextContent(this);
    const fs = getFontSize(this);
    return approxTextWidth(txt, fs);
  };

  proto.getCTM = function () {
    return { a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 };
  };

  proto.getScreenCTM = function () {
    return {
      a: 1, b: 0, c: 0, d: 1, e: 0, f: 0,
      inverse() { return this; },
      multiply(_: unknown) { return this; },
    };
  };

  proto.createSVGMatrix = function () {
    return { a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 };
  };

  proto.createSVGPoint = function () {
    return {
      x: 0,
      y: 0,
      matrixTransform() { return { x: this.x, y: this.y }; },
    };
  };
}

function installCanvasShim(win: Record<string, unknown>): void {
  const proto = (win.HTMLCanvasElement as { prototype: Record<string, unknown> } | undefined)?.prototype;
  if (!proto) return;
  const ctx2d = {
    canvas: null as unknown,
    fillStyle: "#000",
    strokeStyle: "#000",
    lineWidth: 1,
    font: "10px sans-serif",
    textAlign: "start",
    textBaseline: "alphabetic",
    globalAlpha: 1,
    fillRect: () => {},
    strokeRect: () => {},
    clearRect: () => {},
    beginPath: () => {},
    closePath: () => {},
    moveTo: () => {},
    lineTo: () => {},
    arc: () => {},
    arcTo: () => {},
    bezierCurveTo: () => {},
    quadraticCurveTo: () => {},
    rect: () => {},
    fill: () => {},
    stroke: () => {},
    clip: () => {},
    save: () => {},
    restore: () => {},
    translate: () => {},
    scale: () => {},
    rotate: () => {},
    transform: () => {},
    setTransform: () => {},
    resetTransform: () => {},
    drawImage: () => {},
    createLinearGradient: () => ({ addColorStop: () => {} }),
    createRadialGradient: () => ({ addColorStop: () => {} }),
    createPattern: () => ({}),
    fillText: () => {},
    strokeText: () => {},
    measureText: (s: string) => ({
      width: Math.max(1, (s?.length ?? 0) * 6),
      actualBoundingBoxAscent: 8,
      actualBoundingBoxDescent: 2,
    }),
    getImageData: () => ({ data: new Uint8ClampedArray(4), width: 1, height: 1 }),
    putImageData: () => {},
    createImageData: (w: number, h: number) => ({
      data: new Uint8ClampedArray(Math.max(1, w) * Math.max(1, h) * 4),
      width: w,
      height: h,
    }),
    setLineDash: () => {},
    getLineDash: () => [],
    isPointInPath: () => false,
    isPointInStroke: () => false,
  };
  proto.getContext = function (kind: string) {
    if (kind === "2d") return ctx2d;
    return null;
  };
  proto.toDataURL = function () { return "data:image/png;base64,"; };
  proto.toBlob = function (cb: (b: unknown) => void) { cb(null); };
}

export function getDom(): JSDOM {
  if (cached) return cached;
  cached = new JSDOM(
    "<!DOCTYPE html><html><head></head><body></body></html>",
    { pretendToBeVisual: true, url: "http://localhost/" },
  );
  const g = globalThis as Record<string, unknown>;
  const win = cached.window as unknown as Record<string, unknown>;
  installSvgShims(win);
  installCanvasShim(win);
  g.window = win;
  g.document = win.document;
  g.navigator = win.navigator;
  g.HTMLElement = win.HTMLElement;
  g.HTMLCanvasElement = win.HTMLCanvasElement;
  g.Element = win.Element;
  g.Node = win.Node;
  g.DOMParser = win.DOMParser;
  g.XMLSerializer = win.XMLSerializer;
  g.SVGElement = win.SVGElement;
  g.getComputedStyle = win.getComputedStyle;
  const w = globalThis as Record<string, unknown>;
  if (!w.requestAnimationFrame) {
    w.requestAnimationFrame = (cb: (t: number) => void) => {
      return setTimeout(() => cb(Date.now()), 0) as unknown as number;
    };
    w.cancelAnimationFrame = (id: number) => clearTimeout(id as unknown as NodeJS.Timeout);
  }
  const ensureObjUrl = (obj: unknown) => {
    const U = obj as { createObjectURL?: unknown; revokeObjectURL?: unknown } | undefined;
    if (!U) return;
    if (typeof U.createObjectURL !== "function") U.createObjectURL = () => "blob:mdview";
    if (typeof U.revokeObjectURL !== "function") U.revokeObjectURL = () => {};
  };
  ensureObjUrl(w.URL);
  ensureObjUrl((win as { URL?: unknown }).URL);
  return cached;
}
