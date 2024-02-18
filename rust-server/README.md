# Rust-server
This compoment is the core of our project containing most of the logic.

**Used skills (i.a):**
- Low-level HTTP requests.
- Websockets.
- Streams.
- IPC-communication (shmem + unix sockets).
- Multi-threading.
- Asynchronous programming.
- API calls.
- Signed API calls.
- Database connections.
- Session management.
- ...

This server contains a 'low-level' HTTP-server (made without any framework). A TcpListener is bound to a port, when the stream receives a request we manually serialize the incoming string into a Http-object. Among other things this component also contains a websocket-server, API, interface to communicate with the Binance API, logic to execute algorithms and retrieve performance data,...

The programmer can configures his own routes in the HTTP-server. When the incoming request is serialized to a Http-object we check if a path exists for the requested endpoint in the Http-object. If so, we execute the function configured for the requested path and write the response back to the stream.

For the websockets a separate TcpListener is used. The websocket-endpoints are used to feed data to the client with as little latency as possible and without the need to refresh a page.

Finally this also contains all the logic to start and execute a trading algorithm and process the result. When an algorithm is started a websocket stream to the Binance API is initiated to retrieve candlesticks. This data is fed to the trading algorithm using Unix Sockets. In PyExecutor the Python code is executed and the returned result is sent back over the Unix Socket so it can be processed by the Rust-server.

## PyExecutor
PyExecutor is a separate binary responsible for executing the Python code. The reason it is a separate binary is so we are able to isolate this process on OS-level to secure the execution of arbitrary code. The Rust-server and PyExecutor communicate with each other using IPC (shared memory and Unix sockets).
