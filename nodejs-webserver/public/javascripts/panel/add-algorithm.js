const default_code = 
`# No custom imports are allowed. The following packages are automatically included:
# import math
# import pandas
# import numpy


# Write your algorithm logic here. Returning a positive value means that
# value is bought. Returning a negative value is idem but for selling. Return 0
# to do nothing.
def func(data):
    return 0
`

document.getElementById("code_input").value = default_code

var editor = CodeMirror.fromTextArea(code_input, {
    lineNumbers: true,
    mode: "python",
})
