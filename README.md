# mup

Set up reproducible Minecraft servers from the command line.

Why another tool? `mup` saves everything it does to a lockfile. This is helpful for sharing your server setup with others or deploying it to a different machine.
It's perfect for people like me that don't want to bother with containerization or a manager like Pterodactyl, and just want a simple tool to manage their server.

## Features
Supports the following Minecraft server types:
- Vanilla
- Fabric
- Forge/Neoforge
- Paper

And the following mod repositories:
- Modrinth
- Hangar
- CurseForge (planned)

## Examples
```bash
# Initialize a new Paper server in the current directory
mup server init --minecraft-version 1.21.4 --loader paper

# Install a specific version of a mod from Modrinth (default)
mup plugin add --version IPM0JlHd ferrite-core

# Update it to the latest version
mup plugin update ferrite-core
```
