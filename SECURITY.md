# Security Policy

## Supported Versions

Capsule OS is under active development. At this stage, only the latest version on the `main` branch is considered supported.

| Version        | Supported |
| -------------- | --------- |
| v0.1.0         | ✅       |

---

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly.

### Do not open a public issue for security vulnerabilities.

Instead, report it privately:

* Create a detailed report describing the issue
* Include steps to reproduce
* Provide potential impact and affected components

You can report via:

* GitHub private disclosure
* Direct contact with the maintainer

---

## What to Include in a Report

Please include as much detail as possible:

* Description of the vulnerability
* Steps to reproduce
* Expected vs actual behavior
* Logs, screenshots, or proof-of-concept (if applicable)
* Environment details (OS, version, build type)

---

## Scope

Capsule OS runs within a host operating system and does not directly manage hardware. However, security considerations still apply to:

* Virtual filesystem (sandbox boundaries)
* Command execution and shell behavior
* Configuration loading and parsing
* External process execution (e.g. editor launch)
* File access and path handling

---

## Security Considerations

Contributors and users should be aware of the following:

* Capsule OS is not a hardened sandbox and should not be treated as a secure isolation layer
* File operations are mapped to a host directory (`runtime/`) and should be handled carefully
* External commands (e.g. editors) are executed via the host OS
* Configuration files may be modified at runtime

---

## Responsible Disclosure

* Do not exploit vulnerabilities beyond what is necessary to demonstrate them
* Do not access or modify data that does not belong to you
* Allow time for fixes before public disclosure

---

## Updates and Fixes

Security fixes will be applied to the latest version. Users are encouraged to stay up to date.

---

## Disclaimer

Capsule OS is an experimental project and is provided as-is. It is not intended for production or security-critical environments at this stage.
