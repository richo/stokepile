import init, { load_staged_media, clear_staged_media, activate_media, trigger } from '/wasm/stokepile_wasm.js';
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

  load_staged_media();
}

window.init_slider = function() {
  window.init_slider_with_values(null, null);
}

window.init_slider_with_values = function(start, finish) {
  var video = document.getElementById('media-player');
  var slider = document.getElementById('trim-slider');
  video.addEventListener('loadedmetadata', function() {
    var max = Math.ceil(video.duration);
    var begin = start || 0;
    var end = finish || max;

    document.getElementById('max-trim').value = max;

    noUiSlider.create(slider, {
      start: [begin, end],
      connect: true,
      range: {
        'min': 0,
        'max': max,
      }
    });

    var slider_start = document.getElementById('trim-start');
    var slider_end = document.getElementById('trim-end');
    slider.noUiSlider.on('update', function(values, handle) {
      let position = parseInt(values[handle]);
      video.currentTime = position;

      if (handle === 0) {
        slider_start.value = position;
      } else if (handle === 1) {
        slider_end.value = position;
      }
    });
  })
};

window.get_slider_values = function() {
  var slider = document.getElementById('trim-slider');
  values = slider.noUiSlider.get();
  return [parseInt(slider[0]), parseInt(slider[1])];
};

run()
