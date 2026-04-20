/// <reference lib="dom" />
//
// mdview webview client.
// Connects to the same-origin embedded server over WebSocket at `/__mdview_live`
// and applies incremental HTML / theme patches published by the Rust side.
//
// Message shapes (decoded as JSON):
//   { op: "full",  html: string,  theme_css?: string, assets?: string[] }
//   { op: "patch", html: string,  target?: string }                // replace #mdv-content innerHTML
//   { op: "theme", css: string }

type FullMsg = { op: "full"; html: string; theme_css?: string; assets?: string[] };
type PatchMsg = { op: "patch"; html: string; target?: string };
type ThemeMsg = { op: "theme"; css: string };
type Msg = FullMsg | PatchMsg | ThemeMsg;

type ExtensionInit = {
    name: string;
    selector: string;
    init: (el: HTMLElement) => Promise<void> | void;
};

const extensions: ExtensionInit[] = [
    {
        name: "mdv-mermaid-init",
        selector: ".mermaid",
        init: async (el) => {
            try {
                const mermaid = (await import("mermaid")).default;
                mermaid.initialize({ startOnLoad: false, theme: "default" });
                await mermaid.run({ nodes: [el] });
            } catch (err) {
                console.warn("[mdview] mermaid init failed", err);
            }
        },
    },
    {
        name: "mdv-katex-init",
        selector: ".math, .katex-src",
        init: async (el) => {
            try {
                const katex = (await import("katex")).default;
                const src = el.getAttribute("data-src") ?? el.textContent ?? "";
                const display = el.classList.contains("math-display");
                el.innerHTML = katex.renderToString(src, { throwOnError: false, displayMode: display });
            } catch (err) {
                console.warn("[mdview] katex init failed", err);
            }
        },
    },
    {
        name: "mdv-plotly-init",
        selector: ".plotly-src",
        init: async (el) => {
            try {
                const Plotly = (await import("plotly.js-dist-min")).default;
                const spec = JSON.parse(el.getAttribute("data-spec") ?? "{}");
                await Plotly.newPlot(el, spec.data ?? [], spec.layout ?? {}, { displayModeBar: false });
            } catch (err) {
                console.warn("[mdview] plotly init failed", err);
            }
        },
    },
    {
        name: "mdv-drawio-init",
        selector: ".drawio-src",
        init: async (el) => {
            // Drawio viewer is not bundled (no npm package); the Rust side
            // emits a pre-rendered SVG — nothing to do on the client.
            el.dataset.mdvDrawio = "ready";
        },
    },
];

function root(): HTMLElement {
    let el = document.getElementById("mdv-content");
    if (!el) {
        el = document.createElement("article");
        el.id = "mdv-content";
        const main = document.getElementById("mdv") ?? document.body;
        main.appendChild(el);
    }
    return el;
}

function themeNode(): HTMLStyleElement {
    let el = document.getElementById("mdv-theme") as HTMLStyleElement | null;
    if (!el) {
        el = document.createElement("style");
        el.id = "mdv-theme";
        document.head.appendChild(el);
    }
    return el;
}

async function runExtensions(scope: HTMLElement): Promise<void> {
    for (const ext of extensions) {
        const nodes = scope.querySelectorAll<HTMLElement>(ext.selector);
        for (const node of Array.from(nodes)) {
            if (node.dataset.mdvInit === ext.name) continue;
            node.dataset.mdvInit = ext.name;
            try {
                await ext.init(node);
            } catch (err) {
                console.warn(`[mdview] ${ext.name} failed`, err);
            }
        }
    }
}

async function apply(msg: Msg): Promise<void> {
    switch (msg.op) {
        case "full": {
            if (msg.theme_css) themeNode().textContent = msg.theme_css;
            root().innerHTML = msg.html;
            await runExtensions(root());
            break;
        }
        case "patch": {
            const target = msg.target ? document.querySelector<HTMLElement>(msg.target) : root();
            if (target) {
                target.innerHTML = msg.html;
                await runExtensions(target);
            }
            break;
        }
        case "theme": {
            themeNode().textContent = msg.css;
            break;
        }
    }
}

function connect(): void {
    const proto = location.protocol === "https:" ? "wss:" : "ws:";
    const url = `${proto}//${location.host}/__mdview_live`;
    const ws = new WebSocket(url);

    ws.addEventListener("message", (ev) => {
        try {
            const msg = JSON.parse(typeof ev.data === "string" ? ev.data : "") as Msg;
            void apply(msg);
        } catch (err) {
            console.warn("[mdview] bad message", err);
        }
    });

    ws.addEventListener("close", () => {
        setTimeout(connect, 1000);
    });

    ws.addEventListener("error", () => {
        try {
            ws.close();
        } catch {
            /* noop */
        }
    });
}

if (typeof window !== "undefined") {
    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", () => connect());
    } else {
        connect();
    }
}

export { apply, extensions };
