// localStorage belongs to the origin, and maxgfr.github.io is shared by every
// project page: another site filling the quota makes our writes throw. Small
// preferences go through here so neither a read nor a write can break a screen.

export function readLocal(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

/** False when the value could not be stored (full, blocked or missing): it is gone after a reload. */
export function writeLocal(key: string, value: string): boolean {
  try {
    localStorage.setItem(key, value);
    return true;
  } catch {
    return false;
  }
}
