/*
 * mandelbrot-handler.js
 *
 * Basic handler for a mandelbrot server URL namespace in node.
 *
 * Can be mounted within a connect server instance.
 *
 * See http://constantin.glez.de/mandelbrot for details.
 */

// Constants.

// Image constants.
const SIZE = 512; // Default value
const MAX_SIZE = 10000;

// Which subset to render?
const RE_CENTER = -0.75; // X-Center of the picture will represent this value on the real axis.
const IM_CENTER = 0.0;   // Y-Center of the picture will represent this value on the imaginary axis.
const PXPERUNIT = 150;   // How much units in the complex plane are covered by one pixel?

// Other iteration parameters.
const MAX_ITER = 300;
const COLORS = MAX_ITER * 10;
const OPT = 5; // Default optimization level.
const MAX_OPT = 5;

// We'll render at a much higher size than the image we produce, so we can get antialiasing.
// 0: No antialiasing.
// 1: 3x3 anti-aliasing with Gaussian filter.
// 2: 3x3 anti-aliasing with Mitchell-Netravali filter.
// 3: 5x5 anti-aliasing with Gaussian filter.
// 4: 5x5 anti-aliasing with Mitchell-Netravali filter.
const AA = 0;
const MAX_AA = 4;

// Modules we want to use.
var Connect = require('connect');
var Mandelbrot = require('mandelbrot-engine.js');
var Colormap = require('colormap.js');
var Png = require ('png').Png;
var Resize = require ('resize.js');
var Url = require ('url')
var Timer = require('timer.js');

// Show a Mandelbrot set image.
function show_image(req, res) {
  // Extract parameters from the request
  var params = Url.parse(req.url, true);

  // Check parameters and determine final values
  var re_center = Number(params.query.re) || RE_CENTER;
  if (re_center == Number.NaN) re_center = RE_CENTER;

  var im_center = Number(params.query.im) || IM_CENTER;
  if (im_center == Number.NaN) im_center = IM_CENTER;

  var size = Number(params.query.size) || SIZE;
  if (size == Number.NaN) size = SIZE;
  if (size > MAX_SIZE) size = MAX_SIZE;
  if (size < 0) size = SIZE;
  
  var ppu = Number(params.query.ppu) || PXPERUNIT;
  if (ppu == Number.NaN) ppu = PXPERUNIT;
  if (ppu < 0) ppu = PXPERUNIT;
  
  var max = Number(params.query.max) || MAX_ITER;
  if (max == Number.NaN) max = MAX_ITER;
  if (max < 0) max = MAX_ITER;
  
  var opt = Number(params.query.opt) || OPT;
  if (opt == Number.NaN) opt = OPT;
  if (opt < 0) opt = OPT;
  if (opt > MAX_OPT) opt = OPT;
  
  var aa = Number(params.query.aa) || AA;
  if (aa == Number.NaN) aa = AA;
  if (aa < 0) aa = AA;
  if (aa > MAX_AA) aa = MAX_AA;

  // Render a Mandelbrot set into a result array
  switch (aa) {
    case 0:
      var rendersize = size;
      var renderppu = ppu;
      break;
    case 1:
    case 2:
      var rendersize = size * 3;
      var renderppu = ppu * 3;
      break;
    case 3:
    case 4:
      var rendersize = size * 5;
      var renderppu = ppu * 5;
  }

  var totaltime = 0;
  var elapsed = 0;

  Timer.start();
  var result = Mandelbrot.render(rendersize, re_center, im_center, renderppu, max, opt);
  elapsed = Timer.stop();
  totaltime += elapsed;
  process.stdout.write("Mandelbrot set rendering time: " + elapsed + "ms\n");

  Timer.start();
  // Create a colormap.
  var map = Colormap.colormap(COLORS);

  // Create an image buffer.
  var image = new Buffer(rendersize * rendersize * 3);

  // Compute the color normalization factor.
  var f = COLORS/(max + 1);

  // Fill the image buffer with the result from the Mandelbrot set, mapped to the colormap.
  var pos = 0;
  var color = [];
  for (i = 0; i < rendersize * rendersize; i++) {
    color = map[Math.floor(result[i] * f)];
    image[pos++] = color[0];
    image[pos++] = color[1];
    image[pos++] = color[2];
  }
  elapsed = Timer.stop();
  totaltime += elapsed;
  process.stdout.write("Color mapping time: " + elapsed + "ms\n");

  Timer.start();

  // Resize the image using a Gaussian filter to provide high quality anti-aliasing.
  switch (aa) {
    case 0:
      clean_image = image;
      break;
    case 1:
      clean_image = Resize.resizento1(image, rendersize, 3, Resize.gauss(3, 0.5));
      break;
    case 2:
      clean_image = Resize.resizento1(image, rendersize, 3, Resize.mitchell(3, 0.4, 0.3));
      break;
    case 3:
      clean_image = Resize.resizento1(image, rendersize, 5, Resize.gauss(5, 0.5));
      break;
    case 4:
      clean_image = Resize.resizento1(image, rendersize, 5, Resize.mitchell(5, 0.4, 0.3));
      break;
  }

  elapsed = Timer.stop();
  totaltime += elapsed;
  process.stdout.write("Image resizing/filtering time: " + elapsed + "ms\n");

  Timer.start();

  // Convert the image into PNG format.
  var png_image = new Png(clean_image, size, size, 'rgb');
  var png_file = png_image.encodeSync();

  elapsed = Timer.stop();
  totaltime += elapsed;
  process.stdout.write("PNG encoding time: " + elapsed + "ms\n");

  // Return the image to the browser.
  res.writeHead(200, { "Content-Type": "image/png" });
  res.end(png_file.toString('binary'), 'binary');
}

exports.handler = Connect.router(function(app) {
  app.get('/', function(req, res) {
    res.writeHead(307, { "Location": "/mandelbrot/image.png" });
    res.end();
  });
  app.get('/image.png', show_image);
});
