## David Eck
* August, 2019. Added Palette Editor.
* January, 2020. Improved Palette Editor.
* April, 2020.  Cleaned up standard palettes, removed a few, added Dark Colors palette.
* May, 2020.  Change maximum image width and height from 2500 to 10000 (but that doesn't necessarily mean very large images will work on every device).
* October, 2021.  Fixed a bug in startJob() and startSecondPass() that showed up when the change in y-value from one line to the next is very small.  This was done previously by successive subtraction, which introduced errors.  Thanks to Robert Munafo for finding and fixing the bug.
* August, 2022.  Use localStorage to save workerCount between sessions.

## Bill Wood
### Jul/Aug 2019
* Rewrote job processing to be client/server
* Fixed y-value issue as above
* Changed HP_CUTOFF from 16 to 15 to avoid artifacts at the limits of f64 precision
* Added "High Precision" toggle
* Added rows/second display

### Jan/Feb 2023
* Rewrote JavaScript server in Rust to get 10 times speedup in high precision computations
* When zooming, the zoomed area is displayed using image interpolation while calculations are done
* Left, right, up, and down arrow key bindings shift the image appropriately; page down and page up zoom in and out
* Max image width/height is now 7680 (8K)
* Fixed off by factor of 10 error in zoom out by 10,000 and 100,000
* Added option to run calculations locally on the browser in JavaScript, or on the backend Rust server
* Added undo (ctrl-z) and redo (ctrl-shift-z and ctrl-y) key bindings
* Fixed small bug where dx value was being used to increment yval instead of dy value in HP calculations
* Rewrote Javascript code for browser calculations in Rust, compiled to WASM, for 7X speedup