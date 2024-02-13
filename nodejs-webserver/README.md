# NodeJS webserver

Het web-paneel zelf is gemaakt in NodeJS. Deze server communiceert zelf niet met de Binance API. Wat deze wel doet is het inloggen en registreren van gebruikers. Ook alle requests die de gebruiker doet om bijvoorbeeld een
algoritme toe te voegen of te starten wordt door deze server geforward naar de Rust-server waar onze eigen API draait.

Doordat de gebruiker is ingelogd kan deze server de session-token als parameter of in de header toevoegen aan de requests naar de Rust-server om zo de juiste Binance API-keys gelinkt aan de ingelogde gebruiker op te halen.

**Gebruikte vaardigheden (o.a):**
- Webdevelopment.
- Asynchronous programmeren.
- HTTP Requests.
- Websockets.
- Database queries.
- HTML, CSS, Javascript.
- ...
