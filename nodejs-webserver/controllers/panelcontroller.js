// Controller to handle actions done by the user in the panel.

const asyncHandler = require("express-async-handler")
const { Validator } = require('node-input-validator')
const bcrypt = require("bcrypt")
const moment = require('moment')
const WebSocket = require('ws')
const axios = require('axios')
const he = require('he')

const Algorithm = require("../models/algorithm")
const User = require("../models/user")

// Check if user is authenticated.
const check_auth = (req, res, next) => {
    if (!req.session.user) {
        return res.redirect("/")
    }

    next()
}

// Function to get the start_funds of the user. Required to display
// in the panel and to calculate profit/loss.
async function start_funds(session_token)  {
    // Retrieve start_funds from user by session_token.
    var sql = "SELECT start_funds_usdt, start_funds_btc, start_funds_total FROM users WHERE session_token = $1"
    var values = [session_token]
    var q = await client.query(sql, values)

    // Check if user is found.
    if (q.rows.length == 0) {
        return {
            "usdt": 0,
            "btc": 0,
            "total": 0,
        }
    }

    return {
        "usdt": parseFloat(q.rows[0].start_funds_usdt),
        "btc": parseFloat(q.rows[0].start_funds_btc),
        "total": parseFloat(q.rows[0].start_funds_total)
    }
}

// Homepage.
exports.index = [check_auth, asyncHandler(async (req, res, next) => {
    // Get algorithms.
    var sql = "SELECT id, description, ROUND(start_funds_usdt, 2) AS start_funds FROM algorithms WHERE user_id = $1"
    var values = [req.session.user.id]
    var q = await client.query(sql, values)

    res.render("panel/algorithms", { algorithms: q.rows, start_funds: await start_funds(req.sessionID), user: req.session.user })
})]

// Add algorithm page.
exports.add = [check_auth, asyncHandler(async (req, res, next) => {
    res.render("panel/add-algorithm", { start_funds: await start_funds(req.sessionID) })
})]

// Called when the form to add an algorithm is submitted.
exports.add_form = [check_auth, asyncHandler(async (req, res, next) => {

    // Validate input.
    const v = new Validator(req.body, {
        id: "required|minLength:3",
        description: "required|minLength:3",
        start_funds: "required|min:1",
        first_btc_order: "required|min:0",
        interval: "required|in:1s,1m,5m,15m,30m,1h,2h,12h,1d,3d",
        prepend_data: "required|in:0,30m,1h,1d,1w,30d",
        run_every_sec: "required|min:0",
        code: "required",
    })
   
    const valid = await v.check()
    if (!valid) {
        return res.render("panel/add-algorithm", { error: "Ongeldige invoer. Zijn alle velden correct ingevuld?", start_funds: await start_funds(req.sessionID) })
    }

    if (parseFloat(req.body.first_btc_order) > parseFloat(req.body.start_funds)) {
        return res.render("panel/add-algorithm", { error: "Eerste BTC aankoop kan niet groter zijn dan start funds in USDT.", start_funds: await start_funds(req.sessionID) })
    }

    // Send request to Rust.
    try {
        const response = await axios.post("http://127.0.0.1:8080/algorithms/add", 
            {
                id: req.body.id,
                description: req.body.description,
                start_funds: parseFloat(req.body.start_funds),
                first_btc_order: parseFloat(req.body.first_btc_order),
                interval: req.body.interval,
                run_every_sec: parseInt(req.body.run_every_sec),
                prepend_data: req.body.prepend_data,
                code: Buffer.from(req.body.code).toString("base64"),
            },
            {
                insecureHTTPParser: true,
                headers: { 
                    "API_KEY": global.api_key,
                    "session_token": req.sessionID,
                }
            })

        if (response.status == 201) {
            return res.redirect("/panel")
        } else {
            return res.render("panel/add-algorithm", { error: "Fout bij toevoegen.", start_funds: await start_funds(req.sessionID) })
        }
    } catch (err) {
        return res.render("panel/add-algorithm", { error: "Fout bij toevoegen.", start_funds: await start_funds(req.sessionID) })
    }
        
})]

