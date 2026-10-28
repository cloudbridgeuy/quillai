import { createServer, type ServerResponse } from "node:http";
import { writeFileSync, renameSync } from "node:fs";
import { EventEmitter, once } from "node:events";

type RecordedRequest = { id: number; identity: string; model: string; stream: true; closed: boolean; completed: boolean };
const requests: RecordedRequest[] = [];
const streams = new Map<number, ServerResponse>();
const armed = new Map<string, string>();
let unarmed = 0;
const events = new EventEmitter();
function audit() {
  console.log(`E2E provider audit: ${JSON.stringify({ unarmed, streamed: requests.map(({ id, identity, model }) => ({ id, identity, model })) })}`);
}
function rejectUnarmed(response: ServerResponse) {
  unarmed++;
  audit();
  response.writeHead(503, { "Content-Type": "application/json" });
  response.end(JSON.stringify({ error: { message: "E2E provider request was not armed" } }));
}
const server = createServer(async (request, response) => {
  try {
    if (request.method === "GET" && request.url === "/control") {
      response.writeHead(200, { "Content-Type": "application/json" });
      response.end(JSON.stringify(requests));
      return;
    }
    if (request.method === "POST" && request.url === "/v1/chat/completions" && armed.size === 0) {
      request.resume();
      rejectUnarmed(response);
      return;
    }
    const observation = /^\/control\/(\d+)\/(request|closed)$/.exec(request.url ?? "");
    if (request.method === "GET" && observation) {
      const id = Number(observation[1]);
      if (!requests[id]) await once(events, `request:${id}`);
      if (observation[2] === "closed" && !requests[id]!.closed) await once(events, `closed:${id}`);
      response.writeHead(200, { "Content-Type": "application/json" });
      response.end(JSON.stringify(requests[id]));
      return;
    }
    let body = "";
    for await (const chunk of request) {
      body += chunk;
      if (body.length > 1_000_000) throw new Error("Request too large");
    }
    if (request.method === "POST" && request.url === "/control/arm") {
      const input: { identity?: unknown; model?: unknown } = JSON.parse(body);
      const continueIdentity = typeof input.identity === "string" && /^GUZ(?:(807|985)-(Finish|Stop)|866-(seed|length|stop))-[a-f0-9-]{36}$/.test(input.identity) && input.model === "e2e-continue";
      const editIdentity = typeof input.identity === "string" && /^(GUZ865-(commentary|missing|unclosed)|GUZ985-edit)-[a-f0-9-]{36}$/.test(input.identity) && input.model === "e2e-edit";
      const reasoningIdentity = typeof input.identity === "string" && /^GUZ1008-[a-f0-9-]{36}$/.test(input.identity) && (input.model === "e2e-reasoning" || input.model === "e2e-no-reasoning");
      if (typeof input.identity !== "string" || !(continueIdentity || editIdentity || reasoningIdentity)) {
        response.writeHead(422).end();
        return;
      }
      if (armed.has(input.identity) || requests.some(record => record.identity === input.identity)) {
        response.writeHead(409).end();
        return;
      }
      armed.set(input.identity, input.model);
      response.writeHead(200).end();
      return;
    }
    if (request.method === "POST" && request.url === "/v1/chat/completions") {
      if (request.headers.authorization !== "Bearer quillai-e2e-fake-key-not-a-secret") {
        response.writeHead(401).end();
        return;
      }
      const input: { stream?: unknown; model?: unknown; reasoning_effort?: unknown; messages?: Array<{ content?: unknown }> } = JSON.parse(body);
      const match = Array.from(armed).find(([identity, model]) => model === input.model && Array.isArray(input.messages) && input.messages.some(message => typeof message?.content === "string" && message.content.includes(identity)));
      if (!match) {
        rejectUnarmed(response);
        return;
      }
      if (input.stream !== true || typeof input.model !== "string" || !["e2e-continue", "e2e-edit", "e2e-reasoning", "e2e-no-reasoning"].includes(input.model)) {
        response.writeHead(422).end();
        return;
      }
      const [identity] = match;
      armed.delete(identity);
      const record: RecordedRequest = { id: requests.length, identity, model: input.model, stream: true, closed: false, completed: false };
      requests.push(record);
      audit();
      streams.set(record.id, response);
      response.on("close", () => {
        record.closed = true;
        streams.delete(record.id);
        events.emit(`closed:${record.id}`);
      });
      response.writeHead(200, { "Content-Type": "text/event-stream", "Cache-Control": "no-cache" });
      response.flushHeaders();
      events.emit(`request:${record.id}`);
      if (identity.startsWith("GUZ1008-")) {
        if (input.reasoning_effort === "low") response.write(`data: ${JSON.stringify({ choices: [{ delta: { content: "Reasoned continuation." } }] })}\n\n`);
        record.completed = true;
        response.end(`data: ${JSON.stringify({ choices: [{ delta: {}, finish_reason: input.reasoning_effort === "low" ? "stop" : "length" }] })}\n\ndata: [DONE]\n\n`);
      }
      return;
    }
    const control = /^\/control\/(\d+)\/(fragment|usage|finish)$/.exec(request.url ?? "");
    if (request.method === "POST" && control) {
      const id = Number(control[1]);
      const stream = streams.get(id);
      if (!stream || requests[id]!.completed) {
        response.writeHead(409).end();
        return;
      }
      if (control[2] === "fragment") {
        const { text }: { text?: unknown } = JSON.parse(body);
        if (typeof text !== "string" || !text.length) throw new Error("Missing fragment");
        stream.write(`data: ${JSON.stringify({ choices: [{ delta: { content: text } }] })}\n\n`);
      } else if (control[2] === "usage") {
        stream.write(`data: ${JSON.stringify({ choices: [], usage: { prompt_tokens: 11, completion_tokens: 0 } })}\n\n`);
      } else {
        const { reason = "stop" }: { reason?: "stop" | "length" } = body ? JSON.parse(body) : {};
        if (reason !== "stop" && reason !== "length") throw new Error("Invalid finish reason");
        requests[id]!.completed = true;
        stream.end(`data: ${JSON.stringify({ choices: [{ delta: {}, finish_reason: reason }] })}\n\ndata: [DONE]\n\n`);
      }
      response.writeHead(200).end();
      return;
    }
    response.writeHead(404).end();
  } catch (error) {
    console.error(error);
    response.writeHead(400).end();
  }
});

server.listen(0, "127.0.0.1", () => {
  const address = server.address();
  if (!address || typeof address === "string") throw new Error("Missing listener address");
  const { port } = address;
  const readiness = process.argv[2];
  if (!readiness) throw new Error("Missing readiness path");
  writeFileSync(`${readiness}.tmp`, String(port));
  renameSync(`${readiness}.tmp`, readiness);
  console.log(`E2E fake AI: http://127.0.0.1:${port}`);
  audit();
});
