# Architecture

In deze PDF ([Architectuur PDF](architecture.pdf)) vind je een simpel schema over de architectuur gebruikt voor het realiseren van dit project.

Je zal zien dat we enkele onderdelen kunnen onderscheiden.

## NodeJS webserver
De voornaamste taken van de NodeJS webserver gaan omtrent de web-interface van het paneel en het forwarden van requests naar de Rust-server. Dit onderdeel bevat enkele views die de pagina's voorstellen waarop de gebruiker o.a zijn algoritmes ziet
en formulieren heeft om deze toe te voegen. Daarnaast beschikt deze ook over controllers die de logica bevatten om calls te doen naar de Rust-server en de PostgreSQL database.

De NodeJS webserver maakt nooit directe calls naar de Binance API. Wanneer de gebruiker inlogt wordt er een session-token gegenereerd, de gebruiker maakt dan requests (REST of Websocket) naar deze server.
Op diens buurt forward de webserver de request naar de Rust-server met de session-token in de HTTP-header. De Rust-server kan voor dit request dan de juiste API-keys voor de Binance API uit de database
halen om calls te doen naar de Binance API.

## Rust server
De Rust-server bevat een HTTP-server en een Websocket-server. Deze ontvangt de requests van de NodeJS webserver die de session-token meestuurt. De Rust-server haalt de API keys voor de Binance API van deze gebruiker uit de databank.
Daarmee kan de Rust-server calls doen naar de Binance API.

Deze server is ook verantwoordelijk voor het starten/stoppen van algoritmes.

## PyExecutor
De trading algortimes worden geschreven in Python. Deze Python-code wordt in Rust uitgevoerd doormiddel van PyO3. We voeren de Python-code uit in een apart process zodat we
op OS-level veiligheidsmaatregelen kunnen nemen om de arbitrary code veilig uit te voeren.

Elk algoritme dat wordt uitgevoerd is dus een apart process. De PyExecutor en Rust-api communiceren d.m.v IPC-communicatie (shared memory en unix sockets).

## PostgreSQL database
De database-engine gebruikt voor dit project is PostgreSQL. De databank is echter niet heel geavanceerd. Er wordt gebruikt gemaakt van basic queries en views.

## Exchange API
De gebruikte Exchange API is die van Binance. We gebruiken zowel de REST-API en de Websocket-API. Door gebruik te maken van de Websocket-API kunnen we sneller data opvragen in een stream
of orders uitvoeren met minder latency.
