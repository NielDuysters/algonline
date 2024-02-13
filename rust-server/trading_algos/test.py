# No custom imports are allowed. The following packages are automatically included:
# import math
# import pandas
# import numpy


# Write your algorithm logic here. Returning a positive value means that
# value is bought. Returning a negative value is idem but for selling. Return 0
# to do nothing.
def func(data):
	if len(data) > 3:
		if data[len(data)-1].c > data[len(data)-3].c:
			return 0.001
		if data[len(data)-1].c < data[len(data)-3].c:
			return -0.001
	
	return 0
