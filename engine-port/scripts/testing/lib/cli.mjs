export function parseArgs(argv = process.argv.slice(2)) {
  const out = { _: [] };

  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i];
    if (!token.startsWith("--")) {
      out._.push(token);
      continue;
    }

    const withoutPrefix = token.slice(2);
    const eqIdx = withoutPrefix.indexOf("=");
    if (eqIdx >= 0) {
      const key = withoutPrefix.slice(0, eqIdx);
      const value = withoutPrefix.slice(eqIdx + 1);
      out[key] = value;
      continue;
    }

    const key = withoutPrefix;
    const next = argv[i + 1];
    if (next !== undefined && !next.startsWith("--")) {
      out[key] = next;
      i += 1;
    } else {
      out[key] = true;
    }
  }

  return out;
}

export function asBool(value, defaultValue = false) {
  if (value === undefined) {
    return defaultValue;
  }
  if (typeof value === "boolean") {
    return value;
  }

  const lowered = String(value).trim().toLowerCase();
  if (["1", "true", "yes", "y", "on"].includes(lowered)) {
    return true;
  }
  if (["0", "false", "no", "n", "off"].includes(lowered)) {
    return false;
  }

  return defaultValue;
}

export function asInt(value, defaultValue = 0) {
  if (value === undefined || value === "") {
    return defaultValue;
  }
  const parsed = Number.parseInt(String(value), 10);
  return Number.isNaN(parsed) ? defaultValue : parsed;
}

export function nonEmpty(value, defaultValue = "") {
  if (value === undefined || value === null) {
    return defaultValue;
  }
  const trimmed = String(value).trim();
  return trimmed === "" ? defaultValue : trimmed;
}
