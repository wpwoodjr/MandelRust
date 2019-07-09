'use strict';

var utils = require('../utils/writer.js');
var Mandelbrot = require('../service/MandelbrotService');

module.exports.computeMandelbrot = function computeMandelbrot (req, res, next) {
  var mandelbrotCoords = req.swagger.params['mandelbrotCoords'].value;
  Mandelbrot.computeMandelbrot(mandelbrotCoords)
    .then(function (response) {
      utils.writeJson(res, response);
    })
    .catch(function (response) {
      utils.writeJson(res, response);
    });
};
