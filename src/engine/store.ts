// The history: transcripts and their metadata in IndexedDB, on this device
// only. The media is never stored; it stays in memory for the session, and a
// transcript opened later asks for its file again to play it.

import { useSyncExternalStore } from "react";
import type { Doc } from "./types.ts";

const DB = "verbatim";
const STORE = "docs";

let db: Promise<IDBDatabase> | null = null;
function open(): Promise<IDBDatabase> {
  return (db ??= new Promise((resolve, reject) => {
    const req = indexedDB.open(DB, 1);
    req.onupgradeneeded = () => req.result.createObjectStore(STORE, { keyPath: "id" });
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  }));
}

function tx<T>(mode: IDBTransactionMode, run: (s: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  return open().then(
    (d) =>
      new Promise<T>((resolve, reject) => {
        const req = run(d.transaction(STORE, mode).objectStore(STORE));
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
      }),
  );
}

let docs: Doc[] = [];
let loaded = false;
const listeners = new Set<() => void>();
const emit = () => {
  for (const fn of listeners) fn();
};

export async function loadDocs(): Promise<Doc[]> {
  if (!loaded) {
    docs = ((await tx("readonly", (s) => s.getAll())) as Doc[]).sort((a, b) => b.updatedAt - a.updatedAt);
    loaded = true;
    emit();
  }
  return docs;
}

export async function saveDoc(doc: Doc): Promise<void> {
  const next = { ...doc, updatedAt: Date.now() };
  // Memory first, so an update made before the write lands builds on this one.
  docs = [next, ...docs.filter((d) => d.id !== doc.id)];
  emit();
  await tx("readwrite", (s) => s.put(next));
}

export async function deleteDoc(id: string): Promise<void> {
  await tx("readwrite", (s) => s.delete(id));
  docs = docs.filter((d) => d.id !== id);
  media.delete(id);
  emit();
}

export async function clearDocs(): Promise<void> {
  await tx("readwrite", (s) => s.clear());
  docs = [];
  media.clear();
  emit();
}

export function useDocs(): Doc[] {
  return useSyncExternalStore(
    (fn) => {
      listeners.add(fn);
      void loadDocs();
      return () => listeners.delete(fn);
    },
    () => docs,
  );
}

export const getDoc = (id: string) => docs.find((d) => d.id === id);

/** This session's media, by document id. */
export const media = new Map<string, File>();
