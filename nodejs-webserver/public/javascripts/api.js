// Functions to call the NodeJS API. Note that the front-end never calls
// our Rust-API directly.

// Ping our api server.
function ping_api() {
    return new Promise((resolve, reject) => {
        let xhr = new XMLHttpRequest()
        xhr.onreadystatechange = function() {
            if (xhr.readyState == XMLHttpRequest.DONE) {
                if (xhr.status == 200) {
                    resolve(true)
                } else {
                    reject("Error: " + xhr.responseText)
                }
            }
        }

        xhr.open("GET", "http://127.0.0.1:3000/ping", true)
        xhr.send()
    })
}

// Ping exchange server.
function ping_exchange() {
    return new Promise((resolve, reject) => {
        let xhr = new XMLHttpRequest()
        xhr.onreadystatechange = function() {
            if (xhr.readyState == XMLHttpRequest.DONE) {
                if (xhr.status == 200) {
                    resolve(true)
                } else {
                    reject("Error: " + xhr.responseText)
                }
            }
        }

        xhr.open("GET", "http://127.0.0.1:3000/ping-exchange", true)
        xhr.send()
    })
}

// Get data of algorithm.
function get_algorithm(id) {
    return new Promise((resolve, reject) => {
        let xhr = new XMLHttpRequest()
        xhr.onreadystatechange = function() {
            if (xhr.readyState == XMLHttpRequest.DONE) {
                if (xhr.status == 200) {
                    resolve(JSON.parse(xhr.responseText))
                } else {
                    reject("Error: " + xhr.responseText)
                }
            }
        }

        xhr.open("GET", "http://127.0.0.1:3000/algorithms/" + id, true)
        xhr.send()
    })
}

// Get chart-data of algorithm.
function get_algorithm_chart(id, interval) {
    return new Promise((resolve, reject) => {
        let xhr = new XMLHttpRequest()
        xhr.onreadystatechange = function() {
            if (xhr.readyState == XMLHttpRequest.DONE) {
                if (xhr.status == 200) {
                    resolve(JSON.parse(xhr.responseText))
                } else {
                    reject("Error: " + xhr.responseText)
                }
            }
        }

        xhr.open("GET", "http://127.0.0.1:3000/algorithms/" + id + "/chart/" + interval, true)
        xhr.send()
    })
}

// Get order history of algorithm.
function get_algorithm_history(id, start_at) {
    return new Promise((resolve, reject) => {
        let xhr = new XMLHttpRequest()
        xhr.onreadystatechange = function() {
            if (xhr.readyState == XMLHttpRequest.DONE) {
                if (xhr.status == 200) {
                    resolve(JSON.parse(xhr.responseText))
                } else {
                    reject("Error: " + xhr.responseText)
                }
            }
        }

        xhr.open("GET", "http://127.0.0.1:3000/algorithms/" + id + "/history/" + encodeURIComponent(start_at), true)
        xhr.send()
    })
}

// Get current balance of user.
function get_user_balance() {
    return new Promise((resolve, reject) => {
        let xhr = new XMLHttpRequest()
        xhr.onreadystatechange = function() {
            if (xhr.readyState == XMLHttpRequest.DONE) {
                if (xhr.status == 200) {
                    resolve(JSON.parse(xhr.responseText))
                } else {
                    reject("Error: " + xhr.responseText)
                }
            }
        }

        xhr.open("GET", "http://127.0.0.1:3000/user/balance", true)
        xhr.send()
    })
}

// Get klines of BTCUSDT.
function get_klines(interval, amount) {
    return new Promise((resolve, reject) => {
        let xhr = new XMLHttpRequest()
        xhr.onreadystatechange = function() {
            if (xhr.readyState == XMLHttpRequest.DONE) {
                if (xhr.status == 200) {
                    resolve(JSON.parse(xhr.responseText))
                } else {
                    reject("Error: " + xhr.responseText)
                }
            }
        }

        xhr.open("GET", "http://127.0.0.1:3000/klines/" + interval + "/" + amount, true)
        xhr.send()
    })
}
