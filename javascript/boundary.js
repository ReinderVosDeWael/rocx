import {
  configure_host, dispatch, EngineHandle, make_shared, Operation, render_document,
  request_dependency, resolve_component, resolve_input, resolve_program, settle_producer,
} from "./pkg/rocx.js";

const tokens = new WeakMap();
const handles = new WeakMap();
const wrappers = new WeakMap();
const templateFiles = new WeakMap();
const calls = new Map();

// Internal to the Node subpath; neither paths nor loaders enter Wasm.
export function registerTemplateFile(source, load) {
  templateFiles.set(source, load);
}

export function asError(cause) {
  // Keep existing Error identity (including cross-realm errors).
  try {
    if (cause instanceof Error || Error.isError?.(cause)) return cause;
  } catch {
    // A thrown Proxy may trap instanceof; the original value is still a cause.
  }
  return new Error("rocx operation failed", { cause });
}

export function validateTimestamp(timestamp) {
  if (typeof timestamp !== "string" ||
      !/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d{3})?Z$/.test(timestamp)) {
    throw new TypeError("timestamp must be a UTC ISO timestamp");
  }
  const canonical = timestamp.length === 20 ? timestamp.replace("Z", ".000Z") : timestamp;
  const date = new Date(timestamp);
  if (Number.isNaN(date.valueOf()) || date.toISOString() !== canonical || timestamp.startsWith("0000")) {
    throw new TypeError("timestamp is not a valid UTC date and time");
  }
}

function valueToken(kind, payload, transform) {
  const token = Object.freeze({
    map(callback) {
      return valueToken("map", token, requireFunction(callback));
    },
  });
  tokens.set(token, { kind, payload, transform, used: false });
  return token;
}

function requireFunction(callback) {
  if (typeof callback !== "function") throw new TypeError("producer must be a function");
  return callback;
}

export const ready = (value) => valueToken("ready", value);
export const lazy = (producer) => valueToken("lazy", requireFunction(producer));
export const future = (producer) => valueToken("future", requireFunction(producer));

export function deferred(value) {
  const token = Object.freeze({});
  tokens.set(token, { kind: "deferred", shared: { value } });
  return token;
}

export function property(name, value, options) {
  const { condition } = readOptions(options, ["condition"]);
  if (typeof name !== "string" || !name.isWellFormed()) {
    throw new TypeError("property name must be a well-formed string");
  }
  if (condition !== undefined && typeof condition !== "boolean" && typeof condition !== "function") {
    throw new TypeError("condition must be a boolean or function");
  }
  const shared = { name, value, condition };
  const reference = Object.freeze({ id: Object.freeze({}) });
  const token = Object.freeze({ reference: () => reference });
  tokens.set(reference, { kind: "ref", shared });
  tokens.set(token, { kind: "property", shared });
  return token;
}

const host = {
  handleId(value) {
    return handles.get(value)?.id ?? 0;
  },
  inspect(value, consume) {
    const token = tokens.get(value);
    if (!token) return [];
    if (token.shared) {
      const shared = token.shared;
      if (!shared.raw) {
        shared.raw = make_shared(shared.value, shared.name, shared.condition);
        shared.value = undefined;
        shared.condition = undefined;
      }
      return [token.kind, shared.raw.id];
    }
    if (!consume) return [token.kind];
    if (token.used) throw new TypeError("Value tokens are single-use");
    token.used = true;
    const { kind, payload, transform } = token;
    token.payload = token.transform = undefined;
    return [kind, payload, transform];
  },
  start(id, producer, input, mode) {
    const controller = new AbortController();
    const errors = new WeakMap();
    const pending = new Set();
    calls.set(id, { controller, pending });
    const context = Object.freeze({
      signal: controller.signal,
      get(target) {
        const promise = new Promise((resolve, reject) => {
          const accepted = request_dependency(id, target, (ok, value, dependency) => {
            pending.delete(reject);
            if (ok) resolve(value);
            else {
              errors.set(value, dependency);
              reject(value);
            }
          });
          if (accepted) pending.add(reject);
          else reject(controller.signal.reason ?? new Error("Producer is no longer active"));
        });
        // An unawaited dependency can be dropped when its producer settles.
        // Awaiters still see the rejection; the orphan cannot hit the host.
        promise.catch(() => {});
        return promise;
      },
    });
    const reject = (cause) => settle_producer(id, false, cause, errors.get(cause) ?? 0);
    const resolve = (value) => settle_producer(id, true, value, 0);
    try {
      const result = mode === 2 ? producer(input, context) : producer(context);
      if (mode !== 0 && result != null && typeof result.then === "function") {
        // Silence a later rejection from the invalid return then reject now.
        Promise.resolve(result).catch(() => {});
        throw new TypeError("lazy and map callbacks must not return a Promise; use future");
      }
      if (mode !== 0) resolve(result);
      else Promise.resolve(result).then(resolve, reject);
    } catch (cause) {
      reject(cause);
    }
  },
  finish(id, cancelled) {
    const call = calls.get(id);
    calls.delete(id);
    if (!call) return;
    const reason = new Error(cancelled ? "Producer was aborted" : "Producer has settled");
    if (cancelled) call.controller.abort(reason);
    for (const reject of call.pending) reject(reason);
    call.pending.clear();
  },
};

