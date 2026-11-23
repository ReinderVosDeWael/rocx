import init from "./pkg/rocx.js";
import { createGenerator } from "./generate.js";

export const generateDocx = createGenerator(() => init());
