#
# Algorithm using Simple Moving Average.
#

# Calculate to Simple Moving Average.
def sma(close_prices):
  return sum(close_prices) / len(close_prices)

def func(data):

  short_sma = 20
  long_sma = 100
  
  if len(data) <= long_sma:
    return 0
  
  close_prices = [d.c for d in data]
  
  sma_short = sma(close_prices[-short_sma:])
  sma_long = sma(close_prices[-long_sma:])
  
  if sma_short > sma_long:
    return 0.005 # buy
  if sma_short < sma_long:
    return -0.0025 # sell
  
  return 0
