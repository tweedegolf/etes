# Etes 

## Ephemeral Test Environment Service

Easily run different versions and instances of your web / server application during development - given a (fairly strict) set of requirements.

Requirements:

* The applications should be a single binary! Like a Rust application serving it's own static assets with [Memory serve](https://crates.io/crates/memory-serve)
* Etes is build for __GitHub__. It fetches the currently open Pull Requests and Releases and lets users login using GitHub OAuth.
* Binaries should be uploaded, prefereable from a GitHub action.

## Screenshot

![Etes](./screenshot.jpg?raw=true)

## Features

* Start new environments on the fly, from a list of Pull Requests or the latest releases
* Each environment has a unique random name (by combining a few words from a provided word list as a sub-domain)
* Users can shutdown the services they started
* Admins (configured by a list of GitHub usernames) can shut down any service
* configure a cutsom page title and favicon
* Live interface updates
* Bind trigger/latest and build/merge commits

## Configuration

Configuration options can be provided using the environment, or a configuration file.
A configuration file names `config.toml` should be placed in the current working directory.
Environment variables overwrite any options from the configuration file and should have a name prefixed by `ETES_`.

### Required configuration values

- `title`: Page title and header
- `github_token`: GitHub token, no priviliges neccecary, only used for read-only access to PR's and Releases using the GraphQL API
- `github_owner`: GitHub owner or organisation
- `github_repo`: GitHub repository name
- `github_client_id`: GitHub Oauth client ID
- `github_client_secret`: GitHub Oauth client secret
- `authorize_url`: OAuth callback URL
- `session_key`: Session key for cookies
- `api_key`: API key for binary uploads
- `command_args`: Arguments passed to the binary, use {port} to interpolate the port number
- `favicon`: Emoji favicon or letter
- `words`: List of words to combine into a unique service name
- `admins`: Github user names / handles of admins

An example configuration file can be found in this repository.

## Uploading a binary

GitHub action example:

```yaml
  deploy:
    environment:
      name: test
      url: https://${{ github.sha }}.example.com
    permissions:
      contents: read
      deployments: write
    runs-on: ubuntu-latest
    steps:
      - name: Download build artifact
        uses: actions/download-artifact@v4
        with:
          name: executable.bin
      - name: Upload binary to Etes
        run: |
          curl -s \
          -H "Authorization: Bearer ${{ secrets.ETES_API_KEY }}" \
          -T ./executable.bin \
          https://example.com/etes/api/v1/executable/${{ github.sha }}/${{ github.sha }}

```

## Configure reverse proxy for Etes

A reverse proxy that terminates TLS connections should be configured. The base domain should point to port 3000 and all sub-domains should point to port 3001.

Example using caddy:

```
example.com {
    reverse_proxy localhost:3000
}

*.example.com  {
	reverse_proxy localhost:3001
}

```

## Configure Etes as a systemd service

Service file:

```
[Unit]
Description=Etes
After=network.target
Wants=network-online.target

[Service]
Restart=always
Type=simple
ExecStart=/app/etes
WorkingDirectory=/app
User=app
Group=app

[Install]
WantedBy=multi-user.target
```