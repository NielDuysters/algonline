// Executed when the user clicks the button to reset his account.
document.getElementById("reset-button").addEventListener("click", function() {
    if (confirm("Are you sure you want to reset your account? This can't be undone!")) {
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
