// Executed when the user clicks the button to reset his account.
document.getElementById("reset-button").addEventListener("click", function() {
    if (confirm("Bent u zeker dat je jouw account wilt resetten? Dit kan niet meer ongedaan gemaakt worden!")) {
        window.location.href = "/panel/settings/reset"
    }
})

// When the user hovers over the reset-button we make it red to indicate the danger of the action.
document.getElementById("reset-button").addEventListener("mouseenter", function(e) {
    e.currentTarget.querySelector("img").setAttribute("src", "/images/panel/reset-red.png")
    e.currentTarget.addEventListener("mouseleave", function(e) {
        e.currentTarget.querySelector("img").setAttribute("src", "/images/panel/reset.png")
    })
})
