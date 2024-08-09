Array.from(document.querySelectorAll(".del-btn")).map(btn => {
    btn.onclick = async (e) => {
        const userId = e.target.getAttribute("data-userid");
        let res = await fetch("/api/user/"+userId, {
            method: "DELETE",
            credentials: "include",
            mode: "cors"
        });

        res = await res.json();

        if (res.is_error) {
            alert(res.message);
        } else {
            location.reload();
        }
    }
})

document.querySelector("#upload-challenge").addEventListener("click", async (e) => {
    const data = new FormData(document.querySelector(".challenge-upload-form"));

    e.preventDefault();
    
    let parsedStartTime = new Date(document.querySelector("#start-date").value + "T" + document.querySelector("#start-time").value + "Z");
    let parsedEndTime = new Date(document.querySelector("#end-date").value + "T" + document.querySelector("#end-time").value + "Z");

    parsedStartTime = Math.floor(parsedStartTime.getTime() / 1000);
    parsedEndTime = Math.floor(parsedEndTime.getTime() / 1000);

    let result = await fetch("/api/challenge-upload", {
        method: "POST",
        mode: "cors",
        credentials: "include",
        body: data,
        headers: {
            "X-start": parsedStartTime,
            "X-end": parsedEndTime
        }
    });

    result = await result.text();

    if (result.indexOf("File uploaded successfully") != -1) {
        location.reload();
    } else {
        alert("failed to upload challenge, please check file, start/end time");
    }
})