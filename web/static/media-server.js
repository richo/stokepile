import init, { load_staged_media, testing } from '/wasm/stokepile_wasm.js';

console.log("Loaded");
async function run() {
  await init();
  document.getElementById("refresh-button")
    .addEventListener("click", load_staged_media);
}
run()
