# SEPPYL — installation initiale du serveur (Debian 13)

This must be performed **once** by the server administrator. The CI cannot provision SSH credentials, DNS, or root services.

1. Ensure `seppyl.donnarieix.fr` resolves to the server and the wildcard certificate is installed.
2. Application production configuration: Create `/etc/seppyl/api.env` (root owned, mode 0600) with a strong `JWT_SECRET`, `SEPPYL_BIND=127.0.0.1:3101`, for application authentication. Public account creation uses the `/api/signup` endpoint; newly registered accounts have the read-only `user` role. Never commit secrets.
3. Create the dedicated account and directories:
   ```sh
   sudo useradd --system --home /var/lib/seppyl --shell /usr/sbin/nologin seppyl
   sudo install -d -o seppyl -g seppyl -m 0750 /var/lib/seppyl/data
   sudo install -d -m 0755 /etc/seppyl /opt/seppyl/api/releases /var/www/seppyl/releases
   sudo install -m 0600 /dev/null /etc/seppyl/api.env
   ```
   Populate `api.env` using a root editor. Existing app SQLite URL uses the relative path `data/sappyl.db` and is therefore persistent under `/var/lib/seppyl/data`.
4. Install `deploy/seppyl-api.service` to `/etc/systemd/system/`; `sudo systemctl daemon-reload` and `sudo systemctl enable seppyl-api`. Do not start until an API release is deployed.
5. Install `deploy/nginx.conf` to `/etc/nginx/sites-available/seppyl`, enable the symlink to `sites-enabled`, and test via `sudo nginx -t`. Do not reload Nginx until a web release exists.
6. Install the two deploy scripts as `/usr/local/bin/deploy-seppyl-api` and `/usr/local/bin/deploy-seppyl-web` with mode 0755 and appropriate deployment directory ownership for the restricted SSH deploy account.
7. Create **separate SSH deployment keys** for API and Web, each limited in `authorized_keys` with `restrict,command="/usr/local/bin/deploy-seppyl-api"` or the web equivalent. Do not grant unrestricted shell access. Allow the API deploy account a **narrow** passwordless sudoers rule for only `systemctl restart seppyl-api.service` and no other commands.
8. Configure GitHub Actions secrets: `DEPLOY_HOST`, `DEPLOY_PORT`, `DEPLOY_KNOWN_HOSTS`, `DEPLOY_API_SSH_KEY`, and `DEPLOY_WEB_SSH_KEY`. Pin SSH host fingerprints obtained through a trusted channel. Set deploy user to the account holding the forced-command keys (workflow defaults to `lud`; adapt if using a dedicated deploy user).
9. Run the workflow from GitHub Actions. Only after a successful deployment, reload Nginx and check `https://seppyl.donnarieix.fr` and the API endpoint.

**Security:** Avoid reusing the Drive deployment key. Keep SQLite backups, protect `api.env`, monitor public account registration and add rate limiting at the proxy if needed, and note that the certificate created with manual DNS challenge still needs automated renewal separately.
