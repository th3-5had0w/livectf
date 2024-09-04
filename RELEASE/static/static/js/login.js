const btn = document.querySelector('input[name="login"]');

btn.onclick = async (e) => {
    e.preventDefault();

    const username = document.getElementsByName("username")[0].value;
    const password = document.getElementsByName("password")[0].value;

    const data = `username=${username}&password=${password}`
    
    let  res = await fetch("/api/login", {
        method: "POST",
        headers: {
            "Content-Type": "application/x-www-form-urlencoded; charset=UTF-8"
        },
        body: data
    });
    res = await res.json();

    var expires = (new Date(Date.now()+ 86400*1000)).toUTCString();

    if (!res["is_error"]) {
        document.cookie = `auth=${res.message};expires=${expires}`
        window.location.replace("/");
    } else {
        alert(res["message"]);
    }

}