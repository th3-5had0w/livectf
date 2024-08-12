
const modal = document.querySelector(".challenge-modal");
const challenges = document.querySelectorAll(".challenge-wrapper");
const span = document.getElementsByClassName("modal-close")[0];


Array.from(challenges).map(chall => {
  chall.onclick = () => {
    const challTitle = chall.getAttribute("data-chall");
    const challCategory = "Pwn" // we do pwn first
    const challDesc = "Good luck"
    const challScore = chall.getAttribute("data-score") ?? "0";
    const challConn = chall.getAttribute("data-connection");
    const attachment = `/attachments/${challTitle}.tar.gz`;

    const modalTitle = document.querySelector("#modal-chall-title");
    const modalScore = document.querySelector("#chall-score");
    const modalDesc = document.querySelector("#chall-desc");
    const modalRemote = document.querySelector("#remote-content");
    const modalAttachment = document.querySelector("#attachment");

    modalTitle.textContent = challTitle;
    modalScore.textContent = challScore;
    modalDesc.textContent = challDesc;
    modalRemote.textContent = challConn;
    modalAttachment.href = attachment;
    modalAttachment.textContent = `${challTitle}.tar.gz`
    modal.style.display = "block";
  }
})


span.onclick = function() {
  modal.style.display = "none";
}

window.onclick = function(event) {
  if (event.target == modal) {
    modal.style.display = "none";
  }
}

document.querySelector("#flag-submit").addEventListener("keyup", async (e) => {
  if (e.key === 'Enter' || e.keyCode === 13) {
    const flag = e.target.value;

    await fetch(`/submit/${encodeURIComponent(flag)}`, {
      method: "POST",
      credentials: "include",
      mode: "cors"
    });

    alert("Flag is sent for checking");
  }
});