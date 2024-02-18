#
# Algorithm using the Mean Reversion Strategy.
#

# Calculate average price of datapoint.
def avg_price(dp):
    return (dp.h + dp.l + dp.c) / 3

# Calculate the mean.
def mean(datapoints):
    _avg_price = sum(avg_price(dp) for dp in datapoints)
    return _avg_price / len(datapoints)

# Get array of deviations for each datapoint.
def deviations(datapoints, mean):
    return [avg_price(dp) - mean for dp in datapoints]

# Get the z_score.
def zscore(datapoints):
    
    _mean = mean(datapoints[:-1])
    sqrd_deviations = [d ** 2 for d in deviations(datapoints[:-1], _mean)]
    variance = sum(sqrd_deviations[:-1]) / (len(datapoints) - 1)
    std_deviation = math.sqrt(variance)

    z_score = (avg_price(datapoints[-1]) - _mean) / std_deviation
    return z_score

def func(data):

    window = 25

    if len(data) <= 25:
        return 0

    z_score = zscore(data)

    if z_score > 0.1:
        return 0.0005   # buy
    if z_score < -0.1:
        return -0.0008

    return 0