// Only process private Wasm outputs. Never walk author declarations or
// callback values, which may be unreachable until a Rust field is forced.
function expose(value) {
  if (value === null || typeof value !== "object" || value instanceof Uint8Array) return value;
  if (Array.isArray(value)) return Object.freeze(value.map(expose));
  const handle = Object.hasOwn(value, "$rocxHandle") ? value.$rocxHandle : undefined;
  // Custom metadata can use any key, including the private envelope key.
  if (handle instanceof EngineHandle) {
    const existing = wrappers.get(handle);
    if (existing) return existing;
    const wrapper = {};
    if (value.view !== undefined) wrapper.view = expose(value.view);
    if (value.kind === "document") {
      wrapper.toDocx = (options) => render_document(wrapper, options);
    }
    if (value.methods) {
      for (const [method, operation] of Object.entries(value.methods)) {
        wrapper[method] = (...args) => invokeHandle(operation, { target: wrapper, args });
      }
    }
    handles.set(wrapper, handle);
    wrappers.set(handle, wrapper);
    return Object.freeze(wrapper);
  }
  for (const [key, child] of Object.entries(value)) {
    if (child instanceof Uint8Array) {
      Object.defineProperty(value, key, { get: () => child.slice(), enumerable: true });
    } else {
      value[key] = expose(child);
    }
  }
  return Object.freeze(value);
}

function checkSignal(signal) {
  if (signal !== undefined && (
    signal === null || typeof signal.aborted !== "boolean" ||
    typeof signal.addEventListener !== "function" || typeof signal.removeEventListener !== "function"
  )) {
    throw new TypeError("signal must be an AbortSignal");
  }
}

function readOptions(options, keys) {
  if (options === undefined) return {};
  if (options === null || typeof options !== "object" || Array.isArray(options)) {
    throw new TypeError("expected an options record");
  }
  const result = {};
  for (const key of Reflect.ownKeys(options)) {
    if (!keys.includes(key)) throw new TypeError(`unknown option ${String(key)}`);
    const field = Reflect.getOwnPropertyDescriptor(options, key);
    if (!field || !Object.hasOwn(field, "value")) throw new TypeError(`option ${key} must be a data property`);
    result[key] = field.value;
  }
  return result;
}

function aborted(signal) {
  const error = new Error("The operation was aborted", { cause: signal.reason });
  error.name = "AbortError";
  return error;
}

async function waitForInitialization(initialize, signal) {
  if (!signal) return initialize();
  if (signal.aborted) throw aborted(signal);
  let onAbort;
  try {
    await Promise.race([
      initialize(),
      new Promise((_, reject) => {
        onAbort = () => reject(aborted(signal));
        if (signal.aborted) onAbort();
        else signal.addEventListener("abort", onAbort, { once: true });
      }),
    ]);
  } finally {
    if (onAbort) signal.removeEventListener("abort", onAbort);
  }
}

export function createBoundary(initialize) {
  let initialized;
  function init() {
    initialized ??= Promise.resolve().then(initialize).then(() => configure_host(host)).catch((error) => {
      initialized = undefined;
      throw asError(error);
    });
    return initialized;
  }

  async function operation(signal, work) {
    checkSignal(signal);
    await waitForInitialization(init, signal);
    if (signal?.aborted) throw aborted(signal);
    const operation = new Operation();
    const abort = () => operation.abort(signal.reason);
    try {
      signal?.addEventListener("abort", abort, { once: true });
      return await work(operation);
    } catch (cause) {
      throw asError(cause);
    } finally {
      signal?.removeEventListener("abort", abort);
      operation.free();
    }
  }

  return {
    init,
    async resolveDocument(program, options) {
      const { signal } = readOptions(options, ["signal"]);
      // Read the host clock once, even if a deferred timestamp is authored.
      const clock = new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
      return expose(await operation(signal, (op) => resolve_program(program, clock, op)));
    },
    async resolveComponent(declaration, options) {
      const { signal, scope } = readOptions(options, ["signal", "scope"]);
      if (scope !== undefined && scope !== "inline" && scope !== "block") {
        throw new TypeError("scope must be inline or block");
      }
      return expose(await operation(signal, (op) => resolve_component(declaration, scope, op)));
    },
    async resolveValue(value, options) {
      const { signal } = readOptions(options, ["signal"]);
      return operation(signal, (op) => resolve_input(value, op));
    },
    // B1/B2 feature hooks: initialize for root calls; owned handle methods
    // can call dispatch synchronously after initialization.
    async invoke(name, input) {
      await init();
      return invokeHandle(name, input);
    },
  };
}

export function invokeHandle(name, input) {
  try {
    if (name === "toDocxWithTemplate") {
      const [source, ...remaining] = input.args;
      const load = templateFiles.get(source);
      if (load) {
        return loadTemplateFile(load, input.target, remaining);
      }
    }
    return expose(dispatch(name, input));
  } catch (cause) {
    throw asError(cause);
  }
}

async function loadTemplateFile(load, target, remaining) {
  try {
    const source = await load();
    return expose(dispatch("toDocxWithTemplate", { target, args: [source, ...remaining] }));
  } catch (cause) {
    throw asError(cause);
  }
}
