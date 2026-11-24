import type { Condition, ResolveOptions } from "./boundary.js";
import type { DrawingResolvedInlineInput, DrawingValidationInput } from "./drawings.js";
import type { ResolvedComment, StructuredResolvedInlineArms, TextRevision } from "./structured.js";
import type { ResolvedDocument, ResolvedInline, ResolvedParagraph, RunFormat } from "./text-layout.js";

declare const valueType: unique symbol;
declare const propertyId: unique symbol;
export type PropertyId = { readonly [propertyId]: true };

/** A single-use resolution stage. Use a Deferred or Property to share its outcome. */
export interface Value<T> {
  readonly [valueType]: readonly ["value", T];
  map<U>(transform: (value: T, context: ProducerContext) => SyncV<U>): Value<U>;
}
export interface Deferred<T> {
  readonly [valueType]: readonly ["deferred", T];
}
export interface Property<T> {
  readonly [valueType]: readonly ["property", T];
  reference(): PropertyRef<T>;
}
export interface PropertyRef<T> {
  readonly [valueType]: readonly ["ref", T];
  readonly id: PropertyId;
}
export type V<T> = T | Value<T> | Deferred<T> | Property<T> | PropertyRef<T>;
// Only future assimilates thenables returned directly from its producer.
type SyncV<T> = (T & (T extends PromiseLike<unknown> ? never : unknown)) |
  Value<T> | Deferred<T> | Property<T> | PropertyRef<T>;
export interface ProducerContext {
  /** Aborts owned producer work. Cancelling a waiter does not abort shared work. */
  readonly signal: AbortSignal;
  get<T>(value: PropertyRef<T> | Deferred<T>): Promise<T>;
}
export function ready<T>(value: T): Value<T>;
export function lazy<T>(producer: (context: ProducerContext) => SyncV<T>): Value<T>;
export function future<T>(producer: (context: ProducerContext) => V<T> | PromiseLike<V<T>>): Value<T>;
export function deferred<T>(value: V<T>): Deferred<T>;
export function property<T>(name: string, value: V<T>, options?: { readonly condition?: Condition }): Property<T>;
export function resolveValue<T>(value: V<T>, options?: ResolveOptions): Promise<T>;

type TextInlineInput = {
  readonly kind: "text";
  readonly text: string;
  readonly format?: RunFormat;
  readonly comment?: ResolvedComment;
  readonly revision?: TextRevision;
};
type StructuredInlineInput = {
  [K in keyof StructuredResolvedInlineArms]: { readonly kind: K } & StructuredResolvedInlineArms[K];
}[keyof StructuredResolvedInlineArms];
export type CreateResolvedInlineInput = TextInlineInput | DrawingResolvedInlineInput | StructuredInlineInput;
export type ResolvedInlineView = ResolvedInline["view"];

/** Rust's display projection, not a concatenation of the raw text fields. */
export function inlineText(handles: readonly ResolvedInline[]): Promise<string>;
/** Construct a Rust enum arm from ready data; contextual checks are separate. */
export function createResolvedInline<K extends CreateResolvedInlineInput["kind"]>(
  input: { readonly kind: K } & Extract<CreateResolvedInlineInput, { readonly kind: K }>,
): Promise<Extract<ResolvedInline, { readonly view: { readonly kind: K } }>>;

export interface ValidationValues {
  readonly resolvedInline: ResolvedInline;
  readonly resolvedParagraph: ResolvedParagraph;
  readonly resolvedDocument: ResolvedDocument;
}
export type ValidateValueInput = {
  [K in keyof ValidationValues]: { readonly kind: K; readonly value: ValidationValues[K] };
}[keyof ValidationValues] | DrawingValidationInput;
export function validateValue(input: ValidateValueInput): Promise<void>;

export type AuthoringIdKind = "bookmarkName" | "rangeId" | "styleId";
export function validateAuthoringId(kind: AuthoringIdKind, value: string): Promise<void>;
