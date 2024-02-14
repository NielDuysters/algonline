// Arrays holding data for chart retrieved from API.
let data_total = []
let data_usdt = []
let data_btc = []

// Retrieve algorithm id from URL.
let url_parts = window.location.href.split("/")
let id = url_parts[url_parts.length -2]

// Initialize chart.
// Declare y-axis and data already to dynamically update it depending on selected config.
let chart_axisy = []
let chart_data = []

let ws = null

let chart = new CanvasJS.Chart("chart-container",
{
    title: {
        text: "Algorithm performance",
        fontColor: "#FFFFFF",
        fontFamily: 'Roboto',
    },
    zoomEnabled: true,
    axisY: chart_axisy,
    axisX: {
        labelFontColor: "#FFFFFF",
        valueFormatString: "DD/MM HH:mm",
    },
    legend: {
        fontColor: "#FFFFFF",
    },
    backgroundColor: "#2B3139",
    color: "#FFFFFF",
    data: chart_data,
})

// Retrieve chart data.
document.addEventListener('DOMContentLoaded', () => {
    get_chart()
})


// Render chart again when user toggles specific data-point (total, usdt, btc) on/off.
Array.from(document.getElementsByClassName("charts-control-show-line")).forEach((c) => {
    c.addEventListener("change", () => {
        make_chart(data_total, data_usdt, data_btc)
    })
})

// When the user changes the interval (x-axis for all, hour, daily) retrieve chart-again.
document.getElementById("chart-interval").addEventListener("change", () => {
    get_chart()
})

// Load more order-history rows.
document.getElementById("load-more").addEventListener("click", () => {
    load_more_history_rows()
})

// Function to get chart-data.
function get_chart() {
    // Retrieve selected interval (all, hourly, daily).
    let interval = document.getElementById("chart-interval").value

    data_total = []
    data_usdt = []
    data_btc = []

    // Call API.
    get_algorithm_chart(id, interval)
        .then(response => {
            // Display start_funds of algorithm.
            if (response.length > 0) {
                document.getElementById("algorithm-start-funds-total").textContent = " - " + parseFloat(response[0].total).toFixed(2) + " USDT"
                document.getElementById("algorithm-start-funds-btc").textContent = parseFloat(response[0].btc).toFixed(5)
                document.getElementById("algorithm-start-funds-usdt").textContent = parseFloat(response[0].usdt).toFixed(2)
            }

            // Push data from response in correct array.
            for (var d of response) {
                data_total.push({
                    x: new Date(d.timestamp),
                    y: parseFloat(d.total),
                })
                data_usdt.push({
                    x: new Date(d.timestamp),
                    y: parseFloat(d.usdt),
                })
                data_btc.push({
                    x: new Date(d.timestamp),
                    y: parseFloat(d.btc),
                })
            }
    
            // Update current funds stat.
            update_funds_stats("total", parseFloat(data_total[data_total.length - 1].y))
            update_funds_stats("usdt", parseFloat(data_usdt[data_usdt.length - 1].y))
            update_funds_stats("btc", parseFloat(data_btc[data_btc.length - 1].y))

            // Render chart.
            make_chart(data_total, data_usdt, data_btc)
        })
        .catch(error => {
            console.log("Error: " + error)
        })
}

// Function to render chart.
function make_chart(data_total, data_usdt, data_btc) {
    // Retrieve the selected data-points.
    let total = document.querySelector(".charts-control-show-line#show-total").checked
    let usdt = document.querySelector(".charts-control-show-line#show-usdt").checked
    let btc = document.querySelector(".charts-control-show-line#show-btc").checked
  
    chart_axisy = []
    chart_data = []

    if (total) {
        chart_axisy.push({
            title: "Total",
            titleFontColor: "#FFFFFF",
            labelFontColor: "#FFFFFF",
            visible: total,
        })
        chart_data.push({
            type: "line",
            name: "Total",
            showInLegend: total,
            color: "#0D3B9D",
            visible: total,
            markerSize: 0,
            axisYIndex: chart_axisy.length -1,
            dataPoints: data_total,
        })
    }
    if (usdt) {
        chart_axisy.push({
            title: "USDT",
            titleFontColor: "#FFFFFF",
            labelFontColor: "#FFFFFF",
            visible: usdt,
        })
        chart_data.push({
            type: "line",
            name: "USDT",
            color: "#228F8F",
            showInLegend: usdt,
            visible: usdt,
            markerSize: 0,
            axisYIndex: chart_axisy.length -1,
            dataPoints: data_usdt,
        })
    }
    if (btc) {
        chart_axisy.push({
            title: "BTC",
            titleFontColor: "#FFFFFF",
            labelFontColor: "#FFFFFF",
            visible: btc,
        })
        chart_data.push({
            type: "line",
            name: "BTC",
            color: "#F79627",
            showInLegend: btc,
            visible: btc,
            markerSize: 0,
            axisYIndex: chart_axisy.length -1,
            dataPoints: data_btc,
        })
    }

    chart.options.data = chart_data
    chart.options.axisY = chart_axisy
    chart.render()

    websocket_connection()
}

let table = document.getElementsByClassName("table")[0]
let head_row = table.getElementsByClassName("row")[0]
let head_row_cells = head_row.getElementsByClassName("cell")

// Get history and chart datapoints live from websocket.
function websocket_connection() {

    if (ws && ws.readyState == WebSocket.OPEN) {
        ws.close()
    }

    ws = new WebSocket("ws://127.0.0.1:3001/algorithm-stats?id=" + id)

    ws.onmessage = (event) => {
        let json = JSON.parse(event.data)

        if (json.response_type == "HistoryRow") {
            add_row_to_history(json.json, true)
        }
        if (json.response_type == "ChartDataPoint") {
            add_datapoint_to_chart(json.json)
        }
    }
}

