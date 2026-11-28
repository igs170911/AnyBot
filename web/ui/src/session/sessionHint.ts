export const SESSION_HINT_KEY = "anybot.session";

const SESSION_HINT_VALUE = "1";

export function readSessionHint(): boolean {
  try {
    return globalThis.localStorage.getItem(SESSION_HINT_KEY) === SESSION_HINT_VALUE;
  } catch {
    return false;
  }
}

export function writeSessionHint(signedIn: boolean): void {
  try {
    if (signedIn) {
      globalThis.localStorage.setItem(SESSION_HINT_KEY, SESSION_HINT_VALUE);
    } else {
      globalThis.localStorage.removeItem(SESSION_HINT_KEY);
    }
  } catch {
    return;
  }
}
