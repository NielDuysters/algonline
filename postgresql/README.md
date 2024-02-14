# PostgreSQL

Voor dit project is PostgreSQL gebruikt als database-engine. Het database-gebeuren van dit project is vrij basaal. Ik zal echter enkele queries en functies toelichten.

## History-table
Wanneer een algoritme een order uitvoerd wordt de data van dit order bijgehouden in een tabel history.
```
                                  Table "public.history"
    Column    |            Type             | Collation | Nullable |         Default
--------------+-----------------------------+-----------+----------+-------------------------
 algorithm_id | character varying(255)      |           | not null |
 order_id     | character varying(12)       |           |          | NULL::character varying
 action       | character varying(5)        |           |          | NULL::character varying
 btc          | numeric                     |           | not null |
 usdt         | numeric                     |           | not null |
 btc_price    | numeric                     |           | not null |
 created_at   | timestamp without time zone |           | not null | CURRENT_TIMESTAMP
Indexes:
    "history_created_at_idx" btree (created_at DESC)
Foreign-key constraints:
    "history_algorithm_id_fkey" FOREIGN KEY (algorithm_id) REFERENCES algorithms(id) ON DELETE CASCADE
Triggers:
    history_trigger AFTER INSERT OR DELETE OR UPDATE ON history FOR EACH ROW EXECUTE FUNCTION process_history_record()
    refresh_history_view AFTER INSERT OR DELETE OR UPDATE OR TRUNCATE ON history FOR EACH STATEMENT EXECUTE FUNCTION refresh_history_view()
```

Deze info is nodig om o.a de chart van een algoritme te tonen om zo de prijsevolutie van zowel BTC, USDT en het totaal van het portfolio (BTC + USDT) weer te geven over tijd.

**Hier kwam een probleem bij kijken:** Als ik de prijzen van dit algoritme wil weergeven voor timestamp x dan zou ik eerst alle kollomen moeten optellen `SELECT SUM(btc), SUM(usdt) FROM history WHERE algorithm_id = $1 AND created_at <= x`.
Dit geeft de USDT en BTC voor een algoritme voor timestamp x. Voor het totaal van het portfolio moet ik echter ook het aantal BTC voor timestamp x vermemigvuldigen met `btc_price` wat de prijs voor 1 BTC bevat in USDT voor op het tijdstip van de order.
`SELECT SUM(btc), SUM(usdt), SUM(btc) * btc_price FROM history WHERE algorithm_id = $1 AND created_at <= x` (pseudo).

Als ik dan een chart wil weergeven zou ik deze query voor elke minuut van start_timestamp > timestamp x moeten uitvoeren. Dat resulteert in het zeer traag laden van de chart. Daarom heb ik een view gemaakt:
```
    Column    |            Type             | Collation | Nullable | Default | Storage  | Compression | Stats target | Description
--------------+-----------------------------+-----------+----------+---------+----------+-------------+--------------+-------------
 created_at   | timestamp without time zone |           |          |         | plain    |             |              |
 algorithm_id | character varying(255)      |           |          |         | extended |             |              |
 total_usdt   | numeric                     |           |          |         | main     |             |              |
 total_btc    | numeric                     |           |          |         | main     |             |              |
View definition:
 SELECT created_at,
    algorithm_id,
    sum(usdt) OVER (PARTITION BY algorithm_id ORDER BY created_at) AS total_usdt,
    sum(btc) OVER (PARTITION BY algorithm_id ORDER BY created_at) AS total_btc
   FROM history
  GROUP BY algorithm_id, btc, btc_price, usdt, created_at;
```

Deze view bevat voor elke algoritme het `total_usdt` en `total_btc` voor elke timestamp. Op de history-table is er een trigger `refresh_history_view` toegevoegd die na elke insert de volgende procedure uitvoert om de view te refreshen:
```
CREATE OR REPLACE FUNCTION public.refresh_history_view()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
begin
    REFRESH MATERIALIZED VIEW history_aggregate;
    return null;
end $function$
```

De Rust-server voegt elke minuut een rij toe die enkel de btc_price voor dat tijdstip bevat zodat alle algoritmes per minuut kennis hebben van de btc_price ookal is er geen order gebeurt die minuut. Op deze kunnen we per minuut een chart genereren
voor alle algoritmes die de usdt, btc, en total weergeven.

Vervolgens voer ik in Rust de volgende query uit om de chart op te vragen:
```
WITH btc_price_cte AS (
    SELECT created_at, btc_price FROM history where algorithm_id = $1 ORDER BY created_at
)
SELECT
 start_funds_usdt + COALESCE(h.total_usdt, 0) + COALESCE(h.total_btc * btc_price_cte.btc_price, 0) AS current_funds_total,
 start_funds_usdt + COALESCE(h.total_usdt, 0) AS current_funds_usdt,
COALESCE(h.total_btc, 0) AS current_funds_btc,
h.created_at::TEXT AS ts
FROM
    algorithms
LEFT JOIN
    history_aggregate h ON h.algorithm_id = algorithms.id
LEFT JOIN
    btc_price_cte ON btc_price_cte.created_at = h.created_at
WHERE
    algorithms.id = $1
GROUP BY
    algorithm_id, start_funds_usdt, h.total_usdt, h.total_btc, btc_price_cte.btc_price, h.created_at
ORDER BY h.created_at;
```

Met een common table expression vraag ik eerst alle btc prijzen op met de timestamp. De history_aggregate view bevat per timestamp al het totaal van de btc en usdt per tijdstip aangezien deze view telkens refreshed wanneer een rij in history wordt toegevoegd.
Op deze manier kan er veel sneller een overzicht gegeven worden van de prestatie van het algoritme en de prijsevolutie per minuut rekening houdend met de volatiliteit van de btc prijs.

De history tabel heeft ook een procedure `process_history_record()`.
```
CREATE OR REPLACE FUNCTION public.process_history_record()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
    BEGIN
        PERFORM pg_notify('history_record_inserted', row_to_json(NEW)::text);
        RETURN NEW;
    END;
$function$
```

Deze stuurt een postgres notificatie uit naar deze listener in Rust:
```
match client
    .query(
        "LISTEN history_record_inserted", &[]
    )
    .await
    {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{}", e);
        }
    }
```

De chart en history van een algorithme wordt op de pagina van dat algoritme live geupdate d.m.v een websocket. Wanneer de listener een notificatie die de data van de rij bevat binnenkrijgt wordt dit doorgestuurd aan de websocket stream zodat de gebruiker live het algoritme kan opvolgen zonder te moeten herladen.
