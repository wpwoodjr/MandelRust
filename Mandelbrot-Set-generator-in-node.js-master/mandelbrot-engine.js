/*
 * mandelbrot-engine.js
 *
 * A node.js implementation of the Mandelbrot set.
 * Basic engine that generates an array of results for the mandelbrot iteration.
 * This array can then be mixed with a colormap to create nice pictures.
 *
 * See http://constantin.glez.de/mandelbrot for details.
 */

/*
 * Find out whether this complex number is in the mandelbrot set or not.
 * cr: Real part of complex number c to iterate with.
 * ci: Imaginary part of complex number c to iterate with.
 *
 * Returns a value mu, which is the number of iterations needed to escape the e
 * (or 0 if the formula never escaped), plus some extra fractional value to all
 * smooth coloring.
 *
 * Simple, basic implementation without any optimizations.
 */
function iterate_basic(cr, ci, max) {
  var zr = 0;
  var zi = 0;
  var t  = 0; // A temporary store.
  var m2 = 0; // The modulo of the complex number z, squared.
  var zr2 = 0; // Real part of z, squared. Will be reused in this variable late
  var zi2 = 0; // Imaginary part of z, squared.

  // Iterate through all pixels.
  for (var i = 1; i < max; i++) {
    // z = z^2 ...
    t = zr2 - zi2;
    zi = 2 * zr * zi;
    zr = t;

    // ... + c    
    zr += cr;
    zi += ci;

    // To be reused in the test and the next iteration.
    zr2 = zr * zr;
    zi2 = zi * zi;

    // Test if we escaped the equation
    m2 = zr2 + zi2
    if (m2 > 4) { // Mandelbrot escape radius is 2, r^2=4
      // Return smoothed escape value.
      // Look up Mandelbrot on Wikipedia and Google to get the smoothing equation.
      return (i + 1 - (Math.log(Math.log(Math.sqrt(m2))) / Math.LN2));
    }
  }

  return 0;
}

// Find out whether this complex number is in the mandelbrot set or not.
// Optimization: Test if the point is in the period 1 or 2 bulbs before iterating.
// cr: Real part of complex number c to iterate with.
// ci: Imaginary part of complex number c to iterate with.
//
// Returns a value mu, which is the number of iterations needed to escape the e
// (or 0 if the formula never escaped), plus some extra fractional value to all
// smooth coloring.
function iterate_opt(cr, ci, max) {
  var zr = 0;
  var zi = 0;
  var t  = 0; // A temporary store.
  var m2 = 0; // The modulo of the complex number z, squared.
  var zr2 = 0; // Real part of z, squared. Will be reused in this variable late
  var zi2 = 0; // Imaginary part of z, squared.

  // Before we iterate, we'll perform some tests for optimization purposes.

  // Test if the point is within the cardioid bulb to avoid calculation...
  var x4 = cr - 0.25;
  var y2 = ci * ci;
  var q = x4 * x4 + y2;
  t = q * (q + x4);
  if (t < y2 * 0.25) {
    return 0;
  }
  // ...Maybe it's within the period-2-bulb...
  if (((cr + 1) * (cr + 1) + y2) < (1 / 16)) {
    return 0;
  }

  // Ok, we'll have to go through the whole iteration thing.
  for (var i = 1; i < max; i++) {
    // z = z^2 ...
    t = zr2  - zi2;
    zi = 2 * zr * zi;
    zr = t;

    // ... + c    
    zr += cr;
    zi += ci;

    // To be reused in the test and the next iteration.
    zr2 = zr * zr;
    zi2 = zi * zi;

    // Test if we escaped the equation
    m2 = zr2 + zi2
    if (m2 > 4) { // Mandelbrot escape radius is 2, hence 4 since we compare to
      // Return smoothed escape value.
      return (i + 1 - (Math.log(Math.log(Math.sqrt(m2))) / Math.LN2));
    }
  }

  return 0;
}

