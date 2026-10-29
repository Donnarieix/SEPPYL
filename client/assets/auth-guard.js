// Intercept authenticated API 401 responses centrally.
// Local JWT flow only; replace this adapter with the Donnarieix SSO client.
(() => {
  if (window.__seppylAuthGuardInstalled) return;
  window.__seppylAuthGuardInstalled = true;
  const rawFetch = window.fetch.bind(window);
  let redirecting = false;
  window.fetch = async (...args) => {
    const response = await rawFetch(...args);
    if (response.status !== 401 || redirecting) return response;
    const [input, init] = args;
    let headers;
    try {
      headers = new Headers(input instanceof Request ? input.headers : undefined);
      new Headers(init?.headers).forEach((value, key) => headers.set(key, value));
    } catch (_) { return response; }
    const authorization = headers.get("authorization");
    // Only invalidate a session for authenticated requests, never a failed login.
    if (!authorization || !/^Bearer\s+/i.test(authorization)) return response;
    redirecting = true;
    try { localStorage.removeItem("seppyl.session.v1"); } catch (_) {}
    window.location.replace("/login");
    return response;
  };
})();
