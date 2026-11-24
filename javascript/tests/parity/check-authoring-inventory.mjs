// Compare public Rust declarations with the installed TypeScript export barrel.
// Usage: node check-authoring-inventory.mjs RUSTDOC_JSON INSTALLED_INDEX TYPESCRIPT_JS
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [jsonPath, indexPath, typescriptPath] = process.argv.slice(2);
if (!typescriptPath) {
  throw new Error("Expected RUSTDOC_JSON INSTALLED_INDEX TYPESCRIPT_JS");
}
const ts = (await import(pathToFileURL(resolve(typescriptPath)))).default;
const rust = JSON.parse(readFileSync(jsonPath, "utf8"));
const inventory = JSON.parse(readFileSync(new URL("authoring-inventory.json", import.meta.url), "utf8"));
const program = ts.createProgram([resolve(indexPath)], {
  strict: true, noEmit: true, target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.NodeNext,
});
const diagnostics = ts.getPreEmitDiagnostics(program);
if (diagnostics.length) {
  throw new Error(diagnostics.map((d) => ts.flattenDiagnosticMessageText(d.messageText, "\n")).join("\n"));
}
const checker = program.getTypeChecker();
const source = program.getSourceFile(resolve(indexPath));
const exports = new Map(checker.getExportsOfModule(checker.getSymbolAtLocation(source)).map((symbol) => [
  symbol.name, symbol.flags & ts.SymbolFlags.Alias ? checker.getAliasedSymbol(symbol) : symbol,
]));
const host = program.getSourceFile(resolve(indexPath, "../types/file-io.d.ts"));
// The Node subpath is not a dependency of the browser-safe barrel.
const hostProgram = host ? program : ts.createProgram([resolve(indexPath, "../types/file-io.d.ts")], {
  strict: true, noEmit: true, target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.NodeNext,
});
const hostChecker = hostProgram.getTypeChecker();
const hostSource = host ?? hostProgram.getSourceFile(resolve(indexPath, "../types/file-io.d.ts"));
const hostExports = new Set(hostSource ? hostChecker.getExportsOfModule(
  hostChecker.getSymbolAtLocation(hostSource),
).map((symbol) => symbol.name) : []);
const errors = [];
const reviewed = new Set();
const publicValidators = new Set();
const counts = { records: 0, fields: 0, valueSites: 0, enums: 0, variants: 0, projections: 0, validators: 0, exceptions: 0 };

