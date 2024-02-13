const { Validator } = require('node-input-validator')
const express = require('express')
const router = express.Router()

const algorithmcontroller = require("../controllers/algorithmcontroller")
const panelcontroller = require("../controllers/panelcontroller")
const usercontroller = require("../controllers/usercontroller")

router.get('/', function(req, res, next) {
  res.render('index', { user: req.session.user })
})

// User.
router.post("/login", usercontroller.login_form)
router.get("/user/balance", usercontroller.balance)

// Panel.
router.get("/panel", function(req, res, next) {
    res.redirect("/panel/algorithms")
})
router.get("/panel/algorithms", panelcontroller.index)
router.get("/panel/algorithms/add", panelcontroller.add)
router.post("/panel/algorithms/add", panelcontroller.add_form)
router.get("/panel/algorithms/:id/stats", panelcontroller.algorithm_stats)
router.get("/panel/algorithms/:id/details", panelcontroller.algorithm_details)
router.get("/panel/settings", panelcontroller.settings)
router.get("/panel/settings/reset", panelcontroller.reset_user)
router.get("/panel/buy-sell", panelcontroller.buy_sell)
router.post("/panel/buy-sell", panelcontroller.buy_sell_order)
router.get("/ping", panelcontroller.ping)
router.get("/ping-exchange", panelcontroller.ping_exchange)
router.post("/panel/settings/keys", panelcontroller.settings_update_api_keys)

router.get("/klines/:interval/:amount", panelcontroller.btc_klines)

// Algorithms.
router.get("/algorithms/:id", algorithmcontroller.retrieve)
router.get("/algorithms/:id/chart/:interval", algorithmcontroller.chart)
router.get("/algorithms/:id/history/:start_at", algorithmcontroller.history)
router.post("/algorithms/:id/:start_or_run", algorithmcontroller.toggle_running)
router.delete("/algorithms/:id", algorithmcontroller.remove)
router.put("/algorithms/:id/reset", algorithmcontroller.reset)

module.exports = router