// Perform the iteration, test for the period 1 bulb to avoid calculation.
// cr: Real part of complex number c to iterate with.
// ci: Imaginary part of complex number c to iterate with.
//
// Returns a value mu, which is the number of iterations needed to escape the e
// (or 0 if the formula never escaped), plus some extra fractional value to all
// smooth coloring.
function iterate_opt_1(cr, ci, max) {
  var zr = 0;
  var zi = 0;
  var t  = 0; // A temporary store.
  var m2 = 0; // The modulo of the complex number z, squared.
  var zr2 = 0; // Real part of z, squared. Will be reused in this variable late
  var zi2 = 0; // Imaginary part of z, squared.

  // Before we iterate, we'll perform some tests for optimization purposes.

  // Test if the point is within the cardioid bulb to avoid calculation...
  var x4 = cr - 0.25;
  var y2 = ci * ci;
  var q = x4 * x4 + y2;
  t = q * (q + x4);
  if (t < y2 * 0.25) {
    return 0;
  }

  // Ok, we'll have to go through the whole iteration thing.
  for (var i = 1; i < max; i++) {
    // z = z^2 ...
    t = zr2  - zi2;
    zi = 2 * zr * zi;
    zr = t;

    // ... + c    
    zr += cr;
    zi += ci;

    // To be reused in the test and the next iteration.
    zr2 = zr * zr;
    zi2 = zi * zi;

    // Test if we escaped the equation
    m2 = zr2 + zi2
    if (m2 > 4) { // Mandelbrot escape radius is 2, hence 4 since we compare to
      // Return smoothed escape value.
      return (i + 1 - (Math.log(Math.log(Math.sqrt(m2))) / Math.LN2));
    }
  }

  return 0;
}

// Find out whether this complex number is in the mandelbrot set or not.
// Optimization: Test if the point is in the period 2 bulb before iterating.
// cr: Real part of complex number c to iterate with.
// ci: Imaginary part of complex number c to iterate with.
//
// Returns a value mu, which is the number of iterations needed to escape the e
// (or 0 if the formula never escaped), plus some extra fractional value to all
// smooth coloring.
function iterate_opt_2(cr, ci, max) {
  var zr = 0;
  var zi = 0;
  var t  = 0; // A temporary store.
  var m2 = 0; // The modulo of the complex number z, squared.
  var zr2 = 0; // Real part of z, squared. Will be reused in this variable late
  var zi2 = 0; // Imaginary part of z, squared.

  // Before we iterate, we'll perform some tests for optimization purposes.

  // Test if the point is in the period 2 bulb
  var y2 = ci * ci;
  if (((cr + 1) * (cr + 1) + y2) < (1 / 16)) {
    return 0;
  }

  // Ok, we'll have to go through the whole iteration thing.
  for (var i = 1; i < max; i++) {
    // z = z^2 ...
    t = zr2  - zi2;
    zi = 2 * zr * zi;
    zr = t;

    // ... + c    
    zr += cr;
    zi += ci;

    // To be reused in the test and the next iteration.
    zr2 = zr * zr;
    zi2 = zi * zi;

    // Test if we escaped the equation
    m2 = zr2 + zi2
    if (m2 > 4) { // Mandelbrot escape radius is 2, hence 4 since we compare to
      // Return smoothed escape value.
      return (i + 1 - (Math.log(Math.log(Math.sqrt(m2))) / Math.LN2));
    }
  }

  return 0;
}


/*
 * "Object" definitions for Mandelbrot set image stuctures.
 */

// Describe a particular point of the mandelbrot set, with a max iteration value.
function point(re, im, max) {
  this.re = re;
  this.im = im;
  this.max = max;
  return;
}

// A method for image: Clear the buffer.
function image_clear() {
  for (var i = 0; i < this.size * this.size; i++) {
    this.buffer[i] = 0.0;
  }
  return;
};

/*
 * Describe an image table for a rendition of the Mandelbrot set.
 * buffer: An array containing iteration results, one per x/y coordinate. If this is
 *         null, a new buffer will be created.
 * size:   The total width and height of the image. An index + size brings us to the next line
 *         at the same x co-ordinate. This also means that the whole image is always quadratic.
 * x,y:    Starting co-ordinates of the subimage.
 * sx, sy: x and y sizes of the subimage.
 */
function image(buffer, size, x, y, sx, sy) {
  if (buffer != null) {
    this.buffer = buffer;
  } else {
    this.buffer = new Array(size * size);
  }

  this.size = size;
  this.x = x;
  this.y = y;
  this.sx = sx;
  this.sy = sy;

  // Convenience values.

  // Number of index values to go from the end of a line to the beginning of next.
  this.xextra = this.size - this.sx;
  this.startpos = this.y * this.size + this.x;

  // Add a method for clearing the whole image buffer
  this.clear = image_clear;

  return;
}

// Method for mandset: Return a new mandset Object that references the specified subimage.
function mandset_subimage(x, y, sizex, sizey) {
  return new mandset(
    this.center,
    new image(this.image.buffer, this.image.size, x, y, sizex, sizey),
    this.ppu
  );
}

