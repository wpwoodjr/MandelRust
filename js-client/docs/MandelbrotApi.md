# Mandelbrot.MandelbrotApi

All URIs are relative to *http://localhost:8080/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**computeMandelbrot**](MandelbrotApi.md#computeMandelbrot) | **POST** /mandelbrot/compute | Iterate over pixels to determine if each is in the Mandelbrot set


<a name="computeMandelbrot"></a>
# **computeMandelbrot**
> MandelbrotResults computeMandelbrot(mandelbrotCoords)

Iterate over pixels to determine if each is in the Mandelbrot set



### Example
```javascript
var Mandelbrot = require('mandelbrot');

var apiInstance = new Mandelbrot.MandelbrotApi();

var mandelbrotCoords = new Mandelbrot.MandelbrotCoords(); // MandelbrotCoords | Mandelbrot coordinates to compute

apiInstance.computeMandelbrot(mandelbrotCoords).then(function(data) {
  console.log('API called successfully. Returned data: ' + data);
}, function(error) {
  console.error(error);
});

```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **mandelbrotCoords** | [**MandelbrotCoords**](MandelbrotCoords.md)| Mandelbrot coordinates to compute | 

### Return type

[**MandelbrotResults**](MandelbrotResults.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

