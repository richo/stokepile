import init, { load_staged_media, clear_staged_media } from '/wasm/stokepile_wasm.js';

console.log("Loaded");
async function run() {
  await init();
  document.getElementById("refresh-button")
    .addEventListener("click", load_staged_media);
  document.getElementById("clear-button")
    .addEventListener("click", clear_staged_media);
}
run()
