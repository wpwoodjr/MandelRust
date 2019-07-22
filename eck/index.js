'use strict';

var fs = require('fs'),
    path = require('path'),
    http = require('http');

var app = require('connect')();
//var CORS = require('connect-cors');
var swaggerTools = require('swagger-tools');
var jsyaml = require('js-yaml');
var serverPort = 8080;
var serveStatic = require('serve-static');

// swaggerRouter configuration
var options = {
  swaggerUi: path.join(__dirname, '/swagger.json'),
  controllers: path.join(__dirname, './controllers'),
  useStubs: process.env.NODE_ENV === 'development' // Conditionally turn on stubs (mock mode)
};

// The Swagger document (require it, build it programmatically, fetch it from a URL, ...)
var spec = fs.readFileSync(path.join(__dirname,'api/swagger.yaml'), 'utf8');
var swaggerDoc = jsyaml.safeLoad(spec);

//app.use(CORS());
/*{
    origins: [],            // implicit same as ['*'], and null
    methods: ['POST'],      // OPTIONS is always allowed
    headers: [              // both `Exposed` and `Allowed` headers
        'Content-Type',
        'Accept'
    ]
}));*/

// Initialize the Swagger middleware
swaggerTools.initializeMiddleware(swaggerDoc, function (middleware) {

  // Interpret Swagger resources and attach metadata to request - must be first in swagger-tools middleware chain
  app.use(middleware.swaggerMetadata());

  // Validate Swagger requests
  app.use(middleware.swaggerValidator());

  // Route validated requests to appropriate controller
  app.use(middleware.swaggerRouter(options));

  app.use(errorHandler);

  // Serve the Swagger documents and Swagger UI
  app.use(middleware.swaggerUi());

  app.use(serveStatic('client', { fallthrough: false, index: 'MB.html' }));

  // Start the server
  http.createServer(app).listen(serverPort, function () {
    console.log('Your server is listening on port %d (http://localhost:%d)', serverPort, serverPort);
    console.log('Swagger-ui is available on http://localhost:%d/docs', serverPort);
  });

});

// global error handler
function errorHandler(err, req, res, next) {
// http://www.senchalabs.org/connect/errorHandler.html
  let message = "SERVER ERROR";
  if (res.statusCode >= 400 && res.statusCode < 500) {
    message = "CLIENT ERROR";
  }
  res.setHeader('Content-Type', 'application/json');
  res.end(JSON.stringify({ message: message }));
}