function declared(name) {
  const symbol = exports.get(name);
  if (!symbol) {
    errors.push(`missing TypeScript export ${name}`);
    return undefined;
  }
  return checker.getDeclaredTypeOfSymbol(symbol);
}
function property(type, key) {
  const member = type && checker.getPropertyOfType(type, key);
  return member && checker.getTypeOfSymbolAtLocation(member, member.valueDeclaration ?? source);
}
function arms(type) {
  return type ? (type.isUnion() ? type.types : [type]) : [];
}
function views(type) {
  return arms(type).flatMap((arm) => arms(property(arm, "view")));
}
function literals(type) {
  return arms(type).filter((arm) => arm.isStringLiteral()).map((arm) => arm.value);
}
function camel(name) {
  return name.replace(/^r#/, "").replace(/^./, (letter) => letter.toLowerCase())
    .replace(/_([a-z0-9])/g, (_, letter) => letter.toUpperCase());
}
function rustNames(ids) {
  return ids.map((id) => rust.index[id]?.name).filter(Boolean);
}
function fields(type, names, label, exact = false) {
  // Union members can carry exclusive alternatives (e.g. Paragraph's text/runs).
  const keys = new Set(arms(type).flatMap((arm) => checker.getPropertiesOfType(arm).map((p) => p.name)));
  for (const name of names) {
    counts.fields++;
    if (!keys.has(name)) errors.push(`${label}: missing TypeScript field ${name}`);
  }
  if (exact) {
    for (const key of keys) {
      if (key !== "kind" && !names.includes(key)) errors.push(`${label}: unmapped TypeScript field ${key}`);
    }
  }
}
function mappedFields(ids, label) {
  return rustNames(ids).flatMap((field) => inventory.fieldMappings[`${label}.${field.replace(/^r#/, "")}`]
    ?? [camel(field)]);
}
function rustShape(type) {
  const path = type?.resolved_path;
  if (!path) return {};
  const name = path.path.split("::").at(-1);
  const args = path.args?.angle_bracketed?.args.map((arg) => arg.type) ?? [];
  const alias = rust.index[path.id]?.inner.type_alias;
  if (alias && args.length === 0) return rustShape(alias.type);
  return { name, args };
}
function checkValueShape(rustType, tsTypes, label) {
  const { name, args } = rustShape(rustType);
  const stageNames = ["Value", "Deferred", "Property", "PropertyRef"];
  const stages = tsTypes.filter((arm) => stageNames.includes(arm.symbol?.name));
  if (name === "Value") {
    counts.valueSites++;
    const observed = new Set(stages.map((arm) => arm.symbol.name));
    for (const stage of stageNames) {
      if (!observed.has(stage)) errors.push(`${label}: missing deferred input ${stage}`);
    }
    if (rustShape(args[0]).name === "Option" && stages.some((arm) =>
      !arms(checker.getTypeArguments(arm)[0]).some((candidate) => candidate.flags & ts.TypeFlags.Undefined))) {
      errors.push(`${label}: producer for Rust Value<Option<T>> must support resolving to undefined`);
    }
    checkValueShape(args[0], tsTypes.filter((arm) => !stages.includes(arm)), label);
  } else if (name === "Option") {
    checkValueShape(args[0], tsTypes, label);
  } else {
    if (stages.length) errors.push(`${label}: unexpectedly deferred non-Value input`);
    if (name === "Vec") {
      const entries = tsTypes.flatMap((arm) => arms(checker.getIndexTypeOfType(arm, ts.IndexKind.Number)));
      checkValueShape(args[0], entries, `${label}[]`);
    } else if (name === "BTreeMap" && rustShape(args[1]).name === "Value") {
      const keyEnum = rust.index[args[0].resolved_path?.id]?.inner.enum;
      const keys = keyEnum ? rustNames(keyEnum.variants).map(camel) : new Set(tsTypes.flatMap((arm) =>
        checker.getPropertiesOfType(arm).map((p) => p.name)));
      for (const key of keys) {
        checkValueShape(args[1], tsTypes.flatMap((arm) => arms(property(arm, key))), `${label}.${key}`);
      }
    }
  }
}
function token(arm, tag) {
  return (tag ? literals(property(arm, tag))[0] : undefined) ?? literals(arm)[0]
    ?? arm.aliasSymbol?.name ?? arm.symbol?.name;
}
function compareEnum(name, body, config = inventory.enums[name] ?? {}, type = declared(config.ts ?? name), record = true) {
  if (record) counts.enums++;
  if (!type) return;
  const variants = new Map((config.view ? views(type) : arms(type)).map((arm) => [token(arm, config.tag), arm]));
  const expected = new Set();
  const rustVariants = new Set();
  for (const variantId of body.variants) {
    if (record) counts.variants++;
    const variant = rust.index[variantId];
    const variantName = variant.name;
    rustVariants.add(variantName);
    const member = Object.hasOwn(config.variants ?? {}, variantName)
      ? config.variants[variantName] : camel(variantName);
    if (member === null) {
      if (!hostExports.has(config.hostOnly?.[variantName])) {
        errors.push(`${name}.${variantName}: missing host export ${config.hostOnly?.[variantName]}`);
      }
      continue;
    }
    expected.add(member);
    const arm = config.recordVariants?.[variantName] ? declared(config.recordVariants[variantName]) : variants.get(member);
    if (!variants.has(member)) errors.push(`${name}.${variantName}: missing TypeScript variant ${member}`);
    const structuralFields = variant.inner.variant.kind.struct?.fields ?? [];
    const names = [...mappedFields(structuralFields, `${name}.${variantName}`),
      ...(config.tupleFields?.[variantName] ?? [])];
    if (arm) {
      fields(arm, names, `${name}.${variantName}`);
      if (name === "ResolvedInline") {
        const constructor = arms(declared("CreateResolvedInlineInput")).find((input) => token(input, "kind") === member);
        if (!constructor) errors.push(`${name}.${variantName}: missing resolved-inline constructor arm`);
        else fields(constructor, names, `${name}.${variantName} constructor`);
      }
    }
  }
  for (const member of variants.keys()) {
    if (!expected.has(member)) errors.push(`${name}: unmapped TypeScript variant ${member}`);
  }
  for (const variant of Object.keys(config.variants ?? {})) {
    if (!rustVariants.has(variant)) errors.push(`${name}: stale variant mapping ${variant}`);
  }
  if (config.alsoModule && record) {
    const module = program.getSourceFile(resolve(indexPath, "..", config.alsoModule));
    const moduleSymbol = module && checker.getSymbolAtLocation(module);
    const symbol = moduleSymbol && checker.getExportsOfModule(moduleSymbol).find((candidate) => candidate.name === (config.ts ?? name));
    if (!symbol) errors.push(`${name}: missing declaration in ${config.alsoModule}`);
    else compareEnum(name, body, config, checker.getDeclaredTypeOfSymbol(symbol), false);
  }
}

for (const [id, path] of Object.entries(rust.paths)) {
  if (path.crate_id !== 0 || !["struct", "enum"].includes(path.kind) ||
      !inventory.scope.some((prefix) => path.path.join("::").startsWith(prefix))) continue;
  const { name, inner, visibility } = rust.index[id];
  if (visibility !== "public") continue;
  if (reviewed.has(name)) errors.push(`ambiguous Rust type name ${name}; qualify the inventory`);
  reviewed.add(name);
  const inherent = inner[path.kind].impls.map((id) => rust.index[id].inner.impl).filter((impl) => !impl.trait);
  if (inherent.some((impl) => impl.items.some((id) => {
    const item = rust.index[id];
    return item?.name === "validate" && item.visibility === "public";
  })) && name !== "PackagePlan") publicValidators.add(camel(name));
  if (Object.hasOwn(inventory.structuralExceptions, name)) {
    counts.exceptions++;
    continue;
  }
  const flattened = inventory.flattenedEnums[name];
  if (flattened) {
    counts.enums++;
    const type = declared(inventory.records[flattened.owner]);
    if (!type) continue;
    for (const variantId of inner.enum.variants) {
      counts.variants++;
      const variant = rust.index[variantId];
      const key = flattened.variants[variant.name];
      const [member] = variant.inner.variant.kind.tuple ?? [];
      if (!key || !member) errors.push(`${name}.${variant.name}: expected a flattened tuple arm`);
      else checkValueShape(rust.index[member].inner.struct_field, arms(type).flatMap((arm) =>
        arms(property(arm, key))), `${flattened.owner}.${key}`);
    }
    continue;
  }
  const projection = inventory.projections[name];
  if (projection) {
    counts.projections++;
    const type = declared(projection.ts ?? name);
    const projected = projection.view ? property(type, "view") : type;
    if (projected) {
      fields(projected, projection.fields, name);
      if (projection.variants) {
        for (const [variant, names] of Object.entries(projection.variants)) {
          const arm = arms(projected).find((arm) => token(arm, "kind") === variant);
          if (!arm) errors.push(`${name}: missing projected variant ${variant}`);
          else fields(arm, names, `${name}.${variant}`);
        }
      }
    } else if (type) errors.push(`${name}: missing TypeScript view`);
    continue;
  }
  if (path.kind === "enum") {
    compareEnum(name, inner.enum);
    continue;
  }
  counts.records++;
  const plain = inner.struct.kind.plain;
  if (!plain) {
    errors.push(`${name}: expected a plain record, add an explicit mapping for a different representation`);
    continue;
  }
  const type = declared(inventory.records[name] ?? name);
  if (type) {
    fields(type, mappedFields(plain.fields, name), name, true);
    for (const id of plain.fields) {
      const field = rust.index[id];
      if (!field || Object.values(inventory.flattenedEnums).some((entry) =>
        entry.owner === name && entry.field === field.name)) continue;
      const [key] = mappedFields([id], name);
      checkValueShape(field.inner.struct_field, arms(type).flatMap((arm) => arms(property(arm, key))), `${name}.${key}`);
    }
  }
}
const validatorArms = new Set(arms(declared("ValidateValueInput")).map((arm) => token(arm, "kind")));
for (const validator of publicValidators) {
  counts.validators++;
  if (!validatorArms.has(validator)) errors.push(`ValidateValueInput: missing Rust validator ${validator}`);
}
for (const arm of validatorArms) {
  if (!publicValidators.has(arm)) errors.push(`ValidateValueInput: unmapped validator ${arm}`);
}
for (const group of ["records", "enums", "flattenedEnums", "projections", "structuralExceptions"]) {
  for (const name of Object.keys(inventory[group])) {
    if (!reviewed.has(name)) errors.push(`stale ${group} entry: ${name}`);
  }
}
if (errors.length) {
  console.error(errors.join("\n"));
  process.exitCode = 1;
} else console.log(`Authoring inventory: ${Object.entries(counts).map(([key, count]) => `${count} ${key}`).join(", ")}`);
