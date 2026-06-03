# Contributing to Vessel

First off, thank you for considering contributing to Vessel! It's people like you that make Vessel such a great tool.

## Code of Conduct

By participating in this project, you are expected to uphold our Code of Conduct (be kind, be professional).

## How Can I Contribute?

### Reporting Bugs

- **Check if the bug has already been reported.** Search the [issues](https://github.com/vessel-app/vessel/issues).
- **Use the bug report template.** Provide as much detail as possible.
- **Provide a clear description** of how to reproduce the bug.

### Suggesting Enhancements

- **Check if the feature has already been suggested.**
- **Use the feature request template.**
- **Explain the "why".** Why would this feature be useful to most Vessel users?

### Pull Requests

1. **Fork the repo** and create your branch from `main`.
2. **Install dependencies** with `npm install`.
3. **Ensure your code passes checks** by running `npm run check`.
4. **Follow the style.** Match the existing code style and architecture.
5. **Issue a Pull Request.** Provide a clear description of your changes.

## Development Setup

See the [README.md](./README.md) for basic setup instructions.

### Architecture Notes

- **Rust:** Handles webview lifecycles, SQLite, and system metrics.
- **SvelteKit:** Handles the UI and state management.
- **IPC:** See [DOCS/IPC.md](./DOCS/IPC.md) for the communication contract between Rust and TS.

## License

By contributing, you agree that your contributions will be licensed under its MIT License.
