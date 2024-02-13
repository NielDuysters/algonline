const { WebSocketServer, WebSocket } = require('ws')
const session = require('express-session')
const { parse } = require('url')

function handle_websocket(server, sessionMiddleware) {

    const wss = new WebSocketServer({ noServer: true })

    server.on('upgrade', function upgrade(request, socket, head) {
        sessionMiddleware(request, {}, () => {
            
            // Check of client is authenticated.
            if (!request.session || !request.session.user) {
                socket.destroy()
                return
            }
            
            // Extract pathname and parameters.
            const { pathname, query } = parse(request.url, true)

            // Check what functionality of the Websocket API the client requests.
            switch (pathname) {
                case "/btc-candlestick":
                    let interval = query.interval
                    wss.handleUpgrade(request, socket, head, function done(ws) {
                        btc_candlestick(ws, request.session.id, interval)
                    })
                    break
                case "/algorithm-stats":
                    let id = query.id
                    wss.handleUpgrade(request, socket, head, function done(ws) {
                        algorithm_stats(ws, request.session.id, id)
                    })
                    break
                default:
                    socket.destroy()
            }
        });
    });
}

function btc_candlestick(ws, session_token, interval) {
    const api_ws = new WebSocket('ws://127.0.0.1:8081');
    api_ws.on('error', console.error);

    if (interval == null) {
        interval = "1s"
    }

    api_ws.on('open', () => {
        const request = {
            action: "btc-candlestick",
            session_token: session_token,
            api_key: global.api_key,
            params: {
                interval: interval
            }
        }

        api_ws.send(JSON.stringify(request))
    })

    api_ws.on('message', (data) => {
        ws.send(data.toString())
    });
    
    ws.on('close', () => {
        //api_ws.close()
    });

}

function algorithm_stats(ws, session_token, id) {
    const api_ws = new WebSocket('ws://127.0.0.1:8081');
    api_ws.on('error', console.error);

    api_ws.on('open', () => {
        const request = {
            action: "algorithm-stats",
            session_token: session_token,
            api_key: global.api_key,
            params: {
                id: id,
            }
        }

        api_ws.send(JSON.stringify(request))
    })

    api_ws.on('message', (data) => {
        ws.send(data.toString())
    });
    
    ws.on('close', () => {
       // api_ws.close()
    });
}

module.exports = handle_websocket
