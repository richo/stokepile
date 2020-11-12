import init, { load_staged_media, clear_staged_media, activate_media } from '/wasm/stokepile_wasm.js';
import '/vendor/nouislider/nouislider.min.js';

async function run() {
  await init();
  document.getElementById("refresh-button")
    .addEventListener("click", load_staged_media);
  document.getElementById("clear-button")
    .addEventListener("click", clear_staged_media);

  let filter = '.media-list-item';
  document.querySelector("#media-list")
    .addEventListener("click", function(event) {
      activate_media(event.srcElement.dataset.uuid);
    })

  // This all becomes a part of the "select media" call
  var slider = document.getElementById('trim-slider');

  noUiSlider.create(slider, {
    start: [0, 1000],
    connect: true,
    range: {
      'min': 0,
      'max': 1000,
    }
  });

  load_staged_media();
}
run()
