/*
  Class to create a slider which is tied to a text input field
  Bill Wood (with a little assist from ChatGPT), Feb 2023
*/

class Slider {
  constructor(parentElement, name, label, callBack,
      title = "", logarithmic = false, initialValue = 0, minValue = 0, maxValue = 100,
      length = 3, stickyVals = null) {
    this.parentElement = parentElement;
    this.name = name;
    this.label = label;
    this.callBack = callBack;
    this.title = title;
    this.logarithmic = logarithmic;
    this.initialValue = initialValue;
    this.value = NaN;
    this.lastValue = NaN;
    this.minValue = minValue;
    this.maxValue = maxValue;
    this.length = length;
    this.stickyVals = stickyVals && stickyVals.length > 0 ? stickyVals : null;
    this.scale = 0;
    this.createStyleSheet();
    this.createTextInput();
    this.createSlider();
    this.textInput.value = initialValue;
    this.validate(false);
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
      slider.step = 0.1;
      // ensure slider.value == slider.min/maxValue at extremes
      slider.value = Math.log2(this.minValue);
      slider.min = slider.value;
      slider.value = Math.log2(this.maxValue);
      slider.max = slider.value;
      slider.value = Math.log2(this.initialValue);
    } else {
      slider.step = 1;
      slider.value = this.initialValue;
      slider.min = this.minValue;
      slider.max = this.maxValue;
    }

    slider.addEventListener('input', () => {
      // console.log("input:", slider.id, slider.value);
      let value = this.logarithmic ? (2**slider.valueAsNumber) : slider.valueAsNumber;
      if (this.stickyVals != null) {
        let i = 1;
        while (i < this.stickyVals.length && value > this.stickyVals[i]) {
          i++;
        }
        value =
          (i == this.stickyVals.length || value - this.stickyVals[i - 1] < this.stickyVals[i] - value)
          ? this.stickyVals[i - 1]
          : this.stickyVals[i];
      } else if (slider.value == slider.max) {
        value = this.maxValue;
      } else if (slider.value == slider.min) {
        value = this.minValue;
      }
      this.textInput.value = value.toFixed(this.scale);
    });

    slider.addEventListener('change', () => {
      // console.log("change:", slider.id, slider.value);
      this.validate(true);
    });

    this.slider = slider;
    sliderContainer.appendChild(slider);
    this.parentElement.appendChild(sliderContainer);
  }

  createTextInput() {
    const textInputContainer = document.createElement('div');
    textInputContainer.className = 'textInput-container';

    const label = document.createElement('label');
    label.htmlFor = this.name + "-text";
    label.innerText = this.label;
    label.className = "textInput-label";
    label.title = this.title;

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
      this.validate(true);
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

  validate(doCallBack) {
    let value = parseFloat(this.textInput.value);
    // keep current value if not finite
    value = isFinite(value) ? value : this.value;
    value = Math.min(Math.max(this.minValue, value), this.maxValue);
    this.textInput.value = value.toFixed(this.scale);
    value = parseFloat(this.textInput.value);
    this.slider.value = this.logarithmic ? Math.log2(value) : value;
    if (value != this.value) {
      this.lastValue = this.value;
      this.value = value;
      if (doCallBack) {
        this.callBack(value);
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
    this.validate(false);
    return this.value;
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
