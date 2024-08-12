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

Array.from(document.querySelectorAll(".ban-btn")).map(btn => {
    btn.onclick = async (e) => {
        alert("Banning feature is comming up soon...");
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

document.querySelector("#schedule-challenge").addEventListener("click", async (e) => {
    e.preventDefault();
    
    let challenge_name = document.querySelector("#challenge-name").value
    let parsedStartTime = new Date(document.querySelector("#start-date2").value + "T" + document.querySelector("#start-time2").value + "Z");
    let parsedEndTime = new Date(document.querySelector("#end-date2").value + "T" + document.querySelector("#end-time2").value + "Z");

    parsedStartTime = Math.floor(parsedStartTime.getTime() / 1000);
    parsedEndTime = Math.floor(parsedEndTime.getTime() / 1000);

    let result = await fetch(`/api/${challenge_name}/deploy`, {
        method: "POST",
        mode: "cors",
        credentials: "include",
        headers: {
            "X-start": parsedStartTime,
            "X-end": parsedEndTime
        }
    });

    result = await result.json();

    if (!result.is_error) {
        alert(result.message);
        location.reload();
    } else {
        alert(result.message);
    }
});

document.querySelector("#stop-btn").addEventListener("click", async (e) => {
    const challenge_name = e.target.getAttribute("data-challengeid");
    let res = await fetch(`/api/${challenge_name}/destroy`, {
        method: "POST",
        credentials: "include",
        mode: "cors"
    });

    res = await res.json();

    if (res.is_error) {
        alert(res.message);
    } else {
        location.reload();
    }
});