/*
 * Method for mandset: Return a new mandset which describes the intersection with the given
 * coordinates in the complex plane. This is used to apply different optimizations to different
 * areas of interest in the set.
 *
 * Args:
 * re1, im1: Top left corner of the region to intersect with. Can be null for "unlimited top/left".
 * re2, im2: Bottom right corner of the region to intersect with. Can be null for "unlimited bottom/right".
 *
 * Returns:
 * A new mandset Object with an image region that is the intersection of the old Object and the parameters.
 */
function mandset_intersect(re1, im1, re2, im2) {
  var newx = this.image.x;
  var newy = this.image.y;
  var newsx = this.image.sx;
  var newsy = this.image.sy;

  // Figure out the new x and sx values.
  if (re1 == null || re1 < this.lre) { // Keep newx = x;
    if (re2 != null && re2 < this.rre) {
      newsx = Math.floor((re2 - this.lre) * this.ppu + 0.5); // +0.5 so we can avoid floating point SNAFUs.
    } // Otherwise keep newsx = old sx.
  } else {
    newx = Math.floor((re1 - this.lre) * this.ppu + 0.5);
    if (re2 != null && re2 < this.rre) {
      newsx = Math.floor((re2 - re1) * this.ppu + 0.5); // +0.5 so we can avoid floating point SNAFUs.
    } else {
      newsx = newsx + this.image.x - newx;
      //newsx = Math.floor((this.rre - re1) * this.ppu + 0.5);
    }
  }

  // Figure out the new y and sy values.
  if (im1 == null || im1 > this.tim) { // Keep newy = y;
    if (im2 != null && im2 > this.bim) {
      newsy = Math.floor((this.tim - im2) * this.ppu + 0.5); // +0.5 so we can avoid floating point SNAFUs.
    } // Otherwise keep newsx = old sx.
  } else {
    newy = Math.floor((this.tim - im1) * this.ppu + 0.5);
    if (im2 != null && im2 > this.bim) {
      newsy = Math.floor((im1 - im2) * this.ppu + 0.5); // +0.5 so we can avoid floating point SNAFUs.
    } else {
      newsy = newsy + this.image.y - newy;
      //newsy = Math.floor((im1 - this.bim) * this.ppu + 0.5);
    }
  }
    
  return this.subimage(newx, newy, newsx, newsy);
}

// Method for mandset: Dump data for diagnostic purposes.
function mandset_dump() {
  return "Center: " + this.center.re + " + " + this.center.im + "i. Max: " + this.center.max + "\n" +
    "Image: Size: " + this.image.size + ", x: " + this.image.x + ", y:" + this.image.y + ", " + this.image.sx + "x" + this.image.sy + "\n" +
    "Minre: " + this.minre + ", Maxim: " + this.maxim + "\n";
}

/*
 * Describe a Mandelbrot set rendition.
 * center: A point referencing the center of the mandelbrot set to be rendered, plus max iteration.
 * image:  The image (portion) to render into.
 * ppu:    The resolution: How many pixels to use per complex plane unit.
 */
function mandset(center, img, ppu) {
  this.center = center;
  this.image = img;
  this.ppu = ppu;

  // Convenience values
  this.inc = 1 / this.ppu; // The increment in complex value per pixel.
  // Minimum real and maximum imaginary values for the whole image.
  this.minre = this.center.re - (this.image.size / this.ppu) / 2;
  this.maxim = this.center.im + (this.image.size / this.ppu) / 2;

  // Figure out the real and imaginary values for the 4 corners of the (sub)image.
  this.lre = this.minre + this.image.x * this.inc;      // Left real.
  this.rre = this.lre + (this.image.sx - 1) * this.inc; // Right real.
  this.tim = this.maxim - this.image.y * this.inc;      // Top imaginary.
  this.bim = this.tim - (this.image.sy - 1) * this.inc; // Bottom imaginary.

  // Add a method for returning a subimage.
  this.subimage = mandset_subimage;

  // Add a method for returning an intersection with a complex rectangular area.
  this.intersect = mandset_intersect;

  // Add a method for dumping our data.
  this.dump = mandset_dump;
  
  return;
}

/*
 * Set up a buffer, then render the Mandelbrot set into it.
 */
