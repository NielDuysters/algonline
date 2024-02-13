
// We display the current balance of the user in the panel. We update this balance every 1500ms.
show_user_balance()
setInterval(show_user_balance, 1500)

// Check online status of servers.
check_server_status()
setInterval(check_server_status, 10000)

// Function to show latest user_balance.
function show_user_balance() {
    
    // Call API to retrieve current user balance.
    get_user_balance()
        .then(response => {
            // Save usdt, btc, and total.
            const usdt = parseFloat(response[0])
            const btc = parseFloat(response[1])
            const total = parseFloat(response[2])

            // Set current USDT balance.
            const start_funds_usdt = parseFloat(document.getElementById("start-funds-usdt").getAttribute("data-value"))
            let percentage = ((usdt - start_funds_usdt) / start_funds_usdt) * 100
            let profit = start_funds_usdt <= usdt
            const usdt_span = document.getElementById("current-balance-usdt")
            usdt_span.textContent = usdt.toFixed(2) + " (" + (profit ? "+" : "") + percentage.toFixed(2) + "%)"
            usdt_span.style.color = profit ? "#088A08" : "#FF0000"

            // Set current BTC balance.
            const start_funds_btc = parseFloat(document.getElementById("start-funds-btc").getAttribute("data-value"))
            percentage = ((btc - start_funds_btc) / start_funds_btc) * 100
            profit = start_funds_btc <= btc
            const btc_span = document.getElementById("current-balance-btc")
            btc_span.textContent = btc.toFixed(5) + " (" + (profit ? "+" : "") + percentage.toFixed(2) + "%)"
            btc_span.style.color = profit ? "#088A08" : "#FF0000"
            
            // Set current total balance.
            const start_funds_total = parseFloat(document.getElementById("start-funds-total").getAttribute("data-value"))
            percentage = ((total - start_funds_total) / start_funds_total) * 100
            profit = start_funds_total <= total
            const total_span = document.getElementById("current-balance-total")
            total_span.textContent = total.toFixed(2) + " USDT (" + (profit ? "+" : "") + percentage.toFixed(2) + "%)"
            total_span.style.color = profit ? "#088A08" : "#FF0000"
        })
        .catch(error => {
            console.log("Error: " + error)
        })
}

// Ping all the servers and show status.
function check_server_status() {
    
    // Rust API server.
    ping_api()
        .then(response => {
            let server_status = document.getElementById("server-status")
            server_status.textContent = "Online"
            server_status.style.color = "#088A08"
        })
        .catch(error => {
            let server_status = document.getElementById("server-status")
            server_status.textContent = "Offline"
            server_status.style.color = "#FF0000"
        })
    
    // Exchange API server.
    ping_exchange()
        .then(response => {
            let exchange_status = document.getElementById("exchange-status")
            exchange_status.textContent = "Online"
            exchange_status.style.color = "#088A08"
        })
        .catch(error => {
            let exchange_status = document.getElementById("exchange-status")
            exchange_status.textContent = "Offline"
            exchange_status.style.color = "#FF0000"
        })

}

// The panel has little question-mark or details-icons beside elements requiring more explanation.
// Hovering over such a icon will make extra text popup.
Array.from(document.getElementsByClassName("text-popup-button")).forEach((btn) => {
    btn.addEventListener("mouseenter", function(e) {
        const div = document.createElement("div")
        div.classList.add("text-popup")
        div.style.position = "absolute"
        div.style.left = parseInt(e.currentTarget.offsetLeft + 25) + "px"
        div.style.top = parseInt(e.currentTarget.offsetTop - 8) + "px"
        div.textContent = e.currentTarget.getAttribute("data-text")
        document.body.appendChild(div)

        e.currentTarget.addEventListener("mouseleave", function(e) {
            div.remove()
        })
    })
})

// Highlight the active tab in the navigation.
Array.from(document.querySelectorAll("#panel-nav a")).forEach((i) => {
    i.parentElement.classList.remove("active")
    if (window.location.pathname.includes(i.getAttribute("href"))) {
       i.parentElement.classList.add("active") 
    }
})
