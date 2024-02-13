// Function to style and display our custom tables niceky.

let tables = document.getElementsByClassName("table")
Array.from(tables).forEach(function(table) {
    let head_row = table.getElementsByClassName("row")[0]
    let head_row_cells = head_row.getElementsByClassName("cell")

    var body_rows = Array.from(table.getElementsByClassName("row"))
    var body_rows = body_rows.slice(1, body_rows.length)

    body_rows.forEach(function(row) {
        let cells = row.getElementsByClassName("cell")
        Array.from(cells).forEach(function(cell, i) {
            cell.style.width = head_row_cells[i].offsetWidth + "px"
        })
    })
})
