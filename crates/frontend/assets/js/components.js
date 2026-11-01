document.addEventListener("fetcherr", event => {
    if (event.detail?.err?.name === "AbortError") return;
    const message = document.getElementById("request-error");
    if (!message) return;
    message.textContent = "Request failed. Check the node connection and try again.";
    message.hidden = false;
    clearTimeout(message.hideTimer);
    message.hideTimer = setTimeout(() => { message.hidden = true; }, 8000);
});

function filterSoftwareRows() {
    const input = document.getElementById("software-search");
    if (!input) return;
    const query = input.value.trim().toLowerCase();
    document.querySelectorAll("[data-software-row]").forEach(row => {
        row.hidden = !row.dataset.search.includes(query);
    });
}

function filterProcessRows() {
    const input = document.getElementById("process-search");
    if (!input) return;
    const query = input.value.trim().toLowerCase();
    document.querySelectorAll("[data-process-row]").forEach(row => {
        row.hidden = !row.dataset.search.includes(query);
    });
}

async function getUpdateMessage(oldVersion) {
    const now = Math.round(Date.now() / 1000);
    let { newVersion = null, lastChecked = 0 } = JSON.parse(
        localStorage.getItem("update-check") || "{}"
    );

    if (now - lastChecked > 86400) {
        const resp = await fetch("https://api.github.com/repos/ravenclaw900/DietPi-Dashboard/tags");
        const json = await resp.json();

        // Remove preceding 'v'
        newVersion = json[0].name.substring(1);

        localStorage.setItem("update-check", JSON.stringify({ newVersion, lastChecked: now }));
    }

    if (
        newVersion.localeCompare(oldVersion, undefined, {
            numeric: true,
        }) === 1
    ) {
        return `New version available: ${newVersion}`;
    }
}

(() => {
    customElements.define(
        "web-terminal",
        class extends HTMLElement {
            connectedCallback() {
                const term = this.term = new Terminal({ cursorBlink: true, scrollback: 5000 });
                const fit = new FitAddon.FitAddon();
                const controller = this.controller = new AbortController();
                const status = this.parentElement.querySelector(".terminal-status");
                let inputQueue = Promise.resolve();
                let resizeTimer;
                const showStatus = (message, state) => {
                    if (status) { status.textContent = message; status.dataset.state = state; }
                };
                const send = (url, body) => fetch(url, {
                    method: "POST", body, signal: controller.signal,
                    credentials: "same-origin"
                }).then(response => {
                    if (!response.ok || response.redirected) throw new Error("Request failed");
                });
                term.loadAddon(fit);
                term.open(this);
                term.onData(data => {
                    inputQueue = inputQueue.then(() => send("/terminal/write", data)).catch(error => {
                        if (error.name !== "AbortError") showStatus("Input could not be sent. Reload to reconnect.", "error");
                    });
                });
                term.onResize(({ rows, cols }) => {
                    clearTimeout(resizeTimer);
                    resizeTimer = setTimeout(() => send("/terminal/resize", new URLSearchParams({ rows, cols })).catch(error => {
                        if (error.name !== "AbortError") showStatus("Terminal resize failed.", "error");
                    }), 100);
                });
                this.onWindowResize = () => fit.fit();
                window.addEventListener("resize", this.onWindowResize);
                fit.fit();
                fetch("/terminal/stream", { signal: controller.signal, credentials: "same-origin" })
                    .then(async response => {
                        if (!response.ok || response.redirected || !response.body ||
                            !response.headers.get("content-type")?.startsWith("application/octet-stream")) {
                            throw new Error("Terminal stream unavailable");
                        }
                        showStatus("Connected. This session is shared with other dashboard users.", "connected");
                        const reader = response.body.getReader();
                        while (true) {
                            const { value, done } = await reader.read();
                            if (done) break;
                            term.write(value);
                        }
                        showStatus("Terminal disconnected. Reload to reconnect.", "error");
                    })
                    .catch(error => {
                        if (error.name !== "AbortError") showStatus("Terminal disconnected. Reload to reconnect.", "error");
                    });
            }

            disconnectedCallback() {
                this.controller?.abort();
                window.removeEventListener("resize", this.onWindowResize);
                this.term?.dispose();
            }
        }
    );

    customElements.define(
        "code-editor",
        class extends HTMLElement {
            connectedCallback() {
                const textarea = this.querySelector("textarea");
                const pre = this.querySelector("pre");

                const highlight = () => {
                    pre.textContent = textarea.value;
                    microlight(pre);
                };

                const autosize = () => {
                    textarea.style.height = "0px";
                    textarea.style.height = textarea.scrollHeight + "px";
                    pre.style.height = textarea.scrollHeight + "px";
                };

                textarea.addEventListener("input", highlight);
                textarea.addEventListener("input", autosize);

                highlight();
                autosize();
            }
        }
    );
})();
