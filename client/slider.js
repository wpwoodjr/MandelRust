/*
  Class to create a slider which is tied to a text input field
  Bill Wood (with a little assist from ChatGPT), Feb 2023
*/
const keyLeft = 1;
const keyRight = 2;

class Slider {
  constructor(id, label, title,
      logarithmic = false, initialValue = 0, minValue = 0, maxValue = 100, length = 3,
      stickyVals = null,
      textFormat = null,
      onClick = null) {
    this.parentElement = document.getElementById(id);
    this.id = id;
    this.label = label;
    this.title = title;
    this.logarithmic = logarithmic;
    this.initialValue = initialValue;
    this.value = NaN;
    this.minValue = minValue;
    this.maxValue = maxValue;
    this.length = length;
    this.stickyVals = stickyVals && stickyVals.length > 0 ? stickyVals : null;
    this.textFormat = textFormat ? textFormat : (value) => value;
    this.onClick = onClick;
    this.step = 1;
    this.scale = Math.log10(1/this.step);
    this.lastInputValue = NaN;
    this.keydown = null;
    this.lastChangeState = { };
    this.atMax = null;
    this.createStyleSheet();
    this.createTextInput();
    this.createSlider();
    this.validateText(initialValue, false);
  }

  createSlider() {
    const sliderContainer = document.createElement('div');
    sliderContainer.className = 'slider-container';

    const slider = document.createElement('input');
    slider.id = this.id + "-slider";
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
      let newValue = this.logarithmic ? (2**slider.valueAsNumber) : slider.valueAsNumber;
      console.log("input 1:", slider.id, slider.value, newValue);
      if (this.lastChangeState.sliderValue === slider.value) {
        // don't change this.value if slider.value hasn't changed
        newValue = this.lastChangeState.value;
      } else if (this.stickyVals && ! this.keydown) {
        let i = 1;
        while (i < this.stickyVals.length && newValue > this.stickyVals[i]) {
          i++;
        }
        newValue =
          (i === this.stickyVals.length
            || newValue - this.stickyVals[i - 1] < this.stickyVals[i] - newValue
            || this.stickyVals[i] > this.maxValue)
            ? this.stickyVals[i - 1]
            : this.stickyVals[i];
      } else if (slider.value === slider.max) {
        newValue = this.maxValue;
      } else if (slider.value === slider.min) {
        newValue = this.minValue;
      }
      newValue = this.toFixedValue(newValue);
      console.log("input 2:", slider.id, slider.value, newValue);

      // handle case where value doesn't change despite slider being moved - because logarithmic
      if (this.keydown) console.log("kd:", slider.value, newValue, this.lastInputValue);
      if (this.keydown && this.lastInputValue === newValue) {
        newValue += (this.keydown == keyLeft ? -this.step : this.step);
        newValue = this.toFixedValue(newValue);
        slider.value = this.logarithmic ? Math.log2(newValue) : newValue;
      }
      this.keydown = null;

      this.atMax = slider.value === slider.max;
      this.textInput.value = this.textFormat(newValue, this.atMax);
      if (this.onInput) {
        this.onInput(newValue, this.lastInputValue);
      }
      this.lastInputValue = newValue;
    });

    slider.addEventListener('change', () => {
      let newValue = this.lastInputValue;
      console.log("change:", slider.id, slider.value, newValue, this.value);
      this.value = newValue;
      this.saveLastChangeState();
      if (this.onChange) {
        this.onChange(newValue, this.lastChangeState.lastValue);
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
      label.htmlFor = this.id + "-text";
    }

    const textInput = document.createElement('input');
    textInput.id = this.id + "-text";
    textInput.type = 'text';
    textInput.className = "textInput";
    textInput.title = this.title;
    textInput.maxLength = this.length;
    textInput.style.width = `${this.length}ch`;

    textInput.addEventListener('input', () => {
      // console.log("input:", textInput.id, textInput.value);
      let value = parseFloat(textInput.value);
      value = this.logarithmic ? Math.log2(value) : value;
      if (isFinite(value)) {
        this.slider.value = value;
      }
    });

    textInput.addEventListener('change', () => {
      console.log("change:", textInput.id, textInput.value);
      this.validateText(textInput.value, true);
    });

    textInput.addEventListener('focus', () => {
      // console.log("focus:", textInput.id, textInput.value);
      textInput.value = this.value;
    });

    textInput.addEventListener('blur', () => {
      console.log("blur:", textInput.id, textInput.value, this.value, this.maxValue);
      textInput.value = this.textFormat(this.value, this.atMax);
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

  validateText(text, doCallbacks) {
    let value = parseFloat(text);
    // keep current value if not a number
    value = isFinite(value) ? value : this.value;
    value = Math.min(Math.max(this.minValue, value), this.maxValue);
    value = this.toFixedValue(value);
    this.slider.value = this.logarithmic ? Math.log2(value) : value;
    console.log("vT:", value, this.slider.value);
    this.atMax = value === this.maxValue;
    this.textInput.value = this.textFormat(value, this.atMax);
    if (value !== this.value) {///? may not want to check this
      this.value = value;
      this.saveLastChangeState();
      if (doCallbacks) {
        if (this.onInput) {
          this.onInput(value, this.lastChangeState.lastValue);
        }
        if (this.onChange) {
          this.onChange(value, this.lastChangeState.lastValue);
        }
      }
    }
  }

  getValue() {
    return this.value;
  }

  // getLastValue() {
  //   return this.lastChangeState.lastValue;
  // }

  setValue(value) {
    console.log("setValue from:", (new Error()).stack.split("\n")[2].trim().split(" ")[1], value);
    this.validateText(value, false);
    return this.value;
  }

  setDefault() {
    return this.setValue(this.initialValue);
  }

  setMaxValue(newMaxValue, setValueToMax) {
    console.log("setMV 1 new max:", newMaxValue, "old max:", this.maxValue, "cur value:", this.value, "slider value:", this.slider.value, "slider max:", this.slider.max);
    this.maxValue = newMaxValue;
    let saveSliderValue = this.slider.value;
    this.slider.max = "";
    if (this.logarithmic) {
      // ensure slider.value === slider.min/maxValue at extremes
      this.slider.value = Math.ceil(Math.log2(newMaxValue)*10)/10;
    } else {
      this.slider.value = newMaxValue;
    }
    this.slider.max = this.slider.value;
    if (setValueToMax) {
      this.value = newMaxValue;
    // maintain original slider positioning if its < new slider max
    // otherwise leave it at new slider max
    } else if (parseFloat(saveSliderValue) < parseFloat(this.slider.max)) {
      this.slider.value = saveSliderValue;
    }
    // keep this.value unchanged unless current value > new max value (or setValueToMax)
    this.value = Math.min(this.value, this.maxValue);
    this.value = this.toFixedValue(this.value);
    this.atMax = this.value === this.maxValue;
    this.textInput.value = this.textFormat(this.value, this.atMax);
    this.saveLastChangeState();
    console.log("setMV 2 new value:", this.value, "new slider value:", this.slider.value, "new slider max:", this.slider.max);
  }

  // isSliderAtMax() {
  //   console.log("isSliderAtMax:", this.id, this.slider.value, this.slider.max, this.maxValue);
  //   return this.slider.value === this.slider.max;
  // }

  // isValueAtMax() {
  //   console.log("isValueAtMax:", this.id, this.value, this.maxValue);
  //   return this.value === this.maxValue;
  // }

  saveLastChangeState() {
    console.log("changeState:", this.id, this.value, this.slider.value);
    this.lastChangeState.lastValue = this.lastChangeState.value;
    this.lastChangeState.value = this.value;
    this.lastChangeState.sliderValue = this.slider.value;
  }

  toFixedValue(value) {
    return parseFloat(value.toFixed(this.scale));
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
