// SPDX-License-Identifier: MPL-2.0
import { inspect } from "node:util";
if (typeof window !== "undefined")
  throw new Error("Kanche SDK requires a server");
export type Identity = {
  id: string;
  tenant_id: string;
  application_id: string;
};
export type Session = {
  format: "opaque";
  idle_expires_at: number;
  absolute_expires_at: number;
};
export type Verified = { user: Identity; session: Session };
export type HostedTransaction = {
  authorization_url: string;
  expires_at: number;
};
export class KancheError extends Error {
  readonly code: string;
  readonly status: number;
  constructor(code: string, status = 503) {
    super(`Kanche: ${code}`);
    this.name = "KancheError";
    this.code = code;
    this.status = status;
  }
}
export class Grant {
  #credential: string;
  readonly verified: Verified;
  constructor(verified: Verified, credential: string) {
    this.verified = verified;
    this.#credential = credential;
  }
  serverCredential(): string {
    return this.#credential;
  }
  toJSON(): never {
    throw new KancheError("server_grant_cannot_be_serialized");
  }
  [inspect.custom](): string {
    return "KancheGrant [redacted]";
  }
}
export type Config = {
  baseUrl: string;
  applicationSecret: string;
  tenantId: string;
  applicationId: string;
  development?: boolean;
};
export function serviceUrl(value: string, development = false): URL {
  let u: URL;
  try {
    u = new URL(value);
  } catch {
    throw new KancheError("invalid_configuration");
  }
  if (
    u.username ||
    u.password ||
    u.search ||
    u.hash ||
    u.pathname !== "/" ||
    !(
      u.protocol === "https:" ||
      (development &&
        u.protocol === "http:" &&
        ["127.0.0.1", "localhost", "[::1]"].includes(u.hostname))
    )
  )
    throw new KancheError("invalid_configuration");
  return u;
}
export class Kanche {
  #config: Config;
  constructor(config: Config) {
    serviceUrl(config.baseUrl, config.development);
    if (
      !/^[a-f0-9]{64}$/i.test(config.applicationSecret) ||
      !config.tenantId ||
      !config.applicationId
    )
      throw new KancheError("invalid_configuration");
    this.#config = { ...config };
  }
  [inspect.custom](): string {
    return "Kanche [redacted]";
  }
  toJSON(): never {
    throw new KancheError("server_config_cannot_be_serialized");
  }
  async #call(
    path: string,
    method = "GET",
    credential?: string,
    body?: unknown,
  ): Promise<unknown> {
    try {
      const headers: Record<string, string> = {
        authorization: `Bearer ${this.#config.applicationSecret}`,
        accept: "application/json",
      };
      if (credential) headers["kanche-session"] = credential;
      if (body) headers["content-type"] = "application/json";
      const r = await fetch(new URL(path, this.#config.baseUrl), {
        method,
        headers,
        body: body ? JSON.stringify(body) : undefined,
        cache: "no-store",
        redirect: "error",
        signal: AbortSignal.timeout(5000),
      });
      const reader = r.body?.getReader();
      let size = 0;
      const parts: Uint8Array[] = [];
      if (reader) {
        while (true) {
          const part = await reader.read();
          if (part.done) break;
          size += part.value.length;
          if (size > 32768) {
            await reader.cancel();
            throw new KancheError("invalid_response");
          }
          parts.push(part.value);
        }
      }
      const data = size
        ? JSON.parse(Buffer.concat(parts).toString("utf8"))
        : null;
      if (!r.ok) {
        const known = [
          "invalid_request",
          "invalid_application_credentials",
          "authentication_failed",
          "unauthenticated",
          "rate_limited",
          "temporarily_unavailable",
          "invalid_handoff",
          "invalid_redirect_uri",
        ];
        throw new KancheError(
          known.includes(data?.code) ? data.code : "service_error",
          r.status,
        );
      }
      return data;
    } catch (e) {
      if (e instanceof KancheError) throw e;
      throw new KancheError("temporarily_unavailable");
    }
  }
  #verified(value: unknown): Verified {
    const v = value as Verified;
    if (
      !v ||
      !v.user ||
      typeof v.user.id !== "string" ||
      !v.user.id ||
      v.user.tenant_id !== this.#config.tenantId ||
      v.user.application_id !== this.#config.applicationId ||
      v.session?.format !== "opaque" ||
      !Number.isSafeInteger(v.session.idle_expires_at) ||
      !Number.isSafeInteger(v.session.absolute_expires_at) ||
      v.session.idle_expires_at > v.session.absolute_expires_at
    )
      throw new KancheError("invalid_response");
    return {
      user: {
        id: v.user.id,
        tenant_id: v.user.tenant_id,
        application_id: v.user.application_id,
      },
      session: {
        format: "opaque",
        idle_expires_at: v.session.idle_expires_at,
        absolute_expires_at: v.session.absolute_expires_at,
      },
    };
  }
  #grant(value: unknown): Grant {
    const verified = this.#verified(value);
    const credential = (value as { session: { credential?: unknown } }).session
      .credential;
    if (typeof credential !== "string" || !/^[a-f0-9]{64}$/.test(credential))
      throw new KancheError("invalid_response");
    return new Grant(verified, credential);
  }
  async signIn(email: string, password: string): Promise<Grant> {
    return this.#grant(
      await this.#call("/v1/auth/sign-ins", "POST", undefined, {
        email,
        password,
      }),
    );
  }
  async startHosted(
    redirectUri: string,
    state: string,
    codeChallenge: string,
  ): Promise<HostedTransaction> {
    if (
      !/^[a-f0-9]{64}$/.test(state) ||
      !/^[A-Za-z0-9_-]{43}$/.test(codeChallenge) ||
      typeof redirectUri !== "string"
    )
      throw new KancheError("invalid_request", 400);
    const value = (await this.#call(
      "/v1/hosted/transactions",
      "POST",
      undefined,
      {
        redirect_uri: redirectUri,
        state,
        code_challenge: codeChallenge,
        code_challenge_method: "S256",
      },
    )) as HostedTransaction;
    try {
      if (
        !value ||
        typeof value.authorization_url !== "string" ||
        !Number.isSafeInteger(value.expires_at) ||
        value.expires_at <= 0
      )
        throw new KancheError("invalid_response");
      const target = new URL(value.authorization_url);
      const transaction = target.searchParams.get("transaction");
      const expected =
        new URL(this.#config.baseUrl).origin +
        "/hosted/login?transaction=" +
        transaction;
      if (
        !transaction ||
        !/^[a-f0-9]{64}$/.test(transaction) ||
        target.href !== expected ||
        value.authorization_url !== expected
      )
        throw new KancheError("invalid_response");
      return {
        authorization_url: value.authorization_url,
        expires_at: value.expires_at,
      };
    } catch {
      throw new KancheError("invalid_response");
    }
  }
  async exchangeHosted(
    code: string,
    codeVerifier: string,
    redirectUri: string,
  ): Promise<Grant> {
    if (
      !/^[a-f0-9]{64}$/.test(code) ||
      !/^[A-Za-z0-9._~-]{43,128}$/.test(codeVerifier) ||
      typeof redirectUri !== "string"
    )
      throw new KancheError("invalid_handoff", 400);
    return this.#grant(
      await this.#call("/v1/hosted/exchanges", "POST", undefined, {
        code,
        code_verifier: codeVerifier,
        redirect_uri: redirectUri,
      }),
    );
  }
  async verify(credential: string): Promise<Verified> {
    return this.#verified(
      await this.#call("/v1/sessions/current", "GET", credential),
    );
  }
  async renew(credential: string): Promise<Verified> {
    return this.#verified(
      await this.#call("/v1/sessions/current/renew", "POST", credential),
    );
  }
  async revoke(credential: string): Promise<void> {
    await this.#call("/v1/sessions/current/revoke", "POST", credential);
  }
}
