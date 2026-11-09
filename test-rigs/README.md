# Integration rigs — pending W1 work

No Docker services are bundled yet. Use the upstream [moodlehq/moodle-docker](https://github.com/moodlehq/moodle-docker) tooling for separate Moodle 4.5 and 5.3 checkouts. Configure HTTPS, enable mobile web services, and seed two courses and three users. Add Keycloak SAML/OIDC and Mailpit per Alpha plan §5.1.

Pin verified Moodle and IdP image/source versions when the rigs are provisioned. Do not substitute unverified image tags or commit live credentials. The pure Rust unit tests do not need a Moodle instance; end-to-end SSO verification does.
