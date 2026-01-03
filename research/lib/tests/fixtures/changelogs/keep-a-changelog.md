# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Upcoming feature that hasn't been released yet

## [2.5.0] - 2024-03-15

### Added
- New authentication middleware for Express apps
- Support for custom error handlers
- Integration with Winston logger

### Changed
- Updated dependencies to latest versions
- Improved error messages for validation failures

### Fixed
- Race condition in connection pooling
- Memory leak in long-running processes

### Security
- Patched CVE-2024-12345 vulnerability

## [2.4.1] - 2024-02-20

### Fixed
- Critical bug in password hashing algorithm
- Incorrect timestamp formatting in logs

## [2.4.0] - 2024-02-01

### Added
- WebSocket support for real-time updates
- GraphQL schema validation

### Changed
- Refactored database connection handling
- Improved test coverage to 95%

### Deprecated
- Legacy REST API endpoints (will be removed in 3.0.0)

## [2.3.0] - 2024-01-10

### Added
- Support for PostgreSQL databases
- Docker compose configuration for development
- Comprehensive API documentation

### Breaking
- Removed deprecated `authenticate()` function
- Changed default port from 3000 to 8080

## [2.2.0] - 2023-12-05

### Added
- Rate limiting middleware
- Request caching with Redis

## [2.1.0] - 2023-11-15

### Added
- TypeScript type definitions
- Automatic API documentation generation

### Fixed
- CORS configuration issues
- Session expiration handling

## [2.0.0] - 2023-10-01

### Breaking
- Complete rewrite of authentication system
- New database schema (migration required)
- Changed configuration file format

### Added
- OAuth2 support (Google, GitHub, Facebook)
- Two-factor authentication
- User role management

## [1.5.0] - 2023-08-20

### Added
- Email verification for new accounts
- Password reset functionality

## [1.0.0] - 2023-06-01

### Added
- Initial stable release
- User registration and login
- JWT-based authentication
- Basic profile management

## [0.9.0] - 2023-05-15

### Added
- Beta release with core features
- User registration
- Login/logout functionality

### Known Issues
- Email notifications not working
- Performance issues with large datasets

## [0.5.0] - 2023-04-01

### Added
- Alpha release for testing
- Basic user management
- Database schema

[Unreleased]: https://github.com/example/project/compare/v2.5.0...HEAD
[2.5.0]: https://github.com/example/project/compare/v2.4.1...v2.5.0
[2.4.1]: https://github.com/example/project/compare/v2.4.0...v2.4.1
[2.4.0]: https://github.com/example/project/compare/v2.3.0...v2.4.0
[2.3.0]: https://github.com/example/project/compare/v2.2.0...v2.3.0
[2.2.0]: https://github.com/example/project/compare/v2.1.0...v2.2.0
[2.1.0]: https://github.com/example/project/compare/v2.0.0...v2.1.0
[2.0.0]: https://github.com/example/project/compare/v1.5.0...v2.0.0
[1.5.0]: https://github.com/example/project/compare/v1.0.0...v1.5.0
[1.0.0]: https://github.com/example/project/compare/v0.9.0...v1.0.0
[0.9.0]: https://github.com/example/project/compare/v0.5.0...v0.9.0
[0.5.0]: https://github.com/example/project/releases/tag/v0.5.0
