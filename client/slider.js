/*
  Class to create a slider which is tied to a text input field
  Bill Wood (with a little assist from ChatGPT), Feb 2023
*/
const keyLeft = 1;
const keyRight = 2;

class Slider {
  constructor(parentElement, name, label, title, changeCallback, inputCallback = null, clickCallback = null,
      logarithmic = false, initialValue = 0, minValue = 0, maxValue = 100,
      length = 3, stickyVals = null) {
    this.parentElement = parentElement;
    this.name = name;
    this.label = label;
    this.title = title;
    this.changeCallback = changeCallback;
    this.inputCallback = inputCallback;
    this.clickCallback = clickCallback;
    this.logarithmic = logarithmic;
    this.initialValue = initialValue;
    this.value = NaN;
    this.lastValue = NaN;
    this.minValue = minValue;
    this.maxValue = maxValue;
    this.length = length;
    this.stickyVals = stickyVals && stickyVals.length > 0 ? stickyVals : null;
    this.step = 1;
    this.scale = Math.log10(1/this.step);
    this.lastInputValue = NaN;
    this.keydown = null;
    this.createStyleSheet();
    this.createTextInput();
    this.createSlider();
    this.textInput.value = initialValue;
    this.validateText(false);
  }

  createSlider() {
    const sliderContainer = document.createElement('div');
    sliderContainer.className = 'slider-container';

    const slider = document.createElement('input');
    slider.id = this.name + "-slider";
    slider.type = 'range';
    slider.className = 'slider';
    slider.title = this.title;
    if (this.logarithmic) {
      slider.step = this.step/10;
      // ensure slider.value === slider.min/maxValue at extremes
      slider.value = Math.log2(this.minValue);
      slider.min = slider.value;
      slider.value = Math.log2(this.maxValue);
      slider.max = slider.value;
      slider.value = Math.log2(this.initialValue);
    } else {
      slider.step = this.step;
      slider.value = this.initialValue;
      slider.min = this.minValue;
      slider.max = this.maxValue;
    }

    slider.addEventListener('input', () => {
      // console.log("input:", slider.id, slider.value);
      let value = this.logarithmic ? (2**slider.valueAsNumber) : slider.valueAsNumber;
      if (this.stickyVals && ! this.keydown) {
        let i = 1;
        while (i < this.stickyVals.length && value > this.stickyVals[i]) {
          i++;
        }
        value =
          (i === this.stickyVals.length || value - this.stickyVals[i - 1] < this.stickyVals[i] - value)
          ? this.stickyVals[i - 1]
          : this.stickyVals[i];
        value = Math.min(this.maxValue, Math.max(this.minValue, value));
      } else if (slider.value === slider.max) {
        value = this.maxValue;
      } else if (slider.value === slider.min) {
        value = this.minValue;
      }
      this.textInput.value = value.toFixed(this.scale);
      value = parseFloat(this.textInput.value);

      // handle case where value doesn't change because logarithmic
      if (this.keydown && this.lastInputValue === value) {
        value += (this.keydown == keyLeft ? -this.step : this.step);
        this.textInput.value = value.toFixed(this.scale);
        value = parseFloat(this.textInput.value);
      }
      this.keydown = null;

      if (this.inputCallback && this.lastInputValue !== value) {
        this.inputCallback(value);
      }
      this.lastInputValue = value;
    });

    slider.addEventListener('change', () => {
      // console.log("change:", slider.id, slider.value);
      let value = parseFloat(this.textInput.value);
      this.slider.value = this.logarithmic ? Math.log2(value) : value;
      if (value !== this.value) {
        this.lastValue = this.value;
        this.value = value;
        if (this.changeCallback) {
          this.changeCallback(value, this.lastValue);
        }
      }
    });

    slider.addEventListener('keydown', (event) => {
      switch (event.key) {
        case "ArrowLeft":
        case "ArrowDown":
        case "PageDown":
        case "Home":
          this.keydown = keyLeft;
          // console.log("keydown:", this.keydown, event.key, slider.value);
          break;

        case "ArrowRight":
        case "ArrowUp":
        case "PageUp":
        case "End":
          this.keydown = keyRight;
          // console.log("keydown:", this.keydown, event.key, slider.value);
          break;
      }
    });

    this.slider = slider;
    sliderContainer.appendChild(slider);
    this.parentElement.appendChild(sliderContainer);
  }

  createTextInput() {
    const textInputContainer = document.createElement('div');
    textInputContainer.className = 'textInput-container';

    const label = document.createElement(this.clickCallback ? 'button' : 'label');
    label.htmlFor = this.name + "-text";
    label.innerText = this.label;
    label.className = "textInput-label";
    label.title = this.title;
    if (this.clickCallback) {
      label.onclick = this.clickCallback;
    }

    const textInput = document.createElement('input');
    textInput.id = label.htmlFor;
    textInput.type = 'text';
    textInput.className = "textInput";
    textInput.title = this.title;
    textInput.maxLength = this.length;
    textInput.style.width = `${this.length}ch`;
    textInput.value = this.value;

    textInput.addEventListener('input', () => {
      // console.log("input:", textInput.id, textInput.value);
      let value = parseFloat(textInput.value);
      value = this.logarithmic ? Math.log2(value) : value;
      if (isFinite(value)) {
        this.slider.value = value;
      }
    });

    textInput.addEventListener('change', () => {
      // console.log("change:", textInput.id, textInput.value);
      this.validateText(true);
    });

    textInput.addEventListener('keydown', (event) => {
      // console.log("keydown:", textInput.id, textInput.value);
      if (event.key === 'Enter') {
        textInput.blur();
      }
    });

    this.textInput = textInput;
    textInputContainer.appendChild(label);
    textInputContainer.appendChild(textInput);
    this.parentElement.appendChild(textInputContainer);
  }

  validateText(doCallbacks) {
    let value = parseFloat(this.textInput.value);
    // keep current value if not a number
    value = isFinite(value) ? value : this.value;
    value = Math.min(Math.max(this.minValue, value), this.maxValue);
    this.textInput.value = value.toFixed(this.scale);
    value = parseFloat(this.textInput.value);
    this.slider.value = this.logarithmic ? Math.log2(value) : value;
    if (value !== this.value) {
      this.lastValue = this.value;
      this.value = value;
      if (doCallbacks) {
        if (this.inputCallback) {
          this.inputCallback(value);
        }
        if (this.changeCallback) {
          this.changeCallback(value, this.lastValue);
        }
      }
    }
  }

  getValue() {
    return this.value;
  }

  getLastValue() {
    return this.lastValue;
  }

  setValue(value) {
    this.textInput.value = value;
    this.validateText(false);
    return this.value;
  }

  setDefault() {
    return this.setValue(this.initialValue);
  }

  createStyleSheet() {
    if (document.getElementById("slider-style")) {
      return;
    }
    const style = document.createElement('style');
    style.id = "slider-style";
    style.innerHTML = `
      .textInput-container {
        margin-bottom: 5px;
        margin-top: 10px;
      }

      .textInput-label, .textInput {
        display: inline-block;
        vertical-align: middle;
        margin: 0;
        padding: 0;
      }

      .textInput {
        margin-left: 5px;
      }

      .slider-container {
        margin-bottom: 10px;
      }

      .slider-label, .slider {
        display: inline-block;
        vertical-align: middle;
        margin: 0;
        padding: 0;
      }

      .slider {
        margin-left: 10px;
      }
    `;
    document.head.appendChild(style);
  }
};
