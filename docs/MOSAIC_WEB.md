# Open Mosaic Web Oversight

Open Mosaic inherits Zellij's optional web client. The native Mosaic addition is
`mosaic web link`, a small JSON helper for agent oversight tools that need
bookmarkable session links and an explicit watch/control distinction.

The helper does not start a web server, create tokens, or verify that a session
exists. It only describes a safe link target for a running local or proxied
web server.

## Watch Links

```sh
zellij web --create-read-only-token observer
zellij web --start
mosaic web link --session work --mode watch --token-name observer
```

Watch mode is the default. It requires a read-only web login token and maps to
Zellij-derived watcher clients. Watchers cannot create new sessions and their
terminal input is ignored by the server except for local watcher exit keys.

The generated URL is bookmarkable:

```json
{
  "schema_version": "mosaic.control.v1",
  "event": "web.link",
  "web_schema_version": "mosaic.web.v1",
  "mode": "watch",
  "session": "work",
  "url": "http://127.0.0.1:8082/work",
  "read_only_required": true,
  "watcher": true,
  "control_allowed": false,
  "auth": {
    "requires_login_cookie": true,
    "link_contains_token": false,
    "token_delivery": "out_of_band",
    "recommended_token_type": "read_only"
  }
}
```

The link never contains the raw token. Give the token to the browser login flow
out of band.

## Control Links

```sh
zellij web --create-token operator
mosaic web link --session work --mode control --token-name operator
```

Control mode describes a normal web client. It can forward input and, if the
server allows it, create a missing session. Use it only for trusted operators.
Oversight dashboards should prefer watch mode unless active control is
intended.

## Secure Defaults

`mosaic web link` defaults to `http://127.0.0.1:8082` and accepts only `http`
or `https` base URLs. It rejects credentials, query strings, and fragments in
`--base-url` so secrets are not baked into links or logs.

Use `--redact` when returning links through shared logs:

```sh
mosaic web link --session work --base-url https://mosaic.example.test/base/ --redact
```

For reverse proxies with a path prefix, pass the prefix as the base URL:

```sh
mosaic web link --session work --base-url https://mosaic.example.test/base/
```

This produces a session URL such as
`https://mosaic.example.test/base/work`.

When exposing a web server beyond localhost, use TLS, normal network access
controls, and read-only tokens for observer links. The Mosaic helper reports
`web_sharing_required: true` because session access is still governed by the
underlying Zellij-derived web sharing configuration.
