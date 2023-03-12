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
        this.onDoubleTap = options.onDoubleTap || null;
        this.data = options.data || {};

        // Variables to store touch positions and state
        this.startTouches = null;
        this.lastTapTime = 0;
        this.pinchTouches = null;
        this.isDragging = false;

        this.parentElement.addEventListener("touchstart", (event) => this.handleTouchStart(event));
        this.parentElement.addEventListener("touchmove", (event) => this.handleTouchMove(event));
        this.parentElement.addEventListener("touchend", (event) => this.handleTouchEnd(event));

        if (this.onInit) {
            this.onInit();
        }
    }

    handleTouchStart(event) {
        // Store the initial touch position
        this.startTouches = event.targetTouches;
      
        // Reset pinchTouches array
        this.pinchTouches = null;
      }
      
    // Handle touch move event
    handleTouchMove(event) {
        // Prevent scrolling on the page
        event.preventDefault();
        // Check if there are two touches for pinch gesture
        if (event.targetTouches.length === 2) {
            // Store the pinch touch positions
            this.pinchTouches = event.touches;
        } else if (event.targetTouches.length === 1 || this.isDragging) {
            if (! this.isDragging && this.onDragStart) {
                this.onDragStart(this.startTouches[0].clientX, this.startTouches[0].clientY);
                this.isDragging = true;
            }
            if (this.onDragMove) {
                this.onDragMove(event.targetTouches[0].clientX, event.targetTouches[0].clientY);
            }
        }
    }

    // Handle touch end event
    handleTouchEnd(event) {
        // Check for pinch gesture
        if (this.isDragging) {
            this.isDragging = false;
            if (this.onDragEnd) {
                this.onDragEnd();
            }
            return;
        }
        if (event.touches.length === 0 && this.pinchTouches.length === 2) {
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

        // Check for tap gesture
        if (event.touches.length === 0) {
            // Calculate the distance between start and end touches
            const startX = this.startTouches[0].clientX;
            const startY = this.startTouches[0].clientY;
            const endX = event.changedTouches[0].clientX;
            const endY = event.changedTouches[0].clientY;
            const distance = Math.sqrt(Math.pow(endX - startX, 2) + Math.pow(endY - startY, 2));

            // Calculate the time since the last tap
            const currentTime = new Date().getTime();
            const timeSinceLastTap = currentTime - this.lastTapTime;

            // If the distance is less than a threshold value and the time since the last tap is less than a threshold value, it's a double tap gesture
            if (distance < 10 && timeSinceLastTap < 300) {
            console.log("Double tap gesture detected");
            }
            // Otherwise, it's a single tap gesture
            else {
            console.log("Tap gesture detected");
            }

            // Store the current tap time
            this.lastTapTime = currentTime;
        }

        // Check for drag gesture
        if (event.touches.length === 1 && this.pinchTouches.length === 0) {
            console.log("Drag gesture detected");
        }
    }
}