// Called when the user clicks the power-button to start/stop an algorithm.
exports.algorithm_toggle_running = [check_auth, asyncHandler(async (req, res, next) => {
    try {
        const response = await axios.post("http://127.0.0.1:8080/algorithms/" + req.params.id +  "/" + req.params.start_or_run, 
            {
            },
            {
                insecureHTTPParser: true,
                headers: { 
                    "API_KEY": global.api_key,
                    "session_token": req.sessionID,
                }
            })

        return res.send(response.status)
    } catch (err) {
        return res.send(500)
    }
        
})]

// Settings page of user.
exports.settings = [check_auth, asyncHandler(async (req, res, next) => {
    // Get current API Keys.
    var sql = "SELECT api_key, api_secret FROM users WHERE id = $1"
    var values = [req.session.user.id]
    var q = await client.query(sql, values)

    if (q.rows.length == 0) {
        return res.render("panel/settings", { start_funds: await start_funds(req.sessionID), user: req.session.user, error: "Gebruiker niet gevonden." })
    }

    res.render("panel/settings", { start_funds: await start_funds(req.sessionID), user: req.session.user, api_keys: q.rows[0] })
})]

// Initialize the user by retrieving start_funds.
async function init_user(session_token) {
    try {
        const response = await axios.put(`http://127.0.0.1:8080/users/init`, 
            {},
            {
                insecureHTTPParser: true,
                headers: { 
                    "API_KEY": global.api_key,
                    "session_token": session_token,
                }
            })

        if (response.status == 200) {
            return false
        }

        return true
    } catch(err) {
        console.log(err)
        return false
    }
}

// Executed when user submits form to update API keys.
exports.settings_update_api_keys = [check_auth, asyncHandler(async (req, res, next) => {
    // Validate input.
    const v = new Validator(req.body, {
        api_key: "required|minLength:1",
        api_secret: "required|minLength:1",
    })
   
    const valid = await v.check()
    if (!valid) {
        return res.render("panel/settings", { start_funds: await start_funds(req.sessionID), user: req.session.user, api_keys: req.body, error: "Ongeldige invoer." })
    }

    var sql = "UPDATE users SET api_key = $1, api_secret = $2 WHERE id = $3"
    var values = [req.body.api_key, req.body.api_secret, req.session.user.id]
    var q = await client.query(sql, values)
    
    await init_user(req.sessionID)
        
    return res.render("panel/settings", { start_funds: await start_funds(req.sessionID), user: req.session.user, api_keys: req.body, msg: "API Keys zijn geupdate." })

})]

// Called when the user clicks the reset-button on the settings-page.
// All algorithms get deleted and start_funds are reset.
exports.reset_user = [check_auth, asyncHandler(async (req, res, next) => {
    try {
        const response = await axios.put("http://127.0.0.1:8080/users/init", 
            {},
            {
                insecureHTTPParser: true,
                headers: { 
                    "API_KEY": global.api_key,
                    "session_token": req.sessionID,
                }
            })

        if (response.status == 200) {
            return res.redirect("/panel")
        } else {
            return res.render("panel/settings", { error: "Fout bij resetten.", start_funds: await start_funds(req.sessionID), user: req.session.user })
        }
    } catch (err) {
        return res.render("panel/settings", { error: "Fout bij resetten.", start_funds: await start_funds(req.sessionID), user: req.session.user })
    }
})]

// Page with statistics of an algorithm.
exports.algorithm_stats = [check_auth, asyncHandler(async (req, res, next) => {
    try {
        const encoded_start_at = encodeURIComponent(req.params.start_at)
        const history = await axios.get(`http://127.0.0.1:8080/algorithms/${req.params.id}/history/${encoded_start_at}`, 
            {
                insecureHTTPParser: true,
                headers: { 
                    "API_KEY": global.api_key,
                    "session_token": req.sessionID,
                }
            })

        res.render("panel/algorithm-stats", { start_funds: await start_funds(req.sessionID), history: history.data, id: req.params.id, moment: moment, user: req.session.user })
    } catch (err) {
        return res.send(500)
    }
})]

