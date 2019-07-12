# Mandelbrot Compute Backend

## Overview
This API computes the Mandelbrot iteration count for each pixel in a row of coordinates.

Based on work by David Eck, http://math.hws.edu/eck/index.html

### Starting the API
To run the API server with Docker, do:
```
$ npm install
$ docker build --tag harbor.gsk.com/sdi/mandelbrot:<version> .
$ docker run -p 8080:8080 --rm mandelbrot
```
Or, to run with Node, do:
```
$ npm install
$ node index.js
```

### Pushing to GSK Harbor registry
```
$ docker login harbor.gsk.com
$ docker push harbor.gsk.com/sdi/mandelbrot:<version>
```

### Testing the API
Once the server is running, you can interact with it via a browser or using curl.  To use the browser, go to:
```
http://localhost:8080/docs
```
To use curl under Unix / Linux, open a terminal prompt and enter:
```
$  curl -X POST --header 'Content-Type: application/json' --header 'Accept: application/json' -d '{
  "xmin": -2.2,
  "maxIterations": 100,
  "dx": 0.15,
  "columns": 20,
  "y": -0.6
}' 'http://localhost:8080/v1/mandelbrot/compute'
```
which should return:
```
[
  0,
  0,
  1,
  2,
  2,
  2,
  2,
  3,
  3,
  4,
  5,
  41,
  25,
  -1,
  -1,
  -1,
  11,
  15,
  3,
  2
]
```
To use curl under Windows, open a cmd prompt and enter:
```
curl -X POST "http://localhost:8080/v1/mandelbrot/compute" -H "accept: application/json" -H "Content-Type: application/json" -d "{
  \"xmin\": -2.2,
  \"maxIterations\": 100,
  \"dx\": 0.15,
  \"columns\": 20,
  \"y\": -0.6
}"
```

