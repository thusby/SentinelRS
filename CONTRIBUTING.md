# Contributing to SentinelRS

We appreciate your interest in contributing to SentinelRS! Whether it's reporting a bug, suggesting an enhancement, or submitting code changes, your help is valuable.

Please take a moment to review this document to ensure a smooth and effective contribution process.

## How Can I Contribute?

### 1. Reporting Bugs

If you find a bug, please help us by reporting it!

*   **Check existing issues:** Before opening a new issue, please check the [issues page](https://github.com/thusby/SentinelRS/issues) to see if the bug has already been reported.
*   **Use the bug report template:** If it's a new bug, please open a new issue and select the "Bug Report" template. Provide as much detail as possible, including:
    *   A clear and concise description of the bug.
    *   Steps to reproduce the behavior.
    *   Expected behavior.
    *   Actual behavior.
    *   Your macOS version.
    *   SentinelRS version (if applicable).
    *   Any relevant logs or screenshots.

### 2. Suggesting Enhancements

Do you have an idea for a new feature or an improvement to an existing one? We'd love to hear it!

*   **Check existing issues:** Before submitting, please check the [issues page](https://github.com/thusby/SentinelRS/issues) to see if the enhancement has already been suggested.
*   **Use the feature request template:** If it's a new idea, open a new issue and select the "Feature Request" template. Clearly describe:
    *   The problem your suggestion solves.
    *   The proposed solution.
    *   Any alternatives you've considered.

### 3. Code Contributions

We welcome code contributions! If you're looking to contribute code, here's a general workflow:

1.  **Fork the repository:** Start by forking the `thusby/SentinelRS` repository to your own GitHub account.
2.  **Clone your fork:**
    ```bash
    git clone https://github.com/your-username/SentinelRS.git
    cd SentinelRS
    ```
3.  **Set up the development environment:**
    *   Ensure you have [Rust and Cargo](https://rustup.rs/) installed.
    *   Build the application:
        ```bash
        chmod +x build_app.sh
        ./build_app.sh
        ```
    *   The `.app` bundle will be created in `target/release/bundle/macos/SentinelRS.app`.
4.  **Create a new branch:** Create a new branch for your feature or bug fix:
    ```bash
    git checkout -b feature/your-feature-name
    ```
    or
    ```bash
    git checkout -b bugfix/issue-number-description
    ```
5.  **Make your changes:**
    *   Follow the existing code style and conventions.
    *   Add comments and docstrings where necessary, following Rust's best practices.
    *   Ensure your code is well-tested (though formal tests are not yet implemented, consider manual testing).
6.  **Commit your changes:** Write clear and concise commit messages.
    ```bash
    git commit -m "feat: Add new feature"
    ```
    or
    ```bash
    git commit -m "fix: Resolve issue #123 with ... "
    ```
7.  **Push to your fork:**
    ```bash
    git push origin feature/your-feature-name
    ```
8.  **Open a Pull Request (PR):**
    *   Go to your fork on GitHub and open a new pull request against the `main` branch of the original `thusby/SentinelRS` repository.
    *   Provide a clear description of your changes, including why they were made and any relevant issue numbers.

## Code of Conduct

Please note that this project is released with a [Contributor Code of Conduct](https://www.contributor-covenant.org/version/2/1/code_of_conduct/code_of_conduct.md). By participating in this project, you agree to abide by its terms. We strive to create a welcoming and inclusive community.

Thank you for contributing!