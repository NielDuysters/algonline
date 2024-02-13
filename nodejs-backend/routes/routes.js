var express = require('express');
var router = express.Router();
const { Validator } = require('node-input-validator');

const usercontroller = require("../controllers/usercontroller");

/* GET home page. */
router.get('/', function(req, res, next) {
  res.render('index', { title: 'Express' });
});

router.post("/login", function(req, res, next) {
    
    const v = new Validator(req.body, {
        username: "required|minLength:3",
        password: "required|minLength:5",
    })

    v.check().then((m) => {
        if (!m) {
            res.render("index", { title: "Express", error: "Ongeldige invoer." })
        }
    })

    if ('login' in req.body) {
        usercontroller.login(req, res, next)
    }
    else if ('register' in req.body) {
        usercontroller.register(req, res, next)
    }
    else {
        res.status(404).send("Not found")
    }

});

module.exports = router;
