// (NFR-19, NFR-20) Consent for analytics. Nothing is requested from Google until the visitor accepts.
{ // a block, so these names do not collide with the other scripts
  const key = 'spec17.consent', maxAge = 365 * 24 * 60 * 60 * 1000;
  const id = document.currentScript.dataset.gtm;
  const banner = document.getElementById('consent'), settings = document.getElementById('cookie-settings');

  window.dataLayer = window.dataLayer || [];
  function gtag() { dataLayer.push(arguments); } // gtag.js reads the arguments object, not an array
  gtag('consent', 'default', { analytics_storage: 'denied', ad_storage: 'denied', ad_user_data: 'denied', ad_personalization: 'denied' });

  const read = () => { // 'granted', 'denied', or undefined: ask
    // sessionStorage first: it holds a rejection only when localStorage could not be written
    for (const store of ['sessionStorage', 'localStorage']) {
      try {
        const saved = JSON.parse(window[store].getItem(key));
        if (saved && Date.now() - saved.at < maxAge) return saved.choice;
      } catch { /* storage blocked or a damaged value: try the next, else ask */ }
    }
  };

  // tells the other tabs when storage cannot: no `storage` event fires if there was no record to remove
  const channel = typeof BroadcastChannel === 'function' ? new BroadcastChannel(key) : null; // old browsers lack it
  const write = choice => {
    const record = JSON.stringify({ choice, at: Date.now() });
    try { sessionStorage.removeItem(key); } catch { /* storage blocked */ }
    try { localStorage.setItem(key, record); } catch {
      // an older 'granted' record must not outlive this choice
      try { localStorage.removeItem(key); } catch { /* storage blocked */ }
      if (choice !== 'denied') return; // an unsaved Accept lasts for this page only
      try { sessionStorage.setItem(key, record); } catch { /* storage blocked: the rejection lasts for this page */ }
      channel?.postMessage(choice);
    }
  };

  let loaded = false;
  const load = () => {
    if (loaded) return;
    loaded = true;
    gtag('consent', 'update', { analytics_storage: 'granted' });
    // Hugo's Google Analytics snippet; a script cloned out of a template may not run, so each is built anew
    for (const held of document.getElementById('analytics').content.querySelectorAll('script')) {
      const script = document.createElement('script');
      for (const { name, value } of held.attributes) script.setAttribute(name, value);
      script.textContent = held.textContent;
      document.head.append(script);
    }
    if (!id) return;
    dataLayer.push({ 'gtm.start': Date.now(), event: 'gtm.js' });
    document.head.append(Object.assign(document.createElement('script'), { async: true, src: `https://www.googletagmanager.com/gtm.js?id=${encodeURIComponent(id)}` }));
  };

  // GA sets its cookies on the widest domain it can, so expire them on the host and each parent domain
  const forget = () => {
    const parts = location.hostname.split('.');
    const domains = ['', ...parts.map((_, i) => `;domain=${parts.slice(i).join('.')}`)];
    for (const name of document.cookie.split(';').map(c => c.split('=')[0].trim()).filter(n => n === '_ga' || n.startsWith('_ga_'))) {
      for (const domain of domains) document.cookie = `${name}=;max-age=0;path=/${domain}`;
    }
  };

  // GTM cannot be unloaded: drop the cookies and start the page again without it
  const stop = () => {
    gtag('consent', 'update', { analytics_storage: 'denied' });
    forget();
    location.reload();
  };

  const choice = read();
  if (choice === 'granted') load();
  else forget();
  banner.hidden = choice !== undefined;

  for (const button of banner.querySelectorAll('[data-consent]')) {
    button.onclick = () => {
      const next = button.dataset.consent;
      write(next);
      banner.hidden = true;
      if (next === 'granted') load();
      else if (loaded) stop();
    };
  }
  settings.hidden = false;
  settings.onclick = () => { banner.hidden = false; banner.querySelector('button').focus(); };
  // another tab withdrew consent
  addEventListener('storage', e => { if (e.key === key && loaded && read() !== 'granted') stop(); });
  if (channel) channel.onmessage = () => { if (loaded) stop(); };
}
