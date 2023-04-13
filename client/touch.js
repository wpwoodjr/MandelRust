/*
  Class to create a touch interface
  Bill Wood (with a small assist from ChatGPT), March 2023
*/

const TOUCH_NONE = 0;       // no touches active => TOUCHING
const TOUCH_TOUCHING = 1;   // interim state => TAP, DRAG, or PINCH
const TOUCH_TAP = 2;        // single tapping => NONE or DOUBLE_TAP
const TOUCH_DOUBLE_TAP = 3; // double tapping => NONE, DRAG, or PINCH
const TOUCH_DRAG = 4;       // dragging => NONE or TOUCH_TOUCHING
const TOUCH_PINCH = 5;      // pinching => NONE or END_PINCH
const TOUCH_END_PINCH = 6;  // pinch finished => NONE, DRAG, or PINCH
const TOUCH_ERROR = 7;      // error occurred, onTouchEnd was called and waiting for all touches to end => NONE

class Touch {
    constructor(id, options) {
        if (typeof id === "string") {
            this.id = id;
            this.element = document.getElementById(id);
        } else {
            this.element = id;
        }
        options = options || {};
        this.FPS = options.FPS || 60;
        this.onInit = options.onInit || null;
        this.onTouchStart = options.onTouchStart || null;
        this.onTouchEnd = options.onTouchEnd || null;
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
        this.startTime = NaN;
        this.startTouches = [];
        this.endTouches = [];
        this.singleTapTimeout = null;
        this.state = TOUCH_NONE;
        this.element.addEventListener("touchstart", (event) => this.handleTouchStart(event), {passive: false});
        this.element.addEventListener("touchmove", (event) => this.handleTouchMove(event), {passive: false});
        this.element.addEventListener("touchend", (event) => this.handleTouchEnd(event), {passive: false});
        this.element.addEventListener("touchcancel", (event) => this.handleTouchCancel(event), {passive: false});
    }

    handleTouchStart(event) {

        if (this.onInit) {
            this.onInit();
            this.onInit = null;
        }

        // bail out if there are touches outside of element or too many touches in element
        if (event.targetTouches.length !== event.touches.length || event.targetTouches.length > 2) {
            // console.log("touchStart bail out!");
            this.handleTouchCancel();
        }

        // console.log("touchStart:", this.state, event.targetTouches.length);
        switch (this.state) {
            case TOUCH_NONE:
                // console.log("onTouchStart():", event.targetTouches.length);
                if (this.onTouchStart) {
                    this.onTouchStart();
                }
                this.state = TOUCH_TOUCHING;
                break;

            case TOUCH_TOUCHING:
                break;

            case TOUCH_TAP:
                this.tapAbort();
                this.state = TOUCH_DOUBLE_TAP;
                break;

            case TOUCH_DOUBLE_TAP:
                // this can happen if single-tap with one touch then quickly tap with two touches
                this.handleTouchCancel();
                break;

            case TOUCH_DRAG:
                this.dragEnd();
                this.state = TOUCH_TOUCHING;
                break;

            case TOUCH_PINCH:
                console.warn("touch start event and state === PINCH");
                break;

            case TOUCH_END_PINCH:
                break;

            case TOUCH_ERROR:
                break;
        }
        this.startTouches = this.copyTouches(event.targetTouches);
    }

    // Handle touch move event
    handleTouchMove(event) {

        // bail out if there are touches outside of element
        if (event.targetTouches.length !== event.touches.length) {
            // console.log("touchMove bail out!");
            this.handleTouchCancel();
        }

        switch (this.state) {
            case TOUCH_NONE:
                // this can happen when touch is initiated outside the element
                // console.warn("touch move event and state === NONE");
                break;

            case TOUCH_DOUBLE_TAP:   // waiting for second tap touch end, but move happened instead
            case TOUCH_END_PINCH:    // pinch ended with one touch left, now dragging or pinching again
            case TOUCH_TOUCHING:     // one or two touches detected, now dragging or pinching
                // check for start of one touch drag
                if (event.targetTouches.length === 1) {
                    // console.log("start drag");
                    this.state = TOUCH_DRAG;
                    if (this.onDragStart) {
                        this.onDragStart();
                    }
                    this.endTouches = this.copyTouches(event.targetTouches);
                    this.startTime = Date.now();
                    if (this.onDragMove) {
                        event.preventDefault();
                        this.onDragMove(this.startTouches[0].clientX, this.startTouches[0].clientY,
                            this.endTouches[0].clientX, this.endTouches[0].clientY);
                    }

                // check if there are two touches for pinch gesture
                } else if (event.targetTouches.length === 2) {
                    // console.log("start pinch");
                    this.state = TOUCH_PINCH;
                    if (this.onPinchStart) {
                        this.onPinchStart();
                    };
                    this.endTouches = this.copyTouches(event.targetTouches);
                    this.startTime = Date.now();
                    if (this.onPinchMove) {
                        event.preventDefault();
                        this.onPinchMove(
                            [this.startTouches[0].clientX, this.startTouches[0].clientY,
                                this.startTouches[1].clientX, this.startTouches[1].clientY],
                            [this.endTouches[0].clientX, this.endTouches[0].clientY,
                                this.endTouches[1].clientX, this.endTouches[1].clientY]);
                    }
                }
                break;

            case TOUCH_TAP:
                console.warn("touch move event and state === TAP");
                break;

            case TOUCH_DRAG:
                if (this.onDragMove) {
                    event.preventDefault();
                    const elapsed = Date.now() - this.startTime;
                    if (elapsed >= 1000/this.FPS) {
                        this.startTime = Date.now();
                        this.endTouches = this.copyTouches(event.targetTouches);
                        this.onDragMove(this.startTouches[0].clientX, this.startTouches[0].clientY,
                            this.endTouches[0].clientX, this.endTouches[0].clientY);
                    }
                }
                break;

            case TOUCH_PINCH:
                if (this.onPinchMove) {
                    event.preventDefault();
                    // check length because Firefox doesn't always call handleTouchEnd before here
                    if (event.targetTouches.length === 2) {
                        const elapsed = Date.now() - this.startTime;
                        if (elapsed >= 1000/this.FPS) {
                            this.startTime = Date.now();
                            this.endTouches = this.copyTouches(event.targetTouches);
                            this.onPinchMove(
                                [this.startTouches[0].clientX, this.startTouches[0].clientY,
                                    this.startTouches[1].clientX, this.startTouches[1].clientY],
                                [this.endTouches[0].clientX, this.endTouches[0].clientY,
                                    this.endTouches[1].clientX, this.endTouches[1].clientY]);
                        }
                    }
                }
                break;

            case TOUCH_ERROR:
                break;
        }
    }

