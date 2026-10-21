export function isTextInput(el: EventTarget | null): boolean {
  if (el instanceof HTMLInputElement) {
    const type = el.type;
    return type !== "range" && type !== "checkbox";
  }
  if (el instanceof HTMLElement && el.isContentEditable) return true;
  return el instanceof HTMLTextAreaElement || el instanceof HTMLSelectElement;
}
