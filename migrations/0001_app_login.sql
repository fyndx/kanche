-- SPDX-License-Identifier: MPL-2.0
CREATE TABLE tenants (id text PRIMARY KEY, active boolean NOT NULL DEFAULT true);
CREATE TABLE applications (id text PRIMARY KEY, tenant_id text NOT NULL REFERENCES tenants,
 active boolean NOT NULL DEFAULT true, UNIQUE(tenant_id,id));
CREATE TABLE application_keys (digest text PRIMARY KEY, app_id text NOT NULL REFERENCES applications);
CREATE TABLE users (id text PRIMARY KEY, tenant_id text NOT NULL REFERENCES tenants,
 email text NOT NULL, password_hash text NOT NULL, active boolean NOT NULL DEFAULT true,
 verified boolean NOT NULL DEFAULT false, UNIQUE(tenant_id,email), UNIQUE(tenant_id,id));
CREATE TABLE enrollments (tenant_id text NOT NULL, app_id text NOT NULL, user_id text NOT NULL,
 active boolean NOT NULL DEFAULT true, PRIMARY KEY(app_id,user_id),
 FOREIGN KEY(tenant_id,app_id) REFERENCES applications(tenant_id,id),
 FOREIGN KEY(tenant_id,user_id) REFERENCES users(tenant_id,id));
CREATE TABLE sessions (digest text PRIMARY KEY, tenant_id text NOT NULL, app_id text NOT NULL,
 user_id text NOT NULL, idle_expires_at bigint NOT NULL, absolute_expires_at bigint NOT NULL,
 revoked boolean NOT NULL DEFAULT false, CHECK(idle_expires_at <= absolute_expires_at),
 FOREIGN KEY(tenant_id,app_id) REFERENCES applications(tenant_id,id),
 FOREIGN KEY(tenant_id,user_id) REFERENCES users(tenant_id,id));
CREATE INDEX sessions_app_user ON sessions(app_id,user_id);
CREATE TABLE abuse_buckets (bucket text PRIMARY KEY, expires_at bigint NOT NULL, attempts integer NOT NULL);
