class ProgressCircle extends HTMLElement {
    constructor(id, color, backgroundColor, radius, counterClockwise = false) {
        super();
        this.color = color;
        this.radius = radius;
        this.counterClockwise = counterClockwise;
        this.progressValueClass = this.counterClockwise
            ? 'progress-value reverse'
            : 'progress-value';
        this.rotation = this.counterClockwise ? 90 : -90;
        this.strokeWidth = 10;
        this.innerRadius = this.radius - this.strokeWidth/2;
        this.circumference = 2*Math.PI*(this.innerRadius);
        this.attachShadow({ mode: "open" });
        this.render();
        let parentElement = document.getElementById(id);
        parentElement.appendChild(this);
        this.progressValue = this.shadowRoot.querySelector(".progress-value");
        this.progressBackground = this.shadowRoot.querySelector(".progress-background");
        this.progressText = this.shadowRoot.querySelector(".progress-text");
        this.setForegroundColor(color);
        this.setBackgroundColor(backgroundColor);
    }
  
    setPercentage(percentage, text = null) {
        const offset = this.circumference*(1 - percentage);
        this.progressValue.style.strokeDashoffset = offset;
        if (text !== null) {
            this.updateText(text);
        } else {
            this.updateText((percentage*100).toFixed(0) + "%");
        }
    }
  
    updateText(text) {
        this.progressText.textContent = text;
    }

    setForegroundColor(color) {
        this.progressValue.style.stroke = color;
        this.progressText.style.color = color;
    }

    setBackgroundColor(color) {
        this.progressBackground.style.stroke = color;
    }

    render() {
        this.shadowRoot.innerHTML = `
            <style>
                .progress-container {
                    width: ${this.radius * 2}px;
                    height: ${this.radius * 2}px;
                    display: flex;
                    justify-content: center;
                    align-items: center;
                }
                
                .progress {
                    transform: rotate(${this.rotation}deg);
                }
                
                .progress-background,
                .progress-value {
                    fill: none;
                    stroke-width: ${this.strokeWidth};
                }
                
                .progress-background {
                    stroke: #eee;
                }
                
                .progress-value {
                    stroke: #fff;
                    stroke-dasharray: ${this.circumference};
                    stroke-dashoffset: ${this.circumference};
                    transform-origin: center;
                }

                .progress-value.reverse {
                    transform: scaleX(-1);
                }

                .progress-text {
                    color: #fff;
                    position: absolute;
                    top: 0;
                    left: 0;
                    width: 100%;
                    height: 100%;
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    font-size: ${this.radius/2.4}px;
                    font-family: Arial, sans-serif;
                    font-weight: bold;
                    transform: rotate(${-this.rotation}deg); /* Counter-rotate the text */
                }
            </style>
            <div class="progress-container" style="position: relative;">
              <svg class="progress" width="${this.radius * 2}" height="${this.radius * 2}">
                <circle class="progress-background" cx="${this.radius}" cy="${this.radius}" r="${this.innerRadius}" />
                <circle class="${this.progressValueClass}" cx="${this.radius}" cy="${this.radius}" r="${this.innerRadius}" />
                <foreignObject width="100%" height="100%">
                  <div class="progress-text" xmlns="http://www.w3.org/1999/xhtml"></div>
                </foreignObject>
              </svg>
            </div>
        `;
    }
}
  
customElements.define("progress-circle", ProgressCircle);

class ProgressBar extends HTMLElement {
    constructor(id, initialText, barColor, backgroundColor) {
        super();
        let parentElement = document.getElementById(id);
        this.initialText = initialText;
        this.barColor = barColor;
        this.backgroundColor = backgroundColor;
        this.length = 0.9*parentElement.offsetWidth;
        this.attachShadow({ mode: "open" });
        this.render();
        parentElement.appendChild(this);
        this.visibleProgressBar = true;
    }
  
    setPercentage(percentage, text) {
        const progressBar = this.shadowRoot.querySelector(".progress-bar");
        progressBar.style.width = `${percentage*100}%`;
        this.showProgressBar();
        this.updateText(text);
    }
  
    updateText(text) {
        const progressText = this.shadowRoot.querySelector(".progress-text");
        progressText.textContent = text;
    }

    hideProgressBar() {
        if (this.visibleProgressBar) {
            this.visibleProgressBar = false;
            const progressBar = this.shadowRoot.querySelector(".progress");
            progressBar.style.display = "none";
        }
    }

    showProgressBar() {
        if (! this.visibleProgressBar) {
            this.visibleProgressBar = true;
            const progressBar = this.shadowRoot.querySelector(".progress");
            progressBar.style.display = "block";
        }
    }

    updateWidth() {
        this.length = 0.9*this.parentElement.offsetWidth;
        const progressContainer = this.shadowRoot.querySelector(".progress-container");
        progressContainer.style.width = `${this.length}px`;
    }

    render() {
        this.shadowRoot.innerHTML = `
            <style>
            .progress-container {
                width: ${this.length}px;
            }
    
            .progress-text {
                font-family: Arial, sans-serif;
                font-size: 14px;
                margin-bottom: 2px;
                width: 100%; // Limit the text container width to the size of the bar
                word-wrap: break-word; // Wrap the text according to the container width
            }
    
            .progress {
                width: 100%;
                height: 10px;
                background-color: ${this.backgroundColor};
                position: relative;
            }
    
            .progress-bar {
                position: absolute;
                top: 0;
                left: 0;
                height: 100%;
                background-color: ${this.barColor};
                width: 0%;
            }
            </style>
            <div class="progress-container">
            <div class="progress-text">${this.initialText}</div>
            <div class="progress">
                <div class="progress-bar"></div>
            </div>
            </div>
        `;
    }
}
  
customElements.define("progress-bar", ProgressBar);
