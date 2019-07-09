/*
 * server.js
 *
 * A simple connect HTTP server, used to dispatch to more interesting functions.
 *
 * See http://constantin.glez.de/mandelbrot for details.
 */

// Modules we want to use.
var connect = require('connect');

// Our self-written modules.
var simplemandelbrot = require('simplemandelbrot.js'); // Simple demo Mandelbrot generator.
var mandelbrot = require('mandelbrot-handler.js'); // A more complex Mandelbrot generator.

// Simple HTTP server with some dispatch logic.
var server = connect.createServer(
  connect.profiler(),
  connect.favicon(__dirname + '/static/images/favicon.ico'),
  connect.logger(),
  connect.static(__dirname + '/html') // Static pages go here.
);

server.use('/simplemandelbrot', simplemandelbrot.handler);
server.use('/mandelbrot', mandelbrot.handler);

// Hack to determine whether we're running on my laptop or on no.de.
if (process.env.USER == "constantin") {
  server.listen(8000);
} else {
  server.listen(80);
}
