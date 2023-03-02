/*
  Class to create a slider which is tied to a text input field.
  Slider can be logarithmic or linear, and can have "sticky" preset values.
  Bill Wood (with a little assist from ChatGPT), Feb 2023
*/
const keyLeft = 1;
const keyRight = 2;

class Slider {
  constructor(id, options) {
    this.id = id;
    this.parentElement = document.getElementById(id);
    options = options || {};
    this.label = options.label || "";
    this.title = options.title || "";
    this.logarithmic = options.logarithmic || false;
    this.minValue = options.minValue || 0;
    this.maxValue = options.maxValue || 100;
    this.initialValue = options.initialValue || this.minValue;
    this.step = options.step || 1;
    this.textLength = options.textLength || 3;
    this.stickyVals = options.stickyVals || null;
    this.textFormat = options.textFormat || ((value) => value);
    this.button = options.button || false;
    this.onClick = options.onClick || null;
    this.onChange = options.onChange || null;
    this.onInput = options.onInput || null;
    this.notifyOnlyOnDifference = options.notifyOnlyOnChange === false ? false : true;

    this.value = NaN;
    this.lastInputValue = NaN;
    this.scale = Math.log10(1/this.step);
    this.minValue = this.toFixedValue(this.minValue);
    this.maxValue = this.toFixedValue(this.maxValue);
    this.initialValue = this.toFixedValue(this.initialValue);
    this.stickyVals = this.stickyVals && this.stickyVals.length > 0 ? this.stickyVals : null;
    this.keydown = null;
    this.lastChangeState = { };
    this.createStyleSheet();
    this.createTextInput();
    this.createSlider();
    this.setDefault();
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
      // ensure slider.value includes slider.minValue and maxValue at extremes
      slider.value = Math.floor(Math.log2(this.minValue)/slider.step)*slider.step;
      slider.min = slider.value;
      slider.value = Math.ceil(Math.log2(this.maxValue)/slider.step)*slider.step;
      slider.max = slider.value;
    } else {
      slider.step = this.step;
      slider.value = this.minValue;
      slider.min = slider.value;
      slider.value = this.maxValue;
      slider.max = slider.value;
    }

    slider.addEventListener('input', () => {
      if (isNaN(this.lastInputValue)) {
        this.lastInputValue = this.value;
      }

      let newValue = this.logarithmic ? (2**slider.valueAsNumber) : slider.valueAsNumber;
      // console.log("i1:", newValue, this.lastInputValue, slider.value, slider.max, this.lastChangeState.current.sliderValue);

      // this must be done for keydown and stickyVal; and must be done here otherwise stickyVal might return 35 not 30:
        // logarithmic: true, initialValue: 1, minValue: 1, maxValue: 35, textLength: 5,
        // stickyVals: [0,5,10,15,20,25,30,36,40,45,50,55,60,65,70,75,80,85,90,95,100],
      newValue = Math.min(Math.max(this.minValue, newValue), this.maxValue);

      if (this.lastChangeState.current.sliderValue === slider.value) {
        // don't change "this.value" if "slider.value" hasn't changed, b/c onChange won't be called
        newValue = this.lastChangeState.current.value;

      } else if (this.keydown) {
        newValue = this.toFixedValue(newValue);
        // handle case where value doesn't change despite slider being moved - because logarithmic
        if (this.lastInputValue === newValue) {
          newValue += (this.keydown === keyLeft ? -this.step : this.step);
        }
        this.keydown = null;
        // console.log("i2:", newValue, this.lastInputValue, slider.value, slider.max);

      } else {
        if (this.stickyVals) {
          let i = 1;
          while (i < this.stickyVals.length && newValue > this.stickyVals[i]) {
            i++;
          }
          // console.log("i3:",newValue,this.stickyVals[i-1],this.stickyVals[i],this.minValue,this.maxValue);
          newValue =
            (i === this.stickyVals.length
              || this.stickyVals[i] > this.maxValue
              || (newValue - this.stickyVals[i - 1] < this.stickyVals[i] - newValue
                && this.stickyVals[i - 1] >= this.minValue))
              ? this.stickyVals[i - 1]
              : this.stickyVals[i];
        }
        // do this here instead of above in case stickyVals has values in it that need "fixing"
        newValue = this.toFixedValue(newValue);
      }

      // Two reasons for this...
        // 1) for keydown, handle case where newValue is the same at slider.max - 1 and slider.max
        // 2) for stickyVals, handle case where stickyVal picks maxValue but slider is not at max and then
        //    in onChange slider is set to slider.max (stickyVal of 1000 and maxValue of 1000 causes this)
        //    which can cause displayed input value (1000) not to match final value (eg "MaxIters")
      if (newValue === this.minValue) {
        slider.value = slider.min;
      } else if (newValue === this.maxValue) {
        slider.value = slider.max;
      }
      // console.log("i4:", newValue, this.lastInputValue, slider.value, slider.max);

      // do callback
      this.textInput.value = this.textFormat(newValue);
      if (this.onInput && (newValue !== this.lastInputValue || ! this.notifyOnlyOnDifference)) {
        this.onInput(newValue, this.lastInputValue);
      }
      this.lastInputValue = newValue;
  });

