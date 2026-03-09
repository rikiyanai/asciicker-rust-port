import fs from "node:fs/promises";
import { createServer } from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";

const MIME_TYPES = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".txt": "text/plain; charset=utf-8",
};

function contentTypeFor(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  return MIME_TYPES[ext] || "application/octet-stream";
}

function defaultFixtureRoot() {
  const here = path.dirname(fileURLToPath(import.meta.url));
  return path.resolve(here, "..", "fixtures", "site");
}

export async function startFixtureServer({
  host = "127.0.0.1",
  port = 4173,
  rootDir = defaultFixtureRoot(),
} = {}) {
  const resolvedRoot = path.resolve(rootDir);

  const server = createServer(async (req, res) => {
    try {
      const reqUrl = new URL(req.url || "/", `http://${req.headers.host || "localhost"}`);
      const pathname = decodeURIComponent(reqUrl.pathname === "/" ? "/index.html" : reqUrl.pathname);
      const relativePath = pathname.replace(/^\/+/, "");
      const filePath = path.resolve(resolvedRoot, relativePath);

      if (!filePath.startsWith(resolvedRoot)) {
        res.writeHead(403, { "content-type": "text/plain; charset=utf-8" });
        res.end("Forbidden");
        return;
      }

      const body = await fs.readFile(filePath);
      res.writeHead(200, { "content-type": contentTypeFor(filePath) });
      res.end(body);
    } catch (_error) {
      res.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
      res.end("Not Found");
    }
  });

  async function listen(targetPort) {
    await new Promise((resolve, reject) => {
      const onError = (error) => {
        server.off("listening", onListening);
        reject(error);
      };
      const onListening = () => {
        server.off("error", onError);
        resolve();
      };
      server.once("error", onError);
      server.once("listening", onListening);
      server.listen(targetPort, host);
    });
  }

  let selectedPort = port;
  try {
    await listen(selectedPort);
  } catch (error) {
    if (error?.code !== "EADDRINUSE") {
      throw error;
    }
    await listen(0);
    selectedPort = server.address()?.port ?? 0;
  }

  if (!selectedPort) {
    selectedPort = server.address()?.port ?? port;
  }

  return {
    baseUrl: `http://${host}:${selectedPort}`,
    rootDir: resolvedRoot,
    close: () =>
      new Promise((resolve, reject) => {
        server.close((error) => (error ? reject(error) : resolve()));
      }),
  };
}
