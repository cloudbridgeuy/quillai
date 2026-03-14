import { useRef, type FormEvent } from "react";

type ApiError = { error: string; code: string };

export function formatResponseBody(
  ok: boolean,
  status: number,
  statusText: string,
  body: unknown,
): string {
  if (ok) return JSON.stringify(body, null, 2);

  if (body && typeof body === "object" && "error" in body && "code" in body) {
    const apiError = body as ApiError;
    return `Error [${apiError.code}]: ${apiError.error}`;
  }

  return `${status} ${statusText}`;
}

export function APITester() {
  const responseInputRef = useRef<HTMLTextAreaElement>(null);

  const testEndpoint = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    try {
      const form = e.currentTarget;
      const formData = new FormData(form);
      const endpoint = formData.get("endpoint");
      if (typeof endpoint !== "string") return;
      const url = new URL(endpoint, location.href);
      const method = formData.get("method");
      if (typeof method !== "string") return;
      const res = await fetch(url, { method });

      let body: unknown;
      try {
        body = await res.json();
      } catch {
        body = null;
      }
      if (responseInputRef.current) {
        responseInputRef.current.value = formatResponseBody(
          res.ok,
          res.status,
          res.statusText,
          body,
        );
      }
    } catch (error) {
      if (responseInputRef.current) {
        responseInputRef.current.value = String(error);
      }
    }
  };

  return (
    <div className="api-tester">
      <form onSubmit={testEndpoint} className="endpoint-row">
        <select name="method" className="method">
          <option value="GET">GET</option>
          <option value="PUT">PUT</option>
        </select>
        <input type="text" name="endpoint" defaultValue="/api/hello" className="url-input" placeholder="/api/hello" />
        <button type="submit" className="send-button">
          Send
        </button>
      </form>
      <textarea ref={responseInputRef} readOnly placeholder="Response will appear here..." className="response-area" />
    </div>
  );
}
