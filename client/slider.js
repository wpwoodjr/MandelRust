/*
  Class to create a slider which is tied to a text input field
  Bill Wood (with a little assist from ChatGPT), Feb 2023
*/
const keyLeft = 1;
const keyRight = 2;

class Slider {
  constructor(name, label, title,
      logarithmic = false, initialValue = 0, minValue = 0, maxValue = 100, length = 3,
      stickyVals = null,
      textFormat = null,
      onClick = null) {
    this.parentElement = document.getElementById(name);
    this.name = name;
    this.label = label;
    this.title = title;
    this.logarithmic = logarithmic;
    this.initialValue = initialValue;
    this.value = NaN;
    this.lastValue = NaN;
    this.minValue = minValue;
    this.maxValue = maxValue;
    this.length = length;
    this.stickyVals = stickyVals && stickyVals.length > 0 ? stickyVals : null;
    this.textFormat = textFormat ? textFormat : (value) => value;
    this.shadowTextValue = initialValue;
    this.onClick = onClick;
    this.step = 1;
    this.scale = Math.log10(1/this.step);
    this.lastInputValue = NaN;
    this.keydown = null;
    this.createStyleSheet();
    this.createTextInput();
    this.createSlider();
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
      slider.value = Math.floor(Math.log2(this.minValue)*10)/10;
      slider.min = slider.value;
      slider.value = Math.ceil(Math.log2(this.maxValue)*10)/10;
      slider.max = slider.value;
      slider.value = Math.log2(this.initialValue);
    } else {
      slider.step = this.step;
      slider.value = this.minValue;
      slider.min = slider.value;
      slider.value = this.maxValue;
      slider.max = slider.value;
      slider.value = this.initialValue;
    }

