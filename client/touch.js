/*
  Class to create a touch interface
  Bill Wood (with a little assist from ChatGPT), March 2023
*/

class Touch {
    constructor(id, options) {
        this.id = id;
        this.element = document.getElementById(id);
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
        this.element.addEventListener("touchstart", (event) => this.handleTouchStart(event), {passive: false});
        this.element.addEventListener("touchmove", (event) => this.handleTouchMove(event), {passive: false});
        this.element.addEventListener("touchend", (event) => this.handleTouchEnd(event), {passive: false});
        this.element.addEventListener("touchcancel", (event) => this.handleTouchCancel(event), {passive: false});
        this.NONE = 0;
        this.DRAG = 1;
        this.TAP = 2;
        this.DOUBLE_TAP = 3;
        this.PINCH = 4;
        this.END_PINCH = 5;
        this.TOUCHING = 6;
        this.state = this.NONE;
    }

    handleTouchStart(event) {
        console.log("touch start:", event.targetTouches.length);

        if (this.onInit) {
            this.onInit();
            this.onInit = null;
        }

        switch (this.state) {
            case this.NONE:
                if (this.onTouchStart) {
                    this.onTouchStart();
                }
                this.state = this.TOUCHING;
                break;

            case this.DRAG:
                this.dragEnd(event.targetTouches);
                this.state = this.TOUCHING;
                break;

            case this.TAP:
                this.tapEnd();
                this.state = this.DOUBLE_TAP;
                break;

            case this.DOUBLE_TAP:
                break;

            case this.PINCH:
                this.pinchEnd();
                this.state = this.TOUCHING;
                break;

            case this.END_PINCH:
                break;

            case this.TOUCHING:
                break;
        }

        this.startTouches = this.copyTouches(event.targetTouches);
    }

    // Handle touch move event
    handleTouchMove(event) {

        switch (this.state) {
            case this.NONE:
                console.warn("touch move event and state === NONE");
                break;

            case this.DRAG:
                if (event.touches.length !== event.targetTouches.length) {
                    this.dragEnd(event.targetTouches);
                } else if (this.onDragMove) {
                    const elapsed = Date.now() - this.startTime;
                    if (elapsed >= 1000/this.FPS) {
                        this.startTime = Date.now();
                        const id = this.startTouches[0].identifier;
                        for (const e of event.changedTouches) {
                            if (e.identifier === id) {
                                this.onDragMove(this.startTouches[0].clientX, this.startTouches[0].clientY, e.clientX, e.clientY);
                                break;
                            }
                        }
                    }
                }
                break;

            case this.TAP:
                console.warn("touch move event and state === TAP");
                break;

            case this.DOUBLE_TAP:
                console.warn("touch move event and state === DOUBLE_TAP");
                break;

            case this.PINCH:
                if (event.touches.length !== event.targetTouches.length) {
                    this.pinchEnd();
                // check length because Firefox doesn't always call handleTouchEnd before here
                } else if (event.targetTouches.length === 2) {
                    this.endTouches = this.copyTouches(event.targetTouches);
                    if (this.onPinchMove) {
                        const elapsed = Date.now() - this.startTime;
                        if (elapsed >= 1000/this.FPS) {
                            this.startTime = Date.now();
                            this.onPinchMove(
                                [this.startTouches[0].clientX, this.startTouches[0].clientY,
                                    this.startTouches[1].clientX, this.startTouches[1].clientY],
                                [event.targetTouches[0].clientX, event.targetTouches[0].clientY,
                                    event.targetTouches[1].clientX, event.targetTouches[1].clientY]);
                        }
                    }
                }
                break;

            case this.END_PINCH:
                // break;

            case this.TOUCHING:
                // check for start of one touch drag
                if (event.targetTouches.length === 1) {
                    console.log("start drag");
                    this.state = this.DRAG;
                    if (this.onDragStart) {
                        this.onDragStart();
                    }
                    this.startTime = Date.now();
                    if (this.onDragMove) {
                        this.onDragMove(this.startTouches[0].clientX, this.startTouches[0].clientY,
                            event.targetTouches[0].clientX, event.targetTouches[0].clientY);
                    }

                // check if there are two touches for pinch gesture
                } else if (event.targetTouches.length === 2) {
                    console.log("start pinch");
                    this.state = this.PINCH;
                    if (this.onPinchStart) {
                        this.onPinchStart();
                    };
                    this.endTouches = this.copyTouches(event.targetTouches);
                    this.startTime = Date.now();
                    if (this.onPinchMove) {
                        this.onPinchMove(
                            [this.startTouches[0].clientX, this.startTouches[0].clientY,
                                this.startTouches[1].clientX, this.startTouches[1].clientY],
                            [event.targetTouches[0].clientX, event.targetTouches[0].clientY,
                                event.targetTouches[1].clientX, event.targetTouches[1].clientY]);
                    }
                }
            break;
        }

        if (this.state !== this.NONE) {
            event.preventDefault();
        }
    }

