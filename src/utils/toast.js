export function toast(message, type = 'info') {
  const text = String(message ?? '');
  if (typeof window === 'undefined') return;
  window.dispatchEvent(new CustomEvent('hyperion:toast', { detail: { message: text, type } }));
}