exports.render = function (size, re, im, ppu, max, opt) {
  // Create some data structures.
  var center = new point(re, im, max);
  // The subimage structure contains the whole image at first.
  var img = new image(null, size, 0, 0, size, size); // The whole image.
  // Now combine into the mandelbrot set structure.
  var set = new mandset(center, img, ppu);

  /*
   * Four levels of optimization:
   * 0: Use default (= 4).
   * 1: No optimization.
   * 2: Check for known bulbs.
   * 3: Subdivide areas, then check if the circumference is in the set.
   * 4: Both subdivision and known bulb check.
   * 5: Adaptive rendering: Decompose the rendering area into known areas with specific choice
   *    of optimization strategy. Easy areas render in basic mode, complex areas with more
   *    optimization, mirror bottom image if top has been rendered already, etc.
   */
  switch (opt) {
    case 1:
      render_basic(set, iterate_basic);
      return set.image.buffer;

    case 2:
      render_basic(set, iterate_opt);
      return set.image.buffer;

    case 3:
      // The subdivision algorithm assumes that the buffer has been zeroed.
      set.image.clear();
      render_opt(set, iterate_basic);
      return set.image.buffer;

    case 4:
      // The subdivision algorithm assumes that the buffer has been zeroed.
      set.image.clear();
      render_opt(set, iterate_opt);
      return set.image.buffer;

    default: // 0 = 5 = best algorithm.
      // The subdivision algorithm assumes that the buffer has been zeroed.
      set.image.clear();
      render_adaptive(set);
      return set.image.buffer;
  }
}

/*
 * Our main function that does the work.
 *
 * Arguments:
 * set: The description of the Mandelbrot subset to render.
 * image: The description of the (sub)image structure to render into.
 *        We assume that the image is quadratic, i.e. sx = sy. We only use sx and ignore sy.
 *
 */
function render_basic(set, iterator) {
  var zre = 0;
  var zim = 0;

  var pos = set.image.startpos;

  // For debugging.
  //process.stdout.write("Render Basic:\n" + set.dump());

  for (var y = set.image.y; y < set.image.y + set.image.sy; y++) {
    zim = set.maxim - y * set.inc;
    for (var x = set.image.x; x < set.image.x + set.image.sx; x++) {
      zre = set.minre + x * set.inc;
      set.image.buffer[pos++] = iterator(zre, zim, set.center.max);
    }
    pos += set.image.xextra;
  }

  return;
}

/*
 * Walk the circumference of a quadratic area of the mandelbrot set and compute the iterations.
 * Return 0 if all results were 0, 1 otherwise.
 * This will be used to determine if a quadratic area is contained in the Mandelbrot set. Its
 * inner part doesn't need to be rendered if the circumference is completelty part of it.
 */
function walk_around(set, iterator) {
  // We draw 4 lines simultaneously to minimize loop overhead.
  var pos1 = set.image.startpos
  var pos2 = pos1 + set.image.sx - 1;
  var pos3 = pos2 + set.image.size * (set.image.sy - 1);
  var pos4 = pos3 - set.image.sx + 1;
  var zre1 = set.lre;
  var zre2 = set.rre;
  var zre3 = zre2;
  var zre4 = zre1;
  var zim1 = set.tim;
  var zim2 = zim1;
  var zim3 = set.bim
  var zim4 = zim3;
  var touche = 0;

  // Walk the horizontal paths
  for (var i = 0; i < set.image.sx - 1; i++) { // No need to go all the way, the corner's covered elsewhere.
    // Upper edge
    if (set.image.buffer[pos1++] = iterator(zre1, zim1, set.center.max)) touche = 1;
    zre1 += set.inc;

    // Bottom edge
    if (set.image.buffer[pos3--] = iterator(zre3, zim3, set.center.max)) touche = 1;
    zre3 -= set.inc;
  }

  // Walk the vertical paths
  for (var i = 0; i < set.image.sy - 1; i++) { // No need to go all the way, the corner's covered elsewhere.
    // Right edge
    if (set.image.buffer[pos2] = iterator(zre2, zim2, set.center.max)) touche = 1;
    zim2 -= set.inc;
    pos2 += set.image.size;

    // Left edge
    if (set.image.buffer[pos4] = iterator(zre4, zim4, set.center.max)) touche = 1;
    zim4 += set.inc;
    pos4 -= set.image.size;
  }

  return touche;
}

/*
 * Render a horizontal line of pixels. We'll render the top line of the image specified in the set.
 */
function render_hline(set, iterator) {
  var pos = set.image.startpos;
  var zre = set.lre;
  for (var x = set.image.x; x < set.image.x + set.image.sx; x++) {
    set.image.buffer[pos++] = iterator(zre, set.tim, set.center.max);
    zre += set.inc; 
  }
  return;
}

/*
 * Render a vertical line of pixels. We'll render the left vertical line of the image given.
 */
