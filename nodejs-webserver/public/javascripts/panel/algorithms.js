// Get table in DOM.
let table = document.getElementsByClassName("table")[0]

// Get all rows of table except header-row.
let rows = []
if (table != null) {
    rows = Array.from(table.getElementsByClassName("row"))
    rows = rows.slice(1, rows.length)
}

// Call the endpoint to retrieve the data (funds, running or not,...) of all the algorithms every 5000ms
// so it is up-to-date for the user.
show_algorithm_data()
setInterval(show_algorithm_data, 5000)

// Function to show the current data of the algorithm in the panel.
function show_algorithm_data() {
    rows.forEach(function(row) {
        // Get id of algorithm in this row.
        let id = row.id

        // Call API to retrieve algorithm.
        get_algorithm(id)
            .then(response => {

                // Update current funds cell.
                let cfcell = row.getElementsByClassName("current-funds")[0]
                let profit = response.start_funds <= response.current_funds
                cfcell.style.color = profit ? "#088A08" : "#FF0000"
                
                // Calculate profit.
                let percentage = ((response.current_funds - response.start_funds) / response.start_funds) * 100
                cfcell.textContent = response.current_funds.toFixed(2) + " (" + (profit ? "+" : "") + percentage.toFixed(2) + "%)"

                // Update cell indicating if algorithm is running or not.
                let ircell = row.getElementsByClassName("is-running")[0]
                let delete_button = row.getElementsByClassName("delete-button")[0]
                let reset_button = row.getElementsByClassName("reset-button")[0]
                
                if (response.is_running == true) {
                    // Set power-button text to "is-running". We check if the HTML of the cell is already equal
                    // to the HTML we want to update it to avoid blinking DOM.
                    let html = "<img src=\"/images/panel/turn-off.png\" title=\"Stop algoritme\" onclick=\"toggle_running('"+id+"', 'stop')\"><span color=\"#088A08\">Running...</span>"
                    if (ircell.innerHTML.replaceAll("&quot;", "\"") != html) {
                        ircell.innerHTML = html
                    }
                    
                    // Delete- and reset-button are disabled when algorithm is running.
                    delete_button.classList.add("disabled")
                    reset_button.classList.add("disabled")
                } else {
                    // Set power-button text to "inactief". We check if the HTML of the cell is already equal
                    // to the HTML we want to update it to avoid blinking DOM.
                    let html = "<img src=\"/images/panel/turn-on.png\" title=\"Start algoritme\" onclick=\"toggle_running('"+id+"', 'start')\"><span color=\"#6E6E6E\">Inactief.</span>"
                    if (ircell.innerHTML.replaceAll("&quot;", "\"") != html) {
                        ircell.innerHTML = html
                    }
                    
                    // Delete- and reset-button are enabled when algorithm is not running.
                    delete_button.classList.remove("disabled")
                    reset_button.classList.remove("disabled")
                }
            })
            .catch(error => {
                console.log("Error: " + error)
            })
    })
}

// Function executed when the user clicks to power-button to start/stop an algorithm.
function toggle_running(id, start_or_run) {
    // Set temporary "Wait..." text after click so user doesn't click twice.
    let cell = document.querySelectorAll(".row#" + id + " .is-running")[0]
    cell.textContent = "Wait..."

    // Send request to toggle running.
    let xhr = new XMLHttpRequest()
    xhr.onreadystatechange = function() {
        if (xhr.readyState == XMLHttpRequest.DONE) {
            if (xhr.status != 200) {
                alert("Fout bij veranderen van running status...")
            }
        }
    }

    xhr.open("POST", "http://127.0.0.1:3000/algorithms/" + id + "/" + start_or_run, true)
    xhr.send()
}

// Function executed when the user clicks the button to delete an algorithm.
function delete_algorithm(id) {
    // Check if user is sure.
    if (!confirm("Ben je zeker dat je dit algoritme permament wilt verwijderen?")) {
        return;
    }

    // Send request to delete.
    let xhr = new XMLHttpRequest()
    xhr.onreadystatechange = function() {
        if (xhr.readyState == XMLHttpRequest.DONE) {
            if (xhr.status == 204) {
                document.querySelectorAll(".row#" + id)[0].remove()
                alert("Algoritme is verwijderd.")

                if (document.querySelectorAll(".table .table-body .row").length == 0) {
                    window.location.reload()
                }
            } else {
                alert(xhr.responseText)
            }
        }
    }

    xhr.open("DELETE", "http://127.0.0.1:3000/algorithms/" + id, true)
    xhr.send()
}

// Function executed when the user clicks the button to reset an algorithm.
function reset_algorithm(id) {
    // Check if user is sure.
    if (!confirm("Ben je zeker dat je de order-history en start_funds van dit algoritme permament wilt resetten?")) {
        return;
    }
    
    // Send request to delete.
    let xhr = new XMLHttpRequest()
    xhr.onreadystatechange = function() {
        if (xhr.readyState == XMLHttpRequest.DONE) {
            if (xhr.status == 204) {
                alert("Algoritme is gereset.")
            } else {
                alert(xhr.responseText)
            }
        }
    }

    xhr.open("PUT", "http://127.0.0.1:3000/algorithms/" + id + "/reset", true)
    xhr.send()
}
