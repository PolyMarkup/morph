# Morph web workbench

The workbench is a Vite frontend served with the Rust Cloudflare Worker in this
crate. Frontend packages are isolated here; they do not add dependencies to the
Morph core or CLI.

Production: <https://morph-web.erik-pragt.workers.dev>

## Local development

Install a current Node.js release, Rust, the WebAssembly target, and
`worker-build`:

```sh
rustup target add wasm32-unknown-unknown
cargo install worker-build --version 0.7.4 --locked
npm ci
```

Run the Worker API on port 8787:

```sh
npx wrangler@4.116.0 dev --port 8787
```

In a second terminal, run Vite on port 5173:

```sh
npm run dev
```

Vite proxies `/api` requests to the Worker. Wrangler also serves a production
frontend build at <http://localhost:8787>.

## Verification

Run the frontend tests and production build:

```sh
npm run validate
```

Build the Worker separately:

```sh
worker-build --release
```

The repository CI runs both commands as well as the complete Rust workspace
test suite.

## Deployment

Deploy from this directory with:

```sh
npx wrangler@4.116.0 deploy
```

Pushes to `main` or `master` deploy automatically after CI succeeds when the
repository has `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` secrets.

The conversion endpoint accepts at most 256 KiB of UTF-8 input and ten unique
targets. Requests are processed transiently with `Cache-Control: no-store`;
Morph does not persist document content.
