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

    if (!res["is_error"]) {
        window.location.replace("/");
    } else {
        alert(res["message"]);
    }

}