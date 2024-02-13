const asyncHandler = require("express-async-handler")
const bcrypt = require("bcrypt")



exports.login = asyncHandler(async (req, res, next) => {
    var sql = "SELECT id, name, password FROM users WHERE name = $1"
    var values = [req.body.username]
    var q = await client.query(sql, values)

    if (q.rows.length == 0) {
        res.render("index", { error: "Geen gebruiker gevonden." })
    }

    const phash = q.rows[0].password
    const ok = await bcrypt.compare(req.body.password, phash)

    if (!ok) {
        res.render("index", { error: "Fout wachtwoord." });
    }
});

exports.register = asyncHandler(async (req, res, next) => {
    // Check if username already exists.
    var sql = "SELECT id FROM users WHERE name = $1"
    var values = [req.body.username]
    var q = await client.query(sql, values)
    
    if (q.rows.length > 0) {
        res.render("index", { error: "Gebruiker bestaat al." })
    }


    // Register user.
    const salt = await bcrypt.genSalt(10)
    const phash = await bcrypt.hash(req.body.password, salt)

    var sql = "INSERT INTO users (name, password) VALUES ($1, $2)"
    var values = [req.body.username, phash]
    await client.query(sql, values)
    res.send("Registered")
});
