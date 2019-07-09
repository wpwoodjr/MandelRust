/*
 * User interface for Mandelbrot web service on http://constantin.glez.de/mandelbrot
 */

// Constants
const imageDivId = "mandelbrot-image-div";
const imageId = "mandelbrot-image-img";
const paramsDivId = "mandelbrot-image-div";
const paramDisplayIdPrefix = "mandelbrot-params-";
const paramDisplayClassPrefix = paramDisplayIdPrefix + "class";
const controlsDivId = "mandelbrot-controls-div";

const imageBaseURL = "http://constantin.no.de/mandelbrot/image.png?";

const aaValues = [
  "No Antialiasing",
  "Simple: 3x3 Antialiasing, Gaussian filter",
  "Good: 3x3 Antialiasing, Mitchell/Netravali filter",
  "Better: 5x5 Antialiasing, Gaussian filter",
  "Best: 5x5 Antialiasing, Mitchell/Netravali filter"
];

const optValues = [
  "Default Optimization Level",
  "No Optimization",
  "Check for 2 biggest bulbs",
  "Subdivide areas, search for black circumferences",
  "Both subdivision and known bulb check",
  "Adaptive: Per-area optimization (default)"
];

const paramDisplays = [
  { param: "re", pre: "Real: "},
  { param: "im", pre: "Imaginary: "},
  { param: "ppu", pre: "Zoom level: ", post: " Pixel per unit." },
  { param: "max", pre: "Max # of iterations: "}
];

// Default configuration of the Mandelbrot view to be rendered.
const defaultModel = {
  re: -0.75,
  im: 0.0,
  size: 512,
  max: 100,
  ppu: 150,
  aa: 0,
  opt: 0
};

// Global variables.
var image = null;    // The <img> element that contains the Mandelbrot image.
var params = null;   // The <div> element that displays the current parameters.
var controls = null; // The <div> element that containes the controls.

var displaySpans = []; // The <span> elements for display values. Corresponds to paramDisplays.
var timeSpan = null;   // The <span> element for displaying load time.

var model = null;    // The Mandelbrot view configuration.

var time = 0; // Used for timing loads.
var elapsed = 0; // Used for timing loads.

// Update the mandelbrot image with a new set of parameters.
function updateImage() {
  var url = imageBaseURL;

  // Convert all keys in our model to URL parameters.
  for (var key in model) {
    url += key + "=" + model[key] + "&";
  }

  // Add a timestamp to the end of the URL to prevent caching.
  url += "date=" + new Date().getTime();

  // Update the URL of the image. This will trigger a reload.
  time = Date.now();
  image.src = url;

  return;
}

// Update the anti-aliasing settings and trigger an update of the image.
function updateAA(select) {
  var value = select.options[select.selectedIndex].value;

  if (model.aa != value) {
    model.aa = value;
    updateImage();
  }
  return;
}

// Update the optimization settings and trigger an update of the image.
function updateOpt(select) {
  var value = select.options[select.selectedIndex].value;

  if (model.opt != value) {
    model.opt = value;
    updateImage();
  }
  return;
}

// Zoom deeper into the Mandelbrot Set.
function zoom() {
  model.ppu = model.ppu << 1; // 1337 way of saying "* 2".
  updateImage();
  return;
}

// Zoom out of the Mandelbrot Set.
function unzoom() {
  model.ppu = model.ppu >> 1; // Guess what?
  updateImage();
  return;
}

// Increase the iteration depth.
function moreDetails() {
  model.max = model.max << 1;
  updateImage();
  return;
}

// Increase the iteration depth.
function lessDetails() {
  model.max = model.max >> 1;
  updateImage();
  return;
}

// Find the x,y coordinates of a given element on the page.
// Code from here: http://www.quirksmode.org/js/findpos.html
function findPos(obj) {
  var curleft = curtop = 0;
  if (obj.offsetParent) {
    do {
      curleft += obj.offsetLeft;
      curtop +=obj.offsetTop;
    } while (obj = obj.offsetParent);
  }
  return [curleft, curtop];
}


// Figure out the new center position for the image. (onclick handler)
function newPosition(e) {
  var imageElement = document.getElementById(imageId);
  var imageXY = findPos(imageElement);

  var x = e.pageX - imageXY[0];
  var y = e.pageY - imageXY[1];

  model.re = model.re - (model.size/model.ppu)/2 + x / model.ppu;
  model.im = model.im + (model.size/model.ppu)/2 - y / model.ppu;

  updateImage();
  return;
}

// Create the DOM elements needed to display the image.
function createImage() {
  image = document.createElement("img");

  image.alt = "The Mandelbrot Set";
  image.onclick = function() { newPosition(event); }
  image.onload = function() { updateParams(); }
  image.id = imageId;
  document.getElementById(imageDivId).appendChild(image);
  updateImage();

  return;
}

