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

// document.querySelector("#upload-challenge").addEventListener("click", async (e) => {
//     const data = new FormData(document.querySelector(".challenge-upload-form"));

//     e.preventDefault();
    
//     let parsedStartTime = new Date(document.querySelector("#start-date").value + "T" + document.querySelector("#start-time").value + "Z");
//     let parsedEndTime = new Date(document.querySelector("#end-date").value + "T" + document.querySelector("#end-time").value + "Z");

//     parsedStartTime = Math.floor(parsedStartTime.getTime() / 1000);
//     parsedEndTime = Math.floor(parsedEndTime.getTime() / 1000);

//     let result = await fetch("/api/challenge-upload", {
//         method: "POST",
//         mode: "cors",
//         credentials: "include",
//         body: data,
//         headers: {
//             "X-start": parsedStartTime,
//             "X-end": parsedEndTime
//         }
//     });

//     result = await result.json();

//     if (!result.is_error) {
//         alert(result.message);
//         location.reload();
//     } else {
//         alert(result.message);
//     }
// })

document.querySelector("#schedule-challenge")?.addEventListener("click", async (e) => {
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

document.querySelector("#stop-btn")?.addEventListener("click", async (e) => {
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

function _arrayBufferToBase64( buffer ) {
    var binary = '';
    var bytes = new Uint8Array( buffer );
    var len = bytes.byteLength;
    for (var i = 0; i < len; i++) {
        binary += String.fromCharCode( bytes[ i ] );
    }
    return window.btoa( binary );
}


function parseChallenge(ev) {
    ev.preventDefault();
    ev.target.style.display = "none";
    let fileReader = new FileReader();

    fileReader.readAsArrayBuffer(new Blob([ev.dataTransfer.files[0]], {
        type: "application/tar+gzip"
    }));
    fileReader.onloadend = async () => {
        let resp = await fetch("/api/challenge_untar", {
            "method": "POST",
            "headers": {
                "content-type": "application/x-www-form-urlencoded"
            },
            "body": "data="+encodeURIComponent(_arrayBufferToBase64(fileReader.result))
        });

        resp = await resp.json()

        if (resp.length > 0) {
            localStorage.setItem("cached_upload", JSON.stringify(resp))
            const sandboxedIfrm = document.createElement("iframe");
            sandboxedIfrm.sandbox = "allow-scripts";

            const uploadInfo = document.createElement("div");
            uploadInfo.classList.add("upload-info");

            const dynamic_css = `
            @import url('https://fonts.googleapis.com/css2?family=Roboto+Mono:ital,wght@0,100..700;1,100..700&display=swap');

            body {
                display: flex;
                flex-direction: column;
                align-items: center;
                font-family: "Roboto Mono", monospace;
            }

            .upload-entry {
                display: flex;
                padding: 0 15px 0 15px;
                align-items: center;
                justify-content: space-between;
                width: 300px;
                border: 3px dashed black;
                margin: 20px;
            }

            .upload-entry h3 {
                font-size: 0.8rem;
            }

            .visibility-form {
                display: flex;
                flex-direction: column;
                border: none;
            }`;
            
            for (const entry of resp) {
                uploadInfo.innerHTML += `
                <div class="upload-entry"> 
                    <h3 class="upload-filename">${entry.filename}</h3>
                    <fieldset class="visibility-form">
                        <input type="radio" id="public" value="public" name="visibility_${entry.filename}">
                        <label for="public">public</label>
                        <input type="radio" id="private" value="private" name="visibility_${entry.filename}" checked>
                        <label for="private">private</label>
                    </fieldset>
                </div>`;

                sandboxedIfrm.srcdoc = `<style>\n${dynamic_css}\n</style>` + uploadInfo.outerHTML;
                sandboxedIfrm.width = "500px";
                sandboxedIfrm.height = "500px";
                document.querySelector(".form-wrapper").appendChild(sandboxedIfrm)
            }
        }
    }
}

function whenDragOver(ev) {
    ev.target.style.opacity = 0.5;
    ev.preventDefault();
}

function whenDragLeave(ev) {
    ev.target.style.opacity = 1;
    ev.preventDefault();
}

['dragenter', 'dragover', 'dragleave', 'drop'].forEach(eventName => {
    document.querySelector("challenge-dragging")?.addEventListener(eventName, (e) => {
        e.preventDefault()
        e.stopPropagation()
    }, false)
  })