    slider.addEventListener('input', () => {
      // console.log("input:", slider.id, slider.value);
      let newValue = this.logarithmic ? (2**slider.valueAsNumber) : slider.valueAsNumber;
      if (this.stickyVals && ! this.keydown) {
        let i = 1;
        while (i < this.stickyVals.length && newValue > this.stickyVals[i]) {
          i++;
        }
        newValue =
          (i === this.stickyVals.length || newValue - this.stickyVals[i - 1] < this.stickyVals[i] - newValue)
          ? this.stickyVals[i - 1]
          : this.stickyVals[i];
        newValue = Math.min(this.maxValue, Math.max(this.minValue, newValue));
      } else if (slider.value === slider.max) {
        newValue = this.maxValue;
      } else if (slider.value === slider.min) {
        newValue = this.minValue;
      }
      this.shadowTextValue = newValue.toFixed(this.scale);
      newValue = parseFloat(this.shadowTextValue);

      // handle case where value doesn't change because logarithmic
      if (this.keydown && this.lastInputValue === newValue) {
        newValue += (this.keydown == keyLeft ? -this.step : this.step);
        this.shadowTextValue = newValue.toFixed(this.scale);
        newValue = parseFloat(this.shadowTextValue);
      }
      this.keydown = null;

      this.textInput.value = this.textFormat(this.shadowTextValue, this);

      if (this.onInput && this.lastInputValue !== newValue) {
        this.onInput(newValue);
      }
      this.lastInputValue = newValue;
    });
/*
    slider.addEventListener('input', () => {
      let newValue = this.logarithmic ? (2**slider.valueAsNumber) : slider.valueAsNumber;
      // console.log("input:", slider.id, slider.value, newValue, slider.max);

      // look for nearest sticky value
      if (this.stickyVals && ! this.keydown) {
        let i = 1;
        while (i < this.stickyVals.length && newValue > this.stickyVals[i]) {
          i++;
        }
        newValue =
          (i === this.stickyVals.length
            || newValue - this.stickyVals[i - 1] < Math.min(this.maxValue, this.stickyVals[i]) - newValue)
          ? this.stickyVals[i - 1]
          : this.stickyVals[i];
      }

      // handle case where value doesn't change because logarithmic
      if (this.keydown && this.lastInputValue === newValue) {
        newValue += (this.keydown == keyLeft ? -this.step : this.step);
      }
      this.keydown = null;

      newValue = Math.min(this.maxValue, Math.max(this.minValue, newValue));
      this.shadowTextValue = newValue.toFixed(this.scale);
      newValue = parseFloat(this.shadowTextValue);
      this.textInput.value = this.textFormat(this.shadowTextValue, this);

      if (this.onInput && this.lastInputValue !== newValue) {
        this.onInput(newValue);
      }
      this.lastInputValue = newValue;
      // console.log("input end:", slider.id, slider.value, newValue, slider.max);
    });
*/
    slider.addEventListener('change', () => {
      let newValue = parseFloat(this.shadowTextValue);
      // console.log("change:", slider.id, slider.value, newValue, this.value);
      // w/out this, 50 left arrow is higher than 50
      // with this, slider can be moved to max position by it
      slider.value = this.logarithmic ? Math.log2(newValue) : newValue;
      if (newValue !== this.value) {
        this.lastValue = this.value;
        this.value = newValue;
        if (this.onChange) {
          this.onChange(newValue, this.lastValue);
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

        case "Enter":
          slider.blur();
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

    const label = document.createElement(this.onClick ? 'button' : 'label');
    label.innerText = this.label;
    label.className = "textInput-label";
    label.title = this.title;
    if (this.onClick) {
      label.onclick = this.onClick;
    } else {
      label.htmlFor = this.name + "-text";
    }

    const textInput = document.createElement('input');
    textInput.id = this.name + "-text";
    textInput.type = 'text';
    textInput.className = "textInput";
    textInput.title = this.title;
    textInput.maxLength = this.length;
    textInput.style.width = `${this.length}ch`;

    textInput.addEventListener('input', () => {
      // console.log("input:", textInput.id, textInput.value);
      this.shadowTextValue = textInput.value;
      let value = parseFloat(this.shadowTextValue);
      value = this.logarithmic ? Math.log2(value) : value;
      if (isFinite(value)) {
        this.slider.value = value;
      }
    });

    textInput.addEventListener('change', () => {
      // console.log("change:", textInput.id, textInput.value);
      this.validateText(true);
    });

    textInput.addEventListener('focus', () => {
      // console.log("focus:", textInput.id, textInput.value);
      textInput.value = this.shadowTextValue;
    });

    textInput.addEventListener('blur', () => {
      // console.log("blur:", textInput.id, textInput.value);
      textInput.value = this.textFormat(this.shadowTextValue, this);
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
    let value = parseFloat(this.shadowTextValue);
    // keep current value if not a number
    value = isFinite(value) ? value : this.value;
    value = Math.min(Math.max(this.minValue, value), this.maxValue);
    this.shadowTextValue = value.toFixed(this.scale);
    value = parseFloat(this.shadowTextValue);
    this.slider.value = this.logarithmic ? Math.log2(value) : value;
    this.textInput.value = this.textFormat(this.shadowTextValue, this);
    if (value !== this.value) {
      this.lastValue = this.value;
      this.value = value;
      if (doCallbacks) {
        if (this.onInput) {
          this.onInput(value);
        }
        if (this.onChange) {
          this.onChange(value, this.lastValue);
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
    this.shadowTextValue = value;
    this.validateText(false);
    return this.value;
  }

  setDefault() {
    return this.setValue(this.initialValue);
  }

  setMaxValue(value) {
    this.maxValue = value;
    let slider = this.slider;
    slider.max = "";
    if (this.logarithmic) {
      // ensure slider.value === slider.min/maxValue at extremes
      slider.value = Math.log2(value);
    } else {
      slider.value = value;
    }
    slider.max = slider.value;
    this.setValue(this.getValue());
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
