import { asInt, nonEmpty, parseArgs } from "./lib/cli.mjs";
import { startFixtureServer } from "./lib/fixture_server.mjs";

const args = parseArgs();
const host = nonEmpty(args.host, "127.0.0.1");
const port = asInt(args.port, 4173);

const server = await startFixtureServer({ host, port });
console.log(`[fixture-server] serving ${server.rootDir} at ${server.baseUrl}`);

async function shutdown(signal) {
  try {
    console.log(`[fixture-server] ${signal} received, shutting down`);
    await server.close();
  } finally {
    process.exit(0);
  }
}

process.on("SIGINT", () => shutdown("SIGINT"));
process.on("SIGTERM", () => shutdown("SIGTERM"));
