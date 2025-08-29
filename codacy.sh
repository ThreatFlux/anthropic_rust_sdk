#!/bin/bash

# Claude Code + Codacy Setup Script
# This script installs Claude Code, sets up Codacy integration, and creates configuration files

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running on macOS or Linux
if [[ "$OSTYPE" == "darwin"* ]]; then
    SHELL_CONFIG="$HOME/.zshrc"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # Check if zsh is being used, otherwise use bash
    if [[ "$SHELL" == *"zsh"* ]]; then
        SHELL_CONFIG="$HOME/.zshrc"
    else
        SHELL_CONFIG="$HOME/.bashrc"
    fi
else
    print_warning "Unsupported OS detected. Defaulting to ~/.bashrc"
    SHELL_CONFIG="$HOME/.bashrc"
fi

print_status "Using shell configuration file: $SHELL_CONFIG"

# Step 1: Install Claude Code
print_status "Installing Claude Code..."
if command -v npm &> /dev/null; then
    npm install -g @anthropic-ai/claude-code
    print_status "Claude Code installed successfully"
else
    print_error "npm is not installed. Please install Node.js and npm first."
    exit 1
fi

# Step 2: Setup Codacy Environment Variable
print_status "Setting up Codacy environment variable..."

# Prompt for Codacy token
echo "Enter your Codacy Account Token (or press Enter to skip if token already exists in environment/shell config/.env):"
read -p "Token: " CODACY_TOKEN

if [[ -n "$CODACY_TOKEN" ]]; then
    # Add environment variable to shell configuration
    echo "export CODACY_ACCOUNT_TOKEN=$CODACY_TOKEN" >> "$SHELL_CONFIG"
    print_status "Added CODACY_ACCOUNT_TOKEN to $SHELL_CONFIG"
    
    # Export for current session
    export CODACY_ACCOUNT_TOKEN="$CODACY_TOKEN"
    print_status "Environment variable set for current session"
else
    print_status "Skipping token setup - assuming CODACY_ACCOUNT_TOKEN is already configured"
    print_warning "Make sure CODACY_ACCOUNT_TOKEN is available in your environment, shell config, or .env file"
fi

# Step 3: Add MCP Configuration
print_status "Adding Codacy MCP configuration..."
if command -v claude &> /dev/null; then
    claude mcp add codacy npx -- -y @codacy/codacy-mcp --env CODACY_ACCOUNT_TOKEN=\$CODACY_ACCOUNT_TOKEN
    print_status "Codacy MCP configuration added successfully"
else
    print_error "Claude Code CLI not found. Please ensure it's properly installed and in your PATH."
    exit 1
fi

# Step 4: Create CLAUDE.md configuration file
print_status "Creating CLAUDE.md configuration file..."

cat > CLAUDE.md << 'EOF'
---
# Codacy Rules
Configuration for AI behavior when interacting with Codacy's MCP Server

## using any tool that accepts the arguments: `provider`, `organization`, or `repository`
- Use `git remote -v` to determine the correct values dynamically
- Extract provider, organization, and repository from the git remote URL
- Provider mapping:
  - GitHub repositories: provider = "gh"
  - Bitbucket repositories: provider = "bb"
  - GitLab repositories: provider = "gl"
- Parse the organization and repository name from the remote URL

## After ANY successful `edit_file` or `reapply` operation
- YOU MUST IMMEDIATELY run the `codacy_cli_analyze` tool from Codacy's MCP Server for each file that was edited, with:
- `rootPath`: set to the workspace path
- `file`: set to the path of the edited file
- `tool`: leave empty or unset
- If any issues are found in the new edits, propose and apply fixes for them.
- > NOTE: Failure to follow this rule is considered a critical error.

## CRITICAL: Dependencies and Security Checks
- IMMEDIATELY after ANY of these actions:
- Running npm/yarn/pnpm install
- Adding dependencies to package.json
- Adding requirements to requirements.txt
- Adding dependencies to pom.xml
- Adding dependencies to build.gradle
- Any other package manager operations
- You MUST run the `codacy_cli_analyze` tool with:
- `rootPath`: set to the workspace path
- `tool`: set to "trivy"
- `file`: leave empty or unset
- If any vulnerabilities are found because of the newly added packages:
- Stop all other operations
- Propose and apply fixes for the security issues
- Only continue with the original task after security issues are resolved
- EXAMPLE:
- After: npm install react-markdown
- Do: Run codacy_cli_analyze with trivy
- Before: Continuing with any other tasks

## General
- Repeat the relevant steps for each modified file.
- "Propose fixes" means to both suggest and, if possible, automatically apply the fixes.
- You MUST NOT wait for the user to ask for analysis or remind you to run the tool.
- Do not run `codacy_cli_analyze` looking for changes in duplicated code or code complexity metrics.
- Do not run `codacy_cli_analyze` looking for changes in code coverage.
- Do not try to manually install Codacy CLI using either brew, npm, npx, or any other package manager.
- If the Codacy CLI is not installed, just run the `codacy_cli_analyze` tool from Codacy's MCP Server.
- When calling `codacy_cli_analyze`, only send provider, organization and repository if the project is a git repository.

## Whenever a call to a Codacy tool that uses `repository` or `organization` as a parameter returns a 404 error
- Offer to run the `codacy_setup_repository` tool to add the repository to Codacy
- If the user accepts, run the `codacy_setup_repository` tool
- Do not ever try to run the `codacy_setup_repository` tool on your own
- After setup, immediately retry the action that failed (only retry once)
---
EOF

print_status "CLAUDE.md created successfully in the current directory"

# Final instructions
print_status "Setup completed successfully!"
echo ""
print_warning "IMPORTANT: Please restart your terminal or run the following command to reload your shell configuration:"
echo "source $SHELL_CONFIG"
echo ""
print_status "You can now use Claude Code with Codacy integration!"
print_status "The CLAUDE.md file has been created in your current directory with the configuration rules."
