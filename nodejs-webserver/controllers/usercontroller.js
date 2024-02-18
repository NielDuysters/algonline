// Controller to handle users. Login, register or retrieve user-data.

const asyncHandler = require("express-async-handler")
const { Validator } = require('node-input-validator')
const User = require("../models/user")
const bcrypt = require("bcrypt")
const axios = require('axios')

// Check if user is authenticated when retrieving user-data.
const check_auth = (req, res, next) => {
    if (!req.session.user) {
        return res.sendStatus(401)
    }

    next()
}

// Function handling login. Called from exports.login_form.
exports.login = asyncHandler(async (req, res, next) => {
    // Retrieve user from username.
    var sql = "SELECT id, username, password FROM users WHERE username = $1"
    var values = [req.body.username]
    var q = await client.query(sql, values)

    // Check if user is found.
    if (q.rows.length == 0) {
        return res.render("index", { error: "User not found." })
    }

    // Verify password hash.
    const phash = q.rows[0].password
    const ok = await bcrypt.compare(req.body.password, phash)

    if (!ok) {
        return res.render("index", { error: "Wrong credentials." })
    }

    // Save user in session.
    var session = req.session
    session.user = new User(
        q.rows[0].id,
        q.rows[0].username
    )
    
    // Save session token in database.
    var sql = "UPDATE users SET session_token = $1 WHERE id = $2"
    var values = [req.sessionID, session.user.id]
    var q = await client.query(sql, values)

    res.redirect("/panel")
})

// Executed when login or registration form is submitted.
exports.login_form = asyncHandler(async (req, res, next) => {

    // Validate input.
    const v = new Validator(req.body, {
        username: "required|minLength:3",
        password: "required|minLength:5",
    })

    const valid = await v.check()
    if (!valid) {
        return res.render("index", { title: "Express", error: "Invalid input." })
    }

    // Check if action is login or register.
    if ('login' in req.body) {
        exports.login(req, res, next)
    }
    else if ('register' in req.body) {
        exports.register(req, res, next)
    }
    else {
        res.status(404).send("Not found")
    }
})

// Function handling registration. Called from exports.login_form.
exports.register = asyncHandler(async (req, res, next) => {
    // Check if username already exists.
    var sql = "SELECT id FROM users WHERE username = $1"
    var values = [req.body.username]
    var q = await client.query(sql, values)
    
    if (q.rows.length > 0) {
        return res.render("index", { error: "User already exists." })
    }


    // Register user.
    const salt = await bcrypt.genSalt(10)
    const phash = await bcrypt.hash(req.body.password, salt)

    var sql = "INSERT INTO users (username, password) VALUES ($1, $2) RETURNING id"
    var values = [req.body.username, phash]
    var q = await client.query(sql, values)

    if (q.rows.length == 0) {
        return res.render("index", { error: "Unexpected error." })
    }
    
    // Save user in session.
    var session = req.session
    session.user = new User(
        q.rows[0].id,
        req.body.username
    )    
    
    // Save session token in database.
    var sql = "UPDATE users SET session_token = $1 WHERE id = $2"
    var values = [req.sessionID, session.user.id]
    var q = await client.query(sql, values)

    return res.redirect("/panel/settings")
})

// Get current balance of user.
exports.balance = [check_auth, asyncHandler(async (req, res, next) => {
    try {
        const response = await axios.get(`http://127.0.0.1:8080/balance`, 
            {
                insecureHTTPParser: true,
                headers: { 
                    "API_KEY": global.api_key,
                    "session_token": req.sessionID,
                }
            })

        if (response.status == 200) {
            return res.status(200).send(response.data)
        }

        return res.sendStatus(response.status)
    } catch(err) {
        res.status(500).send("Unknown error occurred.")
    }
})]
