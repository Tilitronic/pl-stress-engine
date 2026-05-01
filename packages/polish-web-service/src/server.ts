import Fastify from "fastify";

type StressInfo = {
  word: string;
  syllables: string[];
  syllableIndex: number;
  stressFromEnd: number;
  ipa: string | null;
  confidence: "exact" | "rule" | "default";
};

const host = process.env.HOST ?? "0.0.0.0";
const port = Number(process.env.PORT ?? 8787);

const wasmModuleUrl = new URL(
  "../../../crates/wasm/pkg-node/pl_stress_wasm.js",
  import.meta.url,
);
const wasm = await import(wasmModuleUrl.href);
if (typeof wasm.default === "function") {
  await wasm.default();
}

const stress = wasm.stress as (word: string) => number;
const stressInfo = wasm.stressInfo as (word: string) => StressInfo;

if (typeof stress !== "function" || typeof stressInfo !== "function") {
  throw new Error(
    "WASM exports not found. Build with: pnpm run build:wasm:node",
  );
}

function normalizeSingleWordInput(value: unknown): string | null {
  const word = String(value ?? "").trim();
  if (!word) {
    return null;
  }

  // This service analyzes single words only.
  if (/\s/.test(word)) {
    return null;
  }

  return word;
}

const app = Fastify({ logger: true });

app.get("/health", async () => ({ ok: true }));

app.get<{ Querystring: { word?: string } }>(
  "/stress",
  async (request, reply) => {
    const word = normalizeSingleWordInput(request.query.word);
    if (!word) {
      reply.code(400);
      return { error: "Provide a single word in query parameter: word" };
    }

    return stressInfo(word);
  },
);

app.post<{ Body: { word?: string } }>("/stress", async (request, reply) => {
  const word = normalizeSingleWordInput(request.body.word);
  if (!word) {
    reply.code(400);
    return { error: "Provide a single word in body field: word" };
  }

  return stressInfo(word);
});

app.get<{ Querystring: { word?: string } }>(
  "/stress/index",
  async (request, reply) => {
    const word = normalizeSingleWordInput(request.query.word);
    if (!word) {
      reply.code(400);
      return { error: "Provide a single word in query parameter: word" };
    }

    return { word, syllableIndex: stress(word) };
  },
);

await app.listen({ host, port });
