# Rust-server
Dit is de core van het project. De Rust-server bevat een low-level HTTP server, websocket server, api, en logica om de algoritmes uit te voeren.

**Vaardigeden gebruikt (o.a):**
- Low-level HTTP requests.
- Websockets.
- Streams.
- IPC-communicatie.
- Multi-threading.
- Asynchronous programming.
- API calls.
- Database connections.
- Session management.
- ...

Deze Rust-server bevat een zelfgemaakte HTTP-server, normaal zou hiervoor uiteraard een framework voor gebruikt worden maar op deze manier kon ik mijn begrip omtrent HTTP aantonen.
Er wordt een TcpListener gebruikt, wanneer deze een stream binnenkrijgt interpreteren we deze stream als string en parsen het als een Http-object. We halen de headers, parameters en body manueel uit de stream.

De programmeur kan vervolgens zelf routes configureren. De Http-server kijkt in de path van het Http-object of er een route matched met de gevraagde path, als dat het geval is wordt die route uitgevoerd die vervolgens een HttpResponse returned dat we
terugschrijven naar de stream.

Daarnaast is er ook een aparte TcpListener voor websockets. De gebruiker kan een message sturen naar deze websocket. Dit request bevat een action-parameter. De websocket functie gelinkt aan deze action wordt uitgevoerd en de stream blijft open. Vervolgens kunnen we
data heen en weer over de stream sturen.

De Rust-server bevat ook alle logica om algoritmes te starten en te stoppen. Wanneer een algoritme wordt gestart wordt de configuratie van dit algoritme opgevraagd uit de databank. Vervolgens openen we een websocket-stream naar de Binance API om een stream van candlesticks
te ontvangen. Een apart process van de PyExecutor die de Python-code uitvoert d.m.v PyO3 ontvangt deze data door een unix socket. De PyExecutor stuurt het resultaat van de Python code terug en de Rust-server doet al dan niet een order naar de Binance API.
