// Controller for calls to TradeAlgorithms-endpoint.
// Used to e.g retrieve, delete, start/stop algorithms.

const asyncHandler = require("express-async-handler")
const { Validator } = require('node-input-validator')
const Algorithm = require("../models/algorithm")
const axios = require('axios')

// Check if user is authenticated.
const check_auth = (req, res, next) => {
    if (!req.session.user) {
        return res.send(401)
    }

    next()
}

// Get all algorithms.
exports.retrieve = [check_auth, asyncHandler(async (req, res, next) => {
    try {
        const response = await axios.get(`http://127.0.0.1:8080/algorithms/${req.params.id}`, 
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

        return res.send(response.status)
    } catch(err) {
        res.status(500).send("Onbekende fout opgetreden.")
    }
})]

// Get Python code of algorithm.
exports.code = [check_auth, asyncHandler(async (req, res, next) => {
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
            return res.status(200).send(response.data)
        }

        return res.send(response.status)
    } catch(err) {
        res.status(500).send("Onbekende fout opgetreden.")
    }
})]

// Start or Stop an algorithm.
exports.toggle_running = [check_auth, asyncHandler(async (req, res, next) => {
    try {
        const response = await axios.post(`http://127.0.0.1:8080/algorithms/${req.params.id}/${req.params.start_or_run}`, 
            {},
            {
                insecureHTTPParser: true,
                headers: { 
                    "API_KEY": global.api_key,
                    "session_token": req.sessionID,
                }
            })

        res.send(response.status)
    } catch (err) {
        res.status(response.status).send(err.message)
    }
})]

// Delete an algorithm.
exports.remove = [check_auth, asyncHandler(async (req, res, next) => {
    try {
        const response = await axios.delete(`http://127.0.0.1:8080/algorithms/${req.params.id}`, 
            {
                insecureHTTPParser: true,
                headers: { 
                    "API_KEY": global.api_key,
                    "session_token": req.sessionID,
                }
            })

        res.send(response.status)
    } catch(err) {
        res.status(500).send("Algoritme kon niet verwijderd worden.")
    }
})]

// Reset an algorithm.
exports.reset = [check_auth, asyncHandler(async (req, res, next) => {
    try {
        const response = await axios.put(`http://127.0.0.1:8080/algorithms/${req.params.id}/reset`, 
            {},
            {
                insecureHTTPParser: true,
                headers: { 
                    "API_KEY": global.api_key,
                    "session_token": req.sessionID,
                }
            })

        res.send(response.status)
    } catch(err) {
        console.log(err)
        res.status(500).send("Algoritme kon niet gereset worden.")
    }
})]

// Get chart-data of algorithm.
exports.chart = [check_auth, asyncHandler(async (req, res, next) => {
    try {
        const response = await axios.get(`http://127.0.0.1:8080/algorithms/${req.params.id}/chart/${req.params.interval}`, 
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
        console.log("error")
        console.log(err)
        res.status(500).send("Chart van algoritme kon niet opgehaald worden.")
    }
})]

// Get order-history of algorithm.
exports.history = [check_auth, asyncHandler(async (req, res, next) => {
    try {
        const response = await axios.get(`http://127.0.0.1:8080/algorithms/${req.params.id}/history/${req.params.start_at}`, 
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
        res.status(500).send("History van algoritme kon niet opgehaald worden.")
    }
})]

