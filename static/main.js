function init() {
    let dropArea = document.getElementById('drop-area');

    dropArea.addEventListener('dragenter', handler, false);
    dropArea.addEventListener('dragleave', handler, false);
    dropArea.addEventListener('dragover', handler, false);
    dropArea.addEventListener('drop', handler, false);

    let freeSpaceElem = document.getElementById('freeSpace');
    freeSpace().then(x => {
        freeSpaceElem.innerText = x;
    })
}

function handler(e) {
    e.preventDefault();
    e.stopPropagation();

    let dropArea = document.getElementById('drop-area');
    if (e.type == "dragenter" || e.type == "dragover") {
        dropArea.classList.add('highlight');
    } else {
        dropArea.classList.remove('highlight');
    }

    if (e.type == "drop") {
        let dt = e.dataTransfer;
        handleFiles(dt.files)
    }
}

async function handleFiles(files) {
    ([...files]).forEach(uploadFile)
}

async function uploadFile(file) {
    let formData = new FormData();
    formData.append('file', file);
    formData.append('name', file.name);
    let result = await fetch("/upload", {
        method: "POST",
        body: formData
    });

    let uuid = await result.text();
    let log = document.getElementById('log');
    let line = document.createElement("p");
    line.appendChild(document.createTextNode(file.name + ": "));
    let link = document.createElement("a");
    link.innerText = "/file/" + uuid;
    link.setAttribute("href", "/file/" + uuid);
    line.appendChild(link);
    log.prepend(line);
}

async function freeSpace() {
    resp = await fetch("/free_space")
    return await resp.text()
}
