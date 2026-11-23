import { randomUUID } from "node:crypto";
import { open, readFile, rename, unlink } from "node:fs/promises";
import { basename, dirname, isAbsolute, parse, resolve, sep } from "node:path";
import { future, invokeHandle, registerTemplateFile } from "./boundary.js";
import { render_document } from "./pkg/rocx.js";

export * from "./node.js";

function checkPath(path) {
  if (typeof path !== "string" || !path.length || !path.isWellFormed() || path.includes("\0")) {
    throw new TypeError("path must be a nonempty, well-formed filesystem path");
  }
}

export function templateFile(path, edits) {
  checkPath(path);
  const source = Object.freeze({});
  registerTemplateFile(source, async () => ({ bytes: await readFile(path), edits }));
  return source;
}

export function imageFile(path) {
  checkPath(path);
  return future(async ({ signal }) => {
    const bytes = await readFile(path, { signal });
    return Object.freeze({ bytes, sourceName: basename(path) });
  });
}

function inDirectory(parent, name) {
  return `${parent}${parent.endsWith(sep) ? "" : sep}${name}`;
}

function absoluteDestination(path) {
  const { root } = parse(path);
  // Windows drive-relative and current-drive-rooted paths still depend on
  // process.cwd(); resolve those using Node's platform rules.
  if (sep === "\\" && root && (root.length <= 2)) return resolve(path);
  return isAbsolute(path) ? path : inDirectory(process.cwd(), path);
}

async function replaceFile(path, bytes) {
  const temporary = inDirectory(dirname(path), `.rocx-${randomUUID()}.tmp`);
  // Only clean up a file after acquiring exclusive ownership of its name.
  const file = await open(temporary, "wx");
  try {
    try {
      await file.writeFile(bytes);
    } finally {
      await file.close();
    }
    await rename(temporary, path);
  } catch (error) {
    await unlink(temporary).catch(() => {});
    throw error;
  }
}

function saveOptions(input) {
  if (input === undefined) return {};
  if (input === null || typeof input !== "object" || Array.isArray(input)) {
    throw new TypeError("expected an options record");
  }
  const output = {};
  for (const key of Reflect.ownKeys(input)) {
    if (!["template", "profile", "timestamp", "readLimits"].includes(key)) {
      throw new TypeError(`unknown option ${String(key)}`);
    }
    const descriptor = Reflect.getOwnPropertyDescriptor(input, key);
    if (!descriptor || !Object.hasOwn(descriptor, "value")) {
      throw new TypeError(`option ${key} must be a data property`);
    }
    output[key] = descriptor.value;
  }
  return output;
}

export async function saveDocx(document, path, options) {
  checkPath(path);
  // Keep the rename and its sibling write in the same directory even if
  // process.cwd() changes during asynchronous template reading or saving.
  const destination = absoluteDestination(path);
  const { template, ...renderOptions } = saveOptions(options);
  const bytes = template === undefined
    ? render_document(document, renderOptions)
    : await invokeHandle("toDocxWithTemplate", { target: document, args: [template, renderOptions] });
  await replaceFile(destination, bytes);
}
