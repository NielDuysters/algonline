// Check if user is authenticated.
const check_auth = async (req, res, next) => {
    if (!req.session.user) {
        return res.sendStatus(401)
    }


    // Check if session tokens match.
    var sql = "SELECT session_token FROM users WHERE id = $1"
    var values = [req.session.user.id]
    var q = await client.query(sql, values)

    // Check if user is found.
    if (q.rows.length == 0) {
        return res.sendStatus(401)
    }

    if (q.rows[0].session_token != req.sessionID) {
        return res.redirect('/unauthenticated')
    }

    next()
}

module.exports = check_auth
