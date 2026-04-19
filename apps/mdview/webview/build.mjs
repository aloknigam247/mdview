import { build, context } from "esbuild";
import { mkdirSync, copyFileSync, existsSync, writeFileSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const distDir = resolve(__dirname, "dist");
mkdirSync(distDir, { recursive: true });

const watchMode = process.argv.includes("--watch");

const shared = {
    entryPoints: [resolve(__dirname, "src/client.ts")],
    bundle: true,
    format: "esm",
    platform: "browser",
    target: ["chrome110", "safari16"],
    sourcemap: false,
    minify: true,
    logLevel: "info",
    outfile: resolve(distDir, "client.js"),
    loader: { ".css": "text" },
};

function copyStatic() {
    const indexSrc = resolve(__dirname, "src/index.html");
    if (existsSync(indexSrc)) {
        copyFileSync(indexSrc, resolve(distDir, "index.html"));
    } else {
        writeFileSync(
            resolve(distDir, "index.html"),
            "<!doctype html><meta charset=utf-8><title>mdview</title>"
        );
    }
}

copyStatic();

if (watchMode) {
    const ctx = await context(shared);
    await ctx.watch();
    console.log("[mdview] esbuild watching...");
} else {
    await build(shared);
    console.log("[mdview] build complete:", shared.outfile);
}
