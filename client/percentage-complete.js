class ProgressCircle extends HTMLElement {
    constructor(id, color, backgroundColor, radius) {
        super();
        this.color = color;
        this.backgroundColor = backgroundColor;
        this.radius = radius;
        this.attachShadow({ mode: "open" });
        this.render();
        let parentElement = document.getElementById(id);
        parentElement.appendChild(this);
    }
  
    setPercentage(percentage, text) {
        const progressValue = this.shadowRoot.querySelector(".progress-value");
        const circumference = 2 * Math.PI * (this.radius - 5);
        const offset = circumference - percentage * circumference;
        progressValue.style.strokeDashoffset = offset;
        if (text !== undefined) {
            this.updateText(text);
        } else {
            this.updateText((percentage*100).toFixed(0) + "%");
        }
    }
  
    updateText(text) {
        const progressText = this.shadowRoot.querySelector(".progress-text");
        progressText.textContent = text;
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
                transform: rotate(-90deg);
            }
            
            .progress-background,
            .progress-value {
                fill: none;
                stroke-width: 10;
            }
            
            .progress-background {
                stroke: ${this.backgroundColor};
            }
            
            .progress-value {
                stroke: ${this.color};
                stroke-dasharray: ${2 * Math.PI * (this.radius - 5)};
                stroke-dashoffset: ${2 * Math.PI * (this.radius - 5)};
            }
            .progress-text {
                position: absolute;
                top: 0;
                left: 0;
                width: 100%;
                height: 100%;
                display: flex;
                justify-content: center;
                align-items: center;
                font-size: ${this.radius / 3}px;
                font-family: Arial, sans-serif;
                transform: rotate(90deg); /* Counter-rotate the text */
              }
            </style>
            <div class="progress-container" style="position: relative;">
              <svg class="progress" width="${this.radius * 2}" height="${this.radius * 2}">
                <circle class="progress-background" cx="${this.radius}" cy="${this.radius}" r="${this.radius - 5}" />
                <circle class="progress-value" cx="${this.radius}" cy="${this.radius}" r="${this.radius - 5}" />
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
