import init, { load_staged_media, clear_staged_media, play_media } from '/wasm/stokepile_wasm.js';

async function run() {
  await init();
  document.getElementById("refresh-button")
    .addEventListener("click", load_staged_media);
  document.getElementById("clear-button")
    .addEventListener("click", clear_staged_media);

  let filter = '.media-list-item';
  document.querySelector("#media-list")
    .addEventListener("click", function(event) {
      play_media(event.srcElement.dataset.contentHash);
    })
  load_staged_media();
}
run()