    // Handle touch end event
    handleTouchEnd(event) {

        // console.log("touchEnd:", this.state, this.startTouches.length, event.targetTouches.length);
        let doOnTouchEnd = false;
        switch (this.state) {
            case TOUCH_NONE:
                // this can happen when touch is initiated outside the element
                // console.warn("touch end event and state === NONE");
                break;

            case TOUCH_TOUCHING:
                // check for multi-touch tap
                if (this.startTouches.length > 1) {
                    this.handleTouchCancel();

                // Set timeout for a single tap gesture
                } else {
                    // prevent emulated mouse dblclick
                    this.onDoubleTap && event.preventDefault();
                    const clientX = event.changedTouches[0].clientX;
                    const clientY = event.changedTouches[0].clientY;
                    this.singleTapTimeout = setTimeout(() => {
                        this.tapAbort();
                        if (this.onSingleTap) {
                            this.onSingleTap(clientX, clientY);
                        }
                        if (this.onTouchEnd) {
                            // console.log("onTouchEnd() from single tap");
                            this.onTouchEnd();
                        }
                        this.state = TOUCH_NONE;
                    }, 300);
                    this.state = TOUCH_TAP;
                }
                break;

            case TOUCH_TAP:
                console.warn("touch end event and state === TAP");
                break;

            case TOUCH_DOUBLE_TAP:
                // check for multi-touch tap
                if (this.startTouches.length > 1) {
                    this.handleTouchCancel();

                } else {
                    if (this.onDoubleTap) {
                        // prevent emulated mouse dblclick (Chrome on ios seems to need this here)
                        event.preventDefault();
                        this.onDoubleTap(event.changedTouches[0].clientX, event.changedTouches[0].clientY);
                    }
                    doOnTouchEnd = true;
                }
                break;

            case TOUCH_DRAG:
                this.dragEnd();
                doOnTouchEnd = true;
                break;

            case TOUCH_PINCH:
                this.pinchEnd();
                // still dragging?
                if (event.targetTouches.length === 1) {
                    // console.log("start drag from pinch");
                    this.startTouches = this.copyTouches(event.targetTouches);
                    this.endTouches = this.copyTouches(event.targetTouches);
                    this.state = TOUCH_END_PINCH;
                } else {
                    doOnTouchEnd = true;
                }
                break;

            case TOUCH_END_PINCH:
                doOnTouchEnd = true;
                break;

            case TOUCH_ERROR:
                if (event.targetTouches.length === 0) {
                    this.state = TOUCH_NONE;
                }
                break;
        }

        if (doOnTouchEnd && this.onTouchEnd) {
            // console.log("onTouchEnd() from handleTouchEnd");
            this.onTouchEnd();
            this.state = TOUCH_NONE;
        }
    }
 
    // Handle touch cancel event
    handleTouchCancel(event) {
        // console.log("touch canceled from", (new Error()).stack.split("\n")[2].trim().split(" ")[1], this.state);
        // console.log("touch canceled", this.state);
        if (this.state === TOUCH_DRAG) {
            this.dragEnd();
        } else if (this.state === TOUCH_PINCH) {
            this.pinchEnd();
        } else if (this.state === TOUCH_TAP) {
            this.tapAbort();
        }

        if (this.onTouchEnd && this.state !== TOUCH_NONE && this.state !== TOUCH_ERROR) {
            // console.log("onTouchEnd() from handleTouchCancel");
            this.onTouchEnd();
        }

        // if event is present then handleTouchCancel was called by the browser
        if (event && event.targetTouches.length === 0) {
            this.state = TOUCH_NONE;
        } else if (this.state !== TOUCH_NONE) {
            // wait for touches to end
            this.state = TOUCH_ERROR;
        }
    }

    dragEnd() {
        if (this.onDragEnd) {
            this.onDragEnd(this.startTouches[0].clientX, this.startTouches[0].clientY,
                this.endTouches[0].clientX, this.endTouches[0].clientY);
        }
    }

    pinchEnd() {
        if (this.onPinchEnd) {
            this.onPinchEnd(
                [this.startTouches[0].clientX, this.startTouches[0].clientY,
                    this.startTouches[1].clientX, this.startTouches[1].clientY],
                [this.endTouches[0].clientX, this.endTouches[0].clientY,
                this.endTouches[1].clientX, this.endTouches[1].clientY]);
        }
    }

    tapAbort() {
        if (this.singleTapTimeout) {
            clearTimeout(this.singleTapTimeout);
            this.singleTapTimeout = null;
        }
    }

    // Firefox requires that we copy what we need
    copyTouches(touches) {
        let r = [];
        for (let t of touches) {
            r.push({
                identifier: t.identifier,
                clientX: t.clientX,
                clientY: t.clientY
            });
        }
        return r;
    }
}
