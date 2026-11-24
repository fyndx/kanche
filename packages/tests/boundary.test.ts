// SPDX-License-Identifier: MPL-2.0
import test from "node:test";
import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";
import { inspect } from "node:util";
import { Kanche, Grant, KancheError } from "../server/index.ts";
import { KancheAdapter } from "../nextjs/server.ts";
const random = () => randomBytes(32).toString("hex");
const config = {
  baseUrl: "http://127.0.0.1:8080",
  applicationSecret: random(),
  tenantId: "t",
  applicationId: "a",
  development: true,
};
const verified = {
  user: { id: "u", tenant_id: "t", application_id: "a" },
  session: {
    format: "opaque" as const,
    idle_expires_at: Math.floor(Date.now() / 1000) + 86400,
    absolute_expires_at: Math.floor(Date.now() / 1000) + 604800,
  },
};
test("SDK validates scope, formats and bounds; diagnostics and serialization redact grants", async () => {
  const original = globalThis.fetch;
  const token = random();
  const api = new Kanche(config);
  try {
    globalThis.fetch = async () =>
      Response.json({
        ...verified,
        session: { ...verified.session, credential: token },
      });
    const grant = await api.signIn("synthetic@example.invalid", random());
    assert.ok(grant instanceof Grant);
    assert.throws(() => JSON.stringify(grant));
    assert.ok(!inspect(grant).includes(token));
    assert.ok(!inspect(api).includes(config.applicationSecret));
    assert.throws(() => JSON.stringify(api));
    for (const value of [
      { ...verified, user: { ...verified.user, application_id: "other" } },
      { ...verified, session: { ...verified.session, format: "jwt" } },
      {},
    ]) {
      globalThis.fetch = async () => Response.json(value);
      await assert.rejects(() => api.verify(token), {
        code: "invalid_response",
      });
    }
    globalThis.fetch = async () => new Response("x".repeat(32769));
    await assert.rejects(() => api.verify(token), { code: "invalid_response" });
    globalThis.fetch = async () => {
      throw new Error(token);
    };
    await assert.rejects(
      () => api.verify(token),
      (e) =>
        e instanceof KancheError &&
        !e.message.includes(token) &&
        e.status === 503,
    );
  } finally {
    globalThis.fetch = original;
  }
});
test("adapter enforces CSRF/origin, encrypted host cookie, outage preservation, tamper and key rollout", async () => {
  const original = globalThis.fetch;
  const token = random(),
    old = random(),
    next = random();
  const make = (
    keys: Record<string, string>,
    currentKey: string,
    development = true,
  ) =>
    new KancheAdapter({
      ...config,
      baseUrl: development ? config.baseUrl : "https://kanche.example.invalid",
      origin: development
        ? "http://localhost:3010"
        : "https://demo.example.invalid",
      development,
      cookieKeys: keys,
      currentKey,
    });
  const a = make({ old }, "old");
  const request = (
    op: string,
    body: unknown = {},
    cookie = "",
    csrf = "",
    origin = "http://localhost:3010",
  ) =>
    new Request(`http://localhost:3010/api/kanche/${op}`, {
      method: op === "csrf" ? "GET" : "POST",
      headers: {
        origin,
        cookie,
        "content-type": "application/json",
        "x-csrf-token": csrf,
      },
      body: op === "csrf" ? undefined : JSON.stringify(body),
    });
  try {
    globalThis.fetch = async () =>
      Response.json({
        ...verified,
        session: { ...verified.session, credential: token },
      });
    assert.equal((await a.handle(request("sign-in"), "sign-in")).status, 403);
    const pre = await a.handle(request("csrf"), "csrf");
    const csrf = (await pre.json()).csrf_token;
    const preCookie = pre.headers.getSetCookie()[0].split(";")[0];
    assert.equal(
      (
        await a.handle(
          request("sign-in", {}, preCookie, csrf, "https://hostile.invalid"),
          "sign-in",
        )
      ).status,
      403,
    );
    const logged = await a.handle(
      request(
        "sign-in",
        { email: "u@example.invalid", password: random() },
        preCookie,
        csrf,
      ),
      "sign-in",
    );
    assert.equal(logged.status, 200);
    const body = await logged.text();
    assert.ok(!body.includes(token));
    assert.ok(!body.includes(config.applicationSecret));
    const sessionCookie = logged.headers
      .getSetCookie()
      .find((c) => c.startsWith("kanche_dev_session="))!;
    assert.ok(sessionCookie.includes("HttpOnly"));
    assert.ok(!sessionCookie.includes("Domain="));
    assert.ok(!sessionCookie.includes(token));
    const cookie = sessionCookie.split(";")[0];
    globalThis.fetch = async () => Response.json(verified);
    assert.equal((await a.read(cookie)).authenticated, true);
    assert.equal(
      (await make({ old, next }, "next").read(cookie)).authenticated,
      true,
    );
    assert.equal(
      (await make({ next }, "next").read(cookie)).authenticated,
      false,
    );
    assert.equal((await a.read(cookie + "x")).authenticated, false);
    const outage = async () => {
      throw new Error("upstream");
    };
    globalThis.fetch = outage;
    await assert.rejects(() => a.read(cookie), {
      code: "temporarily_unavailable",
    });
    const signout = await a.handle(
      request("sign-out", {}, `${preCookie}; ${cookie}`, csrf),
      "sign-out",
    );
    assert.equal(signout.status, 503);
    assert.equal(signout.headers.getSetCookie().length, 0);
    globalThis.fetch = async () => new Response(null, { status: 204 });
    const done = await a.handle(
      request("sign-out", {}, `${preCookie}; ${cookie}`, csrf),
      "sign-out",
    );
    assert.equal(done.status, 200);
    assert.ok(done.headers.getSetCookie()[0].includes("1970"));
    const secure = await make({ old }, "old", false).handle(
      new Request("https://demo.example.invalid/api/kanche/csrf"),
      "csrf",
    );
    assert.ok(
      secure.headers.getSetCookie()[0].startsWith("__Host-kanche_csrf="),
    );
    assert.ok(secure.headers.getSetCookie()[0].includes("Secure"));
  } finally {
    globalThis.fetch = original;
  }
});
