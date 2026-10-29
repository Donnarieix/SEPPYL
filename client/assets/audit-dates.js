// Adapt UTC fallbacks to the viewer's local time zone without changing audit data.
(() => {
  if (window.__seppylAuditDatesReady) return;
  window.__seppylAuditDatesReady = true;
  const formatter = new Intl.DateTimeFormat("fr-FR", {
    year: "numeric", month: "2-digit", day: "2-digit",
    hour: "2-digit", minute: "2-digit"
  });
  const render = () => {
    for (const node of document.querySelectorAll(".audit-datetime[data-audit-epoch]")) {
      if (node.dataset.auditFormatted === node.dataset.auditEpoch) continue;
      const seconds = Number(node.dataset.auditEpoch);
      if (!Number.isSafeInteger(seconds)) continue;
      const date = new Date(seconds * 1000);
      if (Number.isNaN(date.getTime())) continue;
      node.textContent = formatter.format(date);
      node.dateTime = date.toISOString();
      node.title = "Heure locale — " + Intl.DateTimeFormat().resolvedOptions().timeZone;
      node.dataset.auditFormatted = node.dataset.auditEpoch;
    }
  };
  render();
  new MutationObserver(render).observe(document.documentElement, {
    childList: true, subtree: true
  });
})();
