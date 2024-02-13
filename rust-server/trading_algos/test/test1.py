def func(data):
    open_prices = [d.o for d in data]
    avg_open = sum(open_prices) / len(open_prices)

    close_prices = [d.c for d in data]
    avg_close = sum(close_prices) / len(close_prices)

    if avg_open > avg_close:
        return 1

    return 0
