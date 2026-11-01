function setChartLegend(panel, index) {
    panel.querySelectorAll(".legend-value").forEach(value => {
        const samples = value.dataset.values.split("|");
        value.textContent = samples[index] || "?";
    });
}

function chartPointer(event) {
    const hitbox = event.target.closest(".graph-hitbox");
    if (!hitbox) return;
    const panel = hitbox.closest(".chart-card");
    const bounds = hitbox.getBoundingClientRect();
    if (!bounds.width) return;
    const position = Math.min(1, Math.max(0, (event.clientX - bounds.left) / bounds.width));
    const index = Math.round((1 - position) * 19);
    const guide = panel.querySelector(".graph-guide");
    const x = Number(hitbox.getAttribute("x")) + (19 - index) * Number(hitbox.getAttribute("width")) / 19;
    panel.chartHoverIndex = index;
    guide.setAttribute("x1", x);
    guide.setAttribute("x2", x);
    guide.setAttribute("visibility", "visible");
    setChartLegend(panel, index);
}

function chartLeave(event) {
    const hitbox = event.target.closest(".graph-hitbox");
    if (!hitbox || (event.relatedTarget && hitbox.contains(event.relatedTarget))) return;
    const panel = hitbox.closest(".chart-card");
    panel.chartHoverIndex = null;
    panel.querySelector(".graph-guide").setAttribute("visibility", "hidden");
    setChartLegend(panel, 0);
}

function animateChartLine(line, nextPoints) {
    if (line.getAttribute("points") === nextPoints) return;
    cancelAnimationFrame(line.chartAnimation);
    const parse = points => points.trim().split(/\s+/).filter(Boolean).map(pair => pair.split(",").map(Number));
    const previous = parse(line.getAttribute("points") || "");
    const next = parse(nextPoints);
    const isArea = line.classList.contains("graph-area");
    const previousCount = previous.length - (isArea ? 2 : 0);
    const nextCount = next.length - (isArea ? 2 : 0);
    if (previousCount < 1 || nextCount < previousCount || nextCount > previousCount + 1 ||
        previous.some(point => point.length !== 2 || point.some(value => !Number.isFinite(value))) ||
        next.some(point => point.length !== 2 || point.some(value => !Number.isFinite(value))) ||
        window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
        line.setAttribute("points", nextPoints);
        return;
    }
    const started = performance.now();
    const tick = now => {
        const progress = Math.min(1, (now - started) / 1600);
        const points = next.map(([x, y], i) => {
            let start;
            if (isArea && i >= nextCount) {
                start = previous[previousCount + i - nextCount];
            } else {
                start = previous[Math.max(0, Math.min(previousCount - 1, i - 1))];
            }
            return `${start[0] + (x - start[0]) * progress},${start[1] + (y - start[1]) * progress}`;
        }).join(" ");
        line.setAttribute("points", progress === 1 ? nextPoints : points);
        if (progress < 1 && line.isConnected) line.chartAnimation = requestAnimationFrame(tick);
    };
    line.chartAnimation = requestAnimationFrame(tick);
}

