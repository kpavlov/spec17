// Theme: system | light | dark, remembered per browser. data-theme on <html> drives tokens.css (the OS preference for system). Runs in <head> so the page does not flash.
{ // a block, so these names do not collide with search.js
  const key = 'spec17.theme';
  // the dashboard's icons: copies of system, sun and moon in web/src/lib/ui/Icon.svelte. Change them there, then here.
  const themes = [
    ['system', '<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>'],
    ['light', '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>'],
    ['dark', '<path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8Z"/>'],
  ];
  let i = 0;
  try { i = Math.max(0, themes.findIndex(t => t[0] === localStorage.getItem(key))); } catch { /* storage blocked */ }
  const apply = () => {
    i ? document.documentElement.dataset.theme = themes[i][0] : document.documentElement.removeAttribute('data-theme'); // tokens.css
    const btn = document.getElementById('theme');
    if (!btn) return;
    const [name, icon] = themes[i], next = themes[(i + 1) % themes.length][0];
    btn.innerHTML = `<svg viewBox="0 0 24 24" width="1.25em" height="1.25em" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${icon}</svg>`;
    btn.title = `Theme: ${name} (click for ${next})`;
    btn.setAttribute('aria-label', `Theme: ${name}, switch to ${next}`);
  };
  apply();
  addEventListener('DOMContentLoaded', () => {
    const btn = document.getElementById('theme');
    btn.hidden = false;
    btn.onclick = () => {
      i = (i + 1) % themes.length;
      try { i ? localStorage.setItem(key, themes[i][0]) : localStorage.removeItem(key); } catch { /* storage blocked */ }
      apply();
    };
    apply();
  });
}
