// SPDX-License-Identifier: MPL-2.0
import "server-only";
import {
  createCipheriv,
  createHash,
  createDecipheriv,
  randomBytes,
  timingSafeEqual,
} from "node:crypto";
import {
  Kanche,
  KancheError,
  serviceUrl,
  type Config,
  type Verified,
  type Grant,
} from "@kanche/server";
import type { BrowserSession } from "./client.ts";
export type AdapterConfig = Config & {
  origin: string;
  cookieKeys: Record<string, string>;
  currentKey: string;
  hostedCallbackUri?: string;
};
const safe = (v: Verified): BrowserSession => ({
  authenticated: true,
  user_id: v.user.id,
  idle_expires_at: v.session.idle_expires_at,
  absolute_expires_at: v.session.absolute_expires_at,
});
type Payload = { token: string; expires: number };
type HostedPayload = { state: string; verifier: string; expires: number };
export class KancheAdapter {
  #api: Kanche;
  #config: AdapterConfig;
  #keys: Record<string, Buffer>;
  #origin: string;
  constructor(config: AdapterConfig) {
    this.#origin = serviceUrl(config.origin, config.development).origin;
    if (
      config.development &&
      !["localhost", "127.0.0.1", "[::1]"].includes(
        new URL(this.#origin).hostname,
      )
    )
      throw new KancheError("invalid_configuration");
    if (
      config.hostedCallbackUri !== undefined &&
      config.hostedCallbackUri !== this.#origin + "/api/kanche/callback"
    )
      throw new KancheError("invalid_hosted_callback");
    this.#config = { ...config, cookieKeys: { ...config.cookieKeys } };
    this.#api = new Kanche(config);
    this.#keys = Object.create(null);
    for (const [id, key] of Object.entries(config.cookieKeys)) {
      if (!/^[a-zA-Z0-9_-]{1,32}$/.test(id) || !/^[a-f0-9]{64}$/i.test(key))
        throw new KancheError("invalid_cookie_keys");
      this.#keys[id] = Buffer.from(key, "hex");
    }
    if (!Object.hasOwn(this.#keys, config.currentKey))
      throw new KancheError("invalid_cookie_keys");
  }
  toJSON(): never {
    throw new KancheError("server_config_cannot_be_serialized");
  }
  #name(purpose: string): string {
    return `${this.#config.development ? "kanche_dev_" : "__Host-kanche_"}${purpose}`;
  }
  #aad(purpose: string, id: string): Buffer {
    return Buffer.from(
      JSON.stringify([
        "kanche-v1",
        this.#origin,
        this.#config.tenantId,
        this.#config.applicationId,
        purpose,
        id,
      ]),
    );
  }
  #seal(purpose: string, payload: Payload | HostedPayload): string {
    const id = this.#config.currentKey;
    const iv = randomBytes(12);
    const cipher = createCipheriv("aes-256-gcm", this.#keys[id], iv);
    cipher.setAAD(this.#aad(purpose, id));
    const encrypted = Buffer.concat([
      cipher.update(JSON.stringify(payload), "utf8"),
      cipher.final(),
    ]);
    return [
      id,
      iv.toString("base64url"),
      encrypted.toString("base64url"),
      cipher.getAuthTag().toString("base64url"),
    ].join(".");
  }
  #decode(purpose: string, cookies: string): Record<string, unknown> | null {
    try {
      const entries = cookies
        .split(";")
        .map((x) => x.trim())
        .filter((x) => x.startsWith(this.#name(purpose) + "="));
      if (entries.length !== 1) return null;
      const value = entries[0].slice(this.#name(purpose).length + 1);
      if (value.length > 2048) return null;
      const parts = value.split(".");
      if (parts.length !== 4) return null;
      const [id, nonce, text, tag] = parts;
      if (!Object.hasOwn(this.#keys, id)) return null;
      const iv = Buffer.from(nonce, "base64url"),
        auth = Buffer.from(tag, "base64url");
      if (iv.length !== 12 || auth.length !== 16) return null;
      const decipher = createDecipheriv("aes-256-gcm", this.#keys[id], iv);
      decipher.setAAD(this.#aad(purpose, id));
      decipher.setAuthTag(auth);
      const payload = JSON.parse(
        Buffer.concat([
          decipher.update(Buffer.from(text, "base64url")),
          decipher.final(),
        ]).toString("utf8"),
      );
      if (
        !payload ||
        typeof payload !== "object" ||
        Array.isArray(payload) ||
        !Number.isSafeInteger(payload.expires) ||
        payload.expires <= Math.floor(Date.now() / 1000)
      )
        return null;
      return payload;
    } catch {
      return null;
    }
  }
  #open(purpose: string, cookies: string): Payload | null {
    const value = this.#decode(purpose, cookies);
    if (
      !value ||
      typeof value.token !== "string" ||
      !/^[a-f0-9]{64}$/.test(value.token)
    )
      return null;
    return { token: value.token, expires: value.expires as number };
  }
  #hosted(cookies: string): HostedPayload | null {
    const value = this.#decode("hosted", cookies);
    if (
      !value ||
      typeof value.state !== "string" ||
      !/^[a-f0-9]{64}$/.test(value.state) ||
      typeof value.verifier !== "string" ||
      !/^[a-f0-9]{64}$/.test(value.verifier)
    )
      return null;
    return {
      state: value.state,
      verifier: value.verifier,
      expires: value.expires as number,
    };
  }
  #cookie(purpose: string, value: string, expires: number): string {
    return `${this.#name(purpose)}=${value}; Path=/; HttpOnly; SameSite=Lax; Expires=${new Date(expires * 1000).toUTCString()}${this.#config.development ? "" : "; Secure"}`;
  }
  async read(cookies: string): Promise<BrowserSession> {
    const payload = this.#open("session", cookies);
    if (!payload) return { authenticated: false };
    try {
      return safe(await this.#api.verify(payload.token));
    } catch (e) {
      if (e instanceof KancheError && e.code === "unauthenticated")
        return { authenticated: false };
      throw e;
    }
  }
  #headers(): Record<string, string> {
    return {
      "cache-control": "no-store",
      "x-content-type-options": "nosniff",
      "referrer-policy": "no-referrer",
      "content-security-policy":
        "default-src 'none'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'",
    };
  }
  #response(body: unknown, status = 200): Response {
    return Response.json(body, { status, headers: this.#headers() });
  }
  #redirect(destination: string): Response {
    return new Response(null, {
      status: 303,
      headers: { ...this.#headers(), location: destination },
    });
  }
  async #persist(grant: Grant, redirect = false): Promise<Response> {
    try {
      const expires = grant.verified.session.absolute_expires_at;
      const response = redirect
        ? this.#redirect("/protected")
        : this.#response(safe(grant.verified));
      response.headers.append(
        "set-cookie",
        this.#cookie(
          "session",
          this.#seal("session", { token: grant.serverCredential(), expires }),
          expires,
        ),
      );
      const fresh = {
        token: randomBytes(32).toString("hex"),
        expires: Math.floor(Date.now() / 1000) + 600,
      };
      response.headers.append(
        "set-cookie",
        this.#cookie("csrf", this.#seal("csrf", fresh), fresh.expires),
      );
      return response;
    } catch {
      try {
        await this.#api.revoke(grant.serverCredential());
      } catch {
        /* expiry bounds orphan */
      }
      throw new KancheError("temporarily_unavailable");
    }
  }
  async #callback(request: Request, cookies: string): Promise<Response> {
    if (!this.#config.hostedCallbackUri)
      return this.#response({ code: "invalid_request" }, 400);
    const url = new URL(request.url),
      params = url.searchParams;
    if (
      url.origin + url.pathname !== this.#config.hostedCallbackUri ||
      url.hash ||
      [...params.keys()].length !== 2 ||
      params.getAll("code").length !== 1 ||
      params.getAll("state").length !== 1
    )
      return this.#response({ code: "invalid_handoff" }, 400);
    const code = params.get("code") ?? "",
      state = params.get("state") ?? "",
      pending = this.#hosted(cookies);
    if (
      !pending ||
      !/^[a-f0-9]{64}$/.test(code) ||
      !/^[a-f0-9]{64}$/.test(state) ||
      !timingSafeEqual(Buffer.from(state), Buffer.from(pending.state))
    )
      return this.#response({ code: "invalid_handoff" }, 400);
    const grant = await this.#api.exchangeHosted(
      code,
      pending.verifier,
      this.#config.hostedCallbackUri,
    );
    const response = await this.#persist(grant, true);
    response.headers.append("set-cookie", this.#cookie("hosted", "", 0));
    return response;
  }
  async handle(request: Request, operation: string): Promise<Response> {
    try {
      const cookies = request.headers.get("cookie") ?? "";
      if (operation === "callback" && request.method === "GET")
        return await this.#callback(request, cookies);
      if (operation === "csrf" && request.method === "GET") {
        const payload = {
          token: randomBytes(32).toString("hex"),
          expires: Math.floor(Date.now() / 1000) + 600,
        };
        const r = this.#response({ csrf_token: payload.token });
        r.headers.append(
          "set-cookie",
          this.#cookie("csrf", this.#seal("csrf", payload), payload.expires),
        );
        return r;
      }
      if (operation === "session" && request.method === "GET")
        return this.#response(await this.read(cookies));
      if (
        !["sign-in", "renew", "sign-out", "hosted-start"].includes(operation) ||
        request.method !== "POST"
      )
        return this.#response({ code: "invalid_request" }, 400);
      if (
        request.headers.get("origin") !== this.#origin ||
        request.headers.get("content-type")?.split(";")[0] !==
          "application/json"
      )
        return this.#response({ code: "csrf_rejected" }, 403);
      const csrf = this.#open("csrf", cookies),
        provided = request.headers.get("x-csrf-token") ?? "";
      if (
        !csrf ||
        !/^[a-f0-9]{64}$/.test(provided) ||
        !timingSafeEqual(Buffer.from(csrf.token), Buffer.from(provided))
      )
        return this.#response({ code: "csrf_rejected" }, 403);
      // Stream the body with a hard bound even for chunked requests.
      let text = "";
      let size = 0;
      const reader = request.body?.getReader();
      const chunks: Uint8Array[] = [];
      if (reader) {
        while (true) {
          const p = await reader.read();
          if (p.done) break;
          size += p.value.length;
          if (size > 32768) {
            await reader.cancel();
            return this.#response({ code: "payload_too_large" }, 413);
          }
          chunks.push(p.value);
        }
        text = Buffer.concat(chunks).toString("utf8");
      }
      let body: Record<string, unknown>;
      try {
        body = JSON.parse(text);
        if (!body || typeof body !== "object" || Array.isArray(body))
          throw new Error();
      } catch {
        return this.#response({ code: "invalid_request" }, 400);
      }
      if (operation === "hosted-start") {
        if (!this.#config.hostedCallbackUri || Object.keys(body).length)
          return this.#response({ code: "invalid_request" }, 400);
        const state = randomBytes(32).toString("hex"),
          verifier = randomBytes(32).toString("hex");
        const challenge = createHash("sha256")
          .update(verifier, "ascii")
          .digest("base64url");
        const transaction = await this.#api.startHosted(
          this.#config.hostedCallbackUri,
          state,
          challenge,
        );
        const expires = Math.min(
          transaction.expires_at,
          Math.floor(Date.now() / 1000) + 300,
        );
        if (expires <= Math.floor(Date.now() / 1000))
          throw new KancheError("invalid_response");
        const response = this.#response(transaction);
        response.headers.append(
          "set-cookie",
          this.#cookie(
            "hosted",
            this.#seal("hosted", { state, verifier, expires }),
            expires,
          ),
        );
        return response;
      }
      if (operation === "sign-in") {
        if (
          typeof body.email !== "string" ||
          typeof body.password !== "string" ||
          Object.keys(body).some((k) => !["email", "password"].includes(k))
        )
          return this.#response({ code: "invalid_request" }, 400);
        return await this.#persist(
          await this.#api.signIn(body.email, body.password),
        );
      }
      if (Object.keys(body).length)
        return this.#response({ code: "invalid_request" }, 400);
      const payload = this.#open("session", cookies);
      if (operation === "renew") {
        if (!payload) return this.#response({ authenticated: false }, 401);
        return this.#response(safe(await this.#api.renew(payload.token)));
      }
      if (payload) await this.#api.revoke(payload.token);
      const r = this.#response({ authenticated: false });
      r.headers.append("set-cookie", this.#cookie("session", "", 0));
      return r;
    } catch (e) {
      // Never echo upstream/configuration errors or serialize credentials.
      const allowed = [
        "authentication_failed",
        "unauthenticated",
        "rate_limited",
        "invalid_request",
        "invalid_handoff",
      ];
      if (e instanceof KancheError && allowed.includes(e.code))
        return this.#response({ code: e.code }, e.status);
      return this.#response({ code: "temporarily_unavailable" }, 503);
    }
  }
}