function updateChart(panel, incoming) {
    const lines = panel.querySelectorAll(".graph-series, .graph-area");
    const nextLines = incoming.querySelectorAll(".graph-series, .graph-area");
    const markers = panel.querySelectorAll(".graph-current");
    const nextMarkers = incoming.querySelectorAll(".graph-current");
    const legends = panel.querySelectorAll(".legend-value");
    const nextLegends = incoming.querySelectorAll(".legend-value");
    if (lines.length !== nextLines.length || markers.length !== nextMarkers.length || legends.length !== nextLegends.length) {
        panel.replaceWith(incoming);
        return incoming;
    }
    lines.forEach((line, i) => animateChartLine(line, nextLines[i].getAttribute("points")));
    markers.forEach((marker, i) => {
        cancelAnimationFrame(marker.chartAnimation);
        const from = Number(marker.getAttribute("cy"));
        const to = Number(nextMarkers[i].getAttribute("cy"));
        if (from === to) return;
        if (!Number.isFinite(from) || !Number.isFinite(to) || window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
            marker.setAttribute("cy", to);
            return;
        }
        const started = performance.now();
        const tick = now => {
            const progress = Math.min(1, (now - started) / 1600);
            marker.setAttribute("cy", from + (to - from) * progress);
            if (progress < 1 && marker.isConnected) marker.chartAnimation = requestAnimationFrame(tick);
        };
        marker.chartAnimation = requestAnimationFrame(tick);
    });
    legends.forEach((legend, i) => { legend.dataset.values = nextLegends[i].dataset.values; });
    setChartLegend(panel, panel.chartHoverIndex ?? 0);
    return panel;
}

function updateCpuPanel(panel, incoming) {
    const current = Array.from(panel.children);
    const next = Array.from(incoming.children);
    if (current.length !== next.length || current.some((item, i) =>
        item.tagName !== next[i].tagName || item.className !== next[i].className)) {
        panel.replaceWith(incoming);
        return incoming;
    }
    current.forEach((item, i) => {
        const replacement = next[i];
        if (item.classList.contains("meter-container")) {
            const bar = item.querySelector(".bar.cpu");
            const newBar = replacement.querySelector(".bar.cpu");
            if (bar && newBar) bar.style.setProperty("--scale", newBar.style.getPropertyValue("--scale"));
        } else if (item.classList.contains("metric-row")) {
            const label = item.querySelector("span");
            const value = item.querySelector("strong");
            if (label && value) {
                label.textContent = replacement.querySelector("span").textContent;
                value.textContent = replacement.querySelector("strong").textContent;
            }
        } else if (item.textContent !== replacement.textContent) {
            item.textContent = replacement.textContent;
        }
    });
    return panel;
}

function startSystemMonitor(root) {
    if (root.monitorStarted) return;
    root.monitorStarted = true;
    root.addEventListener("pointermove", chartPointer);
    root.addEventListener("pointerout", chartLeave);
    const status = document.getElementById("system-status");
    const poll = async () => {
        if (!root.isConnected) return;
        try {
            const response = await fetch("/system", { headers: { "nm-request": "true" }, credentials: "same-origin" });
            if (response.redirected) { window.location.assign(response.url); return; }
            if (!response.ok) throw new Error("Monitoring request failed");
            const template = document.createElement("template");
            template.innerHTML = await response.text();
            const next = template.content.querySelector("#system-swap");
            if (!next && template.content.querySelector('meta[http-equiv="refresh"]')) {
                window.location.assign("/login");
                return;
            }
            if (!next) throw new Error("Monitoring response was invalid");
            if (!root.isConnected) return;
            const wanted = new Set();
            Array.from(next.children).forEach((incoming, index) => {
                const key = incoming.dataset.panel;
                if (!key) return;
                wanted.add(key);
                let panel = Array.from(root.children).find(item => item.dataset.panel === key);
                if (panel && panel.classList.contains("chart-card")) panel = updateChart(panel, incoming);
                else if (panel && key === "cpu-meter") panel = updateCpuPanel(panel, incoming);
                else if (panel) { panel.replaceWith(incoming); panel = incoming; }
                else panel = incoming;
                if (root.children[index] !== panel) root.insertBefore(panel, root.children[index] || null);
            });
            Array.from(root.children).forEach(panel => { if (panel.dataset.panel && !wanted.has(panel.dataset.panel)) panel.remove(); });
            if (status) status.hidden = true;
        } catch (error) {
            if (status && root.isConnected) {
                status.textContent = "Live updates unavailable. Retrying...";
                status.hidden = false;
            }
        }
        if (root.isConnected) setTimeout(poll, 2000);
    };
    setTimeout(poll, 2000);
}

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
