/*
  Class to create a touch interface
  Bill Wood (with a little assist from ChatGPT), March 2023
*/

class Touch {
    constructor(id, options) {
        this.id = id;
        this.element = document.getElementById(id);
        options = options || {};
        this.onInit = options.onInit || null;
        this.onTouchStart = options.onTouchStart || null;
        this.onDragStart = options.onDragStart || null;
        this.onDragMove = options.onDragMove || null;
        this.onDragEnd = options.onDragEnd || null;
        this.onSingleTap = options.onSingleTap || null;
        this.onDoubleTap = options.onDoubleTap || null;
        this.onPinchStart = options.onPinchStart || null;
        this.onPinchMove = options.onPinchMove || null;
        this.onPinchEnd = options.onPinchEnd || null;
        this.allowDocumentTouches = options.allowDocumentTouches || false;
        this.data = options.data || {};

        // Variables to store touch positions and state
        this.startTouches = [];
        this.startTimer = null;
        this.isDragging = false;
        this.isPinching = false;
        this.isTapping = false;
        this.singleTapTimeout = null;
        this.element.addEventListener("touchstart", (event) => this.handleTouchStart(event), {passive: false});
        this.element.addEventListener("touchmove", (event) => this.handleTouchMove(event), {passive: false});
        this.element.addEventListener("touchend", (event) => this.handleTouchEnd(event), {passive: false});
        this.element.addEventListener("touchcancel", (event) => this.handleTouchCancel(event), {passive: false});
        if (! this.allowDocumentTouches) {
            document.addEventListener('touchstart', (event) => this.handleDocumentTouchEvent(event), {passive: false});
            // document.addEventListener('touchmove', (event) => this.handleDocumentTouchEvent(event), {passive: false});
            // document.addEventListener('touchend', (event) => this.handleDocumentTouchEvent(event), {passive: false});
        }
    }

    // prevent touches on the document from interfering with element touches
    handleDocumentTouchEvent(event) {
        if (this.startTouches.length !== 0) {
            event.preventDefault();
        }
    }

    handleTouchStart(event) {
        // console.log("touch start");
        if (this.onInit) {
            this.onInit();
            this.onInit = null;
        }

        if (this.onTouchStart) {
            this.onTouchStart();
        }

        // Store the touch positions if touch events aren't happening outside the target element
        // and dragging, pinching, or tapping have not started yet
        // if (event.touches.length === event.targetTouches.length
        //     && ! (this.isDragging || this.isPinching || this.isTapping)) {
        if (! (this.isDragging || this.isPinching || this.isTapping)) {
            this.startTouches = this.copyTouches(event.targetTouches);
            // this.startTimer = Date.now();
        }
    }
      
    // Handle touch move event
    handleTouchMove(event) {
        // const elapsed = Date.now() - this.startTimer;
        // // console.log("touch move:", elapsed);
        // if (elapsed < 125) {
        //     return;
        // }

        // check for continue drag
        if (this.isDragging) {
            if (this.onDragMove) {
                const id = this.startTouches[0].identifier;
                for (const e of event.changedTouches) {
                    if (e.identifier === id) {
                        this.onDragMove(e.clientX, e.clientY);
                        break;
                    }
                }
            }

        // check for continue two finger pinching///???check ids?
        } else if (this.isPinching) {
            if (this.onPinchMove) {
                this.onPinchMove(event.targetTouches[0].clientX, event.targetTouches[0].clientY,
                    event.targetTouches[1].clientX, event.targetTouches[1].clientY);
            }

        // check for start of one touch drag
        } else if (event.targetTouches.length === 1 && this.startTouches.length === 1) {
            this.isDragging = true;
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
            event.preventDefault();
        }
    }

    // Handle touch end event
    handleTouchEnd(event) {
        // const elapsed = Date.now() - this.startTimer;
        // console.log("touch end:", elapsed);

        if (this.isDragging) {
            this.dragEnd(event);

        } else if (this.isPinching) {
            this.pinchEnd(event);

        // Check for tap gesture
        } else if (event.targetTouches.length === 0 && this.startTouches.length === 1) {
            const startX = this.startTouches[0].clientX;
            const startY = this.startTouches[0].clientY;

            // If the single tap setTimeout hasn't fired, it's a double tap gesture
            if (this.singleTapTimeout != null) {
                this.tapEnd();
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
                    this.tapEnd();
                    if (this.onSingleTap) {
                        this.onSingleTap(startX, startY);
                    }
                }, 300);
            }
        } else {
            this.startTouches = [];
        }
    }
 
    // Handle touch cancel event
    handleTouchCancel(event) {
        // console.log("touch canceled");
        if (this.isDragging) {
            this.dragEnd(event);
        } else if (this.isPinching) {
            this.pinchEnd(event);
        } else if (this.isTapping) {
            this.tapEnd();
        }
    }

    dragEnd(event) {
        const id = this.startTouches[0].identifier;
        for (const e of event.changedTouches) {
            if (e.identifier === id) {
                this.isDragging = false;
                this.startTouches = [];
                if (this.onDragEnd) {
                    this.onDragEnd();
                }
                return;
            }
        }
    }

    pinchEnd(event) {
        const id0 = this.startTouches[0].identifier;
        const id1 = this.startTouches[1].identifier;
        for (const e of event.changedTouches) {
            // end if one of the original fingers was lifted
            if (e.identifier === id0 || e.identifier === id1) {
                this.isPinching = false;
                this.startTouches = [];
                if (this.onPinchEnd) {
                    this.onPinchEnd();
                }
                return;
            }
        }
    }

    tapEnd() {
        this.isTapping = false;
        this.startTouches = [];
        if (this.singleTapTimeout) {
            clearTimeout(this.singleTapTimeout);
            this.singleTapTimeout = null;
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
