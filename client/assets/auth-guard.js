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

// Authenticated live messaging is initialized with the session guard.
/* SEPPYL ticket chat: authenticated SSE, typing and private image transport.
   Tokens never appear in URLs or DOM attributes. Markdown cannot insert raw HTML. */
(() => {
  if (window.__seppylTicketChatInstalled) return;
  window.__seppylTicketChatInstalled = true;
  const readToken = () => {
    try {
      // gloo-storage JSON-encodes Strings; localStorage returns the quoted JSON.
      const stored = localStorage.getItem("seppyl.session.v1") || "";
      if (!stored) return "";
      try { const decoded = JSON.parse(stored); if (typeof decoded === "string") return decoded; }
      catch {}
      return stored;
    } catch { return ""; }
  };
  const connections = new WeakMap(), rendered = new WeakMap(), loaded = new WeakMap(), timers = new WeakMap();
  const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
  const esc = value => String(value).replace(/&/g, "&amp;").replace(/</g, "&lt;")
    .replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#39;");
  function inline(raw) {
    let value = esc(raw);
    value = value.replace(/\x60([^\x60\n]+)\x60/g, (_, v) => "<code>" + v + "</code>");
    value = value.replace(/\[([^\]\n]{1,120})\]\((https?:\/\/[^\s)<]{1,1000})\)/g,
      (_, label, url) => '<a href="' + url.replace(/&amp;/g, "&") +
        '" target="_blank" rel="noopener noreferrer nofollow">' + label + "</a>");
    return value.replace(/\*\*([^*\n]+)\*\*/g, "<strong>$1</strong>")
      .replace(/~~([^~\n]+)~~/g, "<del>$1</del>")
      .replace(/(?<!\*)\*([^*\n]+)\*(?!\*)/g, "<em>$1</em>");
  }
  function markdown(raw) {
    const lines = String(raw || "").replace(/\r\n?/g, "\n").split("\n");
    const out = [];
    let code = false, list = false, quote = false;
    function close() {
      if (list) out.push("</ul>");
      if (quote) out.push("</blockquote>");
      list = quote = false;
    }
    for (const line of lines) {
      if (/^\s*\x60{3}/.test(line)) {
        close(); out.push(code ? "</code></pre>" : "<pre><code>"); code = !code; continue;
      }
      if (code) { out.push(esc(line) + "\n"); continue; }
      const bullet = /^\s*[-*]\s+(.+)$/.exec(line), quoted = /^\s*>\s?(.*)$/.exec(line);
      if (bullet) {
        if (quote) { out.push("</blockquote>"); quote = false; }
        if (!list) { out.push("<ul>"); list = true; }
        out.push("<li>" + inline(bullet[1]) + "</li>");
      } else if (quoted) {
        if (list) { out.push("</ul>"); list = false; }
        if (!quote) { out.push("<blockquote>"); quote = true; }
        out.push("<p>" + inline(quoted[1]) + "</p>");
      } else {
        close(); out.push(line.trim() ? "<p>" + inline(line) + "</p>" : '<div class="chat-markdown-break"></div>');
      }
    }
    close(); if (code) out.push("</code></pre>");
    return out.join("");
  }
  function push(root, payload) {
    const input = root.querySelector("[data-ticket-live-event]");
    if (!input || !root.isConnected) return;
    input.value = JSON.stringify(payload);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }
  function status(root, title, offline = false) {
    const target = root.querySelector("[data-ticket-live-status]");
    if (target) { target.textContent = title; target.classList.toggle("offline", offline); }
  }
  function decorate(root) {
    root.querySelectorAll("[data-ticket-markdown]").forEach(node => {
      const raw = node.getAttribute("data-ticket-raw") || "";
      if (rendered.get(node) !== raw) {
        rendered.set(node, raw);
        node.innerHTML = markdown(raw);
      }
    });
    root.querySelectorAll("img[data-ticket-image]").forEach(async node => {
      if (loaded.has(node)) return;
      const path = node.getAttribute("data-ticket-image"), base = root.getAttribute("data-ticket-base");
      const token = readToken();
      if (!path || !base || !token) return;
      loaded.set(node, null);
      try {
        const reply = await fetch(base + path, { headers: { Authorization: "Bearer " + token }, cache: "no-store" });
        if (!reply.ok) throw new Error("Image indisponible");
        const blob = await reply.blob();
        if (!["image/png", "image/jpeg", "image/webp", "image/gif"].includes(blob.type)) throw new Error("Format non valide");
        const url = URL.createObjectURL(blob);
        if (!node.isConnected) { URL.revokeObjectURL(url); return; }
        loaded.set(node, url);
        node.src = url;
        node.classList.add("loaded");
      } catch {
        node.alt = "Image indisponible";
        loaded.delete(node);
      }
    });
    const feed = root.querySelector(".ticket-chat-feed");
    if (feed && !feed.dataset.initialized) {
      feed.dataset.initialized = "true";
      feed.scrollTop = feed.scrollHeight;
    }
  }
  async function sendTyping(root, typing) {
    const token = readToken(), base = root.getAttribute("data-ticket-base"), id = root.getAttribute("data-ticket-id");
    if (!root.isConnected || !token || !base || !id) return;
    try {
      await fetch(base + "/tickets/" + id + "/typing", {
        method: "POST", headers: { Authorization: "Bearer " + token, "Content-Type": "application/json" },
        body: JSON.stringify({ typing: !!typing }), keepalive: !typing
      });
    } catch {}
  }
  function onTyping(root) {
    if (Date.now() - (root.__lastTyping || 0) > 1500) {
      root.__lastTyping = Date.now(); sendTyping(root, true);
    }
    clearTimeout(timers.get(root));
    timers.set(root, setTimeout(() => sendTyping(root, false), 3000));
  }
  async function stream(root, controller) {
    const base = root.getAttribute("data-ticket-base"), id = root.getAttribute("data-ticket-id");
    const active = () => root.isConnected && !controller.signal.aborted && !!readToken();
    while (active()) {
      try {
        const reply = await fetch(base + "/tickets/" + id + "/events", {
          headers: { Authorization: "Bearer " + readToken(), Accept: "text/event-stream" },
          signal: controller.signal, cache: "no-store"
        });
        if (!reply.ok || !reply.body) throw new Error("Connexion perdue");
        status(root, "En direct");
        const reader = reply.body.getReader(), decoder = new TextDecoder();
        let buffer = "", type = "", data = [];
        while (active()) {
          const { done, value } = await reader.read();
          if (done) break;
          buffer += decoder.decode(value, { stream: true });
          let end;
          while ((end = buffer.indexOf("\n")) !== -1) {
            const line = buffer.slice(0, end).replace(/\r$/, "");
            buffer = buffer.slice(end + 1);
            if (!line) {
              if (type === "ticket" && data.length) {
                try {
                  const event = JSON.parse(data.join("\n"));
                  push(root, event);
                  if (event.kind === "typing") {
                    clearTimeout(root.__peerTypingTimeout);
                    root.__peerTypingTimeout = setTimeout(() =>
                      push(root, { kind: "typing_end", actor: event.actor }), 4500);
                  }
                } catch {}
              }
              type = ""; data = [];
            } else if (line.startsWith("event:")) type = line.slice(6).trim();
            else if (line.startsWith("data:")) data.push(line.slice(5).trimStart());
          }
        }
      } catch {
        if (!active()) break;
        status(root, "Reconnexion…", true);
      }
      if (active()) await sleep(900);
    }
  }
  async function upload(root, file) {
    const statusNode = root.querySelector("[data-chat-upload-feedback]");
    const base = root.getAttribute("data-ticket-base"), id = root.getAttribute("data-ticket-id");
    const token = readToken();
    if (!token || !base || !id) return;
    if (file.size > 4 * 1024 * 1024 || !["image/png", "image/jpeg", "image/webp", "image/gif"].includes(file.type)) {
      if (statusNode) statusNode.textContent = "Image PNG, JPEG, WebP ou GIF (4 Mo maximum).";
      return;
    }
    if (statusNode) statusNode.textContent = "Envoi de l'image…";
    try {
      const reply = await fetch(base + "/tickets/" + id + "/images", {
        method: "POST", headers: { Authorization: "Bearer " + token, "Content-Type": file.type }, body: file
      });
      if (!reply.ok) throw new Error("Envoi impossible");
      if (statusNode) statusNode.textContent = "Image envoyée.";
      push(root, { kind: "message" });
    } catch {
      if (statusNode) statusNode.textContent = "Échec de l'envoi. Réessayez.";
    }
  }
  document.addEventListener("input", event => {
    const root = event.target.closest?.(".ticket-chat");
    if (root && event.target.matches("[data-ticket-composer]")) onTyping(root);
  });
  document.addEventListener("focusout", event => {
    const root = event.target.closest?.(".ticket-chat");
    if (root && event.target.matches("[data-ticket-composer]")) {
      clearTimeout(timers.get(root)); sendTyping(root, false);
    }
  });
  document.addEventListener("change", event => {
    const root = event.target.closest?.(".ticket-chat");
    if (root && event.target.matches("[data-ticket-attach-input]")) {
      const file = event.target.files?.[0];
      if (file) upload(root, file);
      event.target.value = "";
    }
  });
  const observer = new MutationObserver(() => {
    document.querySelectorAll(".ticket-chat[data-ticket-id]").forEach(root => {
      if (!connections.has(root)) {
        const controller = new AbortController();
        connections.set(root, controller);
        stream(root, controller);
      }
      decorate(root);
    });
  });
  observer.observe(document.documentElement, { childList: true, subtree: true });
  document.querySelectorAll(".ticket-chat[data-ticket-id]").forEach(root => {
    const controller = new AbortController();
    connections.set(root, controller);
    stream(root, controller);
    decorate(root);
  });
})();
