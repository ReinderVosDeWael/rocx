/** Checkout-only prototype. Each array is an owned, mutable JS copy. */
export interface ExperimentalLayoutPage {
  /** Session-local revision, starting at 1. */
  readonly revision: number;
  /** Zero-based page index. */
  readonly index: number;
  /**
   * Rows of 9 words: paragraph index, line index within paragraph, scalar
   * start, scalar end (exclusive), width, first glyph record, glyph count,
   * first caret record, caret count. Glyph/caret record indices are page-local.
   */
  readonly lines: Uint32Array<ArrayBuffer>;
  /**
   * Rows of 5 words: glyph ID, x, y, advance, paragraph-relative scalar
   * cluster. Coordinates are line-relative, in 1/64 pt, with y increasing
   * down from the baseline. Read signed geometry words with `word | 0`
   * (including line width and caret x).
   */
  readonly glyphs: Uint32Array<ArrayBuffer>;
  /** Rows of 2 words: paragraph-relative scalar offset, line-relative x. */
  readonly carets: Uint32Array<ArrayBuffer>;
}

/** Physical dimensions, margins, and uniform paragraph spacing in 1/64 pt. */
export interface ExperimentalLayoutPageSpec {
  readonly width: number;
  readonly height: number;
  readonly left: number;
  readonly right: number;
  readonly top: number;
  readonly bottom: number;
  readonly spaceBefore: number;
  readonly spaceAfter: number;
}

export interface ExperimentalPhysicalLayoutPage extends ExperimentalLayoutPage {
  /**
   * Six words per line, in the same order as `lines`: paragraph index, line
   * index within paragraph, x, top, baseline, bottom. Coordinates are signed
   * 1/64 pt from the page's upper-left corner, with y increasing down.
   * Add x and baseline to the line-relative glyph x and y when painting.
   */
  readonly rows: Int32Array<ArrayBuffer>;
}

/** Inputs extracted from a DOCX in the supported plain-section profile. */
export interface ExperimentalLayoutSection {
  readonly paragraphs: readonly string[];
  readonly spec: ExperimentalLayoutPageSpec;
}

export interface ExperimentalLayoutTextPoint {
  /** Zero-based index into the supplied paragraphs. */
  readonly paragraph: number;
  /** Unicode scalar offset in that paragraph, not a UTF-16 index. */
  readonly offset: number;
}

export interface ExperimentalLayoutHit extends ExperimentalLayoutTextPoint {
  readonly revision: number;
}

/** Which row receives the caret at a shared scalar boundary. */
export type ExperimentalLayoutAffinity = "upstream" | "downstream";

export type ExperimentalLayoutCaretMove = "left" | "right" | "up" | "down" | "lineStart" | "lineEnd";

/** A visual caret belonging to one physical snapshot in the current Wasm instance. */
export interface ExperimentalLayoutVisualPosition {
  readonly point: ExperimentalLayoutTextPoint;
  readonly affinity: ExperimentalLayoutAffinity;
  readonly revision: number;
  readonly snapshotId: number;
  /** Page-local horizontal target in 1/64 pt, retained across vertical moves. */
  readonly preferredX?: number;
}

/** Caret bounds in page-local 1/64 pt. Page and row indices are zero-based. */
export interface ExperimentalLayoutCaretBox {
  readonly page: number;
  readonly row: number;
  readonly x: number;
  readonly top: number;
  readonly bottom: number;
}

/** Selection bounds in page-local 1/64 pt. Page and row indices are zero-based. */
export interface ExperimentalLayoutSelectionRect {
  readonly page: number;
  readonly row: number;
  readonly left: number;
  readonly right: number;
  readonly top: number;
  readonly bottom: number;
}

export interface ExperimentalLayoutReuse {
  readonly shapedHits: number;
  readonly lineHits: number;
  readonly pageHits: number;
}

/** Paragraph-cache accounting only; excludes fonts, snapshots and page checkpoints. */
export interface ExperimentalLayoutCacheStats {
  readonly paragraphs: number;
  readonly wrapVariants: number;
  readonly estimatedBytes: number;
  readonly budgetBytes: number;
}
