# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.13.1] - 2021-03-23
### Fixed
- The connect API using the `tokio-openssl` TLS implementation was broken in
  previous versions as no TLS connection was established before trying to
  establish the WebSocket connection. As such, connections always failed.
  Technically this is a breaking change when using this feature but in
  practice this a) wouldn't have worked anyway and b) it's unlikely someone
  uses the API in a way that would stop compiling now.

## [0.13.0] - 2021-02-13
### Changed
- Updated to tungstenite 0.13

## [0.12.0] - 2021-01-09
### Changed
- Updated tungstenite to version 0.12
- Migrated from pin-project to pin-project-lite
- `TokioAdapter` is now created via `TokioAdapter::new`

## [0.11.0] - 2020-12-30
### Changed
- Updated tokio to version 1.0
- Updated async-tls to version 0.11

## [0.10.0] - 2020-10-22
### Changed
- Updated tokio to version 0.3

## [0.9.3] - 2020-10-19
### Fixed
- Configure the server trust anchors for tokio-rustls

## [0.9.2] - 2020-10-17
### Added
- Implemented the `tokio::client_async_tls*` functions for `async-tls` and `tokio-rustls`

### Changed
- Updated pin-project to version 1

## Older releases
No changelog is available for older versions as of yet.

<!--
## [0.9.1] - 2020-10-13
## [0.9.0] - 2020-10-12
## [0.8.0] - 2020-07-27
## [0.7.1] - 2020-07-08
## [0.7.0] - 2020-06-29
## [0.6.0] - 2020-06-18
## [0.5.0] - 2020-05-22
## [0.4.2] - 2020-03-25
## [0.4.1] - 2020-03-24
## [0.4.0] - 2020-02-01
## [0.3.1] - 2020-01-06
## [0.3.0] - 2020-01-05
## [0.2.1] - 2019-12-12
## [0.2.0] - 2019-11-29
## [0.1.1] - 2019-11-29
## [0.1.0] - 2019-11-16
-->


[Unreleased]: https://github.com/sdroege/async-tungstenite/compare/0.12.0...HEAD
[0.12.0]: https://github.com/sdroege/async-tungstenite/compare/0.12.0...0.11.0
[0.11.0]: https://github.com/sdroege/async-tungstenite/compare/0.11.0...0.10.0
[0.10.0]: https://github.com/sdroege/async-tungstenite/compare/0.10.0...0.9.3
[0.9.3]: https://github.com/sdroege/async-tungstenite/compare/0.9.3...0.9.2
[0.9.2]: https://github.com/sdroege/async-tungstenite/compare/0.9.2...0.9.1
<!--
[0.9.1]: https://github.com/sdroege/async-tungstenite/compare/0.9.1...0.9.0
[0.9.0]: https://github.com/sdroege/async-tungstenite/compare/0.9.0...0.8.0
[0.8.0]: https://github.com/sdroege/async-tungstenite/compare/0.8.0...0.7.1
[0.7.1]: https://github.com/sdroege/async-tungstenite/compare/0.7.1...0.7.0
[0.7.0]: https://github.com/sdroege/async-tungstenite/compare/0.7.0...0.6.0
[0.6.0]: https://github.com/sdroege/async-tungstenite/compare/0.6.0...0.5.0
[0.5.0]: https://github.com/sdroege/async-tungstenite/compare/0.5.0...0.4.2
[0.4.2]: https://github.com/sdroege/async-tungstenite/compare/0.4.2...0.4.1
[0.4.1]: https://github.com/sdroege/async-tungstenite/compare/0.4.1...0.4.0
[0.4.0]: https://github.com/sdroege/async-tungstenite/compare/0.4.0...0.3.1
[0.3.1]: https://github.com/sdroege/async-tungstenite/compare/0.3.1...0.3.0
[0.3.0]: https://github.com/sdroege/async-tungstenite/compare/0.3.0...0.2.1
[0.2.1]: https://github.com/sdroege/async-tungstenite/compare/0.2.1...0.2.0
[0.2.0]: https://github.com/sdroege/async-tungstenite/compare/0.2.0...0.1.1
[0.1.1]: https://github.com/sdroege/async-tungstenite/compare/0.1.1...0.1.0
[0.1.0]: https://github.com/sdroege/async-tungstenite/releases/tag/v0.1.0
-->