function render_vline(set, iterator) {
  var pos = set.image.startpos;
  var zim = set.tim;
  for (var y = set.image.y; y < set.image.y + set.image.sy; y++) {
    set.image.buffer[pos] = iterator(set.lre, zim, set.center.max);
    pos += set.image.size;
    zim -= set.inc; 
  }
  return;
}

/*
 * Segment the picture into quadrants, then apply some optimization by figuring out if the
 * circumference of a quadrant is contained inside the Mandelbrot set. Since the Mandelbrot
 * set is interconnected, there can't be any holes or islands, so if the whole circumference
 * of a quadrant is 0, the whole quadrant must be.
 *
 * We will call this function recursively to look at sub-tiles.
 *
 * Inputs:
 * set:      A data structure describing the set and the (sub)image.
 *           We assume that the buffer has been zeroed from the beginning.
 * iterator: A function that computes the Mandelbrot set iteration for us.
 *
 * Assumption: The image to be rendered are quadratic. We will only use set.image.sx
 *             for determining a quadratic tile's size.
 */
function render_opt(set, iterator) {
  var pos = set.image.startpos;

  // For debugging.
  //process.stdout.write("Render Opt:\n" + set.dump());

  // Treat the lower subsizes as special cases to save on overhead.
  switch (set.image.sx) {
    case 1:
      set.image.buffer[pos] = iterator(set.lre, set.tim, set.center.max);
      return;

    // Special case: If we're just a 2x2 subtile, render all.
    case 2:
      set.image.buffer[pos++] = iterator(set.lre, set.tim, set.center.max); // Top left pixel.
      set.image.buffer[pos] = iterator(set.rre, set.tim, set.center.max); // Top right pixel.
      pos += set.image.size;
      set.image.buffer[pos--] = iterator(set.rre, set.bim, set.center.max); // Bottom right pixel.
      set.image.buffer[pos] = iterator(set.lre, set.bim, set.center.max); // Bottom left pixel.
      return;

    // Special case: 3x3.
    case 3:
      var touche = 0;
      var zre = set.lre;
      var zim = set.tim;

      // Walk the 3x3 circumference by hand. Faster than a subroutine and/or loop.
      if (set.image.buffer[pos++] = iterator(zre, zim, set.center.max)) touche = 1;
      zre += set.inc;
      if (set.image.buffer[pos++] = iterator(zre, zim, set.center.max)) touche = 1;
      zre += set.inc;
      if (set.image.buffer[pos] = iterator(zre, zim, set.center.max)) touche = 1;
      zim -= set.inc;
      pos += set.image.size;
      if (set.image.buffer[pos] = iterator(zre, zim, set.center.max)) touche = 1;
      zim -= set.inc;
      pos += set.image.size;
      if (set.image.buffer[pos--] = iterator(zre, zim, set.center.max)) touche = 1;
      zre -= set.inc;
      if (set.image.buffer[pos--] = iterator(zre, zim, set.center.max)) touche = 1;
      zre -= set.inc;
      if (set.image.buffer[pos] = iterator(zre, zim, set.center.max)) touche = 1;
      pos -= set.image.size;
      zim += set.inc;
      if (set.image.buffer[pos++] = iterator(zre, zim, set.center.max)) touche = 1;
      // Fill the rectangle only if needed.
      if (touche) {
        zre += set.inc;
        set.image.buffer[pos] = iterator(zre, zim, set.center.max);
      }
      return;

    // Special case: 4x4.
    case 4:
      var touche = 0;
      var zre = set.lre;
      var zim = set.tim;

      // Walk the 4x4 circumference by hand. Faster than a subroutine and/or loop.
      if (set.image.buffer[pos++] = iterator(zre, zim, set.center.max)) touche = 1;
      zre += set.inc;
      if (set.image.buffer[pos++] = iterator(zre, zim, set.center.max)) touche = 1;
      zre += set.inc;
      if (set.image.buffer[pos++] = iterator(zre, zim, set.center.max)) touche = 1;
      zre = set.rre;
      if (set.image.buffer[pos] = iterator(zre, zim, set.center.max)) touche = 1;
      zim -= set.inc;
      pos += set.image.size;
      if (set.image.buffer[pos] = iterator(zre, zim, set.center.max)) touche = 1;
      zim -= set.inc;
      pos += set.image.size;
      if (set.image.buffer[pos] = iterator(zre, zim, set.center.max)) touche = 1;
      zim = set.bim;
      pos += set.image.size;
      if (set.image.buffer[pos--] = iterator(zre, zim, set.center.max)) touche = 1;
      zre -= set.inc;
      if (set.image.buffer[pos--] = iterator(zre, zim, set.center.max)) touche = 1;
      zre -= set.inc;
      if (set.image.buffer[pos--] = iterator(zre, zim, set.center.max)) touche = 1;
      zre == set.lre;
      if (set.image.buffer[pos] = iterator(zre, zim, set.center.max)) touche = 1;
      pos -= set.image.size;
      zim += set.inc;
      if (set.image.buffer[pos] = iterator(zre, zim, set.center.max)) touche = 1;
      pos -= set.image.size;
      zim += set.inc;
      if (set.image.buffer[pos++] = iterator(zre, zim, set.center.max)) touche = 1;

      // Fill the rectangle only if needed.
      if (touche) {
        zre += set.inc;
        set.image.buffer[pos++] = iterator(zre, zim, set.center.max);
        zre += set.inc;
        set.image.buffer[pos] = iterator(zre, zim, set.center.max);
        zim -= set.inc;
        pos += set.image.size;
        set.image.buffer[pos--] = iterator(zre, zim, set.center.max);
        zre -= set.inc;
        set.image.buffer[pos] = iterator(zre, zim, set.center.max);
      }

      return;

    default:
      // Walk the circumference of the buffer, then figure out if all values were equal.

      if (walk_around(set, iterator)) {

        var newsize = 0;
        // Figure out how to best subdivide the remainder.
        if ((set.image.sx - 2) % 2 == 0) { // We can divide the remainder evenly
          newsize = (set.image.sx - 2) >> 1;
          render_opt(set.subimage(set.image.x + 1, set.image.y + 1, newsize, newsize), iterator);
          render_opt(set.subimage(set.image.x + 1 + newsize, set.image.y + 1, newsize, newsize), iterator);
          render_opt(set.subimage(set.image.x + 1, set.image.y + 1 + newsize, newsize, newsize), iterator);
          render_opt(set.subimage(set.image.x + 1 + newsize, set.image.y + 1 + newsize, newsize, newsize), iterator);
        } else if ((set.image.sx - 2) % 3 == 0) { // We can subdivide by 3.
          newsize = Math.floor((set.image.sx - 2) / 3); // Protect against float imprecision.
          render_opt(set.subimage(set.image.x + 1, set.image.y + 1, newsize, newsize), iterator);
          render_opt(set.subimage(set.image.x + 1 + newsize, set.image.y + 1, newsize, newsize), iterator);
          render_opt(set.subimage(set.image.x + 1 + (newsize << 1), set.image.y + 1, newsize, newsize), iterator);
          render_opt(set.subimage(set.image.x + 1, set.image.y + 1 + newsize, newsize, newsize), iterator);
          render_opt(set.subimage(set.image.x + 1 + newsize, set.image.y + 1 + newsize, newsize, newsize), iterator);
          render_opt(set.subimage(set.image.x + 1 + (newsize << 1), set.image.y + 1 + newsize, newsize, newsize), iterator);
          render_opt(set.subimage(set.image.x + 1, set.image.y + 1 + (newsize << 1), newsize, newsize), iterator);
          render_opt(set.subimage(set.image.x + 1 + newsize, set.image.y + 1 + (newsize << 1), newsize, newsize), iterator);
          render_opt(set.subimage(set.image.x + 1 + (newsize << 1), set.image.y + 1 + (newsize << 1), newsize, newsize), iterator);
        } else { // A generic uneven subsize.
          // Render 1 pixel stripes at bottom and right, then subdivide by 2.
          render_hline(set.subimage(set.image.x, set.image.y + set.image.sy - 2, set.image.sx - 1, 1), iterator);
          render_vline(set.subimage(set.image.x + set.image.sx - 2, set.image.y, 1, set.image.sy - 1), iterator);
           
          newsize = (set.image.sx - 3) >> 1;
          render_opt(set.subimage(set.image.x + 1, set.image.y + 1, newsize, newsize), iterator);
          render_opt(set.subimage(set.image.x + 1 + newsize, set.image.y + 1, newsize, newsize), iterator);
          render_opt(set.subimage(set.image.x + 1, set.image.y + 1 + newsize, newsize, newsize), iterator);
          render_opt(set.subimage(set.image.x + 1 + newsize, set.image.y + 1 + newsize, newsize, newsize), iterator);
        }
        return;
    }
  }
}

