export interface LoginInput {
  readonly url: string;
  readonly account: string;
  readonly password: string;
  readonly createAccount: boolean;
  readonly invite: string;
}

export const DEVICE_LABEL = "web browser";

export function normalizeAccount(raw: string): string {
  return raw.toLowerCase();
}
