#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Disposable loopback-only PostgreSQL; no populated env files or raw captures."""
import os
import secrets
import subprocess
import time

name = 'kanche-test-' + secrets.token_hex(6)
image = 'postgres:17-alpine@sha256:526f86d93c6ea08fd3f1fe5605650cacc781b2c6013169a0ad8f42c7d3d84f32'
try:
    subprocess.run(['docker', 'run', '--detach', '--rm', '--name', name,
                    '-e', 'POSTGRES_HOST_AUTH_METHOD=trust', '-p', '127.0.0.1::5432', image],
                   check=True, stdout=subprocess.DEVNULL)
    port = subprocess.check_output(['docker', 'port', name, '5432/tcp'], text=True).strip().split(':')[-1]
    for _ in range(60):
        if subprocess.run(['docker', 'exec', name, 'pg_isready', '-h', '127.0.0.1', '-U', 'postgres'],
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
            break
        time.sleep(0.2)
    else:
        raise RuntimeError('Disposable PostgreSQL did not become ready')
    env = dict(os.environ, KANCHE_TEST_DATABASE_URL=f'postgres://postgres@127.0.0.1:{port}/postgres')
    for target in ['identity', 'hosted']:
        subprocess.run(['cargo', 'test', '--locked', '--test', target, '--', '--ignored', '--nocapture'], env=env, check=True)
finally:
    subprocess.run(['docker', 'stop', name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