// Create the DOM elements that display the current parameters.
function createParams() {
  params = document.getElementById(paramsDivId);

  // Go through parameter descriptions and create the display elements.
  var displayDiv, displaySpan = null;

  for (var i = 0; i < paramDisplays.length; i++) {
    displayDiv = document.createElement("div");
    // Add hooks for CSS.
    displayDiv.id = paramDisplayIdPrefix + "div-" + paramDisplays[i].param;
    displayDiv.setAttribute(
      "class", paramDisplayClassPrefix + "," + paramDisplayClassPrefix + "-" + paramDisplays[i].param
    );

      if (paramDisplays[i].pre) {
        displayDiv.appendChild(document.createTextNode(paramDisplays[i].pre));
      }

      displaySpan = document.createElement("span");
      displaySpans.push(displaySpan); // We'll use this later for lookups.
      displaySpan.id = paramDisplayIdPrefix + "span-" + paramDisplays[i].param;
      displayDiv.appendChild(displaySpan);

      if (paramDisplays[i].post) {
        displayDiv.appendChild(document.createTextNode(paramDisplays[i].post));
      }

      params.appendChild(displayDiv);
  }

  // Special "parameter": Load time.
  displayDiv = document.createElement("div");
  displayDiv.id = paramDisplayIdPrefix + "div-" + "loadtime";
  displayDiv.setAttribute(
    "class", paramDisplayClassPrefix + "," + paramDisplayClassPrefix + "-" + "loadtime"
  );
  displayDiv.appendChild(document.createTextNode("Render/Load time: "));
  timeSpan = document.createElement("span");
  timeSpan.id = paramDisplayIdPrefix + "span-" + "loadtime";
  displayDiv.appendChild(timeSpan);
  displayDiv.appendChild(document.createTextNode("ms"));
  params.appendChild(displayDiv);
  
  return;
}

// Update parameter display section with current values.
// This is called exactly when loading the image has finished, so we'll also use it to measure time.
function updateParams() {
  elapsed = Date.now() - time;

  var text = "";

  for (var i = 0; i < displaySpans.length; i++) {
    text = document.createTextNode(model[paramDisplays[i].param]);

    if (displaySpans[i].firstChild) {
      displaySpans[i].replaceChild(text, displaySpans[i].firstChild);
    } else {
      displaySpans[i].appendChild(text);
    }
  }

  // Special "parameter": elapsed time.
  text = document.createTextNode(elapsed);
  if (timeSpan.firstChild) {
      timeSpan.replaceChild(text, timeSpan.firstChild);
    } else {
      timeSpan.appendChild(text);
  }

  return;  
}
  

// Create the DOM elements needed for the controls.
function createControls() {
  // Antialiasing controls: A popup menu with options.
  var aaSelection = document.createElement("select");
  aaSelection.name = "aaSelect";
  aaSelection.onchange = function() { updateAA(this); };

  var aaOption;
  for (var i = 0; i < aaValues.length; i++) {
    aaOption = document.createElement("option");
    aaOption.value = i;
    if (i == model.aa) {
      aaOption.selected = "selected";
    }
    aaOption.appendChild(document.createTextNode(aaValues[i]));
    aaSelection.appendChild(aaOption);
  }

  // Optimization controls: A popup menu with options.
  var optSelection = document.createElement("select");
  optSelection.name = "optSelect";
  optSelection.onchange = function() { updateOpt(this); };

  var optOption;
  for (var i = 0; i < optValues.length; i++) {
    optOption = document.createElement("option");
    optOption.value = i;
    if (i == model.opt) {
      optOption.selected = "selected";
    }
    optOption.appendChild(document.createTextNode(optValues[i]));
    optSelection.appendChild(optOption);
  }

  // Action buttons get their own div so they stay together.
  var actionDiv = document.createElement("div");
  actionDiv.id = "mandelbrot-action-buttons-div";

  // Zoom button: 2x zoom at every press.
  var zoomButton = document.createElement("input");
  zoomButton.type = "button";
  zoomButton.value = "Zoom";
  zoomButton.onclick = function() { zoom(); };
  actionDiv.appendChild(zoomButton);

  // Unzoom button: 2x less zoom at every press.
  var unzoomButton = document.createElement("input");
  unzoomButton.type = "button";
  unzoomButton.value = "Unzoom";
  unzoomButton.onclick = function() { unzoom(); };
  actionDiv.appendChild(unzoomButton);

  // Increase Maximum number of recursions.
  var moreDepthButton = document.createElement("input");
  moreDepthButton.type = "button";
  moreDepthButton.value = "More details";
  moreDepthButton.onclick =  function() { moreDetails(); };
  actionDiv.appendChild(moreDepthButton);

  // Decrease Maximum number of recursions.
  var lessDepthButton = document.createElement("input");
  lessDepthButton.type = "button";
  lessDepthButton.value = "Less details";
  lessDepthButton.onclick =  function() { lessDetails(); };
  actionDiv.appendChild(lessDepthButton);

  var form = document.createElement("form");
  form.name = "aaForm";
  form.appendChild(aaSelection);
  form.appendChild(optSelection);
  form.appendChild(actionDiv);

  controls = document.getElementById(controlsDivId);
  controls.appendChild(form);
}

// Initialize everything.
function init() {
  model = defaultModel;

  createImage();
  createParams();
  createControls();
}

// This writes three div tags at the place this script was loaded. They'll be filled
// with the Mandelbrot image and its controls.
document.write("<div id=\"" + imageDivId + "\"></div><div id=\"" + paramsDivId + "\"></div><div id=\"" + controlsDivId + "\"></div>\n");

// Initialize.
init();
