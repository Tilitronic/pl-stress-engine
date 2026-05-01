# Polish Web Service

HTTP service for Polish stress analysis powered by the Rust WASM engine.

Package name:
- `@tilitronic/polish-web-service`

## Run locally

From repository root:

1. Install dependencies
   pnpm install

2. Start service
   pnpm run dev:web-service

If you need to register package globally for local testing:

   cd packages/polish-web-service
   pnpm link --global

If you need a tarball package:

   pnpm --filter @tilitronic/polish-web-service pack --pack-destination .local-packages

Service defaults:
- HOST=0.0.0.0
- PORT=8787

## Endpoints

- GET /health
- GET /stress?word=matematyka
- POST /stress with JSON body: { "word": "matematyka" }
- GET /stress/index?word=matematyka

Example:

   curl "http://localhost:8787/stress?word=matematyka"
