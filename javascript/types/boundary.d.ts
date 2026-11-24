export type Condition = boolean | (() => boolean);
export type Int64 = number | bigint;
export type Rgb = readonly [number, number, number];
export interface ResolveOptions {
  readonly signal?: AbortSignal;
}
export type OutputProfile = "transitional" | "strict";
export interface ReadLimits {
  /** Non-negative safe integer within the Wasm `usize` range. */
  readonly maxParts?: number;
  /** Non-negative safe integer or a bigint within the `u64` range. */
  readonly maxPartBytes?: number | bigint;
  /** Non-negative safe integer or a bigint within the `u64` range. */
  readonly maxExpandedBytes?: number | bigint;
}
export interface RenderOptions {
  readonly profile?: OutputProfile;
  /** Core-properties date text, not parsed or normalized; overrides this output only. */
  readonly timestamp?: string;
  readonly readLimits?: ReadLimits;
}

declare const engineHandle: unique symbol;
export interface ResolvedHandle<K extends string, View> {
  readonly [engineHandle]: K;
  readonly view: Readonly<View>;
}
