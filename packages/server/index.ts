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
  async signIn(email: string, password: string): Promise<Grant> {
    const value = (await this.#call("/v1/auth/sign-ins", "POST", undefined, {
      email,
      password,
    })) as Verified & { session: Session & { credential: string } };
    const verified = this.#verified(value);
    if (!/^[a-f0-9]{64}$/.test(value.session.credential))
      throw new KancheError("invalid_response");
    return new Grant(verified, value.session.credential);
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
