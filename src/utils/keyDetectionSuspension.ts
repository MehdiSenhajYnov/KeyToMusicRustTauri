import { suspendKeyDetection } from "./tauriCommands";

const reasons = new Set<symbol>();
let pending = Promise.resolve();
function sync() {
  const suspended = reasons.size > 0;
  pending = pending.catch(() => {}).then(() => suspendKeyDetection(suspended));
  void pending.catch(console.error);
}
export function isKeyDetectionSuspended(): boolean {
  return reasons.size > 0;
}
/** Temporary suspension must never change the user's saved preference. */
export function acquireKeyDetectionSuspension(): () => void {
  const reason = Symbol();
  reasons.add(reason);
  sync();
  return () => { if (reasons.delete(reason)) sync(); };
}