// Function to add a row-order to the table.
// We can prepend the most recent rows and append the older requested rows.
function add_row_to_history(data, prepend = false) {

    let row = document.createElement("div")
    row.classList.add("row")
    row.classList.add(data.action.toLowerCase())

    row.setAttribute("timestamp", data.created_at.replace("T", " "))

    let date = new Date(data.created_at)
    var cell = document.createElement("div")
    cell.classList.add("cell")
    cell.textContent = date.getFullYear() + "/" + prepend_zero_to_date(date.getMonth() + 1) + "/" + prepend_zero_to_date(date.getDate()) + " " + prepend_zero_to_date(date.getHours()) + ":" + prepend_zero_to_date(date.getMinutes())
    cell.style.width = head_row_cells[0].offsetWidth + "px"
    row.appendChild(cell)
    
    var cell = document.createElement("div")
    cell.classList.add("cell")
    cell.textContent = data.order_id
    cell.style.width = head_row_cells[1].offsetWidth + "px"
    row.appendChild(cell)
    
    var cell = document.createElement("div")
    cell.classList.add("cell")
    cell.textContent = data.action
    cell.style.width = head_row_cells[2].offsetWidth + "px"
    row.appendChild(cell)
    
    var cell = document.createElement("div")
    cell.classList.add("cell")
    cell.textContent = parseFloat(data.btc).toFixed(5)
    cell.style.width = head_row_cells[3].offsetWidth + "px"
    row.appendChild(cell)
    
    var cell = document.createElement("div")
    cell.classList.add("cell")
    cell.textContent = parseFloat(data.usdt).toFixed(2)
    cell.style.width = head_row_cells[4].offsetWidth + "px"
    row.appendChild(cell)
    
    var cell = document.createElement("div")
    cell.classList.add("cell")
    cell.textContent = parseFloat(data.btc_price).toFixed(2)
    cell.style.width = head_row_cells[5].offsetWidth + "px"
    row.appendChild(cell)

    if (prepend) {
        document.querySelector(".table .table-body").prepend(row)
    } else {
        document.querySelector(".table .table-body").append(row)
    }
}

// Add datapoint to chart.
function add_datapoint_to_chart(data) {
    let interval = document.getElementById("chart-interval").value
   
    if (chart.data.length == 0) {
        return
    }

    let latest_timestamp = chart.data[0].dataPoints[chart.data[0].dataPoints.length - 1]["x"]
    switch(interval) {
        case "hourly":
            latest_timestamp.setHours(latest_timestamp.getHours() + 1)
            break
        case "daily":
            latest_timestamp.setHours(latest_timestamp.getHours() + 24)
            break
        default:
            break
    }

    let timestamp = new Date(data.timestamp)
    if (timestamp < latest_timestamp) {
        return 
    }

    for (var d of chart.data) {
        if (d.name.toLowerCase() == "total") {
            d.dataPoints.push({
                x: timestamp,
                y: parseFloat(data.total),
            })

            update_funds_stats("total", parseFloat(data.total))
        }
        
        if (d.name.toLowerCase() == "usdt") {
            d.dataPoints.push({
                x: timestamp,
                y: parseFloat(data.usdt),
            }) 
            
            update_funds_stats("usdt", parseFloat(data.total))
        }
        
        if (d.name.toLowerCase() == "btc") {
            d.dataPoints.push({
                x: timestamp,
                y: parseFloat(data.btc),
            }) 
            
            update_funds_stats("btc", parseFloat(data.btc))
        }
    }

    chart.render()
}

// Function to update the spans/div showing the most recent funds of the algorithm.
function update_funds_stats(data_type, value) {
    switch (data_type) {
        case "total":
            if (data_total.length == 0) { return }

            var start_value = data_total[0].y
            var span = document.getElementById("algorithm-current-balance-total")
            var percentage = (((value + 1) - (start_value + 1)) / (start_value + 1)) * 100
            var profit = start_value <= value
            span.style.color = profit ? "#088A08" : "#FF0000"
            span.textContent = value.toFixed(2) + " USDT (" + (profit ? "+" : "") + percentage.toFixed(2) + "%)"
            break

        case "usdt":
            if (data_usdt.length == 0) { return }

            var start_value = data_usdt[0].y
            var span = document.getElementById("algorithm-current-balance-usdt")
            var percentage = (((value + 1) - (start_value + 1)) / (start_value + 1)) * 100
            var profit = start_value <= value
            span.style.color = profit ? "#088A08" : "#FF0000"
            span.textContent = value.toFixed(2) + " (" + (profit ? "+" : "") + percentage.toFixed(2) + "%)"
            break
        
        case "btc":
            if (data_btc.length == 0) { return }

            var start_value = data_btc[0].y
            var span = document.getElementById("algorithm-current-balance-btc")
            var percentage = (((value + 1) - (start_value + 1)) / (start_value + 1)) * 100
            var profit = start_value <= value
            span.style.color = profit ? "#088A08" : "#FF0000"
            span.textContent = value.toFixed(5) + " (" + (profit ? "+" : "") + percentage.toFixed(2) + "%)"
            break
    }
}

// Function to load more history rows.
function load_more_history_rows() {
    let last_row = Array.from(document.querySelectorAll(".table .table-body .row")).pop()
    let timestamp = last_row.getAttribute("timestamp")

    get_algorithm_history(id, timestamp)
        .then(response => {
            for (var d of response) {
                add_row_to_history(d, false)
            }
        })
        .catch(error => {
            console.log("Error: " + error)
        })

}

function prepend_zero_to_date(str) {
    let string = String(str)

    if (string.length == 1) {
        return String("0") + String(string)
    }

    return string
}
