/*
  Class to create a touch interface
  Bill Wood (with a little assist from ChatGPT), March 2023
*/

class Touch {
    constructor(id, options) {
        this.id = id;
        this.parentElement = document.getElementById(id);
        options = options || {};
        this.onInit = options.onInit || null;
        this.onTouchStart = options.onTouchStart || null;
        this.onDragStart = options.onDragStart || null;
        this.onDragMove = options.onDragMove || null;
        this.onDragEnd = options.onDragEnd || null;
        this.onDragCancel = options.onDragCancel || null;
        this.onSingleTap = options.onSingleTap || null;
        this.onDoubleTap = options.onDoubleTap || null;
        this.onPinchStart = options.onPinchStart || null;
        this.onPinchMove = options.onPinchMove || null;
        this.onPinchEnd = options.onPinchEnd || null;
        this.onPinchCancel = options.onPinchCancel || null;
        this.data = options.data || {};

        // Variables to store touch positions and state
        this.startTouches = [];
        this.isDragging = false;
        this.dragId = NaN;
        this.isPinching = false;
        this.pinchTouches = [];
        this.init = false;
        this.singleTapTimeout = null;
        this.parentElement.addEventListener("touchstart", (event) => this.handleTouchStart(event));
    }

    handleTouchStart(event) {
        // console.log("touch start");
        if (! this.init) {
            this.init = true;
            this.parentElement.addEventListener("touchmove", (event) => this.handleTouchMove(event));
            this.parentElement.addEventListener("touchend", (event) => this.handleTouchEnd(event));
            this.parentElement.addEventListener("touchcancel", (event) => this.handleTouchCancel(event));
            if (this.onInit) {
                this.onInit();
            }
        } else if (this.isDragging || this.isPinching) {
            return;
        }

        // Store the initial touch position
        this.startTouches = this.copyTouches(event.targetTouches);

        // Reset pinchTouches array
        this.pinchTouches = [];

        if (this.onTouchStart) {
            this.onTouchStart(event);
        }
      }
      
    // Handle touch move event
    handleTouchMove(event) {
        // console.log("move");

        // check for continue drag
        if (this.isDragging) {
            if (this.onDragMove) {
                for (const e of event.changedTouches) {
                    if (e.identifier === this.dragId) {
                        this.onDragMove(e.clientX, e.clientY);
                    }
                }
            }

        // check for start of one touch drag
        } else if (event.targetTouches.length === 1 && this.startTouches.length === 1) {
            this.isDragging = true;
            this.dragId = this.startTouches[0].identifier;
            if (this.onDragStart) {
                this.onDragStart(this.startTouches[0].clientX, this.startTouches[0].clientY);
            };
            if (this.onDragMove) {
                this.onDragMove(event.targetTouches[0].clientX, event.targetTouches[0].clientY);
            }

        // Check if there are two touches for pinch gesture
        } else if (event.targetTouches.length === 2 && this.startTouches.length === 2) {
            // Store the pinch touch positions
            this.pinchTouches = event.targetTouches;

            // Calculate the distance between pinch touches
            const pinchStartX1 = this.pinchTouches[0].clientX;
            const pinchStartY1 = this.pinchTouches[0].clientY;
            const pinchStartX2 = this.pinchTouches[1].clientX;
            const pinchStartY2 = this.pinchTouches[1].clientY;
            const pinchStartDistance = Math.sqrt(Math.pow(pinchStartX2 - pinchStartX1, 2) + Math.pow(pinchStartY2 - pinchStartY1, 2));

            const pinchEndX1 = event.changedTouches[0].clientX;
            const pinchEndY1 = event.changedTouches[0].clientY;
            const pinchEndX2 = event.changedTouches[1].clientX;
            const pinchEndY2 = event.changedTouches[1].clientY;
            const pinchEndDistance = Math.sqrt(Math.pow(pinchEndX2 - pinchEndX1, 2) + Math.pow(pinchEndY2 - pinchEndY1, 2));

            // If the distance between pinch touches has increased, it's a pinch out gesture
            if (pinchEndDistance > pinchStartDistance) {
                console.log("Pinch out gesture detected");
            }
            // If the distance between pinch touches has decreased, it's a pinch in gesture
            else if (pinchEndDistance < pinchStartDistance) {
                console.log("Pinch in gesture detected");
            }
        }

        if (this.isDragging || this.isPinching) {
            // Prevent scrolling on the page
            event.preventDefault();
        }
    }

    // Handle touch end event
    handleTouchEnd(event) {
        // console.log("touch end", event);

        if (this.isDragging) {
            this.dragEnd(event, false);

        } else if (this.isPinching) {
            if (event.touches.length === 0) {
                this.isPinching = false;
                if (this.onPinchEnd) {
                    this.onPinchEnd();
                }
            }

        // Check for tap gesture
        } else if (event.targetTouches.length === 0 && this.startTouches.length === 1) {
            // Calculate the distance between start and end touches
            const startX = this.startTouches[0].clientX;
            const startY = this.startTouches[0].clientY;
            const endX = event.changedTouches[0].clientX;
            const endY = event.changedTouches[0].clientY;
            const distanceSq = Math.pow(endX - startX, 2) + Math.pow(endY - startY, 2);

            // If the distance is less than a threshold value and the single tap setTimeout hasn't fired, it's a double tap gesture
            if (distanceSq < 15*15 && this.singleTapTimeout != null) {
                clearTimeout(this.singleTapTimeout);
                this.singleTapTimeout = null;
                if (this.onDoubleTap) {
                    // prevent emulated mouse dblclick
                    event.preventDefault();
                    this.onDoubleTap((startX + endX)/2, (startY + endY)/2);
                }
            }

            // Otherwise, set timeout for a single tap gesture
            else {
                this.singleTapTimeout = setTimeout(() => {
                    this.singleTapTimeout = null;
                    if (this.onSingleTap) {
                        this.onSingleTap(startX, startY);
                    }
                }, 350);
            }
        }
    }
 
    // Handle touch cancel event
    handleTouchCancel(event) {
        // console.log("touch canceled");
        if (this.isDragging) {
            this.dragEnd(event, true);
        } else if (this.isPinching && this.onPinchCancel) {
            this.onPinchCancel();
        }
    }

    dragEnd(event, onCancel) {
        for (const e of event.changedTouches) {
            if (e.identifier === this.dragId) {
                this.isDragging = false;
                this.dragId = NaN;
                this.startTouches = [];
                if (onCancel && this.onDragCancel) {
                    this.onDragCancel();
                // call onDragEnd if no onDragCancel
                } else if (this.onDragEnd) {
                    this.onDragEnd();
                }
                return;
            }
        }
    }

    // Firefox requires that we copy what we need
    copyTouches(touches) {
        let r = [];
        for (let i = 0; i < touches.length; i++) {
            r[i] = {
                identifier: touches[i].identifier,
                clientX: touches[i].clientX,
                clientY: touches[i].clientY
            };
        }
        return r;
    }
}
