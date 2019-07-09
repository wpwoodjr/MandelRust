/*
 * colormap.js
 *
 * A module to deal with colormaps when rendering JavaScript
 *
 * See http://constantin.glez.de/mandelbrot for details
 */

// Generate a colormap of the given size. This is a table with 3-color-entries, each color a byte.
exports.colormap = function (size) {
  colormap = new Array(size);
  var j = 0;

  for (var i = 0; i < size; i++) {
    j = i / size;
    colormap[i] = [
      Math.floor((Math.sin(((j * 4) + 0.0) * Math.PI) + 1) * 255 / 2),
      Math.floor((Math.sin(((j * 8) + 0.0) * Math.PI) + 1) * 255 / 2),
      Math.floor((Math.sin(((j * 16) + 0.0) * Math.PI) + 1) * 255 / 2)
    ];
  }

  // The entry for 0 is always black
  colormap[0] = [0,0,0];
  
  return colormap;
}
