# Calculate to Simple Moving Average.
def sma(close_prices):
    return sum(close_prices) / len(close_prices)

# Calculate Exponential Moving Average.
def ema(close_prices):
    alpha = 2 / (len(close_prices) + 1)
    ema = sma(close_prices)
    for p in close_prices:
        ema = (p * alpha) + (ema * (1 - alpha))

    return ema

def func(data):
 
    window = 100

    if len(data) <= window:
        return 0
  
    # Extract last window-amount of elements from list except last one
    # and put the close_prices in separate list.
    close_prices = [d.c for d in data[-window:-1]]
    last_close_price = data[-1].c
 
    last_ema = ema(close_prices)
 
    if last_close_price > last_ema:
        return 0.005   # buy
    if last_close_price < last_ema:
        return -0.006  # sell
  
    return 0
