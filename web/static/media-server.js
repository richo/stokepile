import init, { load_staged_media } from '/wasm/stokepile.js';

console.log("Loaded");
async function run() {
  await init();
  document.getElementById("refresh-button")
    .addEventListener("click", load_staged_media);
}
run()
