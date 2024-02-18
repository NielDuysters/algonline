# Calculate average price of datapoint.
def avg_price(dp):
    return (dp.h + dp.l + dp.c) / 3

# Calculate the volume-weighted average price.
def vwap(datapoints):
    total_volume = 0.0
    weighted_price = 0.0

    for dp in datapoints:
        weighted_price += avg_price(dp) * dp.v
        total_volume += dp.v

    return weighted_price / total_volume

def func(data):
    window = 25

    if len(data) <= window:
        return 0

    data_points = data[-window:-1]
    last_avg_price = avg_price(data[-1])

    _vwap = vwap(data_points)

    if last_avg_price > _vwap:
        return 0.003   # buy
    if last_avg_price < _vwap:
        return -0.005  # sell

    return 0
