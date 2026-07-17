# Security Policy

## Supported Versions

Security fixes target the current unreleased tree until v0.1.0 is published. After v0.1.0, only the latest minor line is supported unless a release note states otherwise.

## Reporting

Report vulnerabilities privately to the maintainers before public disclosure. Include affected commands, host adapters, bundle inputs, and any filesystem paths involved.

## Security Boundaries

Switchloom must not write user-level client configuration, follow symlinked managed paths, accept absolute or traversing artifact paths, publish local Planr state, or treat unsigned/custom bundles as official recommendations.