// Function to retrieve Python code of algorithm.
async function get_algorithm_code(req) {
    try {
        const response = await axios.get(`http://127.0.0.1:8080/algorithms/${req.params.id}/code`, 
            {
                insecureHTTPParser: true,
                headers: { 
                    "API_KEY": global.api_key,
                    "session_token": req.sessionID,
                }
            })

        if (response.status == 200) {
            return response.data
        }

        return "Fout bij ophalen code."
    } catch(err) {
        return "Fout bij ophalen code."
    }
}


// Details page of algorithm.
exports.algorithm_details = [check_auth, asyncHandler(async (req, res, next) => {
    // Retrieve start_funds from user by session_token.
    var sql = "SELECT id, description, start_funds_usdt, interval, run_every_sec, user_id FROM algorithms WHERE id = $1"
    var values = [req.params.id]
    var q = await client.query(sql, values)

    // Check if algorithm is found.
    if (q.rows.length == 0) {
        return res.status(404)
    }

    // Retrieve code of algorithm.
    let codeb64 = await get_algorithm_code(req)
    let code = Buffer.from(codeb64, "base64").toString()

    res.render("panel/algorithm-details", { start_funds: await start_funds(req.sessionID), algorithm: q.rows[0], code: code, user: req.session.user })
})]



// Page with BTC candlestick-chart and buy/sell option.
exports.buy_sell = [check_auth, asyncHandler(async (req, res, next) => {
    res.render("panel/buy-sell", { start_funds: await start_funds(req.sessionID), user: req.session.user })
})]

// Executed when user makes order on buy-sell page.
exports.buy_sell_order = [check_auth, asyncHandler(async (req, res, next) => {
    // Validate input.
    const v = new Validator(req.body, {
        action: "required|in:buy,sell",
        amount: "required|min:1",
    })
   
    const valid = await v.check()
    if (!valid) {
        return res.render("panel/buy-sell", { start_funds: await start_funds(req.sessionID), user: req.session.user, error: "Ongeldige invoer." })
    }

    try {
        const response = await axios.post(`http://127.0.0.1:8080/order`, 
            {
                action: req.body.action,
                amount: parseFloat(req.body.amount),
            },
            {
                insecureHTTPParser: true,
                headers: { 
                    "API_KEY": global.api_key,
                    "session_token": req.sessionID,
                }
            })

        if (response.status == 200) {
            return res.render("panel/buy-sell", { start_funds: await start_funds(req.sessionID), user: req.session.user, msg: "Order is uitgevoerd." })
        } else {
            return res.render("panel/buy-sell", { start_funds: await start_funds(req.sessionID), user: req.session.user, error: response.data })
        }
    } catch(err) {
        return res.render("panel/buy-sell", { start_funds: await start_funds(req.sessionID), user: req.session.user, error: "Onbekende fout: " + err })
    }
    
})]

// Endpoint to retrieve klines.
exports.btc_klines = [check_auth, asyncHandler(async (req, res, next) => {
    try {
        const response = await axios.get(`http://127.0.0.1:8080/klines/${req.params.interval}/${req.params.amount}`, 
            {
                insecureHTTPParser: true,
                headers: { 
                    "API_KEY": global.api_key,
                    "session_token": req.sessionID,
                }
            })

        if (response.status == 200) {
            return res.send(response.data)
        } else {
            return res.send(response.status)
        }
    } catch(err) {
        res.status(500).send("BTC candlesticks konden niet opgehaald worden.")
    }
})]

// Ping our Rust API server and check if online.
exports.ping = [check_auth, asyncHandler(async (req, res, next) => {
    try {
        const response = await axios.get("http://127.0.0.1:8080/ping", 
            {
                insecureHTTPParser: true,
                headers: { 
                    "API_KEY": global.api_key,
                    "session_token": req.sessionID,
                }
            })

        return res.send(response.status)
    } catch (err) {
        return res.send(500)
    }
})]

// Ping the exchange API server (e.g Binance) and check if online.
exports.ping_exchange = [check_auth, asyncHandler(async (req, res, next) => {
    try {
        const response = await axios.get("https://testnet.binance.vision/api/v3/ping", 
            {
                insecureHTTPParser: true,
                headers: { 
                    "API_KEY": global.api_key,
                    "session_token": req.sessionID,
                }
            })

        return res.send(response.status)
    } catch (err) {
        return res.send(500)
    }
})]
