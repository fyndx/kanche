// SPDX-License-Identifier: MPL-2.0
export type BrowserSession =
  | { authenticated: false }
  | {
      authenticated: true;
      user_id: string;
      idle_expires_at: number;
      absolute_expires_at: number;
    };
