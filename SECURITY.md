# Security Policy

## Supported Versions

Security fixes target the latest minor release line unless a release note states otherwise.

## Reporting

Report vulnerabilities privately to the maintainers before public disclosure. Include affected commands, host adapters, bundle inputs, and any filesystem paths involved.

## Security Boundaries

Switchloom must not write user-level client configuration, follow symlinked managed paths, accept absolute or traversing artifact paths, publish local execution state, or treat custom bundles as official recommendations.
