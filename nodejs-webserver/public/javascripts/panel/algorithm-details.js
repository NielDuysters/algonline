let code_input = document.getElementById("code_input")

var editor = CodeMirror.fromTextArea(code_input, {
    lineNumbers: true,
    mode: "python",
    readOnly: true,
})
