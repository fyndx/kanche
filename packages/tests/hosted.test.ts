// SPDX-License-Identifier: MPL-2.0
import test from "node:test";
import assert from "node:assert/strict";
import { createHash, randomBytes } from "node:crypto";
import { Kanche, KancheError, Grant } from "../server/index.ts";
import { KancheAdapter } from "../nextjs/server.ts";
const random = () => randomBytes(32).toString("hex");
const now = () => Math.floor(Date.now() / 1000);
const config = {
  baseUrl: "http://127.0.0.1:18080",
  applicationSecret: random(),
  tenantId: "t",
  applicationId: "a",
  development: true,
};
const callbackUri = "http://localhost:3010/api/kanche/callback";
const issued = (credential: string) => ({
  user: { id: "u", tenant_id: "t", application_id: "a" },
  session: {
    format: "opaque",
    credential,
    idle_expires_at: now() + 86400,
    absolute_expires_at: now() + 604800,
  },
});
test("hosted SDK validates redirect destination and grant scope without retries or unsafe errors", async () => {
  const original = globalThis.fetch,
    api = new Kanche(config),
    state = random(),
    verifier = random(),
    challenge = createHash("sha256").update(verifier).digest("base64url"),
    handle = random(),
    code = random(),
    token = random();
  try {
    let calls = 0;
    globalThis.fetch = async (input, init) => {
      calls++;
      assert.ok(init?.redirect === "error");
      const body = JSON.parse(String(init?.body));
      if (new URL(String(input)).pathname === "/v1/hosted/transactions") {
        assert.ok(
          body.state === state &&
            body.code_challenge === challenge &&
            body.code_challenge_method === "S256" &&
            body.redirect_uri === callbackUri,
        );
        return Response.json({
          authorization_url:
            config.baseUrl + "/hosted/login?transaction=" + handle,
          expires_at: now() + 300,
        });
      }
      assert.ok(
        body.code === code &&
          body.code_verifier === verifier &&
          body.redirect_uri === callbackUri,
      );
      return Response.json(issued(token));
    };
    assert.ok(
      (
        await api.startHosted(callbackUri, state, challenge)
      ).authorization_url.endsWith(handle),
    );
    const grant = await api.exchangeHosted(code, verifier, callbackUri);
    assert.ok(grant instanceof Grant);
    assert.throws(() => JSON.stringify(grant));
    assert.ok(calls === 2);
    for (const url of [
      "https://hostile.invalid/hosted/login?transaction=" + handle,
      config.baseUrl + "/hosted/login?transaction=" + handle + "&other=1",
      config.baseUrl +
        "/hosted/login?transaction=%" +
        handle.slice(0, 2) +
        handle.slice(2),
      config.baseUrl + "/other?transaction=" + handle,
    ]) {
      globalThis.fetch = async () =>
        Response.json({ authorization_url: url, expires_at: now() + 300 });
      await assert.rejects(
        () => api.startHosted(callbackUri, state, challenge),
        { code: "invalid_response" },
      );
    }
    globalThis.fetch = async () =>
      Response.json({
        ...issued(token),
        user: { id: "u", tenant_id: "other", application_id: "a" },
      });
    await assert.rejects(
      () => api.exchangeHosted(code, verifier, callbackUri),
      { code: "invalid_response" },
    );
    let failures = 0;
    globalThis.fetch = async () => {
      failures++;
      throw new Error(token);
    };
    await assert.rejects(
      () => api.exchangeHosted(code, verifier, callbackUri),
      (e) =>
        e instanceof KancheError &&
        e.code === "temporarily_unavailable" &&
        !e.message.includes(token),
    );
    assert.ok(failures === 1);
    globalThis.fetch = async () =>
      Response.json({ code: "invalid_handoff" }, { status: 400 });
    await assert.rejects(
      () => api.exchangeHosted(code, verifier, callbackUri),
      { code: "invalid_handoff", status: 400 },
    );
  } finally {
    globalThis.fetch = original;
  }
});
test("hosted adapter binds state and PKCE to encrypted cookie, preserves failures and only persists a validated exchange", async () => {
  const original = globalThis.fetch,
    key = random(),
    next = random(),
    token = random(),
    handle = random(),
    code = random();
  const adapter = (
    keys: Record<string, string> = { old: key },
    currentKey = "old",
    callback = callbackUri,
  ) =>
    new KancheAdapter({
      ...config,
      origin: "http://localhost:3010",
      cookieKeys: keys,
      currentKey,
      hostedCallbackUri: callback,
    });
  const app = adapter();
  let state = "",
    challenge = "",
    exchanges = 0,
    redeemed = false;
  const mock = async (input: RequestInfo | URL, init?: RequestInit) => {
    const body = JSON.parse(String(init?.body));
    if (new URL(String(input)).pathname === "/v1/hosted/transactions") {
      state = body.state;
      challenge = body.code_challenge;
      return Response.json({
        authorization_url:
          config.baseUrl + "/hosted/login?transaction=" + handle,
        expires_at: now() + 300,
      });
    }
    exchanges++;
    assert.ok(
      createHash("sha256").update(body.code_verifier).digest("base64url") ===
        challenge,
    );
    if (redeemed)
      return Response.json({ code: "invalid_handoff" }, { status: 400 });
    redeemed = true;
    return Response.json(issued(token));
  };
  const request = (
    op: string,
    body: unknown = {},
    cookie = "",
    csrf = "",
    origin = "http://localhost:3010",
  ) =>
    new Request("http://localhost:3010/api/kanche/" + op, {
      method: op === "csrf" ? "GET" : "POST",
      headers: {
        origin,
        cookie,
        "content-type": "application/json",
        "x-csrf-token": csrf,
      },
      body: op === "csrf" ? undefined : JSON.stringify(body),
    });
  const cb = (
    stateValue = state,
    cookie = "",
    suffix = "",
    origin = "http://localhost:3010",
  ) =>
    new Request(
      origin +
        "/api/kanche/callback?code=" +
        code +
        "&state=" +
        stateValue +
        suffix,
      { headers: { cookie } },
    );
  try {
    assert.throws(
      () =>
        adapter(
          undefined,
          undefined,
          "https://hostile.invalid/api/kanche/callback",
        ),
      { code: "invalid_hosted_callback" },
    );
    globalThis.fetch = mock;
    assert.ok(
      (await app.handle(request("hosted-start"), "hosted-start")).status ===
        403,
    );
    const pre = await app.handle(request("csrf"), "csrf"),
      csrf = (await pre.json()).csrf_token,
      preCookie = pre.headers.getSetCookie()[0].split(";")[0];
    assert.ok(
      (
        await app.handle(
          request(
            "hosted-start",
            {},
            preCookie,
            csrf,
            "https://hostile.invalid",
          ),
          "hosted-start",
        )
      ).status === 403,
    );
    const start = await app.handle(
      request("hosted-start", {}, preCookie, csrf),
      "hosted-start",
    );
    assert.ok(start.status === 200);
    const text = await start.text();
    assert.ok(
      !text.includes(state) && !text.includes(config.applicationSecret),
    );
    const pendingCookie = start.headers.getSetCookie()[0];
    assert.ok(
      pendingCookie.includes("HttpOnly") &&
        pendingCookie.includes("SameSite=Lax") &&
        !pendingCookie.includes("Domain="),
    );
    assert.ok(!pendingCookie.includes(state));
    const cookie = pendingCookie.split(";")[0];
    for (const invalid of [
      cb(random(), cookie),
      cb(state, ""),
      cb(state, cookie, "&code=" + random()),
      cb(state, cookie, "&extra=1"),
      cb(state, cookie, "", "https://hostile.invalid"),
      cb(state, cookie + "x"),
    ]) {
      const r = await app.handle(invalid, "callback");
      assert.ok(r.status === 400 && r.headers.getSetCookie().length === 0);
    }
    assert.ok(exchanges === 0);
    const clock = Date.now;
    try {
      Date.now = () => clock() + 301000;
      assert.ok(
        (await app.handle(cb(state, cookie), "callback")).status === 400,
      );
    } finally {
      Date.now = clock;
    }
    assert.ok(exchanges === 0);
    const removedKey = adapter({ old: next });
    assert.ok(
      (await removedKey.handle(cb(state, cookie), "callback")).status === 400,
    );
    const pendingState = state;
    await app.handle(
      request("hosted-start", {}, preCookie, csrf),
      "hosted-start",
    );
    const newestState = state;
    assert.ok(newestState !== pendingState);
    // The actual browser overwrites one pending cookie; an older callback does not match its latest cookie.
    const newest = await app.handle(
        request("hosted-start", {}, preCookie, csrf),
        "hosted-start",
      ),
      newCookie = newest.headers.getSetCookie()[0].split(";")[0];
    assert.ok(
      (await app.handle(cb(pendingState, newCookie), "callback")).status ===
        400,
    );
    globalThis.fetch = async () => {
      throw new Error(token);
    };
    const unavailable = await app.handle(cb(state, newCookie), "callback");
    assert.ok(
      unavailable.status === 503 &&
        unavailable.headers.getSetCookie().length === 0,
    );
    assert.ok((await unavailable.text()).includes("temporarily_unavailable"));
    globalThis.fetch = mock;
    const rolled = adapter({ old: key, new: next }, "new");
    const success = await rolled.handle(cb(state, newCookie), "callback");
    assert.ok(
      success.status === 303 &&
        success.headers.get("location") === "/protected",
    );
    assert.ok(success.headers.get("referrer-policy") === "no-referrer");
    assert.ok(
      success.headers
        .get("content-security-policy")
        ?.includes("frame-ancestors 'none'"),
    );
    assert.ok(!(await success.text()).includes(token));
    const set = success.headers.getSetCookie();
    assert.ok(
      set.some((c) => c.startsWith("kanche_dev_hosted=") && c.includes("1970")),
    );
    assert.ok(set.some((c) => c.startsWith("kanche_dev_session=new.")));
    assert.ok(set.every((c) => !c.includes(token)));
    // Browser replay after pending-cookie clearing stops before the upstream request.
    const before = exchanges;
    assert.ok(
      (await rolled.handle(cb(state, ""), "callback")).status === 400 &&
        exchanges === before,
    );
    // A copied old pending cookie still cannot redeem a consumed upstream code twice.
    const replay = await rolled.handle(cb(state, newCookie), "callback");
    assert.ok(
      replay.status === 400 && replay.headers.getSetCookie().length === 0,
    );
    assert.ok((await replay.text()).includes("invalid_handoff"));
  } finally {
    globalThis.fetch = original;
  }
});
