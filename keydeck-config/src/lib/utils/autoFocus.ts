// SPDX-License-Identifier: AGPL-3.0-or-later
// Simple action to focus an element once it's mounted
export function autoFocus(node: HTMLElement) {
  const frame = requestAnimationFrame(() => {
    node.focus();
  });

  return {
    destroy() {
      cancelAnimationFrame(frame);
    }
  };
}