/*
 * Subdivide the given subarea of the Mandelbrot set into quadratic regions that are as large as
 * possible, then render them using the optimized quadratic method.
 */
function subdivide_quadratic(set, iterator) {
  // Perhaps we're already quadratic?
  if (set.image.sx == set.image.sy) {
    render_opt(set, iterator);
    return;
  } else if (set.image.sx > set.image.sy) { // wider than tall
    var subsize = set.image.sy;
    // Render all quadratic areas with size y
    for (var x = set.image.x; x < set.image.x + set.image.sx - subsize; x += subsize) {
      render_opt(set.subimage(x, set.image.y, subsize, subsize), iterator);
    }
    // Recurse over the rest, if necessary.
    if (set.image.sx % subsize) {
      subdivide_quadratic(set.subimage(x, set.image.y, set.image.sx % subsize, set.image.sy), iterator);
    } else { // If there's no rest, then there's still one more left to render.
      render_opt(set.subimage(x, set.image.y, subsize, subsize), iterator);
    }
    return;
  } else { // taller than wide
    var subsize = set.image.sx;
    // Render all quadratic areas with size x
    for (var y = set.image.y; y < set.image.y + set.image.sy - subsize; y += subsize) {
      render_opt(set.subimage(set.image.x, y, subsize, subsize), iterator);
    }
    // Recurse over the rest, if necessary.
    if (set.image.sy % subsize) {
      subdivide_quadratic(set.subimage(set.image.x, y, set.image.sx, set.image.sy % subsize), iterator);
    } else {
      render_opt(set.subimage(set.image.x, y, subsize, subsize), iterator);
    }
    return;
  }
}