    // Handle touch end event
    handleTouchEnd(event) {

        switch (this.state) {
            case this.NONE:
                console.warn("touch end event and state === NONE");
                break;

            case this.DRAG:
                this.dragEnd(event.changedTouches);
                break;

            case this.TAP:
                console.warn("touch end event and state === TAP");
                break;

            case this.DOUBLE_TAP:
                this.state = this.NONE;
                if (this.onDoubleTap) {
                    // prevent emulated mouse dblclick (doesn't work in ios)
                    event.preventDefault();
                    this.onDoubleTap(event.changedTouches[0].clientX, event.changedTouches[0].clientY);
                }
                this.startTouches = [];
                break;

            case this.PINCH:
                this.pinchEnd();
                // still dragging?
                if (event.targetTouches.length === 1) {
                    // console.log("start drag from pinch");
                    this.state = this.END_PINCH;
                    this.startTouches = this.copyTouches(event.targetTouches);
                }
                break;

            case this.END_PINCH:
                this.state = this.NONE;
                this.startTouches = [];
                break;

            case this.TOUCHING:
                // Set timeout for a single tap gesture
                this.state = this.TAP;
                const clientX = event.changedTouches[0].clientX;
                const clientY = event.changedTouches[0].clientY;
                this.singleTapTimeout = setTimeout(() => {
                    this.tapEnd();
                    if (this.onSingleTap) {
                        this.onSingleTap(clientX, clientY);
                    }
                    if (this.onTouchEnd) {
                        console.log("touchEnd");
                        this.onTouchEnd();
                    }
                }, 350);
                break;
        }

        if (this.state === this.NONE && this.onTouchEnd) {
            console.log("touchEnd");
            this.onTouchEnd();
        }
    }
 
    // Handle touch cancel event
    handleTouchCancel(event) {
        // console.log("touch canceled");
        if (this.state === this.DRAG) {
            this.dragEnd(event.changedTouches);
        } else if (this.state === this.PINCH) {
            this.pinchEnd();
        } else if (this.state === this.TAP) {
            this.tapEnd();
        } else if (this.state === this.DOUBLE_TAP || this.state === this.END_PINCH || this.state === this.TOUCHING) {
            this.state = this.NONE;
            this.startTouches = [];
        }
        if (this.onTouchEnd) {
            console.log("touchEnd from touchCancel");
            this.onTouchEnd();
        }
    }

    dragEnd(touches) {
        const id = this.startTouches[0].identifier;
        for (const t of touches) {
            if (t.identifier === id) {
                this.state = this.NONE;///??? only end if find id?
                if (this.onDragEnd) {
                    this.onDragEnd(this.startTouches[0].clientX, this.startTouches[0].clientY, t.clientX, t.clientY);
                }
                this.startTouches = [];
                return;
            }
        }
    }

    pinchEnd() {
        this.state = this.NONE;
        if (this.onPinchEnd) {
            this.onPinchEnd(
                [this.startTouches[0].clientX, this.startTouches[0].clientY,
                    this.startTouches[1].clientX, this.startTouches[1].clientY],
                [this.endTouches[0].clientX, this.endTouches[0].clientY,
                this.endTouches[1].clientX, this.endTouches[1].clientY]);
        }
        this.startTouches = [];
        this.endTouches = [];
    }

    tapEnd() {
        this.state = this.NONE;
        if (this.singleTapTimeout) {
            clearTimeout(this.singleTapTimeout);
            this.singleTapTimeout = null;
        }
        this.startTouches = [];
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
