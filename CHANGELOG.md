# Changelog

## 1.1.0

- Updated `reqwest` from 0.11 to 0.12 to fix security vulnerabilities in its dependencies: RUSTSEC-2026-0258 (`h2`), RUSTSEC-2024-0421 (`idna`), RUSTSEC-2026-0098, RUSTSEC-2026-0099 and RUSTSEC-2026-0104 (`rustls-webpki`), and RUSTSEC-2025-0134 (`rustls-pemfile`, unmaintained). The public API is unchanged, and the client keeps its previous redirect limit and TCP keep-alive settings.
- The new HTTP stack behaves differently in a few uncommon setups:
  - Proxy environment variables: `HTTP_PROXY` and `HTTPS_PROXY` now take precedence over `ALL_PROXY`. A `socks5://` proxy in these variables now makes requests fail instead of being ignored. When `REQUEST_METHOD` is set (CGI), all proxy variables are ignored. An empty uppercase variable no longer falls back to the lowercase one.
  - Malformed internationalized host names in `base_url` or `request_origin`, such as `xn--example-.test`, are now rejected.
  - The `Debug` output and `source()` chain of transport errors read differently. Error variants and `Display` messages are unchanged.
- Building now requires Rust 1.88 or newer.

## 1.0.0

- Started the Rust SDK for the IPGeolocation.io IP Location API.
- Added the public client, config, request validation, response models, and metadata types.
- Scoped the SDK to `/v3/ipgeo` and `/v3/ipgeo-bulk`.
