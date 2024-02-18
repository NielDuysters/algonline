# Architecture

In the following PDF ([Architecture PDF](architecture.pdf)) you'll find a simple scheme giving an overview of the architecture of this project.

You'll notice we can distinguish several components.

## NodeJS webserver
The main task of the NodeJS webserver is serving the web-interface of the platform. It contains views with the HTML, CSS and client-side Javascript to i.a serve the user forms to add algorithms or execute orders. As well the code to display the chart and overview of the algorithms. Besides the view the NodeJS component also holds the logic in the controllers to handle the requests send by the user.

The NodeJS server never makes calls directly to the Binance API. Instead it acts as intermediate between the client and the Rust-server. The actions a user executes (e.g starting an algorithm) is forwarded by NodeJS to the Rust-server.
Because authentication and session management is handled in NodeJS it sends the user's session token to the Rust-server to authenticate the requests.

## Rust server
The Rust-server contains a 'low-level' HTTP-server (made without any frameworks) and a websocket server. The HTTP-server acts mainly as REST-api to execute the calls the user makes. It also contains all the logic to run the
algorithms and retrieve data (charts, history) about these.

## PyExecutor
The user writes his trading algorithms in Python. We use PyO3 to execute the Python code directly in Rust. The reason the execution is handled in a separate binary is because this way we can execute each algorithm in a separate process. This way we can apply OS-level security mechanism to secury the execution of arbitrary code.

## PostgreSQL database
The used database-engine is PostgreSQL. The database-architecture in this project is not advanced. We use a few features like views, listeners and commom table expressions.

## Exchange API
To retrieve the data about the market we use the Binance API. We communicate to the Binance API using both REST-calls and websocket streams. By using websocket streams we can receive data and execute orders after interpreting that data with less latency.
