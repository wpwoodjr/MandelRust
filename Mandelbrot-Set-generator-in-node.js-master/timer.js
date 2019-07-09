/*
 * timer.js
 *
 * Simple timing functions.
 *
 * See http://constantin.glez.de/mandelbrot for details.
 */

var time = 0, elapsed = 0;

exports.start = function() {
  time = Date.now();
  return;
}

exports.stop = function() {
  elapsed = Date.now() - time;
  return elapsed;
}
