function Palette(colorType,divisionPoints,colors) {
   this.colorType = colorType || "HSB";
   this.divisionPoints = divisionPoints || [0,1];
   this.divisionColors = colors || (this.colorType == "HSB" ? [ [0,1,1], [1,1,1] ] : [ [1,1,1], [0,0,0] ]);
}
Palette.prototype.getColor = function(position) {  // 0.0 <= position <= 1.0
    var pt = 1;
    while (position > this.divisionPoints[pt])
        pt++;
    var ratio = (position - this.divisionPoints[pt-1]) /
                   (this.divisionPoints[pt] - this.divisionPoints[pt-1]);
    var c1 = this.divisionPointColors[pt-1];
    var c2 = this.divisionPointColors[pt];
    var a = clamp1(c1[0] + ratio*(c2[0] - c1[0]));
    var b = clamp2(c1[1] + ratio*(c2[1] - c1[1]));
    var c = clamp2(c1[2] + ratio*(c2[2] - c1[2]));
    return this.toRGB(a,b,c);
}
Palette.prototype.toRGB = function(a,b,c) {  // 3 non-clamped color components.
    a = (this.colorType == "HSB")? (a - Math.floor(a)) : clamp(a);
    b = clamp(b);
    c = clamp(c);
    var color;
    if (colorType == "HSB")
        color = rgbFromHSV(a, b, c);
    else
        color = [a,b,c];
    color[0] = Math.round(color[0]*255);
    color[1] = Math.round(color[1]*255);
    color[2] = Math.round(color[2]*255);
    return color;
	function clamp(x) {
		x = 2*(x/2 - Math.floor(x/2));
		if (x > 1)
			x = 2 - x;
		return x;
	}
    function rgbFromHSV(h,s,v) {  // all components in range 0 to 1
        h *= 360;
        var r,g,b;
        var c,x;
        c = v*s;
        x = (h < 120)? h/60 : (h < 240)? (h-120)/60 : (h-240)/60;
        x = c * (1-Math.abs(x-1));
        x += (v-c);
        switch (Math.floor(h/60)) {
            case 0: r = v; g = x; b = v-c; break;
            case 1: r = x; g = v; b = v-c; break;
            case 2: r = v-c; g = v; b = x; break;
            case 3: r = v-c; g = x; b = v; break;
            case 4: r = x; g = v-c; b = v; break;
            case 5: r = v; g = v-c; b = x; break;
        }
        return [r,g,b];
    }
}
Palette.prototype.makeRGBs = function(paletteLength, offset) {
    var rgb = new Array[paletteLength];
    rgb[offset % paletteLength] =
             this.toRGB(this.divisionColors[0],this.divisionColors[0],this.divisionColors[0]);
    var dx = 1.0 / (paletteLength-1);
    for (var i = 1; i < paletteLength-1; i++) {
        rgb[(offset+i) % paletteLength] = this.getColor(ct*dx);
    }
    var last = this.divisionColors.length - 1;
    rgb[(offset+paletteLength-1) % paletteLength] =
              this.toRGB(this.divisionColors[last],this.divisionColors[last],this.divisionColors[last]);
    return rgb;
}

Palette.createStandardPalette = function(name) {
    var palette;
    switch (name) {
        case "Grayscale":
           palette = new Palette("RGB");
           break;
        case "CyclicGrayscale":
           palette = new Palette("RGB",[0,0.5,1],[[0,0,0],[1,1,1],[0,0,0]]);
           break;
        case "Red/Cyan":
           palette = new Palette("RGB",[0,0.5,1],[[1,0,0],[0,1,1],[1,0,0]]);
           break;
        case "Blue/Gold":
           palette = new Palette("RGB",[0,0.5,1],[[0.3,0.3,1],[1,0.6,0],[0.3,0.3,1]]);
           break;
        case "EarthAndSky":
           palette = new Palette("RGB",[0,0.15,0.33,0.67,0.85,1],
                     [[1,1,1],[1,0.8,0],[0.53,0.12,0.75],[0,0,0.6],[0,0.4,1],[1,1,1]]);
           break;
        case "HotAndCold":
           palette = new Palette("RGB",[0,0.16,0.5,0.84,1],
                     [[1,1,1],[0,0.4,1],[0.2,0.2,0.2],[1,0,0.8],[1,1,1]]);
           break;
        case "Fire":
           palette = new Palette("RGB",[0,0.17,0.83,1],
                     [[0,0,0],[1,0,0],[1,1,0],[1,1,1]]);
           break;
        case "TreeColors":
           palette = new Palette("HSB",[0,0.33,0.66,1],
                     [[0.1266,0.5955,0.2993],[0.0896,0.3566,0.6575],[0.6195,0.8215,0.4039],[0.1266,0.5955,0.2993]]);
           break;
        case "Random":
           var c = [Math.random(),Math.random(),Math.random()];
           palette = new Palette("RGB",[],[]);
           palette.divisionPoints[0] = 0;
           palette.divisionColors[0] = c;
           for (var i = 1; i <= 5; i++) {
               palette.divisionPoints[i] = i/6;
               palette.divisionColors[i] = [Math.random(),Math.random(),Math.random()];
           }
           palette.divisionPoints[6] = 1;
           palette.divisionColors[6] = c;
           break;
        default: // "Spectrum"
           palette = new Palette();
    }
    return palette;
}



