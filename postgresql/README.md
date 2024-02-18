# PostgreSQL

PostgreSQL is the database-engine of choice for this project. No advanced things are used in the database-architecture and the database-part of the project is pretty basic.

I will go into more detail about how I optimized the way chart-data for a trading algorithm is queried.

## History-table
When an algorithm executes an order the data of this order is stored in the table `history`.
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

This data is necessary to retrieve info about the algorithm. I.e to retrieve a chart showcasing the evolution of the amount of BTC and USDT an algorithm has over time, and as well the total of it's portfolio (total USDT + total BTC in USDT while keeping the volatility of BTC in account).

**This came with an impediment:** When I want to retrieve the chart to display the evolution of the amount of BTC and USDT an algorithm holds I'd have to to sum the total of purchased/sold BTC and USDT for each timestamp. And multiple the amount of BTC with the price of BTC at that timestamp. This is an instensive task causing extremely slow loading times for the chart.

To solve this I created the following view (`history_aggregate`):

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

This view contains the total of BTC and USDT an algorithm has at each specific timestamp. By the trigger bound to the `history` table we automatically update this view each time a row is added/removed/updated in the `history` table. This way we don't have to calculate these values when the user wants to load a chart.

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
The Rust-server inserts a row into the `history` table for all algorithms containing the price of BTC at that time. This way, even when an algorithm doesn't have a record of an order at that time, we are able to calculate the value of the portfolio by utilising this data.

Finally, I can retrieve the data to generate a chart in Rust with this query:

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
By utilising a common table expression I first retrieve all the prices of BTC for each timestamp. Next the view `history_aggregate` is queried so we have the amount of BTC and USDT at each timestamp, with the BTC price from the CTE we can easily calculate all the values required for the chart.

Because the view `history_aggregate` is updated at least every minute doing most of the calculations already we can load the chart a lot faster then if we would do these calculations each time the user requests the chart.

The table `history` also has the following procedure: `process_history_record()`.
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

This procedure send a Postgres-notification to a listener in Rust:
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

The chart and order-history of an algorithm is updated live on the page where the user views the statistics of a specific algorithm. When the listener receives a new notification we push the data it contains to the client using a websocket. This way the user always has the most recent data without a need to refresh the page.