    slider.addEventListener('change', () => {
      let newValue = this.lastInputValue;
      this.lastInputValue = NaN;

      if (slider.value !== slider.min && slider.value !== slider.max) {
        // set slider position to match newValue value
          // do this here instead of in onInput to provide smoother experience while sliding
          // and also did see "missed" slider changes when it was in onInput
        slider.value = this.logarithmic ? Math.log2(newValue) : newValue;
      }
      // console.log("c1:", newValue, this.lastInputValue, slider.value, slider.max);

      let doOnChange = this.onChange && (newValue !== this.value || ! this.notifyOnlyOnDifference);
      if (newValue !== this.value) {
        this.value = newValue;
        this.saveLastChangeState();
      }
      if (doOnChange) {
        // console.log("c2:", slider.id, newValue, this.lastChangeState.prev.value, slider.value);
        this.onChange(newValue, this.lastChangeState.prev.value);
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

    const label = document.createElement(this.button ? 'button' : 'label');
    label.innerText = this.label;
    label.className = "textInput-label";
    label.title = this.title;
    if (this.button) {
      label.onclick = this.onClick;
    } else {
      label.htmlFor = this.id + "-text";
    }

    const textInput = document.createElement('input');
    textInput.id = this.id + "-text";
    textInput.type = 'text';
    textInput.className = "textInput";
    textInput.title = this.title;
    textInput.maxLength = this.textLength;
    textInput.style.width = `${this.textLength}ch`;

    textInput.addEventListener('input', () => {
      // console.log("input:", textInput.id, textInput.value);
      let newValue = parseFloat(textInput.value);
      if (isFinite(newValue)) {
        this.setSliderValueFromText(newValue);
      }
    });

    textInput.addEventListener('change', () => {
      // console.log("change:", textInput.id, textInput.value);
      this.validateText(textInput.value, true);
    });

    textInput.addEventListener('focus', () => {
      // console.log("focus:", textInput.id, textInput.value);
      textInput.value = this.value;
    });

    textInput.addEventListener('blur', () => {
      // console.log("blur:", textInput.id, textInput.value, this.value, this.maxValue);
      textInput.value = this.textFormat(this.value);
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
    let newValue = parseFloat(text);
    // keep current value if not a number
    newValue = isFinite(newValue) ? newValue : this.value;
    newValue = Math.min(Math.max(this.minValue, newValue), this.maxValue);
    newValue = this.toFixedValue(newValue);
    this.setSliderValueFromText(newValue);
    console.log("validateText:", newValue, this.value, this.minValue, this.maxValue, this.slider.value);

    this.textInput.value = this.textFormat(newValue);
    doCallbacks = doCallbacks && (newValue !== this.value || ! this.notifyOnlyOnDifference);
    if (newValue !== this.value) {
      this.value = newValue;
      this.saveLastChangeState();
    }
    if (doCallbacks) {
      if (this.onInput) {
        this.onInput(newValue, this.lastChangeState.prev.value);
      }
      if (this.onChange) {
        this.onChange(newValue, this.lastChangeState.prev.value);
      }
    }
  }

  setMaxValue(newMaxValue, setValueToMax) {
    console.log("setMV 1 new max:", newMaxValue, "old max:", this.maxValue, "cur value:", this.value, "slider value:", this.slider.value, "slider max:", this.slider.max);
    this.maxValue = this.toFixedValue(newMaxValue);
    let saveSliderValue = this.slider.value;
    this.slider.max = "";
    if (this.logarithmic) {
      this.slider.value = Math.ceil(Math.log2(newMaxValue)/this.slider.step)*this.slider.step;
    } else {
      this.slider.value = newMaxValue;
    }
    this.slider.max = this.slider.value;

    let oldValue = this.value;
    if (setValueToMax) {
      this.value = newMaxValue;
    // maintain original slider value if its < new slider max
    // otherwise leave it at new slider max
    } else if (parseFloat(saveSliderValue) < parseFloat(this.slider.max)) {
      this.slider.value = saveSliderValue;
    }

    // keep this.value unchanged unless current value > new max value (or setValueToMax is true)
    this.value = Math.min(this.value, this.maxValue);

    this.textInput.value = this.textFormat(this.value);
    if (oldValue !== this.value) {
      this.value = oldValue;
      this.saveLastChangeState();
    }
    console.log("setMV 2 new value:", this.value, "new slider value:", this.slider.value, "new slider max:", this.slider.max);
  }

  // set slider position based on newValue
  setSliderValueFromText(newValue) {
    if (newValue >= this.maxValue) {
      this.slider.value = this.slider.max;
    } else if (newValue <= this.minValue) {
      this.slider.value = this.slider.min;
    } else {
      this.slider.value = this.logarithmic ? Math.log2(newValue) : newValue;
      // slider at extremes is for min and max values only
      if (this.slider.value === this.slider.max) {
        this.slider.value = this.slider.valueAsNumber - parseFloat(this.slider.step);
      }
      else if (this.slider.value === this.slider.min) {
        this.slider.value = this.slider.valueAsNumber + parseFloat(this.slider.step);
      }
    }
  }

  // getLastValue() {
  //   return this.lastChangeState.lastValue;
  // }

  setValue(value) {
    console.log("setValue from:", this.id, (new Error()).stack.split("\n")[2].trim().split(" ")[1], value);
    this.validateText(value, false);
    return this.value;
  }

  setDefault() {
    return this.setValue(this.initialValue);
  }

  atMax() {
    return this.slider.value === this.slider.max;
  }

  atMin() {
    return this.slider.value === this.slider.min;
  }

  saveLastChangeState() {
    this.lastChangeState.prev = this.lastChangeState.current;
    this.lastChangeState.current = {
      minValue: this.minValue,
      maxValue: this.maxValue,
      value: this.value,
      sliderMin: this.slider.min,
      sliderMax: this.slider.max,
      sliderValue: this.slider.value
    };
  }

  restoreChangeState(cs) {
    console.log("restoreChangeState:", this.id, cs);
    this.minValue = cs.minValue;
    this.maxValue = cs.maxValue;
    this.value = cs.value;
    this.slider.min = cs.sliderMin;
    this.slider.max = cs.sliderMax;
    this.slider.value = cs.sliderValue;
    this.saveLastChangeState();

    this.textInput.value = this.textFormat(this.value);
    if (this.onInput) {
      this.onInput(this.value, this.lastChangeState.prev.value);
    }
    if (this.onChange) {
      this.onChange(this.value, this.lastChangeState.prev.value);
    }
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
