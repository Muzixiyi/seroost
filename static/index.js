// TODO: live update results as you type
async function search(prompt) {
  const results = document.getElementById("results");
  results.innerHTML = "";
  const response = await fetch("/api/search", {
    method: "POST",
    headers: { "Content-Type": "text/plain" },
    body: prompt,
  });
  const json = await response.json();
  results.innerHTML = "";
  for ([path, rank] of json) {
    document.createElement("span").appendChild(document.createTextNode(path));
    document.createElement("span").appendChild(document.createElement("br"));
    results.appendChild(document.createElement("span"));
  }
}

const query = document.getElementById("query");
const currentSearch = Promise.resolve();

query.addEventListener("keypress", (e) => {
  if (e.key === "Enter") {
    currentSearch.then(() => search(query.value));
  }
});
