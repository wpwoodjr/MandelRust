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
        this.onDragStart = options.onDragStart || null;
        this.onDragMove = options.onDragMove || null;
        this.onDragEnd = options.onDragEnd || null;
        this.onSingleTap = options.onSingleTap || null;
        this.onDoubleTap = options.onDoubleTap || null;
        this.onPinchStart = options.onPinchStart || null;
        this.onPinchMove = options.onPinchMove || null;
        this.onPinchEnd = options.onPinchEnd || null;
        this.data = options.data || {};

        // Variables to store touch positions and state
        this.init = false;
        this.startTouches = [];
        this.isDragging = false;
        this.dragId = NaN;
        this.isPinching = false;
        this.isTapping = false;
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
        } else if (this.isDragging || this.isPinching || this.isTapping) {
            return;
        }

        // Store the initial touch positions
        this.startTouches = this.copyTouches(event.targetTouches);
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
                        break;
                    }
                }
            }

        // check for continue two finger pinching
        } else if (this.isPinching) {
            if (this.onPinchMove) {
                if (event.targetTouches.length === 2) {
                    this.onPinchMove(event.targetTouches[0].clientX, event.targetTouches[0].clientY,
                        event.targetTouches[1].clientX, event.targetTouches[1].clientY);
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

        // check if there are two touches for pinch gesture
        } else if (event.targetTouches.length === 2 && this.startTouches.length === 2) {
            this.isPinching = true;
            if (this.onPinchStart) {
                this.onPinchStart(this.startTouches[0].clientX, this.startTouches[0].clientY,
                    this.startTouches[1].clientX, this.startTouches[1].clientY);
            };
            if (this.onPinchMove) {
                this.onPinchMove(event.targetTouches[0].clientX, event.targetTouches[0].clientY,
                    event.targetTouches[1].clientX, event.targetTouches[1].clientY);
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
            this.dragEnd(event);

        } else if (this.isPinching) {
            if (event.touches.length === 0) {
                this.isPinching = false;
                if (this.onPinchEnd) {
                    this.onPinchEnd();
                }
            }

        // Check for tap gesture
        } else if (event.targetTouches.length === 0 && this.startTouches.length === 1) {
            const startX = this.startTouches[0].clientX;
            const startY = this.startTouches[0].clientY;

            // If the single tap setTimeout hasn't fired, it's a double tap gesture
            if (this.singleTapTimeout != null) {
                this.isTapping = false;
                clearTimeout(this.singleTapTimeout);
                this.singleTapTimeout = null;
                if (this.onDoubleTap) {
                    // prevent emulated mouse dblclick
                    event.preventDefault();
                    const endX = event.changedTouches[0].clientX;
                    const endY = event.changedTouches[0].clientY;
                    // console.log(startX,startY,endX,endY);
                    this.onDoubleTap((startX + endX)/2, (startY + endY)/2);
                }
            }

            // Otherwise, set timeout for a single tap gesture
            else {
                this.isTapping = true;
                this.singleTapTimeout = setTimeout(() => {
                    this.isTapping = false;
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
            this.dragEnd(event);
        } else if (this.isPinching) {
            this.isPinching = false;
            if (this.onPinchEnd) {
                this.onPinchEnd();
            }
        } else if (this.isTapping) {
            this.isTapping = false;
            if (this.singleTapTimeout) {
                clearTimeout(this.singleTapTimeout);
                this.singleTapTimeout = null;
            }
        }
    }

    dragEnd(event) {
        for (const e of event.changedTouches) {
            if (e.identifier === this.dragId) {
                this.isDragging = false;
                this.dragId = NaN;
                if (this.onDragEnd) {
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
