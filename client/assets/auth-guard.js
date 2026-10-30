// Client-side navigation guard for the current JWT flow.
// Authorization is still enforced on the API; this is only UX.
(() => {
  if (window.__seppylAuthGuardInstalled) return;
  window.__seppylAuthGuardInstalled = true;
  const key = "seppyl.session.v1";
  const publicPath = (path) => ["/login", "/public", "/mentions-legales",
    "/confidentialite", "/cgu"].includes(path);
  const readToken = () => {
    try { return localStorage.getItem?.(key) || ""; } catch (_) { return ""; }
  };
  const expired = (token) => {
    // A non-JWT token (e.g. during a future SSO migration) is checked by
    // the server on the first request rather than being rejected here.
    try {
      const body=token.split(".")[1];
      if(!body) return false;
      const payload=JSON.parse(atob(body.replace(/-/g,"+").replace(/_/g,"/")));
      return Number.isFinite(payload.exp) && payload.exp*1000 <= Date.now()+1000;
    } catch (_) { return false; }
  };
  let redirecting = false;
  const invalidate = () => {
    if (redirecting) return;
    redirecting = true;
    try { localStorage.removeItem(key); } catch (_) {}
    window.location.replace("/login");
  };
  const guardRoute = () => {
    const path=window.location?.pathname;
    if (typeof path !== "string" || publicPath(path)) return;
    const token=readToken();
    if (!token || expired(token)) invalidate();
  };
  // A hard refresh or direct link to a protected page must never show a
  // transient "Session expirée" panel in place of the login screen.
  guardRoute();
  window.addEventListener?.("popstate",guardRoute);

  const rawFetch = window.fetch.bind(window);
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
    if (!authorization || !/^Bearer\s+/i.test(authorization)) return response;
    const sentToken=authorization.replace(/^Bearer\s+/i,"").trim();
    const current=readToken();
    // A delayed response from an old account cannot log out a newer session.
    if(current && current!==sentToken) return response;
    invalidate();
    return response;
  };
})();