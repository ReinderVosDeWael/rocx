export default {
  // Keep the wasm-bindgen glue next to its .wasm URL in development, too.
  optimizeDeps: { exclude: ["rocx-docx"] },
};
