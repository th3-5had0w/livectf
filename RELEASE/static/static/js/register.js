const btn = document.querySelector('input[name="register"]');

btn.onclick = async (e) => {
    e.preventDefault();

    const email = document.getElementsByName("email")[0].value;
    const username = document.getElementsByName("username")[0].value;
    const password = document.getElementsByName("password")[0].value;

    const data = `email=${email}&username=${username}&password=${password}`
    
    let  res = await fetch("/api/register", {
        method: "POST",
        headers: {
            "Content-Type": "application/x-www-form-urlencoded; charset=UTF-8"
        },
        body: data
    });
    res = await res.json();

    var expires = (new Date(Date.now()+ 86400*1000)).toUTCString();

    if (!res["is_error"]) {
        window.location.replace("/login");
    } else {
        alert(res["message"]);
    }

}