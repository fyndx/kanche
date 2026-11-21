-- SPDX-License-Identifier: MPL-2.0
CREATE TABLE hosted_callbacks (
 app_id text NOT NULL REFERENCES applications,
 redirect_uri text NOT NULL,
 PRIMARY KEY(app_id, redirect_uri)
);
CREATE TABLE hosted_transactions (
 handle_digest text PRIMARY KEY,
 tenant_id text NOT NULL,
 app_id text NOT NULL,
 redirect_uri text NOT NULL,
 state text NOT NULL,
 code_challenge text NOT NULL,
 expires_at bigint NOT NULL,
 browser_digest text,
 csrf_digest text,
 user_id text,
 code_digest text UNIQUE,
 code_expires_at bigint,
 consumed boolean NOT NULL DEFAULT false,
 invalidated boolean NOT NULL DEFAULT false,
 FOREIGN KEY(tenant_id,app_id) REFERENCES applications(tenant_id,id),
 FOREIGN KEY(tenant_id,user_id) REFERENCES users(tenant_id,id),
 FOREIGN KEY(app_id,redirect_uri) REFERENCES hosted_callbacks(app_id,redirect_uri) ON DELETE CASCADE,
 CHECK((user_id IS NULL AND code_digest IS NULL AND code_expires_at IS NULL)
       OR (user_id IS NOT NULL AND code_digest IS NOT NULL AND code_expires_at IS NOT NULL)),
 CHECK(code_expires_at IS NULL OR code_expires_at <= expires_at)
);
CREATE INDEX hosted_transactions_user ON hosted_transactions(app_id,user_id);
