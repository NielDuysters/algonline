const cookieParser = require('cookie-parser')
const sessions = require('express-session')
const createError = require('http-errors')
const websocket = require('./websocket')
const express = require('express')
const logger = require('morgan')
const path = require('path')
const cors = require('cors')
const http = require('http')

const app = express()
const router = require('./routes/routes')
client = require('./db')

global.api_key = "abc123"

// view engine setup
app.set('views', path.join(__dirname, 'views'))
app.set('view engine', 'jade')

app.use(logger('dev'))
app.use(express.json())
app.use(express.urlencoded({ extended: false }))
app.use(cookieParser())
app.use(express.static(path.join(__dirname, 'public')))

// Favicon
app.use('/favicon.ico', express.static('public/images/favicon.ico'))

// sessions
const oneDay = 1000 * 60 * 60 * 24
let session = sessions({
    secret: "2525863e6d60b03afa598280",
    saveUninitialized: true,
    cookie: { maxAge: oneDay },
    resave: false 
})
app.use(session)


app.use('/', router)

// catch 404 and forward to error handler
app.use(function(req, res, next) {
  next(createError(404))
});

// error handler
app.use(function(err, req, res, next) {
  // set locals, only providing error in development
  res.locals.message = err.message
  res.locals.error = req.app.get('env') === 'development' ? err : {}

  // render the error page
  res.status(err.status || 500)
  res.render('error')
});

// Websocket
const ws_server = http.createServer()
websocket(ws_server, session)
ws_server.listen(3001, () => {
    console.log('WebSocket server listening on port 3001')
});

module.exports = app