/*
 * Fill the specified subimage with mirrored values from above the subimage.
 * Used to speed up situations where the image range crosses the 0 imaginary border,
 * since we know the Mandelbrot set is symmetric.
 * We assume that the upper part of the image is large enough to supply the
 * part to fill with values, i.e. y - sy is never smaller than 0.
 */
function mirror_image(img) {
  var src = img.startpos - img.size;
  var dst = img.startpos + img.size;

  var x, y;
  for (y = img.y; y < img.y + img.sy; y++) {
    for (x = img.x; x < img.x + img.sx; x++) {
      img.buffer[dst++] = img.buffer[src++];
    }
    dst -= img.sx; // Back to start of x.
    dst += img.size; // Next line
    src -= img.sx; // Back to start of x.
    src -= img.size; // Previous line.
  }
  return;
}

/*
 * Subdivide the given area of the mandelbrot set into zones, apply a different, optimal
 * optimization setting to that zones.
 * The idea here is to save detection times for zones that are known to not profit from that
 * detection, and to exploit mirroring opportunities.
 */
function render_adaptive(set) {
  // We will fill a todo-list-array with set descriptions, method and iterators, then
  // go through that todo list.
  var todo = [];
  var newset = null;

  // Render the top part until +1.2i with minimal optimization, as it's very easy anyway.
  newset = set.intersect(null, null, null, 1.2 - set.inc); // -set.inc for a slight overlap to the next section.
  if (newset.image.sy > 0) {
    todo.push({
      set: newset,
      method: "basic",
      iterator: iterate_basic
    });
  }

  // Use more optimization for the part between 1.2i and 0.75i. No need to test for bulbs, though.
  // Use the brain-dead algorithm for everything < 0.75 (re) and the subdivision algorithm for the right part.
  newset = set.intersect(null, 1.2, -0.75 + set.inc, 0.75 - set.inc); // Again, a slight overlap for re, too.
  if (newset.image.sx > 0 && newset.image.sy > 0) {
    todo.push({
      set: newset,
      method: "basic",
      iterator: iterate_basic
    });
  }

  newset = set.intersect(-0.75, 1.2, null, 0.75 - set.inc);
  if (newset.image.sy > 0 && newset.image.sx > 0) {
    todo.push({
      set: newset,
      method: "subdivide",
      iterator: iterate_basic
    });
  }

  // Here's the busy part of the set at [0.75i .. 0]. We'll distinguish 4 horizontal blocks
  // that test differently for the bulbs (or not).
  // The end imaginary value is slightly below 0, so we're sure the pixels that are near zero
  // always get rendered, so mirroring works well. I know it's a hack, but it works :).

  // Leftmost part. No tests for bulbs necessary.
  newset = set.intersect(null, 0.75, -1.25 + set.inc, -set.inc);
  if (newset.image.sy > 0 && newset.image.sx > 0) {
    todo.push({
      set: newset,
      method: "subdivide",
      iterator: iterate_basic
    });
  }

  // Period 2 bulb.
  newset = set.intersect(-1.25, 0.75, -0.75 + set.inc, -set.inc);
  if (newset.image.sy > 0 && newset.image.sx > 0) {
    todo.push({
      set: newset,
      method: "subdivide",
      iterator: iterate_opt_2
    });
  }

  // Period 1 bulb.
  newset = set.intersect(-0.75, 0.75, 0.4 + set.inc, -set.inc);
  if (newset.image.sy > 0 && newset.image.sx > 0) {
    todo.push({
      set: newset,
      method: "subdivide",
      iterator: iterate_opt_1
    });
  }

  // Right of period 1 bulb.
  newset = set.intersect(0.4, 0.75, null, -set.inc);
  if (newset.image.sy > 0 && newset.image.sx > 0) {
    todo.push({
      set: newset,
      method: "subdivide",
      iterator: iterate_opt_1
    });
  }

  // Now that we are at the border of 0i, let's look for mirroring opportunities.
  // If the maximum imaginary value is > 0, then the intersection with the negative
  // imaginary plane yields the mirroring opportunity. We then carry the negative of
  // the maximum imaginary value so we know where to continue rendering.
  //
  // But before we mirror, we need to test if the 0i horizontal line aligns exactly
  // with a pixel line, i.e. the maximum imaginary value can be evenly divided by the pixel increment.
  // (Within a certain measure of accuracy, since we're subject to floating point inaccuracies.)
  // This is usually the case, if the application starts with the center and clicking on pixels
  // is used for navigation.
  var cont = 0;
  if (Math.abs(Math.floor(set.maxim / set.inc + 0.5) - (set.maxim / set.inc)) < 0.000001) {
    newset = set.intersect(null, 0, null, -set.maxim);
    if (newset.image.sy > 0 && newset.image.sx > 0) {
      todo.push({
        set: newset,
        method: "mirror",
        iterator: null
      });
      cont = -set.maxim;
    }
  }

  // Now walk through the same optimitation regions modulo the area that has already been
  // covered by mirroring.

  // 0..-0.75i, 4 regions.
  if (cont > -0.75) {
    // Right of period 1 bulb.
    newset = set.intersect(0.4, cont, null, -0.75 - set.inc);
    if (newset.image.sy > 0 && newset.image.sx > 0) {
      todo.push({
        set: newset,
        method: "subdivide",
        iterator: iterate_opt_1
      });
    }

    // Period 1 bulb.
    newset = set.intersect(-0.75, cont, 0.4 + set.inc, -0.75 - set.inc);
    if (newset.image.sy > 0 && newset.image.sx > 0) {
      todo.push({
        set: newset,
        method: "subdivide",
        iterator: iterate_opt_1
      });
    }

    // Period 2 bulb.
    newset = set.intersect(-1.25, cont, -0.75 + set.inc, -0.75 - set.inc);
    if (newset.image.sy > 0 && newset.image.sx > 0) {
      todo.push({
        set: newset,
        method: "subdivide",
        iterator: iterate_opt_2
      });
    }

    // Leftmost part. No tests for bulbs necessary.
    newset = set.intersect(null, cont, -1.25 + set.inc, -0.75 - set.inc);
    if (newset.image.sy > 0 && newset.image.sx > 0) {
      todo.push({
        set: newset,
        method: "subdivide",
        iterator: iterate_basic
      });
    }
  }

  // -0.75i..-1.2, 2 regions.
  if (cont > -1.2) {
    if (cont > -0.75) { // Adjust the place we continue from if necessary.
      cont = -0.75;
    }

    newset = set.intersect(-0.75, cont, null, -1.2);
    if (newset.image.sy > 0 && newset.image.sx > 0) {
      todo.push({
        set: newset,
        method: "subdivide",
        iterator: iterate_basic
      });
    }

    newset = set.intersect(null, cont, -0.75 + set.inc, -1.2);
    if (newset.image.sx > 0 && newset.image.sy > 0) {
      todo.push({
        set: newset,
        method: "basic",
        iterator: iterate_basic
      });
    }
  }

  // -1.2..
  if (cont > -1.2) {
    newset = set.intersect(null, -1.2, null, null); // -set.inc for a slight overlap to the next section.
  } else {
    newset = set.intersect(null, cont, null, null); // -set.inc for a slight overlap to the next section.
  }
  if (newset.image.sy > 0) {
    todo.push({
      set: newset,
      method: "basic",
      iterator: iterate_basic
    });
  }

  // Complete todo-list.
  for (var i = 0; i < todo.length; i++) {
    switch (todo[i].method) {
      case "basic":
        render_basic(todo[i].set, todo[i].iterator);
        break;

      case "subdivide":
        subdivide_quadratic(todo[i].set, todo[i].iterator);
        break;

      case "mirror":
        mirror_image(todo[i].set.image);
        break;
    }
  }
}
