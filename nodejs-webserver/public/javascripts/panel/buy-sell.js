// Initiate empty data-array for candlesticks and empty variable holding websocket.
let data = []
let ws = null

// This will get the value of the BTC price in our first data point.
let first_btc_value = null

// Initialize chart
var chart = new CanvasJS.Chart("chart-container",
{

    title:{
        text: "BTCUSDT",
        fontColor: "#FFFFFF",
        fontFamily: 'Roboto',
    },
    zoomEnabled: true,
    axisY: {
        title: "USDT",
        labelFontColor: "#FFFFFF",
    },
    axisX: {
        labelFontColor: "#FFFFFF",
        intervalType: "second",
        interval: 1,
    },
    legend: {
        fontColor: "#FFFFFF",
    },
    backgroundColor: "#2B3139",
    color: "#FFFFFF",
    data: [{
        type: "line",
        markerSize: 0,
        color: "#F79627",
        dataPoints: data,
    }]
})

// When an interval like 5m or 1h is selected, we only
// insert data from the websocket every 5m/1h.
let insert_data = false
let insert_interval_handler = null

function update_frequency(ms) {
    if (insert_interval_handler) {
        clearInterval(insert_interval_handler)
    }

    if (ms > 0) {
        insert_interval_handler = setInterval(() => {
            insert_data = true
        }, ms)
    } else {
        insert_data = true
    }

}

// When a new interval is selected we need to open a new websocket connection
// for it.
function update_websocket(interval) {
    
    if (ws && ws.readyState == WebSocket.OPEN) {
        ws.close()
    }

    ws = new WebSocket("ws://127.0.0.1:3001/btc-candlestick?interval=" + interval)

    switch (interval) {
        case "1m":
            update_frequency(60000)
            break
        case "5m":
            update_frequency(60000 * 5)
            break
        case "1h":
            update_frequency(60000 * 60)
            break
        case "1d":
            update_frequency(60000 * 60 * 24)
            break
        default:
            update_frequency(0)
    }

    first_btc_value = null
    ws.onmessage = (event) => {
        let json = JSON.parse(event.data)
        
        if (first_btc_value == null) {
            first_btc_value = parseFloat(json.close)
        }

        let btc_value = parseFloat(json.close)
        let percentage = ((btc_value - first_btc_value) / first_btc_value) * 100
        let profit = first_btc_value <= btc_value

        document.getElementById("btc-value").textContent = btc_value.toFixed(5) + " (" +  (profit ? "+" : "") + percentage.toFixed(2) + "%)"
        document.getElementById("btc-value").style.color = profit ? "#088A08" : "#FF0000" 

        if (insert_data) {
            data.push({
                x: new Date(json.timestamp),
                y: parseFloat(btc_value),
            })

            if (interval != "1s") {
                insert_data = false
            }
        }

        if (data.length > 20) {
            data.shift()
        }

        chart.options.data[0].dataPoints = data
        chart.render()
    }
}

update_websocket("1s")

// When a new interval is selected prefill the chart with some datapoints
// from the past.
function insert_start_data() {
    const interval = document.getElementById("interval").value

    // Call API to get klines.
    get_klines(interval, 20)
        .then(response => {
            data = []
            update_websocket(interval)
            switch (interval) {
                case "1m":
                    chart.options.axisX.interval = 1
                    chart.options.axisX.intervalType = "minute"
                    chart.options.axisX.valueFormatString = "HH:mm"
                    break
                case "5m":
                    chart.options.axisX.interval = 5
                    chart.options.axisX.intervalType = "minute"
                    chart.options.axisX.valueFormatString = "HH:mm"
                    break
                case "1h":
                    chart.options.axisX.interval = 1
                    chart.options.axisX.intervalType = "hour"
                    chart.options.axisX.valueFormatString = "HH:00"
                    break
                default:
                    chart.options.axisX.interval = 1
                    chart.options.axisX.intervalType = "second"
                    chart.options.data[0].dataPoints = data
                    chart.render()
                    return
            }
            
            for (var d of response) {
                data.push({
                    x: new Date(d[0]),
                    y: parseFloat(d[4]),
                })
            }

            chart.options.data[0].dataPoints = data
            chart.render()
        })
        .catch(error => {
            console.log("Error: " + error)
        })
}

Array.from(document.querySelectorAll("#chart-controls select")).forEach((e) => {
    e.addEventListener("change", (ev) => {
        insert_start_data()
    })
})
