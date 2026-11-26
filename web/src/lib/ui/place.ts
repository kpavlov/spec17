// put a fixed popover under its trigger, or above it when there is no room below, and keep it on screen
export function place(tip: HTMLElement, trigger: Element) {
  const box = trigger.getBoundingClientRect();
  tip.style.left = Math.max(8, Math.min(box.left, innerWidth - tip.offsetWidth - 8)) + "px";
  tip.style.top = (box.bottom + tip.offsetHeight + 4 <= innerHeight ? box.bottom : Math.max(4, box.top - tip.offsetHeight)) + "px";
}
