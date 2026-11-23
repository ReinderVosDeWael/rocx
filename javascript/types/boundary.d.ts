export type Condition = boolean | (() => boolean);
export type Int64 = number | bigint;
export type Rgb = readonly [number, number, number];
export interface ResolveOptions {
  readonly signal?: AbortSignal;
}
export type OutputProfile = "transitional" | "strict";
export interface ReadLimits {
  readonly maxParts?: number;
  readonly maxPartBytes?: number;
  readonly maxExpandedBytes?: number;
}
export interface RenderOptions {
  readonly profile?: OutputProfile;
  /** UTC ISO timestamp override for this output, not the snapshot's date. */
  readonly timestamp?: string;
  readonly readLimits?: ReadLimits;
}

declare const engineHandle: unique symbol;
export interface ResolvedHandle<K extends string, View> {
  readonly [engineHandle]: K;
  readonly view: Readonly<View>;
}
