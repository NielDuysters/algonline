#
# This algorithm checks how many datapoints are increasing or decreasing consecutively starting by counting from the last datapoint.
# If the count exceeds a threshold we generate the order-signal.
#

# Count how many consecutive days BTCUSDT increased or decreases.
def count(data, threshold):
    count_inc = 0
    count_dec = 0
    
    for i in range(1, len(data)):
        if data[i].c > data[i-1].c:
            count_inc += 1
            count_dec = 0
            
            if count_inc >= threshold:
                return 1
        
        if data[i].c < data[i-1].c:
            count_inc = 0
            count_dec += 1
             
            if count_dec >= (threshold / 2):
                return -1
            
    return 0
    
def func(data):
    window = 50
    threshold = 4
    
    if len(data) <= window:
        return 0
    
    c = count(data[::-1][-window:], threshold)
    
    if c == 1:
        return 0.0008 # buy
    if c == -1:
        return -0.0008 # sell
    
    return 0
